use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub apiname: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub icon_gray: String,
    pub unlocked: bool,
    pub unlocked_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAchievementsSummary {
    pub appid: String,
    pub total: usize,
    pub unlocked: usize,
    pub percentage: f64,
    pub achievements: Vec<Achievement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockedRecord {
    pub apiname: String,
    pub unlocked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnlockedStore {
    pub appid: String,
    #[serde(default)]
    pub unlocked: Vec<UnlockedRecord>,
    #[serde(default)]
    pub manual_locked: Vec<String>,
}

pub fn get_achievements_dir() -> PathBuf {
    let dir = crate::storage::get_appdata_dir().join("achievements");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

pub fn load_unlocked_store(appid: &str) -> UnlockedStore {
    let path = get_achievements_dir().join(format!("{}.json", appid));
    if !path.exists() {
        return UnlockedStore {
            appid: appid.to_string(),
            unlocked: Vec::new(),
            manual_locked: Vec::new(),
        };
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| UnlockedStore {
            appid: appid.to_string(),
            unlocked: Vec::new(),
            manual_locked: Vec::new(),
        }),
        Err(_) => UnlockedStore {
            appid: appid.to_string(),
            unlocked: Vec::new(),
            manual_locked: Vec::new(),
        },
    }
}

pub fn save_unlocked_store(store: &UnlockedStore) {
    let path = get_achievements_dir().join(format!("{}.json", store.appid));
    if let Ok(content) = serde_json::to_string_pretty(store) {
        let _ = fs::write(path, content);
    }
}

pub fn record_unlock(appid: &str, apiname: &str) -> bool {
    let mut store = load_unlocked_store(appid);
    // Remove from manual locks
    store.manual_locked.retain(|l| !l.eq_ignore_ascii_case(apiname));

    if store.unlocked.iter().any(|u| u.apiname.eq_ignore_ascii_case(apiname)) {
        save_unlocked_store(&store);
        return false;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    store.unlocked.push(UnlockedRecord {
        apiname: apiname.to_string(),
        unlocked_at: now,
    });
    save_unlocked_store(&store);
    true
}

pub fn record_lock(appid: &str, apiname: &str) -> bool {
    let mut store = load_unlocked_store(appid);
    store.unlocked.retain(|u| !u.apiname.eq_ignore_ascii_case(apiname));
    if !store.manual_locked.iter().any(|l| l.eq_ignore_ascii_case(apiname)) {
        store.manual_locked.push(apiname.to_string());
    }
    save_unlocked_store(&store);
    true
}

pub fn record_unlock_all(appid: &str, apinames: &[String]) {
    let mut store = load_unlocked_store(appid);
    store.manual_locked.retain(|l| !apinames.iter().any(|n| n.eq_ignore_ascii_case(l)));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    for name in apinames {
        if !store.unlocked.iter().any(|u| u.apiname.eq_ignore_ascii_case(name)) {
            store.unlocked.push(UnlockedRecord {
                apiname: name.to_string(),
                unlocked_at: now,
            });
        }
    }
    save_unlocked_store(&store);
}

pub fn record_reset_all(appid: &str, all_apinames: &[String]) {
    let mut store = load_unlocked_store(appid);
    store.unlocked.clear();
    store.manual_locked = all_apinames.to_vec();
    save_unlocked_store(&store);
}

// ---------------------------------------------------------
// Valve Binary KeyValues Parser for UserGameStatsSchema.bin
// ---------------------------------------------------------
fn parse_kv_string(data: &[u8], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    if *pos < data.len() {
        let s = String::from_utf8_lossy(&data[start..*pos]).to_string();
        *pos += 1; // skip null
        Some(s)
    } else {
        None
    }
}

#[derive(Default, Debug)]
struct KvNode {
    sub: HashMap<String, KvNode>,
    val: Option<String>,
}

fn parse_kv_node(data: &[u8], pos: &mut usize) -> KvNode {
    let mut node = KvNode::default();
    while *pos < data.len() {
        let type_byte = data[*pos];
        *pos += 1;
        if type_byte == 8 {
            break; // end of object
        }
        let Some(key) = parse_kv_string(data, pos) else {
            break;
        };
        match type_byte {
            0 => {
                let child = parse_kv_node(data, pos);
                node.sub.insert(key, child);
            }
            1 => {
                if let Some(val) = parse_kv_string(data, pos) {
                    let mut child = KvNode::default();
                    child.val = Some(val);
                    node.sub.insert(key, child);
                }
            }
            2 => {
                if *pos + 4 <= data.len() {
                    let val = i32::from_le_bytes([data[*pos], data[*pos+1], data[*pos+2], data[*pos+3]]);
                    *pos += 4;
                    let mut child = KvNode::default();
                    child.val = Some(val.to_string());
                    node.sub.insert(key, child);
                }
            }
            3 => *pos += 4, // float
            7 => {
                if *pos + 8 <= data.len() {
                    let val = i64::from_le_bytes([
                        data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                        data[*pos+4], data[*pos+5], data[*pos+6], data[*pos+7],
                    ]);
                    *pos += 8;
                    let mut child = KvNode::default();
                    child.val = Some(val.to_string());
                    node.sub.insert(key, child);
                }
            }
            _ => break,
        }
    }
    node
}

fn get_localized_string(node: Option<&KvNode>, preferred_langs: &[&str]) -> Option<String> {
    let node = node?;
    if let Some(ref v) = node.val {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    for &lang in preferred_langs {
        if let Some(child) = node.sub.get(lang) {
            if let Some(ref v) = child.val {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    for (k, child) in &node.sub {
        if k != "token" {
            if let Some(ref v) = child.val {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

pub fn parse_schema_bin(appid: &str, bin_path: &Path) -> Vec<Achievement> {
    let Ok(data) = fs::read(bin_path) else {
        return Vec::new();
    };
    let mut pos = 0;
    let root = parse_kv_node(&data, &mut pos);
    let mut results = Vec::new();

    // Look for root -> [appid] -> stats -> [*] -> bits -> [*]
    if let Some(app_node) = root.sub.get(appid).or_else(|| root.sub.values().next()) {
        if let Some(stats_node) = app_node.sub.get("stats") {
            for stat in stats_node.sub.values() {
                if let Some(bits_node) = stat.sub.get("bits") {
                    for bit in bits_node.sub.values() {
                        let apiname = bit.sub.get("name").and_then(|n| n.val.clone()).unwrap_or_default();
                        if apiname.is_empty() {
                            continue;
                        }

                        let disp = bit.sub.get("display");
                        let name = disp
                            .and_then(|d| get_localized_string(d.sub.get("name"), &["french", "english"]))
                            .unwrap_or_else(|| apiname.clone());

                        let desc = disp
                            .and_then(|d| get_localized_string(d.sub.get("desc"), &["french", "english"]))
                            .unwrap_or_default();

                        let icon_hash = disp
                            .and_then(|d| d.sub.get("icon"))
                            .and_then(|i| i.val.clone())
                            .unwrap_or_default();

                        let icon_gray_hash = disp
                            .and_then(|d| d.sub.get("icon_gray"))
                            .and_then(|i| i.val.clone())
                            .unwrap_or_default();

                        let icon_url = if !icon_hash.is_empty() {
                            format!("https://shared.akamai.steamstatic.com/community_assets/images/apps/{}/{}", appid, icon_hash)
                        } else {
                            String::new()
                        };
                        let icon_gray_url = if !icon_gray_hash.is_empty() {
                            format!("https://shared.akamai.steamstatic.com/community_assets/images/apps/{}/{}", appid, icon_gray_hash)
                        } else {
                            icon_url.clone()
                        };

                        results.push(Achievement {
                            apiname,
                            name,
                            description: desc,
                            icon: icon_url,
                            icon_gray: icon_gray_url,
                            unlocked: false,
                            unlocked_at: None,
                        });
                    }
                }
            }
        }
    }

    results
}

// -----------------------------------------------------------------------------
// Read unlocked achievements directly from Steam's local cache
// -----------------------------------------------------------------------------
pub fn read_steam_unlocked_cache(appid: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    let steam_dir = crate::loader::get_steam_dir();
    let stats_dir = steam_dir.join("appcache").join("stats");
    if !stats_dir.exists() {
        return map;
    }

    let schema_path = stats_dir.join(format!("UserGameStatsSchema_{}.bin", appid));
    let mut bit_to_apiname: HashMap<String, String> = HashMap::new();
    if schema_path.exists() {
        if let Ok(data) = fs::read(&schema_path) {
            let mut pos = 0;
            let root = parse_kv_node(&data, &mut pos);
            if let Some(app_node) = root.sub.get(appid).or_else(|| root.sub.values().next()) {
                if let Some(stats_node) = app_node.sub.get("stats") {
                    for stat in stats_node.sub.values() {
                        if let Some(bits_node) = stat.sub.get("bits") {
                            for (bit_key, bit_val) in &bits_node.sub {
                                if let Some(ref api) = bit_val.sub.get("name").and_then(|n| n.val.clone()) {
                                    bit_to_apiname.insert(bit_key.clone(), api.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(&stats_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("UserGameStats_") && fname.ends_with(&format!("_{}.bin", appid)) {
                if let Ok(data) = fs::read(entry.path()) {
                    let mut pos = 0;
                    let root = parse_kv_node(&data, &mut pos);
                    if let Some(cache_node) = root.sub.get("cache") {
                        for sub_cat in cache_node.sub.values() {
                            if let Some(ach_times) = sub_cat.sub.get("AchievementTimes") {
                                for (bit_key, time_node) in &ach_times.sub {
                                    let ts = time_node.val.as_deref().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                                    if let Some(apiname) = bit_to_apiname.get(bit_key) {
                                        map.insert(apiname.to_lowercase(), ts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    map
}

pub async fn get_schema_for_appid(appid: &str) -> Vec<Achievement> {
    let steam_dir = crate::loader::get_steam_dir();
    let bin_path = steam_dir.join("appcache").join("stats").join(format!("UserGameStatsSchema_{}.bin", appid));
    if bin_path.exists() {
        let list = parse_schema_bin(appid, &bin_path);
        if !list.is_empty() {
            return list;
        }
    }

    let cache_dir = crate::storage::get_appdata_dir().join("achievements_cache");
    let cache_path = cache_dir.join(format!("{}.json", appid));
    if cache_path.exists() {
        if let Ok(content) = fs::read_to_string(&cache_path) {
            if let Ok(list) = serde_json::from_str::<Vec<Achievement>>(&content) {
                if !list.is_empty() {
                    return list;
                }
            }
        }
    }

    let fallback = fetch_schema_from_web(appid).await;
    if !fallback.is_empty() {
        let _ = fs::create_dir_all(&cache_dir);
        let _ = fs::write(&cache_path, serde_json::to_string_pretty(&fallback).unwrap_or_default());
    }

    fallback
}

async fn fetch_schema_from_web(appid: &str) -> Vec<Achievement> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok();
    let Some(client) = client else {
        return Vec::new();
    };

    let url = format!("https://store.steampowered.com/api/appdetails?appids={}&l=french", appid);
    let Ok(resp) = client.get(&url).header("User-Agent", "Mozilla/5.0").send().await else {
        return Vec::new();
    };
    let Ok(val) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };

    let Some(ach_data) = val.get(appid).and_then(|a| a.get("data")).and_then(|d| d.get("achievements")) else {
        return Vec::new();
    };

    let mut list = Vec::new();
    if let Some(highlighted) = ach_data.get("highlighted").and_then(|h| h.as_array()) {
        for h in highlighted {
            let name = h.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string();
            let path = h.get("path").and_then(|p| p.as_str()).unwrap_or_default().to_string();
            list.push(Achievement {
                apiname: name.clone(),
                name,
                description: String::new(),
                icon: path.clone(),
                icon_gray: path,
                unlocked: false,
                unlocked_at: None,
            });
        }
    }
    list
}

pub fn process_pending_events() {
    let events_path = get_achievements_dir().join("events.log");
    if events_path.exists() {
        if let Ok(content) = fs::read_to_string(&events_path) {
            let _ = fs::remove_file(&events_path);
            for line in content.lines() {
                let parts: Vec<&str> = line.trim().split(':').collect();
                if parts.len() == 3 {
                    let appid = parts[0];
                    let ev_type = parts[1];
                    let apiname = parts[2];
                    if ev_type == "UNLOCK" {
                        record_unlock(appid, apiname);
                    } else if ev_type == "CLEAR" {
                        record_lock(appid, apiname);
                    }
                } else if parts.len() == 2 {
                    let appid = parts[0];
                    let apiname = parts[1];
                    record_unlock(appid, apiname);
                }
            }
        }
    }
}

pub async fn get_game_achievements_summary(game_id: &str) -> Result<GameAchievementsSummary, String> {
    process_pending_events();
    let games = crate::storage::load_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("Jeu introuvable")?;
    let appid = game.steam_appid.as_deref().unwrap_or("480");

    let mut achievements = get_schema_for_appid(appid).await;
    let local_store = load_unlocked_store(appid);
    let steam_unlocked = read_steam_unlocked_cache(appid);

    let mut unlocked_count = 0;
    for ach in &mut achievements {
        let key = ach.apiname.to_lowercase();
        // 1. If user explicitly locked it manually, respect manual lock!
        let is_manually_locked = local_store.manual_locked.iter().any(|l| l.eq_ignore_ascii_case(&ach.apiname));

        if is_manually_locked {
            ach.unlocked = false;
            ach.unlocked_at = None;
        } else if let Some(record) = local_store.unlocked.iter().find(|u| u.apiname.eq_ignore_ascii_case(&ach.apiname)) {
            // 2. Unlocked locally
            ach.unlocked = true;
            ach.unlocked_at = Some(record.unlocked_at);
            unlocked_count += 1;
        } else if let Some(&ts) = steam_unlocked.get(&key) {
            // 3. Unlocked in Steam cache
            ach.unlocked = true;
            ach.unlocked_at = Some(ts);
            unlocked_count += 1;
        }
    }

    let total = achievements.len();
    let percentage = if total > 0 {
        ((unlocked_count as f64 / total as f64) * 100.0).round()
    } else {
        0.0
    };

    Ok(GameAchievementsSummary {
        appid: appid.to_string(),
        total,
        unlocked: unlocked_count,
        percentage,
        achievements,
    })
}

pub fn sync_achievement_to_steam(appid: &str, apiname: &str) -> Result<String, String> {
    let tools = crate::loader::get_tools_dir();
    let sync_exe = tools.join("steam_ach_sync.exe");
    if !sync_exe.exists() {
        return Err("steam_ach_sync.exe introuvable".to_string());
    }

    let output = Command::new(&sync_exe)
        .arg(appid)
        .arg("unlock")
        .arg(apiname)
        .current_dir(sync_exe.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| format!("Erreur exécution sync: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

pub fn sync_all_achievements_to_steam(appid: &str, apinames: &[String]) -> Result<String, String> {
    let tools = crate::loader::get_tools_dir();
    let sync_exe = tools.join("steam_ach_sync.exe");
    if !sync_exe.exists() {
        return Err("steam_ach_sync.exe introuvable".to_string());
    }

    let mut cmd = Command::new(&sync_exe);
    cmd.arg(appid).arg("unlock_all");
    for name in apinames {
        cmd.arg(name);
    }

    let output = cmd
        .current_dir(sync_exe.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| format!("Erreur exécution sync_all: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

pub fn sync_clear_achievement_from_steam(appid: &str, apiname: &str) -> Result<String, String> {
    let tools = crate::loader::get_tools_dir();
    let sync_exe = tools.join("steam_ach_sync.exe");
    if !sync_exe.exists() {
        return Err("steam_ach_sync.exe introuvable".to_string());
    }

    let output = Command::new(&sync_exe)
        .arg(appid)
        .arg("clear")
        .arg(apiname)
        .current_dir(sync_exe.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| format!("Erreur exécution clear: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

pub fn sync_clear_all_from_steam(appid: &str, apinames: &[String]) -> Result<String, String> {
    let tools = crate::loader::get_tools_dir();
    let sync_exe = tools.join("steam_ach_sync.exe");
    if !sync_exe.exists() {
        return Err("steam_ach_sync.exe introuvable".to_string());
    }

    let mut cmd = Command::new(&sync_exe);
    cmd.arg(appid).arg("clear_all");
    for name in apinames {
        cmd.arg(name);
    }

    let output = cmd
        .current_dir(sync_exe.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| format!("Erreur exécution clear_all: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}
