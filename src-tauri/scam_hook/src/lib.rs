// scam_hook.dll — Injected into "Scam Your Friends" by the launcher.
// Inline-hooks WinHttp via MinHook so the response to
// kolkata-auth.jataterworldwide.com/auth is faked to {"ok":true,...}
// without touching network, hosts, or certs.

#![cfg(windows)]
#![allow(non_snake_case)]

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::sync::Mutex;
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, BOOL, TRUE as WIN_TRUE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

// HINTERNET is just an opaque handle (void*). Alias for readability.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct HINTERNET(pub *mut c_void);
unsafe impl Send for HINTERNET {}
unsafe impl Sync for HINTERNET {}

const TARGET_HOST: &str = "kolkata-auth.jataterworldwide.com";
const FAKE_BODY: &[u8] = br#"{"ok":true,"success":true,"authenticated":true,"result":"OK","steamid":"76561199039550522","appid":"4954910"}"#;

// ---------- logging ----------
fn log_line(s: &str) {
    use std::io::Write;
    let temp = std::env::var("TEMP").unwrap_or_else(|_| "C:\\Temp".into());
    let path = format!("{}\\scam_hook.log", temp);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let ts = chrono_lite();
        let _ = writeln!(f, "[{}] {}", ts, s);
    }
}

fn chrono_lite() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}.{:03}", d.as_secs(), d.subsec_millis())
}

// WINHTTP_QUERY_STATUS_CODE = 19, WINHTTP_QUERY_FLAG_NUMBER = 0x20000000
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x20000000;

// ---------- MinHook raw bindings ----------
// We call minhook-sys FFI directly; keeps the crate graph small.

const MH_OK: i32 = 0;

#[link(name = "MinHook", kind = "static")]
extern "system" {
    fn MH_Initialize() -> i32;
    fn MH_CreateHook(target: *mut c_void, detour: *mut c_void, original: *mut *mut c_void) -> i32;
    fn MH_EnableHook(target: *mut c_void) -> i32;
}

// ---------- state ----------

#[derive(Default)]
struct State {
    connects: HashMap<usize, bool>,
    requests: HashMap<usize, ReqState>,
}

