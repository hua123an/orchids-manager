use crate::models::{Account, AccountStore};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AccountManagerState {
    pub store: Arc<Mutex<AccountStore>>,
    pub file_path: PathBuf,
}

impl AccountManagerState {
    pub fn new(app_handle: &AppHandle) -> Self {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        }

        let file_path = app_data_dir.join("accounts.json");
        let store = Self::load_from_disk(&file_path).unwrap_or_default();

        Self {
            store: Arc::new(Mutex::new(store)),
            file_path,
        }
    }

    fn load_from_disk(path: &Path) -> Option<AccountStore> {
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&*store).map_err(|e| e.to_string())?;
        fs::write(&self.file_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_account(&self, account: Account) -> Result<(), String> {
        let mut store = self.store.lock().map_err(|e| e.to_string())?;
        // Remove existing if any
        store.accounts.retain(|a| a.id != account.id);
        store.accounts.push(account);
        drop(store); // unlock before saving
        self.save()
    }

    pub fn get_accounts(&self) -> Result<Vec<Account>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        Ok(store.accounts.clone())
    }

    pub fn delete_account(&self, id: String) -> Result<(), String> {
        let mut store = self.store.lock().map_err(|e| e.to_string())?;
        store.accounts.retain(|a| a.id != id);

        if let Some(active) = &store.active_user_id {
            if active == &id {
                store.active_user_id = None;
            }
        }

        drop(store);
        self.save()
    }

    pub fn set_active(&self, id: String) -> Result<(), String> {
        let mut store = self.store.lock().map_err(|e| e.to_string())?;
        // Verify exists
        if !store.accounts.iter().any(|a| a.id == id) {
            return Err("Account not found".into());
        }
        store.active_user_id = Some(id);
        drop(store);
        self.save()
    }

    pub fn get_active_id(&self) -> Result<Option<String>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        Ok(store.active_user_id.clone())
    }
}
