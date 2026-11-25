use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;
pub const INJECTION_TAG: usize = 0x4D4D_4D01;

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
    pub rule_priority: Vec<RuleCategory>,
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
            rule_priority: RuleCategory::all(),
        }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        self.long_press_ms = self.long_press_ms.clamp(100, 2_000);
        self.double_click_ms = self.double_click_ms.clamp(120, 1_000);

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
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            mappings: Vec::new(),
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

            let mut mapping_ids = Vec::new();
            let before = profile.mappings.len();
            profile.mappings.retain_mut(|mapping| {
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
            if profile.mappings.len() != before {
                notes.push(format!(
                    "Removed {} incomplete mapping(s) from “{}”.",
                    before - profile.mappings.len(),
                    profile.name
                ));
            }
            profile.mappings.truncate(500);
        }

        if !self.profiles.iter().any(|p| p.id == self.active_profile_id) {
            self.active_profile_id = self.profiles[0].id.clone();
            notes.push("Selected the first profile as active.".into());
        }

        self.schema_version = SCHEMA_VERSION;
        notes
    }
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
