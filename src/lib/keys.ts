/**
 * Canonical key catalog for the manual key picker.
 *
 * Tokens must match `src-tauri/src/engine/keys.rs` so a chosen key can be
 * replayed with SendInput. Unknown keys can still be entered by typing a
 * character or their hex virtual-key code (`vkXX`).
 */

export interface KeyOption {
  token: string;
  label: string;
  /** Extra search terms (symbols, common alternate names). */
  aliases?: string[];
}

export interface KeyGroup {
  id: string;
  label: string;
  keys: KeyOption[];
}

export const MODIFIER_KEYS: KeyOption[] = [
  { token: "ctrl", label: "Ctrl", aliases: ["control"] },
  { token: "shift", label: "Shift" },
  { token: "alt", label: "Alt", aliases: ["menu"] },
  { token: "win", label: "Win", aliases: ["windows", "super", "meta"] },
];

const letterKeys: KeyOption[] = "abcdefghijklmnopqrstuvwxyz"
  .split("")
  .map((letter) => ({ token: letter, label: letter.toUpperCase() }));

const digitKeys: KeyOption[] = "0123456789"
  .split("")
  .map((digit) => ({ token: digit, label: digit }));

const functionKeys: KeyOption[] = Array.from({ length: 24 }, (_, index) => {
  const number = index + 1;
  return { token: `f${number}`, label: `F${number}`, aliases: [`f ${number}`] };
});

export const KEY_GROUPS: KeyGroup[] = [
  { id: "letters", label: "Letters", keys: letterKeys },
  { id: "digits", label: "Digits", keys: digitKeys },
  {
    id: "navigation",
    label: "Navigation",
    keys: [
      { token: "up", label: "↑", aliases: ["up", "arrow up"] },
      { token: "down", label: "↓", aliases: ["down", "arrow down"] },
      { token: "left", label: "←", aliases: ["left", "arrow left"] },
      { token: "right", label: "→", aliases: ["right", "arrow right"] },
      { token: "home", label: "Home" },
      { token: "end", label: "End" },
      { token: "pageup", label: "Page Up", aliases: ["pgup", "prior"] },
      { token: "pagedown", label: "Page Down", aliases: ["pgdn", "next"] },
      { token: "insert", label: "Insert", aliases: ["ins"] },
      { token: "delete", label: "Delete", aliases: ["del"] },
    ],
  },
  {
    id: "system",
    label: "Editing & system",
    keys: [
      { token: "enter", label: "Enter", aliases: ["return"] },
      { token: "tab", label: "Tab" },
      { token: "space", label: "Space", aliases: ["spacebar"] },
      { token: "backspace", label: "Backspace", aliases: ["back"] },
      { token: "esc", label: "Esc", aliases: ["escape"] },
      { token: "capslock", label: "Caps Lock", aliases: ["caps"] },
      { token: "numlock", label: "Num Lock" },
      { token: "scrolllock", label: "Scroll Lock" },
      { token: "printscreen", label: "Print Screen", aliases: ["prtsc"] },
      { token: "pause", label: "Pause" },
      { token: "apps", label: "Menu", aliases: ["context menu", "apps"] },
      { token: "sleep", label: "Sleep" },
    ],
  },
  {
    id: "symbols",
    label: "Symbols",
    keys: [
      { token: "grave", label: "`", aliases: ["backtick", "tilde"] },
      { token: "minus", label: "−", aliases: ["dash", "hyphen"] },
      { token: "equal", label: "=", aliases: ["equals", "plus"] },
      { token: "bracketleft", label: "[", aliases: ["left bracket"] },
      { token: "bracketright", label: "]", aliases: ["right bracket"] },
      { token: "backslash", label: "\\", aliases: ["back slash"] },
      { token: "semicolon", label: ";", aliases: ["semi"] },
      { token: "quote", label: "'", aliases: ["apostrophe"] },
      { token: "comma", label: ",", aliases: ["comma"] },
      { token: "period", label: ".", aliases: ["dot", "full stop"] },
      { token: "slash", label: "/", aliases: ["forward slash"] },
    ],
  },
  {
    id: "numpad",
    label: "Numeric keypad",
    keys: [
      ...Array.from({ length: 10 }, (_, digit) => ({
        token: `numpad${digit}`,
        label: `Num ${digit}`,
        aliases: [`numpad ${digit}`],
      })),
      { token: "numpadadd", label: "Num +", aliases: ["num plus"] },
      { token: "numpadsubtract", label: "Num −", aliases: ["num minus"] },
      { token: "numpadmultiply", label: "Num *", aliases: ["num star"] },
      { token: "numpaddivide", label: "Num /", aliases: ["num slash"] },
      { token: "numpaddecimal", label: "Num .", aliases: ["num dot"] },
    ],
  },
  { id: "function", label: "Function keys", keys: functionKeys },
  {
    id: "media",
    label: "Media & browser",
    keys: [
      { token: "volumemute", label: "Mute", aliases: ["volume mute"] },
      { token: "volumedown", label: "Volume Down" },
      { token: "volumeup", label: "Volume Up" },
      { token: "mediaplaypause", label: "Play / Pause", aliases: ["play pause"] },
      { token: "mediastop", label: "Stop Media", aliases: ["media stop"] },
      { token: "medianext", label: "Next Track", aliases: ["next"] },
      { token: "mediaprev", label: "Previous Track", aliases: ["previous"] },
      { token: "browserback", label: "Browser Back", aliases: ["back"] },
      { token: "browserforward", label: "Browser Forward", aliases: ["forward"] },
      { token: "browserrefresh", label: "Refresh", aliases: ["reload"] },
      { token: "browserstop", label: "Browser Stop", aliases: ["stop"] },
      { token: "browserhome", label: "Browser Home", aliases: ["home page"] },
    ],
  },
];

