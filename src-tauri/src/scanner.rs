use crate::models::{NetworkEvidence, NetworkRuntime};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const CHUNK_SIZE: usize = 256 * 1024; // 512 KB chunk
const MAX_SCAN_BYTES: usize = 4 * 1024 * 1024; // 32 MB max scan per binary/metadata

const PHOTON_FILE_MARKERS: &[&str] = &[
    "photon3unity3d.dll",
    "photonrealtime.dll",
    "photonnetwork.dll",
    "exitgames.client.photon.dll",
    "photonunitynetworking.dll",
    "photonunitynetworking.utilities.dll",
    "photonvoice.dll",
    "photonvoice.api.dll",
    "photonvoice.pun.dll",
    "photonvoice.fusion.dll",
    "photonchat.dll",
    "photonbolt.dll",
    "photoncryptoplugin.dll",
    "opus_egpv.dll",
    "fusion.runtime.dll",
    "fusion.realtime.dll",
    "fusion.sockets.dll",
    "fusion.addressables.dll",
    "fusionsdk.dll",
    "nanosockets.dll",
    "quantum.core.dll",
    "quantum.deterministic.dll",
];

const EOS_FILE_MARKERS: &[&str] = &[
    "eossdk-win64-shipping.dll",
    "eossdk-win32-shipping.dll",
    "eossdk.dll",
    "epiconlineservices.dll",
    "eos-sdk-plugin.dll",
];

const STEAMWORKS_FILE_MARKERS: &[&str] = &[
    "stubdrm64.dll",
    "stubdrm.dll",
    "onlinefix64.dll",
    "onlinefix.dll",
    "spacewar.dll",
    "steam_api64.dll",
    "steam_api.dll",
];

const PHOTON_TEXT_MARKERS: &[&[u8]] = &[
    b"exitgames.client.photon",
    b"photon3unity3d",
    b"photonrealtime",
    b"photonengine",
    b"ns.exitgames.com",
    b"ns.photonengine.io",
    b"photon.realtime",
    b"photon.pun",
    b"photon.voice",
    b"photon.fusion",
    b"fusion.sockets",
    b"fusion.protocol",
    b"networkrunner",
    b"startgameargs",
    b"photonnetwork",
    b"loadbalancingclient",
    b"punvoiceclient",
    b"photonclient",
    b"fusion.runtime",
    b"fusion.realtime",
    b"mimick.fusion",
    b"nanosockets",
];

const EOS_TEXT_MARKERS: &[&[u8]] = &[
    b"epiconlineservices",
    b"eossdk",
    b"eos_productid",
    b"eos_auth",
    b"onlinesubsystemeos",
];

const STEAM_TEXT_MARKERS: &[&[u8]] = &[
    b"onlinesubsystemsteam",
    b"isteammatchmaking",
    b"isteamnetworking",
    b"createlobby",
    b"joinlobby",
    b"sendp2ppacket",
    b"readp2ppacket",
    b"acceptp2psession",
    b"steamp2p",
];

pub fn find_game_root(exe_path: &Path) -> PathBuf {
    let parent = match exe_path.parent() {
        Some(p) => p.to_path_buf(),
        None => return exe_path.to_path_buf(),
    };

    // If game is in a Steam library (steamapps/common/<GameFolder>/...), return <GameFolder>
    let mut check = parent.clone();
    while let Some(up) = check.parent() {
        if up.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some("common".to_string()) {
            return check;
        }
        check = up.to_path_buf();
    }

    // Fallback: strip standard engine subfolders (win64, binaries, shipping, etc.)
    let mut curr = parent;
    for _ in 0..6 {
        let name = curr.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if name == "win64" || name == "x64" || name == "bin" || name == "binaries" || name == "shipping" || name == "release" {
            if let Some(up) = curr.parent() {
                curr = up.to_path_buf();
                continue;
            }
        }
        break;
    }
    curr
}

