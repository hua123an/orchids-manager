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
    thread::sleep(Duration::from_secs(2)); // Increased wait
    println!("Orchids killed. Opening DB...");

    // 2. Open DB
    let path = Path::new(ORCHIDS_COOKIE_PATH);

    // Nuke journal files to clear locks/state (unsafe but effective for reset)
    if let Some(parent) = path.parent() {
        if let Some(filename_os) = path.file_name() {
            let filename = filename_os.to_string_lossy();
            let journal = parent.join(format!("{}-journal", filename));
            let wal = parent.join(format!("{}-wal", filename));
            let shm = parent.join(format!("{}-shm", filename));

            let _ = fs::remove_file(journal);
            let _ = fs::remove_file(wal);
            let _ = fs::remove_file(shm);
        }
    }

    if !path.exists() {
        return Err("Orchids cookie database not found".into());
    }

    let mut conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {}", e))?;

    let tx = conn
        .transaction()
        .map_err(|e| format!("Transaction error: {}", e))?;

    // 3. Clear existing cookies (or just __session ones? safer to clear all for clean switch)
    // Let's clear all for now to avoid conflicts.
    println!("Clearing old cookies...");
    tx.execute("DELETE FROM cookies", [])
        .map_err(|e| format!("Failed to clear cookies: {}", e))?;

    // 4. Insert new cookies - FULL SCHEMA (20 columns, all NOT NULL)
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
    thread::sleep(Duration::from_secs(1)); // Brief pause before launch
    let _ = Command::new("open").arg("-a").arg("Orchids").spawn();

    Ok(())
}

pub fn clear_cookies_and_restart(_app_handle: &AppHandle) -> Result<(), String> {
    // 1. Kill Orchids aggressively
    println!("Killing Orchids for logout...");

    // Use killall with suppressed output to avoid "No matching processes" noise
    // And avoid pkill -f which might match "orchids-manager" path!
    let _ = Command::new("killall")
        .arg("Orchids")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    thread::sleep(Duration::from_secs(2)); // Wait longer for lock release

    // 2. Clear Cookies by DELETING the file
    let path = Path::new(ORCHIDS_COOKIE_PATH);
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            println!("Failed to delete Cookies file: {}", e);
            // If delete fails, try SQL delete as fallback
            if let Ok(conn) = Connection::open(path) {
                let _ = conn.execute("DELETE FROM cookies", []);
            }
        } else {
            println!("Cookies file deleted.");
        }
    }

    // Also try to delete journal/wal files
    if let Some(parent) = path.parent() {
        if let Some(filename_os) = path.file_name() {
            let filename = filename_os.to_string_lossy();
            let journal = parent.join(format!("{}-journal", filename));
            let wal = parent.join(format!("{}-wal", filename));

            if journal.exists() {
                let _ = fs::remove_file(journal);
            }
            if wal.exists() {
                let _ = fs::remove_file(wal);
            }
        }
    }

    // 3. Restart Orchids
    thread::sleep(Duration::from_secs(1));
    let _ = Command::new("open").arg("-a").arg("Orchids").spawn();

    Ok(())
}
