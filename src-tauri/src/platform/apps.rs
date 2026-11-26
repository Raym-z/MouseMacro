//! Installed-application discovery for the Exceptions page.
//!
//! The page used to offer only applications that already had a visible window,
//! which made it hard to pre-exclude a game that was not running yet. This
//! module combines three sources:
//!
//! - visible windows (running apps),
//! - Start-menu shortcuts (installed apps),
//! - Steam and Epic libraries (games).
//!
//! Launcher manifests are treated as games by definition. Start-menu entries
//! are marked as games when the shortcut lives under a Games folder or its
//! target sits in a well-known game directory/publisher folder.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    /// Lowercase executable name, e.g. `game.exe`.
    pub process: String,
    /// Friendly name for display.
    pub name: String,
    /// `running`, `startMenu`, `steam`, `epic` or `riot`.
    pub source: String,
    pub game: bool,
    pub running: bool,
}

/// Enumerates installed and running applications. Game libraries are scanned
/// before the Start menu so a game keeps its launcher name and `game` flag.
pub fn list_installed_apps() -> Vec<InstalledApp> {
    let mut apps: HashMap<String, InstalledApp> = HashMap::new();

    for app in crate::platform::foreground::list_open_apps() {
        insert(
            &mut apps,
            InstalledApp {
                process: app.process,
                name: app.title,
                source: "running".into(),
                game: false,
                running: true,
            },
        );
    }

    for app in steam_games() {
        insert(&mut apps, app);
    }
    for app in epic_games() {
        insert(&mut apps, app);
    }
    for app in riot_games() {
        insert(&mut apps, app);
    }

    // Resolve shortcuts on a separate STA thread: ShellLink is an
    // apartment-threaded COM object and callers may already be in another COM
    // apartment (Tauri's main thread, for example).
    for app in shortcut_apps() {
        insert(&mut apps, app);
    }

    let mut list: Vec<InstalledApp> = apps.into_values().collect();
    list.sort_by(|a, b| {
        b.game
            .cmp(&a.game)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.process.cmp(&b.process))
    });
    list
}

fn insert(apps: &mut HashMap<String, InstalledApp>, app: InstalledApp) {
    match apps.get_mut(&app.process) {
        Some(existing) => {
            existing.game |= app.game;
            existing.running |= app.running;
            // A launcher/Start-menu name is cleaner than a live window title.
            if existing.source == "running" && app.source != "running" {
                existing.name = app.name;
                existing.source = app.source;
            } else if existing.name.is_empty()
                || existing.name.eq_ignore_ascii_case(&existing.process)
            {
                existing.name = app.name;
            }
        }
        None => {
            apps.insert(app.process.clone(), app);
        }
    }
}

fn process_from_path(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().trim().to_lowercase())
        .filter(|name| name.ends_with(".exe") && name.len() > 4)
}

// ---------------------------------------------------------------------------
// Start menu
// ---------------------------------------------------------------------------

fn shortcut_apps() -> Vec<InstalledApp> {
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(programdata) = std::env::var_os("PROGRAMDATA") {
        roots.push(
            PathBuf::from(&programdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
        roots.push(PathBuf::from(programdata).join("Desktop"));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(profile).join("Desktop"));
    }

    let mut links = Vec::new();
    for root in roots {
        collect_links(&root, 0, &mut links);
    }

    if links.is_empty() {
        return Vec::new();
    }

    std::thread::Builder::new()
        .name("installed-apps-com".into())
        .spawn(move || resolve_links(&links))
        .ok()
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

fn collect_links(dir: &Path, depth: usize, links: &mut Vec<PathBuf>) {
    if depth > 5 || links.len() >= 900 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_links(&path, depth + 1, links);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            links.push(path);
        }
    }
}

fn resolve_links(links: &[PathBuf]) -> Vec<InstalledApp> {
    use windows::Win32::Foundation::S_OK;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED,
    };

    let initialized = unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED) == S_OK
    };

    let mut apps = Vec::new();
    for link_path in links {
        let Some(display_name) = link_path
            .file_stem()
            .map(|name| name.to_string_lossy().trim().to_string())
        else {
            continue;
        };
        if display_name.is_empty() || is_helper_shortcut(&display_name) {
            continue;
        }

        let Some(target) = shortcut_target(link_path) else {
            continue;
        };
        let Some(process) = process_from_path(&target) else {
            continue;
        };
        // Ignore generic launchers/updaters that appear in many vendors'
        // folders; they are not the application the user wants a rule for.
        if is_helper_process(&process) {
            continue;
        }

        let target_text = target.to_string_lossy().to_lowercase();
        let link_text = link_path.to_string_lossy().to_lowercase();
        let game = !is_launcher_process(&process)
            && looks_like_game_path(&target_text, &link_text);
        apps.push(InstalledApp {
            process,
            name: display_name,
            source: "startMenu".into(),
            game,
            running: false,
        });
    }

    if initialized {
        unsafe { CoUninitialize() };
    }
    apps
}

