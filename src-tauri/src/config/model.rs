use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::engine::keys::{is_modifier_name, normalize_combo, vk_from_name};

pub const SCHEMA_VERSION: u32 = 4;
pub const INJECTION_TAG: usize = 0x4D4D_4D01;

/// Media keys are sent as their dedicated virtual-key codes. Keeping the list
/// here means the UI, validation and injection all agree on the same names.
pub const MEDIA_KEYS: [&str; 7] = [
    "volumeup",
    "volumedown",
    "volumemute",
    "mediaplaypause",
    "medianext",
    "mediaprev",
    "mediastop",
];

pub fn is_media_key(name: &str) -> bool {
    MEDIA_KEYS.iter().any(|key| key.eq_ignore_ascii_case(name))
}

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

/// Wheel handling for physical wheel events. The defaults are a pass-through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScrollSettings {
    pub invert_vertical: bool,
    pub invert_horizontal: bool,
    /// Treat a vertical wheel as horizontal and vice versa.
    pub swap_axes: bool,
    /// How many notches to send per physical notch (1–5).
    pub speed: u32,
    /// Experimental: send more notches when events arrive in quick succession.
    pub acceleration: bool,
}

impl Default for ScrollSettings {
    fn default() -> Self {
        Self {
            invert_vertical: false,
            invert_horizontal: false,
            swap_axes: false,
            speed: 1,
            acceleration: false,
        }
    }
}

impl ScrollSettings {
    pub fn sanitize(&mut self) {
        self.speed = self.speed.clamp(1, 5);
    }

    /// True when the settings leave input untouched.
    pub fn is_identity(&self) -> bool {
        !self.invert_vertical
            && !self.invert_horizontal
            && !self.swap_axes
            && self.speed == 1
            && !self.acceleration
    }

    pub fn transform(&self, direction: ScrollDirection) -> ScrollDirection {
        use ScrollDirection::*;
        let swapped = if self.swap_axes {
            match direction {
                Up => Right,
                Down => Left,
                Right => Up,
                Left => Down,
            }
        } else {
            direction
        };
        match swapped {
            Up if self.invert_vertical => Down,
            Down if self.invert_vertical => Up,
            Left if self.invert_horizontal => Right,
            Right if self.invert_horizontal => Left,
            other => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowAction {
    Minimize,
    Maximize,
    Restore,
    Close,
    SnapLeft,
    SnapRight,
}

impl WindowAction {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            WindowAction::Minimize => "Minimize window",
            WindowAction::Maximize => "Maximize window",
            WindowAction::Restore => "Restore window",
            WindowAction::Close => "Close window",
            WindowAction::SnapLeft => "Snap left",
            WindowAction::SnapRight => "Snap right",
        }
    }
}

fn default_macro_amount() -> i32 {
    3
}

/// One ordered step inside a macro. `delay_ms` is the pause *after* the step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MacroStep {
    TapKeys {
        keys: Vec<String>,
        #[serde(default)]
        delay_ms: u64,
    },
    HoldKeys {
        keys: Vec<String>,
        #[serde(default)]
        delay_ms: u64,
    },
    ReleaseKeys {
        keys: Vec<String>,
        #[serde(default)]
        delay_ms: u64,
    },
    Wait {
        #[serde(default)]
        delay_ms: u64,
    },
    MouseClick {
        button: MouseButton,
        #[serde(default)]
        delay_ms: u64,
    },
    MouseHold {
        button: MouseButton,
        #[serde(default)]
        delay_ms: u64,
    },
    MouseRelease {
        button: MouseButton,
        #[serde(default)]
        delay_ms: u64,
    },
    Scroll {
        direction: ScrollDirection,
        #[serde(default = "default_macro_amount")]
        amount: i32,
        #[serde(default)]
        delay_ms: u64,
    },
    TypeText {
        text: String,
        #[serde(default)]
        delay_ms: u64,
    },
}

