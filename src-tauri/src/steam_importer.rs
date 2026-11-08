use crate::models::Game;
use crate::scanner::scan_game_network_runtime;
use crate::steam::cache_steam_banner;
use crate::storage::{load_games, load_settings, save_games};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;
use walkdir::WalkDir;

const EXE_BLACKLIST: &[&str] = &[
    "unitycrashhandler",
    "crashreport",
    "crashpad",
    "setup",
    "install",
    "uninstall",
    "vcredist",
    "python",
    "steamcmd",
    "clemtoutlauncher",
    "game_loader",
    "helper",
    "overlay",
    "config",
    "touch",
    "dxsetup",
    "easyanticheat",
    "epicgameslauncher",
    "linkshandler",
    "cefprocess",
    "unrealcefsubprocess",
    "prerequisites",
    "redist",
    "physx",
    "eac_setup",
    "dotNetFx",
    "clean.exe",
];

const APPID_BLACKLIST: &[&str] = &[
    "228980",  // Steamworks Common Redistributables
    "1070560", // Steam Linux Runtime
    "1391110", // Steam Linux Runtime - Soldier
    "1628350", // Steam Linux Runtime - Sniper
    "250820",  // SteamVR
];

pub fn detect_steam_install_path() -> PathBuf {
    let settings = load_settings();
    if let Some(ref p) = settings.steam_path {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return pb;
        }
    }

    // Try Windows Registry HKCU\Software\Valve\Steam
    if let Ok(output) = Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("SteamPath") {
                    if let Some(idx) = line.find("REG_SZ") {
                        let path_part = line[idx + 6..].trim();
                        let clean_path = path_part.replace('/', "\\");
                        let pb = PathBuf::from(clean_path);
                        if pb.exists() {
                            return pb;
                        }
                    }
                }
            }
        }
    }

    let default_x86 = PathBuf::from(r"C:\Program Files (x86)\Steam");
    if default_x86.exists() {
        return default_x86;
    }
    PathBuf::from(r"C:\Program Files\Steam")
}

pub fn get_steam_libraries(main_steam_path: &Path) -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    if main_steam_path.exists() {
        libraries.push(main_steam_path.to_path_buf());
    }

    // Parse libraryfolders.vdf
    let vdf_path = main_steam_path.join("steamapps").join("libraryfolders.vdf");
    if vdf_path.exists() {
        if let Ok(content) = fs::read_to_string(&vdf_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"path\"") {
                    let parts: Vec<&str> = trimmed.split('"').filter(|s| !s.trim().is_empty()).collect();
                    if parts.len() >= 2 {
                        let raw_path = parts[1].replace("\\\\", "\\");
                        let pb = PathBuf::from(raw_path);
                        if pb.exists() && !libraries.contains(&pb) {
                            libraries.push(pb);
                        }
                    }
                }
            }
        }
    }

    // Also scan common folders on available drives
    let common_folders = [
        r"Program Files (x86)\Steam",
        r"Program Files\Steam",
        r"Steam",
        r"SteamLibrary",
        r"Games\Steam",
        r"Jeux\Steam",
    ];

    for drive_letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", drive_letter as char);
        let drive_path = Path::new(&drive);
        if drive_path.exists() {
            for folder in common_folders {
                let candidate = drive_path.join(folder);
                if candidate.join("steamapps").exists() && !libraries.contains(&candidate) {
                    libraries.push(candidate);
                }
            }
        }
    }

    libraries
}

