use crate::models::CrashReport;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::WalkDir;

static LAST_CRASH: Mutex<Option<CrashReport>> = Mutex::new(None);

pub fn set_last_crash(report: CrashReport) {
    if let Ok(mut lock) = LAST_CRASH.lock() {
        *lock = Some(report);
    }
}

pub fn get_last_crash() -> Option<CrashReport> {
    LAST_CRASH.lock().ok().and_then(|lock| lock.clone())
}

pub fn clear_last_crash() {
    if let Ok(mut lock) = LAST_CRASH.lock() {
        *lock = None;
    }
}

pub fn evaluate_crash(
    game_id: &str,
    game_name: &str,
    exe_path: &str,
    launch_mode: &str,
    exit_code: i32,
    elapsed_secs: f64,
) -> Option<CrashReport> {
    // Only process early closures (< 5.0 seconds) that indicate a startup failure
    if elapsed_secs >= 5.0 {
        return None;
    }

    // A process exiting early with code != 0, or code == 0 in under 2 seconds (silent launcher crash/re-launch)
    let is_suspicious = exit_code != 0 || elapsed_secs < 2.0;
    if !is_suspicious {
        return None;
    }

    let ucode = exit_code as u32;
    let exit_code_hex = format!("0x{:08X}", ucode);

    let (category, desc, recos) = match ucode {
        0xC0000005 => (
            "STATUS_ACCESS_VIOLATION".to_string(),
            "Violation d'accès mémoire (0xC0000005) : le jeu a tenté de lire ou d'écrire dans une zone mémoire protégée.".to_string(),
            vec![
                "Vérifier qu'aucune DLL externe incompatible (ex: OnlineFix.dll dans un jeu Photon) n'est restée dans le dossier du jeu.".to_string(),
                "Désactiver temporairement les overlays tiers agressifs (RivaTuner, GeForce Experience, MSI Afterburner).".to_string(),
                "Mettre à jour les pilotes de la carte graphique (GPU).".to_string(),
                "Exécuter le launcher ou le jeu en mode Administrateur si le jeu tente d'écrire dans Program Files.".to_string(),
            ],
        ),
        0xC0000135 => (
            "STATUS_DLL_NOT_FOUND".to_string(),
            "DLL système requise introuvable (0xC0000135) : une bibliothèque indispensable au jeu manque dans Windows.".to_string(),
            vec![
                "Installer le package complet Microsoft Visual C++ Redistributable (2015-2022) x64 et x86.".to_string(),
                "Installer le package DirectX End-User Runtimes (June 2010).".to_string(),
                "Installer Microsoft .NET Desktop Runtime (6.0 ou 8.0) si le jeu utilise un wrapper .NET.".to_string(),
            ],
        ),
        0xC000007B => (
            "STATUS_INVALID_IMAGE_FORMAT".to_string(),
            "Incompatibilité de format binaire (0xC000007B) : conflit d'architecture 32-bit contre 64-bit.".to_string(),
            vec![
                "Une DLL 32-bit a été injectée ou placée dans un jeu 64-bit (ou vice versa).".to_string(),
                "Vérifier la version de steam_api.dll / steam_api64.dll dans le dossier du jeu.".to_string(),
                "Réinstaller les redistribuables Visual C++ x64 et x86.".to_string(),
            ],
        ),
        0xC0000409 => (
            "STATUS_STACK_BUFFER_OVERRUN".to_string(),
            "Dépassement de tampon de pile (0xC0000409) : arrêt de sécurité déclenché par Windows.".to_string(),
            vec![
                "Conflit causé par un antivirus ou un système anti-cheat résiduel au démarrage.".to_string(),
                "Ajouter le dossier du jeu aux exclusions de Windows Defender.".to_string(),
            ],
        ),
        0xC00000FD => (
            "STATUS_STACK_OVERFLOW".to_string(),
            "Débordement de pile (0xC00000FD) : récursion infinie ou corruption dans les scripts internes du jeu.".to_string(),
            vec![
                "Vérifier l'intégrité des fichiers du jeu ou supprimer les fichiers de configuration corrompus dans AppData.".to_string(),
            ],
        ),
        0xC000001D => (
            "STATUS_ILLEGAL_INSTRUCTION".to_string(),
            "Instruction processeur non supportée (0xC000001D) : le jeu nécessite un jeu d'instructions (ex: AVX/AVX2) non géré par le CPU.".to_string(),
            vec![
                "Votre processeur ne supporte pas certaines instructions vectorielles requises par le jeu.".to_string(),
            ],
        ),
        1 => (
            "ERROR_GENERAL_FAILURE".to_string(),
            "Arrêt immédiat avec code d'erreur général (Code 1).".to_string(),
            vec![
                "Vérifier la présence et la validité du fichier steam_appid.txt.".to_string(),
                "S'assurer que le client Steam est lancé en arrière-plan si le jeu utilise l'émulation Spacewar (AppID 480).".to_string(),
                "Vérifier si un fichier de logs a été généré dans le dossier du jeu.".to_string(),
            ],
        ),
        0 if elapsed_secs < 2.0 => (
            "EARLY_SILENT_EXIT".to_string(),
            "Fermeture immédiate sans code d'erreur (Code 0 en moins de 2 secondes).".to_string(),
            vec![
                "Le binaire sélectionné est peut-être un lanceur préliminaire qui s'arrête après avoir démarré un autre exécutable (ex: Binaries/Win64/Shipping.exe).".to_string(),
                "Vérifier dans les paramètres du jeu si un autre .exe situé dans les sous-dossiers est le véritable exécutable du jeu.".to_string(),
                "Vérifier si le jeu nécessite des arguments de lancement spécifiques (-force-d3d11, -windowed, etc.).".to_string(),
            ],
        ),
        _ => (
            format!("STATUS_UNKNOWN_{}", exit_code_hex),
            format!("Le jeu s'est arrêté avec le code de sortie non documenté {} après {:.1}s.", exit_code_hex, elapsed_secs),
            vec![
                "Consulter les logs récents ci-dessous pour identifier l'origine du crash.".to_string(),
                "Copier le rapport pour ouvrir un ticket sur le serveur Discord d'assistance.".to_string(),
            ],
        ),
    };

    let (recent_logs, log_source) = find_recent_game_logs(exe_path, launch_mode);

    let mut discord = String::new();
    discord.push_str("```yaml\n");
    discord.push_str("=== RAPPORT DE CRASH CLEMTOUT LAUNCHER ===\n");
    discord.push_str(&format!("Jeu: {}\n", game_name));
    discord.push_str(&format!("Mode de lancement: {}\n", launch_mode));
    discord.push_str(&format!("Code de sortie: {} ({})\n", exit_code_hex, category));
    discord.push_str(&format!("Duree avant crash: {:.2}s\n", elapsed_secs));
    discord.push_str(&format!("Chemin exe: {}\n", exe_path));
    discord.push_str(&format!("Date: {}\n\n", chrono::Utc::now().to_rfc3339()));
    discord.push_str("Diagnostic:\n");
    discord.push_str(&format!("  {}\n\n", desc));
    discord.push_str("Recommandations:\n");
    for r in &recos {
        discord.push_str(&format!("  - {}\n", r));
    }
    if let (Some(logs), Some(src)) = (&recent_logs, &log_source) {
        discord.push_str(&format!("\nDerniers logs (source: {}):\n", src));
        for line in logs.lines() {
            discord.push_str(&format!("  {}\n", line));
        }
    }
    discord.push_str("```");

    let report = CrashReport {
        game_id: game_id.to_string(),
        game_name: game_name.to_string(),
        exe_path: exe_path.to_string(),
        exit_code,
        exit_code_hex,
        elapsed_secs,
        launch_mode: launch_mode.to_string(),
        error_category: category,
        error_description: desc,
        recommendations: recos,
        recent_logs,
        log_source,
        timestamp: chrono::Utc::now().timestamp() as u64,
        discord_markdown: discord,
    };

    set_last_crash(report.clone());
    Some(report)
}