impl MacroStep {
    pub fn delay_ms(&self) -> u64 {
        match self {
            MacroStep::TapKeys { delay_ms, .. }
            | MacroStep::HoldKeys { delay_ms, .. }
            | MacroStep::ReleaseKeys { delay_ms, .. }
            | MacroStep::Wait { delay_ms }
            | MacroStep::MouseClick { delay_ms, .. }
            | MacroStep::MouseHold { delay_ms, .. }
            | MacroStep::MouseRelease { delay_ms, .. }
            | MacroStep::Scroll { delay_ms, .. }
            | MacroStep::TypeText { delay_ms, .. } => *delay_ms,
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
    /// A physical wheel notch with no button held. These mappings have no
    /// `Mapping::button`, so they are matched by direction alone.
    Wheel { direction: ScrollDirection },
}



#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Action {
    Shortcut {
        keys: Vec<String>,
    },
    Key {
        key: String,
    },
    HoldKeys {
        keys: Vec<String>,
    },
    MouseClick {
        button: MouseButton,
    },
    MouseHold {
        button: MouseButton,
    },
    Scroll {
        direction: ScrollDirection,
        amount: i32,
    },
    /// An ordered action sequence. Runs off the hook thread so it can contain
    /// waits without blocking input handling.
    Macro {
        #[serde(default)]
        steps: Vec<MacroStep>,
        #[serde(default = "default_macro_repeat")]
        repeat: u32,
        #[serde(default)]
        interval_ms: u64,
        /// When true, pressing the same source again cancels the running macro.
        #[serde(default)]
        toggle: bool,
    },
    /// Open a program, file, folder or URL through the Windows shell.
    Launch {
        target: String,
    },
    /// Type text into the focused window using Unicode input.
    TypeText {
        text: String,
    },
    /// A system media key.
    Media {
        key: String,
    },
    Window {
        action: WindowAction,
    },
    /// Press a key combination and keep it held until the action runs again.
    ToggleKeys {
        keys: Vec<String>,
    },
    /// Toggle a held mouse button, useful for drag-selecting.
    DragLock {
        button: MouseButton,
    },
    Disable,
    None,
}

fn default_macro_repeat() -> u32 {
    1
}

impl Action {
    /// Hold-style actions stay down until the source trigger is released.
    pub fn is_sustained(&self) -> bool {
        matches!(self, Action::MouseHold { .. } | Action::HoldKeys { .. })
    }
}


/// Extra behaviour for standalone wheel triggers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WheelOptions {
    /// Ignore further notches for this long after firing, so a high-resolution
    /// or free-spinning wheel does not run the action many times per flick.
    pub cooldown_ms: u64,
    /// Run the action and still send the original scroll to the foreground app.
    pub also_scroll: bool,
}

impl Default for WheelOptions {
    fn default() -> Self {
        Self {
            cooldown_ms: 80,
            also_scroll: false,
        }
    }
}

impl WheelOptions {
    pub fn sanitize(&mut self) {
        self.cooldown_ms = self.cooldown_ms.min(1_000);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub id: String,
    pub name: Option<String>,
    /// The button this mapping is attached to. `None` for standalone wheel
    /// mappings, which are matched by direction instead.
    #[serde(default)]
    pub button: Option<MouseButton>,
    pub trigger: Trigger,
    pub action: Action,
    pub enabled: bool,
    /// Only meaningful for [`Trigger::Wheel`]. `None` uses the defaults.
    #[serde(default)]
    pub wheel: Option<WheelOptions>,
}

impl Mapping {
    pub fn new(button: MouseButton, trigger: Trigger, action: Action) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: None,
            button: Some(button),
            trigger,
            action,
            enabled: true,
            wheel: None,
        }
    }

    /// A standalone wheel mapping, fired by a physical wheel notch with no
    /// button held.
    #[allow(dead_code)]
    pub fn wheel(direction: ScrollDirection, action: Action) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: None,
            button: None,
            trigger: Trigger::Wheel { direction },
            action,
            enabled: true,
            wheel: Some(WheelOptions::default()),
        }
    }

    pub fn matches(&self, button: MouseButton, trigger: &Trigger) -> bool {
        self.enabled && self.button == Some(button) && &self.trigger == trigger
    }
}

/// A set of mappings that replaces the profile's mappings while
/// [`Layer::activator`] is held. Mappings that the layer does not define fall
/// back to the profile, so a layer can change just one or two buttons.
/// Keyboard remapping settings. Remapping is off by default and requires an
/// explicit acknowledgement before the always-on hook is installed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyboardSettings {
    pub enabled: bool,
    /// The user has seen and accepted the hook explanation.
    pub acknowledged: bool,
}

/// What a key mapping does when its source combination goes down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum KeyBehaviour {
    /// Press the target keys while the source is held, preserving hold
    /// semantics. The matching release releases the targets.
    Remap { target: Vec<String> },
    /// Run an action once when the source goes down.
    Action { action: Action },
}

