use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopItem {
    pub id: String,
    pub title: String,
    pub preview_url: String,
    pub description: String,
    pub author: String,
    pub subscriptions: String,
    pub is_installed_locally: bool,
}

fn clean_text(raw: &str) -> String {
    let mut s = raw
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("\\u002F", "/")
        .replace("\\/", "/")
        .replace("\\n", " ")
        .replace("\\r", "")
        .replace("\\\"", "\"")
        .trim()
        .to_string();

    while s.ends_with('\\') {
        s.pop();
    }
    s.trim().to_string()
}

pub fn find_local_steam_workshop(appid: &str, item_id: &str) -> Option<PathBuf> {
    let main_steam = crate::steam_importer::detect_steam_install_path();
    let libraries = crate::steam_importer::get_steam_libraries(&main_steam);
    for lib in libraries {
        let p = lib.join("steamapps").join("workshop").join("content").join(appid).join(item_id);
        if p.exists() && p.is_dir() {
            return Some(p);
        }
    }
    None
}

pub fn import_local_workshop_item(appid: &str, item_id: &str, game_path: &str) -> Result<String, String> {
    let source_dir = find_local_steam_workshop(appid, item_id)
        .ok_or_else(|| "Mod introuvable dans vos dossiers Steam locaux. Abonnez-vous d'abord dans le client Steam.".to_string())?;

    let game_dir = Path::new(game_path)
        .parent()
        .ok_or_else(|| "Dossier du jeu introuvable".to_string())?;

    let mods_dir = game_dir.join("Mods");
    if !mods_dir.exists() {
        let _ = fs::create_dir_all(&mods_dir);
    }

    let target_mod_folder = mods_dir.join(item_id);
    if target_mod_folder.exists() {
        let _ = fs::remove_dir_all(&target_mod_folder);
    }

    copy_dir_all(&source_dir, &target_mod_folder)
        .map_err(|e| format!("Impossible de copier les fichiers du mod : {}", e))?;

    Ok(format!("Mod {} importé avec succès depuis votre dossier Steam local !", item_id))
}

pub async fn search_workshop_items(appid: &str, query: &str) -> Result<Vec<WorkshopItem>, String> {
    if appid.is_empty() || appid == "480" {
        return Ok(Vec::new());
    }

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let clean_query = query.replace(' ', "+");
    let url = if clean_query.trim().is_empty() {
        format!(
            "https://steamcommunity.com/workshop/browse/?appid={}&browsesort=trend&section=readytouseitems",
            appid
        )
    } else {
        format!(
            "https://steamcommunity.com/workshop/browse/?appid={}&searchtext={}&browsesort=trend&section=readytouseitems",
            appid, clean_query
        )
    };

    let response = match client
        .get(&url)
        .header("Cookie", "wants_mature_content=1; birthtime=788918401")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Connexion Steam Workshop impossible : {}", e)),
    };

    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let html = response.text().await.unwrap_or_default();
    let mut ids: Vec<String> = Vec::new();
    let mut seen_ids = HashSet::new();

    let needle = "filedetails/?id=";
    let mut cursor = 0;
    while let Some(pos) = html[cursor..].find(needle) {
        let actual_pos = cursor + pos + needle.len();
        let rest = &html[actual_pos..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && !seen_ids.contains(&digits) {
            seen_ids.insert(digits.clone());
            ids.push(digits);
        }
        cursor = actual_pos + 1;
        if ids.len() >= 24 {
            break;
        }
    }

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    // Fetch batch details from Steam Web API
    let mut form_params: Vec<(String, String)> = Vec::new();
    form_params.push(("itemcount".to_string(), ids.len().to_string()));
    for (i, id) in ids.iter().enumerate() {
        form_params.push((format!("publishedfileids[{}]", i), id.clone()));
    }

    let api_resp = client
        .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
        .form(&form_params)
        .send()
        .await;

    let mut items = Vec::new();
    if let Ok(res) = api_resp {
        if let Ok(val) = res.json::<serde_json::Value>().await {
            if let Some(arr) = val["response"]["publishedfiledetails"].as_array() {
                for entry in arr {
                    let id = entry["publishedfileid"].as_str().unwrap_or("").to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let title = clean_text(entry["title"].as_str().unwrap_or("Mod sans titre"));
                    let preview_url = entry["preview_url"].as_str().unwrap_or("").to_string();
                    let desc = clean_text(entry["description"].as_str().unwrap_or(""));
                    let subs = entry["subscriptions"]
                        .as_u64()
                        .map(|s| format!("{} abonnés", s))
                        .unwrap_or_else(|| "Populaire".to_string());

                    let is_installed_locally = find_local_steam_workshop(appid, &id).is_some();

                    items.push(WorkshopItem {
                        id,
                        title,
                        preview_url,
                        description: desc,
                        author: "Communauté Steam".to_string(),
                        subscriptions: subs,
                        is_installed_locally,
                    });
                }
            }
        }
    }

    Ok(items)
}