fn shortcut_target(link_path: &Path) -> Option<PathBuf> {
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

    let wide: Vec<u16> = link_path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect();

    unsafe {
        let link: IShellLinkW =
            CoCreateInstance(&ShellLink, None::<&windows::core::IUnknown>, CLSCTX_INPROC_SERVER)
                .ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        persist
            .Load(PCWSTR(wide.as_ptr()), STGM_READ)
            .ok()?;

        let mut buffer = [0u16; 1024];
        link.GetPath(&mut buffer, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .ok()?;
        let length = buffer
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(buffer.len());
        if length == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..length]);
        let path = PathBuf::from(path);
        path.is_absolute().then_some(path)
    }
}

fn is_helper_shortcut(name: &str) -> bool {
    let name = name.to_lowercase();
    [
        "uninstall",
        "readme",
        "read me",
        "help",
        "website",
        "web site",
        "documentation",
        "support",
        "license",
        "eula",
        "repair",
        "modify",
        "updater",
        "update ",
        "check for updates",
    ]
    .iter()
    .any(|word| name.contains(word))
}

fn is_helper_process(process: &str) -> bool {
    let process = process.to_lowercase();
    [
        "unins000.exe",
        "uninstall.exe",
        "unitycrashhandler",
        "crashreport",
        "crashpad_handler",
        "ue4prereqsetup",
        "dxsetup",
        "vcredist",
        "installer.exe",
        "update.exe",
        "updater.exe",
        "vconsole",
        "webhelper",
        "steamerrorreporter.exe",
    ]
    .iter()
    .any(|needle| process.contains(needle))
}

/// Store clients and launchers are apps, but they are not games. They still
/// appear in the picker so a user can add a rule for them.
fn is_launcher_process(process: &str) -> bool {
    let process = process.to_lowercase();
    [
        "epicgameslauncher.exe",
        "riotclientservices.exe",
        "riotclientux.exe",
        "steam.exe",
        "battle.net.exe",
        "galaxyclient.exe",
        "ubisoftconnect.exe",
        "uplay.exe",
        "eadesktop.exe",
        "origin.exe",
    ]
    .iter()
    .any(|needle| process == *needle)
}

fn looks_like_game_path(target: &str, shortcut: &str) -> bool {
    const GAME_MARKERS: [&str; 17] = [
        "\\steamapps\\common\\",
        "\\epic games\\",
        "\\gog galaxy\\games\\",
        "\\gog games\\",
        "\\ubisoft game launcher\\games\\",
        "\\ubisoft\\ubisoft game launcher\\games\\",
        "\\battle.net\\",
        "\\riot games\\",
        "\\ea games\\",
        "\\origin games\\",
        "\\rockstar games\\",
        "\\bethesda.net launcher\\games\\",
        "\\xboxgames\\",
        "\\yostargames\\",
        "\\gameloop\\",
        "\\games\\",
        "\\game\\",
    ];
    const PUBLISHERS: [&str; 12] = [
        "\\valve\\",
        "\\blizzard",
        "\\activision",
        "\\electronic arts\\",
        "\\ubisoft\\",
        "\\rockstar",
        "\\bethesda",
        "\\2k games\\",
        "\\square enix\\",
        "\\bandai namco\\",
        "\\miHoYo",
        "\\hoyoverse\\",
    ];

    let target = target.to_lowercase();
    if GAME_MARKERS.iter().any(|marker| target.contains(marker)) {
        return true;
    }
    if shortcut.contains("\\start menu\\programs\\games\\")
        || shortcut.contains("\\start menu\\programs\\games\\")
    {
        return true;
    }
    PUBLISHERS
        .iter()
        .any(|publisher| target.contains(&publisher.to_lowercase()))
}

// ---------------------------------------------------------------------------
// Launchers
// ---------------------------------------------------------------------------

