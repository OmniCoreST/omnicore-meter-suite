//! Tauri commands for meter communication

use crate::{PortInfo, MeterIdentity, ConnectionParams};
#[allow(unused_imports)]
use crate::serial::{self, iec62056};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serialport::SerialPort;
use tauri::Emitter;

/// Global connection state
static CONNECTION_STATE: Lazy<Mutex<ConnectionManager>> = Lazy::new(|| {
    Mutex::new(ConnectionManager::new())
});

/// Manages the serial connection and meter state
struct ConnectionManager {
    port: Option<Box<dyn SerialPort>>,
    params: Option<ConnectionParams>,
    identity: Option<MeterIdentity>,
    connected: bool,
    in_programming_mode: bool,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            port: None,
            params: None,
            identity: None,
            connected: false,
            in_programming_mode: false,
        }
    }

    fn is_connected(&self) -> bool {
        self.connected && self.port.is_some()
    }

    fn disconnect(&mut self) {
        self.port = None;
        self.params = None;
        self.identity = None;
        self.connected = false;
        self.in_programming_mode = false;
    }
}

// Make ConnectionManager Send + Sync safe
unsafe impl Send for ConnectionManager {}
unsafe impl Sync for ConnectionManager {}

/// Short read result data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortReadResult {
    // Meter Identity
    pub serial_number: String,
    pub program_version: String,
    pub production_date: String,
    pub calibration_date: String,

    // Date/Time
    pub meter_date: String,
    pub meter_time: String,
    pub day_of_week: u8,

    // Active Energy Import
    pub active_energy_import_total: f64,
    pub active_energy_import_t1: f64,
    pub active_energy_import_t2: f64,
    pub active_energy_import_t3: f64,
    pub active_energy_import_t4: f64,

    // Maximum Demand
    pub max_demand_import: f64,
    pub max_demand_import_timestamp: String,

    // Instantaneous Values
    pub voltage_l1: f64,
    pub voltage_l2: f64,
    pub voltage_l3: f64,
    pub current_l1: f64,
    pub current_l2: f64,
    pub current_l3: f64,
    pub frequency: f64,
    pub power_factor_l1: f64,
    pub power_factor_l2: f64,
    pub power_factor_l3: f64,

    // Status Codes
    pub ff_code: String,
    pub gf_code: String,
    pub battery_status: String,
    pub relay_status: String,
}

/// Progress event for reading operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub step: u32,
    pub total: u32,
    pub message: String,
}

/// Log event for communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub timestamp: String,
    pub log_type: String,
    pub message: String,
    pub data: Option<String>,
}

/// List all available serial ports
#[tauri::command]
pub fn list_serial_ports() -> Result<Vec<PortInfo>, String> {
    log::info!("Listing serial ports");
    serial::list_ports()
}

/// Connect to a meter
#[tauri::command]
pub async fn connect(params: ConnectionParams) -> Result<MeterIdentity, String> {
    log::info!("Connecting to meter on port: {}", params.port);

    // Disconnect any existing connection first
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.is_connected() {
            manager.disconnect();
        }
    } // Drop the lock before await

    // For now, simulate a connection since we don't have real hardware
    // In production, this would perform the IEC 62056-21 handshake

    // Simulate the handshake process
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let identity = MeterIdentity {
        manufacturer: "MKS".to_string(),
        edas_id: "ADM".to_string(),
        model: "M550.2251".to_string(),
        baud_rate_char: "5".to_string(),
        generation: "2".to_string(),
        serial_number: Some("123456789".to_string()),
    };

    // Store connection state (reacquire lock after await)
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        manager.params = Some(params);
        manager.identity = Some(identity.clone());
        manager.connected = true;
    }

    log::info!("Connected to meter: {:?}", identity);
    Ok(identity)
}

/// Disconnect from the meter
#[tauri::command]
pub async fn disconnect() -> Result<(), String> {
    log::info!("Disconnecting from meter");

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    manager.disconnect();

    Ok(())
}

/// Get current connection status
#[tauri::command]
pub fn get_connection_status() -> Result<bool, String> {
    let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    Ok(manager.connected)
}

/// Get current meter identity
#[tauri::command]
pub fn get_meter_identity() -> Result<Option<MeterIdentity>, String> {
    let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    Ok(manager.identity.clone())
}

