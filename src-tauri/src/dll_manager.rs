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

const CREATE_NO_WINDOW: u32 = 0x08000000;

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

fn write_with_retry(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(target, bytes).map_err(|e| format!("Failed to write {}: {}", target.display(), e))
}

pub fn is_steam_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq steam.exe", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
        return stdout.contains("steam.exe");
    }
    false
}

pub fn kill_steam() {
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "steam.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    std::thread::sleep(std::time::Duration::from_millis(600));
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

    // Check OpenSteamTool files in Steam folder
    let ost_path = steam_path.join("OpenSteamTool.dll");
    let ost_expected = compute_sha256(EMBEDDED_OPENSTEAMTOOL_DLL);
    let ost_curr = compute_file_sha256(&ost_path);
    let ost_mismatch = ost_curr.as_deref() != Some(&ost_expected);

    let dwm_path = steam_path.join("dwmapi.dll");
    let dwm_expected = compute_sha256(EMBEDDED_DWMAPI_DLL);
    let dwm_curr = compute_file_sha256(&dwm_path);
    let dwm_mismatch = dwm_curr.as_deref() != Some(&dwm_expected);

    let xin_path = steam_path.join("xinput1_4.dll");
    let xin_expected = compute_sha256(EMBEDDED_XINPUT_DLL);
    let xin_curr = compute_file_sha256(&xin_path);
    let xin_mismatch = xin_curr.as_deref() != Some(&xin_expected);

    let toml_path = steam_path.join("opensteamtool.toml");
    let toml_missing = !toml_path.exists();

    let needs_steam_update = ost_mismatch || dwm_mismatch || xin_mismatch || toml_missing;

    // If any Steam DLL is missing or mismatched, close Steam if running and update
    if needs_steam_update {
        if is_steam_running() {
            kill_steam();
        }

        // OpenSteamTool.dll
        if ost_mismatch {
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

        // dwmapi.dll
        if dwm_mismatch {
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

        // xinput1_4.dll
        if xin_mismatch {
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

        if toml_missing {
            let _ = write_with_retry(&toml_path, EMBEDDED_TOML);
        }
    } else {
        // All files exist and match hashes: DO NOT TOUCH STEAM AT ALL!
        items.push(DllStatusItem {
            name: "OpenSteamTool.dll".to_string(),
            path: ost_path.to_string_lossy().to_string(),
            expected_hash: ost_expected,
            current_hash: ost_curr,
            status: "synced".to_string(),
        });
        items.push(DllStatusItem {
            name: "dwmapi.dll".to_string(),
            path: dwm_path.to_string_lossy().to_string(),
            expected_hash: dwm_expected,
            current_hash: dwm_curr,
            status: "synced".to_string(),
        });
        items.push(DllStatusItem {
            name: "xinput1_4.dll".to_string(),
            path: xin_path.to_string_lossy().to_string(),
            expected_hash: xin_expected,
            current_hash: xin_curr,
            status: "synced".to_string(),
        });
    }

    // Dynamic pattern & signature resolution for Steam updates
    crate::steam_signature_resolver::ensure_signatures_resolved(&steam_path);

    // OnlineFix.dll in AppData (isolated from Steam process)
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenSteamToolStatus {
    pub installed: bool,
    pub hook_type: String,
    pub dll_hash: Option<String>,
    pub steam_path: String,
    pub fallback_enabled: bool,
}

pub fn get_opensteamtool_hook_status() -> OpenSteamToolStatus {
    let steam_path = detect_steam_install_path();
    let dwm_path = steam_path.join("dwmapi.dll");
    let xin_path = steam_path.join("xinput1_4.dll");
    let ost_path = steam_path.join("OpenSteamTool.dll");

    let has_dwm = dwm_path.exists();
    let has_xin = xin_path.exists();
    let has_ost = ost_path.exists();

    let installed = (has_dwm || has_xin) && has_ost;
    let hook_type = match (has_dwm, has_xin) {
        (true, true) => "both",
        (true, false) => "dwmapi",
        (false, true) => "xinput1_4",
        (false, false) => "none",
    }.to_string();

    let dll_hash = compute_file_sha256(&ost_path);

    OpenSteamToolStatus {
        installed,
        hook_type,
        dll_hash,
        steam_path: steam_path.to_string_lossy().to_string(),
        fallback_enabled: true,
    }
}

pub fn uninstall_opensteamtool_hook() -> Result<String, String> {
    let steam_path = detect_steam_install_path();
    let dwm_path = steam_path.join("dwmapi.dll");
    let xin_path = steam_path.join("xinput1_4.dll");
    let ost_path = steam_path.join("OpenSteamTool.dll");

    if is_steam_running() {
        kill_steam();
    }

    let mut removed = Vec::new();
    if dwm_path.exists() {
        fs::remove_file(&dwm_path).map_err(|e| format!("Impossible de supprimer dwmapi.dll: {}", e))?;
        removed.push("dwmapi.dll");
    }
    if xin_path.exists() {
        fs::remove_file(&xin_path).map_err(|e| format!("Impossible de supprimer xinput1_4.dll: {}", e))?;
        removed.push("xinput1_4.dll");
    }
    if ost_path.exists() {
        fs::remove_file(&ost_path).map_err(|e| format!("Impossible de supprimer OpenSteamTool.dll: {}", e))?;
        removed.push("OpenSteamTool.dll");
    }

    if removed.is_empty() {
        Ok("Aucun hook OpenSteamTool n'était présent dans le répertoire Steam.".to_string())
    } else {
        Ok(format!("Hook désinstallé avec succès (supprimé : {}). Steam est revenu à l'état vanilla.", removed.join(", ")))
    }
}
