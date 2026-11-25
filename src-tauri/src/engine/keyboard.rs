//! Pure keyboard remapping state machine.
//!
//! Like the mouse engine it consumes abstract events and returns a suppression
//! decision plus [`Effect`]s, so the behaviour is testable without a hook.
//!
//! Rules:
//!
//! 1. A mapping matches only when its exact modifier set and key are down.
//! 2. The first key-down fires the mapping; auto-repeat is swallowed and never
//!    fires twice.
//! 3. A remap holds its target keys until the source is released.
//! 4. Unmatched keys always pass through untouched.

use std::collections::HashMap;

use crate::config::model::{Action, KeyBehaviour, KeyMapping};
use crate::engine::keys::{is_modifier_name, name_from_vk, normalize_combo, vk_from_name};
use crate::engine::{Effect, Outcome};

#[derive(Debug, Default)]
pub struct KeyboardEngine {
    enabled: bool,
    bindings: HashMap<u16, Vec<KeyMapping>>,
    /// Source keys currently down, mapped to the hold they started (remaps and
    /// sustained actions) or `None` for a one-shot action.
    active: HashMap<u16, Option<String>>,
    seq: u64,
}

impl KeyboardEngine {
    pub fn new(enabled: bool, mappings: &[KeyMapping]) -> Self {
        let mut engine = Self::default();
        engine.reconfigure(enabled, mappings);
        engine
    }

    /// Replaces the bindings. Releasing the old holds is the caller's job: the
    /// returned effects must be executed or synthetic keys would stay down.
    pub fn reconfigure(&mut self, enabled: bool, mappings: &[KeyMapping]) -> Vec<Effect> {
        let effects = self.release_all();
        self.enabled = enabled;
        self.bindings.clear();
        if !enabled {
            return effects;
        }
        for mapping in mappings {
            if !mapping.enabled {
                continue;
            }
            let Some(key) = mapping.source_key() else {
                continue;
            };
            let Some(vk) = vk_from_name(key) else {
                continue;
            };
            self.bindings
                .entry(vk)
                .or_default()
                .push(mapping.clone());
        }
        effects
    }

    /// Releases every synthetic hold this engine started.
    pub fn release_all(&mut self) -> Vec<Effect> {
        self.active
            .drain()
            .filter_map(|(_, token)| token)
            .map(|token| Effect::ReleaseHold { token })
            .collect()
    }

    pub fn is_active(&self) -> bool {
        self.enabled
    }

    /// Handles one key event. `modifiers` are the physical modifier names held
    /// at the moment of the event, excluding the key itself.
    pub fn handle(&mut self, vk: u16, is_down: bool, modifiers: &[String]) -> Outcome {
        let mut outcome = Outcome::default();

        if !is_down {
            // Release first: a mapping may have been removed or disabled while
            // its source was still held, and a target must never stay down.
            if let Some(token) = self.active.remove(&vk) {
                outcome.suppress = true;
                if let Some(token) = token {
                    outcome.effects.push(Effect::ReleaseHold { token });
                }
            }
            return outcome;
        }

        if !self.enabled {
            return outcome;
        }
        let Some(mapping) = self.match_mapping(vk, modifiers).cloned() else {
            return outcome;
        };

        outcome.suppress = true;
        if self.active.contains_key(&vk) {
            // Windows auto-repeat: keep swallowing it, but do not fire again.
            return outcome;
        }

        match mapping.behaviour {
            KeyBehaviour::Remap { target } => {
                let token = self.new_token(&mapping.id);
                outcome.effects.push(Effect::StartHold {
                    token: token.clone(),
                    action: Action::HoldKeys { keys: target },
                });
                self.active.insert(vk, Some(token));
            }
            KeyBehaviour::Action { action } => {
                if action.is_sustained() {
                    let token = self.new_token(&mapping.id);
                    outcome.effects.push(Effect::StartHold {
                        token: token.clone(),
                        action,
                    });
                    self.active.insert(vk, Some(token));
                } else {
                    outcome.effects.push(Effect::Run {
                        action,
                        mapping_id: mapping.id.clone(),
                    });
                    self.active.insert(vk, None);
                }
            }
        }
        outcome
    }

    fn match_mapping(&self, vk: u16, modifiers: &[String]) -> Option<&KeyMapping> {
        let candidates = self.bindings.get(&vk)?;
        let mut combo: Vec<String> = modifiers
            .iter()
            .filter(|key| is_modifier_name(key))
            .cloned()
            .collect();
        combo.push(name_from_vk(vk));
        let combo = normalize_combo(&combo);
        candidates
            .iter()
            .find(|mapping| mapping.enabled && mapping.source == combo)
    }

