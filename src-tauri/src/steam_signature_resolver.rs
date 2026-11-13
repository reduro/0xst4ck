use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn ensure_signatures_resolved(steam_path: &Path) {
    let steamclient_dll = steam_path.join("steamclient64.dll");
    if steamclient_dll.exists() {
        if let Ok(data) = fs::read(&steamclient_dll) {
            let sha256 = compute_sha256(&data);

            let pattern_dir = steam_path
                .join("opensteamtool")
                .join("pattern")
                .join("steamclient");
            let _ = fs::create_dir_all(&pattern_dir);

            let toml_path = pattern_dir.join(format!("{}.toml", sha256));

            if !toml_path.exists() {
                // Find latest template to copy
                let mut newest_content = None;
                let mut newest_time = std::time::UNIX_EPOCH;
                if let Ok(entries) = fs::read_dir(&pattern_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "toml") {
                            if let Ok(meta) = entry.metadata() {
                                if let Ok(mod_time) = meta.modified() {
                                    if mod_time > newest_time {
                                        newest_time = mod_time;
                                        newest_content = fs::read(&p).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(content) = newest_content {
                    let _ = fs::write(&toml_path, content);
                }
            } else if let Ok(content) = fs::read_to_string(&toml_path) {
                // Remove legacy unverified injected hooks that cause 0xC0000005 access violations
                if content.contains("[0xA185DB47]") || content.contains("[0x68211B4D]") {
                    if let Some(pos) = content.find("[0xA185DB47]") {
                        let clean = &content[..pos];
                        let _ = fs::write(&toml_path, clean.trim_end());
                    }
                }
            }

            // IPC pattern template for steamclient
            let ipc_dir = steam_path
                .join("opensteamtool")
                .join("ipc")
                .join("steamclient");
            let _ = fs::create_dir_all(&ipc_dir);
            let ipc_toml = ipc_dir.join(format!("{}.toml", sha256));
            if !ipc_toml.exists() {
                if let Ok(entries) = fs::read_dir(&ipc_dir) {
                    let mut latest_data = None;
                    let mut latest_time = std::time::UNIX_EPOCH;
                    for e in entries.flatten() {
                        let ep = e.path();
                        if ep.extension().map_or(false, |ext| ext == "toml") {
                            if let Ok(meta) = e.metadata() {
                                if let Ok(t) = meta.modified() {
                                    if t > latest_time {
                                        latest_time = t;
                                        latest_data = fs::read(&ep).ok();
                                    }
                                }
                            }
                        }
                    }
                    if let Some(content) = latest_data {
                        let _ = fs::write(&ipc_toml, content);
                    }
                }
            }
        }
    }

    // SteamUI pattern toml for current build
    let steamui_dll = steam_path.join("SteamUI.dll");
    if steamui_dll.exists() {
        if let Ok(ui_bytes) = fs::read(&steamui_dll) {
            let ui_sha256 = compute_sha256(&ui_bytes);
            let ui_pattern_dir = steam_path.join("opensteamtool").join("pattern").join("steamui");
            let _ = fs::create_dir_all(&ui_pattern_dir);
            let ui_toml = ui_pattern_dir.join(format!("{}.toml", ui_sha256));
            if !ui_toml.exists() {
                if let Ok(entries) = fs::read_dir(&ui_pattern_dir) {
                    let mut latest_data = None;
                    let mut latest_time = std::time::UNIX_EPOCH;
                    for e in entries.flatten() {
                        let ep = e.path();
                        if ep.extension().map_or(false, |ext| ext == "toml") {
                            if let Ok(meta) = e.metadata() {
                                if let Ok(t) = meta.modified() {
                                    if t > latest_time {
                                        latest_time = t;
                                        latest_data = fs::read(&ep).ok();
                                    }
                                }
                            }
                        }
                    }
                    if let Some(content) = latest_data {
                        let _ = fs::write(&ui_toml, content);
                    }
                }
            }
        }
    }
}
