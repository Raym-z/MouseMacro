// Temporary diagnostic example: verify WH_KEYBOARD_LL reception in this Rust toolchain.
#[cfg(windows)]
fn main() {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYBD_EVENT_FLAGS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HHOOK, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
    };

    static COUNT: AtomicU64 = AtomicU64::new(0);
    static MOUSE_COUNT: AtomicU64 = AtomicU64::new(0);

    unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            COUNT.fetch_add(1, Ordering::SeqCst);
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            MOUSE_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    let (tx, rx) = mpsc::channel::<usize>();
    std::thread::spawn(move || unsafe {
        let module = GetModuleHandleW(None).unwrap();
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), HINSTANCE(module.0), 0).unwrap();
        let mouse =
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE(module.0), 0).unwrap();
        println!("mouse hook {:?}", mouse);
        tx.send(hook.0 as usize).unwrap();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });

    let hook = HHOOK(rx.recv().unwrap() as *mut core::ffi::c_void);
    println!("installed hook {:?}", hook);
    std::thread::sleep(Duration::from_millis(500));
    unsafe {
        keybd_event(0x47, 0x22, KEYBD_EVENT_FLAGS(0), 0);
        std::thread::sleep(Duration::from_millis(80));
        keybd_event(0x47, 0x22, KEYBD_EVENT_FLAGS(2), 0);
        std::thread::sleep(Duration::from_millis(700));
        let _ = UnhookWindowsHookEx(hook);
    }
    println!(
        "callbacks: {} mouse: {}",
        COUNT.load(Ordering::SeqCst),
        MOUSE_COUNT.load(Ordering::SeqCst)
    );
}

#[cfg(not(windows))]
fn main() {}