fn get_steamcmd_path() -> PathBuf {
    let launcher_tools = PathBuf::from(r"C:\Serveur\clemtout-launcher-v2\tools\steamcmd");
    let installed_tools = dirs::data_local_dir()
        .map(|d| d.join("Programs").join("ClemtoutLauncher").join("tools").join("steamcmd"))
        .unwrap_or_else(|| launcher_tools.clone());

    if installed_tools.join("steamcmd.exe").exists() {
        installed_tools.join("steamcmd.exe")
    } else {
        launcher_tools.join("steamcmd.exe")
    }
}

pub async fn download_workshop_file(appid: &str, item_id: &str, game_path: &str) -> Result<String, String> {
    if appid.is_empty() || appid == "480" {
        return Err("AppID invalide pour le Workshop Steam".to_string());
    }

    let game_dir = Path::new(game_path)
        .parent()
        .ok_or_else(|| "Dossier du jeu introuvable".to_string())?;

    let mods_dir = game_dir.join("Mods");
    if !mods_dir.exists() {
        let _ = fs::create_dir_all(&mods_dir);
    }
    let target_mod_folder = mods_dir.join(item_id);

    // 1. Bypass immédiat : Vérifier si le mod existe déjà dans le cache Steam local
    if let Some(local_path) = find_local_steam_workshop(appid, item_id) {
        if target_mod_folder.exists() {
            let _ = fs::remove_dir_all(&target_mod_folder);
        }
        if copy_dir_all(&local_path, &target_mod_folder).is_ok() {
            return Ok(format!("Mod {} synchronisé directement depuis votre cache Steam local !", item_id));
        }
    }

    // 2. Vérifier si l'API Steam RemoteStorage donne une URL HTTP directe (file_url)
    let client = Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| e.to_string())?;

    let form_params = [
        ("itemcount", "1"),
        ("publishedfileids[0]", item_id),
    ];
    if let Ok(res) = client
        .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
        .form(&form_params)
        .send()
        .await
    {
        if let Ok(val) = res.json::<serde_json::Value>().await {
            if let Some(file_url) = val["response"]["publishedfiledetails"][0]["file_url"].as_str() {
                if !file_url.is_empty() {
                    if let Ok(bytes_resp) = client.get(file_url).send().await {
                        if let Ok(body) = bytes_resp.bytes().await {
                            let _ = fs::create_dir_all(&target_mod_folder);
                            let target_file = target_mod_folder.join(format!("{}.bin", item_id));
                            if fs::write(&target_file, &body).is_ok() {
                                return Ok(format!("Mod {} téléchargé directement via le CDN Steam !", item_id));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Téléchargement via SteamCMD anonyme
    let steamcmd_exe = get_steamcmd_path();
    if !steamcmd_exe.exists() {
        return Err(format!(
            "SteamCMD introuvable dans {}. Vous pouvez vous abonner directement dans le client Steam puis cliquer sur 'Importer'.",
            steamcmd_exe.display()
        ));
    }

    let steamcmd_dir = steamcmd_exe.parent().unwrap();
    let output = Command::new(&steamcmd_exe)
        .current_dir(steamcmd_dir)
        .arg("+login")
        .arg("anonymous")
        .arg("+workshop_download_item")
        .arg(appid)
        .arg(item_id)
        .arg("validate")
        .arg("+quit")
        .output()
        .map_err(|e| format!("Erreur exécution SteamCMD : {}", e))?;

    let content_dir = steamcmd_dir
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join(appid)
        .join(item_id);

    if content_dir.exists() {
        if target_mod_folder.exists() {
            let _ = fs::remove_dir_all(&target_mod_folder);
        }
        copy_dir_all(&content_dir, &target_mod_folder)
            .map_err(|e| format!("Impossible de copier les fichiers du mod : {}", e))?;
        return Ok(format!("Mod {} installé avec succès via SteamCMD !", item_id));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Access Denied") || stdout.contains("k_EResultAccessDenied") {
        return Err(
            "ACCESS_DENIED: Ce jeu bloque le téléchargement anonyme SteamCMD. Ouvrez Steam (avec OpenSteamTool), cliquez sur 'S'abonner dans Steam', puis sur 'Importer depuis Steam'.".to_string()
        );
    }

    Err(format!(
        "Échec du téléchargement du mod : {}",
        stdout.chars().take(200).collect::<String>()
    ))
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