export const MODIFIER_TOKENS = MODIFIER_KEYS.map((key) => key.token);

export function isModifierToken(token: string): boolean {
  return MODIFIER_TOKENS.includes(token.trim().toLowerCase());
}

/** A short label for a key chip. Falls back to the token for unknown keys. */
export function keyTokenLabel(token: string): string {
  const normalized = token.trim().toLowerCase();
  for (const group of KEY_GROUPS) {
    for (const option of group.keys) {
      if (option.token === normalized) return option.label;
    }
  }
  const modifier = MODIFIER_KEYS.find((option) => option.token === normalized);
  if (modifier) return modifier.label;
  if (/^vk[0-9a-f]{2}$/.test(normalized)) {
    return `0x${normalized.slice(2).toUpperCase()}`;
  }
  if (normalized.length === 1) return normalized.toUpperCase();
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

export interface KeySearchResult {
  group: KeyGroup;
  matches: KeyOption[];
}

/** Filters the catalog by label, token or alias. Empty query returns all. */
export function searchKeys(
  query: string,
  exclude: string[] = [],
  includeModifiers = false,
): KeySearchResult[] {
  const needle = query.trim().toLowerCase().replace(/\s+/g, " ");
  const groups: KeyGroup[] = includeModifiers
    ? [{ id: "modifiers", label: "Modifiers", keys: MODIFIER_KEYS }, ...KEY_GROUPS]
    : KEY_GROUPS;
  return groups.map((group) => {
    const matches = group.keys.filter((option) => {
      if (exclude.includes(option.token)) return false;
      if (!needle) return true;
      const haystack = [option.label, option.token, ...(option.aliases ?? [])]
        .join(" ")
        .toLowerCase();
      return haystack.includes(needle);
    });
    return { group, matches };
  }).filter((result) => result.matches.length > 0);
}

const CODE_TO_TOKEN: Record<string, string> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Home: "home",
  End: "end",
  PageUp: "pageup",
  PageDown: "pagedown",
  Insert: "insert",
  Delete: "delete",
  Backspace: "backspace",
  Tab: "tab",
  Enter: "enter",
  NumpadEnter: "enter",
  Escape: "esc",
  Space: "space",
  CapsLock: "capslock",
  NumLock: "numlock",
  ScrollLock: "scrolllock",
  PrintScreen: "printscreen",
  Pause: "pause",
  ContextMenu: "apps",
  Semicolon: "semicolon",
  Equal: "equal",
  NumpadEqual: "equal",
  Comma: "comma",
  Minus: "minus",
  NumpadSubtract: "numpadsubtract",
  Period: "period",
  Slash: "slash",
  NumpadDivide: "numpaddivide",
  Backquote: "grave",
  BracketLeft: "bracketleft",
  Backslash: "backslash",
  BracketRight: "bracketright",
  Quote: "quote",
  NumpadMultiply: "numpadmultiply",
  NumpadAdd: "numpadadd",
  NumpadDecimal: "numpaddecimal",
  ControlLeft: "ctrl",
  ControlRight: "ctrl",
  ShiftLeft: "shift",
  ShiftRight: "shift",
  AltLeft: "alt",
  AltRight: "alt",
  MetaLeft: "win",
  MetaRight: "win",
  AudioVolumeMute: "volumemute",
  AudioVolumeDown: "volumedown",
  AudioVolumeUp: "volumeup",
  MediaPlayPause: "mediaplaypause",
  MediaStop: "mediastop",
  MediaTrackNext: "medianext",
  MediaTrackPrevious: "mediaprev",
  BrowserBack: "browserback",
  BrowserForward: "browserforward",
  BrowserRefresh: "browserrefresh",
  BrowserStop: "browserstop",
  BrowserHome: "browserhome",
};

/**
 * Maps a DOM keyboard event to a canonical key token without using any global
 * hook. Returns `null` for keys that cannot be mapped unambiguously.
 */
export function tokenFromKeyboardEvent(event: KeyboardEvent): string | null {
  const code = event.code;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return `numpad${code.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code.toLowerCase();
  const mapped = CODE_TO_TOKEN[code];
  if (mapped) return mapped;

  // Some layouts and virtual keyboards report no usable `code`.
  const key = event.key;
  if (key.length === 1) {
    const lower = key.toLowerCase();
    if (/^[a-z0-9]$/.test(lower)) return lower;
  }
  return null;
}
