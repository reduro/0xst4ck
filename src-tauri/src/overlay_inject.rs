// Steam overlay injection.
// Spawns the game CREATE_SUSPENDED, injects gameoverlayrenderer64.dll (or 32-bit) from
// the local Steam install, then resumes the main thread. The overlay then
// hooks the game's DirectX / Vulkan renderer so Shift+Tab works immediately.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateProcessW, CreateRemoteThread, GetExitCodeProcess, ResumeThread, WaitForSingleObject,
    CREATE_SUSPENDED, LPTHREAD_START_ROUTINE, PROCESS_INFORMATION, STARTUPINFOW,
};

pub struct SpawnedProcess {
    pub pid: u32,
    pub h_process: HANDLE,
}

unsafe impl Send for SpawnedProcess {}
unsafe impl Sync for SpawnedProcess {}

impl SpawnedProcess {
    pub fn wait_with_code(self) -> i32 {
        unsafe {
            let _ = WaitForSingleObject(self.h_process, u32::MAX);
            let mut code: u32 = 0;
            let _ = GetExitCodeProcess(self.h_process, &mut code);
            let _ = CloseHandle(self.h_process);
            code as i32
        }
    }
}

pub fn is_pe_64bit(path: &Path) -> Option<bool> {
    let mut file = File::open(path).ok()?;
    let mut dos_header = [0u8; 64];
    file.read_exact(&mut dos_header).ok()?;
    if &dos_header[0..2] != b"MZ" {
        return None;
    }
    let e_lfanew = u32::from_le_bytes([
        dos_header[0x3C],
        dos_header[0x3D],
        dos_header[0x3E],
        dos_header[0x3F],
    ]) as u64;

    file.seek(SeekFrom::Start(e_lfanew)).ok()?;
    let mut pe_header = [0u8; 6];
    file.read_exact(&mut pe_header).ok()?;
    if &pe_header[0..4] != b"PE\0\0" {
        return None;
    }
    let machine = u16::from_le_bytes([pe_header[4], pe_header[5]]);
    // 0x8664 = IMAGE_FILE_MACHINE_AMD64
    Some(machine == 0x8664)
}