/// A keyboard source combination mapped to a target or an action.
///
/// `source` is canonical tokens with modifiers first and exactly one
/// non-modifier key. Bare modifiers cannot be sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMapping {
    pub id: String,
    pub name: Option<String>,
    pub source: Vec<String>,
    pub behaviour: KeyBehaviour,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl KeyMapping {
    #[allow(dead_code)]
    pub fn new(source: Vec<String>, behaviour: KeyBehaviour) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: None,
            source,
            behaviour,
            enabled: true,
        }
    }

    /// The single non-modifier key that identifies this mapping.
    pub fn source_key(&self) -> Option<&str> {
        self.source
            .iter()
            .find(|key| !is_modifier_name(key))
            .map(String::as_str)
    }
}

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
    /// Optional wheel override for this application. `None` inherits the
    /// global scroll settings.
    #[serde(default)]
    pub scroll: Option<ScrollSettings>,
    /// Pass keyboard input through untouched in this application.
    #[serde(default)]
    pub keyboard_disabled: bool,
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
            scroll: None,
            keyboard_disabled: false,
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
    /// Physical wheel handling that applies everywhere unless an application
    /// rule overrides it.
    #[serde(default)]
    pub scroll: ScrollSettings,
    /// Opt-in keyboard remapping.
    #[serde(default)]
    pub keyboard: KeyboardSettings,
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
            scroll: ScrollSettings::default(),
            keyboard: KeyboardSettings::default(),
        }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        self.long_press_ms = self.long_press_ms.clamp(100, 2_000);
        self.double_click_ms = self.double_click_ms.clamp(120, 1_000);
        self.scroll.sanitize();

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
    /// Keyboard source combinations. Only active while keyboard remapping is
    /// enabled.
    #[serde(default)]
    pub key_mappings: Vec<KeyMapping>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            mappings: Vec::new(),
            layers: Vec::new(),
            key_mappings: Vec::new(),
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
        // The emergency stop must always win, so key mappings that would
        // shadow it are removed rather than silently ignored at runtime.
        let panic = normalize_combo(&self.settings.panic_hotkey);

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
            sanitize_key_mappings(&mut profile.key_mappings, &owner, &panic, &mut notes);

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
            if let Some(scroll) = rule.scroll.as_mut() {
                scroll.sanitize();
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

        // A standalone wheel trigger owns no button, and a button trigger is
        // meaningless without one. Repair hand-edited or imported combinations
        // instead of letting a mapping that can never fire sit in the list.
        if matches!(mapping.trigger, Trigger::Wheel { .. }) {
            mapping.button = None;
            mapping
                .wheel
                .get_or_insert_with(WheelOptions::default)
                .sanitize();
        } else {
            if mapping.button.is_none() {
                return false;
            }
            mapping.wheel = None;
        }

        sanitize_action(&mut mapping.action)
    });
    if mappings.len() != before {
        notes.push(format!(
            "Removed {} incomplete mapping(s) from {kind} “{owner}”.",
            before - mappings.len()
        ));
    }
    mappings.truncate(500);
}

/// Repairs an action in place. Returns false when nothing usable remains, so
/// the caller can drop the mapping or key mapping that owns it.
fn sanitize_action(action: &mut Action) -> bool {
    match action {
        Action::Shortcut { keys } | Action::HoldKeys { keys } => {
            sanitize_key_list(keys, 8)
        }
        Action::Key { key } => {
            *key = key.trim().to_lowercase();
            !key.is_empty()
        }
        Action::Scroll { amount, .. } => {
            *amount = (*amount).clamp(1, 50);
            true
        }
        Action::Macro {
            steps,
            repeat,
            interval_ms,
            toggle: _,
        } => {
            steps.retain_mut(sanitize_step);
            *repeat = (*repeat).clamp(1, 100);
            *interval_ms = (*interval_ms).min(10_000);
            steps.truncate(40);

            // Honour the two-minute budget even for hand-edited or
            // imported files: trim trailing steps and reduce the repeat
            // count until one full run plus its intervals fits.
            const BUDGET_MS: u64 = 120_000;
            let step_total: u64 = steps.iter().map(MacroStep::delay_ms).sum();
            if step_total > BUDGET_MS {
                let mut running = 0u64;
                steps.retain(|step| {
                    if running.saturating_add(step.delay_ms()) > BUDGET_MS {
                        return false;
                    }
                    running = running.saturating_add(step.delay_ms());
                    true
                });
            }
            let step_total: u64 = steps.iter().map(MacroStep::delay_ms).sum();
            while *repeat > 1 {
                let rounds = *repeat as u64;
                let total = step_total
                    .saturating_mul(rounds)
                    .saturating_add(interval_ms.saturating_mul(rounds.saturating_sub(1)));
                if total <= BUDGET_MS {
                    break;
                }
                *repeat -= 1;
            }
            !steps.is_empty()
        }
        Action::Launch { target } => {
            *target = target.trim().chars().take(1_000).collect();
            !target.is_empty()
        }
        Action::TypeText { text } => {
            *text = text.chars().take(2_000).collect();
            !text.is_empty()
        }
        Action::Media { key } => {
            let lower = key.trim().to_lowercase();
            if is_media_key(&lower) {
                *key = lower;
                true
            } else {
                false
            }
        }
        Action::ToggleKeys { keys } => sanitize_key_list(keys, 4),
        Action::Window { .. }
        | Action::DragLock { .. }
        | Action::MouseClick { .. }
        | Action::MouseHold { .. }
        | Action::Disable
        | Action::None => true,
    }
}

