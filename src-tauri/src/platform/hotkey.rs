//! Global emergency-stop hotkey.
//!
//! `RegisterHotKey` is used instead of a keyboard hook: it does not sit in the
//! input path, cannot be blocked by games, and needs no extra thread-level
//! interception. The registration lives on its own message-loop thread so the
//! combo can be swapped without restarting the app.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL,
    MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW, TranslateMessage,
    MSG, PM_NOREMOVE, WM_APP, WM_HOTKEY,
};

use crate::engine::keys::vk_from_name;
use crate::state::AppState;

const HOTKEY_ID: i32 = 0x4D44;
const WM_RELOAD: u32 = WM_APP + 1;

static THREAD_ID: AtomicU32 = AtomicU32::new(0);
static DESIRED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static WARNING: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();

fn desired() -> &'static Mutex<Vec<String>> {
    DESIRED.get_or_init(|| Mutex::new(Vec::new()))
}

fn warning_slot() -> &'static Mutex<Option<String>> {
    WARNING.get_or_init(|| Mutex::new(None))
}

pub fn warning() -> Option<String> {
    warning_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn set_warning(message: Option<String>) {
    *warning_slot().lock().unwrap_or_else(|e| e.into_inner()) = message;
}

/// Registers the initial combo and starts the listener thread. Safe to call
/// once during setup.
pub fn start(app: AppHandle, combo: Vec<String>) {
    let _ = APP.set(app);
    *desired().lock().unwrap_or_else(|e| e.into_inner()) = combo;

    let spawn = thread::Builder::new()
        .name("panic-hotkey".into())
        .spawn(move || unsafe {
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            THREAD_ID.store(
                windows::Win32::System::Threading::GetCurrentThreadId(),
                Ordering::SeqCst,
            );
            register_current();

            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                match msg.message {
                    WM_HOTKEY if msg.wParam.0 as i32 == HOTKEY_ID => trigger_panic(),
                    WM_RELOAD => register_current(),
                    _ => {}
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnregisterHotKey(None, HOTKEY_ID);
            THREAD_ID.store(0, Ordering::SeqCst);
        });
    if let Err(error) = spawn {
        set_warning(Some(format!(
            "The emergency hotkey thread could not start: {error}"
        )));
    }
}

/// Applies a changed combo. Must be called on every settings save.
pub fn update(combo: Vec<String>) {
    *desired().lock().unwrap_or_else(|e| e.into_inner()) = combo;
    let thread_id = THREAD_ID.load(Ordering::SeqCst);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(
                thread_id,
                WM_RELOAD,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            );
        }
    }
}

unsafe fn register_current() {
    let _ = UnregisterHotKey(None, HOTKEY_ID);
    let combo = desired()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if combo.is_empty() {
        set_warning(None);
        notify_state();
        return;
    }

    match parse_combo(&combo) {
        Ok((modifiers, vk)) => {
            match RegisterHotKey(None, HOTKEY_ID, modifiers | MOD_NOREPEAT, vk as u32) {
                Ok(()) => set_warning(None),
                Err(error) => set_warning(Some(format!(
                    "The emergency hotkey could not be registered (it may be used by another program): {error}"
                ))),
            }
        }
        Err(message) => set_warning(Some(message)),
    }
    notify_state();
}

fn notify_state() {
    if let Some(app) = APP.get() {
        let _ = app.emit("state:changed", ());
    }
}

fn trigger_panic() {
    let Some(app) = APP.get() else {
        return;
    };
    if let Some(state) = app.try_state::<AppState>() {
        state.engine.panic();
    }
    let _ = app.emit(
        "app://panic",
        "Emergency stop: mappings disabled and all held keys released.",
    );
}

/// Parses `["ctrl", "alt", "shift", "f12"]` into Windows modifiers and a
/// virtual-key code. A modifier is required so a bare key is never hijacked.
pub fn parse_combo(combo: &[String]) -> Result<(HOT_KEY_MODIFIERS, u16), String> {
    let mut modifiers = HOT_KEY_MODIFIERS(0);
    let mut key = None;

    for raw in combo {
        let name = raw.trim().to_lowercase();
        match name.as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "shift" => modifiers |= MOD_SHIFT,
            "alt" => modifiers |= MOD_ALT,
            "win" | "windows" | "meta" => modifiers |= MOD_WIN,
            "" => {}
            _ => {
                let vk = vk_from_name(&name)
                    .ok_or_else(|| format!("Unknown key “{raw}” in the emergency hotkey."))?;
                if key.replace(vk).is_some() {
                    return Err("The emergency hotkey can contain only one regular key.".into());
                }
            }
        }
    }

    if modifiers.0 == 0 {
        return Err("The emergency hotkey needs at least one of Ctrl, Alt, Shift or Win.".into());
    }
    let key = key.ok_or_else(|| "The emergency hotkey needs a regular key, such as F12.".to_string())?;
    Ok((modifiers, key))
}

#[cfg(test)]
mod tests {
    use super::parse_combo;

    fn combo(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|key| key.to_string()).collect()
    }

    #[test]
    fn default_combo_parses() {
        let (modifiers, key) = parse_combo(&combo(&["ctrl", "alt", "shift", "f12"])).unwrap();
        assert_ne!(modifiers.0, 0);
        assert_eq!(key, 0x7B);
    }

    #[test]
    fn a_bare_key_is_rejected() {
        assert!(parse_combo(&combo(&["f12"])).is_err());
    }

    #[test]
    fn two_regular_keys_are_rejected() {
        assert!(parse_combo(&combo(&["ctrl", "f12", "f11"])).is_err());
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        assert!(parse_combo(&combo(&["ctrl", "not-a-key"])).is_err());
    }
}
