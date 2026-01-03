use crate::models::CookieData;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_cookies_from_db(db_path: &Path) -> Result<Vec<CookieData>, String> {
    // Copy the DB to a temp file to avoid locking issues if Orchids is writing to it
    let temp_dir = std::env::temp_dir();
    let temp_db_path = temp_dir.join("orchids_cookies_copy.db");

    fs::copy(db_path, &temp_db_path).map_err(|e| format!("Failed to copy cookie DB: {}", e))?;

    let conn = Connection::open(&temp_db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT name, value, host_key, path, is_secure, is_httponly, expires_utc, samesite, priority FROM cookies",
        )
        .map_err(|e| e.to_string())?;

    let cookies_iter = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let value: String = row.get(1)?;
            let domain: String = row.get(2)?;
            let path: String = row.get(3)?;
            let secure: bool = row.get(4)?;
            let http_only: bool = row.get(5)?;
            let expires_utc: i64 = row.get(6)?;
            let samesite: i32 = row.get(7).unwrap_or(-1);
            let priority: i32 = row.get(8).unwrap_or(1);

            // Convert Windows Microseconds to Unix Seconds
            // Unix Epoch (1970) - Windows Epoch (1601) = 11644473600 seconds
            let expiration_date = if expires_utc > 0 {
                Some(((expires_utc as f64) / 1000000.0) - 11644473600.0)
            } else {
                None
            };

            Ok(CookieData {
                name,
                value,
                domain,
                path,
                secure,
                http_only,
                expiration_date,
                samesite: Some(samesite),
                priority: Some(priority),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut cookies = Vec::new();
    for cookie in cookies_iter {
        cookies.push(cookie.map_err(|e| e.to_string())?);
    }

    // Cleanup
    let _ = fs::remove_file(temp_db_path);

    Ok(cookies)
}