pub fn load_steam_user_playtimes(steam_path: &Path) -> std::collections::HashMap<String, (u64, u64)> {
    let mut map = std::collections::HashMap::new();
    let userdata = steam_path.join("userdata");
    if !userdata.exists() {
        return map;
    }

    let entries = match fs::read_dir(&userdata) {
        Ok(e) => e,
        Err(_) => return map,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        if !entry.path().is_dir() {
            continue;
        }

        let localconfig_path = entry.path().join("config").join("localconfig.vdf");
        if !localconfig_path.exists() {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&localconfig_path) {
            let mut current_appid: Option<String> = None;
            let mut current_playtime: u64 = 0;
            let mut current_last_played: u64 = 0;
            let mut in_app_block = false;

            for line in content.lines() {
                let trimmed = line.trim();

                if !in_app_block {
                    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() > 2 {
                        let inner = &trimmed[1..trimmed.len() - 1];
                        if inner.chars().all(|c| c.is_ascii_digit()) {
                            current_appid = Some(inner.to_string());
                            current_playtime = 0;
                            current_last_played = 0;
                        }
                    } else if trimmed == "{" && current_appid.is_some() {
                        in_app_block = true;
                    }
                } else {
                    if trimmed == "}" {
                        if let Some(appid) = current_appid.take() {
                            if current_playtime > 0 || current_last_played > 0 {
                                let item = map.entry(appid).or_insert((0, 0));
                                if current_playtime > item.0 {
                                    item.0 = current_playtime;
                                }
                                if current_last_played > item.1 {
                                    item.1 = current_last_played;
                                }
                            }
                        }
                        in_app_block = false;
                    } else if trimmed.starts_with("\"Playtime\"")
                        || trimmed.starts_with("\"PlaytimeTotal\"")
                        || trimmed.starts_with("\"playtime\"")
                    {
                        let val = extract_vdf_val(trimmed);
                        if let Ok(mins) = val.parse::<u64>() {
                            current_playtime = mins * 60; // Convert minutes to seconds
                        }
                    } else if trimmed.starts_with("\"LastPlayed\"") || trimmed.starts_with("\"lastplayed\"") {
                        let val = extract_vdf_val(trimmed);
                        if let Ok(ts) = val.parse::<u64>() {
                            current_last_played = ts;
                        }
                    }
                }
            }
        }
    }

    map
}

fn score_candidate(path: &Path, install_dir: &Path, game_name: &str) -> i64 {
    let mut score = 0i64;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
    let clean_stem: String = stem.chars().filter(|c| c.is_alphanumeric()).collect();
    let clean_game_name: String = game_name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    let dir_name = install_dir.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let clean_dir_name: String = dir_name.chars().filter(|c| c.is_alphanumeric()).collect();

    let relative = path.strip_prefix(install_dir).unwrap_or(path);
    let depth = relative.components().count();
    if depth == 1 {
        score += 220; // Root executable strongly preferred
    } else if depth <= 3 {
        let parent_name = path.parent().and_then(|p| p.file_name()).unwrap_or_default().to_string_lossy().to_lowercase();
        if parent_name == "bin" || parent_name == "win64" || parent_name == "x64" || parent_name == "binaries" {
            score += 160;
        } else {
            score += 40;
        }
    } else {
        score -= (depth as i64) * 30;
    }

    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("streamingassets")
        || path_str.contains("crashpad")
        || path_str.contains("distribution")
        || path_str.contains("thirdparty")
        || path_str.contains("installer")
        || path_str.contains("support")
    {
        score -= 600;
    }

    if clean_stem == clean_game_name || clean_stem == clean_dir_name {
        score += 300;
    } else if clean_stem.starts_with(&clean_game_name) || clean_stem.starts_with(&clean_dir_name) {
        score += 200;
    } else if clean_stem.contains(&clean_game_name) || clean_game_name.contains(&clean_stem) {
        score += 120;
    }

    if file_name.contains("shipping") {
        score += 90;
    }

    if file_name.contains("64") {
        score += 25;
    }

    if let Ok(meta) = fs::metadata(path) {
        let size_mb = (meta.len() / (1024 * 1024)) as i64;
        score += size_mb.min(50);
    }

    score
}

pub fn find_game_executable(install_dir: &Path, game_name: &str) -> Option<PathBuf> {
    if !install_dir.exists() {
        return None;
    }

    let mut scored_candidates: Vec<(i64, PathBuf)> = Vec::new();

    for entry in WalkDir::new(install_dir)
        .max_depth(7)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "exe" {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    let is_blacklisted = EXE_BLACKLIST.iter().any(|bad| file_name.contains(bad));
                    if !is_blacklisted {
                        let score = score_candidate(path, install_dir, game_name);
                        scored_candidates.push((score, path.to_path_buf()));
                    }
                }
            }
        }
    }

    scored_candidates.sort_by_key(|(s, _)| *s);
    scored_candidates.last().map(|(_, p)| p.clone())
}

