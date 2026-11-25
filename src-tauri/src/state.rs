use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::config::model::{Action, AppConfig, Mapping, Profile, Settings, Theme, Trigger};
use crate::config::store::ConfigStore;
use crate::engine::keys::{normalize_combo, vk_from_name};
use crate::platform::elevation;
use crate::platform::service::{CaptureMode, EngineService};
use crate::tray;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub hooks_active: bool,
    pub capture_mode: String,
    pub elevated: bool,
    pub autostart: bool,
    pub config_path: String,
    pub config_dir: String,
    pub version: String,
    pub platform: String,
    pub load_warning: Option<String>,
    pub hook_warning: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub config: AppConfig,
    pub status: AppStatus,
}

pub struct AppState {
    config: Mutex<AppConfig>,
    write_lock: Mutex<()>,
    pub store: ConfigStore,
    pub engine: Arc<EngineService>,
    pub app: AppHandle,
    load_warning: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(
        app: AppHandle,
        config: AppConfig,
        store: ConfigStore,
        engine: Arc<EngineService>,
        load_warning: Option<String>,
    ) -> Self {
        Self {
            config: Mutex::new(config),
            write_lock: Mutex::new(()),
            store,
            engine,
            app,
            load_warning: Mutex::new(load_warning),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let config = self.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let autostart = self
            .app
            .autolaunch()
            .is_enabled()
            .unwrap_or(config.settings.launch_at_startup);
        let config_path = self.store.path().to_string_lossy().to_string();
        let config_dir = self.store.dir().to_string_lossy().to_string();

        Snapshot {
            status: AppStatus {
                hooks_active: self.engine.hooks_active(),
                capture_mode: self.engine.capture_mode().name().to_string(),
                elevated: elevation::is_elevated(),
                autostart,
                config_path,
                config_dir,
                version: self.app.package_info().version.to_string(),
                platform: format!("Windows · {}", std::env::consts::ARCH),
                load_warning: self
                    .load_warning
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone(),
                hook_warning: self.engine.hook_warning(),
                last_error: self.engine.last_error(),
            },
            config,
        }
    }

    pub fn cfg(&self) -> AppConfig {
        self.config.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn commit(&self, mut config: AppConfig, apply_theme: bool) -> Result<Snapshot, String> {
        let _write = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        config.sanitize();
        self.store.save(&config)?;

        {
            let mut current = self.config.lock().unwrap_or_else(|e| e.into_inner());
            *current = config.clone();
        }

        self.engine.reconfigure(&config);
        if apply_theme {
            apply_window_theme(&self.app, config.settings.theme);
        }
        if let Err(error) = tray::rebuild(&self.app, &config) {
            eprintln!("[MouseMacro] Could not refresh the tray menu: {error}");
        }
        let _ = self.app.emit("state:changed", ());
        Ok(self.snapshot())
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        config.settings.enabled = enabled;
        self.commit(config, false)
    }

    pub fn set_capture_mode(&self, mode: CaptureMode) -> Result<Snapshot, String> {
        self.engine.set_capture_mode(mode)?;
        Ok(self.snapshot())
    }

    pub fn set_active_profile(&self, profile_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        if !config.profiles.iter().any(|p| p.id == profile_id) {
            return Err("That profile no longer exists.".into());
        }
        config.active_profile_id = profile_id.to_string();
        self.commit(config, false)
    }

    pub fn update_settings(&self, mut settings: Settings) -> Result<Snapshot, String> {
        if !(100..=2_000).contains(&settings.long_press_ms) {
            return Err("Long-press timing must be between 100 and 2000 ms.".into());
        }
        if !(120..=1_000).contains(&settings.double_click_ms) {
            return Err("Double-click timing must be between 120 and 1000 ms.".into());
        }
        settings.sanitize();

        let mut config = self.cfg();
        let want_autostart = settings.launch_at_startup;
        let registry = self.app.autolaunch();
        let current_registry = registry.is_enabled().unwrap_or(false);
        let changed_registry = current_registry != want_autostart;
        if changed_registry {
            let result = if want_autostart {
                registry.enable()
            } else {
                registry.disable()
            };
            if let Err(error) = result {
                return Err(format!(
                    "Windows startup could not be updated: {error}"
                ));
            }
        }

        config.settings = settings;
        match self.commit(config, true) {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                if changed_registry {
                    let _ = if want_autostart {
                        registry.disable()
                    } else {
                        registry.enable()
                    };
                }
                Err(error)
            }
        }
    }

    pub fn set_launch_at_startup(&self, enabled: bool) -> Result<Snapshot, String> {
        let mut settings = self.cfg().settings;
        settings.launch_at_startup = enabled;
        self.update_settings(settings)
    }

    pub fn create_profile(&self, name: &str) -> Result<Snapshot, String> {
        let name = validate_profile_name(name)?;
        let mut config = self.cfg();
        ensure_unique_name(&config, &name, None)?;
        let mut profile = Profile::new(name);
        profile.description = String::new();
        config.profiles.push(profile);
        self.commit(config, false)
    }

    pub fn rename_profile(&self, profile_id: &str, name: &str) -> Result<Snapshot, String> {
        let name = validate_profile_name(name)?;
        let mut config = self.cfg();
        ensure_unique_name(&config, &name, Some(profile_id))?;
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        profile.name = name;
        self.commit(config, false)
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        if config.profiles.len() <= 1 {
            return Err("At least one profile is required.".into());
        }
        let index = config
            .profiles
            .iter()
            .position(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        config.profiles.remove(index);
        if config.active_profile_id == profile_id {
            config.active_profile_id = config.profiles[0].id.clone();
        }
        self.commit(config, false)
    }

    pub fn duplicate_profile(&self, profile_id: &str) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let source = config
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let mut name = format!("{} copy", source.name);
        let mut counter = 2;
        while config
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(&name))
        {
            name = format!("{} copy {}", source.name, counter);
            counter += 1;
        }
        let mut duplicate = Profile::new(name);
        duplicate.description = source.description.clone();
        duplicate.mappings = source
            .mappings
            .iter()
            .cloned()
            .map(|mut mapping| {
                mapping.id = uuid::Uuid::new_v4().to_string();
                mapping
            })
            .collect();
        config.profiles.push(duplicate);
        self.commit(config, false)
    }

