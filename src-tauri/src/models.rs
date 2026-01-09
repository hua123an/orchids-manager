use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CookieData {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expiration_date: Option<f64>,
    pub samesite: Option<i32>,
    pub priority: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    // Basic User Info
    pub id: Option<u64>,
    pub user_id: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub image_url: Option<String>,

    // Billing / Plan Info
    pub plan: Option<String>,
    pub credits: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: String, // Clerk User ID
    pub display_name: String,
    pub email: Option<String>,
    pub password: Option<String>, // Added for auto-relogin
    pub avatar_url: Option<String>,

    pub last_active_at: Option<i64>, // Unix Timestamp

    pub user_info: Option<UserInfo>, // Full profile data
    pub cookies: Vec<CookieData>,    // Session Snapshot
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AccountStore {
    pub version: String,
    pub active_user_id: Option<String>,
    pub accounts: Vec<Account>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub folder_path: String,
    pub project_id: String,
    pub project_name: String,
    pub last_opened: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Checkpoint {
    pub id: i64,
    pub message_id: String,
    pub parent_id: Option<i64>,
    pub created_at: i64,
    pub file_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckpointFile {
    pub id: i64,
    pub checkpoint_id: i64,
    pub file_path: String,
    pub status: String,
    pub hash: Option<String>,
    pub mode: i32,
    pub is_text: bool,
    pub has_delta: bool,
}
