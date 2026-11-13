// Pure Rust version.dll proxy & Steam achievements interceptor
// Universal loader-level injection without CreateRemoteThread.
// Hooks SteamAPI_ISteamUserStats_SetAchievement via EAT without ICF collisions.

#![cfg(windows)]

use std::ffi::{c_void, CStr};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::core::PCSTR;
use windows::Win32::Foundation::{BOOL, HINSTANCE, HMODULE, TRUE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
use windows::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS};
use windows::Win32::System::Threading::{CreateThread, THREAD_CREATION_FLAGS};

const DLL_PROCESS_ATTACH: u32 = 1;

static REAL_VERSION: OnceLock<usize> = OnceLock::new();
static mut ORIGINAL_SET_ACHIEVEMENT: usize = 0;

fn get_real_version() -> usize {
    *REAL_VERSION.get_or_init(|| {
        let path = b"C:\\Windows\\System32\\version.dll\0";
        let h = unsafe { LoadLibraryA(PCSTR(path.as_ptr())) };
        h.map(|m| m.0 as usize).unwrap_or(0)
    })
}

macro_rules! forward_export {
    ($name:ident, $sym:expr, ($($param:ident: $pty:ty),*) -> $rty:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn $name($($param: $pty),*) -> $rty {
            let h = get_real_version();
            if h != 0 {
                let func_name = concat!($sym, "\0");
                if let Some(p) = GetProcAddress(HMODULE(h as *mut _), PCSTR(func_name.as_ptr())) {
                    let f: unsafe extern "system" fn($($pty),*) -> $rty = std::mem::transmute(p);
                    return f($($param),*);
                }
            }
            std::mem::zeroed()
        }
    };
}

forward_export!(GetFileVersionInfoA, "GetFileVersionInfoA", (lptstrfilename: *const u8, dwlocalhandle: u32, dwlen: u32, lpdata: *mut c_void) -> BOOL);
forward_export!(GetFileVersionInfoByHandle, "GetFileVersionInfoByHandle", (unknown1: i32, unknown2: *mut c_void, unknown3: *mut c_void, unknown4: *mut c_void) -> BOOL);
forward_export!(GetFileVersionInfoExA, "GetFileVersionInfoExA", (dwflags: u32, lpwstrfilename: *const u8, dwhandle: u32, dwlen: u32, lpdata: *mut c_void) -> BOOL);
forward_export!(GetFileVersionInfoExW, "GetFileVersionInfoExW", (dwflags: u32, lpwstrfilename: *const u16, dwhandle: u32, dwlen: u32, lpdata: *mut c_void) -> BOOL);
forward_export!(GetFileVersionInfoSizeA, "GetFileVersionInfoSizeA", (lptstrfilename: *const u8, lpdwsize: *mut u32) -> u32);
forward_export!(GetFileVersionInfoSizeExA, "GetFileVersionInfoSizeExA", (dwflags: u32, lpwstrfilename: *const u8, lpdwsize: *mut u32) -> u32);
forward_export!(GetFileVersionInfoSizeExW, "GetFileVersionInfoSizeExW", (dwflags: u32, lpwstrfilename: *const u16, lpdwsize: *mut u32) -> u32);
forward_export!(GetFileVersionInfoSizeW, "GetFileVersionInfoSizeW", (lptstrfilename: *const u16, lpdwsize: *mut u32) -> u32);
forward_export!(GetFileVersionInfoW, "GetFileVersionInfoW", (lptstrfilename: *const u16, dwlocalhandle: u32, dwlen: u32, lpdata: *mut c_void) -> BOOL);
forward_export!(VerFindFileA, "VerFindFileA", (uflags: u32, szfilename: *const u8, szwindir: *const u8, szappdir: *const u8, szcurdir: *mut u8, pucurdirle: *mut u32, szdestdir: *mut u8, pudestdirle: *mut u32) -> u32);
forward_export!(VerFindFileW, "VerFindFileW", (uflags: u32, szfilename: *const u16, szwindir: *const u16, szappdir: *const u16, szcurdir: *mut u16, pucurdirle: *mut u32, szdestdir: *mut u16, pudestdirle: *mut u32) -> u32);
forward_export!(VerInstallFileA, "VerInstallFileA", (uflags: u32, szsrcfilename: *const u8, szdestfilename: *const u8, szsrcdir: *const u8, szdestdir: *const u8, szcurdir: *const u8, sztmpfile: *mut u8, putmpfilele: *mut u32) -> u32);
forward_export!(VerInstallFileW, "VerInstallFileW", (uflags: u32, szsrcfilename: *const u16, szdestfilename: *const u16, szsrcdir: *const u16, szdestdir: *const u16, szcurdir: *const u16, sztmpfile: *mut u16, putmpfilele: *mut u32) -> u32);
forward_export!(VerLanguageNameA, "VerLanguageNameA", (wlang: u32, szlang: *mut u8, nsize: u32) -> u32);
forward_export!(VerLanguageNameW, "VerLanguageNameW", (wlang: u32, szlang: *mut u16, nsize: u32) -> u32);
forward_export!(VerQueryValueA, "VerQueryValueA", (pblock: *const c_void, lpsubblock: *const u8, lplpbuffer: *mut *mut c_void, pulen: *mut u32) -> BOOL);
forward_export!(VerQueryValueW, "VerQueryValueW", (pblock: *const c_void, lpsubblock: *const u16, lplpbuffer: *mut *mut c_void, pulen: *mut u32) -> BOOL);

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_h: HINSTANCE, reason: u32, _r: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            let _ = CreateThread(None, 0, Some(worker_entry), None, THREAD_CREATION_FLAGS(0), None);
        }
    }
    TRUE
}

