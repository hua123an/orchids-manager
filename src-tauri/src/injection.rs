use crate::capture_service::{get_orchids_cookie_path, get_orchids_data_dir};
use crate::models::Account;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const ORCHIDS_APP_PATH: &str = "/Applications/Orchids.app/Contents/Resources";
const INJECTION_MARKER: &str = "// --- ORCHIDS MANAGER INJECTION START ---";

pub struct OrchidsCore;

impl OrchidsCore {
    /// (V2) Ensures the Orchids app is unpacked and injected with our NEW V2 script.
    pub fn ensure_patched() -> Result<String, String> {
        let resources_path = Path::new(ORCHIDS_APP_PATH);
        if !resources_path.exists() {
            return Err("Orchids application not found in /Applications".into());
        }

        let main_js_path = resources_path.join("app/main/index.js");

        // 1. Check content
        if main_js_path.exists() {
            let content = fs::read_to_string(&main_js_path).map_err(|e| e.to_string())?;
            // If it contains V2 marker, we are good.
            if content.contains("// --- ORCHIDS MANAGER INJECTION START V2 ---") {
                return Ok("Orchids is patched (V2) and ready.".into());
            }

            // If not (unpatched OR old V1 patch), we proceed to inject.
            // If it's V1, we will strip it in inject_script.
            return Self::inject_script(&main_js_path);
        }

        // 2. Needs Unpacking (ASAR)
        println!("Orchids is packed. Unpacking ASAR...");
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "cd '{}' && npx -y @electron/asar extract app.asar app && mv app.asar app.asar.bak",
                ORCHIDS_APP_PATH
            ))
            .status()
            .map_err(|e| format!("Failed to execute unpack command: {}", e))?;

        if !status.success() {
            return Err("Failed to unpack Orchids ASAR. Install Node.js/Permissions?".into());
        }

        // 3. Inject
        Self::inject_script(&main_js_path)
    }

    fn inject_script(target_path: &Path) -> Result<String, String> {
        println!("Injecting V2 control script into {:?}...", target_path);

        let mut original_content = fs::read_to_string(target_path).unwrap_or_default();
        let injection_code = include_str!("../assets/injection.js");

        // CLEANUP OLD V1 OR PARTIAL INJECTIONS
        // We look for the Start/End markers and remove everything between them.
        // Old Marker: // --- ORCHIDS MANAGER INJECTION START ---

        if let Some(start_idx) = original_content.find(INJECTION_MARKER) {
            // If we find the legacy V1 marker, we attempt to strip it.
            // V1 structure was `(function() { ... })();` prepended.
            // We can search for the end of that IIFE.
            if let Some(end_idx) = original_content.find("})();") {
                let cutoff = end_idx + 5;
                if cutoff < original_content.len() {
                    original_content = original_content[cutoff..].trim_start().to_string();
                }
            } else if let Some(app_start) = original_content.find("const { app,") {
                // Fallback: If we can't find end, look for where original app code likely starts
                original_content = original_content[app_start..].to_string();
            }
        }

        // New Injection (V2) has explicit START and END markers.
        let new_content = format!("{}\n\n{}", injection_code, original_content);
        fs::write(target_path, new_content)
            .map_err(|e| format!("Failed to write injection: {}", e))?;

        Ok("Orchids successfully patched (V2)!".into())
    }

    pub fn switch_account(_app_handle: &AppHandle, account: &Account) -> Result<(), String> {
        println!(
            ">>> Starting Account Switch (V2) for: {}",
            account.display_name
        );

        // 1. KILL
        let _ = Command::new("killall").arg("Orchids").status();
        thread::sleep(Duration::from_millis(1000));

        // 2. PREPARE JSON (Shared Path)
        let home = std::env::var("HOME").map_err(|_| "No HOME env".to_string())?;
        let shared_dir = PathBuf::from(home).join("Library/Application Support/OrchidsManager");

        if !shared_dir.exists() {
            fs::create_dir_all(&shared_dir).map_err(|e| e.to_string())?;
        }

        let restore_file = shared_dir.join("restore_session.json");
        let json = serde_json::to_string_pretty(account).map_err(|e| e.to_string())?;
        fs::write(&restore_file, json)
            .map_err(|e| format!("Failed to write restore file: {}", e))?;

        // 3. WIPE CLEAN
        // 3. WIPE CLEAN
        // Clear Cookies DB - DISABLED: Let Electron session API handle it
        // let cookie_path = get_orchids_cookie_path();
        // if cookie_path.exists() {
        //     let _ = fs::remove_file(&cookie_path);
        //     if let Some(parent) = cookie_path.parent() {
        //         let stem = cookie_path.file_stem().unwrap().to_string_lossy();
        //         let _ = fs::remove_file(parent.join(format!("{}.wal", stem)));
        //         let _ = fs::remove_file(parent.join(format!("{}.shm", stem)));
        //         let _ = fs::remove_file(parent.join(format!("{}-wal", stem)));
        //         let _ = fs::remove_file(parent.join(format!("{}-journal", stem)));
        //     }
        // }

        // Clear Local Storage
        // let orchids_data = get_orchids_data_dir();
        // let ls_path = orchids_data.join("Local Storage");
        // if ls_path.exists() {
        //     let _ = fs::remove_dir_all(ls_path);
        // }

        // 4. RESTART
        println!(">>> Launching Orchids...");
        let child = Command::new("/usr/bin/open")
            .arg("-a")
            .arg("Orchids")
            .spawn()
            .map_err(|e| format!("Failed to launch Orchids: {}", e))?;

        println!(">>> Orchids launched (PID: {:?})", child.id());

        Ok(())
    }
}