    pub fn save_mapping(
        &self,
        profile_id: &str,
        mut mapping: Mapping,
    ) -> Result<Snapshot, String> {
        validate_mapping(&mut mapping)?;
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        if mapping.id.trim().is_empty() {
            mapping.id = uuid::Uuid::new_v4().to_string();
        }
        if let Some(existing) = profile
            .mappings
            .iter_mut()
            .find(|existing| existing.id == mapping.id)
        {
            *existing = mapping;
        } else {
            if profile.mappings.len() >= 500 {
                return Err("This profile already has 500 mappings.".into());
            }
            profile.mappings.push(mapping);
        }
        self.commit(config, false)
    }

    pub fn delete_mapping(
        &self,
        profile_id: &str,
        mapping_id: &str,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let before = profile.mappings.len();
        profile.mappings.retain(|m| m.id != mapping_id);
        if profile.mappings.len() == before {
            return Err("That mapping no longer exists.".into());
        }
        self.commit(config, false)
    }

    pub fn set_mapping_enabled(
        &self,
        profile_id: &str,
        mapping_id: &str,
        enabled: bool,
    ) -> Result<Snapshot, String> {
        let mut config = self.cfg();
        let profile = config
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        let mapping = profile
            .mappings
            .iter_mut()
            .find(|m| m.id == mapping_id)
            .ok_or_else(|| "That mapping no longer exists.".to_string())?;
        mapping.enabled = enabled;
        self.commit(config, false)
    }

