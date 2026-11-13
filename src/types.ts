export interface NetworkEvidence {
  type: string;
  runtime: string;
  path: string;
}

export interface NetworkRuntime {
  runtime: string;
  photon: boolean;
  eos: boolean;
  steamworks?: boolean;
  requirements_summary?: string;
  evidence: NetworkEvidence[];
  scanned: boolean;
}

export interface Game {
  id: string;
  name: string;
  path: string;
  arguments: string;
  steam_appid?: string;
  banner?: string;
  playtime: number;
  requires_photon: boolean;
  last_played: number;
  network_runtime?: NetworkRuntime;
}

export interface Settings {
  language: string;
  theme: string;
  steam_path?: string;
  default_launch_mode: string;
  auto_unpack: boolean;
  auto_copy_hooks: boolean;
  github_repo?: string;
  discord_client_id?: string;
}

export interface Achievement {
  apiname: string;
  name: string;
  description: string;
  icon: string;
  icon_gray: string;
  unlocked: boolean;
  unlocked_at?: number;
}

export interface GameAchievementsSummary {
  appid: string;
  total: number;
  unlocked: number;
  percentage: number;
  achievements: Achievement[];
}

export interface SteamSearchResult {
  appid: number;
  name: string;
  header_image: string;
}

export interface DllStatusItem {
  name: string;
  path: string;
  expected_hash: string;
  current_hash: string | null;
  status: string; // "synced" | "updated" | "failed" | "locked"
}

export interface DllSyncResult {
  all_synced: boolean;
  items: DllStatusItem[];
  message: string;
}

export interface LuaConfigFile {
  file_path: string;
  content: string;
  appid: string;
}

export interface UpdateInfo {
  update_available: boolean;
  current_version: string;
  latest_version: string;
  release_name: string;
  changelog: string;
  download_url?: string;
  asset_name?: string;
}


export interface CrashReport {
  game_id: string;
  game_name: string;
  exe_path: string;
  exit_code: number;
  exit_code_hex: string;
  elapsed_secs: number;
  launch_mode: string;
  error_category: string;
  error_description: string;
  recommendations: string[];
  recent_logs?: string;
  log_source?: string;
  timestamp: number;
  discord_markdown: string;
}

export interface OpenSteamToolStatus {
  installed: boolean;
  hook_type: string;
  dll_hash: string | null;
  steam_path: string;
  fallback_enabled: boolean;
}
