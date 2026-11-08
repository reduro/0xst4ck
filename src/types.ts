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
