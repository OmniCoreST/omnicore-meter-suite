//! Event emission utilities
//!
//! Provides a centralized way to emit communication events to the frontend,
//! eliminating duplicate emit_log closures throughout the codebase.

use super::types::{LogEvent, ProgressEvent};
use tauri::{Emitter, Window};

/// Event emitter for meter communication
///
/// Wraps a Tauri Window and provides convenient methods for emitting
/// log, progress, and activity events to the frontend.
pub struct EventEmitter<'a> {
    window: &'a Window,
}

impl<'a> EventEmitter<'a> {
    /// Create a new EventEmitter for the given window
    pub fn new(window: &'a Window) -> Self {
        Self { window }
    }

    /// Emit a communication log event
    ///
    /// # Arguments
    /// * `log_type` - Type of log: "tx", "rx", "info", "error", "success", "warning"
    /// * `message` - The log message
    /// * `data` - Optional data payload (e.g., hex dump)
    pub fn log(&self, log_type: &str, message: &str, data: Option<&str>) {
        let _ = self.window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
    }

    /// Emit a log event without data (convenience method)
    pub fn log_simple(&self, log_type: &str, message: &str) {
        self.log(log_type, message, None);
    }

    /// Emit a progress event
    ///
    /// # Arguments
    /// * `step` - Current step number (1-based)
    /// * `total` - Total number of steps
    /// * `message` - Progress message
    pub fn progress(&self, step: u32, total: u32, message: &str) {
        let _ = self.window.emit("read-progress", ProgressEvent {
            step,
            total,
            message: message.to_string(),
        });
    }

    /// Emit an activity event (for UI indicators)
    ///
    /// # Arguments
    /// * `activity_type` - Type of activity: "tx", "rx", "idle"
    pub fn activity(&self, activity_type: &str) {
        let _ = self.window.emit("comm-activity", serde_json::json!({
            "type": activity_type
        }));
    }

    /// Log transmit data with hex representation
    pub fn log_tx(&self, description: &str, data: &[u8]) {
        let hex = data.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.log("tx", description, Some(&hex));
        self.activity("tx");
    }

    /// Log receive data with hex representation
    pub fn log_rx(&self, description: &str, data: &[u8]) {
        let hex = data.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.log("rx", description, Some(&hex));
        self.activity("rx");
    }

    /// Log a formatted info message
    pub fn info(&self, message: &str) {
        self.log_simple("info", message);
    }

    /// Log an error message
    pub fn error(&self, message: &str) {
        self.log_simple("error", message);
    }

    /// Log a success message
    pub fn success(&self, message: &str) {
        self.log_simple("success", message);
    }

    /// Log a warning message
    pub fn warning(&self, message: &str) {
        self.log_simple("warning", message);
    }
}
