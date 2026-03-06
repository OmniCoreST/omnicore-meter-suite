use serde::{Deserialize, Serialize};
use tauri::Manager;

mod serial;
mod commands;
mod storage;
mod i18n;
pub mod compliance;

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

// File export command - writes binary data to a given path
#[tauri::command]
fn write_export_file(path: String, data: Vec<u8>) -> Result<String, String> {
    std::fs::write(&path, &data).map_err(|e| format!("Dosya kaydedilemedi: {}", e))?;
    Ok(path)
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

mod compliance_commands {
    use crate::compliance;

    /// Sayaç verisini kural dosyasına göre kontrol eder (legacy v2 compat).
    /// Önce sunucudan güncel versiyon bilgisini çeker (offline ise None).
    #[tauri::command]
    pub async fn check_compliance(
        data: crate::commands::types::ShortReadResult,
        meter_phases: u8,
    ) -> Result<compliance::ComplianceResult, String> {
        let latest = compliance::updater::fetch_latest_version().await;
        let latest_version = latest.map(|v| v.version);
        Ok(compliance::run_check_legacy(&data, latest_version, meter_phases))
    }

    /// Run compliance check on a CommunicationLog (v3 API).
    #[tauri::command]
    pub async fn check_compliance_v3(
        log: compliance::CommunicationLog,
        profile_id: String,
    ) -> Result<compliance::ComplianceResult, String> {
        let latest = compliance::updater::fetch_latest_version().await;
        let latest_version = latest.map(|v| v.version);
        Ok(compliance::run_check(&log, &profile_id, latest_version))
    }

    /// Get available profiles from config
    #[tauri::command]
    pub async fn get_compliance_profiles() -> Result<Vec<compliance::config::Profile>, String> {
        let config = compliance::config::load_config().map_err(|e| e.to_string())?;
        Ok(config.profiles)
    }

    /// Get test plan from config
    #[tauri::command]
    pub async fn get_compliance_test_plan() -> Result<Option<compliance::config::TestPlan>, String> {
        let config = compliance::config::load_config().map_err(|e| e.to_string())?;
        Ok(config.test_plan)
    }

    /// Kural dosyasının tam yolunu döndürür
    #[tauri::command]
    pub async fn get_compliance_rules_path() -> Result<String, String> {
        Ok(compliance::config::get_config_path().display().to_string())
    }

    /// Kural dosyasını diskten yeniden yükler (geçerliliği test eder)
    #[tauri::command]
    pub async fn reload_compliance_rules() -> Result<String, String> {
        let config = compliance::config::load_config()
            .map_err(|e| format!("Config dosyası yüklenemedi: {}", e))?;
        Ok(format!(
            "Config yüklendi: v{} ({} profil, {} kural)",
            config.config_version,
            config.profiles.len(),
            config.rules.len()
        ))
    }

    /// Sunucudan yeni kural dosyası indirir
    #[tauri::command]
    pub async fn update_compliance_rules() -> Result<String, String> {
        let info = compliance::updater::fetch_latest_version().await
            .ok_or_else(|| "Güncelleme sunucusuna ulaşılamadı veya URL yapılandırılmamış".to_string())?;

        let local_version = compliance::config::load_config()
            .map(|r| r.config_version)
            .unwrap_or_default();

        if info.version == local_version {
            return Ok(format!("Zaten güncel (v{})", local_version));
        }

        let new_version = compliance::updater::download_rules(&info.url).await?;
        Ok(format!("Config güncellendi: v{}", new_version))
    }

    /// Mevcut tüm kuralları listeler
    #[tauri::command]
    pub async fn list_compliance_rules() -> Result<Vec<compliance::config::Rule>, String> {
        let config = compliance::config::load_config().map_err(|e| e.to_string())?;
        Ok(config.rules)
    }

    /// Varolan bir kuralı code'una göre günceller
    #[tauri::command]
    pub async fn update_compliance_rule(rule: compliance::config::Rule) -> Result<String, String> {
        let mut config = compliance::config::load_config().map_err(|e| e.to_string())?;
        if let Some(existing) = config.rules.iter_mut().find(|r| r.code == rule.code) {
            *existing = rule;
            compliance::config::save_config(&config).map_err(|e| e.to_string())?;
            Ok("Kural güncellendi".to_string())
        } else {
            Err(format!("Kural bulunamadı: {}", rule.code))
        }
    }

    /// Bir kuralı code'una göre siler
    #[tauri::command]
    pub async fn delete_compliance_rule(code: String) -> Result<String, String> {
        let mut config = compliance::config::load_config().map_err(|e| e.to_string())?;
        let len_before = config.rules.len();
        config.rules.retain(|r| r.code != code);
        if config.rules.len() < len_before {
            compliance::config::save_config(&config).map_err(|e| e.to_string())?;
            Ok("Kural silindi".to_string())
        } else {
            Err(format!("Kural bulunamadı: {}", code))
        }
    }

    /// Yerel dosyadan kural dosyasını içe aktarır
    #[tauri::command]
    pub async fn import_compliance_rules_from_file(path: String) -> Result<String, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Dosya okunamadı: {}", e))?;
        let parsed: compliance::config::ComplianceConfig = toml::from_str(&content)
            .map_err(|e| format!("Geçersiz TOML dosyası: {}", e))?;
        let new_version = parsed.config_version.clone();
        let dest = compliance::config::get_config_path();
        std::fs::write(&dest, content.as_bytes())
            .map_err(|e| format!("Dosya yazılamadı: {}", e))?;
        Ok(format!("Config içe aktarıldı: v{}", new_version))
    }

    /// Verilen TOML bloğunu kural dosyasına ekler
    #[tauri::command]
    pub async fn add_compliance_rule(rule_toml: String) -> Result<String, String> {
        use std::io::Write;
        let path = compliance::config::get_config_path();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(|e| format!("Dosya açılamadı: {}", e))?;
        writeln!(file, "\n{}", rule_toml)
            .map_err(|e| format!("Kural yazılamadı: {}", e))?;
        Ok("Kural başarıyla eklendi".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Remove default menu bar
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.remove_menu();
            }
            // Initialize database
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");
            storage::init_database(&app_data_dir)
                .expect("Failed to initialize database");
            commands::logger::init_session_log(app_data_dir);
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
            commands::read_obis_batch,
            commands::read_load_profile,
            commands::read_packet,
            // Programming commands
            commands::authenticate,
            commands::write_obis,
            commands::change_password,
            commands::sync_time,
            commands::end_session,
            // Session file commands
            commands::sessions::save_session_file,
            commands::sessions::list_session_files,
            commands::sessions::load_session_file,
            commands::sessions::delete_session_file,
            // Database commands
            db_commands::save_session,
            db_commands::get_session,
            db_commands::get_recent_sessions,
            db_commands::delete_session,
            db_commands::save_report,
            db_commands::get_recent_reports,
            db_commands::get_setting,
            db_commands::set_setting,
            write_export_file,
            // Uyumluluk komutları
            compliance_commands::check_compliance,
            compliance_commands::check_compliance_v3,
            compliance_commands::get_compliance_profiles,
            compliance_commands::get_compliance_test_plan,
            compliance_commands::get_compliance_rules_path,
            compliance_commands::reload_compliance_rules,
            compliance_commands::update_compliance_rules,
            compliance_commands::list_compliance_rules,
            compliance_commands::update_compliance_rule,
            compliance_commands::delete_compliance_rule,
            compliance_commands::import_compliance_rules_from_file,
            compliance_commands::add_compliance_rule,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
