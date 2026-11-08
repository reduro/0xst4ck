use crate::models::SteamSearchResult;
use crate::storage::get_banners_dir;
use serde_json::Value;
use std::fs;

fn clean_search_query(raw: &str) -> String {
    let s = raw.trim();
    let lower = s.to_lowercase();
    for marker in &["/app/", "/apps/", "store/"] {
        if let Some(idx) = lower.find(marker) {
            let rest = &s[idx + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return digits;
            }
        }
    }
    s.to_string()
}

pub async fn search_steam_store(query: &str) -> Result<Vec<SteamSearchResult>, String> {
    let cleaned = clean_search_query(query);
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l=french&cc=FR",
        urlencoding_simple(&cleaned)
    );

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Erreur requête Steam: {}", e))?;

    let json: Value = res
        .json()
        .await
        .map_err(|e| format!("Erreur JSON Steam: {}", e))?;

    let mut results = Vec::new();
    if let Some(items) = json["items"].as_array() {
        for item in items.iter().take(10) {
            let appid = item["id"].as_u64().unwrap_or(0);
            let name = item["name"].as_str().unwrap_or("").to_string();
            let tiny_image = item["tiny_image"].as_str().unwrap_or("").to_string();

            if appid > 0 {
                let header_image = if !tiny_image.is_empty() {
                    tiny_image
                } else {
                    format!(
                        "https://cdn.akamai.steamstatic.com/steam/apps/{}/header.jpg",
                        appid
                    )
                };

                results.push(SteamSearchResult {
                    appid,
                    name,
                    header_image,
                });
            }
        }
    }

    Ok(results)
}

pub async fn cache_steam_banner(appid: &str, image_url: &str) -> Result<String, String> {
    let banners_dir = get_banners_dir();
    let filename = format!("{}_banner.jpg", appid);
    let dest_path = banners_dir.join(&filename);

    if dest_path.exists() {
        return Ok(filename);
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| e.to_string())?;

    let capsule_url = format!(
        "https://cdn.akamai.steamstatic.com/steam/apps/{}/library_600x900.jpg",
        appid
    );

    for candidate in &[capsule_url.as_str(), image_url] {
        if candidate.is_empty() {
            continue;
        }
        if let Ok(response) = client.get(*candidate).send().await {
            if response.status().is_success() {
                if let Ok(bytes) = response.bytes().await {
                    let _ = fs::write(&dest_path, bytes);
                    return Ok(filename);
                }
            }
        }
    }

    // Fallback: try default steam capsule header
    let fallback_url = format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
        appid
    );
    if let Ok(response) = client.get(&fallback_url).send().await {
        if response.status().is_success() {
            if let Ok(bytes) = response.bytes().await {
                let _ = fs::write(&dest_path, bytes);
                return Ok(filename);
            }
        }
    }

    Ok(filename)
}

fn urlencoding_simple(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}
