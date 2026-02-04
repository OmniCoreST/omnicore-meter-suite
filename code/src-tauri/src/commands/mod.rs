//! Tauri commands for meter communication

use crate::{PortInfo, MeterIdentity, ConnectionParams};
use crate::serial::iec62056::{self, ProtocolMode, control};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::io::{Read, Write};
use std::time::Duration;
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
    negotiated_baud: u32,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            port: None,
            params: None,
            identity: None,
            connected: false,
            in_programming_mode: false,
            negotiated_baud: 300,
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
        self.negotiated_baud = 300;
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

    // Raw data for debugging
    pub raw_data: Option<String>,
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
    crate::serial::list_ports()
}

/// Connect to a meter
#[tauri::command]
pub async fn connect(params: ConnectionParams, window: tauri::Window) -> Result<MeterIdentity, String> {
    log::info!("Connecting to meter on port: {} at {} baud", params.port, params.baud_rate);

    let emit_log = |log_type: &str, message: &str, data: Option<&str>| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
    };

    // Disconnect any existing connection first
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.is_connected() {
            manager.disconnect();
        }
    }

    let port_name = params.port.clone();
    let timeout_ms = if params.timeout_ms == 0 { 2000 } else { params.timeout_ms };
    let meter_address = params.meter_address.clone();
    let is_optical = params.connection_type == "optical";
    let is_auto = params.connection_type == "auto";

    // Determine initial baud rate based on connection type
    // - Optical: Always start at 300 bps (IEC 62056-21 Mode C requirement)
    // - Auto + specific baud rate: Start at 300 bps first, then negotiate
    // - Auto + auto baud rate (0): Try 9600 first, then 19200
    // - RS485/other + specific baud rate: Use that baud rate directly
    // - RS485/other + auto baud rate (0): Try 9600, then 19200
    let baud_rates_to_try: Vec<u32> = if is_optical {
        // Optical always starts at 300 baud
        vec![300]
    } else if is_auto && params.baud_rate > 0 {
        // Auto connection type with specific baud rate: start at 300 for IEC handshake
        vec![300]
    } else if params.baud_rate == 0 {
        // Auto baud rate detection: try 9600 first, then 19200
        vec![9600, 19200]
    } else {
        // Specific baud rate requested for non-optical/non-auto connection
        vec![params.baud_rate]
    };

    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut successful_baud: u32 = 0;
    let mut total_read: usize = 0;
    let mut response_buf = vec![0u8; 256];

    for (attempt, &try_baud) in baud_rates_to_try.iter().enumerate() {
        emit_log("info", &format!("Seri port açılıyor: {} @ {} baud (7E1) [Deneme {}/{}]",
            port_name, try_baud, attempt + 1, baud_rates_to_try.len()), None);

        // Open serial port with IEC 62056-21 settings (7E1)
        let mut current_port = match iec62056::open_port(&port_name, try_baud, timeout_ms as u64) {
            Ok(p) => p,
            Err(e) => {
                emit_log("warn", &format!("Port açılamadı @ {} baud: {}", try_baud, e), None);
                continue;
            }
        };

        emit_log("success", &format!("Port açıldı @ {} baud", try_baud), None);

        // Build and send handshake request
        let request = iec62056::build_request_message(meter_address.as_deref());
        let request_str = iec62056::format_bytes_for_display(&request);
        emit_log("tx", &request_str, None);

        if let Err(e) = current_port.write_all(&request) {
            emit_log("warn", &format!("Handshake gönderilemedi: {}", e), None);
            continue;
        }
        let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
        let _ = current_port.flush();

        // Wait for response
        emit_log("info", "Yanıt bekleniyor...", None);
        std::thread::sleep(Duration::from_millis(500));

        // Read identification response
        response_buf.fill(0);
        total_read = 0;
        let start_time = std::time::Instant::now();

        loop {
            match current_port.read(&mut response_buf[total_read..]) {
                Ok(n) if n > 0 => {
                    total_read += n;
                    // Check if we have a complete response (ends with CR LF)
                    if total_read >= 2 &&
                       response_buf[total_read - 2] == control::CR &&
                       response_buf[total_read - 1] == control::LF {
                        break;
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if total_read > 0 {
                        break;
                    }
                }
                Err(_) => break,
            }

            if start_time.elapsed() > Duration::from_millis(timeout_ms as u64) {
                break;
            }
        }

        if total_read > 0 {
            // We got a response!
            successful_baud = try_baud;
            port = Some(current_port);
            emit_log("success", &format!("Yanıt alındı @ {} baud ({} byte)", try_baud, total_read), None);
            break;
        } else {
            emit_log("warn", &format!("{} baud'da yanıt alınamadı", try_baud), None);
        }
    }

    let mut port = port.ok_or_else(|| {
        emit_log("error", "Hiçbir baud hızında yanıt alınamadı", None);
        "No response at any baud rate".to_string()
    })?;

    if total_read == 0 {
        emit_log("error", "Sayaçtan yanıt alınamadı", None);
        return Err("No response from meter".to_string());
    }

    let response_formatted = iec62056::format_bytes_for_display(&response_buf[..total_read]);
    emit_log("rx", &response_formatted, None);

    // Parse identification (need string version for parsing)
    let response = String::from_utf8_lossy(&response_buf[..total_read]);
    let ident = iec62056::parse_identification(&response)
        .ok_or_else(|| {
            emit_log("error", "Sayaç tanımlama yanıtı ayrıştırılamadı", None);
            "Failed to parse meter identification".to_string()
        })?;

    emit_log("success", &format!("Sayaç tanımlandı: {} — {} ({})",
        ident.manufacturer, ident.edas_id, ident.model), None);

    // Determine the baud rate to use
    let target_baud = if params.baud_rate > 0 && params.baud_rate != 300 {
        // User specified a fixed baud rate
        params.baud_rate
    } else {
        // Use the meter's maximum supported baud rate
        ident.max_baud_rate
    };

    let baud_char = iec62056::char_from_baud_rate(target_baud)
        .unwrap_or(ident.baud_char);

    // Send ACK for full readout mode (Mode 0 - gets all data)
    let ack = iec62056::build_ack_message(ProtocolMode::Readout, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    emit_log("tx", &ack_formatted, None);

    port.write_all(&ack).map_err(|e| format!("Failed to send ACK: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Wait before baud rate switch
    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", successful_baud, target_baud), None);
    std::thread::sleep(Duration::from_millis(300));

    // Switch baud rate if needed
    if target_baud != successful_baud {
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
            format!("Failed to switch baud rate: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud), None);
    }

    // Create meter identity
    let identity = MeterIdentity {
        manufacturer: ident.manufacturer.clone(),
        edas_id: ident.edas_id.clone(),
        model: ident.model.clone(),
        baud_rate_char: ident.baud_char.to_string(),
        generation: ident.generation.clone(),
        serial_number: None, // Will be read from short packet
    };

    // Store connection state
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        manager.port = Some(port);
        manager.params = Some(params);
        manager.identity = Some(identity.clone());
        manager.connected = true;
        manager.negotiated_baud = target_baud;
    }

    emit_log("success", "Bağlantı başarılı!", None);
    Ok(identity)
}

/// Disconnect from the meter
#[tauri::command]
pub async fn disconnect() -> Result<(), String> {
    log::info!("Disconnecting from meter");

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;

    // Send break command if connected
    if let Some(ref mut port) = manager.port {
        let break_cmd = iec62056::build_break_command();
        let _ = port.write_all(&break_cmd);
        let _ = port.flush();
    }

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

/// Perform a full read operation (Mode 0 - all data)
#[tauri::command]
pub async fn read_full(window: tauri::Window) -> Result<ShortReadResult, String> {
    log::info!("Starting full read operation");

    let emit_progress = |step: u32, total: u32, message: &str| {
        let _ = window.emit("read-progress", ProgressEvent {
            step,
            total,
            message: message.to_string(),
        });
    };

    let emit_log = |log_type: &str, message: &str, data: Option<&str>| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
    };

    let total_steps = 5;

    // Check connection and get port
    emit_progress(1, total_steps, "Bağlantı kontrol ediliyor...");

    let timeout_ms = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if !manager.connected {
            return Err("Not connected to meter".to_string());
        }
        manager.params.as_ref().map(|p| p.timeout_ms).unwrap_or(2000)
    };

    emit_progress(2, total_steps, "Tam okuma paketi alınıyor...");
    emit_log("info", "Tam okuma paketi bekleniyor (Mod 0 - Tüm veriler)...", None);

    // Read the data block from meter
    // After ACK with Mode 0 in connect(), meter should send all data
    // Increase buffer size significantly - meters can send 100KB+ of data
    let mut data_buf = vec![0u8; 131072]; // 128KB buffer for full readout
    let mut total_read = 0;
    let mut found_etx = false;
    let start_time = std::time::Instant::now();

    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        let port = manager.port.as_mut().ok_or("Port not available")?;

        // Wait a bit for data to start arriving
        std::thread::sleep(Duration::from_millis(300));

        let mut last_read_time = std::time::Instant::now();

        loop {
            match port.read(&mut data_buf[total_read..]) {
                Ok(n) if n > 0 => {
                    total_read += n;
                    last_read_time = std::time::Instant::now(); // Reset idle timer on every successful read
                    // Emit RX activity for LED indicator
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                    // Check for end of data block (ETX followed by BCC)
                    if total_read >= 2 {
                        // Look for ETX in the data
                        for i in 0..total_read-1 {
                            if data_buf[i] == control::ETX {
                                // Found ETX, check if we have BCC too (next byte)
                                if i + 1 < total_read {
                                    // We have ETX and BCC, transmission complete
                                    found_etx = true;
                                    break;
                                }
                            }
                        }
                        if found_etx {
                            emit_log("info", &format!("Veri alımı tamamlandı: {} byte, süre: {:.1}s", total_read, start_time.elapsed().as_secs_f32()), None);
                            break; // Exit the read loop
                        }
                    }
                }
                Ok(_) => {
                    // No data in this read, wait a bit
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // Timeout is expected when waiting for data, continue
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    emit_log("error", &format!("Okuma hatası: {}", e), None);
                    return Err(format!("Read error: {}", e));
                }
            }

            // Timeout only if NO data received for 5 seconds (idle timeout)
            // This means meter stopped sending but we haven't received ETX yet
            let idle_time = last_read_time.elapsed();
            if idle_time > Duration::from_millis(5000) {
                if total_read == 0 {
                    emit_log("error", "Zaman aşımı: Hiç veri alınamadı (5s boşta)", None);
                } else {
                    emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı ama ETX yok (5s boşta)", total_read), None);
                }
                break;
            }
        }
    }

    if total_read == 0 {
        emit_log("error", "Veri alınamadı", None);
        return Err("No data received from meter".to_string());
    }

    if !found_etx {
        emit_log("warn", &format!("Veri tam alınamadı: ETX bulunamadı ({} byte alındı)", total_read), None);
    }

    emit_progress(3, total_steps, "Veriler alındı, doğrulanıyor...");

    // Verify BCC if we found ETX
    if found_etx {
        // Find ETX position
        if let Some(etx_idx) = data_buf[..total_read].iter().position(|&b| b == control::ETX) {
            if etx_idx + 1 < total_read {
                let received_bcc = data_buf[etx_idx + 1];
                // Calculate expected BCC (XOR from byte after STX to ETX inclusive)
                if let Some(stx_idx) = data_buf[..total_read].iter().position(|&b| b == control::STX) {
                    let calculated_bcc = iec62056::calculate_bcc(&data_buf[stx_idx+1..=etx_idx]);
                    if calculated_bcc != received_bcc {
                        emit_log("warn", &format!("BCC uyuşmazlığı: beklenen 0x{:02X}, alınan 0x{:02X}", calculated_bcc, received_bcc), None);
                    } else {
                        emit_log("success", &format!("BCC doğrulaması başarılı ({} byte)", total_read), None);
                    }
                }
            }
        }
    }

    // Convert to string for parsing and format for display
    let raw_data = String::from_utf8_lossy(&data_buf[..total_read]).to_string();
    let data_formatted = iec62056::format_bytes_for_display(&data_buf[..total_read]);
    emit_log("rx", &data_formatted, None);

    emit_progress(4, total_steps, "OBIS kodları çözümleniyor...");

    // Parse the OBIS data
    let items = iec62056::parse_data_block(&raw_data);
    emit_log("info", &format!("{} OBIS kodu ayrıştırıldı", items.len()), None);

    // Extract values from parsed items
    let get_value = |code: &str| -> String {
        items.iter()
            .find(|item| item.code == code)
            .map(|item| item.value.clone())
            .unwrap_or_default()
    };

    let get_float = |code: &str| -> f64 {
        get_value(code).parse().unwrap_or(0.0)
    };

    emit_progress(5, total_steps, "Tamamlandı!");
    emit_log("success", "Tam okuma başarıyla tamamlandı", None);

    // Build result (same structure as short read)
    let result = ShortReadResult {
        serial_number: get_value("0.0.0"),
        program_version: get_value("0.2.0"),
        production_date: get_value("96.1.3"),
        calibration_date: get_value("96.2.5"),
        meter_date: get_value("0.9.2"),
        meter_time: get_value("0.9.1"),
        day_of_week: get_value("0.9.5").parse().unwrap_or(0),
        active_energy_import_total: get_float("1.8.0"),
        active_energy_import_t1: get_float("1.8.1"),
        active_energy_import_t2: get_float("1.8.2"),
        active_energy_import_t3: get_float("1.8.3"),
        active_energy_import_t4: get_float("1.8.4"),
        max_demand_import: get_float("1.6.0"),
        max_demand_import_timestamp: get_value("1.6.0"),
        voltage_l1: get_float("32.7.0"),
        voltage_l2: get_float("52.7.0"),
        voltage_l3: get_float("72.7.0"),
        current_l1: get_float("31.7.0"),
        current_l2: get_float("51.7.0"),
        current_l3: get_float("71.7.0"),
        frequency: get_float("14.7.0"),
        power_factor_l1: get_float("33.7.0"),
        power_factor_l2: get_float("53.7.0"),
        power_factor_l3: get_float("73.7.0"),
        ff_code: get_value("F.F.0"),
        gf_code: get_value("F.F.1"),
        battery_status: if get_value("96.6.1").contains("0") { "low".to_string() } else { "full".to_string() },
        relay_status: if get_value("96.3.10").contains("1") { "active".to_string() } else { "passive".to_string() },
        raw_data: Some(raw_data),
    };

    // Update identity with serial number
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut identity) = manager.identity {
            identity.serial_number = Some(result.serial_number.clone());
        }
    }

    Ok(result)
}