fn steam_games() -> Vec<InstalledApp> {
    let mut roots = Vec::new();
    if let Some(root) = steam_root_from_registry() {
        roots.push(root);
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Steam"));
    }

    let mut apps = Vec::new();
    let mut seen_libraries: Vec<PathBuf> = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let mut libraries = vec![root.clone()];
        libraries.extend(steam_library_paths(&root));
        for library in libraries {
            let library = library.canonicalize().unwrap_or(library);
            if seen_libraries.contains(&library) {
                continue;
            }
            seen_libraries.push(library.clone());
            apps.extend(steam_library_games(&library));
        }
    }
    apps
}

fn steam_root_from_registry() -> Option<PathBuf> {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ,
    };

    let mut buffer = [0u16; 512];
    let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Valve\\Steam"),
            w!("SteamPath"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if status.0 != 0 {
        return None;
    }
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    let path = PathBuf::from(String::from_utf16_lossy(&buffer[..length]));
    path.is_dir().then_some(path)
}

fn steam_library_paths(root: &Path) -> Vec<PathBuf> {
    let vdf = root.join("steamapps").join("libraryfolders.vdf");
    let Ok(text) = fs::read_to_string(vdf) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("\"path\"") else {
            continue;
        };
        let Some(value) = rest.trim().strip_prefix('"') else {
            continue;
        };
        let Some(end) = value.rfind('"') else {
            continue;
        };
        let path = value[..end].replace("\\\\", "\\");
        if !path.is_empty() {
            paths.push(PathBuf::from(path));
        }
    }
    paths
}

fn steam_library_games(library: &Path) -> Vec<InstalledApp> {
    let steamapps = library.join("steamapps");
    let Ok(entries) = fs::read_dir(&steamapps) else {
        return Vec::new();
    };
    let mut apps = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("appmanifest_") || !file_name.ends_with(".acf") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let name = acf_value(&text, "name").unwrap_or_default();
        let install_dir = acf_value(&text, "installdir").unwrap_or_default();
        if name.is_empty() || install_dir.is_empty() || is_steam_tool(&name) {
            continue;
        }
        let game_dir = steamapps.join("common").join(&install_dir);
        let Some(exe) = find_main_executable(&game_dir, &name) else {
            continue;
        };
        let Some(process) = process_from_path(&exe) else {
            continue;
        };
        apps.push(InstalledApp {
            process,
            name,
            source: "steam".into(),
            game: true,
            running: false,
        });
    }
    apps
}

fn acf_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(&format!("\"{key}\"")) else {
            continue;
        };
        let value = rest.trim();
        let value = value.strip_prefix('"')?;
        let end = value.rfind('"')?;
        return Some(value[..end].replace("\\\\", "\\"));
    }
    None
}

