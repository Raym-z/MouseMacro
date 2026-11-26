use std::path::PathBuf;

use tauri::State;

use crate::config::model::{Action, ExceptionRule, KeyMapping, Layer, Mapping, Settings};
use crate::platform::elevation;
use crate::platform::hooks::{self, InputSample};
use crate::platform::service::CaptureMode;
use crate::state::{AppState, Snapshot};

#[tauri::command]
pub fn get_wheel_lines() -> u32 {
    crate::platform::simulate::wheel_lines()
}

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
pub fn save_key_mapping(
    state: State<'_, AppState>,
    profile_id: String,
    mapping: KeyMapping,
) -> Result<Snapshot, String> {
    state.save_key_mapping(&profile_id, mapping)
}

#[tauri::command]
pub fn delete_key_mapping(
    state: State<'_, AppState>,
    profile_id: String,
    mapping_id: String,
) -> Result<Snapshot, String> {
    state.delete_key_mapping(&profile_id, &mapping_id)
}

#[tauri::command]
pub fn set_key_mapping_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    mapping_id: String,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_key_mapping_enabled(&profile_id, &mapping_id, enabled)
}

#[tauri::command]
pub fn duplicate_mapping(    state: State<'_, AppState>,
    profile_id: String,
    mapping_id: String,
) -> Result<Snapshot, String> {
    state.duplicate_mapping(&profile_id, &mapping_id)
}

#[tauri::command]
pub fn set_mappings_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    mapping_ids: Vec<String>,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_mappings_enabled(&profile_id, mapping_ids, enabled)
}

#[tauri::command]
pub fn save_layer(
    state: State<'_, AppState>,
    profile_id: String,
    layer: Layer,
) -> Result<Snapshot, String> {
    state.save_layer(&profile_id, layer)
}

#[tauri::command]
pub fn delete_layer(
    state: State<'_, AppState>,
    profile_id: String,
    layer_id: String,
) -> Result<Snapshot, String> {
    state.delete_layer(&profile_id, &layer_id)
}

#[tauri::command]
pub fn set_layer_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    layer_id: String,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_layer_enabled(&profile_id, &layer_id, enabled)
}

#[tauri::command]
pub fn save_layer_mapping(
    state: State<'_, AppState>,
    profile_id: String,
    layer_id: String,
    mapping: Mapping,
) -> Result<Snapshot, String> {
    state.save_layer_mapping(&profile_id, &layer_id, mapping)
}

#[tauri::command]
pub fn delete_layer_mapping(
    state: State<'_, AppState>,
    profile_id: String,
    layer_id: String,
    mapping_id: String,
) -> Result<Snapshot, String> {
    state.delete_layer_mapping(&profile_id, &layer_id, &mapping_id)
}

#[tauri::command]
pub fn set_layer_mapping_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    layer_id: String,
    mapping_id: String,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_layer_mapping_enabled(&profile_id, &layer_id, &mapping_id, enabled)
}

#[tauri::command]
pub fn save_exception(
    state: State<'_, AppState>,
    rule: ExceptionRule,
) -> Result<Snapshot, String> {
    state.save_exception(rule)
}

#[tauri::command]
pub fn delete_exception(
    state: State<'_, AppState>,
    rule_id: String,
) -> Result<Snapshot, String> {
    state.delete_exception(&rule_id)
}

#[tauri::command]
pub fn set_exception_enabled(
    state: State<'_, AppState>,
    rule_id: String,
    enabled: bool,
) -> Result<Snapshot, String> {
    state.set_exception_enabled(&rule_id, enabled)
}

#[tauri::command]
pub fn list_open_apps() -> Vec<crate::platform::foreground::OpenApp> {
    crate::platform::foreground::list_open_apps()
}

#[tauri::command]
pub fn test_action(state: State<'_, AppState>, action: Action) -> Result<(), String> {
    state.test_action(action)
}

