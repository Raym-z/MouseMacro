use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

/// Relaunches the current executable through the UAC `runas` verb.
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| {
            let value = arg.to_string_lossy().to_string();
            if value.contains(' ') {
                format!("\"{value}\"")
            } else {
                value
            }
        })
        .collect();
    let params = args.join(" ");

    let verb = wide(std::ffi::OsStr::new("runas"));
    let exe_wide = wide(exe.as_os_str());
    let params_wide = wide(std::ffi::OsStr::new(&params));

    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR(params_wide.as_ptr()),
            None,
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!(
            "Could not restart as administrator (ShellExecute error {}).",
            result.0 as isize
        ))
    }
}
