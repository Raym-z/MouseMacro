//! Canonical key names shared by the configuration schema, the capture UI and
//! the SendInput executor.
//!
//! Keys are stored as lower-case tokens (`"ctrl"`, `"equal"`, `"f5"`). Anything
//! that is not in the table is stored as `vkXX` and can still be played back.

pub const MODIFIER_ORDER: [&str; 4] = ["ctrl", "shift", "alt", "win"];

pub const VK_CONTROL: u16 = 0x11;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_MENU: u16 = 0x12;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;

pub struct KeyDef {
    pub name: &'static str,
    pub vk: u16,
    pub display: &'static str,
}

const KEYS: &[KeyDef] = &[
    KeyDef { name: "ctrl", vk: VK_CONTROL, display: "Ctrl" },
    KeyDef { name: "shift", vk: VK_SHIFT, display: "Shift" },
    KeyDef { name: "alt", vk: VK_MENU, display: "Alt" },
    KeyDef { name: "win", vk: VK_LWIN, display: "Win" },
    KeyDef { name: "backspace", vk: 0x08, display: "Backspace" },
    KeyDef { name: "tab", vk: 0x09, display: "Tab" },
    KeyDef { name: "enter", vk: 0x0D, display: "Enter" },
    KeyDef { name: "pause", vk: 0x13, display: "Pause" },
    KeyDef { name: "capslock", vk: 0x14, display: "Caps Lock" },
    KeyDef { name: "esc", vk: 0x1B, display: "Esc" },
    KeyDef { name: "space", vk: 0x20, display: "Space" },
    KeyDef { name: "pageup", vk: 0x21, display: "Page Up" },
    KeyDef { name: "pagedown", vk: 0x22, display: "Page Down" },
    KeyDef { name: "end", vk: 0x23, display: "End" },
    KeyDef { name: "home", vk: 0x24, display: "Home" },
    KeyDef { name: "left", vk: 0x25, display: "Left" },
    KeyDef { name: "up", vk: 0x26, display: "Up" },
    KeyDef { name: "right", vk: 0x27, display: "Right" },
    KeyDef { name: "down", vk: 0x28, display: "Down" },
    KeyDef { name: "printscreen", vk: 0x2C, display: "Print Screen" },
    KeyDef { name: "insert", vk: 0x2D, display: "Insert" },
    KeyDef { name: "delete", vk: 0x2E, display: "Delete" },
    KeyDef { name: "numlock", vk: 0x90, display: "Num Lock" },
    KeyDef { name: "scrolllock", vk: 0x91, display: "Scroll Lock" },
    KeyDef { name: "apps", vk: 0x5D, display: "Menu" },
    KeyDef { name: "numpad0", vk: 0x60, display: "Num 0" },
    KeyDef { name: "numpad1", vk: 0x61, display: "Num 1" },
    KeyDef { name: "numpad2", vk: 0x62, display: "Num 2" },
    KeyDef { name: "numpad3", vk: 0x63, display: "Num 3" },
    KeyDef { name: "numpad4", vk: 0x64, display: "Num 4" },
    KeyDef { name: "numpad5", vk: 0x65, display: "Num 5" },
    KeyDef { name: "numpad6", vk: 0x66, display: "Num 6" },
    KeyDef { name: "numpad7", vk: 0x67, display: "Num 7" },
    KeyDef { name: "numpad8", vk: 0x68, display: "Num 8" },
    KeyDef { name: "numpad9", vk: 0x69, display: "Num 9" },
    KeyDef { name: "numpadmultiply", vk: 0x6A, display: "Num *" },
    KeyDef { name: "numpadadd", vk: 0x6B, display: "Num +" },
    KeyDef { name: "numpadsubtract", vk: 0x6D, display: "Num −" },
    KeyDef { name: "numpaddecimal", vk: 0x6E, display: "Num ." },
    KeyDef { name: "numpaddivide", vk: 0x6F, display: "Num /" },
    KeyDef { name: "f1", vk: 0x70, display: "F1" },
    KeyDef { name: "f2", vk: 0x71, display: "F2" },
    KeyDef { name: "f3", vk: 0x72, display: "F3" },
    KeyDef { name: "f4", vk: 0x73, display: "F4" },
    KeyDef { name: "f5", vk: 0x74, display: "F5" },
    KeyDef { name: "f6", vk: 0x75, display: "F6" },
    KeyDef { name: "f7", vk: 0x76, display: "F7" },
    KeyDef { name: "f8", vk: 0x77, display: "F8" },
    KeyDef { name: "f9", vk: 0x78, display: "F9" },
    KeyDef { name: "f10", vk: 0x79, display: "F10" },
    KeyDef { name: "f11", vk: 0x7A, display: "F11" },
    KeyDef { name: "f12", vk: 0x7B, display: "F12" },
    KeyDef { name: "f13", vk: 0x7C, display: "F13" },
    KeyDef { name: "f14", vk: 0x7D, display: "F14" },
    KeyDef { name: "f15", vk: 0x7E, display: "F15" },
    KeyDef { name: "f16", vk: 0x7F, display: "F16" },
    KeyDef { name: "f17", vk: 0x80, display: "F17" },
    KeyDef { name: "f18", vk: 0x81, display: "F18" },
    KeyDef { name: "f19", vk: 0x82, display: "F19" },
    KeyDef { name: "f20", vk: 0x83, display: "F20" },
    KeyDef { name: "f21", vk: 0x84, display: "F21" },
    KeyDef { name: "f22", vk: 0x85, display: "F22" },
    KeyDef { name: "f23", vk: 0x86, display: "F23" },
    KeyDef { name: "f24", vk: 0x87, display: "F24" },
    KeyDef { name: "numlock", vk: 0x90, display: "Num Lock" },
    KeyDef { name: "scrolllock", vk: 0x91, display: "Scroll Lock" },
    KeyDef { name: "semicolon", vk: 0xBA, display: ";" },
    KeyDef { name: "equal", vk: 0xBB, display: "=" },
    KeyDef { name: "comma", vk: 0xBC, display: "," },
    KeyDef { name: "minus", vk: 0xBD, display: "−" },
    KeyDef { name: "period", vk: 0xBE, display: "." },
    KeyDef { name: "slash", vk: 0xBF, display: "/" },
    KeyDef { name: "grave", vk: 0xC0, display: "`" },
    KeyDef { name: "bracketleft", vk: 0xDB, display: "[" },
    KeyDef { name: "backslash", vk: 0xDC, display: "\\" },
    KeyDef { name: "bracketright", vk: 0xDD, display: "]" },
    KeyDef { name: "quote", vk: 0xDE, display: "'" },
    KeyDef { name: "volumemute", vk: 0xAD, display: "Mute" },
    KeyDef { name: "volumedown", vk: 0xAE, display: "Volume Down" },
    KeyDef { name: "volumeup", vk: 0xAF, display: "Volume Up" },
    KeyDef { name: "medianext", vk: 0xB0, display: "Next Track" },
    KeyDef { name: "mediaprev", vk: 0xB1, display: "Previous Track" },
    KeyDef { name: "mediastop", vk: 0xB2, display: "Stop Media" },
    KeyDef { name: "mediaplaypause", vk: 0xB3, display: "Play / Pause" },
    KeyDef { name: "browserback", vk: 0xA6, display: "Browser Back" },
    KeyDef { name: "browserforward", vk: 0xA7, display: "Browser Forward" },
    KeyDef { name: "browserrefresh", vk: 0xA8, display: "Browser Refresh" },
    KeyDef { name: "browserstop", vk: 0xA9, display: "Browser Stop" },
    KeyDef { name: "browserhome", vk: 0xAC, display: "Browser Home" },
    KeyDef { name: "sleep", vk: 0x5F, display: "Sleep" },
];