#[tauri::command]
pub fn stop_macros(state: State<'_, AppState>) {
    state.engine.stop_macros();
}

#[tauri::command]
pub fn get_mapping_activity(
    state: State<'_, AppState>,
) -> Vec<crate::platform::service::MappingActivity> {
    state.engine.activity_snapshot()
}

#[tauri::command]
pub fn clear_mapping_activity(state: State<'_, AppState>) {
    state.engine.clear_activity();
}

#[tauri::command]
pub fn emergency_stop(state: State<'_, AppState>) -> Snapshot {
    state.engine.panic();
    state.snapshot()
}

#[tauri::command]
pub fn inspect_profile_import(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<String>, String> {
    state.inspect_profile_import(&PathBuf::from(path))
}

#[tauri::command]
pub fn import_profile(
    state: State<'_, AppState>,
    path: String,
    allow_launch: bool,
) -> Result<Snapshot, String> {
    state.import_profile(&PathBuf::from(path), allow_launch)
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
pub fn export_config(state: State<'_, AppState>, path: String) -> Result<String, String> {
    state.export_config(&PathBuf::from(path))
}

#[tauri::command]
pub fn import_config(state: State<'_, AppState>, path: String) -> Result<Snapshot, String> {
    state.import_config(&PathBuf::from(path))
}

#[tauri::command]
pub fn list_backups(
    state: State<'_, AppState>,
) -> Vec<crate::config::store::BackupInfo> {
    state.list_backups()
}

#[tauri::command]
pub fn create_backup(
    state: State<'_, AppState>,
) -> Result<crate::config::store::BackupInfo, String> {
    state.create_backup()
}

#[tauri::command]
pub fn restore_backup(
    state: State<'_, AppState>,
    name: String,
) -> Result<Snapshot, String> {
    state.restore_backup(&name)
}

#[tauri::command]
pub fn reload_config(state: State<'_, AppState>) -> Result<Snapshot, String> {
    state.reload()
}

#[tauri::command]
pub fn set_capture_mode(state: State<'_, AppState>, mode: String) -> Result<Snapshot, String> {
    state.set_capture_mode(CaptureMode::from_str(&mode))
}

#[tauri::command]
pub fn start_capture(
    state: State<'_, AppState>,
    kind: String,
) -> Result<crate::platform::capture::CaptureSessionInfo, String> {
    state.start_capture(&kind)
}

#[tauri::command]
pub fn stop_capture(state: State<'_, AppState>, session_id: u64) {
    state.stop_capture(session_id);
}

#[tauri::command]
pub fn get_capture_events(
    state: State<'_, AppState>,
    session_id: u64,
    since_seq: u64,
) -> Vec<crate::platform::capture::CaptureEvent> {
    state.capture_events(session_id, since_seq)
}

/// The session still accepting input. The UI polls this while recording so a
/// missed end event cannot leave it listening to a session that is gone.
#[tauri::command]
pub fn active_capture_session(state: State<'_, AppState>) -> Option<u64> {
    state.active_capture_session()
}

/// Releases toggled drag-lock buttons without pausing the rest of the engine.
#[tauri::command]
pub fn release_drag_locks(state: State<'_, AppState>) -> Snapshot {
    state.release_drag_locks()
}

/// Reinstalls the low-level mouse hook after Windows has dropped it.
#[tauri::command]
pub fn reinstall_input_hooks(state: State<'_, AppState>) -> Result<Snapshot, String> {
    state.reinstall_input_hooks()?;
    Ok(state.snapshot())
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

/// Recent mouse events exactly as the low-level hook saw them, newest first.
/// Includes events that were filtered (synthetic input) so the diagnostics view
/// can explain why a mapping did not fire.
#[tauri::command]
pub fn get_input_diagnostics() -> Vec<InputSample> {
    hooks::recent_samples()
}

#[tauri::command]
pub fn clear_input_diagnostics() {
    hooks::clear_samples()
}
