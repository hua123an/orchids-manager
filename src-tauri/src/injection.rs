use crate::models::Account;
use crate::store::AccountManagerState;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;
use tauri::{AppHandle, Manager, State};

const ORCHIDS_PATH: &str = "/Applications/Orchids.app/Contents/Resources/app/main/index.js";
const INJECTION_MARKER: &str = "// --- ORCHIDS MANAGER INJECTION START ---";
const CAPTURED_FILE_NAME: &str = "captured_session.json";

pub struct InjectionManager {
    injection_code: String,
}

impl InjectionManager {
    pub fn new() -> Self {
        // Only include the script in the debug release or use include_str!
        let code = include_str!("../assets/injection.js");
        Self {
            injection_code: code.to_string(),
        }
    }

    pub fn inject(&self) -> Result<String, String> {
        let target_path = Path::new(ORCHIDS_PATH);
        if !target_path.exists() {
            return Err("Orchids app not found at expected path".into());
        }

        let content = fs::read_to_string(target_path).map_err(|e| e.to_string())?;

        // Check if already injected
        if content.contains(INJECTION_MARKER) {
            // Update injection code if needed (simple overwrite)
            // For now, assume if marker exists it's fine, or we can force update
            return Ok("Already injected".into());
        }

        // Backup
        let _ = fs::copy(target_path, target_path.with_extension("js.bak"));

        // Prepend
        let new_content = format!("{}\n\n{}", self.injection_code, content);
        fs::write(target_path, new_content).map_err(|e| e.to_string())?;

        Ok("Injection successful".into())
    }

    pub fn uninject(&self) -> Result<String, String> {
        let target_path = Path::new(ORCHIDS_PATH);
        if !target_path.exists() {
            return Err("Orchids app not found".into());
        }

        let content = fs::read_to_string(target_path).map_err(|e| e.to_string())?;

        if let Some(start_idx) = content.find(INJECTION_MARKER) {
            // Find end marker
            let end_marker = "// --- ORCHIDS MANAGER INJECTION END ---";
            if let Some(end_idx) = content.find(end_marker) {
                let end_pos = end_idx + end_marker.len();
                let new_content = format!(
                    "{}{}",
                    &content[..start_idx],
                    &content[end_pos..].trim_start()
                );
                fs::write(target_path, new_content).map_err(|e| e.to_string())?;
                return Ok("Uninjected successfully".into());
            }
        }

        Ok("No injection found".into())
    }
}

pub fn check_captured_data(
    app_handle: &AppHandle,
    state: &State<AccountManagerState>,
) -> Result<Option<Account>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data");
    let capture_file = app_data_dir.join(CAPTURED_FILE_NAME);

    if capture_file.exists() {
        let content = fs::read_to_string(&capture_file).map_err(|e| e.to_string())?;
        let account: Account = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        state.add_account(account.clone())?;

        // Cleanup
        let _ = fs::remove_file(capture_file);

        // Close the browser tab (Chrome/Safari/Arc)
        let _ = std::thread::spawn(|| {
            let script = r#"
            try
                tell application "Google Chrome"
                    set windowList to every window
                    repeat with aWindow in windowList
                        set tabList to every tab of aWindow
                        repeat with aTab in tabList
                            if URL of aTab contains "orchids.app" then
                                close aTab
                            end if
                        end repeat
                    end repeat
                end tell
            end try
            try
                tell application "Safari"
                    set windowList to every window
                    repeat with aWindow in windowList
                        set tabList to every tab of aWindow
                        repeat with aTab in tabList
                            if URL of aTab contains "orchids.app" then
                                close aTab
                            end if
                        end repeat
                    end repeat
                end tell
            end try
            try
                tell application "Arc"
                     tell front window
                        set tabList to every tab
                        repeat with aTab in tabList
                            if URL of aTab contains "orchids.app" then
                                close aTab
                            end if
                        end repeat
                    end tell
                end tell
            end try
            "#;
            let _ = std::process::Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output();
        });

        return Ok(Some(account));
    }

    Ok(None)
}