/// Perform a short read operation
#[tauri::command]
pub async fn read_short(window: tauri::Window) -> Result<ShortReadResult, String> {
    log::info!("Starting short read operation");

    let emit_progress = |step: u32, total: u32, message: &str| {
        let _ = window.emit("read-progress", ProgressEvent {
            step,
            total,
            message: message.to_string(),
        });
    };

    let emit_log = |log_type: &str, message: &str, data: Option<&str>| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
    };

    let total_steps = 5;

    // Check connection and get port
    emit_progress(1, total_steps, "Bağlantı kontrol ediliyor...");

    let timeout_ms = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if !manager.connected {
            return Err("Not connected to meter".to_string());
        }
        manager.params.as_ref().map(|p| p.timeout_ms).unwrap_or(2000)
    };

    emit_progress(2, total_steps, "Kısa okuma paketi alınıyor...");
    emit_log("info", "Kısa okuma paketi bekleniyor (Paket 6)...", None);

    // Read the data block from meter
    // After ACK in connect(), meter should send the short read packet
    let mut data_buf = vec![0u8; 8192]; // 8KB should be enough for short read
    let mut total_read = 0;
    let mut found_etx = false;
    let start_time = std::time::Instant::now();

    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        let port = manager.port.as_mut().ok_or("Port not available")?;

        // Wait a bit for data to arrive
        std::thread::sleep(Duration::from_millis(300));

        let mut last_read_time = std::time::Instant::now();

        loop {
            match port.read(&mut data_buf[total_read..]) {
                Ok(n) if n > 0 => {
                    total_read += n;
                    last_read_time = std::time::Instant::now(); // Reset idle timer
                    // Emit RX activity for LED indicator
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                    // Check for end of data block (ETX followed by BCC)
                    if total_read >= 2 {
                        // Look for ETX in the data
                        for i in 0..total_read-1 {
                            if data_buf[i] == control::ETX {
                                // Found ETX, check if we have BCC too (next byte)
                                if i + 1 < total_read {
                                    // We have ETX and BCC, transmission complete
                                    found_etx = true;
                                    break;
                                }
                            }
                        }
                        if found_etx {
                            emit_log("info", &format!("Veri alımı tamamlandı: {} byte, süre: {:.1}s", total_read, start_time.elapsed().as_secs_f32()), None);
                            break; // Exit the read loop
                        }
                    }
                }
                Ok(_) => {
                    // No data in this read, wait a bit
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // Timeout is expected when waiting for data, continue
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    emit_log("error", &format!("Okuma hatası: {}", e), None);
                    return Err(format!("Read error: {}", e));
                }
            }

            // Timeout only if NO data received for 3 seconds (idle timeout for short read)
            let idle_time = last_read_time.elapsed();
            if idle_time > Duration::from_millis(3000) {
                if total_read == 0 {
                    emit_log("error", "Zaman aşımı: Hiç veri alınamadı (3s boşta)", None);
                } else {
                    emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı ama ETX yok (3s boşta)", total_read), None);
                }
                break;
            }
        }
    }

    if total_read == 0 {
        emit_log("error", "Veri alınamadı", None);
        return Err("No data received from meter".to_string());
    }

    if !found_etx {
        emit_log("warn", &format!("Veri tam alınamadı: ETX bulunamadı ({} byte alındı)", total_read), None);
    }

    emit_progress(3, total_steps, "Veriler alındı, doğrulanıyor...");

    // Verify BCC if we found ETX
    if found_etx {
        // Find ETX position
        if let Some(etx_idx) = data_buf[..total_read].iter().position(|&b| b == control::ETX) {
            if etx_idx + 1 < total_read {
                let received_bcc = data_buf[etx_idx + 1];
                // Calculate expected BCC (XOR from byte after STX to ETX inclusive)
                if let Some(stx_idx) = data_buf[..total_read].iter().position(|&b| b == control::STX) {
                    let calculated_bcc = iec62056::calculate_bcc(&data_buf[stx_idx+1..=etx_idx]);
                    if calculated_bcc != received_bcc {
                        emit_log("warn", &format!("BCC uyuşmazlığı: beklenen 0x{:02X}, alınan 0x{:02X}", calculated_bcc, received_bcc), None);
                    } else {
                        emit_log("success", &format!("BCC doğrulaması başarılı ({} byte)", total_read), None);
                    }
                }
            }
        }
    }

    // Convert to string for parsing and format for display
    let raw_data = String::from_utf8_lossy(&data_buf[..total_read]).to_string();
    let data_formatted = iec62056::format_bytes_for_display(&data_buf[..total_read]);
    emit_log("rx", &data_formatted, None);

    emit_progress(4, total_steps, "OBIS kodları çözümleniyor...");

    // Parse the OBIS data
    let items = iec62056::parse_data_block(&raw_data);
    emit_log("info", &format!("{} OBIS kodu ayrıştırıldı", items.len()), None);

    // Extract values from parsed items
    let get_value = |code: &str| -> String {
        items.iter()
            .find(|item| item.code == code)
            .map(|item| item.value.clone())
            .unwrap_or_default()
    };

    let get_float = |code: &str| -> f64 {
        get_value(code).parse().unwrap_or(0.0)
    };

    emit_progress(5, total_steps, "Tamamlandı!");
    emit_log("success", "Kısa okuma başarıyla tamamlandı", None);

    // Build result
    let result = ShortReadResult {
        serial_number: get_value("0.0.0"),
        program_version: get_value("0.2.0"),
        production_date: get_value("96.1.3"),
        calibration_date: get_value("96.2.5"),
        meter_date: get_value("0.9.2"),
        meter_time: get_value("0.9.1"),
        day_of_week: get_value("0.9.5").parse().unwrap_or(0),
        active_energy_import_total: get_float("1.8.0"),
        active_energy_import_t1: get_float("1.8.1"),
        active_energy_import_t2: get_float("1.8.2"),
        active_energy_import_t3: get_float("1.8.3"),
        active_energy_import_t4: get_float("1.8.4"),
        max_demand_import: get_float("1.6.0"),
        max_demand_import_timestamp: get_value("1.6.0"), // Usually includes timestamp
        voltage_l1: get_float("32.7.0"),
        voltage_l2: get_float("52.7.0"),
        voltage_l3: get_float("72.7.0"),
        current_l1: get_float("31.7.0"),
        current_l2: get_float("51.7.0"),
        current_l3: get_float("71.7.0"),
        frequency: get_float("14.7.0"),
        power_factor_l1: get_float("33.7.0"),
        power_factor_l2: get_float("53.7.0"),
        power_factor_l3: get_float("73.7.0"),
        ff_code: get_value("F.F.0"),
        gf_code: get_value("F.F.1"),
        battery_status: if get_value("96.6.1").contains("0") { "low".to_string() } else { "full".to_string() },
        relay_status: if get_value("96.3.10").contains("1") { "active".to_string() } else { "passive".to_string() },
        raw_data: Some(raw_data),
    };

    // Update identity with serial number
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut identity) = manager.identity {
            identity.serial_number = Some(result.serial_number.clone());
        }
    }

    Ok(result)
}

