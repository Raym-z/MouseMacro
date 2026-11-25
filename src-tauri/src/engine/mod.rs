//! Pure, UI-independent input engine.
//!
//! The engine consumes abstract [`InputEvent`]s and returns a suppression
//! decision plus a list of [`Effect`]s. It never talks to Windows directly, so
//! the complete rule state machine can be tested deterministically.
//!
//! Resolution rules for a mapped button:
//!
//! 1. A chord (`button + other button`) or wheel gesture consumes the gesture
//!    and cancels the pending click/long-press.
//! 2. A long press fires at the configured threshold and cancels the click.
//! 3. Otherwise a double click waits for the second press; a single click fires
//!    on release or after the double-click window.
//! 4. If nothing matched, the swallowed physical events are replayed so the
//!    default behaviour is preserved rather than silently lost.

pub mod keys;

use crate::config::model::{
    Action, Mapping, MouseButton, RuleCategory, ScrollDirection, Trigger,
};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    ButtonDown { button: MouseButton, time: u64 },
    ButtonUp { button: MouseButton, time: u64 },
    Wheel { direction: ScrollDirection, time: u64 },
    Tick { time: u64 },
}

impl InputEvent {
    pub fn time(&self) -> u64 {
        match self {
            InputEvent::ButtonDown { time, .. }
            | InputEvent::ButtonUp { time, .. }
            | InputEvent::Wheel { time, .. }
            | InputEvent::Tick { time } => *time,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Execute an action instantly. Sustained actions are pressed and released
    /// immediately when delivered this way.
    Run(Action),
    /// Press a sustained action and keep it held until the matching token is
    /// released.
    StartHold { token: String, action: Action },
    ReleaseHold { token: String },
    /// Re-inject a physical button event that was swallowed while the engine
    /// decided what to do with it.
    Replay {
        button: MouseButton,
        direction: ReplayDirection,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub suppress: bool,
    pub effects: Vec<Effect>,
}

impl Outcome {
    fn pass() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub enabled: bool,
    pub long_press_ms: u64,
    pub double_click_ms: u64,
    pub priority: Vec<RuleCategory>,
    pub mappings: Vec<Mapping>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            long_press_ms: 400,
            double_click_ms: 280,
            priority: RuleCategory::all(),
            mappings: Vec::new(),
        }
    }
}

impl EngineConfig {
    pub fn from_app(config: &crate::config::model::AppConfig) -> Self {
        let settings = &config.settings;
        Self {
            enabled: settings.enabled,
            long_press_ms: settings.long_press_ms,
            double_click_ms: settings.double_click_ms,
            priority: settings.rule_priority.clone(),
            mappings: config
                .active_profile()
                .map(|profile| profile.mappings.clone())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
struct HeldButton {
    button: MouseButton,
    down_time: u64,
    gesture_used: bool,
    long_press_fired: bool,
    /// Holds started from a SingleClick rule's sustained action on press.
    tap_holds: Vec<String>,
    /// Holds started by long press / chord actions.
    holds: Vec<String>,
    chords: Vec<ActiveChord>,
}

#[derive(Debug, Clone)]
struct ActiveChord {
    button: MouseButton,
    tokens: Vec<String>,
}

#[derive(Debug, Clone)]
enum PendingFallback {
    Action(Mapping),
    Replay,
    Nothing,
}

#[derive(Debug, Clone)]
struct PendingTap {
    button: MouseButton,
    deadline: u64,
    fallback: PendingFallback,
}

#[derive(Debug, Clone)]
struct ConsumedUntilUp {
    button: MouseButton,
    tokens: Vec<String>,
}

pub struct InputEngine {
    config: EngineConfig,
    held: Vec<HeldButton>,
    pending: Option<PendingTap>,
    chord_consumed: Vec<MouseButton>,
    consumed_until_up: Vec<ConsumedUntilUp>,
    seq: u64,
}

impl InputEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            held: Vec::new(),
            pending: None,
            chord_consumed: Vec::new(),
            consumed_until_up: Vec::new(),
            seq: 0,
        }
    }

    /// Replaces configuration. Any in-flight gesture is cancelled and all
    /// synthetic holds are released; the caller must execute the returned
    /// effects.
    pub fn reconfigure(&mut self, config: EngineConfig) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.release_all_tokens(&mut effects);
        self.held.clear();
        self.pending = None;
        self.chord_consumed.clear();
        self.consumed_until_up.clear();
        self.config = config;
        effects
    }

    pub fn cancel_all(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.release_all_tokens(&mut effects);
        self.held.clear();
        self.pending = None;
        self.chord_consumed.clear();
        self.consumed_until_up.clear();
        effects
    }

    pub fn next_deadline(&self) -> Option<u64> {
        let mut deadline = self.pending.as_ref().map(|p| p.deadline);
        for held in &self.held {
            if held.gesture_used || held.long_press_fired {
                continue;
            }
            if self.has_mapping(held.button, &Trigger::LongPress) {
                let due = held.down_time.saturating_add(self.config.long_press_ms);
                deadline = Some(deadline.map_or(due, |current| current.min(due)));
            }
        }
        deadline
    }