pub fn steam_overlay_dll(target_exe: &Path) -> Option<PathBuf> {
    let steam_dir = crate::loader::get_steam_dir();
    let is_64 = is_pe_64bit(target_exe).unwrap_or(true);
    let dll_name = if is_64 {
        "gameoverlayrenderer64.dll"
    } else {
        "gameoverlayrenderer.dll"
    };

    let p = steam_dir.join(dll_name);
    if p.exists() {
        return Some(p);
    }

    for base in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
    ] {
        let candidate = Path::new(base).join(dll_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Returns the path to our appid_hook.dll (patches SteamAPI_RestartAppIfNecessary
/// so the game can't bounce itself back to its real appid via Steam).
pub fn appid_hook_dll() -> Option<PathBuf> {
    let tools = crate::loader::get_tools_dir();
    let candidates = [
        tools.join("appid_hook").join("appid_hook.dll"),
        tools.join("appid_hook.dll"),
    ];
    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

pub fn launch_with_overlay(
    exe_path: &str,
    args: &str,
    working_dir: &str,
    env_vars: &[(&str, &str)],
) -> Result<SpawnedProcess, String> {
    let path = Path::new(exe_path);
    let cmdline = if args.trim().is_empty() {
        format!("\"{}\"", exe_path)
    } else {
        format!("\"{}\" {}", exe_path, args)
    };
    let mut cmdline_w: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
    let working_dir_w: Vec<u16> = working_dir.encode_utf16().chain(std::iter::once(0)).collect();

    // Build environment block (UTF-16, double-null-terminated)
    let mut env_map: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    for (k, v) in env_vars {
        env_map.insert((*k).to_string(), (*v).to_string());
    }
    let mut env_block: Vec<u16> = Vec::new();
    for (k, v) in &env_map {
        let entry = format!("{}={}", k, v);
        env_block.extend(entry.encode_utf16());
        env_block.push(0);
    }
    env_block.push(0);

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    const CREATE_UNICODE_ENVIRONMENT: u32 = 0x00000400;
    let flags = CREATE_SUSPENDED | windows::Win32::System::Threading::PROCESS_CREATION_FLAGS(CREATE_UNICODE_ENVIRONMENT);

    let create_result = unsafe {
        CreateProcessW(
            PCWSTR::null(),
            windows::core::PWSTR(cmdline_w.as_mut_ptr()),
            None,
            None,
            false,
            flags,
            Some(env_block.as_ptr() as *const std::ffi::c_void),
            PCWSTR(working_dir_w.as_ptr()),
            &si,
            &mut pi,
        )
    };
    if create_result.is_err() {
        return Err(format!("CreateProcessW: {:?}", create_result.err()));
    }

    // 1. Inject Steam Overlay DLL BEFORE resuming the process!
    // Must be injected while suspended so it hooks DirectX / Vulkan before the game creates its device/swapchain.
    if let Some(overlay) = steam_overlay_dll(path) {
        if let Err(e) = unsafe { inject_dll_path(pi.hProcess, &overlay) } {
            eprintln!("[overlay] gameoverlayrenderer injection: {}", e);
        }
    }

    // 2. Inject appid hook (locks AppID to 480 & disables app restarts)
    if let Some(hook) = appid_hook_dll() {
        let _ = unsafe { inject_dll_path(pi.hProcess, &hook) };
    }

    // 3. Resume the main thread so the game begins execution with all hooks in place!
    unsafe {
        ResumeThread(pi.hThread);
        let _ = CloseHandle(pi.hThread);
    }

    Ok(SpawnedProcess {
        pid: pi.dwProcessId,
        h_process: pi.hProcess,
    })
}

unsafe fn inject_dll_path(h_process: HANDLE, dll_path: &Path) -> Result<(), String> {
    unsafe {
        let wide_path: Vec<u16> = dll_path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let byte_len = wide_path.len() * 2;

        let remote_addr = VirtualAllocEx(
            h_process,
            None,
            byte_len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if remote_addr.is_null() {
            return Err("VirtualAllocEx returned null".into());
        }

        let mut written: usize = 0;
        let write_ok = WriteProcessMemory(
            h_process,
            remote_addr,
            wide_path.as_ptr() as *const std::ffi::c_void,
            byte_len,
            Some(&mut written),
        );
        if write_ok.is_err() {
            return Err("WriteProcessMemory failed".into());
        }

        let k32 = {
            let w: Vec<u16> = "kernel32.dll".encode_utf16().chain(std::iter::once(0)).collect();
            GetModuleHandleW(PCWSTR(w.as_ptr())).map_err(|e| format!("GetModuleHandleW: {}", e))?
        };
        let load_lib = {
            let name = b"LoadLibraryW\0";
            GetProcAddress(k32, PCSTR(name.as_ptr())).ok_or("LoadLibraryW not found")?
        };

        let start_routine: LPTHREAD_START_ROUTINE = Some(std::mem::transmute(load_lib));

        let h_thread = CreateRemoteThread(
            h_process,
            None,
            0,
            start_routine,
            Some(remote_addr),
            0,
            None,
        )
        .map_err(|e| format!("CreateRemoteThread: {}", e))?;

        let wait_res = WaitForSingleObject(h_thread, 2_000);
        let _ = CloseHandle(h_thread);

        if wait_res != WAIT_OBJECT_0 {
            return Err(format!("remote LoadLibraryW wait: {:?}", wait_res));
        }

        Ok(())
    }
}

pub fn spawn_wait_thread(
    proc: SpawnedProcess,
    game_id: String,
    game_name: String,
    exe_path: String,
    launch_mode: String,
) {
    crate::discord_rpc::set_in_game(&game_name);
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let exit_code = proc.wait_with_code();
        crate::discord_rpc::set_idle();
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed >= 5.0 {
            let mut games = crate::storage::load_games();
            if let Some(g) = games.iter_mut().find(|g| g.id == game_id) {
                g.playtime += elapsed as u64;
                g.last_played = chrono::Utc::now().timestamp() as u64;
                let _ = crate::storage::save_games(&games);
            }
        } else {
            crate::crash_handler::evaluate_crash(
                &game_id,
                &game_name,
                &exe_path,
                &launch_mode,
                exit_code,
                elapsed,
            );
        }
    });
}