    fn new_token(&mut self, mapping_id: &str) -> String {
        self.seq = self.seq.wrapping_add(1);
        format!("key:{mapping_id}:{}", self.seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shortcut(keys: &[&str]) -> Action {
        Action::Shortcut {
            keys: keys.iter().map(|key| key.to_string()).collect(),
        }
    }

    fn action_mapping(source: &[&str], action: Action) -> KeyMapping {
        KeyMapping::new(
            source.iter().map(|key| key.to_string()).collect(),
            KeyBehaviour::Action { action },
        )
    }

    fn remap_mapping(source: &[&str], target: &[&str]) -> KeyMapping {
        KeyMapping::new(
            source.iter().map(|key| key.to_string()).collect(),
            KeyBehaviour::Remap {
                target: target.iter().map(|key| key.to_string()).collect(),
            },
        )
    }

    fn mods(names: &[&str]) -> Vec<String> {
        names.iter().map(|key| key.to_string()).collect()
    }

    fn down(engine: &mut KeyboardEngine, vk: u16, modifiers: &[&str]) -> Outcome {
        engine.handle(vk, true, &mods(modifiers))
    }

    fn up(engine: &mut KeyboardEngine, vk: u16, modifiers: &[&str]) -> Outcome {
        engine.handle(vk, false, &mods(modifiers))
    }

    fn runs(outcome: &Outcome) -> Vec<String> {
        outcome
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Run { mapping_id, .. } => Some(mapping_id.clone()),
                _ => None,
            })
            .collect()
    }

    const F12: u16 = 0x7B;
    const F11: u16 = 0x7A;
    const CAPS: u16 = 0x14;

    #[test]
    fn a_plain_hotkey_fires_once_and_swallows_repeats() {
        let mapping = action_mapping(&["f12"], shortcut(&["ctrl", "c"]));
        let id = mapping.id.clone();
        let mut engine = KeyboardEngine::new(true, &[mapping]);

        let first = down(&mut engine, F12, &[]);
        assert!(first.suppress);
        assert_eq!(runs(&first), vec![id.clone()]);

        let repeat = down(&mut engine, F12, &[]);
        assert!(repeat.suppress, "auto-repeat must not reach the app");
        assert!(runs(&repeat).is_empty(), "and must not fire twice");

        let release = up(&mut engine, F12, &[]);
        assert!(release.suppress);
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let mut engine = KeyboardEngine::new(
            true,
            &[action_mapping(&["ctrl", "f12"], shortcut(&["a"]))],
        );

        // Plain F12 is a different source: it must pass through.
        assert!(!down(&mut engine, F12, &[]).suppress);
        assert!(!up(&mut engine, F12, &[]).suppress);

        let combo = down(&mut engine, F12, &["ctrl"]);
        assert!(combo.suppress);
        assert!(!runs(&combo).is_empty());
    }

    #[test]
    fn remap_holds_the_target_until_the_source_is_released() {
        let mapping = remap_mapping(&["capslock"], &["ctrl"]);
        let id = mapping.id.clone();
        let mut engine = KeyboardEngine::new(true, &[mapping]);

        let press = down(&mut engine, CAPS, &[]);
        assert!(press.suppress);
        match press.effects.as_slice() {
            [Effect::StartHold { token, action }] => {
                assert!(token.starts_with(&format!("key:{id}:")));
                assert_eq!(
                    action,
                    &Action::HoldKeys {
                        keys: vec!["ctrl".into()]
                    }
                );
                assert!(action.is_sustained());
            }
            other => panic!("expected a hold, got {other:?}"),
        }

        let release = up(&mut engine, CAPS, &[]);
        assert!(release.suppress);
        assert!(matches!(release.effects.as_slice(), [Effect::ReleaseHold { .. }]));
    }

    #[test]
    fn sustained_actions_release_on_key_up_but_one_shots_do_not() {
        let hold = action_mapping(
            &["f11"],
            Action::HoldKeys {
                keys: vec!["shift".into()],
            },
        );
        let mut engine = KeyboardEngine::new(true, &[hold]);

        let press = down(&mut engine, F11, &[]);
        assert!(matches!(press.effects.as_slice(), [Effect::StartHold { .. }]));
        let release = up(&mut engine, F11, &[]);
        assert!(matches!(release.effects.as_slice(), [Effect::ReleaseHold { .. }]));
    }

    #[test]
    fn unmatched_keys_pass_through() {
        let mut engine =
            KeyboardEngine::new(true, &[action_mapping(&["f12"], shortcut(&["a"]))]);

        assert!(!down(&mut engine, 0x41, &[]).suppress, "letter a");
        assert!(!up(&mut engine, 0x41, &[]).suppress);
    }

    #[test]
    fn disabled_engine_never_suppresses() {
        let mut engine =
            KeyboardEngine::new(false, &[action_mapping(&["f12"], shortcut(&["a"]))]);

        assert!(!down(&mut engine, F12, &[]).suppress);
        assert!(!up(&mut engine, F12, &[]).suppress);
    }

    #[test]
    fn reconfigure_releases_held_targets() {
        let mapping = remap_mapping(&["capslock"], &["ctrl"]);
        let mut engine = KeyboardEngine::new(true, &[mapping]);

        down(&mut engine, CAPS, &[]);
        let effects = engine.reconfigure(false, &[]);
        assert!(matches!(effects.as_slice(), [Effect::ReleaseHold { .. }]));

        // The source is no longer tracked, so its release passes through.
        assert!(!up(&mut engine, CAPS, &[]).suppress);
    }

    #[test]
    fn a_release_is_suppressed_even_if_the_mapping_disappeared() {
        let mapping = remap_mapping(&["capslock"], &["ctrl"]);
        let mut engine = KeyboardEngine::new(true, &[mapping]);

        down(&mut engine, CAPS, &[]);
        engine.reconfigure(true, &[]);
        // reconfigure already released and cleared the hold.
        assert!(!up(&mut engine, CAPS, &[]).suppress);
    }
}