unsafe extern "system" fn worker_entry(_param: *mut c_void) -> u32 {
    let mut patched = false;

    // Scan for steam_api64.dll / steam_api.dll when loaded
    for _ in 0..600 { // 30s
        if !patched {
            let module = if let Ok(m) = GetModuleHandleA(PCSTR(b"steam_api64.dll\0".as_ptr())) {
                if !m.is_invalid() { Some(m) } else { None }
            } else if let Ok(m) = GetModuleHandleA(PCSTR(b"steam_api.dll\0".as_ptr())) {
                if !m.is_invalid() { Some(m) } else { None }
            } else {
                None
            };

            if let Some(m) = module {
                // 1. Neutralize restart checks cleanly
                patch_export(m, b"SteamAPI_RestartAppIfNecessary\0", &[0x31, 0xC0, 0xC3]);
                patch_export(m, b"SteamAPI_RestartApp\0", &[0x31, 0xC0, 0xC3]);
                patch_export(m, b"SteamAPI_IsSteamRunning\0", &[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3]);

                // 2. Hook SteamAPI_ISteamUserStats_SetAchievement via EAT
                if let Some(orig) = hook_eat(m, b"SteamAPI_ISteamUserStats_SetAchievement\0", hook_set_achievement as *const () as usize) {
                    ORIGINAL_SET_ACHIEVEMENT = orig;
                }

                patched = true;
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    0
}

unsafe extern "system" fn hook_set_achievement(stats: usize, pch_name: *const u8) -> bool {
    if !pch_name.is_null() {
        let ach_name = CStr::from_ptr(pch_name as *const i8).to_string_lossy().to_string();
        on_achievement_unlocked(&ach_name);
    }

    if ORIGINAL_SET_ACHIEVEMENT != 0 {
        let orig: unsafe extern "system" fn(usize, *const u8) -> bool = std::mem::transmute(ORIGINAL_SET_ACHIEVEMENT);
        orig(stats, pch_name)
    } else {
        true
    }
}

fn on_achievement_unlocked(apiname: &str) {
    let appid = std::env::var("CLEMTOUT_REAL_APPID")
        .or_else(|_| std::env::var("SteamAppId"))
        .unwrap_or_else(|_| "1012880".to_string());

    // 1. Append to events.log
    if let Ok(appdata) = std::env::var("APPDATA") {
        let ach_dir = PathBuf::from(&appdata).join("clemtoutlauncher").join("achievements");
        let _ = std::fs::create_dir_all(&ach_dir);

        let events_path = ach_dir.join("events.log");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&events_path) {
            let _ = writeln!(f, "{}:UNLOCK:{}", appid, apiname);
        }

        // Also update local JSON store directly
        let store_file = ach_dir.join(format!("{}.json", appid));
        let mut store: serde_json::Value = if store_file.exists() {
            std::fs::read_to_string(&store_file)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| serde_json::json!({ "appid": appid, "unlocked": [], "manual_locked": [] }))
        } else {
            serde_json::json!({ "appid": appid, "unlocked": [], "manual_locked": [] })
        };

        if let Some(manual_locked) = store.get_mut("manual_locked").and_then(|m| m.as_array_mut()) {
            manual_locked.retain(|v| !v.as_str().map(|s| s.eq_ignore_ascii_case(apiname)).unwrap_or(false));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let already = store.get("unlocked")
            .and_then(|u| u.as_array())
            .map(|arr| arr.iter().any(|item| item.get("apiname").and_then(|a| a.as_str()).map(|s| s.eq_ignore_ascii_case(apiname)).unwrap_or(false)))
            .unwrap_or(false);

        if !already {
            if let Some(unlocked_arr) = store.get_mut("unlocked").and_then(|u| u.as_array_mut()) {
                unlocked_arr.push(serde_json::json!({
                    "apiname": apiname,
                    "unlocked_at": now
                }));
            }
        }
        let _ = std::fs::write(&store_file, serde_json::to_string_pretty(&store).unwrap_or_default());
    }

    // 2. Call steam_ach_sync in background to unlock directly on Steam!
    for cand in &[
        "C:\\Serveur\\clemtout-launcher-v2\\tools\\steam_ach_sync.exe",
        "tools\\steam_ach_sync.exe",
    ] {
        let p = PathBuf::from(cand);
        if p.exists() {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = std::process::Command::new(p)
                .arg(&appid)
                .arg("unlock")
                .arg(apiname)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            break;
        }
    }
}

unsafe fn hook_eat(module: HMODULE, target_name: &[u8], detour: usize) -> Option<usize> {
    let base = module.0 as *const u8;
    let e_magic = *(base as *const u16);
    if e_magic != 0x5A4D { // 'MZ'
        return None;
    }

    let e_lfanew = *(base.add(0x3C) as *const i32) as usize;
    let pe_sig = *(base.add(e_lfanew) as *const u32);
    if pe_sig != 0x00004550 { // 'PE\0\0'
        return None;
    }

    let magic_opt = *(base.add(e_lfanew + 24) as *const u16);
    let exp_data_dir_offset = if magic_opt == 0x20B { // PE32+ (64-bit)
        e_lfanew + 24 + 112
    } else { // PE32 (32-bit)
        e_lfanew + 24 + 96
    };

    let exp_va = *(base.add(exp_data_dir_offset) as *const u32) as usize;
    let exp_size = *(base.add(exp_data_dir_offset + 4) as *const u32) as usize;
    if exp_va == 0 || exp_size == 0 {
        return None;
    }

    let exp_dir = base.add(exp_va);
    let num_names = *(exp_dir.add(24) as *const u32) as usize;
    let a_funcs_rva = *(exp_dir.add(28) as *const u32) as usize;
    let a_names_rva = *(exp_dir.add(32) as *const u32) as usize;
    let a_ord_rva = *(exp_dir.add(36) as *const u32) as usize;

    let functions = base.add(a_funcs_rva) as *mut u32;
    let names = base.add(a_names_rva) as *const u32;
    let ordinals = base.add(a_ord_rva) as *const u16;

    for i in 0..num_names {
        let name_rva = *names.add(i) as usize;
        let c_str = CStr::from_ptr(base.add(name_rva) as *const i8);
        if c_str.to_bytes_with_nul() == target_name {
            let ordinal = *ordinals.add(i) as usize;
            let original_rva = *functions.add(ordinal) as usize;
            let original_func = base.add(original_rva) as usize;

            let detour_rva = (detour - base as usize) as u32;
            let target_addr = functions.add(ordinal) as *mut c_void;

            let mut old = PAGE_PROTECTION_FLAGS(0);
            if VirtualProtect(target_addr, 4, PAGE_EXECUTE_READWRITE, &mut old).is_ok() {
                *functions.add(ordinal) = detour_rva;
                let mut _restore = PAGE_PROTECTION_FLAGS(0);
                let _ = VirtualProtect(target_addr, 4, old, &mut _restore);
                return Some(original_func);
            }
        }
    }
    None
}

unsafe fn patch_export(module: HMODULE, name: &[u8], bytes: &[u8]) {
    let Some(func) = GetProcAddress(module, PCSTR(name.as_ptr())) else {
        return;
    };
    let p = func as *mut u8;
    let mut old = PAGE_PROTECTION_FLAGS(0);
    if VirtualProtect(p as *const c_void, bytes.len(), PAGE_EXECUTE_READWRITE, &mut old).is_err() {
        return;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
    let mut _restore = PAGE_PROTECTION_FLAGS(0);
    let _ = VirtualProtect(p as *const c_void, bytes.len(), old, &mut _restore);
}
