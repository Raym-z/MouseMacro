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

export interface Profile {
  id: string;
  name: string;
  description: string;
  mappings: Mapping[];
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

export type View = "dashboard" | "mappings" | "profiles" | "settings" | "about";

export const MOUSE_BUTTONS: MouseButton[] = [
  "left",
  "right",
  "middle",
  "x1",
  "x2",
];

export const DEFAULT_ACTION: Action = { type: "shortcut", keys: [] };

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
