mod commands;
mod config;
mod engine;
mod platform;
mod state;
mod tray;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

use crate::config::store::ConfigStore;
use crate::platform::hooks;
use crate::platform::service::EngineService;
use crate::state::{apply_window_theme, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(state) = app.try_state::<AppState>() {
                state.show_window();
            } else if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = handle
                .path()
                .app_config_dir()
                .map_err(|error| format!("Could not locate the config folder: {error}"))?;
            let store = ConfigStore::new(config_dir)?;
            let loaded = store.load();
            let mut config = loaded.config;

            // The registry is the source of truth for startup integration.
            let autostart = handle.autolaunch().is_enabled().unwrap_or(false);
            if config.settings.launch_at_startup != autostart {
                config.settings.launch_at_startup = autostart;
                let _ = store.save(&config);
            }

            let engine = EngineService::new(handle.clone(), &config);
            app.manage(AppState::new(
                handle.clone(),
                config.clone(),
                store,
                engine.clone(),
                loaded.warning,
            ));

            // Fit the initial window inside the monitor's usable area. Tauri
            // centers using the full monitor size, which can overlap the
            // Windows taskbar on scaled displays.
            if let Some(window) = handle.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let scale = monitor.scale_factor();
                    let size = monitor.size();
                    let available_width = size.width as f64 / scale;
                    let available_height = size.height as f64 / scale;
                    let width = 1200.0_f64
                        .min((available_width - 64.0).max(980.0));
                    let height = 800.0_f64
                        .min((available_height - 112.0).max(620.0));
                    let _ = window.set_size(tauri::LogicalSize::new(width, height));
                    let _ = window.center();
                }
            }

            apply_window_theme(&handle, config.settings.theme);
            if let Err(error) = tray::create(&handle, &config) {
                eprintln!("[MouseMacro] Could not create the tray icon: {error}");
            }
            if let Err(error) = hooks::start(engine.clone()) {
                eprintln!("[MouseMacro] {error}");
                engine.set_hook_warning(Some(format!(
                    "Global input detection could not start: {error}"
                )));
            }
            crate::platform::foreground::start(engine.clone());
            crate::platform::hotkey::start(
                handle.clone(),
                config.settings.panic_hotkey.clone(),
            );

            // Safety net: if the webview never calls `app_ready`, still reveal
            // the window so the app cannot become invisible.
            let fallback = handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                if let Some(state) = fallback.try_state::<AppState>() {
                    if !state.cfg().settings.start_minimized {
                        if let Some(window) = fallback.get_webview_window("main") {
                            if !window.is_visible().unwrap_or(false) {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .try_state::<AppState>()
                    .map(|state| state.cfg().settings.close_to_tray)
                    .unwrap_or(false);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::app_ready,
            commands::get_wheel_lines,
            commands::set_enabled,
            commands::update_settings,
            commands::set_launch_at_startup,
            commands::set_active_profile,
            commands::create_profile,
            commands::rename_profile,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::save_mapping,
            commands::delete_mapping,
            commands::set_mapping_enabled,
            commands::save_key_mapping,
            commands::delete_key_mapping,
            commands::set_key_mapping_enabled,
            commands::duplicate_mapping,
            commands::set_mappings_enabled,
            commands::save_layer,
            commands::delete_layer,
            commands::set_layer_enabled,
            commands::save_layer_mapping,
            commands::delete_layer_mapping,
            commands::set_layer_mapping_enabled,
            commands::save_exception,
            commands::delete_exception,
            commands::set_exception_enabled,
            commands::list_open_apps,
            commands::test_action,
            commands::stop_macros,
            commands::get_mapping_activity,
            commands::clear_mapping_activity,
            commands::emergency_stop,
            commands::inspect_profile_import,
            commands::import_profile,
            commands::export_profile,
            commands::reset_config,
            commands::reload_config,
            commands::export_config,
            commands::import_config,
            commands::list_backups,
            commands::create_backup,
            commands::restore_backup,
            commands::set_capture_mode,
            commands::clear_warnings,
            commands::dismiss_banner,
            commands::open_config_folder,
            commands::relaunch_elevated,
            commands::debug_hook_stats,
            commands::get_input_diagnostics,
            commands::clear_input_diagnostics,
        ]);

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building MouseMacro");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<AppState>() {
                // Never leave a synthetic modifier or mouse button held down.
                state.engine.release_all();
            }
            hooks::stop();
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
    });
}
