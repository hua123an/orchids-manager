use crate::cookie_reader::read_cookies_from_db;
use crate::models::Account;
use crate::store::AccountManagerState;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, Manager, State};

// Helper to get Orchids Application Support directory
pub fn get_orchids_data_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/huaan".to_string());
        std::path::Path::new(&home).join("Library/Application Support/Orchids")
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::path::PathBuf::from(".") // Placeholder
    }
}

// Helper to get Orchids Cookie DB path
pub fn get_orchids_cookie_path() -> std::path::PathBuf {
    get_orchids_data_dir().join("Cookies")
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
            let path_buf = get_orchids_cookie_path();
            let path = path_buf.as_path();
            // Initialize last processed time to 0 so we always process once on start
            let mut last_processed_time = SystemTime::UNIX_EPOCH;

            let _ = app_handle.emit("debug-log", format!("Monitoring started on: {:?}", path));

            loop {
                // Check if we should stop
                if let Ok(running) = is_running.lock() {
                    if !*running {
                        break;
                    }
                }

                if path.exists() {
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            // Process if modified is newer OR if it's the first run (last_processed_time is EPOCH)
                            // But actually, just checking modified > last_processed_time is enough if last is EPOCH.

                            if modified > last_processed_time {
                                let _ = app_handle
                                    .emit("debug-log", "Detected file change, processing...");

                                // Debounce slightly
                                thread::sleep(Duration::from_secs(1));

                                match process_cookies(path) {
                                    Ok(Some(account)) => {
                                        let state = app_handle.state::<AccountManagerState>();
                                        if let Err(e) = state.add_account(account.clone()) {
                                            let _ = app_handle.emit(
                                                "debug-log",
                                                format!("Failed to save: {}", e),
                                            );
                                        } else {
                                            let _ = app_handle.emit(
                                                "debug-log",
                                                format!("Captured: {}", account.display_name),
                                            );
                                            let _ = app_handle.emit("account-captured", account);
                                        }
                                        // Update timestamp only on success? No, always to avoid loop,
                                        // UNLESS we want to retry. But retrying corrupt file is bad.
                                        last_processed_time = modified;
                                    }
                                    Ok(None) => {
                                        let _ =
                                            app_handle.emit("debug-log", "No session cookie found");
                                        // Maybe it's a partial write, don't update last_processed_time?
                                        // No, update it, otherwise we loop forever on a logged-out state.
                                        last_processed_time = modified;
                                    }
                                    Err(e) => {
                                        let _ = app_handle
                                            .emit("debug-log", format!("Read error: {}", e));
                                        // If error (e.g. locked), maybe don't update time so we retry?
                                        // But if persistent error, we loop. Let's update.
                                        last_processed_time = modified;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let _ = app_handle.emit("debug-log", "Cookie file not found");
                }

                thread::sleep(Duration::from_secs(2));
            }
            let _ = app_handle.emit("debug-log", "Monitoring thread exited");
        });
    }

    pub fn stop(&self) {
        let mut running = self.is_running.lock().unwrap();
        *running = false;
    }
}

pub fn process_cookies(path: &Path) -> Result<Option<Account>, String> {
    let cookies = read_cookies_from_db(path)?;

    // Find __session
    let session_cookie = cookies.iter().find(|c| c.name == "__session");
    if session_cookie.is_none() {
        return Ok(None);
    }
    let token = session_cookie.unwrap().value.clone();

    let account = create_account_from_token(&token, cookies)?;
    Ok(Some(account))
}

pub fn create_account_from_token(
    token: &str,
    cookies: Vec<crate::models::CookieData>,
) -> Result<Account, String> {
    // Decode JWT for ID (Basic decode)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return Err("Invalid Token format".to_string());
    }

    let payload = parts[1];
    // JWT standard is Base64Url NO padding
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| {
            // Fallback: Try adding padding and standard decoding
            let padded = match payload.len() % 4 {
                0 => payload.to_string(),
                2 => format!("{}==", payload),
                3 => format!("{}=", payload),
                _ => payload.to_string(),
            };
            general_purpose::STANDARD.decode(&padded)
        })
        .or_else(|_| {
            // One last try: URL Safe WITH padding
            let padded = match payload.len() % 4 {
                0 => payload.to_string(),
                2 => format!("{}==", payload),
                3 => format!("{}=", payload),
                _ => payload.to_string(),
            };
            general_purpose::URL_SAFE.decode(&padded)
        })
        .map_err(|e| format!("Base64 fail: {}", e))?;

    let json_str = String::from_utf8(decoded).map_err(|e| e.to_string())?;
    let claims: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    let user_id = claims["sub"].as_str().ok_or("No sub in token")?;

    println!("Found User ID: {}", user_id);

    // Fetch Profile
    let client = reqwest::blocking::Client::new();
    let resp = client.get(format!("https://orchids-server.calmstone-6964e08a.westeurope.azurecontainerapps.io/user/profile/{}", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("API Error: {}", resp.status()));
    }

    let profile: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

    // Construct Account
    let display_name = if let Some(name) = profile["fullName"].as_str() {
        name.to_string()
    } else if let Some(email) = profile["email"].as_str() {
        email.split('@').next().unwrap_or(email).to_string()
    } else {
        "Unknown User".to_string()
    };

    let account = Account {
        id: user_id.to_string(),
        display_name,
        email: profile["email"].as_str().map(|s| s.to_string()),
        avatar_url: profile["imageUrl"].as_str().map(|s| s.to_string()),
        last_active_at: Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        ),
        user_info: Some(crate::models::UserInfo {
            id: None,
            user_id: user_id.to_string(),
            full_name: profile["fullName"].as_str().map(|s| s.to_string()),
            email: profile["email"].as_str().map(|s| s.to_string()),
            image_url: profile["imageUrl"].as_str().map(|s| s.to_string()),
            plan: profile["plan"].as_str().map(|s| s.to_string()),
            credits: profile["credits"].as_i64(),
        }),
        cookies: cookies,
    };

    Ok(account)
}

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
