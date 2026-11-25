use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::config::model::{
    Action, AppConfig, MouseButton, ScrollDirection,
};
use crate::engine::{keys, Effect, EngineConfig, InputEngine, InputEvent};
use crate::platform::simulate;

pub(crate) fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseInput {
    Down(MouseButton),
    Up(MouseButton),
    Wheel(ScrollDirection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    None,
    Mouse,
    Keyboard,
    Monitor,
}

impl CaptureMode {
    pub fn from_str(value: &str) -> Self {
        match value {
            "mouse" => CaptureMode::Mouse,
            "keyboard" => CaptureMode::Keyboard,
            "monitor" => CaptureMode::Monitor,
            _ => CaptureMode::None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            CaptureMode::None => "none",
            CaptureMode::Mouse => "mouse",
            CaptureMode::Keyboard => "keyboard",
            CaptureMode::Monitor => "monitor",
        }
    }
}

enum HeldSynthetic {
    Keys(Vec<String>),
    Mouse(MouseButton),
}

/// Owns all synthetic down states so they can be released deterministically.
#[derive(Default)]
pub struct Executor {
    holds: Mutex<HashMap<String, HeldSynthetic>>,
    last_error: Mutex<Option<String>>,
}

impl Executor {
    fn run(&self, action: &Action) {
        let result = match action {
            Action::Shortcut { keys } | Action::HoldKeys { keys } => simulate::tap_keys(keys),
            Action::Key { key } => simulate::tap_keys(std::slice::from_ref(key)),
            Action::MouseClick { button } | Action::MouseHold { button } => {
                simulate::click_button(*button)
            }
            Action::Scroll { direction, amount } => simulate::scroll(*direction, *amount),
            Action::Disable | Action::None => Ok(()),
        };
        self.finish(result);
    }

    fn start_hold(&self, token: &str, action: &Action) {
        self.release(token);
        let result = match action {
            Action::HoldKeys { keys } => simulate::press_keys(keys).map(|_| {
                let mut holds = self.holds.lock().unwrap_or_else(|e| e.into_inner());
                holds.insert(token.to_string(), HeldSynthetic::Keys(keys.clone()));
            }),
            Action::MouseHold { button } => simulate::press_button(*button).map(|_| {
                let mut holds = self.holds.lock().unwrap_or_else(|e| e.into_inner());
                holds.insert(token.to_string(), HeldSynthetic::Mouse(*button));
            }),
            other => {
                // Not a sustained action: execute it and do not keep a hold.
                self.run(other);
                Ok(())
            }
        };
        self.finish(result);
    }

    fn release(&self, token: &str) {
        let held = {
            let mut holds = self.holds.lock().unwrap_or_else(|e| e.into_inner());
            holds.remove(token)
        };
        let result = match held {
            Some(HeldSynthetic::Keys(keys)) => simulate::release_keys(&keys),
            Some(HeldSynthetic::Mouse(button)) => simulate::release_button(button),
            None => Ok(()),
        };
        self.finish(result);
    }

    pub fn release_all(&self) {
        let held: Vec<(String, HeldSynthetic)> = {
            let mut holds = self.holds.lock().unwrap_or_else(|e| e.into_inner());
            holds.drain().collect()
        };
        for (_, held) in held {
            let result = match held {
                HeldSynthetic::Keys(keys) => simulate::release_keys(&keys),
                HeldSynthetic::Mouse(button) => simulate::release_button(button),
            };
            self.finish(result);
        }
    }

    fn finish(&self, result: Result<(), String>) {
        if let Err(error) = result {
            eprintln!("[MouseMacro] {error}");
            let mut last = self.last_error.lock().unwrap_or_else(|e| e.into_inner());
            *last = Some(error);
        }
    }

    pub fn clear_error(&self) {
        *self.last_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

struct Inner {
    engine: InputEngine,
}

pub struct EngineService {
    inner: Mutex<Inner>,
    cv: Condvar,
    executor: Executor,
    app: AppHandle,
    capture: AtomicU8,
    accept_external_injection: AtomicBool,
    hooks_active: AtomicBool,
    hook_warning: Mutex<Option<String>>,
}

impl EngineService {
    pub fn new(app: AppHandle, config: &AppConfig) -> Arc<Self> {
        let service = Arc::new(Self {
            inner: Mutex::new(Inner {
                engine: InputEngine::new(EngineConfig::from_app(config)),
            }),
            cv: Condvar::new(),
            executor: Executor::default(),
            app,
            capture: AtomicU8::new(CaptureMode::None as u8),
            accept_external_injection: AtomicBool::new(
                config.settings.process_injected_input,
            ),
            hooks_active: AtomicBool::new(false),
            hook_warning: Mutex::new(None),
        });
        Self::spawn_timer(service.clone());
        service
    }

    fn spawn_timer(service: Arc<Self>) {
        let _ = thread::Builder::new()
            .name("mouse-engine-timer".into())
            .spawn(move || loop {
                let now = now_ms();
                let effects = {
                    let mut inner = service.inner.lock().unwrap_or_else(|e| e.into_inner());
                    match inner.engine.next_deadline() {
                        Some(deadline) if deadline <= now => {
                            let outcome = inner.engine.handle(InputEvent::Tick { time: now });
                            outcome.effects
                        }
                        Some(deadline) => {
                            let wait = Duration::from_millis(
                                deadline.saturating_sub(now).clamp(1, 500),
                            );
                            let _guard = service
                                .cv
                                .wait_timeout(inner, wait)
                                .unwrap_or_else(|e| e.into_inner());
                            continue;
                        }
                        None => {
                            let _guard =
                                service.cv.wait(inner).unwrap_or_else(|e| e.into_inner());
                            continue;
                        }
                    }
                };
                service.cv.notify_all();
                service.execute(effects);
            });
    }

    pub fn reconfigure(&self, config: &AppConfig) {
        self.accept_external_injection.store(
            config.settings.process_injected_input,
            Ordering::SeqCst,
        );
        let effects = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.engine.reconfigure(EngineConfig::from_app(config))
        };
        self.cv.notify_all();
        self.execute(effects);
    }

    /// Whether input synthesized by *other* tools (mouse utilities, touchpad
    /// drivers, remote desktop) should be processed. Our own injections are
    /// always ignored through [`crate::config::model::INJECTION_TAG`].
    pub fn accepts_external_injection(&self) -> bool {
        self.accept_external_injection.load(Ordering::SeqCst)
    }

    pub fn release_all(&self) {
        let effects = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.engine.cancel_all()
        };
        self.cv.notify_all();
        self.execute(effects);
        self.executor.release_all();
    }

    pub fn set_capture_mode(&self, mode: CaptureMode) -> Result<(), String> {
        let previous = self.capture.swap(mode as u8, Ordering::SeqCst);
        if previous != mode as u8 && mode != CaptureMode::None {
            // Do not leave a half-finished gesture behind while the user is
            // pressing buttons in the capture dialog.
            self.release_all();
        }
        if mode == CaptureMode::Keyboard {
            if let Err(error) = crate::platform::hooks::enable_keyboard_hook() {
                let message = format!("Shortcut capture is unavailable: {error}");
                self.set_hook_warning(Some(message.clone()));
                // Drop out of capture mode so the UI does not sit in a dead
                // "listening" state, and let the caller explain why.
                self.capture
                    .store(CaptureMode::None as u8, Ordering::SeqCst);
                self.cv.notify_all();
                return Err(message);
            }
        }
        self.cv.notify_all();
        Ok(())
    }

    pub fn capture_mode(&self) -> CaptureMode {
        match self.capture.load(Ordering::SeqCst) {
            1 => CaptureMode::Mouse,
            2 => CaptureMode::Keyboard,
            3 => CaptureMode::Monitor,
            _ => CaptureMode::None,
        }
    }

    /// Entry point for the low-level mouse hook. Returns true when the physical
    /// event must be swallowed.
    pub fn handle_mouse(&self, input: MouseInput) -> bool {
        match self.capture_mode() {
            CaptureMode::Monitor => {
                self.emit_mouse_capture(input);
                return false;
            }
            CaptureMode::Mouse => {
                self.emit_mouse_capture(input);
                if matches!(input, MouseInput::Down(_)) {
                    self.capture
                        .store(CaptureMode::None as u8, Ordering::SeqCst);
                    let _ = self.app.emit("input://capture-ended", ());
                }
                return false;
            }
            _ => {}
        }

        let time = now_ms();
        let event = match input {
            MouseInput::Down(button) => InputEvent::ButtonDown { button, time },
            MouseInput::Up(button) => InputEvent::ButtonUp { button, time },
            MouseInput::Wheel(direction) => InputEvent::Wheel { direction, time },
        };

        let outcome = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.engine.handle(event)
        };
        self.cv.notify_all();
        self.execute(outcome.effects);
        outcome.suppress
    }

    /// Entry point for the low-level keyboard hook. Keyboard hooks are only
    /// installed while a shortcut is being captured.
    pub fn handle_key(&self, vk: u16, is_down: bool) -> bool {
        if self.capture_mode() != CaptureMode::Keyboard {
            return false;
        }
        if !is_down {
            return true;
        }

        if vk == 0x1B {
            self.capture
                .store(CaptureMode::None as u8, Ordering::SeqCst);
            let _ = self.app.emit("input://capture", json!({ "kind": "cancel" }));
            return true;
        }

        if keys::is_modifier_vk(vk) {
            let held = Self::pressed_modifiers();
            let _ = self.app.emit(
                "input://capture",
                json!({ "kind": "modifiers", "keys": held }),
            );
            return true;
        }

        let mut combo = Self::pressed_modifiers();
        combo.push(keys::name_from_vk(vk));
        let combo = keys::normalize_combo(&combo);
        self.capture
            .store(CaptureMode::None as u8, Ordering::SeqCst);
        let _ = self.app.emit(
            "input://capture",
            json!({ "kind": "keys", "keys": combo }),
        );
        true
    }

    fn pressed_modifiers() -> Vec<String> {
        let mut held = Vec::new();
        for (vk, name) in [
            (0x11u16, "ctrl"),
            (0x10, "shift"),
            (0x12, "alt"),
            (0x5B, "win"),
            (0x5C, "win"),
        ] {
            let pressed = unsafe {
                windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk as i32) as u16
                    & 0x8000
                    != 0
            };
            if pressed && !held.iter().any(|k| k == name) {
                held.push(name.to_string());
            }
        }
        held
    }

    fn emit_mouse_capture(&self, input: MouseInput) {
        let payload = match input {
            MouseInput::Down(button) => json!({
                "kind": "mouse",
                "event": "down",
                "button": button,
                "label": button.label(),
            }),
            MouseInput::Up(button) => json!({
                "kind": "mouse",
                "event": "up",
                "button": button,
                "label": button.label(),
            }),
            MouseInput::Wheel(direction) => json!({
                "kind": "mouse",
                "event": "wheel",
                "direction": direction,
                "label": direction.label(),
            }),
        };
        let _ = self.app.emit("input://capture", payload);
    }

    fn execute(&self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Run(action) => self.executor.run(&action),
                Effect::StartHold { token, action } => {
                    self.executor.start_hold(&token, &action)
                }
                Effect::ReleaseHold { token } => self.executor.release(&token),
                Effect::Replay { button, direction } => {
                    let down = matches!(
                        direction,
                        crate::engine::ReplayDirection::Down
                    );
                    if let Err(error) = simulate::replay_button(button, down) {
                        self.executor.finish(Err(error));
                    }
                }
            }
        }
    }

    pub fn set_hooks_active(&self, active: bool) {
        self.hooks_active.store(active, Ordering::SeqCst);
    }

    pub fn hooks_active(&self) -> bool {
        self.hooks_active.load(Ordering::SeqCst)
    }

    pub fn set_hook_warning(&self, warning: Option<String>) {
        *self.hook_warning.lock().unwrap_or_else(|e| e.into_inner()) = warning;
    }

    pub fn hook_warning(&self) -> Option<String> {
        self.hook_warning
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn last_error(&self) -> Option<String> {
        self.executor.last_error()
    }

    pub fn clear_errors(&self) {
        self.executor.clear_error();
        self.set_hook_warning(None);
    }
}
