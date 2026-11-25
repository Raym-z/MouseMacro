import type {
  Action,
  Mapping,
  MouseButton,
  RuleCategory,
  ScrollDirection,
  Trigger,
} from "./types";

const BUTTON_LABEL: Record<MouseButton, string> = {
  left: "Left",
  right: "Right",
  middle: "Middle",
  x1: "B4",
  x2: "B5",
};

const BUTTON_NAME: Record<MouseButton, string> = {
  left: "left button",
  right: "right button",
  middle: "middle button",
  x1: "button 4",
  x2: "button 5",
};

const KEY_DISPLAY: Record<string, string> = {
  ctrl: "Ctrl",
  shift: "Shift",
  alt: "Alt",
  win: "Win",
  esc: "Esc",
  escape: "Esc",
  enter: "Enter",
  return: "Enter",
  tab: "Tab",
  space: "Space",
  backspace: "Backspace",
  delete: "Delete",
  del: "Delete",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "Page Up",
  pagedown: "Page Down",
  up: "Up",
  down: "Down",
  left: "Left",
  right: "Right",
  printscreen: "Print Screen",
  capslock: "Caps Lock",
  numlock: "Num Lock",
  scrolllock: "Scroll Lock",
  apps: "Menu",
  semicolon: ";",
  equal: "=",
  comma: ",",
  minus: "−",
  period: ".",
  slash: "/",
  backslash: "\\",
  bracketleft: "[",
  bracketright: "]",
  quote: "'",
  grave: "`",
  numpadmultiply: "Num *",
  numpadadd: "Num +",
  numpadsubtract: "Num −",
  numpaddecimal: "Num .",
  numpaddivide: "Num /",
  volumemute: "Mute",
  volumedown: "Volume Down",
  volumeup: "Volume Up",
  medianext: "Next Track",
  mediaprev: "Previous Track",
  mediastop: "Stop Media",
  mediaplaypause: "Play / Pause",
  browserback: "Browser Back",
  browserforward: "Browser Forward",
  browserrefresh: "Refresh",
  browserstop: "Stop",
  browserhome: "Browser Home",
};

