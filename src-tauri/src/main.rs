// GUI application: Windows must never create a console window for it, in debug
// builds too. When logs are needed, start it with `--console` (or set
// `MOUSEMACRO_CONSOLE=1`) from an existing terminal to attach a console.
#![windows_subsystem = "windows"]

fn main() {
    attach_debug_console();
    mousemacro_lib::run();
}

/// Attaches stdout/stderr to the launching terminal, or allocates a new console
/// when there is none. Only active when explicitly requested, so normal launches
/// stay completely windowless.
#[cfg(windows)]
fn attach_debug_console() {
    use std::os::windows::io::AsRawHandle;

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{
        AllocConsole, AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
        STD_OUTPUT_HANDLE,
    };

    let requested = std::env::args().any(|arg| arg == "--console")
        || std::env::var_os("MOUSEMACRO_CONSOLE").is_some();
    if !requested {
        return;
    }

    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            let _ = AllocConsole();
        }
    }

    // Re-open the console device and point the CRT handles at it. The file is
    // leaked on purpose: the handle must stay valid for the process lifetime.
    if let Ok(file) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("CONOUT$")
    {
        let handle = HANDLE(file.as_raw_handle() as *mut core::ffi::c_void);
        unsafe {
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
            let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
        }
        std::mem::forget(file);
    }
}

#[cfg(not(windows))]
fn attach_debug_console() {}