pub fn scan_game_network_runtime(game_path: &str) -> NetworkRuntime {
    let path = Path::new(game_path);
    if !path.exists() {
        return NetworkRuntime {
            runtime: "none".to_string(),
            photon: false,
            eos: false,
            steamworks: false,
            requirements_summary: "Fichier exécutable introuvable".to_string(),
            evidence: Vec::new(),
            scanned: false,
        };
    }

    let root = find_game_root(path);

    let mut is_photon = false;
    let mut is_eos = false;
    let mut is_steamworks = false;
    let mut evidence = Vec::new();

    // Priority buckets for deep inspection:
    // 1. global-metadata.dat & Assembly-CSharp.dll (fastest & most reliable for Unity IL2CPP / Mono)
    // 2. GameAssembly.dll & Shipping .exe
    let mut high_priority_bins: Vec<PathBuf> = Vec::new();
    let mut low_priority_bins: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(&root).max_depth(4).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_lowercase();
        let relative_path = entry
            .path()
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_name.clone());

        // File markers check
        if PHOTON_FILE_MARKERS.contains(&file_name.as_str())
            || (file_name.starts_with("photon") && file_name.ends_with(".dll"))
            || (file_name.starts_with("fusion.") && file_name.ends_with(".dll"))
        {
            is_photon = true;
            evidence.push(NetworkEvidence {
                evidence_type: "file".to_string(),
                runtime: "photon".to_string(),
                path: relative_path.clone(),
            });
        }

        if EOS_FILE_MARKERS.contains(&file_name.as_str()) {
            is_eos = true;
            evidence.push(NetworkEvidence {
                evidence_type: "file".to_string(),
                runtime: "eos".to_string(),
                path: relative_path.clone(),
            });
        }

        if STEAMWORKS_FILE_MARKERS.contains(&file_name.as_str()) {
            is_steamworks = true;
            evidence.push(NetworkEvidence {
                evidence_type: "file".to_string(),
                runtime: "steamworks".to_string(),
                path: relative_path.clone(),
            });
        }

        if file_name == "global-metadata.dat" || file_name == "scriptingassemblies.json" || file_name == "assembly-csharp.dll" || file_name == "assembly-csharp-firstpass.dll" {
            high_priority_bins.push(entry.path().to_path_buf());
        } else {
            let is_target_or_shipping_exe = file_name.ends_with(".exe")
                && !file_name.contains("crash")
                && !file_name.contains("redist")
                && !file_name.contains("prereq")
                && !file_name.contains("unity")
                && (file_name.contains("shipping") || relative_path.to_lowercase().contains("binaries"));

            if file_name == "gameassembly.dll" || is_target_or_shipping_exe {
                low_priority_bins.push(entry.path().to_path_buf());
            }
        }
    }

    if path.is_file() && !low_priority_bins.contains(&path.to_path_buf()) {
        low_priority_bins.push(path.to_path_buf());
    }

    let mut binary_candidates = high_priority_bins;
    binary_candidates.extend(low_priority_bins);

    // Deep text inspection on key binaries/metadata if Photon or EOS not yet confirmed by filename
    if !is_photon || !is_eos {
        for bin_path in binary_candidates.into_iter().take(4) {
            if is_photon && is_eos {
                break;
            }

            if let Ok(mut f) = File::open(&bin_path) {
                let file_label = bin_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let mut total_scanned = 0usize;
                let mut buffer = vec![0u8; CHUNK_SIZE];
                let mut overlap = Vec::new();

                while total_scanned < MAX_SCAN_BYTES {
                    let bytes_read = match f.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(_) => break,
                    };

                    let mut current_window = overlap.clone();
                    current_window.extend_from_slice(&buffer[..bytes_read]);
                    let slice_lower = current_window.to_ascii_lowercase();

                    if !is_photon {
                        for marker in PHOTON_TEXT_MARKERS {
                            if slice_lower.windows(marker.len()).any(|w| w == *marker) {
                                is_photon = true;
                                evidence.push(NetworkEvidence {
                                    evidence_type: "marker".to_string(),
                                    runtime: "photon".to_string(),
                                    path: file_label.clone(),
                                });
                                break;
                            }
                        }
                    }

                    if !is_eos {
                        for marker in EOS_TEXT_MARKERS {
                            if slice_lower.windows(marker.len()).any(|w| w == *marker) {
                                is_eos = true;
                                evidence.push(NetworkEvidence {
                                    evidence_type: "marker".to_string(),
                                    runtime: "eos".to_string(),
                                    path: file_label.clone(),
                                });
                                break;
                            }
                        }
                    }

                    if !is_steamworks {
                        for marker in STEAM_TEXT_MARKERS {
                            if slice_lower.windows(marker.len()).any(|w| w == *marker) {
                                is_steamworks = true;
                                evidence.push(NetworkEvidence {
                                    evidence_type: "marker".to_string(),
                                    runtime: "steamworks".to_string(),
                                    path: file_label.clone(),
                                });
                                break;
                            }
                        }
                    }

                    if is_photon && is_eos {
                        break;
                    }

                    total_scanned += bytes_read;
                    let overlap_len = 64.min(bytes_read);
                    overlap = buffer[bytes_read - overlap_len..bytes_read].to_vec();
                }
            }
        }
    }

    let (runtime, requirements_summary) = if is_photon && is_eos {
        ("photon_eos".to_string(), "Multijoueur Photon + Epic (EOS)".to_string())
    } else if is_photon {
        ("photon".to_string(), "Multijoueur Photon (Relais Loader)".to_string())
    } else if is_eos {
        ("eos".to_string(), "Multijoueur Epic (OnlineFix EOS)".to_string())
    } else {
        ("spacewar".to_string(), "Steam Spacewar (AppID 480)".to_string())
    };

    NetworkRuntime {
        runtime,
        photon: is_photon,
        eos: is_eos,
        steamworks: is_steamworks,
        requirements_summary,
        evidence,
        scanned: true,
    }
}