/// Repairs a profile's key mappings, dropping anything that could never match.
fn sanitize_key_mappings(
    mappings: &mut Vec<KeyMapping>,
    owner: &str,
    panic: &[String],
    notes: &mut Vec<String>,
) {
    let before = mappings.len();
    let mut ids: Vec<String> = Vec::new();
    let mut sources: Vec<Vec<String>> = Vec::new();
    mappings.retain_mut(|mapping| {
        mapping.name = mapping
            .name
            .as_ref()
            .map(|name| name.trim().chars().take(60).collect::<String>())
            .filter(|name| !name.is_empty());
        if mapping.id.trim().is_empty() || ids.contains(&mapping.id) {
            mapping.id = Uuid::new_v4().to_string();
        }
        ids.push(mapping.id.clone());

        mapping.source = normalize_combo(&mapping.source);
        if mapping.source.len() > 4 {
            return false;
        }
        if mapping.source.iter().any(|key| vk_from_name(key).is_none()) {
            return false;
        }
        let non_modifiers = mapping
            .source
            .iter()
            .filter(|key| !is_modifier_name(key))
            .count();
        if non_modifiers != 1 || mapping.source.is_empty() {
            return false;
        }
        if !panic.is_empty() && mapping.source == panic {
            return false;
        }
        if sources.contains(&mapping.source) {
            return false;
        }
        sources.push(mapping.source.clone());

        match &mut mapping.behaviour {
            KeyBehaviour::Remap { target } => sanitize_key_list(target, 8),
            KeyBehaviour::Action { action } => sanitize_action(action),
        }
    });
    mappings.truncate(200);
    if mappings.len() != before {
        notes.push(format!(
            "Removed {} keyboard mapping(s) that could not match in “{owner}”.",
            before - mappings.len()
        ));
    }
}

/// Lowercases, trims and de-duplicates a key list. Returns false when nothing
/// usable remains so the caller can drop the mapping or macro step.
fn sanitize_key_list(keys: &mut Vec<String>, max: usize) -> bool {
    let mut seen: Vec<String> = Vec::new();
    keys.retain_mut(|key| {
        let normalized = key.trim().to_lowercase();
        if normalized.is_empty() || seen.contains(&normalized) {
            return false;
        }
        seen.push(normalized.clone());
        *key = normalized;
        true
    });
    keys.truncate(max);
    !keys.is_empty()
}