#[derive(Default, Clone)]
struct ReqState {
    targeted: bool,
    read_pos: usize,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

fn mark_connect(h: HINTERNET) {
    STATE.lock().unwrap().connects.insert(h.0 as usize, true);
}
fn is_connect_targeted(h: HINTERNET) -> bool {
    STATE.lock().unwrap().connects.get(&(h.0 as usize)).copied().unwrap_or(false)
}
fn mark_request(h: HINTERNET) {
    STATE.lock().unwrap().requests.insert(h.0 as usize, ReqState { targeted: true, read_pos: 0 });
}
fn is_request_targeted(h: HINTERNET) -> bool {
    STATE.lock().unwrap().requests.get(&(h.0 as usize)).map(|s| s.targeted).unwrap_or(false)
}
fn advance_read(h: HINTERNET, n: usize) {
    if let Some(s) = STATE.lock().unwrap().requests.get_mut(&(h.0 as usize)) {
        s.read_pos += n;
    }
}
fn current_read_pos(h: HINTERNET) -> usize {
    STATE.lock().unwrap().requests.get(&(h.0 as usize)).map(|s| s.read_pos).unwrap_or(0)
}

// ---------- function types ----------

type FnConnect = unsafe extern "system" fn(HINTERNET, PCWSTR, u16, u32) -> HINTERNET;
type FnOpenRequest = unsafe extern "system" fn(HINTERNET, PCWSTR, PCWSTR, PCWSTR, PCWSTR, *const PCWSTR, u32) -> HINTERNET;
type FnSendRequest = unsafe extern "system" fn(HINTERNET, PCWSTR, u32, *const c_void, u32, u32, usize) -> BOOL;
type FnReceiveResponse = unsafe extern "system" fn(HINTERNET, *mut c_void) -> BOOL;
type FnQueryHeaders = unsafe extern "system" fn(HINTERNET, u32, PCWSTR, *mut c_void, *mut u32, *mut u32) -> BOOL;
type FnQueryDataAvailable = unsafe extern "system" fn(HINTERNET, *mut u32) -> BOOL;
type FnReadData = unsafe extern "system" fn(HINTERNET, *mut c_void, u32, *mut u32) -> BOOL;

// trampolines to the original functions
static mut ORIG_CONNECT: Option<FnConnect> = None;
static mut ORIG_OPEN_REQUEST: Option<FnOpenRequest> = None;
static mut ORIG_SEND_REQUEST: Option<FnSendRequest> = None;
static mut ORIG_RECEIVE_RESPONSE: Option<FnReceiveResponse> = None;
static mut ORIG_QUERY_HEADERS: Option<FnQueryHeaders> = None;
static mut ORIG_QUERY_DATA_AVAILABLE: Option<FnQueryDataAvailable> = None;
static mut ORIG_READ_DATA: Option<FnReadData> = None;

// ---------- hook implementations ----------

fn pcwstr_to_string(p: PCWSTR) -> String {
    if p.0.is_null() { return String::new(); }
    unsafe { widestring::U16CStr::from_ptr_str(p.0).to_string_lossy() }
}

unsafe extern "system" fn hk_connect(h: HINTERNET, server: PCWSTR, port: u16, flags: u32) -> HINTERNET {
    let server_str = pcwstr_to_string(server);
    log_line(&format!("WinHttpConnect server='{}' port={}", server_str, port));
    let real = ORIG_CONNECT.unwrap()(h, server, port, flags);
    if real.0 as usize != 0 && server_str.eq_ignore_ascii_case(TARGET_HOST) {
        log_line(&format!("  -> TARGETED hConnect={:p}", real.0));
        mark_connect(real);
    }
    real
}

unsafe extern "system" fn hk_open_request(
    hc: HINTERNET, verb: PCWSTR, obj: PCWSTR, ver: PCWSTR, refr: PCWSTR,
    accept: *const PCWSTR, flags: u32,
) -> HINTERNET {
    let real = ORIG_OPEN_REQUEST.unwrap()(hc, verb, obj, ver, refr, accept, flags);
    if real.0 as usize != 0 && is_connect_targeted(hc) {
        mark_request(real);
    }
    real
}

unsafe extern "system" fn hk_send_request(
    hr: HINTERNET, headers: PCWSTR, hlen: u32, opt: *const c_void, olen: u32, tlen: u32, ctx: usize,
) -> BOOL {
    if is_request_targeted(hr) {
        return WIN_TRUE.into();
    }
    ORIG_SEND_REQUEST.unwrap()(hr, headers, hlen, opt, olen, tlen, ctx)
}

unsafe extern "system" fn hk_receive_response(hr: HINTERNET, reserved: *mut c_void) -> BOOL {
    if is_request_targeted(hr) {
        return WIN_TRUE.into();
    }
    ORIG_RECEIVE_RESPONSE.unwrap()(hr, reserved)
}

unsafe extern "system" fn hk_query_headers(
    hr: HINTERNET, info: u32, name: PCWSTR, buf: *mut c_void, blen: *mut u32, idx: *mut u32,
) -> BOOL {
    if is_request_targeted(hr) {
        let base = info & !WINHTTP_QUERY_FLAG_NUMBER;
        let is_num = (info & WINHTTP_QUERY_FLAG_NUMBER) != 0;
        if base == WINHTTP_QUERY_STATUS_CODE && is_num {
            if !buf.is_null() && !blen.is_null() && *blen >= 4 {
                *(buf as *mut u32) = 200;
                *blen = 4;
                return WIN_TRUE.into();
            }
        }
        if !blen.is_null() { *blen = 0; }
        return WIN_TRUE.into();
    }
    ORIG_QUERY_HEADERS.unwrap()(hr, info, name, buf, blen, idx)
}

unsafe extern "system" fn hk_query_data_available(hr: HINTERNET, avail: *mut u32) -> BOOL {
    if is_request_targeted(hr) {
        let remaining = FAKE_BODY.len().saturating_sub(current_read_pos(hr));
        if !avail.is_null() { *avail = remaining as u32; }
        return WIN_TRUE.into();
    }
    ORIG_QUERY_DATA_AVAILABLE.unwrap()(hr, avail)
}

unsafe extern "system" fn hk_read_data(hr: HINTERNET, buf: *mut c_void, blen: u32, read: *mut u32) -> BOOL {
    if is_request_targeted(hr) {
        log_line(&format!("WinHttpReadData TARGETED hr={:p} blen={}", hr.0, blen));
        let pos = current_read_pos(hr);
        let remaining = FAKE_BODY.len().saturating_sub(pos);
        let n = std::cmp::min(remaining, blen as usize);
        if n > 0 && !buf.is_null() {
            std::ptr::copy_nonoverlapping(FAKE_BODY.as_ptr().add(pos), buf as *mut u8, n);
        }
        if !read.is_null() { *read = n as u32; }
        advance_read(hr, n);
        return WIN_TRUE.into();
    }
    ORIG_READ_DATA.unwrap()(hr, buf, blen, read)
}

// ---------- hook installation ----------

unsafe fn get_proc(dll_handle: windows::Win32::Foundation::HMODULE, name: &str) -> Option<*mut c_void> {
    let c = CString::new(name).ok()?;
    let fp = GetProcAddress(dll_handle, PCSTR(c.as_ptr() as *const u8))?;
    Some(fp as *mut c_void)
}

unsafe fn hook_one(
    target: *mut c_void,
    detour: *mut c_void,
    trampoline_slot: *mut *mut c_void,
) -> bool {
    if target.is_null() { return false; }
    let r = MH_CreateHook(target, detour, trampoline_slot);
    if r != MH_OK { return false; }
    MH_EnableHook(target) == MH_OK
}

unsafe fn install_hooks() -> bool {
    // Ensure winhttp.dll is loaded
    let w: Vec<u16> = "winhttp.dll".encode_utf16().chain(std::iter::once(0)).collect();
    let mut hmod = GetModuleHandleW(PCWSTR(w.as_ptr())).ok();
    if hmod.is_none() {
        if let Ok(h) = LoadLibraryW(PCWSTR(w.as_ptr())) {
            hmod = Some(h);
        }
    }
    let hmod = match hmod { Some(h) => h, None => return false };

    if MH_Initialize() != MH_OK {
        // Already initialized is ok
    }

    let p_connect = get_proc(hmod, "WinHttpConnect");
    let p_open    = get_proc(hmod, "WinHttpOpenRequest");
    let p_send    = get_proc(hmod, "WinHttpSendRequest");
    let p_recv    = get_proc(hmod, "WinHttpReceiveResponse");
    let p_qh      = get_proc(hmod, "WinHttpQueryHeaders");
    let p_qd      = get_proc(hmod, "WinHttpQueryDataAvailable");
    let p_rd      = get_proc(hmod, "WinHttpReadData");

    let mut tramp: *mut c_void = std::ptr::null_mut();

    if let Some(p) = p_connect {
        if hook_one(p, hk_connect as *mut c_void, &mut tramp) {
            ORIG_CONNECT = Some(std::mem::transmute::<_, FnConnect>(tramp));
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = p_open {
        if hook_one(p, hk_open_request as *mut c_void, &mut tramp) {
            ORIG_OPEN_REQUEST = Some(std::mem::transmute::<_, FnOpenRequest>(tramp));
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = p_send {
        if hook_one(p, hk_send_request as *mut c_void, &mut tramp) {
            ORIG_SEND_REQUEST = Some(std::mem::transmute::<_, FnSendRequest>(tramp));
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = p_recv {
        if hook_one(p, hk_receive_response as *mut c_void, &mut tramp) {
            ORIG_RECEIVE_RESPONSE = Some(std::mem::transmute::<_, FnReceiveResponse>(tramp));
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = p_qh {
        if hook_one(p, hk_query_headers as *mut c_void, &mut tramp) {
            ORIG_QUERY_HEADERS = Some(std::mem::transmute::<_, FnQueryHeaders>(tramp));
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = p_qd {
        if hook_one(p, hk_query_data_available as *mut c_void, &mut tramp) {
            ORIG_QUERY_DATA_AVAILABLE = Some(std::mem::transmute::<_, FnQueryDataAvailable>(tramp));
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = p_rd {
        if hook_one(p, hk_read_data as *mut c_void, &mut tramp) {
            ORIG_READ_DATA = Some(std::mem::transmute::<_, FnReadData>(tramp));
        }
    }

    true
}

// ---------- DllMain ----------

#[no_mangle]
pub extern "system" fn DllMain(_hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        log_line("==== DLL_PROCESS_ATTACH ====");
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            unsafe {
                let ok = install_hooks();
                log_line(&format!("install_hooks() => {}", ok));
                let _ = install_wininet_hooks();
                scan_loaded_modules();
            }
        });
    }
    WIN_TRUE.into()
}

// ---------- diagnostic: list loaded modules ----------
unsafe fn scan_loaded_modules() {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::ProcessStatus::{EnumProcessModules, GetModuleBaseNameW};
    use windows::Win32::System::Threading::GetCurrentProcess;

    let h = GetCurrentProcess();
    let mut mods: Vec<HMODULE> = vec![HMODULE::default(); 1024];
    let mut needed: u32 = 0;
    let bytes_size = (mods.len() * std::mem::size_of::<HMODULE>()) as u32;
    if EnumProcessModules(h, mods.as_mut_ptr(), bytes_size, &mut needed).is_ok() {
        let count = needed as usize / std::mem::size_of::<HMODULE>();
        log_line(&format!("---- {} modules loaded ----", count));
        for m in &mods[..count.min(mods.len())] {
            if m.0.is_null() { continue; }
            let mut buf = [0u16; 512];
            let n = GetModuleBaseNameW(h, *m, &mut buf);
            if n > 0 {
                let s = String::from_utf16_lossy(&buf[..n as usize]);
                let low = s.to_lowercase();
                if low.contains("curl") || low.contains("http") || low.contains("ssl")
                    || low.contains("unity") || low.contains("mono") || low.contains("net")
                {
                    log_line(&format!("  mod: {}", s));
                }
            }
        }
    }
}

// ---------- WinInet hooks (older HTTP API, used by some games) ----------

type FnInternetConnectW = unsafe extern "system" fn(HINTERNET, PCWSTR, u16, PCWSTR, PCWSTR, u32, usize, usize) -> HINTERNET;
type FnHttpOpenRequestW = unsafe extern "system" fn(HINTERNET, PCWSTR, PCWSTR, PCWSTR, PCWSTR, *const PCWSTR, u32, usize) -> HINTERNET;
type FnHttpSendRequestW = unsafe extern "system" fn(HINTERNET, PCWSTR, u32, *const c_void, u32) -> BOOL;
type FnInternetReadFile = unsafe extern "system" fn(HINTERNET, *mut c_void, u32, *mut u32) -> BOOL;

static mut ORIG_WI_CONNECT: Option<FnInternetConnectW> = None;
static mut ORIG_WI_OPEN: Option<FnHttpOpenRequestW> = None;
static mut ORIG_WI_SEND: Option<FnHttpSendRequestW> = None;
static mut ORIG_WI_READ: Option<FnInternetReadFile> = None;

unsafe extern "system" fn wi_connect(h: HINTERNET, server: PCWSTR, port: u16, user: PCWSTR, pass: PCWSTR, svc: u32, flags: usize, ctx: usize) -> HINTERNET {
    let s = pcwstr_to_string(server);
    log_line(&format!("InternetConnectW server='{}' port={}", s, port));
    let r = ORIG_WI_CONNECT.unwrap()(h, server, port, user, pass, svc, flags, ctx);
    if r.0 as usize != 0 && s.eq_ignore_ascii_case(TARGET_HOST) {
        log_line(&format!("  WININET -> TARGETED hConnect={:p}", r.0));
        mark_connect(r);
    }
    r
}

unsafe extern "system" fn wi_open(hc: HINTERNET, verb: PCWSTR, obj: PCWSTR, ver: PCWSTR, refr: PCWSTR, accept: *const PCWSTR, flags: u32, ctx: usize) -> HINTERNET {
    let r = ORIG_WI_OPEN.unwrap()(hc, verb, obj, ver, refr, accept, flags, ctx);
    if r.0 as usize != 0 && is_connect_targeted(hc) {
        log_line(&format!("WININET HttpOpenRequestW TARGETED hr={:p}", r.0));
        mark_request(r);
    }
    r
}

unsafe extern "system" fn wi_send(hr: HINTERNET, headers: PCWSTR, hlen: u32, opt: *const c_void, olen: u32) -> BOOL {
    if is_request_targeted(hr) {
        log_line(&format!("WININET HttpSendRequestW TARGETED hr={:p}", hr.0));
        return WIN_TRUE.into();
    }
    ORIG_WI_SEND.unwrap()(hr, headers, hlen, opt, olen)
}

unsafe extern "system" fn wi_read(hr: HINTERNET, buf: *mut c_void, blen: u32, read: *mut u32) -> BOOL {
    if is_request_targeted(hr) {
        let pos = current_read_pos(hr);
        let remaining = FAKE_BODY.len().saturating_sub(pos);
        let n = std::cmp::min(remaining, blen as usize);
        if n > 0 && !buf.is_null() {
            std::ptr::copy_nonoverlapping(FAKE_BODY.as_ptr().add(pos), buf as *mut u8, n);
        }
        if !read.is_null() { *read = n as u32; }
        advance_read(hr, n);
        log_line(&format!("WININET InternetReadFile TARGETED hr={:p} returned {}", hr.0, n));
        return WIN_TRUE.into();
    }
    ORIG_WI_READ.unwrap()(hr, buf, blen, read)
}

unsafe fn install_wininet_hooks() -> bool {
    let w: Vec<u16> = "wininet.dll".encode_utf16().chain(std::iter::once(0)).collect();
    let mut hmod = GetModuleHandleW(PCWSTR(w.as_ptr())).ok();
    if hmod.is_none() {
        if let Ok(h) = LoadLibraryW(PCWSTR(w.as_ptr())) {
            hmod = Some(h);
        }
    }
    let hmod = match hmod { Some(h) => h, None => { log_line("wininet.dll not available"); return false; } };

    let mut tramp: *mut c_void = std::ptr::null_mut();
    if let Some(p) = get_proc(hmod, "InternetConnectW") {
        if hook_one(p, wi_connect as *mut c_void, &mut tramp) {
            ORIG_WI_CONNECT = Some(std::mem::transmute::<_, FnInternetConnectW>(tramp));
            log_line("hooked WinInet InternetConnectW");
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = get_proc(hmod, "HttpOpenRequestW") {
        if hook_one(p, wi_open as *mut c_void, &mut tramp) {
            ORIG_WI_OPEN = Some(std::mem::transmute::<_, FnHttpOpenRequestW>(tramp));
            log_line("hooked WinInet HttpOpenRequestW");
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = get_proc(hmod, "HttpSendRequestW") {
        if hook_one(p, wi_send as *mut c_void, &mut tramp) {
            ORIG_WI_SEND = Some(std::mem::transmute::<_, FnHttpSendRequestW>(tramp));
            log_line("hooked WinInet HttpSendRequestW");
        }
    }
    tramp = std::ptr::null_mut();
    if let Some(p) = get_proc(hmod, "InternetReadFile") {
        if hook_one(p, wi_read as *mut c_void, &mut tramp) {
            ORIG_WI_READ = Some(std::mem::transmute::<_, FnInternetReadFile>(tramp));
            log_line("hooked WinInet InternetReadFile");
        }
    }
    true
}
