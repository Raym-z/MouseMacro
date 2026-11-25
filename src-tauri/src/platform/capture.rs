//! Capture sessions for the “Record” controls.
//!
//! A session is owned by the backend: it buffers every event the hooks see, so
//! the UI can attach late, re-render or reconnect without losing the input it
//! is trying to record. Only the first mouse gesture is suppressed, so the
//! click being recorded cannot act on the editor while Stop stays clickable.

use std::collections::VecDeque;

use serde::Serialize;

use crate::config::model::{MouseButton, ScrollDirection};
use crate::engine::keys::is_modifier_name;

/// Sessions end on their own after this long so a lost UI cannot leave input
/// swallowed forever.
pub const CAPTURE_TIMEOUT_MS: u64 = 20_000;

/// Events kept per session. Enough for modifier juggling and a few retries.
pub const MAX_CAPTURE_EVENTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Mouse,
    Keyboard,
}

impl CaptureKind {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "mouse" => Some(Self::Mouse),
            "keyboard" => Some(Self::Keyboard),
            _ => None,
        }
    }
}

/// How the backend treated an event, shown as a badge in the capture log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureDecision {
    /// Became the recorded value.
    Selected,
    /// Seen and usable, but not the selection (or a later event).
    Seen,
    /// Rejected by the injected-input setting.
    Filtered,
    /// MouseMacro's own synthetic input.
    Own,
}

#[derive(Debug, Clone, Copy)]
pub struct InputMeta {
    pub injected: bool,
    pub own: bool,
    /// Passed the injected-input filter.
    pub accepted: bool,
}

