import { invoke } from "@tauri-apps/api/core";
import type { Mapping, CaptureMode, Settings, Snapshot } from "./types";

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
  importProfile: (path: string) =>
    invoke<Snapshot>("import_profile", { path }),
  exportProfile: (profileId: string, path: string) =>
    invoke<string>("export_profile", { profileId, path }),
  resetConfig: () => invoke<Snapshot>("reset_config"),
  setCaptureMode: (mode: CaptureMode) =>
    invoke<Snapshot>("set_capture_mode", { mode }),
  clearWarnings: () => invoke<Snapshot>("clear_warnings"),
  openConfigFolder: () => invoke<void>("open_config_folder"),
  relaunchElevated: () => invoke<void>("relaunch_elevated"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "Something went wrong.";
}
