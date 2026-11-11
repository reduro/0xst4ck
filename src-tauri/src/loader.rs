use crate::scanner::scan_game_network_runtime;
use crate::storage::{load_games, load_settings, save_games};
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn ensure_onlinefix_dll() -> PathBuf {
    crate::dll_manager::ensure_onlinefix_dll()
}

pub fn get_tools_dir() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        let candidate = exe.parent().unwrap_or(Path::new("")).join("tools");
        if candidate.exists() {
            return candidate;
        }
    }
    let dev_tools = PathBuf::from("src-tauri").join("tools");
    if dev_tools.exists() {
        return dev_tools;
    }
    PathBuf::from("tools")
}

pub fn get_steam_dir() -> PathBuf {
    let default_path = PathBuf::from(r"C:\Program Files (x86)\Steam");
    if default_path.exists() {
        return default_path;
    }
    PathBuf::from(r"C:\Program Files\Steam")
}

pub fn get_photon_loader_dir() -> PathBuf {
    let tools_dir = get_tools_dir();
    let bundled = tools_dir.join("photon");
    if bundled.join("Loader.exe").exists() {
        return bundled;
    }
    let fallback = PathBuf::from(r"E:\photon\Loader");
    if fallback.join("Loader.exe").exists() {
        return fallback;
    }
    bundled
}

pub fn unpack_if_needed(target_exe: &Path) -> bool {
    let tools_dir = get_tools_dir();
    let unpacker = tools_dir.join("Unpacker").join("ClemtoutLauncher.CLI.exe");

    if !unpacker.exists() {
        return false;
    }

    let original_path = target_exe.to_path_buf();
    let backup_path = PathBuf::from(format!("{}.pack", original_path.display()));

    let unpacked_v1 = original_path.with_extension("exe.unpacked.exe");
    let stem = original_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let unpacked_v2 = original_path.with_file_name(format!("{}.unpacked.exe", stem));

    let mut unpacked_found = if unpacked_v1.exists() {
        Some(unpacked_v1.clone())
    } else if unpacked_v2.exists() {
        Some(unpacked_v2.clone())
    } else {
        None
    };

    if unpacked_found.is_none() {
        let _ = Command::new(&unpacker)
            .arg("--quiet")
            .arg(&original_path)
            .current_dir(unpacker.parent().unwrap_or(Path::new(".")))
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if unpacked_v1.exists() {
            unpacked_found = Some(unpacked_v1);
        } else if unpacked_v2.exists() {
            unpacked_found = Some(unpacked_v2);
        }
    }

    if let Some(unpacked) = unpacked_found {
        if backup_path.exists() {
            let _ = fs::remove_file(&backup_path);
        }
        let _ = fs::rename(&original_path, &backup_path);
        let _ = fs::rename(&unpacked, &original_path);
        return true;
    }

    false
}

