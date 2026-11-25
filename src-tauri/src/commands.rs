use std::path::PathBuf;

use tauri::State;

use crate::config::model::{Mapping, Settings};
use crate::platform::elevation;
use crate::platform::service::CaptureMode;
use crate::state::{AppState, Snapshot};

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snapshot()
}

#[tauri::command]
pub fn app_ready(state: State<'_, AppState>) -> Snapshot {
    let snapshot = state.snapshot();
    state.show_window_if_needed();
    snapshot
}

#[tauri::command]
pub fn set_enabled(state: State<'_, AppState>, enabled: bool) -> Result<Snapshot, String> {
    state.set_enabled(enabled)
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Snapshot, String> {
    state.update_settings(settings)
}

#[tauri::command]
pub fn set_launch_at_startup(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_launch_at_startup(enabled)
}

#[tauri::command]
pub fn set_active_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Snapshot, String> {
    state.set_active_profile(&profile_id)
}

#[tauri::command]
pub fn create_profile(state: State<'_, AppState>, name: String) -> Result<Snapshot, String> {
    state.create_profile(&name)
}

#[tauri::command]
pub fn rename_profile(
    state: State<'_, AppState>,
    profile_id: String,
    name: String,
) -> Result<Snapshot, String> {
    state.rename_profile(&profile_id, &name)
}

#[tauri::command]
pub fn delete_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Snapshot, String> {
    state.delete_profile(&profile_id)
}

#[tauri::command]
pub fn duplicate_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Snapshot, String> {
    state.duplicate_profile(&profile_id)
}

#[tauri::command]
pub fn save_mapping(
    state: State<'_, AppState>,
    profile_id: String,
    mapping: Mapping,
) -> Result<Snapshot, String> {
    state.save_mapping(&profile_id, mapping)
}

#[tauri::command]
pub fn delete_mapping(
    state: State<'_, AppState>,
    profile_id: String,
    mapping_id: String,
) -> Result<Snapshot, String> {
    state.delete_mapping(&profile_id, &mapping_id)
}

#[tauri::command]
pub fn set_mapping_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    mapping_id: String,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_mapping_enabled(&profile_id, &mapping_id, enabled)
}

#[tauri::command]
pub fn import_profile(state: State<'_, AppState>, path: String) -> Result<Snapshot, String> {
    state.import_profile(&PathBuf::from(path))
}

#[tauri::command]
pub fn export_profile(
    state: State<'_, AppState>,
    profile_id: String,
    path: String,
) -> Result<String, String> {
    state.export_profile(&profile_id, &PathBuf::from(path))
}

#[tauri::command]
pub fn reset_config(state: State<'_, AppState>) -> Result<Snapshot, String> {
    state.reset()
}

#[tauri::command]
pub fn set_capture_mode(state: State<'_, AppState>, mode: String) -> Result<Snapshot, String> {
    state.set_capture_mode(CaptureMode::from_str(&mode))
}

#[tauri::command]
pub fn clear_warnings(state: State<'_, AppState>) -> Snapshot {
    state.clear_warnings()
}

#[tauri::command]
pub fn open_config_folder(state: State<'_, AppState>) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(state.store.dir())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the configuration folder: {error}"))
}

#[tauri::command]
pub fn relaunch_elevated(state: State<'_, AppState>) -> Result<(), String> {
    if elevation::is_elevated() {
        return Err("MouseMacro is already running as administrator.".into());
    }
    elevation::relaunch_elevated()?;
    state.app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn dismiss_banner(state: State<'_, AppState>) -> Snapshot {
    state.clear_warnings()
}

/// Development diagnostic: verifies that the low-level hooks are receiving
/// events, independently of whether those events are consumed.
#[tauri::command]
pub fn debug_hook_stats() -> (u64, u64, i64, i64) {
    crate::platform::hooks::callback_stats()
}
