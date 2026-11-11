// Steam overlay injection.
// Spawns the game CREATE_SUSPENDED, injects gameoverlayrenderer64.dll from
// the local Steam install, then resumes the main thread. The overlay then
// hooks the game's renderer regardless of how Steam detected the process.

use std::ffi::CString;
use std::path::{Path, PathBuf};
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateProcessW, CreateRemoteThread, ResumeThread, WaitForSingleObject,
    CREATE_SUSPENDED, LPTHREAD_START_ROUTINE, PROCESS_INFORMATION, STARTUPINFOW,
};

pub struct SpawnedProcess {
    pub pid: u32,
    pub h_process: HANDLE,
}

unsafe impl Send for SpawnedProcess {}
unsafe impl Sync for SpawnedProcess {}

impl SpawnedProcess {
    pub fn wait(self) {
        unsafe {
            let _ = WaitForSingleObject(self.h_process, u32::MAX);
            let _ = CloseHandle(self.h_process);
        }
    }
}

pub fn steam_overlay_dll() -> Option<PathBuf> {
    for base in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
    ] {
        let p = Path::new(base).join("gameoverlayrenderer64.dll");
        if p.exists() {
            return Some(p);
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
    overlay_dll: &Path,
) -> Result<SpawnedProcess, String> {
    let overlay_str = overlay_dll.to_string_lossy().into_owned();

    let cmdline = if args.trim().is_empty() {
        format!("\"{}\"", exe_path)
    } else {
        format!("\"{}\" {}", exe_path, args)
    };
    let mut cmdline_w: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
    let working_dir_w: Vec<u16> = working_dir.encode_utf16().chain(std::iter::once(0)).collect();

    // Build environment block (UTF-16, double-null-terminated). We inherit the
    // parent env and override/add our vars.
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

    // Inject overlay DLL — failure here is non-fatal, we still want the game to run
    let inject_res = unsafe { inject_dll(pi.hProcess, &overlay_str) };

    // Inject appid hook (neutralizes SteamAPI_RestartAppIfNecessary). Must happen
    // BEFORE ResumeThread so the hook arms before the game calls into Steamworks.
    if let Some(hook) = appid_hook_dll() {
        let hook_str = hook.to_string_lossy().into_owned();
        if let Err(e) = unsafe { inject_dll(pi.hProcess, &hook_str) } {
            eprintln!("[appid_hook] injection failed ({}) — game may still restart as its real appid", e);
        }
    } else {
        eprintln!("[appid_hook] appid_hook.dll not found in tools/ — Steam may show real appid");
    }

    unsafe {
        ResumeThread(pi.hThread);
        let _ = CloseHandle(pi.hThread);
    }

    if let Err(e) = inject_res {
        eprintln!("[overlay] injection failed ({}) — game continues without overlay", e);
    }

    Ok(SpawnedProcess {
        pid: pi.dwProcessId,
        h_process: pi.hProcess,
    })
}

unsafe fn inject_dll(h_process: HANDLE, dll_path: &str) -> Result<(), String> {
    unsafe {
        let mut path_bytes: Vec<u8> = dll_path.as_bytes().to_vec();
        path_bytes.push(0);

        let remote_addr = VirtualAllocEx(
            h_process,
            None,
            path_bytes.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if remote_addr.is_null() {
            return Err("VirtualAllocEx returned null".into());
        }

        let mut written: usize = 0;
        WriteProcessMemory(
            h_process,
            remote_addr,
            path_bytes.as_ptr() as *const std::ffi::c_void,
            path_bytes.len(),
            Some(&mut written),
        )
        .map_err(|e| format!("WriteProcessMemory: {}", e))?;

        let k32 = {
            let w: Vec<u16> = "kernel32.dll".encode_utf16().chain(std::iter::once(0)).collect();
            GetModuleHandleW(PCWSTR(w.as_ptr())).map_err(|e| format!("GetModuleHandleW: {}", e))?
        };
        let load_lib = {
            let name = b"LoadLibraryA\0";
            GetProcAddress(k32, PCSTR(name.as_ptr())).ok_or("LoadLibraryA not found")?
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

        let wait_res = WaitForSingleObject(h_thread, 10_000);
        let _ = CloseHandle(h_thread);

        if wait_res != WAIT_OBJECT_0 {
            return Err(format!("remote LoadLibrary wait: {:?}", wait_res));
        }

        let _ = CString::new("").unwrap(); // force CString link
        Ok(())
    }
}

pub fn spawn_wait_thread(proc: SpawnedProcess, game_id: String) {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        proc.wait();
        let elapsed = start.elapsed().as_secs();
        if elapsed >= 5 {
            let mut games = crate::storage::load_games();
            if let Some(g) = games.iter_mut().find(|g| g.id == game_id) {
                g.playtime += elapsed;
                g.last_played = chrono::Utc::now().timestamp() as u64;
                let _ = crate::storage::save_games(&games);
            }
        }
    });
}
