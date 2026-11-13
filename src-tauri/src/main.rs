#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
fn is_another_instance_running() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    let wide_name: Vec<u16> = OsStr::new("Local\\ClemtoutLauncherSingleInstanceMutex")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let _ = CreateMutexW(None, true, PCWSTR(wide_name.as_ptr()));
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

fn main() {
    #[cfg(windows)]
    if is_another_instance_running() {
        std::process::exit(0);
    }

    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
        let log_content = format!("PANIC at {}: {}\n{:?}", location, payload, std::backtrace::Backtrace::capture());
        let _ = std::fs::write("C:\\Serveur\\clemtout-launcher-v2\\panic.log", log_content);
    }));

    clemtout_launcher_lib::run()
}