pub fn launch_game_dynamic(game_id: &str, mode: &str) -> Result<String, String> {
    let mut games = load_games();
    let game_index = games
        .iter()
        .position(|g| g.id == game_id)
        .ok_or_else(|| "Jeu introuvable".to_string())?;

    let game_path_str = games[game_index].path.clone();
    let game_args_str = games[game_index].arguments.clone();
    let game_name_str = games[game_index].name.clone();
    let path = Path::new(&game_path_str);

    if !path.exists() {
        return Err(format!("L'exécutable n'existe pas : {}", game_path_str));
    }

    let working_dir = path
        .parent()
        .ok_or_else(|| "Dossier du jeu invalide".to_string())?;

    // USE CACHED RUNTIME OR SCAN ONCE:
    let runtime = if let Some(ref rt) = games[game_index].network_runtime {
        rt.clone()
    } else {
        let rt = scan_game_network_runtime(&game_path_str);
        games[game_index].network_runtime = Some(rt.clone());
        games[game_index].requires_photon = rt.photon;
        let _ = save_games(&games);
        rt
    };
    let requires_photon = runtime.photon;

    // 1. Direct mode:
    if mode == "direct" {
        let mut cmd = Command::new(path);
        cmd.current_dir(working_dir);
        if !game_args_str.trim().is_empty() {
            for a in game_args_str.split_whitespace() {
                cmd.arg(a);
            }
        }
        let child = cmd.spawn().map_err(|e| format!("Erreur direct: {}", e))?;
        track_playtime_async(child, game_id.to_string());
        return Ok("Lancé en mode Direct".to_string());
    }

    // 2. Steamless Unpacking check:
    let settings = load_settings();
    if settings.auto_unpack {
        unpack_if_needed(path);
    }

    // 3. Photon Loader check:
    let photon_dir = get_photon_loader_dir();
    let photon_loader_exe = photon_dir.join("Loader.exe");

    if (requires_photon || mode == "photon") && photon_loader_exe.exists() {
        let mut cmd = Command::new(&photon_loader_exe);
        cmd.current_dir(&photon_dir)
            .arg(&game_path_str)
            .arg("--overlay")
            .env("SteamAppId", "480")
            .env("SteamGameId", "480")
            .env("SteamOverlayGameId", "480")
            .env("SteamNoAppIdCheck", "1")
            .env("SteamEnv", "1")
            .env("SteamClientLaunch", "1");

        let child = cmd
            .spawn()
            .map_err(|e| format!("Erreur Photon Loader: {}", e))?;

        track_playtime_async(child, game_id.to_string());
        return Ok("Lancé avec succès via le Relais Photon P2P".to_string());
    }

    // 4. Clean Launch with Steamworks / Spacewar 480 Emulation Environment:
    let appid = "480";

    // Guarantee steam_appid.txt is set to 480 in working directory for normal Spacewar launch
    let txt_path = working_dir.join("steam_appid.txt");
    let _ = fs::write(&txt_path, "480");

    let mut cmd = Command::new(&game_path_str);
    cmd.current_dir(&working_dir);
    if !game_args_str.trim().is_empty() {
        cmd.args(game_args_str.split_whitespace());
    }

    cmd.env("SteamAppId", "480")
        .env("SteamGameId", "480")
        .env("SteamOverlayGameId", "480")
        .env("SteamNoAppIdCheck", "1")
        .env("SteamEnv", "1")
        .env("SteamClientLaunch", "1")
        .env("ENABLE_STEAM_OVERLAY", "1");

    // Clean up any rogue OnlineFix.dll left in a Photon game directory
    if requires_photon || mode == "photon" {
        let rogue_of = working_dir.join("OnlineFix.dll");
        if rogue_of.exists() {
            let _ = fs::remove_file(&rogue_of);
        }
    }

    // If EOS is required, copy OnlineFix.dll to game directory if not present (NEVER for Photon games)
    if (runtime.eos || mode == "eos") && !requires_photon && mode != "photon" {
        let of_src = ensure_onlinefix_dll();
        if of_src.exists() {
            let of_dst = working_dir.join("OnlineFix.dll");
            if !of_dst.exists() {
                let _ = fs::copy(&of_src, &of_dst);
            }
        }
    }

    // If Steam overlay DLL is available, spawn suspended + inject it so Shift+Tab
    // works even when the game is launched outside Steam's lifecycle.
    if let Some(overlay_dll) = crate::overlay_inject::steam_overlay_dll() {
        let wd = working_dir.to_string_lossy().into_owned();
        let env_vars: Vec<(&str, &str)> = vec![
            ("SteamAppId", "480"),
            ("SteamGameId", "480"),
            ("SteamOverlayGameId", "480"),
            ("SteamNoAppIdCheck", "1"),
            ("SteamEnv", "1"),
            ("SteamClientLaunch", "1"),
            ("ENABLE_STEAM_OVERLAY", "1"),
        ];
        match crate::overlay_inject::launch_with_overlay(
            &game_path_str,
            &game_args_str,
            &wd,
            &env_vars,
            &overlay_dll,
        ) {
            Ok(proc) => {
                crate::overlay_inject::spawn_wait_thread(proc, game_id.to_string());
                return Ok(format!("Lancé avec Steam overlay injecté (AppID {})", appid));
            }
            Err(e) => {
                eprintln!("[overlay] {}, fallback to plain spawn", e);
            }
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Impossible de démarrer le jeu : {}", e))?;

    track_playtime_async(child, game_id.to_string());
    Ok(format!("Lancé avec succès (Spacewar AppID {})", appid))
}

fn track_playtime_async(mut child: Child, game_id: String) {
    thread::spawn(move || {
        let start = std::time::Instant::now();
        let _ = child.wait();
        let elapsed_secs = start.elapsed().as_secs();

        if elapsed_secs >= 5 {
            let mut games = load_games();
            if let Some(target) = games.iter_mut().find(|g| g.id == game_id) {
                target.playtime += elapsed_secs;
                target.last_played = chrono::Utc::now().timestamp() as u64;
                let _ = save_games(&games);
            }
        }
    });
}