fn find_recent_game_logs(exe_path: &str, launch_mode: &str) -> (Option<String>, Option<String>) {
    let p = Path::new(exe_path);
    let working_dir = p.parent().unwrap_or(p);

    // 1. If Photon mode, check SteamBridge_log.txt first
    if launch_mode == "photon" {
        let sb_log = Path::new(r"C:\Users\Public\SteamBridge_log.txt");
        if sb_log.exists() {
            if let Ok(content) = fs::read_to_string(sb_log) {
                let tail = get_tail_lines(&content, 20);
                if !tail.is_empty() {
                    return (Some(tail), Some("SteamBridge_log.txt".to_string()));
                }
            }
        }
    }

    // 2. Check local working directory for output_log.txt or crash.log
    let local_candidates = [
        working_dir.join("output_log.txt"),
        working_dir.join("crash.log"),
        working_dir.join("crashpad.log"),
    ];
    for cand in &local_candidates {
        if cand.exists() {
            if let Ok(content) = fs::read_to_string(cand) {
                let tail = get_tail_lines(&content, 25);
                if !tail.is_empty() {
                    return (Some(tail), Some(cand.file_name().unwrap_or_default().to_string_lossy().to_string()));
                }
            }
        }
    }

    // 3. Check Unity LocalLow Player.log in AppData\LocalLow
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let locallow = PathBuf::from(user_profile).join("AppData").join("LocalLow");
        if locallow.exists() {
            // Find most recently modified Player.log in LocalLow
            let mut best_log: Option<(PathBuf, std::time::SystemTime)> = None;
            for entry in WalkDir::new(&locallow).max_depth(3).into_iter().filter_map(|e| e.ok()) {
                if entry.file_name().to_string_lossy().to_lowercase() == "player.log" {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            if best_log.as_ref().map_or(true, |(_, best_time)| mtime > *best_time) {
                                best_log = Some((entry.path().to_path_buf(), mtime));
                            }
                        }
                    }
                }
            }

            if let Some((log_path, mtime)) = best_log {
                if let Ok(elapsed) = mtime.elapsed() {
                    // Only use if modified in the last 15 minutes
                    if elapsed.as_secs() < 900 {
                        if let Ok(content) = fs::read_to_string(&log_path) {
                            let tail = get_tail_lines(&content, 25);
                            if !tail.is_empty() {
                                return (Some(tail), Some(format!("Unity Player.log ({})", log_path.file_name().unwrap_or_default().to_string_lossy())));
                            }
                        }
                    }
                }
            }
        }
    }

    (None, None)
}

fn get_tail_lines(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}
