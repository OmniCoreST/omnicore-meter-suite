//! Session log file management
//!
//! Opens a timestamped log file at startup and appends every comm-log event to it.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_FILE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Called once at application startup. Creates comm_logs/{timestamp}.txt under app_data_dir.
pub fn init_session_log(app_data_dir: PathBuf) {
    let log_dir = app_data_dir.join("comm_logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let path = log_dir.join(format!("comm_{}.txt", ts));
    let _ = std::fs::File::create(&path);
    if let Ok(mut guard) = LOG_FILE_PATH.lock() {
        *guard = Some(path);
    }
}

/// Appends a single log line to the active session log file.
pub fn write_log(timestamp: &str, log_type: &str, message: &str) {
    if let Ok(guard) = LOG_FILE_PATH.lock() {
        if let Some(ref path) = *guard {
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path) {
                let _ = writeln!(file, "[{}] [{}] {}", timestamp, log_type.to_uppercase(), message);
            }
        }
    }
}

/// Parses a space-separated hex string and appends a formatted hex dump block.
///
/// Example output (16 bytes per line):
///   0000  2F 3F 21 0D 0A                                     /?!..
pub fn write_hex_dump(hex_str: &str) {
    let bytes: Vec<u8> = hex_str
        .split_whitespace()
        .filter_map(|h| u8::from_str_radix(h, 16).ok())
        .collect();

    if bytes.is_empty() {
        return;
    }

    if let Ok(guard) = LOG_FILE_PATH.lock() {
        if let Some(ref path) = *guard {
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path) {
                for (i, chunk) in bytes.chunks(16).enumerate() {
                    let offset = i * 16;
                    let hex_cols: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
                    // Pad hex section to 48 chars (16 × "XX ") for alignment
                    let hex_padded = format!("{:<48}", hex_cols.join(" "));
                    let ascii: String = chunk.iter().map(|&b| {
                        if (0x20..=0x7E).contains(&b) { b as char } else { '.' }
                    }).collect();
                    let _ = writeln!(file, "  {:04X}  {}  {}", offset, hex_padded, ascii);
                }
            }
        }
    }
}

/// Returns the full path of the current session log file (for display in the UI).
pub fn log_file_path() -> Option<PathBuf> {
    LOG_FILE_PATH.lock().ok()?.clone()
}
