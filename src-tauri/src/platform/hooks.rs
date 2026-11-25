//! Windows low-level mouse and keyboard hooks.
//!
//! Hooks are installed on a dedicated thread with its own message loop, which
//! is required for `WH_MOUSE_LL` / `WH_KEYBOARD_LL`. Every injected event carries
//! [`INJECTION_TAG`] and is ignored, preventing feedback loops.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, OnceLock};
use std::thread;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    PeekMessageW, TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG,
    MSLLHOOKSTRUCT, PM_NOREMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_MOUSEMOVE, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

use crate::config::model::{MouseButton, ScrollDirection, INJECTION_TAG};
use crate::platform::service::{EngineService, MouseInput};

static SERVICE: OnceLock<Arc<EngineService>> = OnceLock::new();
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static KEYBOARD_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static KEYBOARD_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);
static MOUSE_CALLBACKS: AtomicU64 = AtomicU64::new(0);
static KEY_CALLBACKS: AtomicU64 = AtomicU64::new(0);
static MOUSE_HOOK_HANDLE: AtomicI64 = AtomicI64::new(0);
static KEY_HOOK_HANDLE: AtomicI64 = AtomicI64::new(0);

const LLMHF_INJECTED: u32 = 0x0000_0001;
const LLMHF_LOWER_IL_INJECTED: u32 = 0x0000_0002;
const LLKHF_INJECTED: u32 = 0x0000_0010;

/// Debug builds accept other tools' injected input so the full capture and
/// remapping pipeline can be tested end to end. Set
/// `MOUSEMACRO_DEBUG_IGNORE_INJECTED=1` to restore the production behaviour.
/// Release builds always ignore injected events, and events tagged by this app
/// are always ignored.
fn allow_external_injection() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var_os("MOUSEMACRO_DEBUG_IGNORE_INJECTED").is_none()
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}

fn should_ignore_injected(flags_injected: bool, extra_info: usize) -> bool {
    extra_info == INJECTION_TAG || (flags_injected && !allow_external_injection())
}

struct Startup {
    fatal: Option<String>,
    warning: Option<String>,
}

pub fn start(service: Arc<EngineService>) -> Result<(), String> {
    if RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }
    let _ = SERVICE.set(service.clone());
    service.set_hook_warning(None);

    let thread_service = service.clone();
    let (tx, rx) = mpsc::channel::<Startup>();
    thread::Builder::new()
        .name("mouse-hooks".into())
        .spawn(move || unsafe {
            let module: HINSTANCE = GetModuleHandleW(None)
                .map(|module| HINSTANCE(module.0))
                .unwrap_or_default();

            // Force this thread's message queue into existence before the
            // low-level hooks are installed. WH_*_LL callbacks are delivered
            // through that queue.
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);

            let mouse_hook: HHOOK = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    let _ = tx.send(Startup {
                        fatal: Some(format!("Could not install the mouse hook: {error}")),
                        warning: None,
                    });
                    return;
                }
            };

            MOUSE_HOOK_HANDLE.store(mouse_hook.0 as i64, Ordering::SeqCst);
            HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);
            RUNNING.store(true, Ordering::SeqCst);
            thread_service.set_hooks_active(true);
            let _ = tx.send(Startup {
                fatal: None,
                warning: None,
            });

            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(mouse_hook);
            HOOK_THREAD_ID.store(0, Ordering::SeqCst);
            RUNNING.store(false, Ordering::SeqCst);
            thread_service.set_hooks_active(false);
        })
        .map_err(|error| format!("Could not start the input thread: {error}"))?;

    match rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(startup) => {
            if let Some(warning) = startup.warning {
                service.set_hook_warning(Some(warning));
            }
            if let Some(error) = startup.fatal {
                return Err(error);
            }
            Ok(())
        }
        Err(_) => Err("The input hook thread did not start in time.".into()),
    }
}

/// Starts a dedicated keyboard-hook thread on demand. The keyboard hook is only
/// needed while the UI is recording a shortcut, so it stays out of the input
/// path the rest of the time. Keeping it on its own thread also isolates the
/// callback from mouse-hook processing.
pub fn enable_keyboard_hook() -> Result<(), String> {
    // Only trust a live hook handle. A thread that failed to spawn previously
    // could leave the running flag set, which would silently disable capture.
    if KEY_HOOK_HANDLE.load(Ordering::SeqCst) != 0 {
        return Ok(());
    }
    if KEYBOARD_THREAD_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let spawn = thread::Builder::new()
        .name("keyboard-hooks".into())
        .spawn(move || unsafe {
            let module: HINSTANCE = GetModuleHandleW(None)
                .map(|module| HINSTANCE(module.0))
                .unwrap_or_default();
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);

            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    KEYBOARD_THREAD_RUNNING.store(false, Ordering::SeqCst);
                    let _ = tx.send(Err(format!("{error}")));
                    return;
                }
            };

            KEY_HOOK_HANDLE.store(hook.0 as i64, Ordering::SeqCst);
            KEYBOARD_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);
            let _ = tx.send(Ok(()));

            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(hook);
            KEY_HOOK_HANDLE.store(0, Ordering::SeqCst);
            KEYBOARD_THREAD_ID.store(0, Ordering::SeqCst);
            KEYBOARD_THREAD_RUNNING.store(false, Ordering::SeqCst);
        });

    if let Err(error) = spawn {
        KEYBOARD_THREAD_RUNNING.store(false, Ordering::SeqCst);
        return Err(format!(
            "Could not start the keyboard hook thread: {error}"
        ));
    }

    match rx.recv_timeout(std::time::Duration::from_secs(4)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("Could not install the keyboard hook: {error}")),
        Err(_) => Err("The keyboard hook thread did not start in time.".into()),
    }
}