use crate::capture_service::ORCHIDS_COOKIE_PATH;
use rusqlite::Connection;
use std::thread;
use std::time::Duration;

pub fn trigger_restore(_app_handle: &AppHandle, account: &Account) -> Result<(), String> {
    // 1. Kill Orchids
    println!("Killing Orchids...");
    let _ = Command::new("killall")
        .arg("Orchids")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    // Wait for it to die
    thread::sleep(Duration::from_millis(100)); // Minimal wait
    println!("Orchids killed. Opening DB...");

    // 2. Open DB
    let path = Path::new(ORCHIDS_COOKIE_PATH);
    // REMOVED manual deletion of journal/wal files to let SQLite handle recovery/consistency

    if !path.exists() {
        return Err("Orchids cookie database not found".into());
    }

    let mut conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {}", e))?;

    // Set busy timeout to allow file locking to resolve
    let _ = conn.execute("PRAGMA busy_timeout = 5000;", []);

    let tx = conn
        .transaction()
        .map_err(|e| format!("Transaction error: {}", e))?;

    // 3. Clear existing cookies
    println!("Clearing old cookies...");
    tx.execute("DELETE FROM cookies", [])
        .map_err(|e| format!("Failed to clear cookies: {}", e))?;

    // 4. Insert new cookies - FULL SCHEMA
    let mut stmt = tx.prepare(
        "INSERT INTO cookies (
            creation_utc, host_key, top_frame_site_key, name, value, encrypted_value, path,
            expires_utc, is_secure, is_httponly, last_access_utc, has_expires, is_persistent,
            priority, samesite, source_scheme, source_port, last_update_utc, source_type, has_cross_site_ancestor
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    ).map_err(|e| e.to_string())?;

    for cookie in &account.cookies {
        let expires_utc = if let Some(exp) = cookie.expiration_date {
            ((exp + 11644473600.0) * 1_000_000.0) as i64
        } else {
            0
        };

        let now_win_us = ((SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            + 11644473600.0)
            * 1_000_000.0) as i64;

        stmt.execute(rusqlite::params![
            now_win_us,                          // creation_utc
            &cookie.domain,                      // host_key
            "",                                  // top_frame_site_key
            &cookie.name,                        // name
            &cookie.value,                       // value
            b"",                                 // encrypted_value (empty blob)
            &cookie.path,                        // path
            expires_utc,                         // expires_utc
            cookie.secure,                       // is_secure
            cookie.http_only,                    // is_httponly
            now_win_us,                          // last_access_utc
            if expires_utc > 0 { 1 } else { 0 }, // has_expires
            1,                                   // is_persistent
            cookie.priority.unwrap_or(1),        // priority
            cookie.samesite.unwrap_or(-1),       // samesite
            if cookie.secure { 2 } else { 1 },   // source_scheme
            443,                                 // source_port
            now_win_us,                          // last_update_utc
            0,                                   // source_type (0 = unknown)
            0,                                   // has_cross_site_ancestor (0 = false)
        ])
        .map_err(|e| format!("Insert failed for {}: {}", cookie.name, e))?;
    }
    println!("Cookies inserted.");

    drop(stmt);
    tx.commit().map_err(|e| format!("Commit failed: {}", e))?;

    println!("Cookies restored successfully.");

    // 5. Restart Orchids
    println!("Restarting Orchids...");
    thread::sleep(Duration::from_millis(100));
    let _ = Command::new("open").arg("-a").arg("Orchids").spawn();

    Ok(())
}

fn clear_cookies_db(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let conn = Connection::open(path).map_err(|e| format!("DB Open Error: {}", e))?;
    let _ = conn.execute("PRAGMA busy_timeout = 5000;", []);
    conn.execute("DELETE FROM cookies", [])
        .map_err(|e| format!("DB Delete Error: {}", e))?;
    Ok(())
}

