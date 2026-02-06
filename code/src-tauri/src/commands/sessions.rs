//! Session file management
//!
//! Handles saving, loading, listing, and deleting meter session files.

use super::types::SessionData;

/// Get the sessions folder path (next to executable)
fn get_sessions_folder() -> Result<std::path::PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_dir = exe_path.parent().ok_or("Failed to get exe directory")?;
    let sessions_dir = exe_dir.join("omnicore-meter-sessions");

    // Create directory if it doesn't exist
    if !sessions_dir.exists() {
        std::fs::create_dir_all(&sessions_dir)
            .map_err(|e| format!("Failed to create sessions directory: {}", e))?;
    }

    Ok(sessions_dir)
}

/// Delete existing sessions for a meter (helper function)
fn delete_meter_sessions(sessions_dir: &std::path::Path, flag: &str, serial_number: &str) -> Result<(), String> {
    let prefix = format!("{}-{}-",
        flag.replace(|c: char| !c.is_alphanumeric(), "_"),
        serial_number.replace(|c: char| !c.is_alphanumeric(), "_")
    );

    let entries = std::fs::read_dir(sessions_dir)
        .map_err(|e| format!("Failed to read sessions directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with(&prefix) && filename.ends_with(".json") {
                log::info!("Deleting existing session: {:?}", path);
                std::fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete session file: {}", e))?;
            }
        }
    }

    Ok(())
}

/// Save session to JSON file
#[tauri::command]
pub async fn save_session_file(
    flag: String,
    serial_number: String,
    model: String,
    note: String,
    meter_data: serde_json::Value,
    connection_info: serde_json::Value,
    overwrite_existing: bool,
) -> Result<String, String> {
    log::info!("Saving session for {}-{}", flag, serial_number);

    let sessions_dir = get_sessions_folder()?;

    // If overwrite is enabled, delete existing sessions for this meter
    if overwrite_existing {
        delete_meter_sessions(&sessions_dir, &flag, &serial_number)?;
    }

    // Generate filename: flag-serialnumber-YYYYmmddHHnn.json
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M").to_string();
    let filename = format!("{}-{}-{}.json",
        flag.replace(|c: char| !c.is_alphanumeric(), "_"),
        serial_number.replace(|c: char| !c.is_alphanumeric(), "_"),
        timestamp
    );
    let file_path = sessions_dir.join(&filename);

    // Build session data
    let session = SessionData {
        flag,
        serial_number,
        model,
        saved_at: now.format("%Y-%m-%d %H:%M:%S").to_string(),
        note,
        meter_data,
        connection_info,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&session)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;

    // Write to file
    std::fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write session file: {}", e))?;

    log::info!("Session saved to: {:?}", file_path);
    Ok(filename)
}

/// List saved session files
#[tauri::command]
pub async fn list_session_files() -> Result<Vec<serde_json::Value>, String> {
    let sessions_dir = get_sessions_folder()?;

    let mut sessions = Vec::new();
    let entries = std::fs::read_dir(&sessions_dir)
        .map_err(|e| format!("Failed to read sessions directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<serde_json::Value>(&content) {
                    let mut session_info = session.clone();
                    if let Some(obj) = session_info.as_object_mut() {
                        obj.insert("fileName".to_string(),
                            serde_json::Value::String(
                                path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string()
                            )
                        );
                    }
                    sessions.push(session_info);
                }
            }
        }
    }

    // Sort by saved_at descending (newest first)
    sessions.sort_by(|a, b| {
        let a_time = a.get("savedAt").and_then(|v| v.as_str()).unwrap_or("");
        let b_time = b.get("savedAt").and_then(|v| v.as_str()).unwrap_or("");
        b_time.cmp(a_time)
    });

    Ok(sessions)
}

/// Load a specific session file
#[tauri::command]
pub async fn load_session_file(filename: String) -> Result<serde_json::Value, String> {
    let sessions_dir = get_sessions_folder()?;
    let file_path = sessions_dir.join(&filename);

    if !file_path.exists() {
        return Err(format!("Session file not found: {}", filename));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse session file: {}", e))
}

/// Delete a specific session file
#[tauri::command]
pub async fn delete_session_file(filename: String) -> Result<(), String> {
    let sessions_dir = get_sessions_folder()?;
    let file_path = sessions_dir.join(&filename);

    if !file_path.exists() {
        return Err(format!("Session file not found: {}", filename));
    }

    std::fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete session file: {}", e))?;

    log::info!("Session file deleted: {}", filename);
    Ok(())
}
