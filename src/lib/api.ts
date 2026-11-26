import { invoke } from "@tauri-apps/api/core";
import type {
  Action,
  BackupInfo,
  CaptureEvent,
  CaptureKind,
  CaptureMode,
  CaptureSessionInfo,
  ExceptionRule,
  InputSample,
  KeyMapping,
  Layer,
  Mapping,
  InstalledAppsScan,
  MappingActivity,
  OpenApp,
  Settings,
  Snapshot,
} from "./types";

export const api = {
  getSnapshot: () => invoke<Snapshot>("get_snapshot"),
  appReady: () => invoke<Snapshot>("app_ready"),
  setEnabled: (enabled: boolean) =>
    invoke<Snapshot>("set_enabled", { enabled }),
  updateSettings: (settings: Settings) =>
    invoke<Snapshot>("update_settings", { settings }),
  setLaunchAtStartup: (enabled: boolean) =>
    invoke<Snapshot>("set_launch_at_startup", { enabled }),
  setActiveProfile: (profileId: string) =>
    invoke<Snapshot>("set_active_profile", { profileId }),
  createProfile: (name: string) =>
    invoke<Snapshot>("create_profile", { name }),
  renameProfile: (profileId: string, name: string) =>
    invoke<Snapshot>("rename_profile", { profileId, name }),
  deleteProfile: (profileId: string) =>
    invoke<Snapshot>("delete_profile", { profileId }),
  duplicateProfile: (profileId: string) =>
    invoke<Snapshot>("duplicate_profile", { profileId }),
  saveMapping: (profileId: string, mapping: Mapping) =>
    invoke<Snapshot>("save_mapping", { profileId, mapping }),
  deleteMapping: (profileId: string, mappingId: string) =>
    invoke<Snapshot>("delete_mapping", { profileId, mappingId }),
  setMappingEnabled: (
    profileId: string,
    mappingId: string,
    enabled: boolean,
  ) =>
    invoke<Snapshot>("set_mapping_enabled", {
      profileId,
      mappingId,
      enabled,
    }),
  duplicateMapping: (profileId: string, mappingId: string) =>
    invoke<Snapshot>("duplicate_mapping", { profileId, mappingId }),
  saveKeyMapping: (profileId: string, mapping: KeyMapping) =>
    invoke<Snapshot>("save_key_mapping", { profileId, mapping }),
  deleteKeyMapping: (profileId: string, mappingId: string) =>
    invoke<Snapshot>("delete_key_mapping", { profileId, mappingId }),
  setKeyMappingEnabled: (
    profileId: string,
    mappingId: string,
    enabled: boolean,
  ) =>
    invoke<Snapshot>("set_key_mapping_enabled", {
      profileId,
      mappingId,
      enabled,
    }),
  setMappingsEnabled: (
    profileId: string,
    mappingIds: string[],
    enabled: boolean,
  ) =>
    invoke<Snapshot>("set_mappings_enabled", {
      profileId,
      mappingIds,
      enabled,
    }),
  saveLayer: (profileId: string, layer: Layer) =>
    invoke<Snapshot>("save_layer", { profileId, layer }),
  deleteLayer: (profileId: string, layerId: string) =>
    invoke<Snapshot>("delete_layer", { profileId, layerId }),
  setLayerEnabled: (profileId: string, layerId: string, enabled: boolean) =>
    invoke<Snapshot>("set_layer_enabled", {
      profileId,
      layerId,
      enabled,
    }),
  saveLayerMapping: (profileId: string, layerId: string, mapping: Mapping) =>
    invoke<Snapshot>("save_layer_mapping", {
      profileId,
      layerId,
      mapping,
    }),
  deleteLayerMapping: (
    profileId: string,
    layerId: string,
    mappingId: string,
  ) =>
    invoke<Snapshot>("delete_layer_mapping", {
      profileId,
      layerId,
      mappingId,
    }),
  setLayerMappingEnabled: (
    profileId: string,
    layerId: string,
    mappingId: string,
    enabled: boolean,
  ) =>
    invoke<Snapshot>("set_layer_mapping_enabled", {
      profileId,
      layerId,
      mappingId,
      enabled,
    }),
  saveException: (rule: ExceptionRule) =>
    invoke<Snapshot>("save_exception", { rule }),
  deleteException: (ruleId: string) =>
    invoke<Snapshot>("delete_exception", { ruleId }),
  setExceptionEnabled: (ruleId: string, enabled: boolean) =>
    invoke<Snapshot>("set_exception_enabled", { ruleId, enabled }),
  listOpenApps: () => invoke<OpenApp[]>("list_open_apps"),
  scanInstalledApps: () =>
    invoke<InstalledAppsScan>("scan_installed_apps"),
  testAction: (action: Action) => invoke<void>("test_action", { action }),
  stopMacros: () => invoke<void>("stop_macros"),
  getMappingActivity: () =>
    invoke<MappingActivity[]>("get_mapping_activity"),
  clearMappingActivity: () => invoke<void>("clear_mapping_activity"),
  emergencyStop: () => invoke<Snapshot>("emergency_stop"),
  exportConfig: (path: string) =>
    invoke<string>("export_config", { path }),
  importConfig: (path: string) =>
    invoke<Snapshot>("import_config", { path }),
  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  createBackup: () => invoke<BackupInfo>("create_backup"),
  restoreBackup: (name: string) =>
    invoke<Snapshot>("restore_backup", { name }),
  importProfile: (path: string, allowLaunch: boolean) =>
    invoke<Snapshot>("import_profile", { path, allowLaunch }),
  inspectProfileImport: (path: string) =>
    invoke<string[]>("inspect_profile_import", { path }),
  exportProfile: (profileId: string, path: string) =>
    invoke<string>("export_profile", { profileId, path }),
  getWheelLines: () => invoke<number>("get_wheel_lines"),
  resetConfig: () => invoke<Snapshot>("reset_config"),
  reloadConfig: () => invoke<Snapshot>("reload_config"),
  setCaptureMode: (mode: CaptureMode) =>
    invoke<Snapshot>("set_capture_mode", { mode }),
  startCapture: (kind: CaptureKind) =>
    invoke<CaptureSessionInfo>("start_capture", { kind }),
  stopCapture: (sessionId: number) =>
    invoke<void>("stop_capture", { sessionId }),
  getCaptureEvents: (sessionId: number, sinceSeq: number) =>
    invoke<CaptureEvent[]>("get_capture_events", { sessionId, sinceSeq }),
  activeCaptureSession: () => invoke<number | null>("active_capture_session"),
  releaseDragLocks: () => invoke<Snapshot>("release_drag_locks"),
  reinstallHooks: () => invoke<Snapshot>("reinstall_input_hooks"),
  clearWarnings: () => invoke<Snapshot>("clear_warnings"),
  openConfigFolder: () => invoke<void>("open_config_folder"),
  relaunchElevated: () => invoke<void>("relaunch_elevated"),
  getInputDiagnostics: () => invoke<InputSample[]>("get_input_diagnostics"),
  clearInputDiagnostics: () => invoke<void>("clear_input_diagnostics"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "Something went wrong.";
}
