use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};
use crate::steam_importer::detect_steam_install_path;

pub static EMBEDDED_ONLINEFIX_DLL: &[u8] = include_bytes!("../tools/OnlineFix.dll");
pub static EMBEDDED_OPENSTEAMTOOL_DLL: &[u8] = include_bytes!("../tools/launchertools/OpenSteamTool.dll");
pub static EMBEDDED_DWMAPI_DLL: &[u8] = include_bytes!("../tools/launchertools/dwmapi.dll");
pub static EMBEDDED_XINPUT_DLL: &[u8] = include_bytes!("../tools/launchertools/xinput1_4.dll");
pub static EMBEDDED_TOML: &[u8] = include_bytes!("../tools/launchertools/opensteamtool.toml");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DllStatusItem {
    pub name: String,
    pub path: String,
    pub expected_hash: String,
    pub current_hash: Option<String>,
    pub status: String, // "synced", "updated", "failed", "locked"
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DllSyncResult {
    pub all_synced: bool,
    pub items: Vec<DllStatusItem>,
    pub message: String,
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn compute_file_sha256(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    Some(compute_sha256(&bytes))
}

fn try_kill_steam() {
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "steam.exe"])
        .creation_flags(0x08000000)
        .status();
    std::thread::sleep(std::time::Duration::from_millis(600));
}

fn write_with_retry(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(target, bytes) {
        Ok(_) => Ok(()),
        Err(_) => {
            // Likely locked by Steam. Kill steam and retry once.
            try_kill_steam();
            fs::write(target, bytes).map_err(|e| format!("Failed to write {}: {}", target.display(), e))
        }
    }
}

pub fn ensure_onlinefix_dll() -> PathBuf {
    let app_data = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let target_dir = PathBuf::from(app_data).join("clemtoutlauncher").join("tools");
    let _ = fs::create_dir_all(&target_dir);
    let target_path = target_dir.join("OnlineFix.dll");

    let expected_hash = compute_sha256(EMBEDDED_ONLINEFIX_DLL);
    let current_hash = compute_file_sha256(&target_path);

    if current_hash.as_deref() != Some(&expected_hash) {
        let _ = write_with_retry(&target_path, EMBEDDED_ONLINEFIX_DLL);
    }

    target_path
}