pub async fn import_steam_library_games() -> Result<usize, String> {
    let mut games = load_games();
    let mut existing_appids: std::collections::HashSet<String> = games
        .iter()
        .filter_map(|g| g.steam_appid.clone())
        .collect();
    let mut existing_paths: std::collections::HashSet<String> = games
        .iter()
        .map(|g| g.path.replace('/', "\\").to_lowercase())
        .collect();

    let main_steam = detect_steam_install_path();
    let steam_playtimes = load_steam_user_playtimes(&main_steam);
    let mut updated_existing = false;

    // Refresh playtime and last_played for already installed games (fast lookup, no disk scan)
    for g in games.iter_mut() {
        if let Some(ref aid) = g.steam_appid {
            if let Some(&(pt, lp)) = steam_playtimes.get(aid) {
                if pt > g.playtime {
                    g.playtime = pt;
                    updated_existing = true;
                }
                if lp > g.last_played {
                    g.last_played = lp;
                    updated_existing = true;
                }
            }
        }
    }

    let libraries = get_steam_libraries(&main_steam);
    let mut imported_count = 0;

    for lib in libraries {
        let steamapps = lib.join("steamapps");
        if !steamapps.exists() {
            continue;
        }

        let entries = match fs::read_dir(&steamapps) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with("appmanifest_") && filename.ends_with(".acf") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let mut name = String::new();
                    let mut appid = String::new();
                    let mut installdir = String::new();
                    let mut last_played: u64 = 0;

                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("\"name\"") {
                            name = extract_vdf_val(trimmed);
                        } else if trimmed.starts_with("\"appid\"") {
                            appid = extract_vdf_val(trimmed);
                        } else if trimmed.starts_with("\"installdir\"") {
                            installdir = extract_vdf_val(trimmed);
                        } else if trimmed.starts_with("\"LastPlayed\"") {
                            let raw_time = extract_vdf_val(trimmed);
                            last_played = raw_time.parse().unwrap_or(0);
                        }
                    }

                    if appid.is_empty() || name.is_empty() {
                        continue;
                    }

                    if APPID_BLACKLIST.contains(&appid.as_str()) {
                        continue;
                    }

                    let lower_name = name.to_lowercase();
                    if lower_name.contains("redistributable")
                        || lower_name.contains("dedicated server")
                        || lower_name.contains("controller config")
                        || lower_name.contains("steamworks common")
                    {
                        continue;
                    }

                    let full_install_path = steamapps.join("common").join(&installdir);
                    if let Some(exe_path) = find_game_executable(&full_install_path, &name) {
                        let exe_str = exe_path.to_string_lossy().to_string();
                        let norm_exe = exe_str.replace('/', "\\").to_lowercase();

                        if existing_appids.contains(&appid) || existing_paths.contains(&norm_exe) {
                            continue;
                        }

                        existing_appids.insert(appid.clone());
                        existing_paths.insert(norm_exe);

                        let runtime = scan_game_network_runtime(&exe_str);
                        let requires_photon = runtime.photon;

                        // Download banner
                        let capsule_url = format!(
                            "https://cdn.akamai.steamstatic.com/steam/apps/{}/library_600x900.jpg",
                            appid
                        );
                        let banner = cache_steam_banner(&appid, &capsule_url).await.ok();

                        let (vdf_playtime, vdf_last_played) = steam_playtimes.get(&appid).copied().unwrap_or((0, 0));
                        let final_last_played = if last_played > 0 { last_played } else { vdf_last_played };

                        let game = Game {
                            id: Uuid::new_v4().to_string(),
                            name,
                            path: exe_str,
                            arguments: String::new(),
                            steam_appid: Some(appid.clone()),
                            banner,
                            playtime: vdf_playtime,
                            requires_photon,
                            last_played: final_last_played,
                            network_runtime: Some(runtime),
                        };

                        games.push(game);
                        imported_count += 1;
                    }
                }
            }
        }
    }

    if imported_count > 0 || updated_existing {
        save_games(&games)?;
    }

    Ok(imported_count)
}

fn extract_vdf_val(line: &str) -> String {
    let parts: Vec<&str> = line.split('"').collect();
    if parts.len() >= 4 {
        parts[3].to_string()
    } else {
        String::new()
    }
}

