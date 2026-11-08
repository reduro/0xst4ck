use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaConfigFile {
    pub file_path: String,
    pub content: String,
    pub appid: String,
}

fn get_steam_path() -> Option<PathBuf> {
    let settings = crate::storage::load_settings();
    if let Some(steam_path) = settings.steam_path {
        if !steam_path.trim().is_empty() {
            let p = PathBuf::from(steam_path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    let default_p = PathBuf::from(r"C:\Program Files (x86)\Steam");
    if default_p.exists() {
        return Some(default_p);
    }
    None
}

pub fn read_game_lua(appid: &str) -> Result<LuaConfigFile, String> {
    let steam_dir = get_steam_path().ok_or_else(|| "Dossier Steam introuvable".to_string())?;

    let candidate_dirs = [
        steam_dir.join("config").join("lua"),
        steam_dir.join("config").join("stplug-in"),
    ];

    for dir in &candidate_dirs {
        if dir.exists() {
            let file = dir.join(format!("{}.lua", appid));
            if file.exists() {
                let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
                return Ok(LuaConfigFile {
                    file_path: file.to_string_lossy().to_string(),
                    content,
                    appid: appid.to_string(),
                });
            }
        }
    }

    let target = steam_dir.join("config").join("lua").join(format!("{}.lua", appid));
    Ok(LuaConfigFile {
        file_path: target.to_string_lossy().to_string(),
        content: String::new(),
        appid: appid.to_string(),
    })
}

pub fn save_game_lua(file_path: &str, content: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, content).map_err(|e| format!("Impossible d'enregistrer le fichier Lua : {}", e))
}

pub fn update_lua_manifest(appid: &str, depot_id: &str, manifest_id: &str) -> Result<String, String> {
    let steam_dir = get_steam_path().ok_or_else(|| "Dossier Steam introuvable".to_string())?;

    let candidate_dirs = [
        steam_dir.join("config").join("lua"),
        steam_dir.join("config").join("stplug-in"),
    ];

    let mut found_path = None;
    let mut content = String::new();

    for dir in &candidate_dirs {
        let file = dir.join(format!("{}.lua", appid));
        if file.exists() {
            if let Ok(c) = fs::read_to_string(&file) {
                found_path = Some(file);
                content = c;
                break;
            }
        }
    }

    let target_path = found_path.unwrap_or_else(|| {
        let p = steam_dir.join("config").join("lua");
        let _ = fs::create_dir_all(&p);
        p.join(format!("{}.lua", appid))
    });

    let new_line = format!("setManifestid({}, \"{}\")", depot_id.trim(), manifest_id.trim());

    let pattern = format!("setManifestid({}", depot_id.trim());
    let updated_content = if let Some(idx) = content.find(&pattern) {
        let line_end = content[idx..].find('\n').map(|p| idx + p).unwrap_or(content.len());
        let mut res = String::new();
        res.push_str(&content[..idx]);
        res.push_str(&new_line);
        res.push_str(&content[line_end..]);
        res
    } else {
        let mut res = content;
        if !res.is_empty() && !res.ends_with('\n') {
            res.push('\n');
        }
        res.push_str(&new_line);
        res.push('\n');
        res
    };

    fs::write(&target_path, &updated_content).map_err(|e| e.to_string())?;

    let stplug_file = steam_dir.join("config").join("stplug-in").join(format!("{}.lua", appid));
    if stplug_file.exists() && stplug_file != target_path {
        let _ = fs::write(&stplug_file, &updated_content);
    }

    Ok(format!("Manifest {} mis à jour dans {}", manifest_id, target_path.display()))
}
