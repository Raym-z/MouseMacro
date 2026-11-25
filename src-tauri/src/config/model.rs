use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 2;
pub const INJECTION_TAG: usize = 0x4D4D_4D01;

/// Default emergency stop. `F12` is reachable on every keyboard and unlikely to
/// collide with application shortcuts when combined with three modifiers.
pub const DEFAULT_PANIC_HOTKEY: [&str; 4] = ["ctrl", "alt", "shift", "f12"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

impl MouseButton {
    pub fn label(self) -> &'static str {
        match self {
            MouseButton::Left => "Left",
            MouseButton::Right => "Right",
            MouseButton::Middle => "Middle",
            MouseButton::X1 => "B4",
            MouseButton::X2 => "B5",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn label(self) -> &'static str {
        match self {
            ScrollDirection::Up => "Scroll up",
            ScrollDirection::Down => "Scroll down",
            ScrollDirection::Left => "Scroll left",
            ScrollDirection::Right => "Scroll right",
        }
    }

}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Trigger {
    SingleClick,
    DoubleClick,
    LongPress,
    Release,
    WithButton { button: MouseButton },
    WithWheel { direction: ScrollDirection },
}



#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    Shortcut { keys: Vec<String> },
    Key { key: String },
    HoldKeys { keys: Vec<String> },
    MouseClick { button: MouseButton },
    MouseHold { button: MouseButton },
    Scroll {
        direction: ScrollDirection,
        amount: i32,
    },
    Disable,
    None,
}

impl Action {
    /// Hold-style actions stay down until the source trigger is released.
    pub fn is_sustained(&self) -> bool {
        matches!(self, Action::MouseHold { .. } | Action::HoldKeys { .. })
    }

}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub id: String,
    pub name: Option<String>,
    pub button: MouseButton,
    pub trigger: Trigger,
    pub action: Action,
    pub enabled: bool,
}

impl Mapping {
    pub fn new(button: MouseButton, trigger: Trigger, action: Action) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: None,
            button,
            trigger,
            action,
            enabled: true,
        }
    }

    pub fn matches(&self, button: MouseButton, trigger: &Trigger) -> bool {
        self.enabled && self.button == button && &self.trigger == trigger
    }
}

/// A set of mappings that replaces the profile's mappings while
/// [`Layer::activator`] is held. Mappings that the layer does not define fall
/// back to the profile, so a layer can change just one or two buttons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub activator: MouseButton,
    pub enabled: bool,
    #[serde(default)]
    pub mappings: Vec<Mapping>,
}

impl Layer {
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, activator: MouseButton) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            activator,
            enabled: true,
            mappings: Vec::new(),
        }
    }

}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExceptionMode {
    /// Let the profile's mappings apply normally. This is the implicit state
    /// when no rule exists; an explicit rule is only needed to override a
    /// broader rule, so the UI usually removes the rule instead.
    #[default]
    Always,
    /// Pass physical input through untouched in this application.
    Disable,
    /// Use another profile while this application is in the foreground.
    Profile,
}

/// A per-executable behaviour rule. `process` is stored as a lowercase file
/// name (`photoshop.exe`) so hand-edited paths still normalise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionRule {
    pub id: String,
    pub process: String,
    pub mode: ExceptionMode,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl ExceptionRule {
    #[allow(dead_code)]
    pub fn new(process: impl Into<String>, mode: ExceptionMode) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            process: process.into(),
            mode,
            profile_id: None,
            enabled: true,
        }
    }

    pub fn matches(&self, process: &str) -> bool {
        self.enabled && self.process.eq_ignore_ascii_case(process)
    }
}

fn default_true() -> bool {
    true
}

