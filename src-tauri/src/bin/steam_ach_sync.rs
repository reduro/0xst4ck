use std::env;
use std::ffi::{c_void, CString};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

type FnInit = unsafe extern "system" fn() -> bool;
type FnShutdown = unsafe extern "system" fn();
type FnRunCallbacks = unsafe extern "system" fn();
type FnRegisterCallback = unsafe extern "system" fn(*mut c_void, i32);
type FnUnregisterCallback = unsafe extern "system" fn(*mut c_void);
type FnGetStats = unsafe extern "system" fn() -> usize;
type FnRequestCurrentStats = unsafe extern "system" fn(usize) -> u64;
type FnSetAchievement = unsafe extern "system" fn(usize, *const u8) -> bool;
type FnClearAchievement = unsafe extern "system" fn(usize, *const u8) -> bool;
type FnGetAchievement = unsafe extern "system" fn(usize, *const u8, *mut bool) -> bool;
type FnStoreStats = unsafe extern "system" fn(usize) -> bool;

#[repr(C)]
struct CallbackVTable {
    run1: unsafe extern "system" fn(*mut c_void, *mut c_void),
    run2: unsafe extern "system" fn(*mut c_void, *mut c_void, bool, u64),
    get_size: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
struct Callback {
    vptr: *const CallbackVTable,
    flags: i32,
    callback_id: i32,
}

static STATS_RECEIVED: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn cb_run1(_this: *mut c_void, _param: *mut c_void) {
    STATS_RECEIVED.store(true, Ordering::SeqCst);
}

unsafe extern "system" fn cb_run2(_this: *mut c_void, _param: *mut c_void, _fail: bool, _handle: u64) {
    STATS_RECEIVED.store(true, Ordering::SeqCst);
}

unsafe extern "system" fn cb_get_size(_this: *mut c_void) -> i32 {
    32
}

static VTABLE: CallbackVTable = CallbackVTable {
    run1: cb_run1,
    run2: cb_run2,
    get_size: cb_get_size,
};

fn find_steam_api64() -> Option<PathBuf> {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("steam_api64.dll");
            if candidate.exists() {
                return Some(candidate);
            }
            let candidate_tools = parent.join("tools").join("steam_api64.dll");
            if candidate_tools.exists() {
                return Some(candidate_tools);
            }
        }
    }
    for cand in &[
        "tools/steam_api64.dll",
        "src-tauri/tools/steam_api64.dll",
        "C:\\Serveur\\clemtout-launcher-v2\\tools\\steam_api64.dll",
        "C:\\Serveur\\.venv\\clemtout-launcher\\backend\\Patch-steam\\steam_api64.dll",
    ] {
        let p = PathBuf::from(cand);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(r#"{{"error": "Usage: steam_ach_sync <appid> <action: unlock|clear|check|unlock_all|clear_all> [apiname...]"}}"#);
        std::process::exit(1);
    }

    let appid = &args[1];
    let action = &args[2];

    unsafe {
        env::set_var("SteamAppId", appid);
        env::set_var("SteamGameId", appid);
        env::set_var("SteamOverlayGameId", appid);
    }

    // Always write steam_appid.txt next to exe and in cwd before loading steam_api64
    let _ = fs::write("steam_appid.txt", appid);
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let _ = fs::write(parent.join("steam_appid.txt"), appid);
        }
    }

    let Some(dll_path) = find_steam_api64() else {
        eprintln!(r#"{{"error": "steam_api64.dll not found"}}"#);
        std::process::exit(2);
    };

    let c_path = CString::new(dll_path.to_string_lossy().as_bytes()).unwrap();
    let h_mod = unsafe { LoadLibraryA(PCSTR(c_path.as_ptr() as *const u8)) };
    if h_mod.is_err() {
        eprintln!(r#"{{"error": "Failed to load steam_api64.dll"}}"#);
        std::process::exit(3);
    }
    let h_mod = h_mod.unwrap();

    let steam_init: FnInit = {
        let mut ptr = None;
        for v in &["SteamAPI_Init", "SteamAPI_InitSafe", "SteamAPI_InitFlat"] {
            let c = CString::new(*v).unwrap();
            if let Some(p) = unsafe { GetProcAddress(h_mod, PCSTR(c.as_ptr() as *const u8)) } {
                ptr = Some(unsafe { std::mem::transmute::<_, FnInit>(p) });
                break;
            }
        }
        ptr.unwrap_or_else(|| {
            eprintln!(r#"{{"error": "Export SteamAPI_Init not found"}}"#);
            std::process::exit(4);
        })
    };

    let get_stats: FnGetStats = {
        let mut ptr = None;
        for v in &["SteamAPI_SteamUserStats_v012", "SteamAPI_SteamUserStats_v013", "SteamAPI_SteamUserStats_v011"] {
            let c = CString::new(*v).unwrap();
            if let Some(p) = unsafe { GetProcAddress(h_mod, PCSTR(c.as_ptr() as *const u8)) } {
                ptr = Some(unsafe { std::mem::transmute::<_, FnGetStats>(p) });
                break;
            }
        }
        ptr.unwrap_or_else(|| {
            eprintln!(r#"{{"error": "Export SteamUserStats not found"}}"#);
            std::process::exit(4);
        })
    };

    macro_rules! get_proc {
        ($name:expr, $ty:ty) => {{
            let c_name = CString::new($name).unwrap();
            let p = unsafe { GetProcAddress(h_mod, PCSTR(c_name.as_ptr() as *const u8)) };
            match p {
                Some(func) => unsafe { std::mem::transmute::<_, $ty>(func) },
                None => {
                    eprintln!(concat!(r#"{{"error": "Export "#, $name, r#" not found"}}"#));
                    std::process::exit(4);
                }
            }
        }};
    }

    let steam_shutdown: FnShutdown = get_proc!("SteamAPI_Shutdown", FnShutdown);
    let steam_callbacks: FnRunCallbacks = get_proc!("SteamAPI_RunCallbacks", FnRunCallbacks);
    let reg_cb: FnRegisterCallback = get_proc!("SteamAPI_RegisterCallback", FnRegisterCallback);
    let unreg_cb: FnUnregisterCallback = get_proc!("SteamAPI_UnregisterCallback", FnUnregisterCallback);
    let req_stats: FnRequestCurrentStats = get_proc!("SteamAPI_ISteamUserStats_RequestCurrentStats", FnRequestCurrentStats);
    let set_ach: FnSetAchievement = get_proc!("SteamAPI_ISteamUserStats_SetAchievement", FnSetAchievement);
    let clear_ach: FnClearAchievement = get_proc!("SteamAPI_ISteamUserStats_ClearAchievement", FnClearAchievement);
    let get_ach: FnGetAchievement = get_proc!("SteamAPI_ISteamUserStats_GetAchievement", FnGetAchievement);
    let store_stats: FnStoreStats = get_proc!("SteamAPI_ISteamUserStats_StoreStats", FnStoreStats);

    if !unsafe { steam_init() } {
        eprintln!(r#"{{"error": "SteamAPI_Init failed (is Steam running?)"}}"#);
        std::process::exit(5);
    }

    let stats_ptr = unsafe { get_stats() };
    if stats_ptr == 0 {
        eprintln!(r#"{{"error": "Failed to get ISteamUserStats pointer"}}"#);
        unsafe { steam_shutdown() };
        std::process::exit(6);
    }

    let mut cb = Callback {
        vptr: &VTABLE,
        flags: 0,
        callback_id: 1101, // UserStatsReceived_t
    };

    unsafe {
        reg_cb(&mut cb as *mut _ as *mut c_void, 1101);
        req_stats(stats_ptr);
    }

    // Wait up to 1.5 seconds for stats to arrive
    for _ in 0..30 {
        unsafe { steam_callbacks() };
        if STATS_RECEIVED.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let mut modified = false;

    match action.as_str() {
        "unlock" => {
            if args.len() < 4 {
                eprintln!(r#"{{"error": "Missing achievement name"}}"#);
            } else {
                let name = &args[3];
                let c_name = CString::new(name.as_bytes()).unwrap();
                let ok = unsafe { set_ach(stats_ptr, c_name.as_ptr() as *const u8) };
                if ok {
                    modified = true;
                    println!(r#"{{"success": true, "action": "unlock", "apiname": "{}"}}"#, name);
                } else {
                    println!(r#"{{"success": false, "error": "SetAchievement failed"}}"#);
                }
            }
        }
        "unlock_all" => {
            let names = &args[3..];
            let mut count = 0;
            for n in names {
                let c_name = CString::new(n.as_bytes()).unwrap();
                let ok = unsafe { set_ach(stats_ptr, c_name.as_ptr() as *const u8) };
                if ok {
                    count += 1;
                }
            }
            if count > 0 {
                modified = true;
            }
            println!(r#"{{"success": true, "action": "unlock_all", "count": {}}}"#, count);
        }
        "clear" => {
            if args.len() < 4 {
                eprintln!(r#"{{"error": "Missing achievement name"}}"#);
            } else {
                let name = &args[3];
                let c_name = CString::new(name.as_bytes()).unwrap();
                let ok = unsafe { clear_ach(stats_ptr, c_name.as_ptr() as *const u8) };
                if ok {
                    modified = true;
                    println!(r#"{{"success": true, "action": "clear", "apiname": "{}"}}"#, name);
                } else {
                    println!(r#"{{"success": false, "error": "ClearAchievement failed"}}"#);
                }
            }
        }
        "clear_all" => {
            let names = &args[3..];
            let mut count = 0;
            for n in names {
                let c_name = CString::new(n.as_bytes()).unwrap();
                let ok = unsafe { clear_ach(stats_ptr, c_name.as_ptr() as *const u8) };
                if ok {
                    count += 1;
                }
            }
            if count > 0 {
                modified = true;
            }
            println!(r#"{{"success": true, "action": "clear_all", "count": {}}}"#, count);
        }
        "check" => {
            if args.len() < 4 {
                eprintln!(r#"{{"error": "Missing achievement name"}}"#);
            } else {
                let name = &args[3];
                let c_name = CString::new(name.as_bytes()).unwrap();
                let mut achieved = false;
                let ok = unsafe { get_ach(stats_ptr, c_name.as_ptr() as *const u8, &mut achieved) };
                println!(r#"{{"success": {}, "apiname": "{}", "unlocked": {}}}"#, ok, name, achieved);
            }
        }
        other => {
            eprintln!(r#"{{"error": "Unknown action: {}"}}"#, other);
        }
    }

    if modified {
        unsafe {
            store_stats(stats_ptr);
        }
        for _ in 0..8 {
            unsafe { steam_callbacks() };
            thread::sleep(Duration::from_millis(50));
        }
    }

    unsafe {
        unreg_cb(&mut cb as *mut _ as *mut c_void);
        steam_shutdown();
    }
}
