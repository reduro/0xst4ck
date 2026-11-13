pub mod dll_manager;
pub mod steam_signature_resolver;
pub mod loader;
pub mod overlay_inject;
pub mod lua_manager;
pub mod models;
pub mod scanner;
pub mod storage;
pub mod steam;
pub mod steam_importer;
pub mod workshop;
pub mod updater;
pub mod crash_handler;
pub mod discord_rpc;
pub mod achievements;

use models::{CrashReport, Game, NetworkRuntime, Settings, SteamSearchResult};
use std::fs;
use uuid::Uuid;

#[tauri::command]
fn get_games() -> Result<Vec<Game>, String> {
    Ok(storage::load_games())
}

#[tauri::command]
async fn add_game(
    name: String,
    path: String,
    arguments: Option<String>,
    steam_appid: Option<String>,
    banner_url: Option<String>,
) -> Result<Game, String> {
    let mut games = storage::load_games();

    let id = Uuid::new_v4().to_string();
    let runtime = scanner::scan_game_network_runtime(&path);
    let requires_photon = runtime.photon;

    let banner = if let (Some(appid), Some(url)) = (&steam_appid, &banner_url) {
        steam::cache_steam_banner(appid, url).await.ok()
    } else if let Some(appid) = &steam_appid {
        let default_url = format!(
            "https://cdn.akamai.steamstatic.com/steam/apps/{}/library_600x900.jpg",
            appid
        );
        steam::cache_steam_banner(appid, &default_url).await.ok()
    } else {
        None
    };

    let game = Game {
        id,
        name,
        path,
        arguments: arguments.unwrap_or_default(),
        steam_appid,
        banner,
        playtime: 0,
        requires_photon,
        last_played: 0,
        network_runtime: Some(runtime),
    };

    games.push(game.clone());
    storage::save_games(&games)?;
    Ok(game)
}

#[tauri::command]
fn update_game(game: Game) -> Result<Game, String> {
    let mut games = storage::load_games();
    if let Some(pos) = games.iter().position(|g| g.id == game.id) {
        games[pos] = game.clone();
        storage::save_games(&games)?;
        Ok(game)
    } else {
        Err("Jeu introuvable".to_string())
    }
}

#[tauri::command]
fn delete_game(id: String) -> Result<(), String> {
    let mut games = storage::load_games();
    games.retain(|g| g.id != id);
    storage::save_games(&games)
}

#[tauri::command]
fn delete_games(ids: Vec<String>) -> Result<(), String> {
    let mut games = storage::load_games();
    let set: std::collections::HashSet<String> = ids.into_iter().collect();
    games.retain(|g| !set.contains(&g.id));
    storage::save_games(&games)
}

#[tauri::command]
async fn launch_game(id: String, mode: Option<String>) -> Result<String, String> {
    let settings = storage::load_settings();
    let launch_mode = mode.unwrap_or(settings.default_launch_mode);
    loader::launch_game_dynamic(&id, &launch_mode)
}

#[tauri::command]
fn scan_game_runtime(path: String) -> Result<NetworkRuntime, String> {
    Ok(scanner::scan_game_network_runtime(&path))
}

#[tauri::command]
async fn search_steam(query: String) -> Result<Vec<SteamSearchResult>, String> {
    steam::search_steam_store(&query).await
}

#[tauri::command]
async fn search_steam_games(query: String) -> Result<Vec<SteamSearchResult>, String> {
    steam::search_steam_store(&query).await
}

#[tauri::command]
async fn download_steam_banner(appid: String, url: String) -> Result<String, String> {
    steam::cache_steam_banner(&appid, &url).await
}