/// Perform a short read operation
#[tauri::command]
pub async fn read_short(window: tauri::Window) -> Result<ShortReadResult, String> {
    log::info!("Starting short read operation");

    // Check connection status (scope ensures lock is dropped before async)
    {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if !manager.connected {
            return Err("Not connected to meter".to_string());
        }
    }

    // Emit progress events
    let emit_progress = |step: u32, total: u32, message: &str| {
        let _ = window.emit("read-progress", ProgressEvent {
            step,
            total,
            message: message.to_string(),
        });
    };

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    let total_steps = 8;

    // Step 1: Opening port
    emit_progress(1, total_steps, "Opening serial port...");
    emit_log("info", "Opening serial port");
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Step 2: Sending handshake
    emit_progress(2, total_steps, "Sending handshake...");
    emit_log("tx", "/?!");
    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

    // Step 3: Identifying device
    emit_progress(3, total_steps, "Identifying device...");
    emit_log("rx", "/MKS5<2>ADM(M550.2251)");
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Step 4: Switching baud rate
    emit_progress(4, total_steps, "Switching baud rate...");
    emit_log("info", "Switching to 9600 baud");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Step 5: Requesting short packet
    emit_progress(5, total_steps, "Requesting short read packet...");
    emit_log("tx", "ACK 6 5 6");
    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

    // Step 6: Receiving data
    emit_progress(6, total_steps, "Receiving data...");
    emit_log("rx", "Data block received (2048 bytes)");
    tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

    // Step 7: Parsing data
    emit_progress(7, total_steps, "Parsing data...");
    emit_log("info", "Parsing OBIS codes");
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Step 8: Complete
    emit_progress(8, total_steps, "Complete");
    emit_log("success", "Short read completed successfully");

    // Return mock data
    Ok(ShortReadResult {
        serial_number: "123456789".to_string(),
        program_version: "V01.00".to_string(),
        production_date: "2024-06-30".to_string(),
        calibration_date: "2024-06-30".to_string(),
        meter_date: "2024-12-15".to_string(),
        meter_time: "14:30:35".to_string(),
        day_of_week: 4,
        active_energy_import_total: 123456.789,
        active_energy_import_t1: 45678.123,
        active_energy_import_t2: 34567.234,
        active_energy_import_t3: 43211.432,
        active_energy_import_t4: 0.0,
        max_demand_import: 123.456,
        max_demand_import_timestamp: "2024-02-01 13:30".to_string(),
        voltage_l1: 220.5,
        voltage_l2: 221.3,
        voltage_l3: 219.8,
        current_l1: 16.5,
        current_l2: 15.8,
        current_l3: 17.2,
        frequency: 49.9,
        power_factor_l1: 0.97,
        power_factor_l2: 0.96,
        power_factor_l3: 0.98,
        ff_code: "0000000000000090".to_string(),
        gf_code: "0000000000000004".to_string(),
        battery_status: "full".to_string(),
        relay_status: "active".to_string(),
    })
}

/// Read a specific OBIS code
#[tauri::command]
pub async fn read_obis(obis_code: String) -> Result<String, String> {
    log::info!("Reading OBIS code: {}", obis_code);

    let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }

    // Mock response based on OBIS code
    let value = match obis_code.as_str() {
        "0.0.0" => "123456789",
        "0.9.1" => "14:30:35",
        "0.9.2" => "2024-12-15",
        "1.8.0" => "123456.789*kWh",
        "1.8.1" => "45678.123*kWh",
        "1.8.2" => "34567.234*kWh",
        "1.8.3" => "43211.432*kWh",
        _ => "0",
    };

    Ok(value.to_string())
}

/// Write a value to an OBIS code (requires programming mode)
#[tauri::command]
pub async fn write_obis(obis_code: String, value: String) -> Result<(), String> {
    log::info!("Writing OBIS code: {} = {}", obis_code, value);

    let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }
    if !manager.in_programming_mode {
        return Err("Meter is not in programming mode".to_string());
    }

    // In production, this would send the write command
    Ok(())
}

/// Authenticate with the meter (enter programming mode)
#[tauri::command]
pub async fn authenticate(password: String) -> Result<bool, String> {
    log::info!("Authenticating with meter");

    // Check connection and validate password
    {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if !manager.connected {
            return Err("Not connected to meter".to_string());
        }
    } // Drop the lock before await

    // Validate password format (8 digits)
    if password.len() != 8 || !password.chars().all(|c| c.is_ascii_digit()) {
        return Err("Password must be exactly 8 digits".to_string());
    }

    // In production, this would send the password command
    // For now, simulate authentication
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Set programming mode (reacquire lock after await)
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        manager.in_programming_mode = true;
    }

    Ok(true)
}

/// Sync meter time to computer time
#[tauri::command]
pub async fn sync_time() -> Result<(), String> {
    log::info!("Syncing meter time");

    let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }
    if !manager.in_programming_mode {
        return Err("Meter is not in programming mode".to_string());
    }

    // In production, this would write the time OBIS codes
    let now = chrono::Local::now();
    let _time_str = now.format("%H:%M:%S").to_string();
    let _date_str = now.format("%y-%m-%d").to_string();
    let _dow = now.format("%u").to_string(); // 1-7, Monday = 1

    Ok(())
}

/// End the programming session
#[tauri::command]
pub async fn end_session() -> Result<(), String> {
    log::info!("Ending programming session");

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }

    // In production, this would send the break command
    manager.in_programming_mode = false;
    Ok(())
}