/// Map a virtual-key code to the canonical token used in profiles.
pub fn name_from_vk(vk: u16) -> String {
    if let Some(def) = KEYS.iter().find(|def| def.vk == vk) {
        // Prefer the generic modifier names over left/right variants.
        return def.name.to_string();
    }
    match vk {
        VK_LWIN | VK_RWIN => "win".into(),
        0xA0 | 0xA1 => "shift".into(),
        0xA2 | 0xA3 => "ctrl".into(),
        0xA4 | 0xA5 => "alt".into(),
        0x41..=0x5A => ((vk as u8 as char).to_ascii_lowercase()).to_string(),
        0x30..=0x39 => ((vk as u8 as char)).to_string(),
        _ => format!("vk{vk:02x}"),
    }
}

pub fn vk_from_name(name: &str) -> Option<u16> {
    let name = name.trim().to_lowercase();
    if let Some(def) = KEYS.iter().find(|def| def.name == name) {
        return Some(def.vk);
    }
    let bytes = name.as_bytes();
    if bytes.len() == 1 {
        let ch = bytes[0];
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            return Some(ch.to_ascii_uppercase() as u16);
        }
    }
    if let Some(hex) = name.strip_prefix("vk") {
        if hex.len() == 2 {
            return u16::from_str_radix(hex, 16).ok();
        }
    }
    None
}