    pub fn handle(&mut self, event: InputEvent) -> Outcome {
        if !self.config.enabled {
            // Never consume input while disabled. Stale state can only exist if
            // the caller forgot to reconfigure, but clear it defensively.
            if !self.held.is_empty() || self.pending.is_some() {
                self.held.clear();
                self.pending = None;
                self.chord_consumed.clear();
                self.consumed_until_up.clear();
            }
            return Outcome::pass();
        }

        let now = event.time();
        let mut outcome = Outcome::default();

        // A pending double-click window may have elapsed between timer ticks.
        self.flush_expired_pending(now, &mut outcome.effects);

        match event {
            InputEvent::ButtonDown { button, time } => {
                self.on_button_down(button, time, &mut outcome);
            }
            InputEvent::ButtonUp { button, time } => {
                self.on_button_up(button, time, &mut outcome);
            }
            InputEvent::Wheel { direction, .. } => {
                self.on_wheel(direction, &mut outcome);
            }
            InputEvent::Tick { time } => {
                self.on_tick(time, &mut outcome);
            }
        }

        outcome
    }

    fn on_button_down(&mut self, button: MouseButton, _time: u64, outcome: &mut Outcome) {
        // Gestures have the highest priority: a held mapped button + this
        // button can form a chord.
        if let Some(held_index) = self.find_chord(button) {
            let mapping = self
                .find_mapping(self.held[held_index].button, &Trigger::WithButton { button })
                .cloned();
            if let Some(mapping) = mapping {
                outcome.suppress = true;
                let held = &mut self.held[held_index];
                held.gesture_used = true;
                Self::release_tokens(held, outcome);
                let token = self.new_token(&mapping.id, "chord");
                if mapping.action.is_sustained() {
                    outcome.effects.push(Effect::StartHold {
                        token: token.clone(),
                        action: mapping.action.clone(),
                    });
                    self.held[held_index].holds.push(token.clone());
                } else {
                    self.run_action(&mapping.action, &mut outcome.effects);
                }
                self.held[held_index].chords.push(ActiveChord {
                    button,
                    tokens: vec![token],
                });
                self.chord_consumed.push(button);
                return;
            }
        }

        // If a double-click is pending for this same button, this is the second
        // press: fire the double action and consume the button until release.
        if let Some(pending) = self.pending.as_ref() {
            if pending.button == button {
                let pending = self.pending.take().expect("checked above");
                let double_mapping = self
                    .find_mapping(button, &Trigger::DoubleClick)
                    .cloned();
                outcome.suppress = true;
                let mut tokens = Vec::new();
                if let Some(mapping) = double_mapping {
                    let token = self.new_token(&mapping.id, "double");
                    if mapping.action.is_sustained() {
                        outcome.effects.push(Effect::StartHold {
                            token: token.clone(),
                            action: mapping.action.clone(),
                        });
                        tokens.push(token);
                    } else {
                        self.run_action(&mapping.action, &mut outcome.effects);
                    }
                } else {
                    match pending.fallback {
                        PendingFallback::Action(single) => {
                            self.run_action(&single.action, &mut outcome.effects);
                        }
                        PendingFallback::Replay => {
                            outcome.effects.push(Effect::Replay {
                                button,
                                direction: ReplayDirection::Down,
                            });
                            outcome.effects.push(Effect::Replay {
                                button,
                                direction: ReplayDirection::Up,
                            });
                        }
                        PendingFallback::Nothing => {}
                    }
                }
                self.consumed_until_up.push(ConsumedUntilUp { button, tokens });
                return;
            }
        }

        // A button only needs to be tracked if it participates as the primary
        // button in at least one enabled mapping. Everything else passes through
        // with zero latency.
        if !self.config.mappings.iter().any(|m| m.enabled && m.button == button) {
            return;
        }

        outcome.suppress = true;

        let tap_mapping = self
            .find_mapping(button, &Trigger::SingleClick)
            .filter(|m| m.action.is_sustained())
            .cloned();

        let mut held = HeldButton {
            button,
            down_time: _time,
            gesture_used: false,
            long_press_fired: false,
            tap_holds: Vec::new(),
            holds: Vec::new(),
            chords: Vec::new(),
        };

        if let Some(mapping) = tap_mapping {
            let token = self.new_token(&mapping.id, "tap");
            outcome.effects.push(Effect::StartHold {
                token: token.clone(),
                action: mapping.action.clone(),
            });
            held.tap_holds.push(token);
        }

        self.held.push(held);
    }