fn sanitize_step(step: &mut MacroStep) -> bool {
    match step {
        MacroStep::TapKeys { keys, delay_ms }
        | MacroStep::HoldKeys { keys, delay_ms }
        | MacroStep::ReleaseKeys { keys, delay_ms } => {
            *delay_ms = (*delay_ms).min(60_000);
            sanitize_key_list(keys, 8)
        }
        MacroStep::Wait { delay_ms } => {
            *delay_ms = (*delay_ms).clamp(1, 60_000);
            true
        }
        MacroStep::MouseClick { delay_ms, .. }
        | MacroStep::MouseHold { delay_ms, .. }
        | MacroStep::MouseRelease { delay_ms, .. } => {
            *delay_ms = (*delay_ms).min(60_000);
            true
        }
        MacroStep::Scroll {
            amount, delay_ms, ..
        } => {
            *amount = (*amount).clamp(1, 50);
            *delay_ms = (*delay_ms).min(60_000);
            true
        }
        MacroStep::TypeText { text, delay_ms } => {
            *text = text.chars().take(2_000).collect();
            *delay_ms = (*delay_ms).min(60_000);
            !text.is_empty()
        }
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
        assert!(!config.settings.keyboard.enabled);
        assert!(!config.settings.keyboard.acknowledged);
        assert!(config.profiles.iter().all(|p| p.key_mappings.is_empty()));
    }

    #[test]
    fn keyboard_mappings_round_trip_and_sanitize_drops_unmatchable_sources() {
        let mut config = AppConfig::default();
        let profile = config.active_profile_mut().unwrap();
        profile.key_mappings = vec![
            KeyMapping::new(
                vec!["CapsLock".into()],
                KeyBehaviour::Remap {
                    target: vec!["ctrl".into()],
                },
            ),
            // Bare modifier: can never be a source.
            KeyMapping::new(
                vec!["ctrl".into()],
                KeyBehaviour::Remap {
                    target: vec!["alt".into()],
                },
            ),
            // Two non-modifier keys: not a hotkey.
            KeyMapping::new(
                vec!["f12".into(), "f13".into()],
                KeyBehaviour::Remap {
                    target: vec!["a".into()],
                },
            ),
            // The emergency stop must always win.
            KeyMapping::new(
                vec![
                    "ctrl".into(),
                    "alt".into(),
                    "shift".into(),
                    "f12".into(),
                ],
                KeyBehaviour::Action {
                    action: Action::None,
                },
            ),
            KeyMapping::new(
                vec!["f12".into()],
                KeyBehaviour::Remap {
                    target: vec!["a".into()],
                },
            ),
            // Duplicate source: the first mapping wins.
            KeyMapping::new(
                vec!["f12".into()],
                KeyBehaviour::Remap {
                    target: vec!["b".into()],
                },
            ),
        ];

        let notes = config.sanitize();
        assert!(!notes.is_empty());
        let keys = &config.active_profile().unwrap().key_mappings;
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].source, vec!["capslock"]);
        assert_eq!(keys[1].source, vec!["f12"]);
        assert!(keys.iter().all(|mapping| mapping.enabled));

        let json = serde_json::to_string(&keys[0]).unwrap();
        assert!(json.contains("\"mode\":\"remap\""), "{json}");
        assert!(json.contains("\"target\":[\"ctrl\"]"), "{json}");

        let action = serde_json::to_string(&keys[1]).unwrap();
        assert!(!action.contains("action"));
    }

    #[test]
    fn wheel_mappings_round_trip_without_a_button() {
        let mut config = AppConfig::default();
        let mut wheel = Mapping::wheel(
            ScrollDirection::Left,
            Action::Shortcut {
                keys: vec!["alt".into(), "left".into()],
            },
        );
        wheel.wheel = Some(WheelOptions {
            cooldown_ms: 99_999,
            also_scroll: true,
        });
        let id = wheel.id.clone();
        config.active_profile_mut().unwrap().mappings.push(wheel);

        let notes = config.sanitize();
        assert!(notes.is_empty(), "{notes:?}");

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"direction\":\"left\""));
        assert!(json.contains("\"cooldownMs\":1000"));

        let mut back: AppConfig = serde_json::from_str(&json).unwrap();
        back.sanitize();
        let mapping = back
            .active_profile()
            .unwrap()
            .mappings
            .iter()
            .find(|mapping| mapping.id == id)
            .unwrap();
        assert_eq!(mapping.button, None);
        assert_eq!(
            mapping.trigger,
            Trigger::Wheel {
                direction: ScrollDirection::Left
            }
        );
    }

    #[test]
    fn sanitize_repairs_button_and_wheel_trigger_mismatches() {
        let mut config = AppConfig::default();
        let mut stray_button = Mapping::new(
            MouseButton::X1,
            Trigger::Wheel {
                direction: ScrollDirection::Up,
            },
            Action::None,
        );
        stray_button.button = Some(MouseButton::X1);
        let mut buttonless = Mapping::new(MouseButton::X1, Trigger::SingleClick, Action::None);
        buttonless.button = None;

        let profile = config.active_profile_mut().unwrap();
        profile.mappings.push(stray_button);
        profile.mappings.push(buttonless);

        config.sanitize();
        let mappings = &config.active_profile().unwrap().mappings;
        assert!(mappings
            .iter()
            .any(|mapping| mapping.button.is_none() && matches!(mapping.trigger, Trigger::Wheel { .. })));
        assert!(!mappings.iter().any(|mapping| mapping.button.is_none()
            && !matches!(mapping.trigger, Trigger::Wheel { .. })));
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

    #[test]
    fn scroll_settings_invert_and_swap_axes() {
        let inverted = ScrollSettings {
            invert_vertical: true,
            ..ScrollSettings::default()
        };
        assert_eq!(inverted.transform(ScrollDirection::Up), ScrollDirection::Down);
        assert_eq!(
            inverted.transform(ScrollDirection::Down),
            ScrollDirection::Up
        );
        assert_eq!(
            inverted.transform(ScrollDirection::Left),
            ScrollDirection::Left
        );

        let swapped = ScrollSettings {
            swap_axes: true,
            ..ScrollSettings::default()
        };
        assert_eq!(swapped.transform(ScrollDirection::Up), ScrollDirection::Right);
        assert_eq!(
            swapped.transform(ScrollDirection::Left),
            ScrollDirection::Down
        );

        let horizontal = ScrollSettings {
            invert_horizontal: true,
            ..ScrollSettings::default()
        };
        assert_eq!(
            horizontal.transform(ScrollDirection::Right),
            ScrollDirection::Left
        );
    }

    #[test]
    fn macro_sanitize_drops_invalid_steps_and_clamps_limits() {
        let mut config = AppConfig::default();
        config.profiles[0].mappings.push(Mapping::new(
            MouseButton::X1,
            Trigger::SingleClick,
            Action::Macro {
                steps: vec![
                    MacroStep::TapKeys {
                        keys: vec![" Ctrl ".into(), "".into(), "z".into()],
                        delay_ms: 999_999,
                    },
                    MacroStep::TapKeys {
                        keys: vec![],
                        delay_ms: 0,
                    },
                ],
                repeat: 500,
                interval_ms: 99_999,
                toggle: false,
            },
        ));
        config.sanitize();

        let action = &config.profiles[0].mappings.last().unwrap().action;
        match action {
            Action::Macro {
                steps,
                repeat,
                interval_ms,
                ..
            } => {
                assert_eq!(steps.len(), 1);
                assert_eq!(*repeat, 1);
                assert_eq!(*interval_ms, 10_000);
                match &steps[0] {
                    MacroStep::TapKeys { keys, delay_ms } => {
                        assert_eq!(keys, &vec!["ctrl".to_string(), "z".to_string()]);
                        assert_eq!(*delay_ms, 60_000);
                    }
                    other => panic!("unexpected step {other:?}"),
                }
            }
            other => panic!("unexpected action {other:?}"),
        }
    }

    #[test]
    fn unknown_media_actions_are_removed() {
        let mut config = AppConfig::default();
        config.profiles[0].mappings.push(Mapping::new(
            MouseButton::X1,
            Trigger::SingleClick,
            Action::Media {
                key: "not-a-key".into(),
            },
        ));
        let before = config.profiles[0].mappings.len();
        config.sanitize();
        assert_eq!(config.profiles[0].mappings.len(), before - 1);
    }

    #[test]
    fn macro_repeat_stays_within_the_duration_budget() {
        let mut config = AppConfig::default();
        config.profiles[0].mappings.push(Mapping::new(
            MouseButton::X1,
            Trigger::SingleClick,
            Action::Macro {
                steps: vec![MacroStep::Wait { delay_ms: 1_000 }],
                repeat: 500,
                interval_ms: 0,
                toggle: false,
            },
        ));
        config.sanitize();
        match &config.profiles[0].mappings.last().unwrap().action {
            Action::Macro { repeat, .. } => {
                assert!(*repeat <= 120, "repeat {repeat} exceeds the budget");
            }
            other => panic!("unexpected action {other:?}"),
        }
    }

    #[test]
    fn macro_json_uses_camel_case_fields() {
        let step = MacroStep::Wait { delay_ms: 25 };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"delayMs\""), "{json}");
        assert!(!json.contains("delay_ms"), "{json}");

        let action = Action::Macro {
            steps: vec![step],
            repeat: 2,
            interval_ms: 50,
            toggle: true,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"intervalMs\""), "{json}");
        assert!(!json.contains("interval_ms"), "{json}");

        let round_trip: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, action);
    }
}
