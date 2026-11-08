#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
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
