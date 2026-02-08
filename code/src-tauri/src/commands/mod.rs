//! Tauri commands for meter communication

pub mod types;
pub mod state;
pub mod events;
pub mod io;
pub mod sessions;

pub use types::*;
pub use state::CONNECTION_STATE;
pub use events::EventEmitter;
pub use io::{ReadConfig, ReadResult, read_until_etx, verify_bcc, extract_data_block, send_break_command, resolve_initial_bauds, resolve_target_baud};
pub use sessions::{save_session_file, list_session_files, load_session_file, delete_session_file};

use crate::{PortInfo, MeterIdentity, ConnectionParams};
use crate::serial::iec62056::{self, ProtocolMode, control};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;
use tauri::Emitter;

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
            if let Some(ref mut port) = manager.port {
                let break_cmd = iec62056::build_break_command();
                let _ = port.write_all(&break_cmd);
                let _ = port.flush();
                std::thread::sleep(Duration::from_millis(300));
            }
            manager.disconnect();
        }
    }

    let port_name = params.port.clone();
    let timeout_ms = if params.timeout_ms == 0 { 2000 } else { params.timeout_ms };
    let meter_address = params.meter_address.clone();

    // Determine initial baud rates based on connection type (Turkish MASS standard)
    let baud_rates_to_try = io::resolve_initial_bauds(&params.connection_type, params.baud_rate);

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

    // Determine the target baud rate for data transfer
    let (target_baud, baud_char) = io::resolve_target_baud(
        &params.connection_type, params.baud_rate, ident.max_baud_rate, ident.baud_char
    );

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
/// This is an ATOMIC operation: opens port, handshakes, reads, closes port
#[tauri::command]
pub async fn read_full(window: tauri::Window) -> Result<ShortReadResult, String> {
    log::info!("Starting full read operation (atomic)");

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

    let total_steps = 6;

    // Step 1: Get connection parameters from stored state
    emit_progress(1, total_steps, "Bağlantı parametreleri alınıyor...");

    let (timeout_ms, port_name, meter_address, connection_type, configured_baud) = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.params.is_none() {
            return Err("Bağlantı parametresi yok. Önce 'Bağlan' butonuna tıklayın.".to_string());
        }
        let params = manager.params.as_ref().unwrap();
        (
            if params.timeout_ms == 0 { 2000 } else { params.timeout_ms },
            params.port.clone(),
            params.meter_address.clone(),
            params.connection_type.clone(),
            params.baud_rate,
        )
    };

    // Step 2: Check if connect() already has an open port with data flowing.
    // connect() sends ACK for Mode 0 and switches baud, so the meter is already
    // transmitting data. We can just take the port and read directly.
    let existing_port = {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.port.is_some() && manager.negotiated_baud > 0 {
            emit_log("info", "Mevcut bağlantı kullanılıyor (sayaç zaten veri gönderiyor)...", None);
            let port = manager.port.take(); // Take ownership, leave None
            manager.connected = false;
            port
        } else {
            // No open port — clean up any stale state
            if manager.port.is_some() {
                manager.disconnect();
            }
            None
        }
    };

    let (mut port, atomic_ident) = if let Some(p) = existing_port {
        // Port already open from connect() — skip handshake/ACK, go straight to reading
        emit_progress(4, total_steps, "Tam okuma paketi alınıyor...");
        emit_log("info", "Tam okuma paketi bekleniyor (Mod 0 - Tüm veriler)...", None);
        (p, None)
    } else {
        // No existing port — do full atomic sequence
        emit_progress(2, total_steps, "Seri port açılıyor...");

        let baud_rates = io::resolve_initial_bauds(&connection_type, configured_baud);
        let mut found_port: Option<Box<dyn SerialPort>> = None;
        let mut ident: Option<iec62056::MeterIdent> = None;
        let mut initial_baud: u32 = 0;

        for (attempt, &try_baud) in baud_rates.iter().enumerate() {
            emit_log("info", &format!("Port açılıyor: {} @ {} baud (7E1) [Deneme {}/{}]",
                port_name, try_baud, attempt + 1, baud_rates.len()), None);

            let mut current_port = match iec62056::open_port(&port_name, try_baud, timeout_ms as u64) {
                Ok(p) => p,
                Err(e) => {
                    emit_log("warn", &format!("Port açılamadı @ {} baud: {}", try_baud, e), None);
                    continue;
                }
            };

            emit_log("success", &format!("Port açıldı @ {} baud", try_baud), None);

            let request = iec62056::build_request_message(meter_address.as_deref());
            let request_str = iec62056::format_bytes_for_display(&request);
            emit_log("tx", &request_str, None);

            if let Err(e) = current_port.write_all(&request) {
                emit_log("warn", &format!("Handshake gönderilemedi: {}", e), None);
                continue;
            }
            let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
            let _ = current_port.flush();

            emit_log("info", "Yanıt bekleniyor...", None);
            std::thread::sleep(Duration::from_millis(500));

            let mut response_buf = vec![0u8; 256];
            let mut ident_read = 0;
            let handshake_start = std::time::Instant::now();

            loop {
                match current_port.read(&mut response_buf[ident_read..]) {
                    Ok(n) if n > 0 => {
                        ident_read += n;
                        let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                        if ident_read >= 2 &&
                           response_buf[ident_read - 2] == control::CR &&
                           response_buf[ident_read - 1] == control::LF {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        if ident_read > 0 { break; }
                    }
                    Err(_) => break,
                }
                if handshake_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                    break;
                }
            }

            if ident_read > 0 {
                let response_formatted = iec62056::format_bytes_for_display(&response_buf[..ident_read]);
                emit_log("rx", &response_formatted, None);

                let response = String::from_utf8_lossy(&response_buf[..ident_read]);
                if let Some(parsed) = iec62056::parse_identification(&response) {
                    emit_log("success", &format!("Sayaç tanımlandı: {} — {} ({})",
                        parsed.manufacturer, parsed.edas_id, parsed.model), None);
                    initial_baud = try_baud;
                    ident = Some(parsed);
                    found_port = Some(current_port);
                    break;
                } else {
                    emit_log("warn", "Sayaç tanımlama yanıtı ayrıştırılamadı", None);
                }
            } else {
                emit_log("warn", &format!("{} baud'da yanıt alınamadı", try_baud), None);
            }
        }

        let mut p = found_port.ok_or_else(|| {
            emit_log("error", "Hiçbir baud hızında yanıt alınamadı", None);
            "Hiçbir baud hızında yanıt alınamadı".to_string()
        })?;
        let ident = ident.unwrap();

        emit_progress(3, total_steps, "Tam okuma modu seçiliyor...");

        // Send ACK with Mode 0 (Readout)
        let (target_baud, baud_char) = io::resolve_target_baud(
            &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
        );

        let ack = iec62056::build_ack_message(ProtocolMode::Readout, baud_char);
        let ack_formatted = iec62056::format_bytes_for_display(&ack);
        emit_log("tx", &ack_formatted, None);

        p.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
        let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
        let _ = p.flush();

        // Wait and switch baud rate
        emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud), None);
        std::thread::sleep(Duration::from_millis(300));

        if target_baud != initial_baud {
            p.set_baud_rate(target_baud).map_err(|e| {
                emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
                format!("Baud hızı değiştirilemedi: {}", e)
            })?;
            emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud), None);
        }

        emit_progress(4, total_steps, "Tam okuma paketi alınıyor...");
        emit_log("info", "Tam okuma paketi bekleniyor (Mod 0 - Tüm veriler)...", None);
        (p, Some(ident))
    };

    // Step 6: Read the data block from meter
    let mut data_buf = vec![0u8; 131072]; // 128KB buffer for full readout
    let mut total_read = 0;
    let mut found_etx = false;
    let read_start = std::time::Instant::now();
    let mut last_read_time = std::time::Instant::now();
    let mut time_of_09x_read: Option<u64> = None;

    // Wait a bit for data to start arriving
    std::thread::sleep(Duration::from_millis(300));

    loop {
        match port.read(&mut data_buf[total_read..]) {
            Ok(n) if n > 0 => {
                total_read += n;
                last_read_time = std::time::Instant::now();
                let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                // Capture system time when both 0.9.1 and 0.9.2 have arrived in the buffer
                if time_of_09x_read.is_none() && total_read >= 6 {
                    let buf = &data_buf[..total_read];
                    let has_091 = buf.windows(6).any(|w| w == b"0.9.1(");
                    let has_092 = buf.windows(6).any(|w| w == b"0.9.2(");
                    if has_091 && has_092 {
                        time_of_09x_read = Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64
                        );
                    }
                }

                // Check for ETX
                if total_read >= 2 {
                    for i in 0..total_read-1 {
                        if data_buf[i] == control::ETX && i + 1 < total_read {
                            found_etx = true;
                            break;
                        }
                    }
                    if found_etx {
                        emit_log("info", &format!("Veri alımı tamamlandı: {} byte, süre: {:.1}s",
                            total_read, read_start.elapsed().as_secs_f32()), None);
                        break;
                    }
                }
            }
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                emit_log("error", &format!("Okuma hatası: {}", e), None);
                // Send break and close port before returning error
                let break_cmd = iec62056::build_break_command();
                let _ = port.write_all(&break_cmd);
                let _ = port.flush();
                return Err(format!("Okuma hatası: {}", e));
            }
        }

        // Idle timeout: 5 seconds
        if last_read_time.elapsed() > Duration::from_millis(5000) {
            if total_read == 0 {
                emit_log("error", "Zaman aşımı: Hiç veri alınamadı (5s boşta)", None);
            } else {
                emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı ama ETX yok", total_read), None);
            }
            break;
        }
    }

    // Step 7: Send Break command and close port
    emit_log("info", "Oturum sonlandırılıyor...", None);
    let break_cmd = iec62056::build_break_command();
    let break_formatted = iec62056::format_bytes_for_display(&break_cmd);
    emit_log("tx", &break_formatted, None);
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(500));
    drop(port); // Close the port
    emit_log("info", "Port kapatıldı", None);

    // Validate received data
    if total_read == 0 {
        emit_log("error", "Veri alınamadı", None);
        return Err("Sayaçtan veri alınamadı".to_string());
    }

    if !found_etx {
        emit_log("warn", &format!("Veri tam alınamadı: ETX bulunamadı ({} byte alındı)", total_read), None);
    }

    emit_progress(5, total_steps, "Veriler doğrulanıyor...");

    // Verify BCC if we found ETX
    if found_etx {
        if let Some(etx_idx) = data_buf[..total_read].iter().position(|&b| b == control::ETX) {
            if etx_idx + 1 < total_read {
                let received_bcc = data_buf[etx_idx + 1];
                if let Some(stx_idx) = data_buf[..total_read].iter().position(|&b| b == control::STX) {
                    let calculated_bcc = iec62056::calculate_bcc(&data_buf[stx_idx+1..=etx_idx]);
                    if calculated_bcc != received_bcc {
                        emit_log("warn", &format!("BCC uyuşmazlığı: beklenen 0x{:02X}, alınan 0x{:02X}",
                            calculated_bcc, received_bcc), None);
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

    // Parse the OBIS data
    let items = iec62056::parse_data_block(&raw_data);
    emit_log("info", &format!("{} OBIS kodu ayrıştırıldı", items.len()), None);

    // Extract values from parsed items
    let get_value = |code: &str| -> String {
        items.iter()
            .find(|item| item.code == code || item.code.starts_with(&format!("{}*", code)))
            .map(|item| item.value.clone())
            .unwrap_or_default()
    };

    let get_float = |code: &str| -> f64 {
        get_value(code).parse().unwrap_or(0.0)
    };

    emit_log("success", "Tam okuma başarıyla tamamlandı", None);
    emit_progress(6, total_steps, "Tamamlandı!");

    // Build result
    let result = ShortReadResult {
        serial_number: {
            let sn = get_value("0.0.0");
            if sn.is_empty() { get_value("96.1.0") } else { sn }
        },
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
        relay_status: {
            let relay_val = get_value("96.3.10");
            if relay_val.is_empty() { "".to_string() }
            else if relay_val.contains("0") { "active".to_string() }
            else { "passive".to_string() }
        },
        raw_data: Some(raw_data),
        time_of_09x_read,
    };

    // Update stored identity with serial number (for display purposes)
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        // If we did a fresh atomic read, store identity from handshake
        if let Some(ident) = atomic_ident {
            manager.identity = Some(MeterIdentity {
                manufacturer: ident.manufacturer.clone(),
                edas_id: ident.edas_id.clone(),
                model: ident.model.clone(),
                baud_rate_char: ident.baud_char.to_string(),
                generation: ident.generation.clone(),
                serial_number: Some(result.serial_number.clone()),
            });
        } else if let Some(ref mut identity) = manager.identity {
            // Existing identity from connect() — just update serial number
            identity.serial_number = Some(result.serial_number.clone());
        }
        // Mark as not connected since we closed the port
        manager.connected = false;
        manager.port = None;
    }

    Ok(result)
}

/// Perform a short read operation (Mode 6 - short packet)
/// This is an ATOMIC operation: opens port, handshakes, reads, closes port
#[tauri::command]
pub async fn read_short(window: tauri::Window) -> Result<ShortReadResult, String> {
    log::info!("Starting short read operation (atomic)");

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

    let total_steps = 6;

    // Step 1: Get connection parameters from stored state
    emit_progress(1, total_steps, "Bağlantı parametreleri alınıyor...");

    let (timeout_ms, port_name, meter_address, connection_type, configured_baud) = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.params.is_none() {
            return Err("Bağlantı parametresi yok. Önce 'Bağlan' butonuna tıklayın.".to_string());
        }
        let params = manager.params.as_ref().unwrap();
        (
            if params.timeout_ms == 0 { 2000 } else { params.timeout_ms },
            params.port.clone(),
            params.meter_address.clone(),
            params.connection_type.clone(),
            params.baud_rate,
        )
    };

    // Step 2: Close any existing connection - we'll do a fresh atomic read
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut port) = manager.port {
            emit_log("info", "Mevcut bağlantı kapatılıyor...", None);
            let break_cmd = iec62056::build_break_command();
            let _ = port.write_all(&break_cmd);
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(300));
        }
        manager.disconnect();
    }

    emit_progress(2, total_steps, "Seri port açılıyor...");

    // Step 3-4: Open port and handshake with baud rate retry
    let baud_rates = io::resolve_initial_bauds(&connection_type, configured_baud);
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut ident: Option<iec62056::MeterIdent> = None;
    let mut initial_baud: u32 = 0;

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        emit_log("info", &format!("Port açılıyor: {} @ {} baud (7E1) [Deneme {}/{}]",
            port_name, try_baud, attempt + 1, baud_rates.len()), None);

        let mut current_port = match iec62056::open_port(&port_name, try_baud, timeout_ms as u64) {
            Ok(p) => p,
            Err(e) => {
                emit_log("warn", &format!("Port açılamadı @ {} baud: {}", try_baud, e), None);
                continue;
            }
        };

        emit_log("success", &format!("Port açıldı @ {} baud", try_baud), None);

        let request = iec62056::build_request_message(meter_address.as_deref());
        let request_str = iec62056::format_bytes_for_display(&request);
        emit_log("tx", &request_str, None);

        if let Err(e) = current_port.write_all(&request) {
            emit_log("warn", &format!("Handshake gönderilemedi: {}", e), None);
            continue;
        }
        let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
        let _ = current_port.flush();

        emit_log("info", "Yanıt bekleniyor...", None);
        std::thread::sleep(Duration::from_millis(500));

        let mut response_buf = vec![0u8; 256];
        let mut ident_read = 0;
        let handshake_start = std::time::Instant::now();

        loop {
            match current_port.read(&mut response_buf[ident_read..]) {
                Ok(n) if n > 0 => {
                    ident_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    if ident_read >= 2 &&
                       response_buf[ident_read - 2] == control::CR &&
                       response_buf[ident_read - 1] == control::LF {
                        break;
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if ident_read > 0 { break; }
                }
                Err(_) => break,
            }
            if handshake_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                break;
            }
        }

        if ident_read > 0 {
            let response_formatted = iec62056::format_bytes_for_display(&response_buf[..ident_read]);
            emit_log("rx", &response_formatted, None);

            let response = String::from_utf8_lossy(&response_buf[..ident_read]);
            if let Some(parsed) = iec62056::parse_identification(&response) {
                emit_log("success", &format!("Sayaç tanımlandı: {} — {} ({})",
                    parsed.manufacturer, parsed.edas_id, parsed.model), None);
                initial_baud = try_baud;
                ident = Some(parsed);
                port = Some(current_port);
                break;
            } else {
                emit_log("warn", "Sayaç tanımlama yanıtı ayrıştırılamadı", None);
            }
        } else {
            emit_log("warn", &format!("{} baud'da yanıt alınamadı", try_baud), None);
        }
    }

    let mut port = port.ok_or_else(|| {
        emit_log("error", "Hiçbir baud hızında yanıt alınamadı", None);
        "Hiçbir baud hızında yanıt alınamadı".to_string()
    })?;
    let ident = ident.unwrap();

    emit_progress(3, total_steps, "Kısa okuma modu seçiliyor...");

    // Step 5: Send ACK with Mode 6 (ShortRead)
    let (target_baud, baud_char) = io::resolve_target_baud(
        &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
    );

    let ack = iec62056::build_ack_message(ProtocolMode::ShortRead, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    emit_log("tx", &ack_formatted, None);

    port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    // Wait and switch baud rate
    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud), None);
    std::thread::sleep(Duration::from_millis(300));

    if target_baud != initial_baud {
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
            format!("Baud hızı değiştirilemedi: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud), None);
    }

    emit_progress(4, total_steps, "Kısa okuma paketi alınıyor...");
    emit_log("info", "Kısa okuma paketi bekleniyor (Mod 6)...", None);

    // Step 6: Read the data block from meter
    let mut data_buf = vec![0u8; 8192]; // 8KB buffer for short read
    let mut total_read = 0;
    let mut found_etx = false;
    let read_start = std::time::Instant::now();
    let mut last_read_time = std::time::Instant::now();
    let mut time_of_09x_read: Option<u64> = None;

    // Wait a bit for data to start arriving
    std::thread::sleep(Duration::from_millis(300));

    loop {
        match port.read(&mut data_buf[total_read..]) {
            Ok(n) if n > 0 => {
                total_read += n;
                last_read_time = std::time::Instant::now();
                let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                // Capture system time when both 0.9.1 and 0.9.2 have arrived in the buffer
                if time_of_09x_read.is_none() && total_read >= 6 {
                    let buf = &data_buf[..total_read];
                    let has_091 = buf.windows(6).any(|w| w == b"0.9.1(");
                    let has_092 = buf.windows(6).any(|w| w == b"0.9.2(");
                    if has_091 && has_092 {
                        time_of_09x_read = Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64
                        );
                    }
                }

                // Check for ETX
                if total_read >= 2 {
                    for i in 0..total_read-1 {
                        if data_buf[i] == control::ETX && i + 1 < total_read {
                            found_etx = true;
                            break;
                        }
                    }
                    if found_etx {
                        emit_log("info", &format!("Veri alımı tamamlandı: {} byte, süre: {:.1}s",
                            total_read, read_start.elapsed().as_secs_f32()), None);
                        break;
                    }
                }
            }
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                emit_log("error", &format!("Okuma hatası: {}", e), None);
                // Send break and close port before returning error
                let break_cmd = iec62056::build_break_command();
                let _ = port.write_all(&break_cmd);
                let _ = port.flush();
                return Err(format!("Okuma hatası: {}", e));
            }
        }

        // Idle timeout: 3 seconds for short read
        if last_read_time.elapsed() > Duration::from_millis(3000) {
            if total_read == 0 {
                emit_log("error", "Zaman aşımı: Hiç veri alınamadı (3s boşta)", None);
            } else {
                emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı ama ETX yok", total_read), None);
            }
            break;
        }
    }

    // Step 7: Send Break command and close port
    emit_log("info", "Oturum sonlandırılıyor...", None);
    let break_cmd = iec62056::build_break_command();
    let break_formatted = iec62056::format_bytes_for_display(&break_cmd);
    emit_log("tx", &break_formatted, None);
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(500));
    drop(port); // Close the port
    emit_log("info", "Port kapatıldı", None);

    // Validate received data
    if total_read == 0 {
        emit_log("error", "Veri alınamadı", None);
        return Err("Sayaçtan veri alınamadı".to_string());
    }

    if !found_etx {
        emit_log("warn", &format!("Veri tam alınamadı: ETX bulunamadı ({} byte alındı)", total_read), None);
    }

    emit_progress(5, total_steps, "Veriler doğrulanıyor...");

    // Verify BCC if we found ETX
    if found_etx {
        if let Some(etx_idx) = data_buf[..total_read].iter().position(|&b| b == control::ETX) {
            if etx_idx + 1 < total_read {
                let received_bcc = data_buf[etx_idx + 1];
                if let Some(stx_idx) = data_buf[..total_read].iter().position(|&b| b == control::STX) {
                    let calculated_bcc = iec62056::calculate_bcc(&data_buf[stx_idx+1..=etx_idx]);
                    if calculated_bcc != received_bcc {
                        emit_log("warn", &format!("BCC uyuşmazlığı: beklenen 0x{:02X}, alınan 0x{:02X}",
                            calculated_bcc, received_bcc), None);
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

    // Parse the OBIS data
    let items = iec62056::parse_data_block(&raw_data);
    emit_log("info", &format!("{} OBIS kodu ayrıştırıldı", items.len()), None);

    // Extract values from parsed items
    let get_value = |code: &str| -> String {
        items.iter()
            .find(|item| item.code == code || item.code.starts_with(&format!("{}*", code)))
            .map(|item| item.value.clone())
            .unwrap_or_default()
    };

    let get_float = |code: &str| -> f64 {
        get_value(code).parse().unwrap_or(0.0)
    };

    emit_log("success", "Kısa okuma başarıyla tamamlandı", None);
    emit_progress(6, total_steps, "Tamamlandı!");

    // Build result
    let result = ShortReadResult {
        serial_number: {
            let sn = get_value("0.0.0");
            if sn.is_empty() { get_value("96.1.0") } else { sn }
        },
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
        relay_status: {
            let relay_val = get_value("96.3.10");
            if relay_val.is_empty() { "".to_string() }
            else if relay_val.contains("0") { "active".to_string() }
            else { "passive".to_string() }
        },
        raw_data: Some(raw_data),
        time_of_09x_read,
    };

    // Update stored identity with serial number (for display purposes)
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        // Update or create identity
        if manager.identity.is_none() {
            manager.identity = Some(MeterIdentity {
                manufacturer: ident.manufacturer.clone(),
                edas_id: ident.edas_id.clone(),
                model: ident.model.clone(),
                baud_rate_char: ident.baud_char.to_string(),
                generation: ident.generation.clone(),
                serial_number: Some(result.serial_number.clone()),
            });
        } else if let Some(ref mut identity) = manager.identity {
            identity.serial_number = Some(result.serial_number.clone());
        }
        // Mark as not connected since we closed the port
        manager.connected = false;
        manager.port = None;
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

    let raw = &buf[..total];
    let response = String::from_utf8_lossy(raw).to_string();
    emit_log("rx", &response);

    // Extract data between STX and ETX to exclude protocol framing and BCC byte
    // BCC can be any byte value including ')' (0x29) which corrupts parsing
    let data_slice = match (raw.iter().position(|&b| b == control::STX),
                            raw.iter().position(|&b| b == control::ETX)) {
        (Some(s), Some(e)) if s < e => &raw[s + 1..e],
        _ => raw,
    };
    let cleaned = String::from_utf8_lossy(data_slice)
        .chars().filter(|c| !c.is_control()).collect::<String>();

    // Parse the OBIS response
    if let Some(item) = iec62056::parse_obis_response(cleaned.trim()) {
        Ok(if let Some(unit) = item.unit {
            format!("{}*{}", item.value, unit)
        } else {
            item.value
        })
    } else {
        Ok(cleaned.trim().to_string())
    }
}

/// Batch-read multiple OBIS codes using Mode 0 (Data Readout).
/// Opens port → handshake → Mode 0 → reads full data block → parses requested OBIS codes.
/// Does NOT require prior connect() or password.
#[tauri::command]
pub async fn read_obis_batch(
    obis_codes: Vec<String>,
    password: Option<String>,
    window: tauri::Window,
) -> Result<std::collections::HashMap<String, String>, String> {
    log::info!("Batch OBIS read: {:?} (atomic, Mode 0)", obis_codes);

    let emit_log = |log_type: &str, message: &str| {
        let _ = window.emit("comm-log", LogEvent {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
    };

    if obis_codes.is_empty() {
        return Err("OBIS kodu listesi boş".to_string());
    }

    // Step 1: Get connection parameters from stored state
    let (timeout_ms, port_name, meter_address, connection_type, configured_baud) = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.params.is_none() {
            return Err("Bağlantı parametresi yok. Önce 'Bağlan' butonuna tıklayın.".to_string());
        }
        let params = manager.params.as_ref().unwrap();
        (
            if params.timeout_ms == 0 { 2000 } else { params.timeout_ms },
            params.port.clone(),
            params.meter_address.clone(),
            params.connection_type.clone(),
            params.baud_rate,
        )
    };

    // Step 2: Close any existing connection
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut port) = manager.port {
            emit_log("info", "Mevcut bağlantı kapatılıyor...");
            let break_cmd = iec62056::build_break_command();
            let _ = port.write_all(&break_cmd);
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(300));
        }
        manager.disconnect();
    }

    // Wait for meter to fully reset to idle state before reconnecting
    emit_log("info", "Sayacın sıfırlanması bekleniyor...");
    std::thread::sleep(Duration::from_millis(1500));

    // Step 3: Open port and handshake with baud rate retry
    let baud_rates = io::resolve_initial_bauds(&connection_type, configured_baud);
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut ident: Option<iec62056::MeterIdent> = None;
    let mut initial_baud: u32 = 0;

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        emit_log("info", &format!("Port açılıyor: {} @ {} baud (7E1) [Deneme {}/{}]",
            port_name, try_baud, attempt + 1, baud_rates.len()));

        let mut current_port = match iec62056::open_port(&port_name, try_baud, timeout_ms as u64) {
            Ok(p) => p,
            Err(e) => {
                emit_log("warn", &format!("Port açılamadı @ {} baud: {}", try_baud, e));
                continue;
            }
        };

        emit_log("success", &format!("Port açıldı @ {} baud", try_baud));

        let request = iec62056::build_request_message(meter_address.as_deref());
        let request_str = iec62056::format_bytes_for_display(&request);
        emit_log("tx", &request_str);

        if let Err(e) = current_port.write_all(&request) {
            emit_log("warn", &format!("Handshake gönderilemedi: {}", e));
            continue;
        }
        let _ = current_port.flush();

        emit_log("info", "Yanıt bekleniyor...");
        std::thread::sleep(Duration::from_millis(500));

        let mut response_buf = vec![0u8; 256];
        let mut ident_read = 0;
        let handshake_start = std::time::Instant::now();

        loop {
            match current_port.read(&mut response_buf[ident_read..]) {
                Ok(n) if n > 0 => {
                    ident_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    if ident_read >= 2 &&
                       response_buf[ident_read - 2] == control::CR &&
                       response_buf[ident_read - 1] == control::LF {
                        break;
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if ident_read > 0 { break; }
                }
                Err(_) => break,
            }
            if handshake_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                break;
            }
        }

        if ident_read > 0 {
            let response_formatted = iec62056::format_bytes_for_display(&response_buf[..ident_read]);
            emit_log("rx", &response_formatted);

            let response = String::from_utf8_lossy(&response_buf[..ident_read]);
            if let Some(parsed) = iec62056::parse_identification(&response) {
                emit_log("success", &format!("Sayaç tanımlandı: {} — {} ({})",
                    parsed.manufacturer, parsed.edas_id, parsed.model));
                initial_baud = try_baud;
                ident = Some(parsed);
                port = Some(current_port);
                break;
            } else {
                emit_log("warn", "Sayaç tanımlama yanıtı ayrıştırılamadı");
            }
        } else {
            emit_log("warn", &format!("{} baud'da yanıt alınamadı", try_baud));
        }
    }

    let mut port = port.ok_or_else(|| {
        emit_log("error", "Hiçbir baud hızında yanıt alınamadı");
        "Hiçbir baud hızında yanıt alınamadı".to_string()
    })?;
    let ident = ident.unwrap();

    // Step 4: Send ACK with Mode 0 (Data Readout - no password needed)
    let (target_baud, baud_char) = io::resolve_target_baud(
        &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
    );

    let ack = iec62056::build_ack_message(ProtocolMode::Readout, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    emit_log("tx", &ack_formatted);

    port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    // Wait and switch baud rate
    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud));
    std::thread::sleep(Duration::from_millis(300));

    if target_baud != initial_baud {
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e));
            format!("Baud hızı değiştirilemedi: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud));
    }

    // Step 5: Read the full data block from meter (Mode 0 readout)
    emit_log("info", "Veri bloğu bekleniyor (Mod 0 - Tüm veriler)...");
    std::thread::sleep(Duration::from_millis(300));

    let mut data_buf = vec![0u8; 131072]; // 128KB buffer
    let mut total_read = 0;
    let mut found_etx = false;
    let read_start = std::time::Instant::now();
    let mut last_read_time = std::time::Instant::now();

    loop {
        match port.read(&mut data_buf[total_read..]) {
            Ok(n) if n > 0 => {
                total_read += n;
                last_read_time = std::time::Instant::now();
                let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                // Check for ETX
                if total_read >= 2 {
                    for i in 0..total_read - 1 {
                        if data_buf[i] == control::ETX && i + 1 < total_read {
                            found_etx = true;
                            break;
                        }
                    }
                    if found_etx {
                        emit_log("info", &format!("Veri alımı tamamlandı: {} byte, süre: {:.1}s",
                            total_read, read_start.elapsed().as_secs_f32()));
                        break;
                    }
                }
            }
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                emit_log("error", &format!("Okuma hatası: {}", e));
                break;
            }
        }

        // Idle timeout: 5 seconds
        if last_read_time.elapsed() > Duration::from_millis(5000) {
            if total_read == 0 {
                emit_log("error", "Zaman aşımı: Hiç veri alınamadı");
            } else {
                emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı", total_read));
            }
            break;
        }
    }

    // Step 6: Send Break command and close port
    emit_log("info", "Oturum sonlandırılıyor...");
    let break_cmd = iec62056::build_break_command();
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(100));
    drop(port);
    emit_log("info", "Port kapatıldı");

    // Mark as not connected since we closed the port
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        manager.connected = false;
        manager.port = None;
        manager.in_programming_mode = false;
    }

    if total_read == 0 {
        return Err("Sayaçtan veri alınamadı".to_string());
    }

    // Step 7: Parse the data block and extract requested OBIS codes
    let raw_data = String::from_utf8_lossy(&data_buf[..total_read]).to_string();
    let items = iec62056::parse_data_block(&raw_data);
    emit_log("info", &format!("{} OBIS kodu ayrıştırıldı", items.len()));

    let mut results = std::collections::HashMap::new();
    let total_codes = obis_codes.len();

    for code in &obis_codes {
        let trimmed = code.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        // Search for the OBIS code in parsed items
        if let Some(item) = items.iter().find(|item| item.code == trimmed || item.code.starts_with(&format!("{}*", trimmed))) {
            let value = if let Some(ref unit) = item.unit {
                format!("{}*{}", item.value, unit)
            } else {
                item.value.clone()
            };
            emit_log("success", &format!("{} = {}", trimmed, value));
            results.insert(trimmed, value);
        } else {
            emit_log("warn", &format!("{} veri bloğunda bulunamadı", trimmed));
            results.insert(trimmed, String::new());
        }
    }

    emit_log("success", &format!("OBIS toplu okuma tamamlandı: {} / {} başarılı",
        results.values().filter(|v| !v.is_empty()).count(), total_codes));

    Ok(results)
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
/// This is an ATOMIC operation: opens port, handshakes, enters Mode 1 (Programming),
/// switches baud, sends password. Does NOT require a prior active connection —
/// only needs stored params from a previous connect() call.
#[tauri::command]
pub async fn authenticate(password: String, window: tauri::Window) -> Result<bool, String> {
    log::info!("Authenticating with meter (atomic)");

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

    // Step 1: Get connection parameters from stored state
    let (timeout_ms, port_name, meter_address, connection_type, configured_baud) = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.params.is_none() {
            return Err("Bağlantı parametresi yok. Önce 'Bağlan' butonuna tıklayın.".to_string());
        }
        let params = manager.params.as_ref().unwrap();
        (
            if params.timeout_ms == 0 { 2000 } else { params.timeout_ms },
            params.port.clone(),
            params.meter_address.clone(),
            params.connection_type.clone(),
            params.baud_rate,
        )
    };

    // Step 2: Close any existing connection
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut port) = manager.port {
            emit_log("info", "Mevcut bağlantı kapatılıyor...");
            let break_cmd = iec62056::build_break_command();
            let _ = port.write_all(&break_cmd);
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(300));
        }
        manager.disconnect();
    }

    // Wait for meter to fully reset to idle state before reconnecting
    emit_log("info", "Sayacın sıfırlanması bekleniyor...");
    std::thread::sleep(Duration::from_millis(1500));

    // Step 3: Open port and handshake with baud rate retry
    let baud_rates = io::resolve_initial_bauds(&connection_type, configured_baud);
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut ident: Option<iec62056::MeterIdent> = None;
    let mut initial_baud: u32 = 0;

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        emit_log("info", &format!("Port açılıyor: {} @ {} baud (7E1) [Deneme {}/{}]",
            port_name, try_baud, attempt + 1, baud_rates.len()));

        let mut current_port = match iec62056::open_port(&port_name, try_baud, timeout_ms as u64) {
            Ok(p) => p,
            Err(e) => {
                emit_log("warn", &format!("Port açılamadı @ {} baud: {}", try_baud, e));
                continue;
            }
        };

        emit_log("success", &format!("Port açıldı @ {} baud", try_baud));

        let request = iec62056::build_request_message(meter_address.as_deref());
        let request_str = iec62056::format_bytes_for_display(&request);
        emit_log("tx", &request_str);

        if let Err(e) = current_port.write_all(&request) {
            emit_log("warn", &format!("Handshake gönderilemedi: {}", e));
            continue;
        }
        let _ = current_port.flush();

        emit_log("info", "Yanıt bekleniyor...");
        std::thread::sleep(Duration::from_millis(500));

        let mut response_buf = vec![0u8; 256];
        let mut ident_read = 0;
        let handshake_start = std::time::Instant::now();

        loop {
            match current_port.read(&mut response_buf[ident_read..]) {
                Ok(n) if n > 0 => {
                    ident_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    if ident_read >= 2 &&
                       response_buf[ident_read - 2] == control::CR &&
                       response_buf[ident_read - 1] == control::LF {
                        break;
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if ident_read > 0 { break; }
                }
                Err(_) => break,
            }
            if handshake_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                break;
            }
        }

        if ident_read > 0 {
            let response_formatted = iec62056::format_bytes_for_display(&response_buf[..ident_read]);
            emit_log("rx", &response_formatted);

            let response = String::from_utf8_lossy(&response_buf[..ident_read]);
            if let Some(parsed) = iec62056::parse_identification(&response) {
                emit_log("success", &format!("Sayaç tanımlandı: {} — {} ({})",
                    parsed.manufacturer, parsed.edas_id, parsed.model));
                initial_baud = try_baud;
                ident = Some(parsed);
                port = Some(current_port);
                break;
            } else {
                emit_log("warn", "Sayaç tanımlama yanıtı ayrıştırılamadı");
            }
        } else {
            emit_log("warn", &format!("{} baud'da yanıt alınamadı", try_baud));
        }
    }

    let mut port = port.ok_or_else(|| {
        emit_log("error", "Hiçbir baud hızında yanıt alınamadı");
        "Hiçbir baud hızında yanıt alınamadı".to_string()
    })?;
    let ident = ident.unwrap();

    // Step 4: Send ACK with Mode 1 (Programming mode)
    let (target_baud, baud_char) = io::resolve_target_baud(
        &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
    );

    let ack = iec62056::build_ack_message(ProtocolMode::Programming, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    emit_log("tx", &ack_formatted);

    port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    // Wait and switch baud rate
    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud));
    std::thread::sleep(Duration::from_millis(300));

    if target_baud != initial_baud {
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e));
            format!("Baud hızı değiştirilemedi: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud));
    }

    // Wait for meter to be ready
    std::thread::sleep(Duration::from_millis(500));

    // Read P0 response from meter (programming mode acknowledgment)
    let mut prog_buf = vec![0u8; 256];
    let mut prog_read = 0;
    let prog_start = std::time::Instant::now();

    loop {
        match port.read(&mut prog_buf[prog_read..]) {
            Ok(n) if n > 0 => {
                prog_read += n;
                let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                // Check if we received ETX (complete P0 message)
                if prog_buf[..prog_read].iter().any(|&b| b == control::ETX) {
                    // Wait a bit more for the BCC byte
                    std::thread::sleep(Duration::from_millis(50));
                    if let Ok(extra) = port.read(&mut prog_buf[prog_read..]) {
                        prog_read += extra;
                    }
                    break;
                }
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                if prog_read > 0 { break; }
            }
            Err(_) => break,
        }
        if prog_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
            break;
        }
    }

    if prog_read > 0 {
        let prog_formatted = iec62056::format_bytes_for_display(&prog_buf[..prog_read]);
        emit_log("rx", &prog_formatted);
    }

    // Step 5: Send P1 password with P0 seed encryption (IEC 62056-21 Algorithm 1)
    emit_log("info", "Programlama moduna geçildi, şifre gönderiliyor...");

    // Parse P0 seed for challenge-response authentication
    let seed_opt = if prog_read > 0 {
        if let Some(seed) = iec62056::parse_p0_seed(&prog_buf[..prog_read]) {
            let seed_display: String = seed.iter().map(|b| {
                if *b >= 0x20 && *b <= 0x7E { (*b as char).to_string() }
                else { format!("<0x{:02X}>", b) }
            }).collect();
            emit_log("info", &format!("P0 seed ({} byte): {}", seed.len(), seed_display));
            Some(seed)
        } else {
            None
        }
    } else {
        None
    };

    // Small delay before sending password to let the meter prepare
    std::thread::sleep(Duration::from_millis(200));

    // If meter sent P0 seed, Base64-decode it, XOR with password, hex-encode, send as P2
    // Per IEC 62056-21: P0=challenge, P2=encrypted result, P1=plaintext only
    let cmd = if let Some(ref seed) = seed_opt {
        // Log Base64-decoded seed for diagnostics
        if let Some(decoded) = iec62056::base64_decode(seed) {
            let dec_hex: Vec<String> = decoded.iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("info", &format!("Seed Base64 decoded ({} byte): [{}]", decoded.len(), dec_hex.join(" ")));
        }
        let encrypted = iec62056::encrypt_password_with_seed(&password, seed);
        let enc_hex: Vec<String> = encrypted.iter().map(|b| format!("{:02X}", b)).collect();
        emit_log("info", &format!("XOR sonucu (ham): [{}]", enc_hex.join(" ")));
        let built_cmd = iec62056::build_encrypted_password_command(&encrypted);
        // Show the hex-encoded ASCII that will be sent
        let hex_str: String = encrypted.iter().map(|b| format!("{:02X}", b)).collect();
        emit_log("info", &format!("P2 ile gönderilecek (hex-encoded): ({})", hex_str));
        built_cmd
    } else {
        emit_log("info", "P0 seed yok, şifre düz metin gönderiliyor (P1)");
        iec62056::build_password_command(&password)
    };

    // Log the command bytes (for debugging)
    let cmd_hex: Vec<String> = cmd.iter().map(|b| format!("{:02X}", b)).collect();
    emit_log("info", &format!("P2 komutu: {} byte [{}]", cmd.len(), cmd_hex.join(" ")));
    emit_log("tx", "P2 (********)");

    port.write_all(&cmd).map_err(|e| format!("Write failed: {}", e))?;
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Wait for response
    std::thread::sleep(Duration::from_millis(500));

    // Read response — some meters send multi-byte responses
    let mut buf = [0u8; 64];
    let read_result = port.read(&mut buf);
    match read_result {
        Ok(n) if n > 0 => {
            let response_formatted = iec62056::format_bytes_for_display(&buf[..n]);
            emit_log("rx", &format!("Şifre yanıtı: {}", response_formatted));

            // Check first byte for ACK/NAK
            if buf[0] == control::ACK {
                emit_log("success", "Şifre kabul edildi - Programlama modu aktif");
                // Store port in CONNECTION_STATE for subsequent write_obis/sync_time calls
                let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
                manager.port = Some(port);
                manager.connected = true;
                manager.in_programming_mode = true;
                manager.negotiated_baud = target_baud;
                Ok(true)
            } else if buf[0] == control::NAK {
                emit_log("error", "Şifre reddedildi (NAK)!");
                drop(port);
                Ok(false)
            } else if buf[0] == control::SOH {
                // Meter sent SOH — check if it's a B0 (Break) command meaning password rejected
                let response_str = String::from_utf8_lossy(&buf[1..n]);
                if response_str.contains("B0") {
                    emit_log("error", "Şifre reddedildi - Sayaç oturumu sonlandırdı (B0 Break)");
                    emit_log("warn", "DİKKAT: 3 hatalı şifre girişinde sayaç 6 saat kilitlenir! Sayaç kilitliyse lütfen bekleyin.");
                } else {
                    emit_log("error", &format!("Şifre reddedildi - Sayaç yanıtı: {}", response_str.trim()));
                }
                drop(port);
                Ok(false)
            } else {
                emit_log("error", &format!("Beklenmeyen yanıt: 0x{:02X} ({} byte)", buf[0], n));
                drop(port);
                Err(format!("Unexpected response from meter: 0x{:02X}", buf[0]))
            }
        }
        Ok(_) => {
            emit_log("error", "Şifre yanıtı boş");
            drop(port);
            Err("Empty response from meter".to_string())
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
            emit_log("error", "Şifre yanıtı zaman aşımı");
            drop(port);
            Err("Timeout waiting for password response".to_string())
        }
        Err(e) => {
            emit_log("error", &format!("Şifre yanıtı okuma hatası: {}", e));
            drop(port);
            Err(format!("Read error: {}", e))
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

    // Close port and mark disconnected — meter returns to idle after break
    manager.port = None;
    manager.connected = false;
    manager.in_programming_mode = false;
    emit_log("info", "Programlama oturumu sonlandırıldı, port kapatıldı");

    Ok(())
}

/// Read load profile data from meter (Mode 1 - Programming mode)
/// This is an ATOMIC operation: opens port, handshakes, enters programming mode, reads, closes port
/// Uses R2 command with P.01/P.02/P.03
#[tauri::command]
pub async fn read_load_profile(
    profile_number: u8,
    start_time: Option<String>,
    end_time: Option<String>,
    window: tauri::Window,
) -> Result<LoadProfileResult, String> {
    log::info!("Reading load profile {} with range: {:?} - {:?} (atomic)", profile_number, start_time, end_time);

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

    let total_steps = 7;

    // Step 1: Get connection parameters from stored state
    emit_progress(1, total_steps, "Bağlantı parametreleri alınıyor...");

    let (timeout_ms, port_name, meter_address, connection_type, configured_baud) = {
        let manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if manager.params.is_none() {
            return Err("Bağlantı parametresi yok. Önce 'Bağlan' butonuna tıklayın.".to_string());
        }
        let params = manager.params.as_ref().unwrap();
        (
            if params.timeout_ms == 0 { 2000 } else { params.timeout_ms },
            params.port.clone(),
            params.meter_address.clone(),
            params.connection_type.clone(),
            params.baud_rate,
        )
    };

    // Step 2: Close any existing connection - we'll do a fresh atomic read
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut port) = manager.port {
            emit_log("info", "Mevcut bağlantı kapatılıyor...", None);
            let break_cmd = iec62056::build_break_command();
            let _ = port.write_all(&break_cmd);
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(300));
        }
        manager.disconnect();
    }

    emit_progress(2, total_steps, "Seri port açılıyor...");

    // Step 3-4: Open port and handshake with baud rate retry
    let baud_rates = io::resolve_initial_bauds(&connection_type, configured_baud);
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut ident: Option<iec62056::MeterIdent> = None;
    let mut initial_baud: u32 = 0;

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        emit_log("info", &format!("Port açılıyor: {} @ {} baud (7E1) [Deneme {}/{}]",
            port_name, try_baud, attempt + 1, baud_rates.len()), None);

        let mut current_port = match iec62056::open_port(&port_name, try_baud, timeout_ms as u64) {
            Ok(p) => p,
            Err(e) => {
                emit_log("warn", &format!("Port açılamadı @ {} baud: {}", try_baud, e), None);
                continue;
            }
        };

        emit_log("success", &format!("Port açıldı @ {} baud", try_baud), None);

        let request = iec62056::build_request_message(meter_address.as_deref());
        let request_str = iec62056::format_bytes_for_display(&request);
        emit_log("tx", &request_str, None);

        if let Err(e) = current_port.write_all(&request) {
            emit_log("warn", &format!("Handshake gönderilemedi: {}", e), None);
            continue;
        }
        let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
        let _ = current_port.flush();

        emit_log("info", "Yanıt bekleniyor...", None);
        std::thread::sleep(Duration::from_millis(500));

        let mut response_buf = vec![0u8; 256];
        let mut ident_read = 0;
        let handshake_start = std::time::Instant::now();

        loop {
            match current_port.read(&mut response_buf[ident_read..]) {
                Ok(n) if n > 0 => {
                    ident_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    if ident_read >= 2 &&
                       response_buf[ident_read - 2] == control::CR &&
                       response_buf[ident_read - 1] == control::LF {
                        break;
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if ident_read > 0 { break; }
                }
                Err(_) => break,
            }
            if handshake_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                break;
            }
        }

        if ident_read > 0 {
            let response_formatted = iec62056::format_bytes_for_display(&response_buf[..ident_read]);
            emit_log("rx", &response_formatted, None);

            let response = String::from_utf8_lossy(&response_buf[..ident_read]);
            if let Some(parsed) = iec62056::parse_identification(&response) {
                emit_log("success", &format!("Sayaç tanımlandı: {} — {} ({})",
                    parsed.manufacturer, parsed.edas_id, parsed.model), None);
                initial_baud = try_baud;
                ident = Some(parsed);
                port = Some(current_port);
                break;
            } else {
                emit_log("warn", "Sayaç tanımlama yanıtı ayrıştırılamadı", None);
            }
        } else {
            emit_log("warn", &format!("{} baud'da yanıt alınamadı", try_baud), None);
        }
    }

    let mut port = port.ok_or_else(|| {
        emit_log("error", "Hiçbir baud hızında yanıt alınamadı", None);
        "Hiçbir baud hızında yanıt alınamadı".to_string()
    })?;
    let ident = ident.unwrap();

    emit_progress(3, total_steps, "Programlama moduna geçiliyor...");

    // Step 5: Send ACK with Mode 1 (Programming mode)
    let (target_baud, baud_char) = io::resolve_target_baud(
        &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
    );

    let ack = iec62056::build_ack_message(ProtocolMode::Programming, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    emit_log("tx", &ack_formatted, None);

    port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    // Wait and switch baud rate
    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud), None);
    std::thread::sleep(Duration::from_millis(300));

    if target_baud != initial_baud {
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
            format!("Baud hızı değiştirilemedi: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud), None);
    }

    // Wait for meter to be ready (it may send password request)
    std::thread::sleep(Duration::from_millis(500));

    // Read any response from meter (password request or acknowledgment)
    let mut prog_buf = vec![0u8; 256];
    let prog_read = port.read(&mut prog_buf).unwrap_or(0);
    if prog_read > 0 {
        let prog_formatted = iec62056::format_bytes_for_display(&prog_buf[..prog_read]);
        emit_log("rx", &prog_formatted, None);

        // Check if meter is requesting password
        if prog_read >= 7 && prog_buf[0] == control::SOH && prog_buf[1] == b'P' {
            emit_log("warn", "Sayaç şifre gerektiriyor - yük profili okumak için önce giriş yapın", None);
        }
    }

    emit_log("success", "Programlama moduna geçildi", None);

    emit_progress(4, total_steps, &format!("P.{:02} yük profili sorgulanıyor...", profile_number));

    // Step 6: Build and send load profile command
    let cmd = iec62056::build_load_profile_command(
        profile_number,
        start_time.as_deref(),
        end_time.as_deref(),
    );
    let cmd_formatted = iec62056::format_bytes_for_display(&cmd);
    emit_log("tx", &cmd_formatted, None);

    port.write_all(&cmd).map_err(|e| format!("Komut gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    emit_progress(5, total_steps, "Veri blokları alınıyor...");
    emit_log("info", "Yük profili verisi bekleniyor (bu işlem uzun sürebilir)...", None);

    // Step 7: Read response - load profile can be very large
    // Use growable buffer — profile 2 (10 columns) can exceed 1MB
    let mut data_buf: Vec<u8> = Vec::with_capacity(1048576); // Start with 1MB capacity
    let mut chunk_buf = [0u8; 8192]; // Read in 8KB chunks
    let mut found_etx = false;
    let read_start = std::time::Instant::now();
    let mut last_read_time = std::time::Instant::now();
    let mut block_count = 0;

    std::thread::sleep(Duration::from_millis(500));

    loop {
        match port.read(&mut chunk_buf) {
            Ok(n) if n > 0 => {
                let old_len = data_buf.len();
                data_buf.extend_from_slice(&chunk_buf[..n]);
                last_read_time = std::time::Instant::now();
                let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                // Count data blocks for progress indication
                let new_blocks = data_buf[old_len..]
                    .iter()
                    .filter(|&&b| b == control::CR)
                    .count();
                if new_blocks > 0 {
                    block_count += new_blocks;
                    if block_count % 50 == 0 {
                        emit_log("info", &format!("{} satır alındı ({} byte, {:.1}s)...",
                            block_count, data_buf.len(), read_start.elapsed().as_secs_f32()), None);
                    }
                }

                // Check for ETX in newly received bytes
                for i in (old_len..data_buf.len()).rev() {
                    if data_buf[i] == control::ETX {
                        found_etx = true;
                        break;
                    }
                }
                if found_etx {
                    emit_log("info", &format!("Veri alımı tamamlandı: {} byte, {} satır, süre: {:.1}s",
                        data_buf.len(), block_count, read_start.elapsed().as_secs_f32()), None);
                    break;
                }
            }
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                emit_log("error", &format!("Okuma hatası: {}", e), None);
                let break_cmd = iec62056::build_break_command();
                let _ = port.write_all(&break_cmd);
                let _ = port.flush();
                return Err(format!("Okuma hatası: {}", e));
            }
        }

        // Sliding idle timeout only — resets with every data arrival
        // No global timeout: we don't know the data size
        if last_read_time.elapsed() > Duration::from_millis(15000) {
            if data_buf.is_empty() {
                emit_log("error", "Zaman aşımı: Hiç veri alınamadı (15s). Sayaç bu profili desteklemiyor olabilir.", None);
                let break_cmd = iec62056::build_break_command();
                let _ = port.write_all(&break_cmd);
                let _ = port.flush();
                return Err("15 saniye zaman aşımı. Sayaç bu profili desteklemiyor olabilir.".to_string());
            } else {
                emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı, {} satır, süre: {:.1}s",
                    data_buf.len(), block_count, read_start.elapsed().as_secs_f32()), None);
            }
            break;
        }
    }

    // Step 8: Send Break command and close port
    emit_log("info", "Oturum sonlandırılıyor...", None);
    let break_cmd = iec62056::build_break_command();
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(100));
    drop(port); // Close the port
    emit_log("info", "Port kapatıldı", None);

    // Mark as not connected since we closed the port
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        manager.connected = false;
        manager.port = None;
        manager.in_programming_mode = false;
    }

    emit_progress(6, total_steps, "Yük profili verileri ayrıştırılıyor...");

    // Convert to string
    let total_read = data_buf.len();
    let raw_data = String::from_utf8_lossy(&data_buf).to_string();
    let data_formatted = iec62056::format_bytes_for_display(&data_buf[..total_read.min(2000)]);
    let truncation_note = if total_read > 2000 {
        Some(format!("... ({} byte toplam)", total_read))
    } else {
        None
    };
    emit_log("rx", &data_formatted, truncation_note.as_deref());

    // Parse load profile entries
    // Format varies by meter:
    //   Type A: P.01(yy-mm-dd,hh:mm)(value1)(value2)...(status)
    //   Type B (BYL): LPCH:1.8.0*kWh\r\n(yy-mm-dd,hh:mm)(value)\r\n...
    let mut entries: Vec<LoadProfileEntry> = Vec::new();

    for line in raw_data.lines() {
        // Strip control chars (STX=0x02, ETX=0x03, SOH=0x01, etc.)
        let clean: String = line.chars()
            .filter(|c| !c.is_ascii_control())
            .collect();
        let trimmed = clean.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Skip header/metadata lines
        if trimmed.starts_with("LPCH:") || trimmed.starts_with("LPC:") {
            continue;
        }

        // Find where parenthesized data starts
        let data_part = if trimmed.starts_with("P.") {
            // Type A: P.01(date,time)(value)...
            match trimmed.find('(') {
                Some(pos) => &trimmed[pos..],
                None => continue,
            }
        } else if trimmed.starts_with('(') {
            // Type B: (date,time)(value)...
            trimmed
        } else {
            continue;
        };

        // Find all parenthesized values
        let mut values: Vec<&str> = Vec::new();
        let mut depth = 0;
        let mut start = 0;

        for (i, c) in data_part.char_indices() {
            if c == '(' {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    values.push(&data_part[start..i]);
                }
            }
        }

        if values.len() >= 2 {
            let timestamp = values[0].to_string();
            let mut numeric_values: Vec<f64> = Vec::new();
            let mut status: Option<String> = None;

            for val in &values[1..] {
                // Values can be comma-separated inside parentheses:
                //   Single: (000000.000) or (123.45*kWh)
                //   Multi:  (220.61,000.52,003.02,000.028,0.00,50.0)
                let parts: Vec<&str> = val.split(',').collect();
                for part in &parts {
                    let part = part.trim();
                    if part.is_empty() { continue; }
                    // Strip optional unit suffix: "123.45*kWh" -> "123.45"
                    let num_str = if let Some(star_pos) = part.find('*') {
                        &part[..star_pos]
                    } else {
                        part
                    };
                    if let Ok(num) = num_str.parse::<f64>() {
                        numeric_values.push(num);
                    } else if part.len() <= 16 && !part.is_empty() && part.chars().all(|c| c.is_ascii_hexdigit()) {
                        status = Some(part.to_string());
                    }
                }
            }

            if !numeric_values.is_empty() || status.is_some() {
                entries.push(LoadProfileEntry {
                    timestamp,
                    values: numeric_values,
                    status,
                });
            }
        }
    }

    emit_progress(7, total_steps, "Tamamlandı!");

    if entries.is_empty() && total_read > 0 {
        emit_log("warn", &format!("Uyarı: {} byte veri alındı ama hiç kayıt ayrıştırılamadı. Veri formatı beklenenden farklı olabilir.", total_read), None);
        // Log first few lines for debugging
        let preview_lines: Vec<&str> = raw_data.lines().take(10).collect();
        for (i, line) in preview_lines.iter().enumerate() {
            emit_log("info", &format!("Satır {}: {}", i+1, line), None);
        }
    } else if entries.is_empty() {
        emit_log("warn", "Hiç kayıt bulunamadı. Sayaç bu profil için veri döndürmedi.", None);
    } else {
        emit_log("success", &format!("Yük profili okundu: {} kayıt", entries.len()), None);
    }

    Ok(LoadProfileResult {
        profile_number,
        entries,
        raw_data,
    })
}
