use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::config::model::{
    Action, AppConfig, ExceptionMode, MacroStep, MouseButton, ScrollDirection, ScrollSettings,
};
use crate::engine::keyboard::KeyboardEngine;
use crate::engine::{keys, Effect, EngineConfig, InputEngine, InputEvent};
use crate::platform::capture::{
    CaptureEvent, CaptureKind, CaptureRuntime, CaptureSessionInfo, InputMeta,
};
use crate::platform::simulate;

pub(crate) fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseInput {
    Down(MouseButton),
    Up(MouseButton),
    /// A physical wheel event. `notches` is the reported delta in 120-unit
    /// notches, at least 1.
    Wheel {
        direction: ScrollDirection,
        notches: i32,
    },
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

#[derive(Clone)]
enum HeldSynthetic {
    Keys(Vec<String>),
    Mouse(MouseButton),
    /// Several holds started together by a combined action. Released in
    /// reverse order so tools that changed on press unwind cleanly.
    Group(Vec<HeldSynthetic>),
}

impl HeldSynthetic {
    fn identity(&self) -> String {
        match self {
            HeldSynthetic::Keys(keys) => combo_identity(keys),
            HeldSynthetic::Mouse(button) => format!("mouse:{}", button.label()),
            HeldSynthetic::Group(items) => items
                .iter()
                .map(HeldSynthetic::identity)
                .collect::<Vec<_>>()
                .join("|"),
        }
    }

    fn release(self) -> Result<(), String> {
        match self {
            HeldSynthetic::Keys(keys) => simulate::release_keys(&keys),
            HeldSynthetic::Mouse(button) => simulate::release_button(button),
            HeldSynthetic::Group(items) => {
                let mut result = Ok(());
                for item in items.into_iter().rev() {
                    let next = item.release();
                    if result.is_ok() {
                        result = next;
                    }
                }
                result
            }
        }
    }
}

/// Order-insensitive identity for a key combination, used to match hold and
/// release steps even in hand-edited configurations.
fn combo_identity(keys: &[String]) -> String {
    let mut sorted = keys.to_vec();
    sorted.sort();
    format!("keys:{}", sorted.join("+"))
}

/// Owns all synthetic down states so they can be released deterministically.
/// The fields are shared with macro threads, which is why every mutable piece
/// sits behind its own `Arc` rather than a single outer lock.
#[derive(Clone, Default)]
pub struct Executor {
    holds: Arc<Mutex<HashMap<String, HeldSynthetic>>>,
    /// Sticky state for toggle-key and drag-lock actions.
    toggles: Arc<Mutex<HashMap<String, HeldSynthetic>>>,
    /// Cancellation flags for running macros, keyed by mapping id.
    macros: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    seq: Arc<AtomicU64>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl Executor {
    fn next_token(&self, kind: &str, key: &str) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("{kind}:{key}:{seq}")
    }

    fn run(&self, key: &str, action: &Action) {
        let result = match action {
            Action::Shortcut { keys } | Action::HoldKeys { keys } => simulate::tap_keys(keys),
            Action::Key { key: k } => simulate::tap_keys(std::slice::from_ref(k)),
            Action::MouseClick { button } | Action::MouseHold { button } => {
                simulate::click_button(*button)
            }
            Action::Scroll { direction, amount } => simulate::scroll(*direction, *amount),
            Action::Macro {
                steps,
                repeat,
                interval_ms,
                toggle,
            } => self.start_macro(key, steps, *repeat, *interval_ms, *toggle),
            Action::Launch { target } => simulate::launch(target),
            Action::TypeText { text } => simulate::type_text(text),
            Action::Media { key: media } => simulate::media_key(media),
            Action::Window { action } => simulate::window_action(*action),
            Action::ToggleKeys { keys } => {
                self.toggle(&combo_identity(keys), HeldSynthetic::Keys(keys.clone()))
            }
            Action::DragLock { button } => {
                self.toggle(&format!("mouse:{}", button.label()), HeldSynthetic::Mouse(*button))
            }
            Action::Combined { actions } => {
                // Give each child its own key so two macros in one group do not
                // cancel each other.
                for (index, child) in actions.iter().enumerate() {
                    self.run(&format!("{key}#{index}"), child);
                }
                Ok(())
            }
            Action::Disable | Action::None => Ok(()),
        };
        self.finish(result);
    }

    /// Presses or releases a sticky toggle. Repeat presses release it again.
    fn toggle(&self, key: &str, state: HeldSynthetic) -> Result<(), String> {
        let held = {
            let mut toggles = self.toggles.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = toggles.remove(key) {
                Some(existing)
            } else {
                toggles.insert(key.to_string(), state.clone());
                None
            }
        };
        match held {
            Some(held) => held.release(),
            None => match state {
                HeldSynthetic::Keys(keys) => simulate::press_keys(&keys),
                HeldSynthetic::Mouse(button) => simulate::press_button(button),
                // Toggle actions never store a group; nothing to press.
                HeldSynthetic::Group(_) => Ok(()),
            },
        }
    }

    fn start_hold(&self, token: &str, action: &Action) {
        self.release(token);
        let result = match action {
            Action::HoldKeys { keys } => simulate::press_keys(keys).map(|_| {
                self.holds
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(token.to_string(), HeldSynthetic::Keys(keys.clone()));
            }),
            Action::MouseHold { button } => simulate::press_button(*button).map(|_| {
                self.holds
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(token.to_string(), HeldSynthetic::Mouse(*button));
            }),
            Action::Combined { actions } => {
                // Non-hold children run now, in order, so a key tap happens
                // before the mouse button goes down (H before the pan drag).
                let key = token.split(':').nth(1).unwrap_or("test");
                let mut held: Vec<HeldSynthetic> = Vec::new();
                let mut result = Ok(());
                for (index, child) in actions.iter().enumerate() {
                    let next = match child {
                        Action::HoldKeys { keys } => simulate::press_keys(keys).map(|_| {
                            held.push(HeldSynthetic::Keys(keys.clone()));
                        }),
                        Action::MouseHold { button } => {
                            simulate::press_button(*button).map(|_| {
                                held.push(HeldSynthetic::Mouse(*button));
                            })
                        }
                        other => {
                            self.run(&format!("{key}#{index}"), other);
                            Ok(())
                        }
                    };
                    if let Err(error) = next {
                        result = Err(error);
                        break;
                    }
                }
                if !held.is_empty() {
                    // Store even after a failure so the pressed keys can be
                    // released by Stop, the panic hotkey or an app switch.
                    self.holds
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(token.to_string(), HeldSynthetic::Group(held));
                }
                result
            }
            other => {
                // Not a sustained action: execute it and do not keep a hold.
                let key = token.split(':').nth(1).unwrap_or("test");
                self.run(key, other);
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
            Some(held) => held.release(),
            None => Ok(()),
        };
        self.finish(result);
    }

    pub fn release_all(&self) {
        self.cancel_macros();
        let held: Vec<HeldSynthetic> = {
            let mut holds = self.holds.lock().unwrap_or_else(|e| e.into_inner());
            holds.drain().map(|(_, held)| held).collect()
        };
        let toggles: Vec<HeldSynthetic> = {
            let mut toggles = self.toggles.lock().unwrap_or_else(|e| e.into_inner());
            toggles.drain().map(|(_, held)| held).collect()
        };
        for held in held.into_iter().chain(toggles) {
            self.finish(held.release());
        }
    }

    pub fn cancel_macros(&self) {
        let macros = {
            let mut macros = self.macros.lock().unwrap_or_else(|e| e.into_inner());
            macros.drain().map(|(_, flag)| flag).collect::<Vec<_>>()
        };
        for flag in macros {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// Stops every macro and immediately releases keys and buttons that the
    /// macros are holding. Used by the editor's Stop button.
    pub fn stop_macros(&self) {
        self.cancel_macros();
        let held: Vec<HeldSynthetic> = {
            let mut holds = self.holds.lock().unwrap_or_else(|e| e.into_inner());
            let tokens: Vec<String> = holds
                .keys()
                .filter(|token| token.starts_with("macro:"))
                .cloned()
                .collect();
            tokens
                .into_iter()
                .filter_map(|token| holds.remove(&token))
                .collect()
        };
        for held in held {
            self.finish(held.release());
        }
    }

    fn start_macro(
        &self,
        key: &str,
        steps: &[MacroStep],
        repeat: u32,
        interval_ms: u64,
        toggle: bool,
    ) -> Result<(), String> {
        if steps.is_empty() {
            return Err("This macro has no steps yet.".into());
        }

        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut macros = self.macros.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = macros.remove(key) {
                // Non-toggle macros replace an earlier run instead of stacking
                // up; a toggle press stops its own macro.
                existing.store(true, Ordering::SeqCst);
                if toggle {
                    return Ok(());
                }
            }
            macros.insert(key.to_string(), cancel.clone());
        }

        let executor = self.clone();
        let thread_key = key.to_string();
        let error_key = key.to_string();
        let steps = steps.to_vec();
        let repeat = repeat.clamp(1, 100);
        let interval_ms = interval_ms.min(10_000);
        let spawn = thread::Builder::new()
            .name("mousemacro-macro".into())
            .spawn(move || {
                executor.execute_macro(&thread_key, &steps, repeat, interval_ms, &cancel);
            });
        if let Err(error) = spawn {
            let mut macros = self.macros.lock().unwrap_or_else(|e| e.into_inner());
            macros.remove(error_key.as_str());
            return Err(format!("Could not start the macro: {error}"));
        }
        Ok(())
    }

    fn execute_macro(
        &self,
        key: &str,
        steps: &[MacroStep],
        repeat: u32,
        interval_ms: u64,
        cancel: &Arc<AtomicBool>,
    ) {
        let base = self.next_token("macro", key);
        // Tokens created by hold steps, paired with their release identity.
        let mut held: Vec<(String, String)> = Vec::new();

        'outer: for round in 0..repeat {
            if round > 0 && !sleep_cancellable(cancel, interval_ms) {
                break;
            }
            for (index, step) in steps.iter().enumerate() {
                if cancel.load(Ordering::SeqCst) {
                    break 'outer;
                }
                let token = format!("{base}:{round}:{index}");
                if let Err(error) = self.execute_step(step, &token, &mut held) {
                    self.finish(Err(error));
                    break 'outer;
                }
                if !sleep_cancellable(cancel, step.delay_ms()) {
                    break 'outer;
                }
            }
        }

        for (token, _) in held.drain(..) {
            self.release(&token);
        }

        // Clear the registry entry only if this run is still the registered one.
        let mut macros = self.macros.lock().unwrap_or_else(|e| e.into_inner());
        if macros
            .get(key)
            .is_some_and(|flag| Arc::ptr_eq(flag, cancel))
        {
            macros.remove(key);
        }
    }

    fn execute_step(
        &self,
        step: &MacroStep,
        token: &str,
        held: &mut Vec<(String, String)>,
    ) -> Result<(), String> {
        match step {
            MacroStep::TapKeys { keys, .. } => simulate::tap_keys(keys),
            MacroStep::HoldKeys { keys, .. } => {
                self.start_hold(token, &Action::HoldKeys { keys: keys.clone() });
                held.push((token.to_string(), HeldSynthetic::Keys(keys.clone()).identity()));
                Ok(())
            }
            MacroStep::ReleaseKeys { keys, .. } => {
                self.release_identity(held, &HeldSynthetic::Keys(keys.clone()).identity());
                Ok(())
            }
            MacroStep::Wait { .. } => Ok(()),
            MacroStep::MouseClick { button, .. } => simulate::click_button(*button),
            MacroStep::MouseHold { button, .. } => {
                self.start_hold(token, &Action::MouseHold { button: *button });
                held.push((token.to_string(), HeldSynthetic::Mouse(*button).identity()));
                Ok(())
            }
            MacroStep::MouseRelease { button, .. } => {
                self.release_identity(held, &HeldSynthetic::Mouse(*button).identity());
                Ok(())
            }
            MacroStep::Scroll {
                direction, amount, ..
            } => simulate::scroll(*direction, *amount),
            MacroStep::TypeText { text, .. } => simulate::type_text(text),
        }
    }

    fn release_identity(&self, held: &mut Vec<(String, String)>, identity: &str) {
        let mut index = 0;
        while index < held.len() {
            if held[index].1 == identity {
                let (token, _) = held.remove(index);
                self.release(&token);
            } else {
                index += 1;
            }
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

/// Sleeps in short slices so a cancel (panic, toggle press, app switch) is not
/// delayed by a long macro wait.
fn sleep_cancellable(cancel: &AtomicBool, ms: u64) -> bool {
    let mut remaining = ms;
    while remaining > 0 {
        if cancel.load(Ordering::SeqCst) {
            return false;
        }
        let chunk = remaining.min(25);
        thread::sleep(Duration::from_millis(chunk));
        remaining -= chunk;
    }
    !cancel.load(Ordering::SeqCst)
}

struct Inner {
    engine: InputEngine,
    keyboard: KeyboardEngine,
}

/// Per-application state that is not persisted. `panic` is the emergency stop;
/// the exception fields are resolved from the current foreground executable.
#[derive(Debug, Clone, Default)]
struct RuntimeState {
    foreground: Option<String>,
    exception_mode: ExceptionMode,
    profile_override: Option<String>,
    panic: bool,
    /// Keyboard remapping is off in this application, either explicitly or
    /// because the rule disables all mappings.
    keyboard_disabled: bool,
    /// Wheel transform in effect. `None` passes physical wheels through.
    scroll: Option<ScrollSettings>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingActivity {
    pub mapping_id: String,
    pub age_ms: u64,
}

pub struct EngineService {
    inner: Mutex<Inner>,
    cv: Condvar,
    executor: Executor,
    app: AppHandle,
    config: Mutex<AppConfig>,
    runtime: Mutex<RuntimeState>,
    activity: Mutex<HashMap<String, u64>>,
    capture: AtomicU8,
    capture_runtime: Mutex<CaptureRuntime>,
    accept_external_injection: AtomicBool,
    hooks_active: AtomicBool,
    hook_warning: Mutex<Option<String>>,
    last_wheel_ms: AtomicU64,
}

impl EngineService {
    pub fn new(app: AppHandle, config: &AppConfig) -> Arc<Self> {
        let mut runtime = RuntimeState::default();
        Self::resolve_runtime(config, &mut runtime);
        let effective = Self::compute_effective(config, &runtime);
        let service = Arc::new(Self {
            inner: Mutex::new(Inner {
                engine: InputEngine::new(effective.clone()),
                keyboard: KeyboardEngine::new(effective.keyboard, &effective.key_mappings),
            }),
            cv: Condvar::new(),
            executor: Executor::default(),
            app,
            config: Mutex::new(config.clone()),
            runtime: Mutex::new(runtime),
            activity: Mutex::new(HashMap::new()),
            capture: AtomicU8::new(CaptureMode::None as u8),
            capture_runtime: Mutex::new(CaptureRuntime::default()),
            accept_external_injection: AtomicBool::new(
                config.settings.process_injected_input,
            ),
            hooks_active: AtomicBool::new(false),
            hook_warning: Mutex::new(None),
            last_wheel_ms: AtomicU64::new(0),
        });
        service.sync_keyboard_hook();
        Self::spawn_timer(service.clone());
        service
    }

    fn spawn_timer(service: Arc<Self>) {
        let _ = thread::Builder::new()
            .name("mouse-engine-timer".into())
            .spawn(move || loop {
                let now = now_ms();
                service.expire_capture(now);
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
                            let _guard = service
                                .cv
                                .wait_timeout(inner, Duration::from_millis(500))
                                .unwrap_or_else(|e| e.into_inner());
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
        let effective = {
            let mut stored = self.config.lock().unwrap_or_else(|e| e.into_inner());
            *stored = config.clone();
            let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            Self::resolve_runtime(config, &mut runtime);
            Self::compute_effective(config, &runtime)
        };
        self.apply_effective(effective, false);
    }

    /// Recomputes the engine's effective configuration from the active
    /// exceptions and the emergency stop.
    fn compute_effective(config: &AppConfig, runtime: &RuntimeState) -> EngineConfig {
        let mut effective = EngineConfig::from_app(config);
        if runtime.panic || !effective.enabled {
            effective.enabled = false;
            effective.keyboard = false;
            return effective;
        }
        if runtime.keyboard_disabled {
            effective.keyboard = false;
        }

        match runtime.exception_mode {
            ExceptionMode::Disable => {
                effective.enabled = false;
                effective.keyboard = false;
            }
            ExceptionMode::Profile => {
                if let Some(id) = runtime.profile_override.as_deref() {
                    if let Some(profile) = config.profiles.iter().find(|p| p.id == id) {
                        effective.mappings = profile.mappings.clone();
                        effective.layers = profile.layers.clone();
                        effective.key_mappings = profile.key_mappings.clone();
                    }
                }
            }
            ExceptionMode::Always => {}
        }
        effective
    }

    fn resolve_runtime(config: &AppConfig, runtime: &mut RuntimeState) {
        runtime.exception_mode = ExceptionMode::Always;
        runtime.profile_override = None;
        runtime.keyboard_disabled = false;
        runtime.scroll = Some(config.settings.scroll.clone());

        let Some(process) = runtime.foreground.as_deref() else {
            return;
        };
        if let Some(rule) = config.exceptions.iter().find(|rule| rule.matches(process)) {
            runtime.exception_mode = rule.mode;
            if rule.mode == ExceptionMode::Profile {
                runtime.profile_override = rule.profile_id.clone();
            }
            runtime.keyboard_disabled =
                rule.keyboard_disabled || rule.mode == ExceptionMode::Disable;
            // An explicit wheel override always applies. With mappings off and
            // no override, the wheel is left completely alone.
            runtime.scroll = rule.scroll.clone().or_else(|| {
                if rule.mode == ExceptionMode::Disable {
                    None
                } else {
                    Some(config.settings.scroll.clone())
                }
            });
        }
    }

    /// The wheel transform for the current app, or `None` when physical wheel
    /// events should pass through untouched.
    fn effective_scroll(&self) -> Option<ScrollSettings> {
        let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        if !config.settings.enabled {
            return None;
        }
        let runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
        if runtime.panic {
            return None;
        }
        runtime.scroll.clone()
    }

    fn apply_effective(&self, effective: EngineConfig, notify: bool) {
        let keyboard = effective.keyboard;
        let key_mappings = effective.key_mappings.clone();
        let effects = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let mut effects = inner.engine.reconfigure(effective);
            effects.extend(inner.keyboard.reconfigure(keyboard, &key_mappings));
            effects
        };
        self.cv.notify_all();
        self.execute(effects);
        self.sync_keyboard_hook();
        if notify {
            let _ = self.app.emit("state:changed", ());
        }
    }

    /// Installs the keyboard hook while remapping is opted in, and removes it
    /// when it is turned off. Capture mode keeps the hook on its own.
    fn sync_keyboard_hook(&self) {
        let install = {
            let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
            config.settings.keyboard.enabled && config.settings.keyboard.acknowledged
        };
        if install {
            if let Err(error) = crate::platform::hooks::enable_keyboard_hook() {
                self.set_hook_warning(Some(format!(
                    "Keyboard remapping is unavailable: {error}"
                )));
            }
        } else if self.capture_mode() != CaptureMode::Keyboard {
            crate::platform::hooks::disable_keyboard_hook();
        }
    }

    /// Called by the foreground watcher whenever the user switches apps.
    pub fn set_foreground(&self, process: Option<String>) {
        let normalized = process
            .as_deref()
            .and_then(crate::config::model::normalize_process_name);

        let effective = {
            let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
            let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            if runtime.foreground == normalized {
                return;
            }
            runtime.foreground = normalized;
            Self::resolve_runtime(&config, &mut runtime);
            Self::compute_effective(&config, &runtime)
        };
        self.apply_effective(effective, true);
    }

    pub fn foreground_process(&self) -> Option<String> {
        self.runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .foreground
            .clone()
    }

    pub fn exception_mode(&self) -> ExceptionMode {
        self.runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .exception_mode
    }

    /// The profile the engine is actually using, including app overrides.
    pub fn effective_profile_id(&self) -> String {
        let config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
        runtime
            .profile_override
            .clone()
            .unwrap_or(config.active_profile_id)
    }

    /// Emergency stop: disable mappings and release every synthetic key and
    /// button immediately. The persisted "enabled" setting is untouched, so
    /// re-enabling from the tray resumes normal behaviour.
    pub fn panic(&self) {
        let effective = {
            let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
            let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            runtime.panic = true;
            Self::compute_effective(&config, &runtime)
        };
        self.apply_effective(effective, true);
        self.executor.release_all();
    }

    pub fn clear_panic(&self) {
        let effective = {
            let config = self.config.lock().unwrap_or_else(|e| e.into_inner());
            let mut runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
            if !runtime.panic {
                return;
            }
            runtime.panic = false;
            Self::compute_effective(&config, &runtime)
        };
        self.apply_effective(effective, true);
    }

    pub fn panic_active(&self) -> bool {
        self.runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .panic
    }

    /// Runs an action once for the editor's Test button. Sticky actions are
    /// toggled on and then immediately off so a preview never leaves a key or
    /// mouse button held down.
    pub fn test_action(&self, action: &Action) {
        match action {
            Action::ToggleKeys { .. } | Action::DragLock { .. } => {
                self.executor.run("test", action);
                self.executor.run("test", action);
            }
            _ => self.executor.run("test", action),
        }
    }

    /// Stops every running macro and releases the keys/buttons they hold.
    pub fn stop_macros(&self) {
        self.executor.stop_macros();
    }

    fn record_fired(&self, mapping_id: &str) {
        let mut activity = self.activity.lock().unwrap_or_else(|e| e.into_inner());
        activity.insert(mapping_id.to_string(), now_ms());
        if activity.len() > 500 {
            let oldest = activity
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(id, _)| id.clone());
            if let Some(id) = oldest {
                activity.remove(&id);
            }
        }
    }

    pub fn activity_snapshot(&self) -> Vec<MappingActivity> {
        let now = now_ms();
        let activity = self.activity.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<MappingActivity> = activity
            .iter()
            .map(|(mapping_id, at)| MappingActivity {
                mapping_id: mapping_id.clone(),
                age_ms: now.saturating_sub(*at),
            })
            .collect();
        entries.sort_by_key(|entry| entry.age_ms);
        entries
    }

    pub fn clear_activity(&self) {
        self.activity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
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

    /// Kept for Input diagnostics, which only needs the passive monitor mode.
    /// Recording uses the capture session API instead.
    pub fn set_capture_mode(&self, mode: CaptureMode) -> Result<(), String> {
        match mode {
            CaptureMode::Monitor => {
                self.stop_session(None, "replaced");
                self.capture
                    .store(CaptureMode::Monitor as u8, Ordering::SeqCst);
            }
            CaptureMode::None => {
                self.stop_session(None, "stopped");
                self.capture
                    .store(CaptureMode::None as u8, Ordering::SeqCst);
                self.sync_keyboard_hook();
            }
            CaptureMode::Mouse | CaptureMode::Keyboard => {
                return Err("Use start_capture to record input.".into());
            }
        }
        self.cv.notify_all();
        Ok(())
    }

    /// Starts a recording session. Any previous session is stopped first.
    pub fn start_capture(&self, kind: CaptureKind) -> Result<CaptureSessionInfo, String> {
        self.stop_session(None, "replaced");
        let info = {
            let mut runtime = self
                .capture_runtime
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            runtime.start(kind, now_ms())
        };
        self.capture.store(
            match kind {
                CaptureKind::Mouse => CaptureMode::Mouse as u8,
                CaptureKind::Keyboard => CaptureMode::Keyboard as u8,
            },
            Ordering::SeqCst,
        );

        if kind == CaptureKind::Keyboard {
            if let Err(error) = crate::platform::hooks::restart_keyboard_hook() {
                let message = format!("Keyboard recording is unavailable: {error}");
                self.set_hook_warning(Some(message.clone()));
                self.stop_session(Some(info.id), "failed");
                self.capture
                    .store(CaptureMode::None as u8, Ordering::SeqCst);
                self.cv.notify_all();
                return Err(message);
            }
        }
        self.cv.notify_all();
        Ok(info)
    }

    pub fn stop_capture(&self, id: u64) {
        let stopped = self.stop_session(Some(id), "stopped");
        if stopped {
            self.capture
                .store(CaptureMode::None as u8, Ordering::SeqCst);
            self.sync_keyboard_hook();
            self.cv.notify_all();
        }
    }

    pub fn capture_events(&self, id: u64, since_seq: u64) -> Vec<CaptureEvent> {
        self.capture_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .events_since(id, since_seq)
            .unwrap_or_default()
    }

    /// The session still accepting input, if any. The UI polls this so a
    /// missed `capture://ended` event cannot leave a dead session on screen.
    pub fn active_capture_session(&self) -> Option<u64> {
        self.capture_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active_id()
    }

    /// Stops a session, emitting `capture://ended` so the UI can leave the
    /// listening state even when the watchdog or a replacement stopped it.
    /// With `only_id`, a stale id is ignored.
    fn stop_session(&self, only_id: Option<u64>, reason: &str) -> bool {
        let stopped = {
            let mut runtime = self
                .capture_runtime
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            match only_id {
                Some(id) => {
                    if runtime.stop(id) {
                        Some(id)
                    } else {
                        None
                    }
                }
                None => runtime.stop_any().map(|(id, _)| id),
            }
        };
        let Some(id) = stopped else {
            return false;
        };
        let _ = self.app.emit(
            "capture://ended",
            json!({ "sessionId": id, "reason": reason }),
        );
        true
    }

    fn expire_capture(&self, now: u64) {
        let expired = {
            let mut runtime = self
                .capture_runtime
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            runtime.expire(now)
        };
        if let Some((id, _)) = expired {
            self.capture
                .store(CaptureMode::None as u8, Ordering::SeqCst);
            let _ = self.app.emit(
                "capture://ended",
                json!({ "sessionId": id, "reason": "timeout" }),
            );
            self.sync_keyboard_hook();
            self.cv.notify_all();
        }
    }

    fn emit_capture_event(&self, id: u64, event: &CaptureEvent) {
        let _ = self.app.emit(
            "capture://event",
            json!({ "sessionId": id, "event": event }),
        );
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
    pub fn handle_mouse(&self, input: MouseInput, meta: InputMeta) -> bool {
        if self.capture_mode() == CaptureMode::Monitor {
            self.emit_mouse_capture(input);
            return false;
        }

        let time = now_ms();
        let mut capture = self
            .capture_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if capture.session_kind() == Some(CaptureKind::Mouse) {
            // Recording accepts any input except MouseMacro's own injections.
            // The "process input from other tools" setting protects the
            // mapping engine from loops; while the user is explicitly
            // recording, input from vendor or remote tools must be recordable.
            let recorded = match input {
                MouseInput::Down(button) => capture.record_mouse(
                    time,
                    Some(button),
                    true,
                    None,
                    InputMeta {
                        accepted: !meta.own,
                        ..meta
                    },
                ),
                MouseInput::Up(button) => capture.record_mouse(
                    time,
                    Some(button),
                    false,
                    None,
                    InputMeta {
                        accepted: !meta.own,
                        ..meta
                    },
                ),
                MouseInput::Wheel { direction, .. } => capture.record_mouse(
                    time,
                    None,
                    false,
                    Some(direction),
                    InputMeta {
                        accepted: !meta.own,
                        ..meta
                    },
                ),
            };
            if let Some((id, event, selected)) = recorded {
                let suppress = selected
                    || matches!(
                        input,
                        MouseInput::Up(button) if capture.take_mouse_up(button)
                    );
                drop(capture);
                self.emit_capture_event(id, &event);
                return suppress;
            }
        }

        // A swallowed down can outlive its session when Stop is pressed before
        // the matching up arrives. Swallow that up so no stray release leaks.
        if let MouseInput::Up(button) = input {
            if capture.take_mouse_up(button) {
                return true;
            }
        }
        drop(capture);

        if !meta.accepted {
            return false;
        }

        let (event, transform) = match input {
            MouseInput::Down(button) => (InputEvent::ButtonDown { button, time }, None),
            MouseInput::Up(button) => (InputEvent::ButtonUp { button, time }, None),
            MouseInput::Wheel { direction, notches } => {
                match self.effective_scroll() {
                    Some(settings) if !settings.is_identity() => {
                        let target = settings.transform(direction);
                        (
                            InputEvent::Wheel {
                                direction: target,
                                notches,
                                time,
                            },
                            Some((settings, target, notches)),
                        )
                    }
                    _ => (
                        InputEvent::Wheel {
                            direction,
                            notches,
                            time,
                        },
                        None,
                    ),
                }
            }
        };

        let outcome = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.engine.handle(event)
        };
        self.cv.notify_all();
        self.execute(outcome.effects);

        if let Some((settings, direction, notches)) = transform {
            // The engine matched a gesture: the physical event is consumed.
            if outcome.suppress {
                return true;
            }
            let multiplier = self.wheel_multiplier(&settings);
            let delta = 120 * notches.max(1) * multiplier;
            if let Err(error) = simulate::scroll_raw(direction, delta) {
                self.executor.finish(Err(error));
            }
            return true;
        }

        outcome.suppress
    }

    /// Extra notches for accelerated scrolling. Time-based because Windows
    /// applications only understand whole wheel notches.
    fn wheel_multiplier(&self, settings: &ScrollSettings) -> i32 {
        let base = settings.speed as i32;
        if !settings.acceleration {
            return base;
        }
        let now = now_ms();
        let last = self.last_wheel_ms.swap(now, Ordering::SeqCst);
        if last == 0 {
            return base;
        }
        let bonus = match now.saturating_sub(last) {
            0..=59 => 2,
            60..=139 => 1,
            _ => 0,
        };
        (base + bonus).min(5)
    }

    /// Entry point for the low-level keyboard hook.
    pub fn handle_key(&self, vk: u16, is_down: bool, meta: InputMeta) -> bool {
        let time = now_ms();
        let mut capture = self
            .capture_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if capture.session_kind() == Some(CaptureKind::Keyboard) {
            // Escape ends the session. Its matching key-up must stay swallowed
            // after the session is already gone.
            if vk == 0x1B && is_down {
                if let Some((id, _)) = capture.stop_any() {
                    capture.arm_key_up_suppression(vk);
                    drop(capture);
                    self.capture
                        .store(CaptureMode::None as u8, Ordering::SeqCst);
                    let _ = self.app.emit(
                        "capture://ended",
                        json!({ "sessionId": id, "reason": "cancelled" }),
                    );
                    self.sync_keyboard_hook();
                    self.cv.notify_all();
                    return true;
                }
            }

            // Recording accepts any input except MouseMacro's own injections:
            // vendor utilities, remote sessions and on-screen keyboards often
            // deliver input as injected events, and they must still be
            // recordable even when the mapping engine ignores injected input.
            let recording = InputMeta {
                accepted: !meta.own,
                ..meta
            };
            let recorded = capture.record_key(time, vk, is_down, recording);
            if let Some((id, event)) = recorded {
                drop(capture);
                self.emit_capture_event(id, &event);
                // Keys recorded from the user never reach the focused app.
                // MouseMacro's own injected output still passes through so
                // remaps and macros are not broken by an open session.
                return recording.accepted;
            }
        }

        if !is_down && capture.take_key_up(vk) {
            return true;
        }
        drop(capture);

        if !meta.accepted {
            return false;
        }
        self.handle_keyboard(vk, is_down)
    }

    /// Matches a physical key against the profile's keyboard mappings. Returns
    /// true when the hook must swallow it.
    fn handle_keyboard(&self, vk: u16, is_down: bool) -> bool {
        // Bare modifiers are never sources, and querying their state would only
        // slow the hot path down.
        if keys::is_modifier_vk(vk) {
            return false;
        }
        let outcome = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if !inner.keyboard.is_active() {
                return false;
            }
            let modifiers = Self::pressed_modifiers();
            inner.keyboard.handle(vk, is_down, &modifiers)
        };
        self.cv.notify_all();
        self.execute(outcome.effects);
        outcome.suppress
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
            MouseInput::Wheel { direction, .. } => json!({
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
                Effect::Run {
                    action,
                    mapping_id,
                } => {
                    self.record_fired(&mapping_id);
                    self.executor.run(&mapping_id, &action)
                }
                Effect::StartHold { token, action } => {
                    // Tokens are `kind:mapping-id:sequence`.
                    if let Some(mapping_id) = token.split(':').nth(1) {
                        self.record_fired(mapping_id);
                    }
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
                Effect::ReplayScroll { direction, notches } => {
                    // Re-use the same speed/acceleration the physical path would
                    // have applied. `wheel_multiplier` is only reached here when
                    // the event was consumed, so acceleration is counted once.
                    let multiplier = self
                        .effective_scroll()
                        .filter(|settings| !settings.is_identity())
                        .map(|settings| self.wheel_multiplier(&settings))
                        .unwrap_or(1);
                    let delta = 120 * notches.max(1) * multiplier;
                    if let Err(error) = simulate::scroll_raw(direction, delta) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        Action, AppConfig, ExceptionMode, ExceptionRule, Layer, Mapping, MouseButton,
        ScrollSettings, Trigger,
    };

    fn config_with_alternative_profile() -> (AppConfig, String) {
        let mut config = AppConfig::default();
        config.settings.enabled = true;
        let mut alternative = crate::config::model::Profile::new("Alternative");
        alternative.mappings = vec![Mapping::new(
            MouseButton::X1,
            Trigger::SingleClick,
            Action::Shortcut {
                keys: vec!["ctrl".into(), "q".into()],
            },
        )];
        let id = alternative.id.clone();
        config.profiles.push(alternative);
        (config, id)
    }

    #[test]
    fn disable_exception_turns_the_engine_off() {
        let config = AppConfig::default();
        let runtime = RuntimeState {
            exception_mode: ExceptionMode::Disable,
            ..RuntimeState::default()
        };
        let effective = EngineService::compute_effective(&config, &runtime);
        assert!(!effective.enabled);
    }

    #[test]
    fn profile_exception_swaps_mappings_and_layers() {
        let (config, profile_id) = config_with_alternative_profile();
        let mut layer = Layer::new("Alt layer", MouseButton::X2);
        layer.mappings.push(Mapping::new(
            MouseButton::Left,
            Trigger::SingleClick,
            Action::Shortcut {
                keys: vec!["ctrl".into(), "w".into()],
            },
        ));
        let mut config = config;
        config
            .profiles
            .iter_mut()
            .find(|profile| profile.id == profile_id)
            .unwrap()
            .layers
            .push(layer);

        let runtime = RuntimeState {
            exception_mode: ExceptionMode::Profile,
            profile_override: Some(profile_id),
            ..RuntimeState::default()
        };
        let effective = EngineService::compute_effective(&config, &runtime);
        assert!(effective.enabled);
        assert_eq!(effective.mappings.len(), 1);
        assert_eq!(effective.layers.len(), 1);
    }

    #[test]
    fn panic_disables_the_engine_even_without_an_exception() {
        let config = AppConfig::default();
        let runtime = RuntimeState {
            panic: true,
            ..RuntimeState::default()
        };
        assert!(!EngineService::compute_effective(&config, &runtime).enabled);
    }

    #[test]
    fn the_global_switch_wins_over_a_profile_exception() {
        let (mut config, profile_id) = config_with_alternative_profile();
        config.settings.enabled = false;
        let runtime = RuntimeState {
            exception_mode: ExceptionMode::Profile,
            profile_override: Some(profile_id),
            ..RuntimeState::default()
        };
        assert!(!EngineService::compute_effective(&config, &runtime).enabled);
    }

    #[test]
    fn the_first_matching_rule_resolves_the_runtime() {
        let mut config = AppConfig::default();
        config.exceptions = vec![
            ExceptionRule::new("chrome.exe", ExceptionMode::Disable),
            ExceptionRule::new("chrome.exe", ExceptionMode::Profile),
        ];
        let mut runtime = RuntimeState {
            foreground: Some("chrome.exe".into()),
            ..RuntimeState::default()
        };
        EngineService::resolve_runtime(&config, &mut runtime);
        assert_eq!(runtime.exception_mode, ExceptionMode::Disable);
    }

    #[test]
    fn leaving_a_rule_clears_the_override() {
        let (config, profile_id) = config_with_alternative_profile();
        let mut runtime = RuntimeState {
            exception_mode: ExceptionMode::Profile,
            profile_override: Some(profile_id),
            foreground: Some("not-listed.exe".into()),
            ..RuntimeState::default()
        };
        EngineService::resolve_runtime(&config, &mut runtime);
        assert_eq!(runtime.exception_mode, ExceptionMode::Always);
        assert!(runtime.profile_override.is_none());
    }

    #[test]
    fn application_scroll_override_wins_over_global() {
        let mut config = AppConfig::default();
        config.settings.scroll.invert_vertical = true;
        let mut rule = ExceptionRule::new("game.exe", ExceptionMode::Always);
        rule.scroll = Some(ScrollSettings {
            swap_axes: true,
            ..ScrollSettings::default()
        });
        config.exceptions.push(rule);

        let mut runtime = RuntimeState {
            foreground: Some("game.exe".into()),
            ..RuntimeState::default()
        };
        EngineService::resolve_runtime(&config, &mut runtime);
        let scroll = runtime.scroll.expect("override should apply");
        assert!(scroll.swap_axes);
        assert!(!scroll.invert_vertical);
    }

    #[test]
    fn mappings_off_without_a_scroll_override_passes_wheels_through() {
        let mut config = AppConfig::default();
        config.settings.scroll.invert_vertical = true;
        config
            .exceptions
            .push(ExceptionRule::new("game.exe", ExceptionMode::Disable));

        let mut runtime = RuntimeState {
            foreground: Some("game.exe".into()),
            ..RuntimeState::default()
        };
        EngineService::resolve_runtime(&config, &mut runtime);
        assert!(runtime.scroll.is_none());
    }

    #[test]
    fn key_identity_is_order_insensitive() {
        assert_eq!(
            combo_identity(&["shift".into(), "ctrl".into()]),
            combo_identity(&["ctrl".into(), "shift".into()])
        );
        assert_ne!(
            combo_identity(&["ctrl".into()]),
            combo_identity(&["shift".into()])
        );
    }
}