fn is_steam_tool(name: &str) -> bool {
    let name = name.to_lowercase();
    [
        "steamworks common redistributables",
        "steamvr",
        "steam linux runtime",
        "steam client",
        "proton",
        "dedicated server",
        "soundtrack",
        "sdk",
        "tools",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

fn epic_games() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    // Modern Epic manifests contain the launch executable verbatim.
    if let Some(programdata) = std::env::var_os("PROGRAMDATA") {
        let manifests = PathBuf::from(&programdata)
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests");
        if let Ok(entries) = fs::read_dir(manifests) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("item")) {
                    if let Some(app) = epic_manifest_app(&path) {
                        apps.push(app);
                    }
                }
            }
        }

        let launcher_json = PathBuf::from(programdata)
            .join("Epic")
            .join("UnrealEngineLauncher")
            .join("LauncherInstalled.dat");
        if let Ok(text) = fs::read_to_string(launcher_json) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(list) = value
                    .get("InstallationList")
                    .and_then(|list| list.as_array())
                {
                    for item in list {
                        let Some(location) = item
                            .get("InstallLocation")
                            .and_then(|value| value.as_str())
                        else {
                            continue;
                        };
                        let name = item
                            .get("AppName")
                            .and_then(|value| value.as_str())
                            .unwrap_or("Epic game");
                        if is_epic_tool(name) {
                            continue;
                        }
                        if let Some(app) = app_from_install_dir(Path::new(location), name, "epic") {
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    apps
}

fn epic_manifest_app(path: &Path) -> Option<InstalledApp> {
    let text = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let name = value.get("DisplayName")?.as_str()?.trim();
    let location = value.get("InstallLocation")?.as_str()?;
    if name.is_empty() || is_epic_tool(name) {
        return None;
    }
    if let Some(executable) = value.get("LaunchExecutable").and_then(|v| v.as_str()) {
        let path = Path::new(location).join(executable);
        if let Some(process) = process_from_path(&path) {
            return Some(InstalledApp {
                process,
                name: name.to_string(),
                source: "epic".into(),
                game: true,
                running: false,
            });
        }
    }
    app_from_install_dir(Path::new(location), name, "epic")
}

fn is_epic_tool(name: &str) -> bool {
    let name = name.to_lowercase();
    ["unreal engine", "epic games launcher", "marketplace", "unreal marketplace"]
        .iter()
        .any(|needle| name.contains(needle))
}

fn app_from_install_dir(dir: &Path, name: &str, source: &str) -> Option<InstalledApp> {
    let exe = find_main_executable(dir, name)?;
    Some(InstalledApp {
        process: process_from_path(&exe)?,
        name: name.to_string(),
        source: source.into(),
        game: true,
        running: false,
    })
}

/// Riot launches its games through `Riot Client`, so the Start-menu shortcut
/// points at `RiotClientServices.exe` and the real game binary lives under the
/// product folder. Scan those folders and add the likely game executables.
fn riot_games() -> Vec<InstalledApp> {
    let mut roots = Vec::new();
    // Riot installs to `C:\Riot Games` by default but allows another drive.
    for letter in b'A'..=b'Z' {
        let path = PathBuf::from(format!("{}:\\Riot Games", letter as char));
        if path.is_dir() {
            roots.push(path);
        }
    }
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(program_files) = std::env::var_os(variable) {
            roots.push(PathBuf::from(program_files).join("Riot Games"));
        }
    }

    let mut apps = Vec::new();
    let mut seen = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let product_dir = entry.path();
            let Some(name) = product_dir
                .file_name()
                .map(|name| name.to_string_lossy().trim().to_string())
            else {
                continue;
            };
            if name.eq_ignore_ascii_case("Riot Client")
                || name.eq_ignore_ascii_case("Metadata")
                || !product_dir.is_dir()
            {
                continue;
            }

            let mut candidates = Vec::new();
            collect_executables(&product_dir, 0, 5, &mut candidates);
            candidates.retain(|path| {
                fs::metadata(path)
                    .map(|metadata| metadata.len() >= 1024 * 1024)
                    .unwrap_or(false)
            });
            candidates.sort_by_key(|path| {
                let score = executable_name_score(path, &name);
                let size = fs::metadata(path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                (score, size)
            });
            candidates.reverse();
            candidates.truncate(3);

            for exe in candidates {
                let Some(process) = process_from_path(&exe) else {
                    continue;
                };
                if seen.contains(&process) {
                    continue;
                }
                seen.push(process.clone());
                apps.push(InstalledApp {
                    process,
                    name: name.clone(),
                    source: "riot".into(),
                    game: true,
                    running: false,
                });
            }
        }
    }
    apps
}

// ---------------------------------------------------------------------------
// Executable discovery inside a game folder
// ---------------------------------------------------------------------------

fn find_main_executable(dir: &Path, name_hint: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let mut candidates = Vec::new();
    collect_executables(dir, 0, 3, &mut candidates);
    candidates.into_iter().max_by_key(|path| {
        let score = executable_name_score(path, name_hint);
        let size = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        // Name similarity wins over size: dev tools such as vconsole2.exe can
        // be larger than the actual game executable.
        (score, size)
    })
}

/// Scores how likely an executable is the game's main binary.
///
/// 3 = the executable equals the full game name (`stardewvalley`).
/// 2 = the executable equals the initials (`cs2` for Counter-Strike 2).
/// 1 = the executable contains a distinctive game-name token.
fn executable_name_score(path: &Path, name_hint: &str) -> u8 {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return 0;
    };
    let exe = alphanumeric(stem);
    let name = alphanumeric(name_hint);
    if exe.is_empty() || name.is_empty() {
        return 0;
    }
    if exe == name {
        return 3;
    }
    let initials: String = name_hint
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.chars().next())
        .collect::<String>()
        .to_lowercase();
    if exe == initials {
        return 2;
    }
    if exe.contains(&name) || name.contains(&exe) {
        return 1;
    }
    0
}