impl InputMeta {
    #[allow(dead_code)]
    pub fn physical() -> Self {
        Self {
            injected: false,
            own: false,
            accepted: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEvent {
    pub seq: u64,
    pub at_ms: u64,
    pub source: &'static str,
    pub kind: &'static str,
    pub button: Option<MouseButton>,
    pub direction: Option<ScrollDirection>,
    pub vk: Option<u16>,
    pub token: Option<String>,
    pub modifiers: Vec<String>,
    pub injected: bool,
    pub own: bool,
    pub repeat: bool,
    pub decision: CaptureDecision,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSessionInfo {
    pub id: u64,
    pub events: Vec<CaptureEvent>,
}

#[derive(Debug)]
struct CaptureSession {
    id: u64,
    kind: CaptureKind,
    started_ms: u64,
    seq: u64,
    events: VecDeque<CaptureEvent>,
    /// A mouse button has been selected; later downs are just `Seen` so the
    /// click on “Stop” cannot overwrite the choice.
    mouse_selected: bool,
    /// Keys currently down, to recognise auto-repeat.
    held_keys: Vec<u16>,
}

impl CaptureSession {
    fn new(id: u64, kind: CaptureKind, now: u64) -> Self {
        Self {
            id,
            kind,
            started_ms: now,
            seq: 0,
            events: VecDeque::new(),
            mouse_selected: false,
            held_keys: Vec::new(),
        }
    }

    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    fn push(&mut self, event: CaptureEvent) {
        self.events.push_back(event);
        while self.events.len() > MAX_CAPTURE_EVENTS {
            self.events.pop_front();
        }
    }
}

/// Shared capture state: the active session plus delayed suppression that must
/// outlive it (a swallowed down whose up has not arrived yet).
#[derive(Debug, Default)]
pub struct CaptureRuntime {
    next_id: u64,
    session: Option<CaptureSession>,
    /// Button whose down was swallowed; its up must be swallowed too.
    suppress_up: Option<MouseButton>,
    /// Key whose down was swallowed to stop a session (Escape).
    suppress_key_up: Option<u16>,
}

impl CaptureRuntime {
    pub fn start(&mut self, kind: CaptureKind, now: u64) -> CaptureSessionInfo {
        self.next_id += 1;
        let id = self.next_id;
        let session = CaptureSession::new(id, kind, now);
        self.session = Some(session);
        self.suppress_key_up = None;
        CaptureSessionInfo {
            id,
            events: Vec::new(),
        }
    }

    /// Stops the session with this id. Stale ids are ignored so an unmounting
    /// component cannot kill the session that replaced it.
    pub fn stop(&mut self, id: u64) -> bool {
        if self.session.as_ref().is_some_and(|session| session.id == id) {
            self.session = None;
            return true;
        }
        false
    }

    /// Stops whatever is active and returns its id, used by replacements,
    /// Escape and the watchdog.
    pub fn stop_any(&mut self) -> Option<(u64, CaptureKind)> {
        let session = self.session.take()?;
        Some((session.id, session.kind))
    }

    pub fn expire(&mut self, now: u64) -> Option<(u64, CaptureKind)> {
        let expired = self.session.as_ref().is_some_and(|session| {
            now.saturating_sub(session.started_ms) >= CAPTURE_TIMEOUT_MS
        });
        if expired {
            self.stop_any()
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn session_id(&self) -> Option<u64> {
        self.session.as_ref().map(|session| session.id)
    }

    pub fn session_kind(&self) -> Option<CaptureKind> {
        self.session.as_ref().map(|session| session.kind)
    }

    /// True while a swallowed mouse down is still waiting for its up.
    #[allow(dead_code)]
    pub fn pending_mouse_up(&self) -> Option<MouseButton> {
        self.suppress_up
    }

    /// Consumes the pending up suppression for this button.
    pub fn take_mouse_up(&mut self, button: MouseButton) -> bool {
        if self.suppress_up == Some(button) {
            self.suppress_up = None;
            true
        } else {
            false
        }
    }

    pub fn take_key_up(&mut self, vk: u16) -> bool {
        if self.suppress_key_up == Some(vk) {
            self.suppress_key_up = None;
            true
        } else {
            false
        }
    }

    /// Records a mouse event if a mouse session is active. Returns the session
    /// id, the event for the UI and whether the hook must swallow it.
    pub fn record_mouse(
        &mut self,
        now: u64,
        button: Option<MouseButton>,
        is_down: bool,
        wheel: Option<ScrollDirection>,
        meta: InputMeta,
    ) -> Option<(u64, CaptureEvent, bool)> {
        let session = self.session.as_ref()?;
        if session.kind != CaptureKind::Mouse {
            return None;
        }
        let id = session.id;
        let kind = if wheel.is_some() {
            "wheel"
        } else if is_down {
            "down"
        } else {
            "up"
        };
        let decision = if meta.own {
            CaptureDecision::Own
        } else if !meta.accepted {
            CaptureDecision::Filtered
        } else if wheel.is_none() && is_down && button.is_some() && !session.mouse_selected {
            CaptureDecision::Selected
        } else {
            CaptureDecision::Seen
        };

        let selected = decision == CaptureDecision::Selected;
        if selected {
            self.suppress_up = button;
        }

        let seq = self.session.as_mut().map(|session| session.next_seq())?;
        if let Some(session) = self.session.as_mut() {
            if selected {
                session.mouse_selected = true;
            }
        }
        let event = CaptureEvent {
            seq,
            at_ms: now,
            source: "mouse",
            kind,
            button,
            direction: wheel,
            vk: None,
            token: None,
            modifiers: Vec::new(),
            injected: meta.injected,
            own: meta.own,
            repeat: false,
            decision,
        };
        if let Some(session) = self.session.as_mut() {
            session.push(event.clone());
        }
        Some((id, event, selected))
    }

    /// Records a keyboard event if a keyboard session is active. Every key is
    /// swallowed while the session runs, so the combination cannot leak into
    /// the focused application.
    pub fn record_key(
        &mut self,
        now: u64,
        vk: u16,
        is_down: bool,
        meta: InputMeta,
        modifiers: Vec<String>,
    ) -> Option<(u64, CaptureEvent)> {
        let session = self.session.as_ref()?;
        if session.kind != CaptureKind::Keyboard {
            return None;
        }
        let id = session.id;
        let token = crate::engine::keys::name_from_vk(vk);
        let modifier_key = is_modifier_name(&token);
        let repeat = is_down && session.held_keys.contains(&vk);

        let decision = if meta.own {
            CaptureDecision::Own
        } else if !meta.accepted {
            CaptureDecision::Filtered
        } else if is_down && !modifier_key && !repeat {
            CaptureDecision::Selected
        } else {
            CaptureDecision::Seen
        };

        let seq = self.session.as_mut()?.next_seq();
        if let Some(session) = self.session.as_mut() {
            if is_down {
                if !session.held_keys.contains(&vk) {
                    session.held_keys.push(vk);
                }
            } else {
                session.held_keys.retain(|held| *held != vk);
            }
        }
        let event = CaptureEvent {
            seq,
            at_ms: now,
            source: "keyboard",
            kind: if is_down { "down" } else { "up" },
            button: None,
            direction: None,
            vk: Some(vk),
            token: Some(token),
            modifiers,
            injected: meta.injected,
            own: meta.own,
            repeat,
            decision,
        };
        if let Some(session) = self.session.as_mut() {
            session.push(event.clone());
        }
        Some((id, event))
    }

    pub fn events_since(&self, id: u64, since_seq: u64) -> Option<Vec<CaptureEvent>> {
        let session = self.session.as_ref()?;
        if session.id != id {
            return None;
        }
        Some(
            session
                .events
                .iter()
                .filter(|event| event.seq > since_seq)
                .cloned()
                .collect(),
        )
    }

    pub fn arm_key_up_suppression(&mut self, vk: u16) {
        self.suppress_key_up = Some(vk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> CaptureRuntime {
        CaptureRuntime::default()
    }

    #[test]
    fn only_the_first_mouse_down_is_selected_and_suppressed() {
        let mut runtime = runtime();
        let info = runtime.start(CaptureKind::Mouse, 1_000);
        assert_eq!(info.id, 1);

        let (id, first, suppress) = runtime
            .record_mouse(
                1_010,
                Some(MouseButton::Left),
                true,
                None,
                InputMeta::physical(),
            )
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(first.decision, CaptureDecision::Selected);
        assert!(suppress, "the recorded click must not reach the app");
        assert_eq!(runtime.pending_mouse_up(), Some(MouseButton::Left));

        let (_, second, suppress) = runtime
            .record_mouse(
                1_100,
                Some(MouseButton::Right),
                true,
                None,
                InputMeta::physical(),
            )
            .unwrap();
        assert_eq!(second.decision, CaptureDecision::Seen);
        assert!(!suppress, "later clicks must stay available for Stop");

        assert!(runtime.take_mouse_up(MouseButton::Left));
        assert_eq!(runtime.pending_mouse_up(), None);
        assert!(!runtime.take_mouse_up(MouseButton::Left));
    }

    #[test]
    fn wheel_is_recorded_but_never_selected() {
        let mut runtime = runtime();
        runtime.start(CaptureKind::Mouse, 0);
        let (_, event, suppress) = runtime
            .record_mouse(
                10,
                None,
                false,
                Some(ScrollDirection::Left),
                InputMeta::physical(),
            )
            .unwrap();
        assert_eq!(event.kind, "wheel");
        assert_eq!(event.decision, CaptureDecision::Seen);
        assert!(!suppress);
    }

    #[test]
    fn keyboard_selects_the_first_key_with_modifiers_and_ignores_repeat() {
        let mut runtime = runtime();
        runtime.start(CaptureKind::Keyboard, 0);

        let (_, ctrl) = runtime
            .record_key(10, 0x11, true, InputMeta::physical(), vec!["ctrl".into()])
            .unwrap();
        assert_eq!(ctrl.decision, CaptureDecision::Seen, "modifier alone is not a source");

        let modifiers = vec!["ctrl".to_string()];
        let (_, first) = runtime
            .record_key(20, 0x7B, true, InputMeta::physical(), modifiers.clone())
            .unwrap();
        assert_eq!(first.token.as_deref(), Some("f12"));
        assert_eq!(first.decision, CaptureDecision::Selected);
        assert_eq!(first.modifiers, modifiers);

        let (_, repeat) = runtime
            .record_key(30, 0x7B, true, InputMeta::physical(), modifiers.clone())
            .unwrap();
        assert!(repeat.repeat);
        assert_eq!(repeat.decision, CaptureDecision::Seen, "auto-repeat must not reselect");

        let (_, release) = runtime
            .record_key(40, 0x7B, false, InputMeta::physical(), modifiers)
            .unwrap();
        assert_eq!(release.kind, "up");
    }

    #[test]
    fn injected_and_own_events_are_classified_not_selected() {
        let mut runtime = runtime();
        runtime.start(CaptureKind::Keyboard, 0);
        let filtered = InputMeta {
            injected: true,
            own: false,
            accepted: false,
        };
        let (_, event) = runtime
            .record_key(10, 0x7B, true, filtered, Vec::new())
            .unwrap();
        assert_eq!(event.decision, CaptureDecision::Filtered);

        let own = InputMeta {
            injected: true,
            own: true,
            accepted: false,
        };
        let (_, event) = runtime
            .record_key(20, 0x7B, true, own, Vec::new())
            .unwrap();
        assert_eq!(event.decision, CaptureDecision::Own);
    }

    #[test]
    fn stopping_with_a_stale_id_keeps_the_newer_session() {
        let mut runtime = runtime();
        let first = runtime.start(CaptureKind::Mouse, 0);
        let second = runtime.start(CaptureKind::Mouse, 10);
        assert_ne!(first.id, second.id);
        assert!(!runtime.stop(first.id));
        assert_eq!(runtime.session_id(), Some(second.id));
        assert!(runtime.stop(second.id));
        assert_eq!(runtime.session_id(), None);
    }

    #[test]
    fn sessions_expire_and_the_buffer_is_bounded() {
        let mut runtime = runtime();
        runtime.start(CaptureKind::Keyboard, 1_000);
        for index in 0..(MAX_CAPTURE_EVENTS + 10) {
            runtime.record_key(
                1_000 + index as u64,
                0x41,
                true,
                InputMeta::physical(),
                Vec::new(),
            );
        }
        let id = runtime.session_id().unwrap();
        let events = runtime.events_since(id, 0).unwrap();
        assert_eq!(events.len(), MAX_CAPTURE_EVENTS);
        assert!(events.windows(2).all(|pair| pair[0].seq < pair[1].seq));

        assert!(runtime.expire(1_000 + CAPTURE_TIMEOUT_MS - 1).is_none());
        let (expired, kind) = runtime.expire(1_000 + CAPTURE_TIMEOUT_MS).unwrap();
        assert_eq!(expired, id);
        assert_eq!(kind, CaptureKind::Keyboard);
        assert_eq!(runtime.session_id(), None);
    }

    #[test]
    fn events_since_returns_only_the_missing_suffix() {
        let mut runtime = runtime();
        runtime.start(CaptureKind::Keyboard, 0);
        for _ in 0..3 {
            runtime.record_key(10, 0x41, true, InputMeta::physical(), Vec::new());
        }
        let id = runtime.session_id().unwrap();
        let tail = runtime.events_since(id, 2).unwrap();
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].seq, 3);
        assert!(runtime.events_since(id + 1, 0).is_none());
    }
}
