use std::mem::size_of;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEINPUT, MOUSE_EVENT_FLAGS,
    MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, PostMessageW, ShowWindow, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
    SW_SHOWNORMAL,
};

const XBUTTON1: u32 = 1;
const XBUTTON2: u32 = 2;

use crate::config::model::{INJECTION_TAG, MouseButton, ScrollDirection, WindowAction};
use crate::engine::keys::{ordered_for_press, vk_from_name};

/// Sends native input. Every event is tagged so the low-level hook can ignore
/// it; this is what prevents remapped actions from recursively triggering more
/// mappings.
pub fn send(inputs: &[INPUT]) -> Result<(), String> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(format!(
            "Windows rejected {}/{} simulated input events. The target app may be running elevated; restart MouseMacro as administrator.",
            inputs.len().saturating_sub(sent as usize),
            inputs.len()
        ));
    }
    Ok(())
}

fn is_extended_key(vk: u16) -> bool {
    matches!(
        vk,
        0x21..=0x28 // page up/down, end, home, arrows
            | 0x2D | 0x2E // insert, delete
            | 0x5B | 0x5C // windows keys
            | 0x5D // menu
            | 0x6F // numpad divide
            | 0x90 // num lock
            | 0xA3 // right control
            | 0xA5 // right alt
            | 0xAD..=0xB7 // media keys
    )
}

fn keyboard_input(vk: u16, up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTION_TAG,
            },
        },
    }
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS, data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTION_TAG,
            },
        },
    }
}

fn ordered_vks(keys: &[String]) -> Vec<u16> {
    ordered_for_press(keys)
        .iter()
        .filter_map(|key| vk_from_name(key))
        .collect()
}

pub fn tap_keys(keys: &[String]) -> Result<(), String> {
    let vks = ordered_vks(keys);
    let mut inputs = Vec::with_capacity(vks.len() * 2);
    for vk in &vks {
        inputs.push(keyboard_input(*vk, false));
    }
    for vk in vks.iter().rev() {
        inputs.push(keyboard_input(*vk, true));
    }
    send(&inputs)
}

pub fn press_keys(keys: &[String]) -> Result<(), String> {
    let inputs: Vec<INPUT> = ordered_vks(keys)
        .iter()
        .map(|vk| keyboard_input(*vk, false))
        .collect();
    send(&inputs)
}

pub fn release_keys(keys: &[String]) -> Result<(), String> {
    let mut vks = ordered_vks(keys);
    vks.reverse();
    let inputs: Vec<INPUT> = vks.iter().map(|vk| keyboard_input(*vk, true)).collect();
    send(&inputs)
}

fn button_flags(button: MouseButton, down: bool) -> (MOUSE_EVENT_FLAGS, u32) {
    match button {
        MouseButton::Left => (
            if down {
                MOUSEEVENTF_LEFTDOWN
            } else {
                MOUSEEVENTF_LEFTUP
            },
            0,
        ),
        MouseButton::Right => (
            if down {
                MOUSEEVENTF_RIGHTDOWN
            } else {
                MOUSEEVENTF_RIGHTUP
            },
            0,
        ),
        MouseButton::Middle => (
            if down {
                MOUSEEVENTF_MIDDLEDOWN
            } else {
                MOUSEEVENTF_MIDDLEUP
            },
            0,
        ),
        MouseButton::X1 => (
            if down {
                MOUSEEVENTF_XDOWN
            } else {
                MOUSEEVENTF_XUP
            },
            XBUTTON1,
        ),
        MouseButton::X2 => (
            if down {
                MOUSEEVENTF_XDOWN
            } else {
                MOUSEEVENTF_XUP
            },
            XBUTTON2,
        ),
    }
}

pub fn click_button(button: MouseButton) -> Result<(), String> {
    let (down_flag, data) = button_flags(button, true);
    let (up_flag, _) = button_flags(button, false);
    send(&[
        mouse_input(down_flag, data),
        mouse_input(up_flag, data),
    ])
}

pub fn press_button(button: MouseButton) -> Result<(), String> {
    let (flag, data) = button_flags(button, true);
    send(&[mouse_input(flag, data)])
}

pub fn release_button(button: MouseButton) -> Result<(), String> {
    let (flag, data) = button_flags(button, false);
    send(&[mouse_input(flag, data)])
}

pub fn replay_button(button: MouseButton, down: bool) -> Result<(), String> {
    let (flag, data) = button_flags(button, down);
    send(&[mouse_input(flag, data)])
}

pub fn scroll(direction: ScrollDirection, amount: i32) -> Result<(), String> {
    let amount = amount.clamp(1, 50);
    scroll_raw(direction, 120 * amount)
}

