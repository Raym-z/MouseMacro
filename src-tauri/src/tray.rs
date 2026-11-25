use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::model::AppConfig;
use crate::state::AppState;

fn menu_for(app: &AppHandle, config: &AppConfig) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Open MouseMacro", true, None::<&str>)?;
    let enabled = CheckMenuItem::with_id(
        app,
        "enabled",
        "Enable mappings",
        true,
        config.settings.enabled,
        None::<&str>,
    )?;
    let profiles = Submenu::with_id(app, "profiles", "Profile", true)?;
    for profile in &config.profiles {
        let item = CheckMenuItem::with_id(
            app,
            format!("profile:{}", profile.id),
            &profile.name,
            true,
            profile.id == config.active_profile_id,
            None::<&str>,
        )?;
        profiles.append(&item)?;
    }
    let panic = MenuItem::with_id(app, "panic", "Stop all input", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let version = MenuItem::with_id(
        app,
        "version",
        format!("MouseMacro {}", app.package_info().version),
        false,
        None::<&str>,
    )?;
    let exit = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;

    let menu = Menu::new(app)?;
    menu.append(&open)?;
    menu.append(&enabled)?;
    menu.append(&profiles)?;
    menu.append(&panic)?;
    menu.append(&settings)?;
    menu.append(&separator)?;
    menu.append(&version)?;
    menu.append(&exit)?;
    Ok(menu)
}

fn tooltip(config: &AppConfig) -> String {
    let profile = config
        .active_profile()
        .map(|p| p.name.as_str())
        .unwrap_or("No profile");
    format!(
        "MouseMacro — {} · {}",
        if config.settings.enabled {
            "Mappings on"
        } else {
            "Mappings off"
        },
        profile
    )
}

pub fn create(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let menu = menu_for(app, config)?;
    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip(tooltip(config))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    state.show_window();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn rebuild(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let menu = menu_for(app, config)?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(&tooltip(config)))?;
    }
    Ok(())
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    match id {
        "open" => state.show_window(),
        "settings" => {
            state.show_window();
            let _ = app.emit("app://navigate", "settings");
        }
        "enabled" => {
            let enabled = !state.cfg().settings.enabled;
            report(app, state.set_enabled(enabled).map(|_| ()));
        }
        "panic" => {
            state.engine.panic();
        }
        "exit" => {
            app.exit(0);
        }
        "version" => {}
        other if other.starts_with("profile:") => {
            let profile_id = other.trim_start_matches("profile:");
            report(app, state.set_active_profile(profile_id).map(|_| ()));
        }
        _ => {}
    }
}

fn report(app: &AppHandle, result: Result<(), String>) {
    if let Err(error) = result {
        eprintln!("[MouseMacro] {error}");
        let _ = app.emit("app://error", error);
    }
}