    fn on_button_up(&mut self, button: MouseButton, time: u64, outcome: &mut Outcome) {
        // Release a chord's secondary button.
        if let Some(pos) = self.chord_consumed.iter().position(|b| *b == button) {
            self.chord_consumed.remove(pos);
            outcome.suppress = true;
            let mut tokens = Vec::new();
            for held in &mut self.held {
                if let Some(chord) = held.chords.iter().position(|c| c.button == button) {
                    let chord = held.chords.remove(chord);
                    // The hold was registered on the primary button too; drop
                    // it there so a later primary release cannot release twice.
                    for token in &chord.tokens {
                        held.holds.retain(|candidate| candidate != token);
                    }
                    tokens.extend(chord.tokens);
                }
            }
            for token in tokens {
                outcome.effects.push(Effect::ReleaseHold { token });
            }
            return;
        }

        // Release the second press of a double-click action.
        if let Some(pos) = self.consumed_until_up.iter().position(|c| c.button == button) {
            let consumed = self.consumed_until_up.remove(pos);
            outcome.suppress = true;
            for token in consumed.tokens {
                outcome.effects.push(Effect::ReleaseHold { token });
            }
            return;
        }

        let Some(index) = self.held.iter().position(|h| h.button == button) else {
            // We never swallowed this button's down.
            return;
        };

        let held = self.held.remove(index);
        outcome.suppress = true;
        for token in held.tap_holds.iter().chain(held.holds.iter()) {
            outcome.effects.push(Effect::ReleaseHold { token: token.clone() });
        }

        if held.gesture_used || held.long_press_fired {
            return;
        }

        let double = self.find_mapping(button, &Trigger::DoubleClick).cloned();
        let release = self.find_mapping(button, &Trigger::Release).cloned();
        let single = self.find_mapping(button, &Trigger::SingleClick).cloned();

        // Choose at most one of the release-time categories using configured
        // priority. Gesture and long press have already been resolved.
        let mut candidates: Vec<(usize, RuleCategory)> = Vec::new();
        if double.is_some() {
            candidates.push((
                self.config.priority_rank(RuleCategory::DoubleClick),
                RuleCategory::DoubleClick,
            ));
        }
        if release.is_some() {
            candidates.push((
                self.config.priority_rank(RuleCategory::Release),
                RuleCategory::Release,
            ));
        }
        if single.is_some() {
            candidates.push((
                self.config.priority_rank(RuleCategory::SingleClick),
                RuleCategory::SingleClick,
            ));
        }
        candidates.sort_by_key(|(rank, _)| *rank);

        match candidates.first().map(|(_, category)| *category) {
            Some(RuleCategory::DoubleClick) => {
                let fallback = match single {
                    Some(mapping) if !mapping.action.is_sustained() => {
                        PendingFallback::Action(mapping)
                    }
                    // A sustained single-click action already ran between press
                    // and release; it must not fire a second time.
                    Some(_) => PendingFallback::Nothing,
                    None => PendingFallback::Replay,
                };
                self.pending = Some(PendingTap {
                    button,
                    deadline: time.saturating_add(self.config.double_click_ms),
                    fallback,
                });
            }
            Some(RuleCategory::Release) => {
                if let Some(mapping) = release {
                    self.run_action(&mapping.action, &mut outcome.effects);
                }
            }
            Some(RuleCategory::SingleClick) => {
                if let Some(mapping) = single {
                    if mapping.action.is_sustained() {
                        // Already started on button down; releasing the token
                        // above is the whole action.
                    } else {
                        self.run_action(&mapping.action, &mut outcome.effects);
                    }
                }
            }
            _ => {
                // Nothing matched. Replay the swallowed physical click so the
                // default button behaviour still reaches the foreground app.
                outcome.effects.push(Effect::Replay {
                    button,
                    direction: ReplayDirection::Down,
                });
                outcome.effects.push(Effect::Replay {
                    button,
                    direction: ReplayDirection::Up,
                });
            }
        }
    }

    fn on_wheel(&mut self, direction: ScrollDirection, outcome: &mut Outcome) {
        for index in (0..self.held.len()).rev() {
            let button = self.held[index].button;
            let mapping = self
                .find_mapping(button, &Trigger::WithWheel { direction })
                .cloned();
            if let Some(mapping) = mapping {
                outcome.suppress = true;
                let held = &mut self.held[index];
                held.gesture_used = true;
                Self::release_tokens(held, outcome);
                self.run_action(&mapping.action, &mut outcome.effects);
                return;
            }
        }
    }

    fn on_tick(&mut self, time: u64, outcome: &mut Outcome) {
        let long_press_ms = self.config.long_press_ms;
        for index in 0..self.held.len() {
            if self.held[index].gesture_used || self.held[index].long_press_fired {
                continue;
            }
            let button = self.held[index].button;
            if time < self.held[index].down_time.saturating_add(long_press_ms) {
                continue;
            }
            let mapping = self.find_mapping(button, &Trigger::LongPress).cloned();
            if let Some(mapping) = mapping {
                self.held[index].long_press_fired = true;
                let held = &mut self.held[index];
                Self::release_tokens(held, outcome);
                let token = self.new_token(&mapping.id, "long");
                if mapping.action.is_sustained() {
                    outcome.effects.push(Effect::StartHold {
                        token: token.clone(),
                        action: mapping.action.clone(),
                    });
                    self.held[index].holds.push(token);
                } else {
                    self.run_action(&mapping.action, &mut outcome.effects);
                }
            }
        }

        if self.pending.as_ref().is_some_and(|p| time >= p.deadline) {
            if let Some(pending) = self.pending.take() {
                Self::apply_fallback(pending.button, pending.fallback, &mut outcome.effects);
            }
        }
    }

