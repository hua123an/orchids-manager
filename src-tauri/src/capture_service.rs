use crate::models::{Account, CookieData, UserInfo};
use crate::store::AccountManagerState;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

use base64::{engine::general_purpose, Engine as _};

// Helper: Real Orchids App Data (for cleaning)
pub fn get_orchids_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        Path::new(&home).join("Library/Application Support/Orchids")
    }
    #[cfg(not(target_os = "macos"))]
    {
        PathBuf::from(".")
    }
}

pub fn get_orchids_cookie_path() -> PathBuf {
    get_orchids_data_dir().join("Cookies")
}

// Helper: Shared Manager Data (for capturing/restoring)
pub fn get_manager_shared_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        Path::new(&home).join("Library/Application Support/OrchidsManager")
    }
    #[cfg(not(target_os = "macos"))]
    {
        PathBuf::from(".")
    }
}

pub struct CaptureService {
    is_running: Arc<Mutex<bool>>,
}

impl CaptureService {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start_monitoring(&self, app_handle: AppHandle) {
        let is_running = self.is_running.clone();
        let mut running = is_running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;
        drop(running);

        thread::spawn(move || {
            let cookie_path = get_orchids_cookie_path();
            let _ = app_handle.emit(
                "debug-log",
                format!("Monitoring Cookies at: {:?}", cookie_path),
            );

            // Keep track of the last captured ID to avoid log spam,
            // but we still want to update if credits change, so we might check periodically.
            let mut last_captured_id = String::new();

            loop {
                // Check stop
                if let Ok(running) = is_running.lock() {
                    if !*running {
                        break;
                    }
                }

                if cookie_path.exists() {
                    match process_cookies(&cookie_path) {
                        Ok(Some(account)) => {
                            let state = app_handle.state::<AccountManagerState>();
                            let acc_id = account.id.clone();

                            // Try to add/update
                            if let Ok(_) = state.add_account(account.clone()) {
                                if last_captured_id != acc_id {
                                    let _ = app_handle.emit(
                                        "debug-log",
                                        format!("Auto-Captured Account: {}", acc_id),
                                    );

                                    // Emit success to stop automation UI
                                    let _ = app_handle.emit(
                                        "register_success",
                                        serde_json::json!({
                                            "email": account.email.clone().unwrap_or_default(),
                                            "password": "Captured via DB"
                                        }),
                                    );

                                    last_captured_id = acc_id;
                                }
                            }
                        }
                        Ok(None) => {
                            // No session in DB (logged out?)
                        }
                        Err(e) => {
                            // Suppress excessive error logs for locked DBs etc
                            // let _ = app_handle.emit("debug-log", format!("Capture Error: {}", e));
                        }
                    }
                }

                // Poll every 3 seconds
                thread::sleep(Duration::from_secs(3));
            }
            let _ = app_handle.emit("debug-log", "Monitoring thread exited");
        });
    }

    pub fn stop(&self) {
        let mut running = self.is_running.lock().unwrap();
        *running = false;
    }
}

// Kept for backward compat or manual fetches if needed
pub fn fetch_fresh_credits(user_id: &str, token: &str) -> Result<i64, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client.get(format!("https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/{}", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("API Error: {}", resp.status()));
    }

    let profile: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    profile["credits"]
        .as_i64()
        .ok_or("No credits field".to_string())
}

pub fn get_user_id_from_token(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload_segment = parts[1];
    let mut padded = payload_segment.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    let normalized = padded.replace("-", "+").replace("_", "/");

    if let Ok(decoded) = general_purpose::STANDARD.decode(normalized) {
        if let Ok(json_str) = String::from_utf8(decoded) {
            if let Ok(claims) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) {
                    return Some(sub.to_string());
                }
            }
        }
    }
    None
}

pub fn process_cookies(path: &Path) -> Result<Option<Account>, String> {
    // 1. Try reading the SQLite Cookies file from the Orchids App
    if let Ok(cookies) = crate::cookie_reader::read_cookies_from_db(path) {
        if let Some(session_cookie) = cookies.iter().find(|c| c.name == "__session") {
            let token = &session_cookie.value;

            // 2. Decode JWT to get User ID
            if let Some(user_id) = get_user_id_from_token(token) {
                // 3. Fetch Full Profile
                let client = reqwest::blocking::Client::new();
                let resp = client.get(format!("https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/{}", user_id))
                     .header("Authorization", format!("Bearer {}", token))
                     .send()
                     .map_err(|e| e.to_string())?;

                if resp.status().is_success() {
                    if let Ok(profile) = resp.json::<serde_json::Value>() {
                        let credits = profile["credits"].as_i64().unwrap_or(0);
                        let plan = profile["plan"].as_str().unwrap_or("Free").to_string();
                        let email = profile["email"].as_str().unwrap_or("").to_string();
                        let image = profile["imageUrl"].as_str().map(|s| s.to_string());
                        let full_name = profile["firstName"].as_str().unwrap_or("User").to_string();

                        let new_account = Account {
                            id: user_id.clone(),
                            display_name: full_name.clone(),
                            email: Some(email.clone()),
                            password: None, // Captured from session, password unknown
                            avatar_url: image.clone(),
                            last_active_at: Some(
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                            ),
                            user_info: Some(UserInfo {
                                id: None,
                                user_id: user_id.clone(),
                                full_name: Some(full_name),
                                email: Some(email),
                                image_url: image,
                                plan: Some(plan),
                                credits: Some(credits),
                            }),
                            cookies: cookies.clone(), // Save all cookies
                        };
                        return Ok(Some(new_account));
                    }
                }
            }
        }
    }

    // Fallback: Check JSON file (if manual import fails to read DB, maybe user wants to load last auto-captured?)
    let shared_dir = get_manager_shared_dir();
    let captured_file = shared_dir.join("captured_session.json");
    if captured_file.exists() {
        let content = fs::read_to_string(&captured_file).map_err(|e| e.to_string())?;
        let account: Account = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(Some(account));
    }

    Ok(None)
}
