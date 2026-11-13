use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde_json::json;

pub const DEFAULT_DISCORD_CLIENT_ID: &str = "1425874117537828915";

#[derive(Debug, Clone)]
pub enum DiscordActivity {
    Idle,
    InGame {
        game_name: String,
        start_time: u64,
    },
    Custom {
        details: String,
        state: String,
        start_time: Option<u64>,
    },
    Clear,
    Refresh,
}

static DISCORD_TX: OnceLock<Sender<DiscordActivity>> = OnceLock::new();
static IS_ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_language(lang: &str) {
    IS_ENGLISH.store(lang.trim().eq_ignore_ascii_case("en"), Ordering::Relaxed);
    update_activity(DiscordActivity::Refresh);
}

pub fn init_discord_rpc(custom_client_id: Option<String>) {
    if DISCORD_TX.get().is_some() {
        return;
    }

    let (tx, rx) = channel::<DiscordActivity>();
    let _ = DISCORD_TX.set(tx.clone());

    let client_id = custom_client_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_DISCORD_CLIENT_ID.to_string());

    thread::Builder::new()
        .name("discord_rpc_worker".into())
        .spawn(move || {
            worker_loop(rx, client_id);
        })
        .expect("failed to spawn discord rpc worker");

    // Default activity
    let _ = tx.send(DiscordActivity::Idle);
}

pub fn update_activity(activity: DiscordActivity) {
    if let Some(tx) = DISCORD_TX.get() {
        let _ = tx.send(activity);
    }
}

pub fn set_in_game(game_name: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    update_activity(DiscordActivity::InGame {
        game_name: game_name.to_string(),
        start_time: now,
    });
}

pub fn set_idle() {
    update_activity(DiscordActivity::Idle);
}

struct DiscordPipe {
    pipe: std::fs::File,
}

impl DiscordPipe {
    fn try_connect(client_id: &str) -> Option<Self> {
        for i in 0..10 {
            let path = format!(r"\\.\pipe\discord-ipc-{}", i);
            if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(&path) {
                // Handshake Opcode 0
                let handshake = json!({
                    "v": 1,
                    "client_id": client_id
                })
                .to_string();
                let payload_bytes = handshake.as_bytes();
                let mut header = [0u8; 8];
                header[0..4].copy_from_slice(&0u32.to_le_bytes());
                header[4..8].copy_from_slice(&(payload_bytes.len() as u32).to_le_bytes());

                if file.write_all(&header).is_err() || file.write_all(payload_bytes).is_err() {
                    continue;
                }

                // Read reply header
                let mut rep_hdr = [0u8; 8];
                if file.read_exact(&mut rep_hdr).is_err() {
                    continue;
                }
                let len = u32::from_le_bytes([rep_hdr[4], rep_hdr[5], rep_hdr[6], rep_hdr[7]]) as usize;
                let mut buf = vec![0u8; len];
                if file.read_exact(&mut buf).is_err() {
                    continue;
                }

                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&buf) {
                    if val.get("code").is_some() && val["code"].as_i64() == Some(4000) {
                        eprintln!("[discord_rpc] Invalid client ID: {}", client_id);
                        return None;
                    }
                }

                return Some(DiscordPipe { pipe: file });
            }
        }
        None
    }

    fn send_activity(&mut self, activity: &DiscordActivity) -> Result<(), std::io::Error> {
        let en = IS_ENGLISH.load(Ordering::Relaxed);
        let (details, state, start_time) = match activity {
            DiscordActivity::Idle => (
                if en { "In the launcher" } else { "Dans le launcher" }.to_string(),
                if en { "Browsing game library" } else { "Parcourt sa bibliothèque" }.to_string(),
                None,
            ),
            DiscordActivity::InGame { game_name, start_time } => (
                game_name.clone(),
                if en { "In-game (Spacewar P2P)" } else { "En jeu (Spacewar P2P)" }.to_string(),
                Some(*start_time),
            ),
            DiscordActivity::Custom { details, state, start_time } => (
                details.clone(),
                state.clone(),
                *start_time,
            ),
            DiscordActivity::Clear => {
                let frame = json!({
                    "cmd": "SET_ACTIVITY",
                    "args": {
                        "pid": std::process::id(),
                        "activity": serde_json::Value::Null
                    },
                    "nonce": uuid::Uuid::new_v4().to_string()
                })
                .to_string();
                return self.send_frame(&frame);
            }
            DiscordActivity::Refresh => return Ok(()),
        };

        let mut act_json = json!({
            "details": details,
            "state": state,
            "assets": {
                "large_image": "clemtout_logo",
                "large_text": "Clemtout Launcher v2",
                "small_image": "steam_logo",
                "small_text": "Steamworks Spacewar"
            }
        });

        if let Some(st) = start_time {
            act_json["timestamps"] = json!({
                "start": st
            });
        }

        let frame = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": act_json
            },
            "nonce": uuid::Uuid::new_v4().to_string()
        })
        .to_string();

        self.send_frame(&frame)
    }

    fn send_frame(&mut self, payload: &str) -> Result<(), std::io::Error> {
        let bytes = payload.as_bytes();
        let mut header = [0u8; 8];
        header[0..4].copy_from_slice(&1u32.to_le_bytes()); // Opcode 1 = FRAME
        header[4..8].copy_from_slice(&(bytes.len() as u32).to_le_bytes());

        self.pipe.write_all(&header)?;
        self.pipe.write_all(bytes)?;
        self.pipe.flush()?;

        // Read response
        let mut rep_hdr = [0u8; 8];
        self.pipe.read_exact(&mut rep_hdr)?;
        let len = u32::from_le_bytes([rep_hdr[4], rep_hdr[5], rep_hdr[6], rep_hdr[7]]) as usize;
        let mut buf = vec![0u8; len];
        self.pipe.read_exact(&mut buf)?;

        Ok(())
    }
}

fn worker_loop(rx: Receiver<DiscordActivity>, client_id: String) {
    let mut current_pipe: Option<DiscordPipe> = None;
    let mut last_activity: Option<DiscordActivity> = None;

    loop {
        // Wait for next activity command or timeout after 15s to check pipe health
        let msg = match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(DiscordActivity::Refresh) => {
                if let Some(act) = &last_activity {
                    act.clone()
                } else {
                    continue;
                }
            }
            Ok(act) => {
                last_activity = Some(act.clone());
                act
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(act) = &last_activity {
                    act.clone()
                } else {
                    continue;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if current_pipe.is_none() {
            current_pipe = DiscordPipe::try_connect(&client_id);
        }

        if let Some(pipe) = &mut current_pipe {
            if let Err(e) = pipe.send_activity(&msg) {
                eprintln!("[discord_rpc] Error sending activity: {}, reconnecting", e);
                current_pipe = None;
            }
        }
    }
}