fn solve_pow(challenge: &str, difficulty: usize) -> Option<String> {
    let target = "0".repeat(difficulty);
    for nonce in 0..20_000_000u64 {
        let input = format!("{}:{}", challenge, nonce);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash_bytes = hasher.finalize();
        let hex = format!("{:x}", hash_bytes);
        if hex.starts_with(&target) {
            return Some(nonce.to_string());
        }
    }
    None
}

async fn fetch_fallback_lua(appid: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Challenge
    let challenge_payload = serde_json::json!({ "action": "challenge" });
    let resp_chal = client.post("https://walftech.com/gate.php")
        .header("accept", "*/*")
        .header("content-type", "application/json")
        .header("origin", "https://walftech.com")
        .header("referer", "https://walftech.com/generator.html")
        .json(&challenge_payload)
        .send()
        .await
        .map_err(|e| format!("Gate challenge network error: {}", e))?;

    if !resp_chal.status().is_success() {
        return Err(format!("Gate challenge failed with HTTP {}", resp_chal.status()));
    }

    let chal_json: serde_json::Value = resp_chal.json().await.map_err(|e| format!("Gate challenge invalid JSON: {}", e))?;
    let challenge = chal_json.get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Challenge field missing in gate response".to_string())?;
    let difficulty = chal_json.get("difficulty")
        .and_then(|v| v.as_u64())
        .unwrap_or(4) as usize;

    // 2. Solve PoW
    let nonce = solve_pow(challenge, difficulty)
        .ok_or_else(|| "Failed to solve Proof-of-Work challenge within threshold".to_string())?;

    // 3. Redeem token
    let redeem_payload = serde_json::json!({
        "action": "redeem",
        "id": appid,
        "challenge": challenge,
        "nonce": nonce
    });

    let resp_redeem = client.post("https://walftech.com/gate.php")
        .header("accept", "*/*")
        .header("content-type", "application/json")
        .header("origin", "https://walftech.com")
        .header("referer", "https://walftech.com/generator.html")
        .json(&redeem_payload)
        .send()
        .await
        .map_err(|e| format!("Gate redeem network error: {}", e))?;

    if !resp_redeem.status().is_success() {
        return Err(format!("Gate redeem failed with HTTP {}", resp_redeem.status()));
    }

    let redeem_json: serde_json::Value = resp_redeem.json().await.map_err(|e| format!("Gate redeem invalid JSON: {}", e))?;
    let token = redeem_json.get("token")
        .or_else(|| redeem_json.get("result"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "No token returned from gate redeem".to_string())?;

    // 4. Download payload
    let download_url = format!("https://walftech.com/depotbox_lua.php?id={}&token={}&format=lua", appid, token);
    let resp_file = client.get(&download_url)
        .header("referer", "https://walftech.com/generator.html")
        .send()
        .await
        .map_err(|e| format!("Depotbox download error: {}", e))?;

    if !resp_file.status().is_success() {
        return Err(format!("Depotbox returned HTTP {}", resp_file.status()));
    }

    let bytes = resp_file.bytes().await.map_err(|e| e.to_string())?.to_vec();
    if bytes.is_empty() {
        return Err("Depotbox returned empty payload".to_string());
    }

    Ok(bytes)
}

fn save_lua_payload(bytes: &[u8], appid: &str, steam_path: &Path) -> Result<String, String> {
    let config_dir = steam_path.join("config");
    let lua_dir = config_dir.join("lua");
    let stplugin_dir = config_dir.join("stplug-in");
    let _ = fs::create_dir_all(&lua_dir);
    let _ = fs::create_dir_all(&stplugin_dir);

    if bytes.starts_with(b"PK") {
        let cursor = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to read Lua zip: {}", e))?;
        let mut count = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = file.name().to_string();
            if name.ends_with(".lua") {
                let mut content = String::new();
                let _ = file.read_to_string(&mut content);

                let clean_lines: Vec<&str> = content
                    .lines()
                    .filter(|l| !l.trim().to_lowercase().starts_with("setmanifest"))
                    .collect();
                let clean_content = clean_lines.join("\n");

                let file_basename = Path::new(&name).file_name().unwrap_or_default();
                let dest_lua = lua_dir.join(file_basename);
                let dest_st = stplugin_dir.join(file_basename);

                let _ = fs::write(&dest_lua, &clean_content);
                let _ = fs::write(&dest_st, &clean_content);
                count += 1;
            }
        }
        Ok(format!("{} script(s) Steam Lua extrait(s) et déployé(s) dans Steam/config !", count))
    } else {
        let text = String::from_utf8_lossy(bytes);
        if text.contains("<html") || text.contains("cf-challenge") {
            return Err("Protection Cloudflare active. Glissez-déposez manuellement votre fichier .lua.".to_string());
        }

        let clean_lines: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim().to_lowercase().starts_with("setmanifest"))
            .collect();
        let clean_content = clean_lines.join("\n");

        let filename = format!("{}.lua", appid);
        fs::write(lua_dir.join(&filename), &clean_content).map_err(|e| format!("Erreur écriture lua: {}", e))?;
        fs::write(stplugin_dir.join(&filename), &clean_content).map_err(|e| format!("Erreur écriture stplug-in: {}", e))?;

        Ok(format!("Fichier Lua pour AppID {} généré avec succès dans Steam/config !", appid))
    }
}

