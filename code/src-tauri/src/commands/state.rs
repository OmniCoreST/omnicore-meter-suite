//! Connection state management
//!
//! Manages the global serial connection state for interactive sessions.

use crate::{ConnectionParams, MeterIdentity};
use once_cell::sync::Lazy;
use serialport::SerialPort;
use std::sync::Mutex;

/// Global connection state
pub static CONNECTION_STATE: Lazy<Mutex<ConnectionManager>> = Lazy::new(|| {
    Mutex::new(ConnectionManager::new())
});

/// Manages the serial connection and meter state
pub struct ConnectionManager {
    pub port: Option<Box<dyn SerialPort>>,
    pub params: Option<ConnectionParams>,
    pub identity: Option<MeterIdentity>,
    pub connected: bool,
    pub in_programming_mode: bool,
    pub negotiated_baud: u32,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            port: None,
            params: None,
            identity: None,
            connected: false,
            in_programming_mode: false,
            negotiated_baud: 300,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected && self.port.is_some()
    }

    pub fn disconnect(&mut self) {
        self.port = None;
        self.params = None;
        self.identity = None;
        self.connected = false;
        self.in_programming_mode = false;
        self.negotiated_baud = 300;
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

// Make ConnectionManager Send + Sync safe
// SAFETY: We access ConnectionManager through a Mutex, which provides synchronization
unsafe impl Send for ConnectionManager {}
unsafe impl Sync for ConnectionManager {}