pub fn clear_cookies_and_restart(_app_handle: &AppHandle) -> Result<(), String> {
    // 1. Kill Orchids aggressively
    println!("Killing Orchids for logout...");
    let _ = Command::new("killall")
        .arg("Orchids")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    thread::sleep(Duration::from_millis(100));

    // 2. Clear Cookies (SQL prefered to preserve DB structure)
    let path = Path::new(ORCHIDS_COOKIE_PATH);
    if let Err(e) = clear_cookies_db(path) {
        println!("SQL Clear failed, deleting file: {}", e);
        let _ = fs::remove_file(path);
    }

    // 3. Restart Orchids
    println!("Restarting Orchids...");
    thread::sleep(Duration::from_millis(100));
    let _ = Command::new("open").arg("-a").arg("Orchids").spawn();

    Ok(())
}

pub fn reset_machine_id(_app_handle: &AppHandle) -> Result<String, String> {
    println!("Resetting Machine ID...");

    // 0. Backup Local Storage (Protection against app wipe)
    let ls_path = Path::new("/Users/huaan/Library/Application Support/Orchids/Local Storage");
    let ls_bak = Path::new("/Users/huaan/Library/Application Support/Orchids/Local Storage.bak");
    if ls_path.exists() {
        println!("Backing up Local Storage...");
        let _ = Command::new("cp")
            .arg("-R")
            .arg(ls_path)
            .arg(ls_bak)
            .status();
    }

    // 1. Kill Orchids
    println!("Killing Orchids...");
    let _ = Command::new("killall")
        .arg("Orchids")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    thread::sleep(Duration::from_millis(500));

    // 2. Generate new UUID
    let output = Command::new("uuidgen")
        .output()
        .map_err(|e| format!("Failed to generate UUID: {}", e.to_string()))?;

    let new_uuid = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();

    if new_uuid.is_empty() {
        return Err("Generated empty UUID".into());
    }

    // 3. Write to .updaterId
    let updater_path = Path::new("/Users/huaan/Library/Application Support/Orchids/.updaterId");
    if let Some(parent) = updater_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(updater_path, &new_uuid).map_err(|e| format!("Failed to write .updaterId: {}", e))?;

    println!("New Machine ID written: {}", new_uuid);

    // 4. Clear Cookies - Attempt SQL Clean First
    let cookie_path = Path::new(ORCHIDS_COOKIE_PATH);
    if let Err(e) = clear_cookies_db(cookie_path) {
        println!("SQL Clear failed, deleting file: {}", e);
        let _ = fs::remove_file(cookie_path); // Fallback
                                              // Also remove journals in fallback case only
        if let Some(parent) = cookie_path.parent() {
            if let Some(filename_os) = cookie_path.file_name() {
                let filename = filename_os.to_string_lossy();
                let _ = fs::remove_file(parent.join(format!("{}-journal", filename)));
                let _ = fs::remove_file(parent.join(format!("{}-wal", filename)));
            }
        }
    }

    // 5. Restore Local Storage if it was deleted/wiped (Pre-emptive restore won't stop app from wiping if it wants to,
    // but copying it back *might* help if we restore after launch? No, file locks)
    // Actually, if we just *don't delete it*, and the App wipes it, we can't stop the app.
    // BUT the backup allows the USER to recover manualy if needed.
    // Let's try to restore it inplace if it's missing.
    if !ls_path.exists() && ls_bak.exists() {
        println!("Restoring Local Storage...");
        let _ = Command::new("cp")
            .arg("-R")
            .arg(ls_bak)
            .arg(ls_path)
            .status();
    }

    // 5. Restart Orchids
    println!("Restarting Orchids...");
    thread::sleep(Duration::from_millis(500));
    let _ = Command::new("open").arg("-a").arg("Orchids").spawn();

    Ok(format!(
        "Reset Success. App Restarted. New ID: {}",
        new_uuid
    ))
}
