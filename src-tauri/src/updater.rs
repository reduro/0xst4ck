use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_name: String,
    pub changelog: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
}

pub async fn check_for_updates(repo: Option<String>) -> Result<UpdateInfo, String> {
    let target_repo = repo
        .filter(|r| !r.trim().is_empty())
        .unwrap_or_else(|| "reduro/0xst4ck".to_string());

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let client = Client::builder()
        .user_agent("ClemtoutLauncher-Updater")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://api.github.com/repos/{}/releases/latest", target_repo.trim());

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => return Err(format!("Erreur réseau GitHub: {}", e)),
    };

    if !resp.status().is_success() {
        return Err(format!("Impossible de contacter GitHub (HTTP {})", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let raw_tag = val["tag_name"].as_str().unwrap_or("0.0.0");
    let latest_version = raw_tag.trim_start_matches('v').to_string();
    let release_name = val["name"].as_str().unwrap_or(raw_tag).to_string();
    let changelog = val["body"].as_str().unwrap_or("").to_string();

    let mut download_url = None;
    let mut asset_name = None;

    if let Some(assets) = val["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if name.ends_with(".exe") {
                download_url = asset["browser_download_url"].as_str().map(|s| s.to_string());
                asset_name = Some(name.to_string());
                break;
            }
        }
    }

    let is_newer = compare_versions(&latest_version, &current_version);

    Ok(UpdateInfo {
        update_available: is_newer,
        current_version,
        latest_version,
        release_name,
        changelog,
        download_url,
        asset_name,
    })
}

fn compare_versions(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
            .collect()
    };
    let l_parts = parse(latest);
    let c_parts = parse(current);
    for i in 0..std::cmp::max(l_parts.len(), c_parts.len()) {
        let l = l_parts.get(i).copied().unwrap_or(0);
        let c = c_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    false
}

pub async fn download_and_run_installer(download_url: String) -> Result<String, String> {
    let client = Client::builder()
        .user_agent("ClemtoutLauncher-Updater")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Erreur téléchargement: {}", e))?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Erreur lecture binaire: {}", e))?;

    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join("Clemtout-Launcher-Update.exe");

    fs::write(&installer_path, &bytes).map_err(|e| format!("Erreur écriture fichier: {}", e))?;

    // Launch installer detached
    Command::new(&installer_path)
        .spawn()
        .map_err(|e| format!("Erreur exécution installeur: {}", e))?;

    Ok("L'installeur a été lancé !".to_string())
}