/// Human-readable label for a key token. Kept as the canonical formatting
/// helper for diagnostics and future native menus.
#[allow(dead_code)]
pub fn display_key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    if let Some(def) = KEYS.iter().find(|def| def.name == lower) {
        return def.display.to_string();
    }
    if lower.len() == 1 {
        return lower.to_uppercase();
    }
    if let Some(hex) = lower.strip_prefix("vk") {
        return format!("Key 0x{}", hex.to_uppercase());
    }
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn is_modifier(name: &str) -> bool {
    matches!(
        name.trim().to_lowercase().as_str(),
        "ctrl" | "control" | "shift" | "alt" | "win" | "meta" | "super"
    )
}

pub fn normalize_modifier(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "ctrl" | "control" => Some("ctrl"),
        "shift" => Some("shift"),
        "alt" => Some("alt"),
        "win" | "meta" | "super" => Some("win"),
        _ => None,
    }
}

/// Sorts keys so modifiers are pressed first. Non-modifiers keep their order.
pub fn ordered_for_press(keys: &[String]) -> Vec<String> {
    let mut ordered: Vec<String> = Vec::with_capacity(keys.len());
    for modifier in MODIFIER_ORDER {
        for key in keys {
            if normalize_modifier(key) == Some(modifier) && !ordered.iter().any(|k| k == modifier) {
                ordered.push(modifier.to_string());
            }
        }
    }
    for key in keys {
        let lower = key.trim().to_lowercase();
        if is_modifier(&lower) {
            continue;
        }
        if !ordered.contains(&lower) {
            ordered.push(lower);
        }
    }
    ordered
}

/// Parses a keyboard capture payload such as `["ctrl","shift","equal"]`.
pub fn normalize_combo(keys: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for key in keys {
        let lower = key.trim().to_lowercase();
        if lower.is_empty() {
            continue;
        }
        let normalized = normalize_modifier(&lower).unwrap_or(&lower).to_string();
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    ordered_for_press(&out)
}

/// True when a canonical token names one of the four modifiers.
pub fn is_modifier_name(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    MODIFIER_ORDER.iter().any(|modifier| *modifier == lower)
}

pub fn is_modifier_vk(vk: u16) -> bool {    matches!(
        vk,
        VK_CONTROL | VK_SHIFT | VK_MENU | VK_LWIN | VK_RWIN | 0xA0..=0xA5
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_virtual_keys() {
        let cases = [
            "a", "z", "0", "9", "f5", "tab", "enter", "esc", "space", "backspace",
            "delete", "home", "end", "pageup", "pagedown", "up", "down", "left", "right",
            "semicolon", "equal", "comma", "minus", "period", "slash", "grave",
            "bracketleft", "backslash", "bracketright", "quote", "volumeup",
        ];
        for name in cases {
            let vk = vk_from_name(name).unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(name_from_vk(vk), name, "vk {vk:#x} for {name}");
        }
    }

    #[test]
    fn unknown_keys_round_trip_as_hex() {
        let name = name_from_vk(0xE8);
        assert_eq!(name, "vke8");
        assert_eq!(vk_from_name(&name), Some(0xE8));
        assert_eq!(display_key("vke8"), "Key 0xE8");
    }

    #[test]
    fn modifiers_are_ordered_first() {
        let keys = vec!["z".to_string(), "shift".to_string(), "ctrl".to_string()];
        assert_eq!(ordered_for_press(&keys), vec!["ctrl", "shift", "z"]);
    }

    #[test]
    fn aliases_are_normalized() {
        assert_eq!(
            normalize_combo(&["Control".into(), "Z".into(), "ctrl".into()]),
            vec!["ctrl", "z"]
        );
        assert_eq!(normalize_modifier("meta"), Some("win"));
    }
}