    fn flush_expired_pending(&mut self, now: u64, effects: &mut Vec<Effect>) {
        if self.pending.as_ref().is_some_and(|p| now >= p.deadline) {
            if let Some(pending) = self.pending.take() {
                Self::apply_fallback(pending.button, pending.fallback, effects);
            }
        }
    }

    fn apply_fallback(button: MouseButton, fallback: PendingFallback, effects: &mut Vec<Effect>) {
        match fallback {
            PendingFallback::Action(mapping) => Self::push_run(&mapping.action, effects),
            PendingFallback::Replay => {
                effects.push(Effect::Replay {
                    button,
                    direction: ReplayDirection::Down,
                });
                effects.push(Effect::Replay {
                    button,
                    direction: ReplayDirection::Up,
                });
            }
            PendingFallback::Nothing => {}
        }
    }

    fn find_chord(&self, secondary: MouseButton) -> Option<usize> {
        for (index, held) in self.held.iter().enumerate().rev() {
            if held.button == secondary {
                continue;
            }
            if self
                .find_mapping(held.button, &Trigger::WithButton { button: secondary })
                .is_some()
            {
                return Some(index);
            }
        }
        None
    }

    fn find_mapping(&self, button: MouseButton, trigger: &Trigger) -> Option<&Mapping> {
        self.config
            .mappings
            .iter()
            .find(|mapping| mapping.matches(button, trigger))
    }

    fn has_mapping(&self, button: MouseButton, trigger: &Trigger) -> bool {
        self.find_mapping(button, trigger).is_some()
    }

    fn run_action(&self, action: &Action, effects: &mut Vec<Effect>) {
        Self::push_run(action, effects);
    }

    fn push_run(action: &Action, effects: &mut Vec<Effect>) {
        if !matches!(action, Action::None | Action::Disable) {
            effects.push(Effect::Run(action.clone()));
        }
    }

    fn release_all_tokens(&mut self, effects: &mut Vec<Effect>) {
        let mut tokens = Vec::new();
        for held in &mut self.held {
            tokens.extend(held.tap_holds.drain(..));
            tokens.extend(held.holds.drain(..));
            for chord in &mut held.chords {
                tokens.extend(chord.tokens.drain(..));
            }
        }
        for consumed in &mut self.consumed_until_up {
            tokens.extend(consumed.tokens.drain(..));
        }
        let mut seen = HashSet::new();
        for token in tokens {
            if seen.insert(token.clone()) {
                effects.push(Effect::ReleaseHold { token });
            }
        }
    }

    fn release_tokens(held: &mut HeldButton, outcome: &mut Outcome) {
        let mut seen = HashSet::new();
        for token in held.tap_holds.drain(..).chain(held.holds.drain(..)) {
            if seen.insert(token.clone()) {
                outcome.effects.push(Effect::ReleaseHold { token });
            }
        }
        for chord in &mut held.chords {
            for token in chord.tokens.drain(..) {
                if seen.insert(token.clone()) {
                    outcome.effects.push(Effect::ReleaseHold { token });
                }
            }
        }
    }

    fn new_token(&mut self, mapping_id: &str, kind: &str) -> String {
        self.seq = self.seq.wrapping_add(1);
        format!("{kind}:{mapping_id}:{}", self.seq)
    }
}

impl EngineConfig {
    fn priority_rank(&self, category: RuleCategory) -> usize {
        self.priority
            .iter()
            .position(|c| *c == category)
            .unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Action, Mapping, MouseButton, RuleCategory, ScrollDirection, Trigger};

    fn shortcut(keys: &[&str]) -> Action {
        Action::Shortcut {
            keys: keys.iter().map(|k| k.to_string()).collect(),
        }
    }

    fn hold_keys(keys: &[&str]) -> Action {
        Action::HoldKeys {
            keys: keys.iter().map(|k| k.to_string()).collect(),
        }
    }

    fn mapping(button: MouseButton, trigger: Trigger, action: Action) -> Mapping {
        let mut mapping = Mapping::new(button, trigger, action);
        mapping.id = format!("m{}", uuid::Uuid::new_v4());
        mapping
    }

    fn engine(mappings: Vec<Mapping>) -> InputEngine {
        InputEngine::new(EngineConfig {
            enabled: true,
            long_press_ms: 400,
            double_click_ms: 280,
            priority: RuleCategory::all(),
            mappings,
        })
    }