fn alphanumeric(text: &str) -> String {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn collect_executables(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    found: &mut Vec<PathBuf>,
) {
    if depth > max_depth || found.len() >= 160 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if is_ignored_game_dir(&path) {
                continue;
            }
            collect_executables(&path, depth + 1, max_depth, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let name = name.to_lowercase();
            if is_helper_process(&name) {
                continue;
            }
            let size = fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            // Tiny launchers and crash reporters are not the game itself.
            if size >= 96 * 1024 {
                found.push(path);
            }
        }
    }
}

fn is_ignored_game_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    let name = name.to_lowercase();
    [
        "_commonredist",
        "redist",
        "redistributables",
        "directx",
        "vcredist",
        "dotnet",
        "support",
        "docs",
        "tools",
        "editor",
        "crashreport",
        "easyanticheat",
        "battleye",
        "engine\\extras",
        "linux",
        "mac",
        "steamworks",
    ]
    .iter()
    .any(|needle| name == *needle || name.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_names_are_skipped() {
        assert!(is_helper_shortcut("Uninstall Game"));
        assert!(is_helper_shortcut("Readme"));
        assert!(!is_helper_shortcut("Photoshop"));
        assert!(is_helper_process("unins000.exe"));
        assert!(is_helper_process("UnityCrashHandler64.exe"));
        assert!(!is_helper_process("game.exe"));
        assert!(is_launcher_process("RiotClientServices.exe"));
        assert!(!is_launcher_process("VALORANT-Win64-Shipping.exe"));
    }

    #[test]
    fn game_paths_are_recognised() {
        assert!(looks_like_game_path(
            r"c:\program files (x86)\steam\steamapps\common\portal\portal.exe",
            r"c:\programdata\microsoft\windows\start menu\programs\portal.lnk"
        ));
        assert!(looks_like_game_path(
            r"c:\yostargames\arknights_en_gamelauncher\arknights_en_gamelauncher.exe",
            r"c:\programdata\microsoft\windows\start menu\programs\arknights.lnk"
        ));
        assert!(!looks_like_game_path(
            r"c:\program files\google\chrome\application\chrome.exe",
            r"c:\programdata\microsoft\windows\start menu\programs\google chrome.lnk"
        ));
    }

    #[test]
    fn steam_manifest_values_are_parsed() {
        let text = "\"AppState\"\n{\n\t\"appid\"\t\t\"570\"\n\t\"name\"\t\t\"Dota 2\"\n\t\"installdir\"\t\t\"dota 2 beta\"\n}\n";
        assert_eq!(acf_value(text, "name").as_deref(), Some("Dota 2"));
        assert_eq!(
            acf_value(text, "installdir").as_deref(),
            Some("dota 2 beta")
        );
    }

    #[test]
    fn the_main_game_executable_is_preferred_by_name() {
        assert_eq!(
            executable_name_score(Path::new("cs2.exe"), "Counter-Strike 2"),
            2
        );
        assert_eq!(
            executable_name_score(Path::new("Stardew Valley.exe"), "Stardew Valley"),
            3
        );
        assert_eq!(
            executable_name_score(Path::new("vconsole2.exe"), "Counter-Strike 2"),
            0
        );
    }

    #[test]
    fn steam_tools_are_not_treated_as_games() {
        assert!(is_steam_tool("SteamVR"));
        assert!(is_steam_tool("Steamworks Common Redistributables"));
        assert!(!is_steam_tool("Half-Life 2"));
    }

    #[test]
    #[ignore = "diagnostic: scans the local machine"]
    fn print_installed_apps() {
        let apps = list_installed_apps();
        println!("{} installed/running apps", apps.len());
        for app in apps {
            println!(
                "{:<32} {:<9} game={} running={} {}",
                app.process, app.source, app.game, app.running, app.name
            );
        }
    }

    #[test]
    fn installed_apps_are_deduplicated_by_process() {
        let mut apps = HashMap::new();
        insert(
            &mut apps,
            InstalledApp {
                process: "game.exe".into(),
                name: "Game".into(),
                source: "startMenu".into(),
                game: false,
                running: false,
            },
        );
        insert(
            &mut apps,
            InstalledApp {
                process: "game.exe".into(),
                name: "Game".into(),
                source: "steam".into(),
                game: true,
                running: true,
            },
        );
        let app = apps.get("game.exe").unwrap();
        assert!(app.game);
        assert!(app.running);
    }
}
