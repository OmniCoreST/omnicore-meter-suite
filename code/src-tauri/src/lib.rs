use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

mod serial;
mod commands;
mod storage;
mod i18n;

pub use commands::*;
pub use storage::{Session, Report, AppSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub description: Option<String>,
    pub port_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterIdentity {
    pub manufacturer: String,
    pub edas_id: String,
    pub model: String,
    pub baud_rate_char: String,
    pub generation: String,
    pub serial_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionParams {
    pub connection_type: String,
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u32,
    pub meter_address: Option<String>,
    pub password: Option<String>,
}

// Database commands
mod db_commands {
    use super::*;
    use crate::storage;

    /// Save a session to the database
    #[tauri::command]
    pub fn save_session(
        session: Session,
        overwrite: bool,
    ) -> Result<i64, String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;

        if overwrite {
            // Check if session exists for this meter
            if let Some(existing) = db.find_session_by_meter(&session.meter_serial, &session.meter_flag)
                .map_err(|e| e.to_string())? {
                db.update_session(existing.id, &session).map_err(|e| e.to_string())?;
                return Ok(existing.id);
            }
        }

        db.save_session(&session).map_err(|e| e.to_string())
    }

    /// Get a session by ID
    #[tauri::command]
    pub fn get_session(id: i64) -> Result<Option<Session>, String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.get_session(id).map_err(|e| e.to_string())
    }

    /// Get recent sessions
    #[tauri::command]
    pub fn get_recent_sessions(limit: u32) -> Result<Vec<Session>, String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.get_recent_sessions(limit).map_err(|e| e.to_string())
    }

    /// Delete a session
    #[tauri::command]
    pub fn delete_session(id: i64) -> Result<(), String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.delete_session(id).map_err(|e| e.to_string())
    }

    /// Save a report
    #[tauri::command]
    pub fn save_report(report: Report) -> Result<i64, String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.save_report(&report).map_err(|e| e.to_string())
    }

    /// Get recent reports
    #[tauri::command]
    pub fn get_recent_reports(limit: u32) -> Result<Vec<Report>, String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.get_recent_reports(limit).map_err(|e| e.to_string())
    }

    /// Get a setting value
    #[tauri::command]
    pub fn get_setting(key: String) -> Result<Option<String>, String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.get_setting(&key).map_err(|e| e.to_string())
    }

    /// Set a setting value
    #[tauri::command]
    pub fn set_setting(key: String, value: String) -> Result<(), String> {
        let guard = storage::get_database()?;
        let db = guard.as_ref().ok_or("Database not initialized")?;
        db.set_setting(&key, &value).map_err(|e| e.to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Initialize database
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");
            storage::init_database(&app_data_dir)
                .expect("Failed to initialize database");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Connection commands
            commands::list_serial_ports,
            commands::connect,
            commands::disconnect,
            commands::get_connection_status,
            commands::get_meter_identity,
            // Reading commands
            commands::read_short,
            commands::read_full,
            commands::read_obis,
            // Programming commands
            commands::authenticate,
            commands::write_obis,
            commands::sync_time,
            commands::end_session,
            // Database commands
            db_commands::save_session,
            db_commands::get_session,
            db_commands::get_recent_sessions,
            db_commands::delete_session,
            db_commands::save_report,
            db_commands::get_recent_reports,
            db_commands::get_setting,
            db_commands::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