export function keyLabel(key: string): string {
  const normalized = key.trim().toLowerCase();
  if (KEY_DISPLAY[normalized]) return KEY_DISPLAY[normalized];
  if (normalized.startsWith("numpad") && /^\d$/.test(normalized.slice(6))) {
    return `Num ${normalized.slice(6)}`;
  }
  if (/^f\d{1,2}$/.test(normalized)) return normalized.toUpperCase();
  if (/^vk[0-9a-f]{2}$/.test(normalized)) {
    return `Key 0x${normalized.slice(2).toUpperCase()}`;
  }
  if (normalized.length === 1) return normalized.toUpperCase();
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

export function keysLabel(keys: string[]): string {
  return keys.map(keyLabel).join(" + ");
}

export function buttonLabel(button: MouseButton): string {
  return BUTTON_LABEL[button];
}

export function buttonName(button: MouseButton): string {
  return BUTTON_NAME[button];
}

export function isModifier(key: string): boolean {
  return ["ctrl", "shift", "alt", "win"].includes(key.toLowerCase());
}

export function scrollLabel(direction: ScrollDirection): string {
  return direction.charAt(0).toUpperCase() + direction.slice(1);
}

export function triggerLabel(trigger: Trigger): string {
  switch (trigger.type) {
    case "singleClick":
      return "Single click";
    case "doubleClick":
      return "Double click";
    case "longPress":
      return "Long press";
    case "release":
      return "On release";
    case "withButton":
      return `+ ${buttonLabel(trigger.button)}`;
    case "withWheel":
      return `+ Scroll ${trigger.direction}`;
  }
}

export function triggerDescription(mapping: Mapping): string {
  const button = buttonLabel(mapping.button);
  switch (mapping.trigger.type) {
    case "singleClick":
      return `Click ${button} once`;
    case "doubleClick":
      return `Double-click ${button}`;
    case "longPress":
      return `Hold ${button}`;
    case "release":
      return `Release ${button}`;
    case "withButton":
      return `Hold ${button}, then press ${buttonLabel(mapping.trigger.button)}`;
    case "withWheel":
      return `Hold ${button}, then scroll ${mapping.trigger.direction}`;
  }
}

export function actionLabel(action: Action): string {
  switch (action.type) {
    case "shortcut":
      return keysLabel(action.keys);
    case "key":
      return keyLabel(action.key);
    case "holdKeys":
      return `Hold ${keysLabel(action.keys)}`;
    case "mouseClick":
      return `Click ${buttonName(action.button)}`;
    case "mouseHold":
      return `Hold ${buttonName(action.button)}`;
    case "scroll":
      return `Scroll ${action.direction} ×${action.amount}`;
    case "disable":
      return "Do nothing (disabled)";
    case "none":
      return "No action";
  }
}

export function actionKeys(action: Action): string[] | null {
  if (action.type === "shortcut" || action.type === "holdKeys") return action.keys;
  if (action.type === "key") return [action.key];
  return null;
}

export function actionTypeLabel(action: Action): string {
  switch (action.type) {
    case "shortcut":
      return "Keyboard shortcut";
    case "key":
      return "Single key";
    case "holdKeys":
      return "Hold keys";
    case "mouseClick":
      return "Mouse click";
    case "mouseHold":
      return "Mouse button hold";
    case "scroll":
      return "Scroll";
    case "disable":
      return "Disable button";
    case "none":
      return "No action";
  }
}

export const RULE_CATEGORY_LABEL: Record<RuleCategory, string> = {
  gesture: "Button combinations & gestures",
  longPress: "Long press",
  doubleClick: "Double click",
  release: "Button release",
  singleClick: "Single click",
};

export const RULE_CATEGORY_HINT: Record<RuleCategory, string> = {
  gesture: "Holding one button while pressing another or scrolling",
  longPress: "Fires while the button is still held",
  doubleClick: "Two clicks inside the double-click window",
  release: "Fires whenever the button is released",
  singleClick: "A quick press and release",
};

const BUTTON_ORDER: Record<MouseButton, number> = {
  left: 0,
  right: 1,
  middle: 2,
  x1: 3,
  x2: 4,
};

const TRIGGER_ORDER: Record<Trigger["type"], number> = {
  withButton: 0,
  withWheel: 1,
  longPress: 2,
  doubleClick: 3,
  release: 4,
  singleClick: 5,
};

export function sortMappings(mappings: Mapping[]): Mapping[] {
  return [...mappings].sort((a, b) => {
    const button = BUTTON_ORDER[a.button] - BUTTON_ORDER[b.button];
    if (button !== 0) return button;
    return TRIGGER_ORDER[a.trigger.type] - TRIGGER_ORDER[b.trigger.type];
  });
}

export function triggerKey(trigger: Trigger): string {
  switch (trigger.type) {
    case "withButton":
      return `withButton:${trigger.button}`;
    case "withWheel":
      return `withWheel:${trigger.direction}`;
    default:
      return trigger.type;
  }
}

/**
 * The engine resolves the first enabled mapping for a button + trigger pair.
 * Every later duplicate can never fire, so the UI marks it instead of silently
 * letting the user wonder why an edit did nothing.
 */
export function shadowedMappingIds(mappings: Mapping[]): Set<string> {
  const seen = new Set<string>();
  const shadowed = new Set<string>();
  for (const mapping of mappings) {
    if (!mapping.enabled) continue;
    const key = `${mapping.button}:${triggerKey(mapping.trigger)}`;
    if (seen.has(key)) {
      shadowed.add(mapping.id);
    } else {
      seen.add(key);
    }
  }
  return shadowed;
}

export function formatAge(ageMs: number): string {
  if (ageMs < 1_500) return "just now";
  const seconds = Math.round(ageMs / 1_000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  return `${Math.round(minutes / 60)}h ago`;
}

export function countByButton(
  mappings: Mapping[],
): Array<{ button: MouseButton; count: number }> {
  return (Object.keys(BUTTON_ORDER) as MouseButton[])
    .map((button) => ({
      button,
      count: mappings.filter((mapping) => mapping.button === button).length,
    }))
    .filter((entry) => entry.count > 0);
}

export function validateMapping(mapping: Mapping): string | null {
  if (mapping.trigger.type === "withButton" && mapping.trigger.button === mapping.button) {
    return "Choose a different second button for the combination.";
  }
  const action = mapping.action;
  if (action.type === "shortcut" || action.type === "holdKeys") {
    if (action.keys.length === 0) {
      return "Choose or record a keyboard shortcut before saving.";
    }
    if (action.keys.length > 8) {
      return "A shortcut can contain at most 8 keys.";
    }
  }
  if (action.type === "key" && !action.key) {
    return "Choose or record a key before saving.";
  }
  if (action.type === "scroll" && (action.amount < 1 || action.amount > 50)) {
    return "Scroll amount must be between 1 and 50.";
  }
  if (mapping.name && mapping.name.length > 60) {
    return "Mapping labels can be at most 60 characters.";
  }
  return null;
}
