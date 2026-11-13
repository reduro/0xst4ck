use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvidence {
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub runtime: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkRuntime {
    pub runtime: String,
    pub photon: bool,
    pub eos: bool,
    #[serde(default)]
    pub steamworks: bool,
    #[serde(default)]
    pub requirements_summary: String,
    pub evidence: Vec<NetworkEvidence>,
    pub scanned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub arguments: String,
    #[serde(default)]
    pub steam_appid: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub playtime: u64,
    #[serde(default)]
    pub requires_photon: bool,
    #[serde(default)]
    pub last_played: u64,
    #[serde(default)]
    pub network_runtime: Option<NetworkRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GamesContainer {
    pub games: Vec<Game>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_lang")]
    pub language: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub steam_path: Option<String>,
    #[serde(default = "default_spacewar")]
    pub default_launch_mode: String,
    #[serde(default = "default_true")]
    pub auto_unpack: bool,
    #[serde(default = "default_true")]
    pub auto_copy_hooks: bool,
    #[serde(default = "default_repo")]
    pub github_repo: String,
    #[serde(default)]
    pub discord_client_id: Option<String>,
}

fn default_lang() -> String {
    "fr".to_string()
}

fn default_spacewar() -> String {
    "spacewar".to_string()
}

fn default_true() -> bool {
    true
}

fn default_repo() -> String {
    "reduro/0xst4ck".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "fr".to_string(),
            theme: "dark".to_string(),
            steam_path: None,
            default_launch_mode: "spacewar".to_string(),
            auto_unpack: true,
            auto_copy_hooks: true,
            github_repo: "reduro/0xst4ck".to_string(),
            discord_client_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamSearchResult {
    pub appid: u64,
    pub name: String,
    pub header_image: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub game_id: String,
    pub game_name: String,
    pub exe_path: String,
    pub exit_code: i32,
    pub exit_code_hex: String,
    pub elapsed_secs: f64,
    pub launch_mode: String,
    pub error_category: String,
    pub error_description: String,
    pub recommendations: Vec<String>,
    pub recent_logs: Option<String>,
    pub log_source: Option<String>,
    pub timestamp: u64,
    pub discord_markdown: String,
}