    fn runs(outcome: &Outcome) -> Vec<&Action> {
        outcome
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Run(action) => Some(action),
                _ => None,
            })
            .collect()
    }

    fn down(engine: &mut InputEngine, button: MouseButton, time: u64) -> Outcome {
        engine.handle(InputEvent::ButtonDown { button, time })
    }

    fn up(engine: &mut InputEngine, button: MouseButton, time: u64) -> Outcome {
        engine.handle(InputEvent::ButtonUp { button, time })
    }

    fn wheel(engine: &mut InputEngine, direction: ScrollDirection, time: u64) -> Outcome {
        engine.handle(InputEvent::Wheel { direction, time })
    }

    fn tick(engine: &mut InputEngine, time: u64) -> Outcome {
        engine.handle(InputEvent::Tick { time })
    }

    #[test]
    fn b5_tap_runs_on_release_and_swallows_both_events() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::SingleClick,
            shortcut(&["ctrl", "z"]),
        )]);

        let press = down(&mut engine, MouseButton::X2, 1_000);
        assert!(press.suppress);
        assert!(press.effects.is_empty());

        let release = up(&mut engine, MouseButton::X2, 1_120);
        assert!(release.suppress);
        assert_eq!(runs(&release), vec![&shortcut(&["ctrl", "z"])]);
    }

    #[test]
    fn b5_scroll_gesture_wins_and_cancels_the_tap() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "equal"]),
            ),
        ]);

        assert!(down(&mut engine, MouseButton::X2, 1_000).suppress);

        let gesture = wheel(&mut engine, ScrollDirection::Up, 1_100);
        assert!(gesture.suppress, "wheel must be consumed by the gesture");
        assert_eq!(runs(&gesture), vec![&shortcut(&["ctrl", "equal"])]);

        let release = up(&mut engine, MouseButton::X2, 1_200);
        assert!(release.suppress);
        assert!(runs(&release).is_empty(), "tap must not fire after a gesture");
    }

    #[test]
    fn repeated_scroll_gestures_each_execute() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Down },
                shortcut(&["ctrl", "minus"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        for offset in 0..3 {
            let gesture = wheel(&mut engine, ScrollDirection::Down, 1_100 + offset * 20);
            assert!(gesture.suppress);
            assert_eq!(runs(&gesture), vec![&shortcut(&["ctrl", "minus"])]);
        }
        let release = up(&mut engine, MouseButton::X2, 1_200);
        assert!(runs(&release).is_empty());
    }

    #[test]
    fn unmatched_wheel_passes_through_and_does_not_cancel_tap() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "equal"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let gesture = wheel(&mut engine, ScrollDirection::Down, 1_100);
        assert!(!gesture.suppress);
        let release = up(&mut engine, MouseButton::X2, 1_200);
        assert_eq!(runs(&release), vec![&shortcut(&["ctrl", "z"])]);
    }

    #[test]
    fn unmapped_buttons_pass_through_without_tracking() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::SingleClick,
            shortcut(&["ctrl", "z"]),
        )]);
        assert!(!down(&mut engine, MouseButton::Left, 1_000).suppress);
        assert!(!wheel(&mut engine, ScrollDirection::Up, 1_050).suppress);
        assert!(!up(&mut engine, MouseButton::Left, 1_100).suppress);
    }

    #[test]
    fn disabled_engine_never_swallows_input() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::SingleClick,
            shortcut(&["ctrl", "z"]),
        )]);
        engine.reconfigure(EngineConfig {
            enabled: false,
            ..EngineConfig::default()
        });
        assert!(!down(&mut engine, MouseButton::X2, 1_000).suppress);
        assert!(!up(&mut engine, MouseButton::X2, 1_100).suppress);
    }

    #[test]
    fn long_press_fires_at_threshold_and_cancels_tap() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::LongPress, shortcut(&["alt", "tab"])),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let not_yet = tick(&mut engine, 1_390);
        assert!(runs(&not_yet).is_empty());
        let fired = tick(&mut engine, 1_400);
        assert_eq!(runs(&fired), vec![&shortcut(&["alt", "tab"])]);
        let release = up(&mut engine, MouseButton::X2, 1_600);
        assert!(release.suppress);
        assert!(runs(&release).is_empty(), "tap must not fire after long press");
    }

    #[test]
    fn early_release_before_long_press_still_taps() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::LongPress, shortcut(&["alt", "tab"])),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let release = up(&mut engine, MouseButton::X2, 1_200);
        assert_eq!(runs(&release), vec![&shortcut(&["ctrl", "z"])]);
        assert!(tick(&mut engine, 1_500).effects.is_empty());
    }

    #[test]
    fn long_press_before_gesture_is_cancelled_by_wheel() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::LongPress, shortcut(&["alt", "tab"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "equal"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let gesture = wheel(&mut engine, ScrollDirection::Up, 1_200);
        assert_eq!(runs(&gesture), vec![&shortcut(&["ctrl", "equal"])]);
        assert!(tick(&mut engine, 1_500).effects.is_empty());
    }

    #[test]
    fn double_click_executes_only_double_action() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::DoubleClick, shortcut(&["ctrl", "y"])),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let first_up = up(&mut engine, MouseButton::X2, 1_050);
        assert!(first_up.suppress);
        assert!(runs(&first_up).is_empty());

        let second_down = down(&mut engine, MouseButton::X2, 1_180);
        assert_eq!(runs(&second_down), vec![&shortcut(&["ctrl", "y"])]);
        let second_up = up(&mut engine, MouseButton::X2, 1_220);
        assert!(second_up.suppress);
        assert!(runs(&second_up).is_empty());

        assert!(tick(&mut engine, 2_000).effects.is_empty());
    }

    #[test]
    fn single_click_with_double_mapping_fires_after_window() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::DoubleClick, shortcut(&["ctrl", "y"])),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let first_up = up(&mut engine, MouseButton::X2, 1_050);
        assert!(runs(&first_up).is_empty());
        assert_eq!(engine.next_deadline(), Some(1_330));
        let timeout = tick(&mut engine, 1_330);
        assert_eq!(runs(&timeout), vec![&shortcut(&["ctrl", "z"])]);
    }

    #[test]
    fn double_only_mapping_replays_single_click_after_timeout() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::DoubleClick,
            shortcut(&["ctrl", "y"]),
        )]);
        down(&mut engine, MouseButton::X2, 1_000);
        up(&mut engine, MouseButton::X2, 1_050);
        let timeout = tick(&mut engine, 1_330);
        assert_eq!(
            timeout.effects,
            vec![
                Effect::Replay { button: MouseButton::X2, direction: ReplayDirection::Down },
                Effect::Replay { button: MouseButton::X2, direction: ReplayDirection::Up },
            ]
        );
    }

    #[test]
    fn button_chord_is_consumed_and_cancels_the_tap() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(
                MouseButton::X2,
                Trigger::WithButton { button: MouseButton::Left },
                shortcut(&["ctrl", "c"]),
            ),
        ]);
        assert!(down(&mut engine, MouseButton::X2, 1_000).suppress);
        let chord = down(&mut engine, MouseButton::Left, 1_080);
        assert!(chord.suppress);
        assert_eq!(runs(&chord), vec![&shortcut(&["ctrl", "c"])]);
        assert!(up(&mut engine, MouseButton::Left, 1_120).suppress);
        let release = up(&mut engine, MouseButton::X2, 1_200);
        assert!(release.suppress);
        assert!(runs(&release).is_empty());
    }

    #[test]
    fn chord_mapping_beats_secondary_buttons_own_mapping() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X1, Trigger::SingleClick, shortcut(&["ctrl", "c"])),
            mapping(
                MouseButton::X2,
                Trigger::WithButton { button: MouseButton::X1 },
                shortcut(&["ctrl", "v"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let chord = down(&mut engine, MouseButton::X1, 1_050);
        assert_eq!(runs(&chord), vec![&shortcut(&["ctrl", "v"])]);
        assert!(up(&mut engine, MouseButton::X1, 1_100).suppress);
        assert!(runs(&up(&mut engine, MouseButton::X2, 1_150)).is_empty());
    }

    #[test]
    fn unmapped_gesture_button_replays_default_click() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::WithWheel { direction: ScrollDirection::Up },
            shortcut(&["ctrl", "equal"]),
        )]);
        down(&mut engine, MouseButton::X2, 1_000);
        let release = up(&mut engine, MouseButton::X2, 1_050);
        assert!(release.suppress);
        assert_eq!(
            release.effects,
            vec![
                Effect::Replay { button: MouseButton::X2, direction: ReplayDirection::Down },
                Effect::Replay { button: MouseButton::X2, direction: ReplayDirection::Up },
            ]
        );
    }

    #[test]
    fn release_trigger_runs_on_release() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::Release,
            shortcut(&["ctrl", "s"]),
        )]);
        down(&mut engine, MouseButton::X2, 1_000);
        let release = up(&mut engine, MouseButton::X2, 1_300);
        assert_eq!(runs(&release), vec![&shortcut(&["ctrl", "s"])]);
    }

    #[test]
    fn sustained_tap_action_holds_until_release() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::SingleClick,
            hold_keys(&["ctrl"]),
        )]);
        let press = down(&mut engine, MouseButton::X2, 1_000);
        assert!(press.suppress);
        let token = match press.effects.as_slice() {
            [Effect::StartHold { token, .. }] => token.clone(),
            other => panic!("expected one StartHold, got {other:?}"),
        };
        let release = up(&mut engine, MouseButton::X2, 1_100);
        assert_eq!(release.effects, vec![Effect::ReleaseHold { token }]);
        assert!(runs(&release).is_empty());
    }

    #[test]
    fn gesture_cancels_sustained_tap_hold() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, hold_keys(&["ctrl"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "equal"]),
            ),
        ]);
        let press = down(&mut engine, MouseButton::X2, 1_000);
        assert!(matches!(press.effects.as_slice(), [Effect::StartHold { .. }]));
        let gesture = wheel(&mut engine, ScrollDirection::Up, 1_100);
        assert!(matches!(gesture.effects.first(), Some(Effect::ReleaseHold { .. })));
        assert_eq!(runs(&gesture), vec![&shortcut(&["ctrl", "equal"])]);
        let release = up(&mut engine, MouseButton::X2, 1_200);
        assert!(release.effects.is_empty());
    }

    #[test]
    fn sustained_long_press_holds_until_release() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::LongPress,
            hold_keys(&["alt"]),
        )]);
        down(&mut engine, MouseButton::X2, 1_000);
        let fired = tick(&mut engine, 1_400);
        assert!(matches!(fired.effects.as_slice(), [Effect::StartHold { .. }]));
        let release = up(&mut engine, MouseButton::X2, 1_700);
        assert!(matches!(release.effects.as_slice(), [Effect::ReleaseHold { .. }]));
    }

    #[test]
    fn cancel_all_releases_every_hold() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::SingleClick,
            hold_keys(&["ctrl"]),
        )]);
        down(&mut engine, MouseButton::X2, 1_000);
        let effects = engine.cancel_all();
        assert!(matches!(effects.as_slice(), [Effect::ReleaseHold { .. }]));
        assert!(engine.cancel_all().is_empty());
    }

    #[test]
    fn reconfigure_releases_holds_and_clears_state() {
        let mut engine = engine(vec![mapping(
            MouseButton::X2,
            Trigger::SingleClick,
            hold_keys(&["ctrl"]),
        )]);
        down(&mut engine, MouseButton::X2, 1_000);
        let effects = engine.reconfigure(EngineConfig::default());
        assert!(matches!(effects.as_slice(), [Effect::ReleaseHold { .. }]));
        assert!(!down(&mut engine, MouseButton::X2, 1_200).suppress);
    }

    #[test]
    fn most_recent_held_button_wins_wheel_gesture() {
        let mut engine = engine(vec![
            mapping(
                MouseButton::X1,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "c"]),
            ),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "v"]),
            ),
        ]);
        down(&mut engine, MouseButton::X1, 1_000);
        down(&mut engine, MouseButton::X2, 1_020);
        let gesture = wheel(&mut engine, ScrollDirection::Up, 1_050);
        assert_eq!(runs(&gesture), vec![&shortcut(&["ctrl", "v"])]);
    }

    #[test]
    fn priority_setting_can_prefer_single_click_over_release() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::Release, shortcut(&["ctrl", "s"])),
        ]);
        engine.reconfigure(EngineConfig {
            enabled: true,
            long_press_ms: 400,
            double_click_ms: 280,
            priority: vec![
                RuleCategory::Gesture,
                RuleCategory::LongPress,
                RuleCategory::DoubleClick,
                RuleCategory::SingleClick,
                RuleCategory::Release,
            ],
            mappings: engine.config.mappings.clone(),
        });
        down(&mut engine, MouseButton::X2, 1_000);
        let release = up(&mut engine, MouseButton::X2, 1_100);
        assert_eq!(runs(&release), vec![&shortcut(&["ctrl", "z"])]);
    }

    #[test]
    fn pending_double_is_flushed_when_next_press_starts_after_deadline() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::DoubleClick, shortcut(&["ctrl", "y"])),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        up(&mut engine, MouseButton::X2, 1_050);
        let next_down = down(&mut engine, MouseButton::X2, 1_400);
        assert_eq!(runs(&next_down), vec![&shortcut(&["ctrl", "z"])]);
        assert!(next_down.suppress);
        let next_up = up(&mut engine, MouseButton::X2, 1_450);
        assert!(runs(&next_up).is_empty());
    }

    #[test]
    fn simultaneous_buttons_keep_independent_taps() {
        let mut engine = engine(vec![
            mapping(MouseButton::X1, Trigger::SingleClick, shortcut(&["ctrl", "c"])),
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "v"])),
        ]);
        down(&mut engine, MouseButton::X1, 1_000);
        down(&mut engine, MouseButton::X2, 1_010);
        let first = up(&mut engine, MouseButton::X1, 1_100);
        assert_eq!(runs(&first), vec![&shortcut(&["ctrl", "c"])]);
        let second = up(&mut engine, MouseButton::X2, 1_120);
        assert_eq!(runs(&second), vec![&shortcut(&["ctrl", "v"])]);
    }

    #[test]
    fn timeouts_are_reported_only_while_relevant() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::LongPress, shortcut(&["alt", "tab"])),
        ]);
        assert_eq!(engine.next_deadline(), None);
        down(&mut engine, MouseButton::X2, 1_000);
        assert_eq!(engine.next_deadline(), Some(1_400));
        tick(&mut engine, 1_400);
        assert_eq!(engine.next_deadline(), None);
    }

    #[test]
    fn priority_can_prefer_release_over_double_click() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(MouseButton::X2, Trigger::DoubleClick, shortcut(&["ctrl", "y"])),
            mapping(MouseButton::X2, Trigger::Release, shortcut(&["ctrl", "s"])),
        ]);
        engine.reconfigure(EngineConfig {
            enabled: true,
            long_press_ms: 400,
            double_click_ms: 280,
            priority: vec![
                RuleCategory::Gesture,
                RuleCategory::LongPress,
                RuleCategory::Release,
                RuleCategory::DoubleClick,
                RuleCategory::SingleClick,
            ],
            mappings: engine.config.mappings.clone(),
        });
        down(&mut engine, MouseButton::X2, 1_000);
        let release = up(&mut engine, MouseButton::X2, 1_100);
        assert_eq!(runs(&release), vec![&shortcut(&["ctrl", "s"])]);
        assert_eq!(engine.next_deadline(), None);
    }

    #[test]
    fn horizontal_wheel_gestures_are_consumed() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Right },
                shortcut(&["alt", "right"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let gesture = wheel(&mut engine, ScrollDirection::Right, 1_100);
        assert!(gesture.suppress);
        assert_eq!(runs(&gesture), vec![&shortcut(&["alt", "right"])]);
        assert!(runs(&up(&mut engine, MouseButton::X2, 1_200)).is_empty());
    }

    #[test]
    fn chord_hold_is_released_when_the_second_button_is_released() {
        let mut engine = engine(vec![
            mapping(
                MouseButton::X2,
                Trigger::WithButton { button: MouseButton::Left },
                hold_keys(&["ctrl"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let chord = down(&mut engine, MouseButton::Left, 1_050);
        assert!(matches!(chord.effects.as_slice(), [Effect::StartHold { .. }]));
        let secondary_up = up(&mut engine, MouseButton::Left, 1_100);
        assert!(matches!(
            secondary_up.effects.as_slice(),
            [Effect::ReleaseHold { .. }]
        ));
        assert!(up(&mut engine, MouseButton::X2, 1_200).effects.is_empty());
    }

    #[test]
    fn simultaneous_long_presses_both_fire() {
        let mut engine = engine(vec![
            mapping(MouseButton::X1, Trigger::LongPress, shortcut(&["ctrl", "c"])),
            mapping(MouseButton::X2, Trigger::LongPress, shortcut(&["ctrl", "v"])),
        ]);
        down(&mut engine, MouseButton::X1, 1_000);
        down(&mut engine, MouseButton::X2, 1_010);
        let fired = tick(&mut engine, 1_410);
        assert_eq!(
            runs(&fired),
            vec![&shortcut(&["ctrl", "c"]), &shortcut(&["ctrl", "v"])]
        );
    }

    #[test]
    fn long_press_releases_a_sustained_tap_hold() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, hold_keys(&["ctrl"])),
            mapping(MouseButton::X2, Trigger::LongPress, shortcut(&["alt", "tab"])),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        let fired = tick(&mut engine, 1_400);
        assert!(matches!(fired.effects.first(), Some(Effect::ReleaseHold { .. })));
        assert_eq!(runs(&fired), vec![&shortcut(&["alt", "tab"])]);
        assert!(up(&mut engine, MouseButton::X2, 1_500).effects.is_empty());
    }

    #[test]
    fn gesture_then_tap_works_on_the_next_press() {
        let mut engine = engine(vec![
            mapping(MouseButton::X2, Trigger::SingleClick, shortcut(&["ctrl", "z"])),
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "equal"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        wheel(&mut engine, ScrollDirection::Up, 1_050);
        up(&mut engine, MouseButton::X2, 1_100);

        down(&mut engine, MouseButton::X2, 1_200);
        let tap = up(&mut engine, MouseButton::X2, 1_260);
        assert_eq!(runs(&tap), vec![&shortcut(&["ctrl", "z"])]);
    }

    #[test]
    fn disabling_mid_gesture_releases_holds_and_passes_through() {
        let mut engine = engine(vec![
            mapping(
                MouseButton::X2,
                Trigger::WithButton { button: MouseButton::Left },
                hold_keys(&["ctrl"]),
            ),
        ]);
        down(&mut engine, MouseButton::X2, 1_000);
        down(&mut engine, MouseButton::Left, 1_040);
        let effects = engine.reconfigure(EngineConfig {
            enabled: false,
            mappings: engine.config.mappings.clone(),
            ..EngineConfig::default()
        });
        assert!(matches!(effects.as_slice(), [Effect::ReleaseHold { .. }]));
        assert!(!up(&mut engine, MouseButton::Left, 1_100).suppress);
        assert!(!up(&mut engine, MouseButton::X2, 1_120).suppress);

        let reenabled = EngineConfig {
            enabled: true,
            mappings: engine.config.mappings.clone(),
            ..EngineConfig::default()
        };
        engine.reconfigure(reenabled);
        assert!(down(&mut engine, MouseButton::X1, 1_200).suppress == false);
        assert!(down(&mut engine, MouseButton::X2, 1_220).suppress);
    }

    #[test]
    fn unmapped_button_held_does_not_block_gestures_on_another_button() {
        let mut engine = engine(vec![
            mapping(
                MouseButton::X2,
                Trigger::WithWheel { direction: ScrollDirection::Up },
                shortcut(&["ctrl", "equal"]),
            ),
        ]);
        assert!(!down(&mut engine, MouseButton::X1, 1_000).suppress);
        assert!(down(&mut engine, MouseButton::X2, 1_020).suppress);
        let gesture = wheel(&mut engine, ScrollDirection::Up, 1_050);
        assert!(gesture.suppress);
        assert_eq!(runs(&gesture), vec![&shortcut(&["ctrl", "equal"])]);
    }
}