/// Sends an exact wheel delta. Used when the scroll settings transform or
/// accelerate a physical wheel event.
pub fn scroll_raw(direction: ScrollDirection, delta: i32) -> Result<(), String> {
    let delta = delta.abs().max(1);
    let (flag, data) = match direction {
        ScrollDirection::Up => (MOUSEEVENTF_WHEEL, delta as u32),
        ScrollDirection::Down => (MOUSEEVENTF_WHEEL, (-delta) as u32),
        ScrollDirection::Right => (MOUSEEVENTF_HWHEEL, delta as u32),
        ScrollDirection::Left => (MOUSEEVENTF_HWHEEL, (-delta) as u32),
    };
    send(&[mouse_input(flag, data)])
}

/// Types text as Unicode code units. This bypasses the keyboard layout, so
/// non-Latin characters arrive intact.
pub fn type_text(text: &str) -> Result<(), String> {
    let mut inputs = Vec::new();
    for unit in text.encode_utf16() {
        for up in [false, true] {
            let mut flags = KEYEVENTF_UNICODE;
            if up {
                flags |= KEYEVENTF_KEYUP;
            }
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: INJECTION_TAG,
                    },
                },
            });
        }
    }
    // Send in chunks so a very long text does not exceed the API's practical
    // batch size while still holding every code unit in order.
    for chunk in inputs.chunks(256) {
        send(chunk)?;
    }
    Ok(())
}

pub fn media_key(key: &str) -> Result<(), String> {
    tap_keys(&[key.to_string()])
}

pub fn window_action(action: WindowAction) -> Result<(), String> {
    match action {
        WindowAction::SnapLeft => return tap_keys(&["win".into(), "left".into()]),
        WindowAction::SnapRight => return tap_keys(&["win".into(), "right".into()]),
        _ => {}
    }

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err("No foreground window to act on.".into());
    }

    if action == WindowAction::Close {
        // WM_SYSCOMMAND / SC_CLOSE. `PostMessageW` is asynchronous, which is
        // what a close button press does.
        return unsafe {
            PostMessageW(
                hwnd,
                0x0112,
                windows::Win32::Foundation::WPARAM(0xF060),
                windows::Win32::Foundation::LPARAM(0),
            )
        }
        .map_err(|error| format!("Could not close the window: {error}"));
    }

    let changed = match action {
        WindowAction::Minimize => unsafe { ShowWindow(hwnd, SW_MINIMIZE) },
        WindowAction::Maximize => unsafe { ShowWindow(hwnd, SW_MAXIMIZE) },
        WindowAction::Restore => unsafe { ShowWindow(hwnd, SW_RESTORE) },
        WindowAction::Close | WindowAction::SnapLeft | WindowAction::SnapRight => {
            return Ok(());
        }
    };
    if !changed.as_bool() {
        return Err("Windows could not change the foreground window.".into());
    }
    Ok(())
}

pub fn launch(target: &str) -> Result<(), String> {    let target = target.trim();
    if target.is_empty() {
        return Err("Enter a program, file or URL to launch.".into());
    }
    let target = windows::core::HSTRING::from(target);
    let result = unsafe {
        ShellExecuteW(
            windows::Win32::Foundation::HWND::default(),
            windows::core::w!("open"),
            &target,
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err(format!(
            "Windows could not open “{}”. Check the path or URL.",
            target
        ));
    }
    Ok(())
}

/// The system wheel setting in lines per notch. Shown for reference; the app's
/// own speed multiplier is separate.
pub fn wheel_lines() -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };
    let mut lines: u32 = 3;
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWHEELSCROLLLINES,
            0,
            Some(&mut lines as *mut u32 as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    lines.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_flags_are_distinct() {
        let (left, _) = button_flags(MouseButton::Left, true);
        let (right, _) = button_flags(MouseButton::Right, true);
        let (x1, data1) = button_flags(MouseButton::X1, true);
        let (x2, data2) = button_flags(MouseButton::X2, true);
        assert_ne!(left, right);
        assert_eq!(x1, x2);
        assert_ne!(data1, data2);
    }

    #[test]
    fn key_order_is_deterministic() {
        let keys = vec!["z".to_string(), "shift".to_string(), "ctrl".to_string()];
        assert_eq!(ordered_vks(&keys), vec![0x11, 0x10, 0x5A]);
    }

    #[test]
    fn unknown_key_names_are_skipped() {
        let keys = vec!["bogus".to_string(), "a".to_string()];
        assert_eq!(ordered_vks(&keys), vec![0x41]);
    }
}