/// Read a specific OBIS code
#[tauri::command]
pub async fn read_obis(obis_code: String, window: tauri::Window) -> Result<String, String> {
    log::info!("Reading OBIS code: {}", obis_code);

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }

    let port = manager.port.as_mut().ok_or("Port not available")?;

    // Build and send read command
    let cmd = iec62056::build_read_command(&obis_code);
    emit_log("tx", &format!("R2 {}()", obis_code));

    port.write_all(&cmd).map_err(|e| format!("Write failed: {}", e))?;
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Read response
    let mut buf = vec![0u8; 256];
    let mut total = 0;

    std::thread::sleep(Duration::from_millis(200));

    match port.read(&mut buf) {
        Ok(n) => total = n,
        Err(e) => return Err(format!("Read failed: {}", e)),
    }

    if total == 0 {
        return Err("No response".to_string());
    }

    let response = String::from_utf8_lossy(&buf[..total]).to_string();
    emit_log("rx", &response);

    // Parse the OBIS response
    if let Some(item) = iec62056::parse_obis_response(&response) {
        Ok(if let Some(unit) = item.unit {
            format!("{}*{}", item.value, unit)
        } else {
            item.value
        })
    } else {
        Ok(response)
    }
}

/// Write a value to an OBIS code (requires programming mode)
#[tauri::command]
pub async fn write_obis(obis_code: String, value: String, window: tauri::Window) -> Result<(), String> {
    log::info!("Writing OBIS code: {} = {}", obis_code, value);

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }
    if !manager.in_programming_mode {
        return Err("Meter is not in programming mode".to_string());
    }

    let port = manager.port.as_mut().ok_or("Port not available")?;

    // Build and send write command
    let cmd = iec62056::build_write_command(&obis_code, &value);
    emit_log("tx", &format!("W2 {}({})", obis_code, value));

    port.write_all(&cmd).map_err(|e| format!("Write failed: {}", e))?;
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Wait for ACK
    std::thread::sleep(Duration::from_millis(200));

    let mut buf = [0u8; 1];
    match port.read(&mut buf) {
        Ok(1) if buf[0] == control::ACK => {
            emit_log("rx", "ACK");
            Ok(())
        }
        Ok(1) if buf[0] == control::NAK => {
            emit_log("error", "NAK - Yazma reddedildi");
            Err("Write rejected by meter (NAK)".to_string())
        }
        _ => {
            emit_log("error", "Geçersiz yanıt");
            Err("Invalid response from meter".to_string())
        }
    }
}