pub fn verify_and_sync_all_dlls() -> DllSyncResult {
    let steam_path = detect_steam_install_path();
    let mut items = Vec::new();
    let mut all_ok = true;

    // 1. Steam/OpenSteamTool.dll
    let ost_path = steam_path.join("OpenSteamTool.dll");
    let ost_expected = compute_sha256(EMBEDDED_OPENSTEAMTOOL_DLL);
    let ost_curr = compute_file_sha256(&ost_path);
    if ost_curr.as_deref() != Some(&ost_expected) {
        match write_with_retry(&ost_path, EMBEDDED_OPENSTEAMTOOL_DLL) {
            Ok(_) => {
                items.push(DllStatusItem {
                    name: "OpenSteamTool.dll".to_string(),
                    path: ost_path.to_string_lossy().to_string(),
                    expected_hash: ost_expected.clone(),
                    current_hash: Some(ost_expected.clone()),
                    status: "updated".to_string(),
                });
            }
            Err(_) => {
                all_ok = false;
                items.push(DllStatusItem {
                    name: "OpenSteamTool.dll".to_string(),
                    path: ost_path.to_string_lossy().to_string(),
                    expected_hash: ost_expected,
                    current_hash: ost_curr,
                    status: "locked".to_string(),
                });
            }
        }
    } else {
        items.push(DllStatusItem {
            name: "OpenSteamTool.dll".to_string(),
            path: ost_path.to_string_lossy().to_string(),
            expected_hash: ost_expected,
            current_hash: ost_curr,
            status: "synced".to_string(),
        });
    }

    // 2. Steam/dwmapi.dll
    let dwm_path = steam_path.join("dwmapi.dll");
    let dwm_expected = compute_sha256(EMBEDDED_DWMAPI_DLL);
    let dwm_curr = compute_file_sha256(&dwm_path);
    if dwm_curr.as_deref() != Some(&dwm_expected) {
        match write_with_retry(&dwm_path, EMBEDDED_DWMAPI_DLL) {
            Ok(_) => {
                items.push(DllStatusItem {
                    name: "dwmapi.dll".to_string(),
                    path: dwm_path.to_string_lossy().to_string(),
                    expected_hash: dwm_expected.clone(),
                    current_hash: Some(dwm_expected.clone()),
                    status: "updated".to_string(),
                });
            }
            Err(_) => {
                all_ok = false;
                items.push(DllStatusItem {
                    name: "dwmapi.dll".to_string(),
                    path: dwm_path.to_string_lossy().to_string(),
                    expected_hash: dwm_expected,
                    current_hash: dwm_curr,
                    status: "locked".to_string(),
                });
            }
        }
    } else {
        items.push(DllStatusItem {
            name: "dwmapi.dll".to_string(),
            path: dwm_path.to_string_lossy().to_string(),
            expected_hash: dwm_expected,
            current_hash: dwm_curr,
            status: "synced".to_string(),
        });
    }

    // 3. Steam/xinput1_4.dll
    let xin_path = steam_path.join("xinput1_4.dll");
    let xin_expected = compute_sha256(EMBEDDED_XINPUT_DLL);
    let xin_curr = compute_file_sha256(&xin_path);
    if xin_curr.as_deref() != Some(&xin_expected) {
        match write_with_retry(&xin_path, EMBEDDED_XINPUT_DLL) {
            Ok(_) => {
                items.push(DllStatusItem {
                    name: "xinput1_4.dll".to_string(),
                    path: xin_path.to_string_lossy().to_string(),
                    expected_hash: xin_expected.clone(),
                    current_hash: Some(xin_expected.clone()),
                    status: "updated".to_string(),
                });
            }
            Err(_) => {
                all_ok = false;
                items.push(DllStatusItem {
                    name: "xinput1_4.dll".to_string(),
                    path: xin_path.to_string_lossy().to_string(),
                    expected_hash: xin_expected,
                    current_hash: xin_curr,
                    status: "locked".to_string(),
                });
            }
        }
    } else {
        items.push(DllStatusItem {
            name: "xinput1_4.dll".to_string(),
            path: xin_path.to_string_lossy().to_string(),
            expected_hash: xin_expected,
            current_hash: xin_curr,
            status: "synced".to_string(),
        });
    }

    // 4. Steam/opensteamtool.toml
    let toml_path = steam_path.join("opensteamtool.toml");
    if !toml_path.exists() {
        let _ = write_with_retry(&toml_path, EMBEDDED_TOML);
    }

    // 5. OnlineFix.dll in AppData
    let app_data = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let ofix_path = PathBuf::from(app_data).join("clemtoutlauncher").join("tools").join("OnlineFix.dll");
    let ofix_expected = compute_sha256(EMBEDDED_ONLINEFIX_DLL);
    let ofix_curr = compute_file_sha256(&ofix_path);
    if ofix_curr.as_deref() != Some(&ofix_expected) {
        match write_with_retry(&ofix_path, EMBEDDED_ONLINEFIX_DLL) {
            Ok(_) => {
                items.push(DllStatusItem {
                    name: "OnlineFix.dll".to_string(),
                    path: ofix_path.to_string_lossy().to_string(),
                    expected_hash: ofix_expected.clone(),
                    current_hash: Some(ofix_expected.clone()),
                    status: "updated".to_string(),
                });
            }
            Err(_) => {
                all_ok = false;
                items.push(DllStatusItem {
                    name: "OnlineFix.dll".to_string(),
                    path: ofix_path.to_string_lossy().to_string(),
                    expected_hash: ofix_expected,
                    current_hash: ofix_curr,
                    status: "failed".to_string(),
                });
            }
        }
    } else {
        items.push(DllStatusItem {
            name: "OnlineFix.dll".to_string(),
            path: ofix_path.to_string_lossy().to_string(),
            expected_hash: ofix_expected,
            current_hash: ofix_curr,
            status: "synced".to_string(),
        });
    }

    let message = if all_ok {
        "Toutes les DLLs et hooks sont vérifiés et synchronisés avec succès (SHA-256 valide).".to_string()
    } else {
        "Certaines DLLs n'ont pas pu être mises à jour (fichier verrouillé par Steam ou processus actif).".to_string()
    };

    DllSyncResult {
        all_synced: all_ok,
        items,
        message,
    }
}
