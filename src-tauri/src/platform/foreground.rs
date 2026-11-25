//! Foreground application tracking for per-app exceptions.
//!
//! A lightweight poller (four times per second) reads the foreground window's
//! executable name and hands it to the engine service. Polling avoids a second
//! `SetWinEventHook` message loop and is negligible for this workload.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, TRUE};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible,
};

use crate::platform::service::EngineService;

static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApp {
    pub process: String,
    pub title: String,
}

pub fn start(service: Arc<EngineService>) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = thread::Builder::new()
        .name("foreground-watcher".into())
        .spawn(move || {
            let mut last: Option<String> = None;
            loop {
                let current = foreground_process();
                if current != last {
                    service.set_foreground(current.clone());
                    last = current;
                }
                thread::sleep(Duration::from_millis(250));
            }
        });
}

/// Lowercase executable name of the foreground window, if it can be read.
pub fn foreground_process() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        process_name_for_window(hwnd)
    }
}

unsafe fn process_name_for_window(hwnd: HWND) -> Option<String> {
    if hwnd.0.is_null() {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    process_name_for_pid(pid)
}

unsafe fn process_name_for_pid(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_WIN32,
        PWSTR(buffer.as_mut_ptr()),
        &mut size,
    );
    let _ = CloseHandle(handle);
    result.ok()?;

    let path = String::from_utf16_lossy(&buffer[..size as usize]);
    let name = path.rsplit(['\\', '/']).next()?.trim().to_lowercase();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Visible top-level windows, de-duplicated by process. Used by the Exceptions
/// page so an app can be added without typing its executable name.
pub fn list_open_apps() -> Vec<OpenApp> {
    let mut apps: Vec<OpenApp> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_window),
            LPARAM(&mut apps as *mut Vec<OpenApp> as isize),
        );
    }

    apps.sort_by(|a, b| {
        a.process
            .to_lowercase()
            .cmp(&b.process.to_lowercase())
            .then_with(|| a.title.cmp(&b.title))
    });
    let mut seen = Vec::new();
    apps.retain(|app| {
        let key = app.process.to_lowercase();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    apps
}

unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }
    let length = GetWindowTextLengthW(hwnd);
    if length <= 0 {
        return TRUE;
    }

    let mut buffer = vec![0u16; length as usize + 1];
    let copied = GetWindowTextW(hwnd, &mut buffer);
    if copied <= 0 {
        return TRUE;
    }
    let title: String = String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .chars()
        .take(80)
        .collect();
    if title.is_empty() {
        return TRUE;
    }

    if let Some(process) = process_name_for_window(hwnd) {
        let list = &mut *(lparam.0 as *mut Vec<OpenApp>);
        list.push(OpenApp { process, title });
    }
    TRUE
}
