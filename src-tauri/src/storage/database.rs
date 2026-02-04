//! SQLite database for storing meter sessions and reports

use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Database connection singleton
static DATABASE: Lazy<Mutex<Option<Database>>> = Lazy::new(|| Mutex::new(None));

/// Session record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: i64,
    pub meter_serial: String,
    pub meter_model: String,
    pub meter_flag: String,
    pub timestamp: String,
    pub connection_type: String,
    pub result_status: String,
    pub note: Option<String>,
    pub data_json: String,
}

/// Report record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub id: i64,
    pub session_id: i64,
    pub report_type: String,
    pub filename: String,
    pub filepath: String,
    pub created_at: String,
}

/// App settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub key: String,
    pub value: String,
}

/// Database manager
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database connection
    pub fn new(db_path: &PathBuf) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Initialize database tables
    fn initialize(&self) -> SqlResult<()> {
        // Sessions table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                meter_serial TEXT NOT NULL,
                meter_model TEXT NOT NULL,
                meter_flag TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                connection_type TEXT NOT NULL,
                result_status TEXT NOT NULL,
                note TEXT,
                data_json TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Reports table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS reports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                report_type TEXT NOT NULL,
                filename TEXT NOT NULL,
                filepath TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            )",
            [],
        )?;

        // Settings table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_meter_serial ON sessions(meter_serial)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_timestamp ON sessions(timestamp DESC)",
            [],
        )?;

        Ok(())
    }

    /// Save a session
    pub fn save_session(&self, session: &Session) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO sessions (meter_serial, meter_model, meter_flag, timestamp, connection_type, result_status, note, data_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.meter_serial,
                session.meter_model,
                session.meter_flag,
                session.timestamp,
                session.connection_type,
                session.result_status,
                session.note,
                session.data_json,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Update a session (for overwriting)
    pub fn update_session(&self, id: i64, session: &Session) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE sessions SET
                meter_model = ?1,
                timestamp = ?2,
                result_status = ?3,
                note = ?4,
                data_json = ?5
             WHERE id = ?6",
            params![
                session.meter_model,
                session.timestamp,
                session.result_status,
                session.note,
                session.data_json,
                id,
            ],
        )?;
        Ok(())
    }

    /// Get session by ID
    pub fn get_session(&self, id: i64) -> SqlResult<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meter_serial, meter_model, meter_flag, timestamp, connection_type, result_status, note, data_json
             FROM sessions WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Session {
                id: row.get(0)?,
                meter_serial: row.get(1)?,
                meter_model: row.get(2)?,
                meter_flag: row.get(3)?,
                timestamp: row.get(4)?,
                connection_type: row.get(5)?,
                result_status: row.get(6)?,
                note: row.get(7)?,
                data_json: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find session by meter serial (for overwriting)
    pub fn find_session_by_meter(&self, meter_serial: &str, meter_flag: &str) -> SqlResult<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meter_serial, meter_model, meter_flag, timestamp, connection_type, result_status, note, data_json
             FROM sessions WHERE meter_serial = ?1 AND meter_flag = ?2
             ORDER BY timestamp DESC LIMIT 1"
        )?;

        let mut rows = stmt.query(params![meter_serial, meter_flag])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Session {
                id: row.get(0)?,
                meter_serial: row.get(1)?,
                meter_model: row.get(2)?,
                meter_flag: row.get(3)?,
                timestamp: row.get(4)?,
                connection_type: row.get(5)?,
                result_status: row.get(6)?,
                note: row.get(7)?,
                data_json: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get recent sessions
    pub fn get_recent_sessions(&self, limit: u32) -> SqlResult<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meter_serial, meter_model, meter_flag, timestamp, connection_type, result_status, note, data_json
             FROM sessions ORDER BY timestamp DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(Session {
                id: row.get(0)?,
                meter_serial: row.get(1)?,
                meter_model: row.get(2)?,
                meter_flag: row.get(3)?,
                timestamp: row.get(4)?,
                connection_type: row.get(5)?,
                result_status: row.get(6)?,
                note: row.get(7)?,
                data_json: row.get(8)?,
            })
        })?;

        rows.collect()
    }

    /// Delete a session
    pub fn delete_session(&self, id: i64) -> SqlResult<()> {
        // Delete associated reports first
        self.conn.execute("DELETE FROM reports WHERE session_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Save a report
    pub fn save_report(&self, report: &Report) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO reports (session_id, report_type, filename, filepath)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                report.session_id,
                report.report_type,
                report.filename,
                report.filepath,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get reports for a session
    pub fn get_reports_for_session(&self, session_id: i64) -> SqlResult<Vec<Report>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, report_type, filename, filepath, created_at
             FROM reports WHERE session_id = ?1 ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(Report {
                id: row.get(0)?,
                session_id: row.get(1)?,
                report_type: row.get(2)?,
                filename: row.get(3)?,
                filepath: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect()
    }

    /// Get recent reports
    pub fn get_recent_reports(&self, limit: u32) -> SqlResult<Vec<Report>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, report_type, filename, filepath, created_at
             FROM reports ORDER BY created_at DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(Report {
                id: row.get(0)?,
                session_id: row.get(1)?,
                report_type: row.get(2)?,
                filename: row.get(3)?,
                filepath: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect()
    }

    /// Get a setting value
    pub fn get_setting(&self, key: &str) -> SqlResult<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Set a setting value
    pub fn set_setting(&self, key: &str, value: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a setting
    pub fn delete_setting(&self, key: &str) -> SqlResult<()> {
        self.conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }
}

/// Initialize the database
pub fn init_database(app_data_dir: &PathBuf) -> Result<(), String> {
    let db_path = app_data_dir.join("omnicore.db");

    // Create directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let db = Database::new(&db_path).map_err(|e| e.to_string())?;

    let mut guard = DATABASE.lock().map_err(|e| e.to_string())?;
    *guard = Some(db);

    log::info!("Database initialized at {:?}", db_path);
    Ok(())
}

/// Get database instance
pub fn get_database() -> Result<std::sync::MutexGuard<'static, Option<Database>>, String> {
    DATABASE.lock().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_database_operations() {
        // Create temp database
        let temp_dir = std::env::temp_dir().join("omnicore_test");
        fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        let db = Database::new(&db_path).unwrap();

        // Test session operations
        let session = Session {
            id: 0,
            meter_serial: "123456789".to_string(),
            meter_model: "M550.2251".to_string(),
            meter_flag: "MKS".to_string(),
            timestamp: "2024-12-15 14:30:00".to_string(),
            connection_type: "optical".to_string(),
            result_status: "success".to_string(),
            note: Some("Test session".to_string()),
            data_json: "{}".to_string(),
        };

        let id = db.save_session(&session).unwrap();
        assert!(id > 0);

        let retrieved = db.get_session(id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().meter_serial, "123456789");

        // Test settings
        db.set_setting("theme", "dark").unwrap();
        let value = db.get_setting("theme").unwrap();
        assert_eq!(value, Some("dark".to_string()));

        // Cleanup
        fs::remove_file(&db_path).ok();
    }
}