/// Authenticate with the meter (enter programming mode)
#[tauri::command]
pub async fn authenticate(password: String, window: tauri::Window) -> Result<bool, String> {
    log::info!("Authenticating with meter");

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    // Validate password format (8 digits)
    if password.len() != 8 || !password.chars().all(|c| c.is_ascii_digit()) {
        return Err("Password must be exactly 8 digits".to_string());
    }

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }

    let port = manager.port.as_mut().ok_or("Port not available")?;

    // Build and send password command
    let cmd = iec62056::build_password_command(&password);
    emit_log("tx", "P1 (********)");

    port.write_all(&cmd).map_err(|e| format!("Write failed: {}", e))?;
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Wait for response
    std::thread::sleep(Duration::from_millis(500));

    let mut buf = [0u8; 1];
    match port.read(&mut buf) {
        Ok(1) if buf[0] == control::ACK => {
            emit_log("success", "Şifre kabul edildi - Programlama modu aktif");
            manager.in_programming_mode = true;
            Ok(true)
        }
        Ok(1) if buf[0] == control::NAK => {
            emit_log("error", "Şifre reddedildi!");
            Ok(false)
        }
        _ => {
            emit_log("error", "Geçersiz yanıt");
            Err("Invalid response from meter".to_string())
        }
    }
}

