use crate::models::{Game, GamesContainer, Settings};
use std::fs;
use std::path::PathBuf;

pub fn get_appdata_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("clemtoutlauncher");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

pub fn get_banners_dir() -> PathBuf {
    let dir = get_appdata_dir().join("banners");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

pub fn get_games_path() -> PathBuf {
    get_appdata_dir().join("games.json")
}

pub fn get_settings_path() -> PathBuf {
    get_appdata_dir().join("settings.json")
}

pub fn load_games() -> Vec<Game> {
    let path = get_games_path();
    if !path.exists() {
        return Vec::new();
    }

    let mut raw_games = match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<GamesContainer>(&content) {
            Ok(container) => container.games,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let mut seen_appids = std::collections::HashSet::new();
    let mut seen_paths = std::collections::HashSet::new();
    let mut deduplicated = Vec::new();
    let mut had_duplicates = false;

    for game in raw_games.drain(..) {
        let norm_path = game.path.replace('/', "\\").to_lowercase();
        let appid_opt = game.steam_appid.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        if let Some(ref aid) = appid_opt {
            if aid == "228980" || aid == "1070560" || aid == "250820" || aid == "2250500" || aid == "4704690" {
                had_duplicates = true;
                continue;
            }
        }
        let lower_name = game.name.to_lowercase();
        if lower_name.contains("redistributable") || lower_name.contains("steamworks common") {
            had_duplicates = true;
            continue;
        }

        let mut is_dup = false;
        if let Some(ref aid) = appid_opt {
            if !seen_appids.insert(aid.clone()) {
                is_dup = true;
            }
        }
        if !norm_path.is_empty() && !seen_paths.insert(norm_path) {
            is_dup = true;
        }

        let mut final_game = game;
        let p_lower = final_game.path.to_lowercase();
        if p_lower.contains("streamingassets") || p_lower.contains("crashpad") || p_lower.contains("distribution") {
            let p_buf = PathBuf::from(&final_game.path);
            if let Some(parent) = p_buf.parent() {
                let mut dir = parent;
                while let Some(up) = dir.parent() {
                    if up.file_name().map(|n| n.to_string_lossy().to_lowercase() == "common").unwrap_or(false) {
                        break;
                    }
                    dir = up;
                }
                if let Some(better_exe) = crate::steam_importer::find_game_executable(dir, &final_game.name) {
                    final_game.path = better_exe.to_string_lossy().to_string();
                    had_duplicates = true;
                }
            }
        }

        if let Some(ref mut rt) = final_game.network_runtime {
            if !rt.photon && !rt.eos {
                rt.runtime = "spacewar".to_string();
                rt.requirements_summary = "Steam Spacewar (AppID 480)".to_string();
            }
        }

        if final_game.network_runtime.is_none() || !final_game.network_runtime.as_ref().map(|r| r.scanned).unwrap_or(false) {
            let rt = crate::scanner::scan_game_network_runtime(&final_game.path);
            final_game.requires_photon = rt.photon;
            final_game.network_runtime = Some(rt);
            had_duplicates = true;
        }

        if is_dup {
            had_duplicates = true;
        } else {
            deduplicated.push(final_game);
        }
    }

    if deduplicated.iter().any(|g| g.playtime == 0 && g.steam_appid.is_some()) {
        let steam_path = crate::steam_importer::detect_steam_install_path();
        let steam_playtimes = crate::steam_importer::load_steam_user_playtimes(&steam_path);
        for g in deduplicated.iter_mut() {
            if let Some(ref aid) = g.steam_appid {
                if let Some(&(pt, lp)) = steam_playtimes.get(aid) {
                    if pt > g.playtime {
                        g.playtime = pt;
                        had_duplicates = true;
                    }
                    if lp > g.last_played {
                        g.last_played = lp;
                        had_duplicates = true;
                    }
                }
            }
        }
    }

    if had_duplicates {
        let _ = save_games(&deduplicated);
    }

    deduplicated
}

pub fn save_games(games: &[Game]) -> Result<(), String> {
    let path = get_games_path();
    let container = GamesContainer {
        games: games.to_vec(),
    };
    let json_data = serde_json::to_string_pretty(&container)
        .map_err(|e| format!("Erreur sérialisation games: {}", e))?;
    fs::write(path, json_data).map_err(|e| format!("Erreur écriture games.json: {}", e))?;
    Ok(())
}

pub fn load_settings() -> Settings {
    let path = get_settings_path();
    if !path.exists() {
        let s = Settings::default();
        let _ = save_settings(&s);
        return s;
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<Settings>(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = get_settings_path();
    let json_data = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Erreur sérialisation settings: {}", e))?;
    fs::write(path, json_data).map_err(|e| format!("Erreur écriture settings.json: {}", e))?;
    Ok(())
}