#[tauri::command]
fn open_game_folder(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let folder = if p.is_file() {
        p.parent().ok_or("Impossible de trouver le dossier")?
    } else {
        p
    };

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Erreur explorer: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn import_steam_games() -> Result<usize, String> {
    steam_importer::import_steam_library_games().await
}

#[tauri::command]
fn get_detected_steam_path() -> Result<String, String> {
    let p = steam_importer::detect_steam_install_path();
    if p.exists() {
        Ok(p.to_string_lossy().to_string())
    } else {
        Err("Dossier Steam introuvable".to_string())
    }
}

#[tauri::command]
async fn generate_steam_game(appid: String) -> Result<String, String> {
    steam_importer::generate_steam_game_lua(&appid, None).await
}

#[tauri::command]
fn change_game_path(id: String, new_path: String) -> Result<Game, String> {
    let mut games = storage::load_games();
    if let Some(pos) = games.iter().position(|g| g.id == id) {
        games[pos].path = new_path;
        let rt = scanner::scan_game_network_runtime(&games[pos].path);
        games[pos].requires_photon = rt.photon;
        games[pos].network_runtime = Some(rt);
        storage::save_games(&games)?;
        Ok(games[pos].clone())
    } else {
        Err("Jeu introuvable".to_string())
    }
}

#[tauri::command]
fn get_opensteamtool_server() -> Result<String, String> {
    Ok(steam_importer::get_opensteamtool_config_server())
}

#[tauri::command]
fn set_opensteamtool_server(url: String) -> Result<(), String> {
    steam_importer::set_opensteamtool_config_server(&url).map(|_| ())
}

#[tauri::command]
fn get_settings() -> Result<Settings, String> {
    Ok(storage::load_settings())
}

#[tauri::command]
fn save_settings(settings: Settings) -> Result<(), String> {
    discord_rpc::set_language(&settings.language);
    storage::save_settings(&settings)
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(TABLE[(b0 >> 2) as usize] as char);
        result.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            result.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[tauri::command]
fn get_banner_image(filename: String) -> Result<String, String> {
    let banners_dir = storage::get_banners_dir();
    let file_path = banners_dir.join(filename);
    if !file_path.exists() {
        return Err("Image non trouvée".to_string());
    }

    match fs::read(&file_path) {
        Ok(bytes) => {
            let encoded = base64_encode(&bytes);
            Ok(format!("data:image/jpeg;base64,{}", encoded))
        }
        Err(e) => Err(format!("Erreur lecture image: {}", e)),
    }
}

#[tauri::command]
fn get_last_crash() -> Result<Option<CrashReport>, String> {
    Ok(crash_handler::get_last_crash())
}

#[tauri::command]
fn clear_last_crash() -> Result<(), String> {
    crash_handler::clear_last_crash();
    Ok(())
}

#[tauri::command]
fn verify_and_sync_dlls() -> Result<dll_manager::DllSyncResult, String> {
    Ok(dll_manager::verify_and_sync_all_dlls())
}

#[tauri::command]
fn get_opensteamtool_hook_status() -> Result<dll_manager::OpenSteamToolStatus, String> {
    Ok(dll_manager::get_opensteamtool_hook_status())
}

#[tauri::command]
fn uninstall_opensteamtool_hook() -> Result<String, String> {
    dll_manager::uninstall_opensteamtool_hook()
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_game_lua(appid: String) -> Result<lua_manager::LuaConfigFile, String> {
    lua_manager::read_game_lua(&appid)
}

#[tauri::command]
fn save_game_lua(file_path: String, content: String) -> Result<(), String> {
    lua_manager::save_game_lua(&file_path, &content)
}

#[tauri::command]
fn set_game_manifest(appid: String, depot_id: String, manifest_id: String) -> Result<String, String> {
    lua_manager::update_lua_manifest(&appid, &depot_id, &manifest_id)
}

#[tauri::command]
async fn check_launcher_update(repo: Option<String>) -> Result<updater::UpdateInfo, String> {
    updater::check_for_updates(repo).await
}

#[tauri::command]
async fn download_launcher_update(download_url: String) -> Result<String, String> {
    updater::download_and_run_installer(download_url).await
}

#[tauri::command]
fn exit_launcher() {
    std::process::exit(0);
}

#[tauri::command]
async fn get_game_achievements(game_id: String) -> Result<achievements::GameAchievementsSummary, String> {
    achievements::get_game_achievements_summary(&game_id).await
}

#[tauri::command]
async fn toggle_achievement(game_id: String, apiname: String, unlock: bool) -> Result<achievements::GameAchievementsSummary, String> {
    let games = storage::load_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("Jeu introuvable")?;
    let appid = game.steam_appid.as_deref().unwrap_or("480");
    if unlock {
        achievements::record_unlock(appid, &apiname);
        let _ = achievements::sync_achievement_to_steam(appid, &apiname);
    } else {
        achievements::record_lock(appid, &apiname);
        let _ = achievements::sync_clear_achievement_from_steam(appid, &apiname);
    }
    achievements::get_game_achievements_summary(&game_id).await
}

#[tauri::command]
async fn unlock_achievement_on_steam(game_id: String, apiname: String) -> Result<String, String> {
    let games = storage::load_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("Jeu introuvable")?;
    let appid = game.steam_appid.as_deref().unwrap_or("480");
    achievements::record_unlock(appid, &apiname);
    achievements::sync_achievement_to_steam(appid, &apiname)
}

#[tauri::command]
async fn clear_achievement_on_steam(game_id: String, apiname: String) -> Result<String, String> {
    let games = storage::load_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("Jeu introuvable")?;
    let appid = game.steam_appid.as_deref().unwrap_or("480");
    achievements::record_lock(appid, &apiname);
    achievements::sync_clear_achievement_from_steam(appid, &apiname)
}

#[tauri::command]
async fn unlock_all_achievements_on_steam(game_id: String) -> Result<achievements::GameAchievementsSummary, String> {
    let games = storage::load_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("Jeu introuvable")?;
    let appid = game.steam_appid.as_deref().unwrap_or("480");
    let summary = achievements::get_game_achievements_summary(&game_id).await?;
    let apinames: Vec<String> = summary.achievements.into_iter().map(|a| a.apiname).collect();
    achievements::record_unlock_all(appid, &apinames);
    let _ = achievements::sync_all_achievements_to_steam(appid, &apinames);
    achievements::get_game_achievements_summary(&game_id).await
}

#[tauri::command]
async fn reset_all_achievements(game_id: String) -> Result<achievements::GameAchievementsSummary, String> {
    let games = storage::load_games();
    let game = games.iter().find(|g| g.id == game_id).ok_or("Jeu introuvable")?;
    let appid = game.steam_appid.as_deref().unwrap_or("480");
    let summary = achievements::get_game_achievements_summary(&game_id).await?;
    let all_apinames: Vec<String> = summary.achievements.into_iter().map(|a| a.apiname).collect();
    achievements::record_reset_all(appid, &all_apinames);
    let _ = achievements::sync_clear_all_from_steam(appid, &all_apinames);
    achievements::get_game_achievements_summary(&game_id).await
}

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let settings = storage::load_settings();
            discord_rpc::set_language(&settings.language);
            discord_rpc::init_discord_rpc(settings.discord_client_id);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_games,
            add_game,
            update_game,
            delete_game,
            delete_games,
            launch_game,
            scan_game_runtime,
            search_steam,
            search_steam_games,
            download_steam_banner,
            open_game_folder,
            import_steam_games,
            get_detected_steam_path,
            generate_steam_game,
            change_game_path,
            get_opensteamtool_server,
            set_opensteamtool_server,
            get_settings,
            save_settings,
            get_banner_image,
            verify_and_sync_dlls,
            get_opensteamtool_hook_status,
            uninstall_opensteamtool_hook,
            open_external_url,
            get_game_lua,
            save_game_lua,
            set_game_manifest,
            check_launcher_update,
            download_launcher_update,
            exit_launcher,
            get_last_crash,
            clear_last_crash,
            get_game_achievements,
            toggle_achievement,
            unlock_achievement_on_steam,
            clear_achievement_on_steam,
            unlock_all_achievements_on_steam,
            reset_all_achievements
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