/// Sync meter time to computer time
#[tauri::command]
pub async fn sync_time(window: tauri::Window) -> Result<(), String> {
    log::info!("Syncing meter time");

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if !manager.connected {
            return Err("Not connected to meter".to_string());
        }
        if !manager.in_programming_mode {
            return Err("Meter is not in programming mode".to_string());
        }
    }

    let now = chrono::Local::now();
    let time_str = now.format("%H:%M:%S").to_string();
    let date_str = now.format("%y-%m-%d").to_string();
    let dow = now.format("%u").to_string(); // 1-7, Monday = 1

    emit_log("info", &format!("Saat senkronizasyonu: {} {}", date_str, time_str));

    // Write time
    write_obis("0.9.1".to_string(), time_str, window.clone()).await?;
    // Write date
    write_obis("0.9.2".to_string(), date_str, window.clone()).await?;
    // Write day of week
    write_obis("0.9.5".to_string(), dow, window).await?;

    Ok(())
}

/// End the programming session
#[tauri::command]
pub async fn end_session(window: tauri::Window) -> Result<(), String> {
    log::info!("Ending programming session");

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }

    // Send break command
    if let Some(ref mut port) = manager.port {
        let cmd = iec62056::build_break_command();
        emit_log("tx", "B0 (Break)");

        let _ = port.write_all(&cmd);
        let _ = port.flush();
    }

    manager.in_programming_mode = false;
    emit_log("info", "Programlama oturumu sonlandırıldı");

    Ok(())
}