/// Normalises a user-supplied executable name: lowercase basename, `.exe`
/// suffix, no path. Returns `None` when nothing usable remains.
pub fn normalize_process_name(raw: &str) -> Option<String> {
    let base = raw
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if base.is_empty() {
        return None;
    }
    let with_extension = if base.ends_with(".exe") {
        base
    } else if base.contains('.') {
        base
    } else {
        format!("{base}.exe")
    };
    let trimmed: String = with_extension.chars().take(120).collect();
    Some(trimmed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleCategory {
    Gesture,
    LongPress,
    DoubleClick,
    Release,
    SingleClick,
}

impl RuleCategory {
    pub fn all() -> Vec<RuleCategory> {
        vec![
            RuleCategory::Gesture,
            RuleCategory::LongPress,
            RuleCategory::DoubleClick,
            RuleCategory::Release,
            RuleCategory::SingleClick,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub enabled: bool,
    pub launch_at_startup: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub theme: Theme,
    pub long_press_ms: u64,
    pub double_click_ms: u64,
    pub process_injected_input: bool,
    pub rule_priority: Vec<RuleCategory>,
    /// Global emergency stop. An empty list disables the hotkey.
    #[serde(default = "default_panic_hotkey")]
    pub panic_hotkey: Vec<String>,
    /// False on a fresh install so the app can offer guided setup.
    #[serde(default)]
    pub onboarded: bool,
}

fn default_panic_hotkey() -> Vec<String> {
    DEFAULT_PANIC_HOTKEY
        .iter()
        .map(|key| (*key).to_string())
        .collect()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            launch_at_startup: false,
            start_minimized: false,
            close_to_tray: true,
            theme: Theme::System,
            long_press_ms: 400,
            double_click_ms: 280,
            process_injected_input: false,
            rule_priority: RuleCategory::all(),
            panic_hotkey: default_panic_hotkey(),
            onboarded: false,
        }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        self.long_press_ms = self.long_press_ms.clamp(100, 2_000);
        self.double_click_ms = self.double_click_ms.clamp(120, 1_000);

        self.panic_hotkey = self
            .panic_hotkey
            .iter()
            .map(|key| key.trim().to_lowercase())
            .filter(|key| !key.is_empty())
            .fold(Vec::new(), |mut keys, key| {
                if !keys.contains(&key) {
                    keys.push(key);
                }
                keys
            });
        self.panic_hotkey.truncate(5);

        let mut seen = Vec::new();
        self.rule_priority.retain(|category| {
            if seen.contains(category) {
                false
            } else {
                seen.push(*category);
                true
            }
        });
        for category in RuleCategory::all() {
            if !self.rule_priority.contains(&category) {
                self.rule_priority.push(category);
            }
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub mappings: Vec<Mapping>,
    /// Held-button layers, evaluated before `mappings` while active.
    #[serde(default)]
    pub layers: Vec<Layer>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            mappings: Vec::new(),
            layers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "current_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub settings: Settings,
    pub profiles: Vec<Profile>,
    pub active_profile_id: String,
    /// Per-application overrides, matched against the foreground executable.
    #[serde(default)]
    pub exceptions: Vec<ExceptionRule>,
}

fn current_schema() -> u32 {
    SCHEMA_VERSION
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut default_profile = Profile::new("Default");
        default_profile.description = "Everyday browsing and desktop use".into();
        default_profile.mappings = vec![
            Mapping::new(
                MouseButton::X2,
                Trigger::SingleClick,
                Action::Shortcut {
                    keys: vec!["ctrl".into(), "z".into()],
                },
            ),
            {
                let mut m = Mapping::new(
                    MouseButton::X2,
                    Trigger::WithWheel {
                        direction: ScrollDirection::Up,
                    },
                    Action::Shortcut {
                        keys: vec!["ctrl".into(), "equal".into()],
                    },
                );
                m.id = "seed-b5-wheel-up".into();
                m
            },
            {
                let mut m = Mapping::new(
                    MouseButton::X2,
                    Trigger::WithWheel {
                        direction: ScrollDirection::Down,
                    },
                    Action::Shortcut {
                        keys: vec!["ctrl".into(), "minus".into()],
                    },
                );
                m.id = "seed-b5-wheel-down".into();
                m
            },
        ];
        default_profile.mappings[0].id = "seed-b5-tap".into();

        let mut productivity = Profile::new("Productivity");
        productivity.description = "Copy, paste and window management shortcuts".into();
        productivity.mappings = vec![
            Mapping::new(
                MouseButton::X1,
                Trigger::SingleClick,
                Action::Shortcut {
                    keys: vec!["ctrl".into(), "c".into()],
                },
            ),
            Mapping::new(
                MouseButton::X2,
                Trigger::SingleClick,
                Action::Shortcut {
                    keys: vec!["ctrl".into(), "v".into()],
                },
            ),
        ];

        let gaming = Profile::new("Gaming");
        let mut design = Profile::new("Design");
        design.description = "Precision shortcuts for creative work".into();

        let profiles = vec![default_profile, productivity, gaming, design];
        let active_profile_id = profiles[0].id.clone();

        Self {
            schema_version: SCHEMA_VERSION,
            settings: Settings::default(),
            profiles,
            active_profile_id,
            exceptions: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn active_profile(&self) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_profile_id)
    }

    #[cfg(test)]
    pub fn active_profile_mut(&mut self) -> Option<&mut Profile> {
        let id = self.active_profile_id.clone();
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Repairs configs loaded from disk or imported from elsewhere. Returns human-readable notes.
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut notes = Vec::new();
        let migrating = self.schema_version < SCHEMA_VERSION;
        self.settings.sanitize();

        if self.profiles.is_empty() {
            let mut profile = Profile::new("Default");
            profile.description = "Everyday browsing and desktop use".into();
            self.profiles.push(profile);
            notes.push("Added a default profile because none existed.".into());
        }

        let mut profile_ids = Vec::new();
        for profile in &mut self.profiles {
            let name = profile.name.trim();
            profile.name = if name.is_empty() {
                "Untitled profile".into()
            } else {
                name.chars().take(60).collect()
            };
            profile.description = profile.description.trim().chars().take(160).collect();
            if profile.id.trim().is_empty() || profile_ids.contains(&profile.id) {
                profile.id = Uuid::new_v4().to_string();
                notes.push(format!("Repaired an invalid profile id for “{}”.", profile.name));
            }
            profile_ids.push(profile.id.clone());

            let owner = profile.name.clone();
            sanitize_mappings(&mut profile.mappings, &owner, "profile", &mut notes);

            let mut layer_ids = Vec::new();
            let mut activators = Vec::new();
            let before_layers = profile.layers.len();
            profile.layers.retain_mut(|layer| {
                let name = layer.name.trim();
                layer.name = if name.is_empty() {
                    "Layer".into()
                } else {
                    name.chars().take(60).collect()
                };
                if layer.id.trim().is_empty() || layer_ids.contains(&layer.id) {
                    layer.id = Uuid::new_v4().to_string();
                }
                layer_ids.push(layer.id.clone());

                // Two layers cannot share an activator: the first one wins, so
                // keep the list unambiguous instead of silently ignoring one.
                if activators.contains(&layer.activator) {
                    return false;
                }
                activators.push(layer.activator);

                let owner = layer.name.clone();
                sanitize_mappings(&mut layer.mappings, &owner, "layer", &mut notes);
                true
            });
            if profile.layers.len() != before_layers {
                notes.push(format!(
                    "Removed {} layer(s) that shared a button in “{}”.",
                    before_layers - profile.layers.len(),
                    profile.name
                ));
            }
            profile.layers.truncate(20);
        }

        if !self.profiles.iter().any(|p| p.id == self.active_profile_id) {
            self.active_profile_id = self.profiles[0].id.clone();
            notes.push("Selected the first profile as active.".into());
        }

        let profile_ids: Vec<String> = self.profiles.iter().map(|p| p.id.clone()).collect();
        let mut processes = Vec::new();
        let before_exceptions = self.exceptions.len();
        self.exceptions.retain_mut(|rule| {
            let Some(process) = normalize_process_name(&rule.process) else {
                return false;
            };
            rule.process = process;
            if processes.contains(&rule.process) {
                return false;
            }
            processes.push(rule.process.clone());

            if rule
                .profile_id
                .as_deref()
                .is_some_and(|id| !profile_ids.iter().any(|known| known == id))
            {
                rule.profile_id = None;
            }
            if rule.mode == ExceptionMode::Profile && rule.profile_id.is_none() {
                rule.mode = ExceptionMode::Disable;
                notes.push(format!(
                    "“{}” had no profile, so it now passes input through.",
                    rule.process
                ));
            }
            true
        });
        if self.exceptions.len() != before_exceptions {
            notes.push(format!(
                "Removed {} duplicate or invalid application rule(s).",
                before_exceptions - self.exceptions.len()
            ));
        }
        self.exceptions.truncate(100);

        // Existing installs should not see the first-run wizard after upgrading.
        if migrating {
            self.settings.onboarded = true;
        }

        self.schema_version = SCHEMA_VERSION;
        notes
    }
}

fn sanitize_mappings(
    mappings: &mut Vec<Mapping>,
    owner: &str,
    kind: &str,
    notes: &mut Vec<String>,
) {
    let mut mapping_ids = Vec::new();
    let before = mappings.len();
    mappings.retain_mut(|mapping| {
        mapping.name = mapping
            .name
            .as_ref()
            .map(|n| n.trim().chars().take(60).collect::<String>())
            .filter(|n| !n.is_empty());
        if mapping.id.trim().is_empty() || mapping_ids.contains(&mapping.id) {
            mapping.id = Uuid::new_v4().to_string();
        }
        mapping_ids.push(mapping.id.clone());

        match &mut mapping.action {
            Action::Shortcut { keys } | Action::HoldKeys { keys } => {
                keys.retain(|key| !key.trim().is_empty());
                for key in keys.iter_mut() {
                    *key = key.trim().to_lowercase();
                }
                keys.dedup();
                keys.truncate(8);
                !keys.is_empty()
            }
            Action::Key { key } => {
                *key = key.trim().to_lowercase();
                !key.is_empty()
            }
            Action::Scroll { amount, .. } => {
                *amount = (*amount).clamp(1, 50);
                true
            }
            _ => true,
        }
    });
    if mappings.len() != before {
        notes.push(format!(
            "Removed {} incomplete mapping(s) from {kind} “{owner}”.",
            before - mappings.len()
        ));
    }
    mappings.truncate(500);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_and_has_seed_mappings() {
        let mut config = AppConfig::default();
        let notes = config.sanitize();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(config.profiles.len(), 4);
        assert_eq!(config.active_profile().unwrap().mappings.len(), 3);
        assert!(!config.settings.enabled);
    }

    #[test]
    fn sanitize_repairs_empty_profiles_and_priority() {
        let mut config = AppConfig {
            schema_version: 0,
            settings: Settings {
                long_press_ms: 1,
                double_click_ms: 9_000,
                rule_priority: vec![RuleCategory::Gesture, RuleCategory::Gesture],
                ..Settings::default()
            },
            profiles: vec![],
            active_profile_id: "missing".into(),
            exceptions: vec![],
        };
        let notes = config.sanitize();
        assert!(!notes.is_empty());
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.active_profile_id, config.profiles[0].id);
        assert_eq!(config.settings.long_press_ms, 100);
        assert_eq!(config.settings.double_click_ms, 1_000);
        assert_eq!(config.settings.rule_priority.len(), 5);
    }

    #[test]
    fn migrating_an_old_config_marks_it_onboarded() {
        let mut config = AppConfig {
            schema_version: 1,
            ..AppConfig::default()
        };
        config.settings.onboarded = false;
        let notes = config.sanitize();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert!(config.settings.onboarded);
    }

    #[test]
    fn exceptions_are_normalised_and_deduplicated() {
        let mut config = AppConfig::default();
        let profile_id = config.profiles[1].id.clone();
        config.exceptions = vec![
            ExceptionRule::new(r"C:\Program Files\Google\Chrome\chrome.exe", ExceptionMode::Disable),
            ExceptionRule::new("CHROME", ExceptionMode::Profile),
            ExceptionRule::new("photoshop", ExceptionMode::Profile),
            ExceptionRule::new("", ExceptionMode::Disable),
        ];
        config.exceptions[2].profile_id = Some(profile_id.clone());
        let notes = config.sanitize();
        assert_eq!(config.exceptions.len(), 2, "{notes:?}");
        assert_eq!(config.exceptions[0].process, "chrome.exe");
        assert_eq!(config.exceptions[1].process, "photoshop.exe");
        assert_eq!(config.exceptions[1].profile_id.as_deref(), Some(profile_id.as_str()));
    }

    #[test]
    fn exception_with_missing_profile_falls_back_to_disable() {
        let mut config = AppConfig::default();
        let mut rule = ExceptionRule::new("game.exe", ExceptionMode::Profile);
        rule.profile_id = Some("missing".into());
        config.exceptions.push(rule);
        config.sanitize();
        assert_eq!(config.exceptions[0].mode, ExceptionMode::Disable);
        assert!(config.exceptions[0].profile_id.is_none());
    }

    #[test]
    fn duplicate_layer_activators_are_removed() {
        let mut config = AppConfig::default();
        let profile = config.active_profile_mut().unwrap();
        let mut first = Layer::new("Shift", MouseButton::X2);
        first.mappings.push(Mapping::new(
            MouseButton::Left,
            Trigger::SingleClick,
            Action::Shortcut {
                keys: vec!["ctrl".into(), "c".into()],
            },
        ));
        let second = Layer::new("Also shift", MouseButton::X2);
        profile.layers = vec![first, second];
        config.sanitize();
        assert_eq!(config.active_profile().unwrap().layers.len(), 1);
        assert_eq!(config.active_profile().unwrap().layers[0].name, "Shift");
    }

    #[test]
    fn process_names_become_lowercase_executables() {
        assert_eq!(
            normalize_process_name(r"C:\Program Files\Google\Chrome\Chrome.EXE").as_deref(),
            Some("chrome.exe")
        );
        assert_eq!(
            normalize_process_name("Photoshop").as_deref(),
            Some("photoshop.exe")
        );
        assert_eq!(normalize_process_name("   "), None);
    }

    #[test]
    fn panic_hotkey_is_lowercased_and_deduplicated() {
        let mut settings = Settings {
            panic_hotkey: vec!["Ctrl".into(), " CTRL ".into(), "F12".into(), "".into()],
            ..Settings::default()
        };
        settings.sanitize();
        assert_eq!(settings.panic_hotkey, vec!["ctrl", "f12"]);
    }

    #[test]
    fn invalid_shortcuts_are_removed() {
        let mut config = AppConfig::default();
        config.profiles[0].mappings.push(Mapping::new(
            MouseButton::X1,
            Trigger::SingleClick,
            Action::Shortcut { keys: vec![] },
        ));
        let before = config.profiles[0].mappings.len();
        config.sanitize();
        assert_eq!(config.profiles[0].mappings.len(), before - 1);
    }
}