fn extract_steam_appid_from_input(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
        return Some(s.to_string());
    }
    let lower = s.to_lowercase();
    for marker in &["/app/", "/apps/", "store/"] {
        if let Some(idx) = lower.find(marker) {
            let rest = &s[idx + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
}

pub async fn generate_steam_game_lua(appid_or_name: &str, custom_lua: Option<String>) -> Result<String, String> {
    let steam_path = detect_steam_install_path();

    let resolved_appid = if let Some(id) = extract_steam_appid_from_input(appid_or_name) {
        id
    } else {
        let results = crate::steam::search_steam_store(appid_or_name.trim()).await?;
        if let Some(first) = results.first() {
            first.appid.to_string()
        } else {
            return Err(format!("Aucun jeu Steam trouvé pour '{}'", appid_or_name));
        }
    };
    let appid = resolved_appid.as_str();

    if let Some(lua) = custom_lua {
        return save_lua_payload(lua.as_bytes(), appid, &steam_path);
    }

    // Try proxy.php first
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://walftech.com/proxy.php?id={}", appid);
    let mut payload_bytes: Option<Vec<u8>> = match client.get(&url)
        .header("referer", "https://walftech.com/generator.html")
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let b = resp.bytes().await.unwrap_or_default().to_vec();
            if b.starts_with(b"<html") || b.windows(12).any(|w| w == b"cf-challenge") {
                None
            } else {
                Some(b)
            }
        }
        _ => None,
    };

    // If direct proxy failed or got 403, run the PoW challenge resolver
    if payload_bytes.is_none() {
        match fetch_fallback_lua(appid).await {
            Ok(bytes) => payload_bytes = Some(bytes),
            Err(e) => return Err(format!("Erreur API Walftech (Résolution PoW: {})", e)),
        }
    }

    let final_bytes = payload_bytes.ok_or_else(|| "Impossible d'obtenir le payload Lua pour cet AppID".to_string())?;
    save_lua_payload(&final_bytes, appid, &steam_path)
}

pub fn get_opensteamtool_config_server() -> String {
    let steam_path = detect_steam_install_path();
    let toml_path = steam_path.join("opensteamtool.toml");
    if toml_path.exists() {
        if let Ok(content) = fs::read_to_string(&toml_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("url =") || trimmed.starts_with("url=") {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        return val.trim().trim_matches('"').to_string();
                    }
                }
            }
        }
    }
    "steamrun".to_string()
}

pub fn set_opensteamtool_config_server(server: &str) -> Result<String, String> {
    let steam_path = detect_steam_install_path();
    let toml_path = steam_path.join("opensteamtool.toml");
    if !toml_path.exists() {
        return Err(format!("opensteamtool.toml introuvable dans {}", steam_path.display()));
    }

    let content = fs::read_to_string(&toml_path).map_err(|e| e.to_string())?;
    let mut new_lines = Vec::new();
    let mut replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("url =") || trimmed.starts_with("url=")) && !replaced {
            new_lines.push(format!("url = \"{}\"", server));
            replaced = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !replaced {
        new_lines.push(format!("\nurl = \"{}\"", server));
    }

    fs::write(&toml_path, new_lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(format!("Serveur mis à jour sur '{}' dans opensteamtool.toml", server))
}