    pub fn import_profile(&self, path: &std::path::Path) -> Result<Snapshot, String> {
        let mut imported = self.store.read_profile(path)?;
        imported.id = uuid::Uuid::new_v4().to_string();
        for mapping in &mut imported.mappings {
            mapping.id = uuid::Uuid::new_v4().to_string();
            // Imported files may contain incomplete or unknown mappings. Keep
            // them visible but inert so the user can repair them.
            if validate_mapping(mapping).is_err() {
                mapping.enabled = false;
                mapping.action = Action::Disable;
            }
        }

        let mut config = self.cfg();
        let mut name = imported.name.trim().chars().take(60).collect::<String>();
        if name.is_empty() {
            name = "Imported profile".into();
        }
        let base = name.clone();
        let mut counter = 2;
        while config
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(&name))
        {
            name = format!("{base} {counter}");
            counter += 1;
        }
        imported.name = name;
        config.active_profile_id = imported.id.clone();
        config.profiles.push(imported);
        self.commit(config, false)
    }

    pub fn export_profile(
        &self,
        profile_id: &str,
        path: &std::path::Path,
    ) -> Result<String, String> {
        let config = self.cfg();
        let profile = config
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| "That profile no longer exists.".to_string())?;
        self.store.write_profile(profile, path)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn reset(&self) -> Result<Snapshot, String> {
        let registry = self.app.autolaunch();
        let _ = registry.disable();
        let config = AppConfig::default();
        *self
            .load_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.engine.clear_errors();
        let snapshot = self.commit(config, true)?;
        self.engine.set_capture_mode(CaptureMode::None)?;
        Ok(snapshot)
    }

    pub fn clear_warnings(&self) -> Snapshot {
        *self
            .load_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.engine.clear_errors();
        self.snapshot()
    }

    pub fn show_window(&self) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    pub fn show_window_if_needed(&self) {
        if !self.cfg().settings.start_minimized {
            self.show_window();
        }
    }
}

fn validate_profile_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the profile a name.".into());
    }
    if name.chars().count() > 60 {
        return Err("Profile names can be at most 60 characters.".into());
    }
    Ok(name.to_string())
}

fn ensure_unique_name(
    config: &AppConfig,
    name: &str,
    except_id: Option<&str>,
) -> Result<(), String> {
    if config.profiles.iter().any(|profile| {
        profile.name.eq_ignore_ascii_case(name) && Some(profile.id.as_str()) != except_id
    }) {
        return Err(format!("A profile named “{name}” already exists."));
    }
    Ok(())
}

pub fn apply_window_theme(app: &AppHandle, theme: Theme) {
    let tauri_theme = match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_theme(tauri_theme);
    }
}

/// Validates a mapping and normalizes its key tokens in place.
pub fn validate_mapping(mapping: &mut Mapping) -> Result<(), String> {
    mapping.name = mapping
        .name
        .as_ref()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    if mapping.name.as_ref().is_some_and(|name| name.chars().count() > 60) {
        return Err("Mapping labels can be at most 60 characters.".into());
    }
    if let Trigger::WithButton { button } = &mapping.trigger {
        if *button == mapping.button {
            return Err("A button combination needs a second, different button.".into());
        }
    }

    match &mut mapping.action {
        Action::Shortcut { keys } | Action::HoldKeys { keys } => {
            *keys = normalize_combo(keys);
            if keys.is_empty() {
                return Err("Choose at least one key for this action.".into());
            }
            if keys.len() > 8 {
                return Err("A shortcut can contain at most 8 keys.".into());
            }
            if let Some(unknown) = keys.iter().find(|key| vk_from_name(key).is_none()) {
                return Err(format!("Unknown key “{unknown}”."));
            }
        }
        Action::Key { key } => {
            let normalized = key.trim().to_lowercase();
            if vk_from_name(&normalized).is_none() {
                return Err(format!("Unknown key “{key}”."));
            }
            *key = normalized;
        }
        Action::Scroll { amount, .. } => {
            if !(1..=50).contains(amount) {
                return Err("Scroll amount must be between 1 and 50.".into());
            }
        }
        Action::MouseClick { .. }
        | Action::MouseHold { .. }
        | Action::Disable
        | Action::None => {}
    }
    Ok(())
}
