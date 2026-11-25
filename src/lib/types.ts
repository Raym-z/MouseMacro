export type MouseButton = "left" | "right" | "middle" | "x1" | "x2";
export type ScrollDirection = "up" | "down" | "left" | "right";

export type Trigger =
  | { type: "singleClick" }
  | { type: "doubleClick" }
  | { type: "longPress" }
  | { type: "release" }
  | { type: "withButton"; button: MouseButton }
  | { type: "withWheel"; direction: ScrollDirection };

export type Action =
  | { type: "shortcut"; keys: string[] }
  | { type: "key"; key: string }
  | { type: "holdKeys"; keys: string[] }
  | { type: "mouseClick"; button: MouseButton }
  | { type: "mouseHold"; button: MouseButton }
  | { type: "scroll"; direction: ScrollDirection; amount: number }
  | { type: "disable" }
  | { type: "none" };

export interface Mapping {
  id: string;
  name: string | null;
  button: MouseButton;
  trigger: Trigger;
  action: Action;
  enabled: boolean;
}

export interface Layer {
  id: string;
  name: string;
  activator: MouseButton;
  enabled: boolean;
  mappings: Mapping[];
}

export interface Profile {
  id: string;
  name: string;
  description: string;
  mappings: Mapping[];
  layers: Layer[];
}

export type ExceptionMode = "always" | "disable" | "profile";

export interface ExceptionRule {
  id: string;
  process: string;
  mode: ExceptionMode;
  profileId: string | null;
  enabled: boolean;
}

export type RuleCategory =
  | "gesture"
  | "longPress"
  | "doubleClick"
  | "release"
  | "singleClick";

export type Theme = "system" | "light" | "dark";

export interface Settings {
  enabled: boolean;
  launchAtStartup: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  theme: Theme;
  longPressMs: number;
  doubleClickMs: number;
  /** Accept mouse/keyboard events synthesized by other input tools. */
  processInjectedInput: boolean;
  rulePriority: RuleCategory[];
  /** Emergency stop combo. Empty disables the hotkey. */
  panicHotkey: string[];
  /** False on a fresh install; the guided setup sets it to true. */
  onboarded: boolean;
}

export interface InputSample {
  ageMs: number;
  kind: "wheel" | "down" | "up";
  direction: string | null;
  button: string | null;
  axis: "vertical" | "horizontal" | null;
  delta: number;
  injected: boolean;
  ownInjection: boolean;
  accepted: boolean;
  matched: boolean;
}

export interface AppConfig {
  schemaVersion: number;
  settings: Settings;
  profiles: Profile[];
  activeProfileId: string;
  exceptions: ExceptionRule[];
}

export interface MappingActivity {
  mappingId: string;
  ageMs: number;
}

export interface OpenApp {
  process: string;
  title: string;
}

export interface BackupInfo {
  name: string;
  path: string;
  size: number;
  savedAt: number;
}

export interface AppStatus {
  hooksActive: boolean;
  captureMode: "none" | "mouse" | "keyboard" | "monitor";
  elevated: boolean;
  autostart: boolean;
  configPath: string;
  configDir: string;
  version: string;
  platform: string;
  loadWarning: string | null;
  hookWarning: string | null;
  lastError: string | null;
  foregroundApp: string | null;
  effectiveProfileId: string;
  exceptionMode: ExceptionMode;
  panicActive: boolean;
  hotkeyWarning: string | null;
}

export interface Snapshot {
  config: AppConfig;
  status: AppStatus;
}

export type CapturePayload =
  | { kind: "mouse"; event: "down" | "up"; button: MouseButton; label: string }
  | {
      kind: "mouse";
      event: "wheel";
      direction: ScrollDirection;
      label: string;
    }
  | { kind: "keys"; keys: string[] }
  | { kind: "modifiers"; keys: string[] }
  | { kind: "cancel" };

export type CaptureMode = "none" | "mouse" | "keyboard" | "monitor";

export type View =
  | "dashboard"
  | "mappings"
  | "layers"
  | "profiles"
  | "exceptions"
  | "settings"
  | "about";

export const MOUSE_BUTTONS: MouseButton[] = [
  "left",
  "right",
  "middle",
  "x1",
  "x2",
];

export const DEFAULT_ACTION: Action = { type: "shortcut", keys: [] };

export function newLayer(activator: MouseButton = "x2"): Layer {
  return {
    id: "",
    name: "",
    activator,
    enabled: true,
    mappings: [],
  };
}

export function newMapping(button: MouseButton = "x2"): Mapping {
  return {
    id: "",
    name: null,
    button,
    trigger: { type: "singleClick" },
    action: { type: "shortcut", keys: [] },
    enabled: true,
  };
}
