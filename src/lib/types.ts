export type MouseButton = "left" | "right" | "middle" | "x1" | "x2";
export type ScrollDirection = "up" | "down" | "left" | "right";

export type Trigger =
  | { type: "singleClick" }
  | { type: "doubleClick" }
  | { type: "longPress" }
  | { type: "release" }
  | { type: "withButton"; button: MouseButton }
  | { type: "withWheel"; direction: ScrollDirection }
  /** A bare wheel notch, with no button held. */
  | { type: "wheel"; direction: ScrollDirection };

export type Action =
  | { type: "shortcut"; keys: string[] }
  | { type: "key"; key: string }
  | { type: "holdKeys"; keys: string[] }
  | { type: "mouseClick"; button: MouseButton }
  | { type: "mouseHold"; button: MouseButton }
  | { type: "scroll"; direction: ScrollDirection; amount: number }
  | {
      type: "macro";
      steps: MacroStep[];
      repeat: number;
      intervalMs: number;
      toggle: boolean;
    }
  | { type: "launch"; target: string }
  | { type: "typeText"; text: string }
  | { type: "media"; key: string }
  | { type: "window"; action: WindowAction }
  | { type: "toggleKeys"; keys: string[] }
  | { type: "dragLock"; button: MouseButton }
  | { type: "disable" }
  | { type: "none" };

export type WindowAction =
  | "minimize"
  | "maximize"
  | "restore"
  | "close"
  | "snapLeft"
  | "snapRight";

export type MacroStep =
  | { type: "tapKeys"; keys: string[]; delayMs: number }
  | { type: "holdKeys"; keys: string[]; delayMs: number }
  | { type: "releaseKeys"; keys: string[]; delayMs: number }
  | { type: "wait"; delayMs: number }
  | { type: "mouseClick"; button: MouseButton; delayMs: number }
  | { type: "mouseHold"; button: MouseButton; delayMs: number }
  | { type: "mouseRelease"; button: MouseButton; delayMs: number }
  | {
      type: "scroll";
      direction: ScrollDirection;
      amount: number;
      delayMs: number;
    }
  | { type: "typeText"; text: string; delayMs: number };

export type MacroStepType = MacroStep["type"];

export interface WheelOptions {
  /** Ignore further notches for this long after firing. */
  cooldownMs: number;
  /** Run the action and still send the original scroll to the app. */
  alsoScroll: boolean;
}

export interface Mapping {
  id: string;
  name: string | null;
  /** `null` for standalone wheel triggers, which have no button. */
  button: MouseButton | null;
  trigger: Trigger;
  action: Action;
  enabled: boolean;
  /** Only meaningful for a `wheel` trigger; `null` uses the defaults. */
  wheel: WheelOptions | null;
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
  keyMappings: KeyMapping[];
}

export type ExceptionMode = "always" | "disable" | "profile";

export interface ScrollSettings {
  invertVertical: boolean;
  invertHorizontal: boolean;
  swapAxes: boolean;
  /** Wheel notches sent per physical notch, 1–5. */
  speed: number;
  /** Experimental time-based acceleration. */
  acceleration: boolean;
}

export interface ExceptionRule {
  id: string;
  process: string;
  mode: ExceptionMode;
  profileId: string | null;
  enabled: boolean;
  /** Optional wheel override; null inherits the global setting. */
  scroll: ScrollSettings | null;
  /** Pass keyboard input through untouched in this application. */
  keyboardDisabled: boolean;
}

export type RuleCategory =
  | "gesture"
  | "longPress"
  | "doubleClick"
  | "release"
  | "singleClick";

export type Theme = "system" | "light" | "dark";

export interface KeyboardSettings {
  /** Opt-in always-on keyboard hook. */
  enabled: boolean;
  /** The user has seen the hook explanation. */
  acknowledged: boolean;
}

export type KeyBehaviour =
  | { mode: "remap"; target: string[] }
  | { mode: "action"; action: Action };

export interface KeyMapping {
  id: string;
  name: string | null;
  /** Canonical tokens, modifiers first, exactly one non-modifier key. */
  source: string[];
  behaviour: KeyBehaviour;
  enabled: boolean;
}

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
  scroll: ScrollSettings;
  keyboard: KeyboardSettings;
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
  | "keyboard"
  | "scroll"
  | "settings"
  | "about";

export const MOUSE_BUTTONS: MouseButton[] = [
  "left",
  "right",
  "middle",
  "x1",
  "x2",
];

export const MEDIA_KEYS: Array<{ value: string; label: string }> = [
  { value: "mediaplaypause", label: "Play / Pause" },
  { value: "medianext", label: "Next track" },
  { value: "mediaprev", label: "Previous track" },
  { value: "mediastop", label: "Stop" },
  { value: "volumeup", label: "Volume up" },
  { value: "volumedown", label: "Volume down" },
  { value: "volumemute", label: "Mute" },
];

export const WINDOW_ACTIONS: Array<{ value: WindowAction; label: string }> = [
  { value: "minimize", label: "Minimize" },
  { value: "maximize", label: "Maximize" },
  { value: "restore", label: "Restore" },
  { value: "close", label: "Close" },
  { value: "snapLeft", label: "Snap left" },
  { value: "snapRight", label: "Snap right" },
];

export function defaultScroll(): ScrollSettings {
  return {
    invertVertical: false,
    invertHorizontal: false,
    swapAxes: false,
    speed: 1,
    acceleration: false,
  };
}

export function newMacroStep(type: MacroStepType): MacroStep {
  switch (type) {
    case "tapKeys":
    case "holdKeys":
    case "releaseKeys":
      return { type, keys: [], delayMs: 60 };
    case "wait":
      return { type, delayMs: 250 };
    case "mouseClick":
    case "mouseHold":
    case "mouseRelease":
      return { type, button: "left", delayMs: 60 };
    case "scroll":
      return { type, direction: "down", amount: 3, delayMs: 60 };
    case "typeText":
      return { type, text: "", delayMs: 60 };
  }
}

export const DEFAULT_ACTION: Action = { type: "shortcut", keys: [] };

export function newKeyMapping(): KeyMapping {
  return {
    id: "",
    name: null,
    source: [],
    behaviour: { mode: "remap", target: [] },
    enabled: true,
  };
}

export function newLayer(activator: MouseButton = "x2"): Layer {
  return {
    id: "",
    name: "",
    activator,
    enabled: true,
    mappings: [],
  };
}

export function newMapping(button: MouseButton | null = "x2"): Mapping {  return {
    id: "",
    name: null,
    button,
    trigger:
      button === null
        ? { type: "wheel", direction: "up" }
        : { type: "singleClick" },
    action: { type: "shortcut", keys: [] },
    enabled: true,
    wheel:
      button === null ? { cooldownMs: 80, alsoScroll: false } : null,
  };
}