pub fn stop() {
    let thread_id = HOOK_THREAD_ID.load(Ordering::SeqCst);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, 0x0012 /* WM_QUIT */, WPARAM(0), LPARAM(0));
        }
    }
    let keyboard_thread_id = KEYBOARD_THREAD_ID.load(Ordering::SeqCst);
    if keyboard_thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(
                keyboard_thread_id,
                0x0012, /* WM_QUIT */
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

/// Diagnostic callback counters for development builds.
pub fn callback_stats() -> (u64, u64, i64, i64) {
    (
        MOUSE_CALLBACKS.load(Ordering::Relaxed),
        KEY_CALLBACKS.load(Ordering::Relaxed),
        MOUSE_HOOK_HANDLE.load(Ordering::Relaxed),
        KEY_HOOK_HANDLE.load(Ordering::Relaxed),
    )
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    MOUSE_CALLBACKS.fetch_add(1, Ordering::Relaxed);
    let suppress = catch_unwind(AssertUnwindSafe(|| {
        handle_mouse_message(wparam.0 as u32, lparam)
    }))
    .unwrap_or_else(|_| {
        // An engine panic must never swallow input or tear down the process.
        false
    });

    if suppress {
        LRESULT(1)
    } else {
        CallNextHookEx(None, code, wparam, lparam)
    }
}

unsafe fn handle_mouse_message(message: u32, lparam: LPARAM) -> bool {
    if message == WM_MOUSEMOVE {
        return false;
    }
    let Some(service) = SERVICE.get() else {
        return false;
    };
    let Some(info) = (lparam.0 as *const MSLLHOOKSTRUCT).as_ref() else {
        return false;
    };
    let injected = info.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED) != 0;
    if should_ignore_injected(injected, info.dwExtraInfo) {
        return false;
    }

    let input = match message {
        WM_LBUTTONDOWN => MouseInput::Down(MouseButton::Left),
        WM_LBUTTONUP => MouseInput::Up(MouseButton::Left),
        WM_RBUTTONDOWN => MouseInput::Down(MouseButton::Right),
        WM_RBUTTONUP => MouseInput::Up(MouseButton::Right),
        WM_MBUTTONDOWN => MouseInput::Down(MouseButton::Middle),
        WM_MBUTTONUP => MouseInput::Up(MouseButton::Middle),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let xbutton = ((info.mouseData >> 16) & 0xffff) as u16;
            let button = match xbutton {
                1 => MouseButton::X1,
                2 => MouseButton::X2,
                _ => return false,
            };
            if message == WM_XBUTTONDOWN {
                MouseInput::Down(button)
            } else {
                MouseInput::Up(button)
            }
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            let delta = (((info.mouseData >> 16) & 0xffff) as u16) as i16;
            if delta == 0 {
                return false;
            }
            if message == WM_MOUSEWHEEL {
                MouseInput::Wheel(if delta > 0 {
                    ScrollDirection::Up
                } else {
                    ScrollDirection::Down
                })
            } else {
                MouseInput::Wheel(if delta > 0 {
                    ScrollDirection::Right
                } else {
                    ScrollDirection::Left
                })
            }
        }
        _ => return false,
    };

    service.handle_mouse(input)
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    KEY_CALLBACKS.fetch_add(1, Ordering::Relaxed);
    let suppress = catch_unwind(AssertUnwindSafe(|| {
        handle_keyboard_message(wparam.0 as u32, lparam)
    }))
    .unwrap_or(false);

    if suppress {
        LRESULT(1)
    } else {
        CallNextHookEx(None, code, wparam, lparam)
    }
}

unsafe fn handle_keyboard_message(message: u32, lparam: LPARAM) -> bool {
    if !matches!(
        message,
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
    ) {
        return false;
    }
    let Some(service) = SERVICE.get() else {
        return false;
    };
    if service.capture_mode() != crate::platform::service::CaptureMode::Keyboard {
        return false;
    }
    let Some(info) = (lparam.0 as *const KBDLLHOOKSTRUCT).as_ref() else {
        return false;
    };
    let injected = info.flags.0 & LLKHF_INJECTED != 0;
    if should_ignore_injected(injected, info.dwExtraInfo) {
        return false;
    }
    let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    service.handle_key(info.vkCode as u16, is_down)
}
