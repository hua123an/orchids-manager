use crate::models::{Checkpoint, CheckpointFile, Project};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_orchids_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join("Library/Application Support/Orchids")
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(rename = "recentProjects")]
    recent_projects: Vec<Project>,
}

pub fn list_projects() -> Result<Vec<Project>, String> {
    let config_path = get_orchids_data_dir().join("config.json");
    if !config_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let config: Config = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config.recent_projects)
}

pub fn list_checkpoints(project_id: &str) -> Result<Vec<Checkpoint>, String> {
    let db_path = get_orchids_data_dir()
        .join("checkpoints")
        .join(project_id)
        .join("checkpoints.db");

    if !db_path.exists() {
        return Err(format!("Database not found at {:?}", db_path));
    }

    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.message_id, c.parent_id, c.created_at, COUNT(f.id) as file_count 
             FROM checkpoints c 
             LEFT JOIN checkpoint_files f ON c.id = f.checkpoint_id 
             GROUP BY c.id 
             ORDER BY c.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Checkpoint {
                id: row.get(0)?,
                message_id: row.get(1)?,
                parent_id: row.get(2)?,
                created_at: row.get(3)?,
                file_count: Some(row.get(4)?),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut checkpoints = Vec::new();
    for row in rows {
        checkpoints.push(row.map_err(|e| e.to_string())?);
    }

    Ok(checkpoints)
}

pub fn list_checkpoint_files(project_id: &str, checkpoint_id: i64) -> Result<Vec<CheckpointFile>, String> {
    let db_path = get_orchids_data_dir()
        .join("checkpoints")
        .join(project_id)
        .join("checkpoints.db");

    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, checkpoint_id, file_path, status, hash, mode, is_text, has_delta 
             FROM checkpoint_files 
             WHERE checkpoint_id = ?",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![checkpoint_id], |row| {
            Ok(CheckpointFile {
                id: row.get(0)?,
                checkpoint_id: row.get(1)?,
                file_path: row.get(2)?,
                status: row.get(3)?,
                hash: row.get(4)?,
                mode: row.get(5)?,
                is_text: row.get(6).map(|v: i32| v != 0)?,
                has_delta: row.get(7).map(|v: i32| v != 0)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(|e| e.to_string())?);
    }

    Ok(files)
}

pub fn get_file_content(project_id: &str, hash: &str) -> Result<Vec<u8>, String> {
    let blob_path = get_orchids_data_dir()
        .join("checkpoints")
        .join(project_id)
        .join("blobs")
        .join(hash);

    if !blob_path.exists() {
        return Err(format!("Blob not found at {:?}", blob_path));
    }

    fs::read(blob_path).map_err(|e| e.to_string())
}

pub fn rollback_project(project_id: &str, checkpoint_id: i64) -> Result<(), String> {
    // 1. Get project path
    let projects = list_projects()?;
    let project = projects
        .iter()
        .find(|p| p.project_id == project_id)
        .ok_or("Project not found")?;
    let project_path = Path::new(&project.folder_path);

    if !project_path.exists() {
        return Err(format!("Project folder not found at {:?}", project_path));
    }

    // 2. Get files for this checkpoint
    let files = list_checkpoint_files(project_id, checkpoint_id)?;

    // 3. Restore files
    for file in files {
        let target_path = project_path.join(&file.file_path);
        
        if file.status == "deleted" {
            if target_path.exists() {
                let _ = fs::remove_file(&target_path);
            }
            continue;
        }

        if let Some(hash) = file.hash {
            let content = get_file_content(project_id, &hash)?;
            
            // Create parent directories if they don't exist
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            
            fs::write(&target_path, content).map_err(|e| e.to_string())?;
            
            // On Unix, restore mode (permissions)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&target_path, fs::Permissions::from_mode(file.mode as u32));
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_projects() -> Result<Vec<Project>, String> {
    list_projects()
}

#[tauri::command]
pub fn get_checkpoints(project_id: String) -> Result<Vec<Checkpoint>, String> {
    list_checkpoints(&project_id)
}

#[tauri::command]
pub fn get_checkpoint_files(project_id: String, checkpoint_id: i64) -> Result<Vec<CheckpointFile>, String> {
    list_checkpoint_files(&project_id, checkpoint_id)
}

#[tauri::command]
pub fn get_file_content_base64(project_id: String, hash: String) -> Result<String, String> {
    let bytes = get_file_content(&project_id, &hash)?;
    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes))
}

#[tauri::command]
pub fn rollback_checkpoint(project_id: String, checkpoint_id: i64) -> Result<(), String> {
    rollback_project(&project_id, checkpoint_id)
}
