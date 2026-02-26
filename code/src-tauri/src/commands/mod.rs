//! Tauri commands for meter communication

pub mod types;
pub mod state;
pub mod events;
pub mod io;
pub mod sessions;
pub mod logger;

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

/// Authenticate at 19200 baud flag — set by change_password() before calling authenticate()
static AUTH_FORCE_19200: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
        let full_msg = match data {
            Some(d) => format!("{} | {}", message, d),
            None => message.to_string(),
        };
        logger::write_log(&timestamp, log_type, &full_msg);
        if let Some(d) = data {
            logger::write_hex_dump(d);
        }
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
    // Try 19200 first: if the meter is at MAS6 we connect immediately without sending
    // garbage bytes at wrong baud rates (which confuses RS485 meters).
    let baud_rates_to_try = io::resolve_initial_bauds(&params.connection_type, params.baud_rate);

    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut successful_baud: u32 = 0;
    let mut total_read: usize = 0;
    let mut response_buf = vec![0u8; 256];

    for (attempt, &try_baud) in baud_rates_to_try.iter().enumerate() {
        // Send B0 at this baud first to clear any stuck session at this baud rate.
        // This avoids sending garbage bytes at wrong baud rates which confuse the meter.
        if let Ok(mut bp) = iec62056::open_port(&port_name, try_baud, 200) {
            let break_cmd = iec62056::build_break_command();
            let break_str = iec62056::format_bytes_for_display(&break_cmd);
            emit_log("info", &format!("Önceki oturum temizleniyor (Break @ {} baud)...", try_baud), None);
            emit_log("tx", &format!("[{}bd] {}", try_baud, break_str), None);
            let _ = bp.write_all(&break_cmd);
            let _ = bp.flush();
            std::thread::sleep(Duration::from_millis(200));
        }

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
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
        let full_msg = match data {
            Some(d) => format!("{} | {}", message, d),
            None => message.to_string(),
        };
        logger::write_log(&timestamp, log_type, &full_msg);
        if let Some(d) = data {
            logger::write_hex_dump(d);
        }
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
        active_energy_export_total: get_float("2.8.0"),
        active_energy_export_t1: get_float("2.8.1"),
        active_energy_export_t2: get_float("2.8.2"),
        active_energy_export_t3: get_float("2.8.3"),
        active_energy_export_t4: get_float("2.8.4"),
        reactive_energy_inductive_import: get_float("5.8.0"),
        reactive_energy_capacitive_import: get_float("6.8.0"),
        reactive_energy_inductive_export: get_float("7.8.0"),
        reactive_energy_capacitive_export: get_float("8.8.0"),
        max_demand_import: get_float("1.6.0"),
        max_demand_import_timestamp: get_value("1.6.0"),
        max_demand_export: get_float("2.6.0"),
        max_demand_export_timestamp: get_value("2.6.0"),
        total_active_power: get_float("15.7.0"),
        active_power_l1: get_float("35.7.0"),
        active_power_l2: get_float("55.7.0"),
        active_power_l3: get_float("75.7.0"),
        total_reactive_power: get_float("131.7.0"),
        neutral_current: get_float("91.7.0"),
        demand_period: get_value("0.8.0"),
        lp_period: get_value("0.8.4"),
        load_limit_threshold: get_value("96.3.12"),
        load_limit_period: get_value("96.3.13"),
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
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
        let full_msg = match data {
            Some(d) => format!("{} | {}", message, d),
            None => message.to_string(),
        };
        logger::write_log(&timestamp, log_type, &full_msg);
        if let Some(d) = data {
            logger::write_hex_dump(d);
        }
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
        active_energy_export_total: get_float("2.8.0"),
        active_energy_export_t1: get_float("2.8.1"),
        active_energy_export_t2: get_float("2.8.2"),
        active_energy_export_t3: get_float("2.8.3"),
        active_energy_export_t4: get_float("2.8.4"),
        reactive_energy_inductive_import: get_float("5.8.0"),
        reactive_energy_capacitive_import: get_float("6.8.0"),
        reactive_energy_inductive_export: get_float("7.8.0"),
        reactive_energy_capacitive_export: get_float("8.8.0"),
        max_demand_import: get_float("1.6.0"),
        max_demand_import_timestamp: get_value("1.6.0"),
        max_demand_export: get_float("2.6.0"),
        max_demand_export_timestamp: get_value("2.6.0"),
        total_active_power: get_float("15.7.0"),
        active_power_l1: get_float("35.7.0"),
        active_power_l2: get_float("55.7.0"),
        active_power_l3: get_float("75.7.0"),
        total_reactive_power: get_float("131.7.0"),
        neutral_current: get_float("91.7.0"),
        demand_period: get_value("0.8.0"),
        lp_period: get_value("0.8.4"),
        load_limit_threshold: get_value("96.3.12"),
        load_limit_period: get_value("96.3.13"),
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

/// Perform an atomic mode-specific read (Modes 5, 7, 8, 9).
///
/// Opens port, handshakes, sends ACK with the requested mode, reads data, closes port.
/// Returns the raw data string and metadata.
fn atomic_mode_read(
    mode: ProtocolMode,
    read_config: &io::ReadConfig,
    window: &tauri::Window,
) -> Result<PacketReadResult, String> {
    let emit_log = |log_type: &str, message: &str, data: Option<&str>| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
        let full_msg = match data {
            Some(d) => format!("{} | {}", message, d),
            None => message.to_string(),
        };
        logger::write_log(&timestamp, log_type, &full_msg);
        if let Some(d) = data {
            logger::write_hex_dump(d);
        }
    };

    let mode_num = mode.as_char().to_digit(10).unwrap_or(0) as u8;

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
            emit_log("info", "Mevcut bağlantı kapatılıyor...", None);
            let break_cmd = iec62056::build_break_command();
            let _ = port.write_all(&break_cmd);
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(300));
        }
        manager.disconnect();
    }

    // Step 3: Open port and handshake with baud rate retry
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

    // Step 4: Send ACK with requested mode
    let (target_baud, baud_char) = io::resolve_target_baud(
        &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
    );

    let ack = iec62056::build_ack_message(mode, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    emit_log("tx", &ack_formatted, None);

    port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    // Step 5: Switch baud rate
    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud), None);
    std::thread::sleep(Duration::from_millis(300));

    if target_baud != initial_baud {
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
            format!("Baud hızı değiştirilemedi: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı", target_baud), None);
    }

    emit_log("info", &format!("Mod {} paketi bekleniyor...", mode_num), None);

    // Step 6: Read data using read_until_etx
    let read_result = io::read_until_etx(&mut port, Some(window), read_config)?;

    // Step 7: Send break and close port
    emit_log("info", "Oturum sonlandırılıyor...", None);
    let break_cmd = iec62056::build_break_command();
    let break_formatted = iec62056::format_bytes_for_display(&break_cmd);
    emit_log("tx", &break_formatted, None);
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(500));
    drop(port);
    emit_log("info", "Port kapatıldı", None);

    // Validate
    if read_result.bytes_read == 0 {
        emit_log("warn", &format!("Mod {} için veri gelmedi (sayaç bu modu desteklemiyor olabilir)", mode_num), None);
        return Ok(PacketReadResult {
            mode: mode_num,
            raw_data: String::new(),
            bytes_read: 0,
            read_duration_ms: 0,
            bcc_valid: false,
        });
    }

    if !read_result.found_etx {
        emit_log("warn", &format!("ETX bulunamadı ({} byte alındı)", read_result.bytes_read), None);
    }

    // Step 8: Verify BCC
    let bcc_valid = if read_result.found_etx {
        match io::verify_bcc(&read_result.data) {
            Ok(true) => {
                emit_log("success", &format!("BCC doğrulaması başarılı ({} byte)", read_result.bytes_read), None);
                true
            }
            Ok(false) => {
                emit_log("warn", "BCC uyuşmazlığı", None);
                false
            }
            Err(e) => {
                emit_log("warn", &format!("BCC doğrulanamadı: {}", e), None);
                false
            }
        }
    } else {
        false
    };

    // Convert to string
    let raw_data = String::from_utf8_lossy(&read_result.data).to_string();
    let data_formatted = iec62056::format_bytes_for_display(&read_result.data);
    emit_log("rx", &data_formatted, None);

    let duration_ms = read_result.duration.as_millis() as u64;
    emit_log("success", &format!("Mod {} okuma tamamlandı: {} byte, {:.1}s",
        mode_num, read_result.bytes_read, duration_ms as f64 / 1000.0), None);

    // Mark as not connected since we closed the port
    {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        manager.connected = false;
        manager.port = None;
    }

    Ok(PacketReadResult {
        mode: mode_num,
        raw_data,
        bytes_read: read_result.bytes_read,
        read_duration_ms: duration_ms,
        bcc_valid,
    })
}

/// Read a mode-specific packet from the meter (Modes 5, 7, 8, 9)
/// This is an ATOMIC operation: opens port, handshakes, reads, closes port
#[tauri::command]
pub async fn read_packet(mode: u8, window: tauri::Window) -> Result<PacketReadResult, String> {
    let protocol_mode = match mode {
        5 => ProtocolMode::TechQuality,
        7 => ProtocolMode::Historical,
        8 => ProtocolMode::Warnings,
        9 => ProtocolMode::Outages,
        _ => return Err(format!("Desteklenmeyen mod: {}. Geçerli modlar: 5, 7, 8, 9", mode)),
    };

    log::info!("Starting mode {} read operation (atomic)", mode);
    let read_config = io::ReadConfig::mode_read(mode);

    let _ = window.emit("read-progress", ProgressEvent {
        step: 1,
        total: 3,
        message: format!("Mod {} paketi okunuyor...", mode),
    });

    let result = atomic_mode_read(protocol_mode, &read_config, &window)?;

    let _ = window.emit("read-progress", ProgressEvent {
        step: 3,
        total: 3,
        message: "Tamamlandı!".to_string(),
    });

    Ok(result)
}

/// Read a specific OBIS code
#[tauri::command]
pub async fn read_obis(obis_code: String, window: tauri::Window) -> Result<String, String> {
    log::info!("Reading OBIS code: {}", obis_code);

    let emit_log = |log_type: &str, message: &str| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
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
    _password: Option<String>,
    window: tauri::Window,
) -> Result<std::collections::HashMap<String, String>, String> {
    log::info!("Batch OBIS read: {:?} (atomic, Mode 0)", obis_codes);

    let emit_log = |log_type: &str, message: &str| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
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
        let request_hex: Vec<String> = request.iter().map(|b| format!("{:02X}", b)).collect();
        emit_log("tx", &format!("{} [{}]", request_str, request_hex.join(" ")));

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
            let ident_hex: Vec<String> = response_buf[..ident_read].iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("rx", &format!("{} [{}]", response_formatted, ident_hex.join(" ")));

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
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
    };

    let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
    if !manager.connected {
        return Err("Not connected to meter".to_string());
    }
    if !manager.in_programming_mode {
        return Err("Meter is not in programming mode".to_string());
    }

    let port = manager.port.as_mut().ok_or("Port not available")?;

    // Clear any leftover data in serial buffer before writing
    let _ = port.clear(serialport::ClearBuffer::Input);

    // Build and send write command
    let cmd = iec62056::build_write_command(&obis_code, &value);
    let cmd_hex: Vec<String> = cmd.iter().map(|b| format!("{:02X}", b)).collect();
    emit_log("tx", &format!("W2 {}({}) [{}]", obis_code, value, cmd_hex.join(" ")));

    // Delay before sending write command
    std::thread::sleep(Duration::from_millis(500));

    port.write_all(&cmd).map_err(|e| format!("Write failed: {}", e))?;
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Wait for response
    std::thread::sleep(Duration::from_millis(500));

    let mut buf = [0u8; 32];
    match port.read(&mut buf) {
        Ok(n) if n > 0 => {
            let hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("rx", &format!("Yanıt ({} byte): [{}]", n, hex.join(" ")));
            if buf[0] == control::ACK {
                emit_log("rx", "ACK");
                Ok(())
            } else if buf[0] == control::NAK {
                emit_log("error", "NAK - Yazma reddedildi. Şifre yanlış olabilir veya sayaç fabrika modunda değil.");
                Err("NAK - Şifre yanlış veya sayaç bu işleme izin vermiyor".to_string())
            } else {
                let formatted = iec62056::format_bytes_for_display(&buf[..n]);
                emit_log("error", &format!("Beklenmeyen yanıt: {}", formatted));
                Err(format!("Unexpected response: 0x{:02X}", buf[0]))
            }
        }
        _ => {
            emit_log("error", "Yanıt alınamadı");
            Err("No response from meter".to_string())
        }
    }
}

/// Authenticate with the meter (enter programming mode)
/// This is an ATOMIC operation: opens port, handshakes, enters Mode 1 (Programming),
/// switches baud, sends password. Does NOT require a prior active connection —
/// only needs stored params from a previous connect() call.
/// Authentication always uses P1 command (IEC 62056-21). The meter determines
/// the access level based on which stored password matches.
#[tauri::command]
pub async fn authenticate(password: String, level: Option<u8>, window: tauri::Window) -> Result<bool, String> {
    let password_level = level.unwrap_or(1);
    log::info!("Authenticating with meter (atomic) - P{} command", password_level);

    let emit_log = |log_type: &str, message: &str| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
    };

    // Validate password format (8 digits per TEDAS/MASS spec)
    if password.len() != 8 || !password.chars().all(|c| c.is_ascii_digit()) {
        return Err("Şifre tam olarak 8 rakam olmalıdır".to_string());
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

    // Wait for meter to reset to idle state
    emit_log("info", "Sayacın sıfırlanması bekleniyor...");
    std::thread::sleep(Duration::from_millis(1500));

    // Step 3: Open port and handshake with baud rate retry
    // B0 is sent per-baud inside the loop (not upfront) to avoid sending garbage at wrong bauds.
    // If AUTH_FORCE_19200: try 19200 first. This avoids a mid-session baud switch which can
    // leave the meter's RX at the original baud. MMS connects directly at 19200 for MAS6 meters.
    let baud_rates = if AUTH_FORCE_19200.load(std::sync::atomic::Ordering::Relaxed) {
        let mut bauds = vec![19200u32];
        for b in io::resolve_initial_bauds(&connection_type, configured_baud) {
            if !bauds.contains(&b) { bauds.push(b); }
        }
        bauds
    } else {
        io::resolve_initial_bauds(&connection_type, configured_baud)
    };
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut ident: Option<iec62056::MeterIdent> = None;
    let mut initial_baud: u32 = 0;

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        // Send B0 at this baud first to clear any stuck session at this specific baud rate
        if let Ok(mut bp) = iec62056::open_port(&port_name, try_baud, 200) {
            let break_cmd = iec62056::build_break_command();
            let _ = bp.write_all(&break_cmd);
            let _ = bp.flush();
            std::thread::sleep(Duration::from_millis(200));
        }

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
        let request_hex: Vec<String> = request.iter().map(|b| format!("{:02X}", b)).collect();
        emit_log("tx", &format!("{} [{}]", request_str, request_hex.join(" ")));

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
            let ident_hex: Vec<String> = response_buf[..ident_read].iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("rx", &format!("{} [{}]", response_formatted, ident_hex.join(" ")));

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
    // If AUTH_FORCE_19200 is set: request baud '6' (19200) regardless of meter's reported max.
    // This negotiates a 19200 session even if the meter reports MAS5 (stored baud 9600).
    let (target_baud, baud_char) = if AUTH_FORCE_19200.load(std::sync::atomic::Ordering::Relaxed) {
        (19200u32, '6')
    } else {
        io::resolve_target_baud(&connection_type, configured_baud, ident.max_baud_rate, ident.baud_char)
    };

    let ack = iec62056::build_ack_message(ProtocolMode::Programming, baud_char);
    let ack_formatted = iec62056::format_bytes_for_display(&ack);
    let ack_hex: Vec<String> = ack.iter().map(|b| format!("{:02X}", b)).collect();
    emit_log("tx", &format!("{} [{}]", ack_formatted, ack_hex.join(" ")));

    port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
    let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
    let _ = port.flush();

    emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud));

    if target_baud != initial_baud {
        // Clear any remaining bytes at old baud BEFORE switching
        let _ = port.clear(serialport::ClearBuffer::Input);
        // Switch baud immediately — meter sends P0 right after ACK at the new baud
        port.set_baud_rate(target_baud).map_err(|e| {
            emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e));
            format!("Baud hızı değiştirilemedi: {}", e)
        })?;
        emit_log("success", &format!("Baud hızı {} olarak ayarlandı, P0 bekleniyor...", target_baud));
        // Do NOT sleep here — start reading P0 immediately.
        // The meter sends P0 at the new baud shortly after receiving ACK.
    } else {
        std::thread::sleep(Duration::from_millis(150));
    }

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
        let prog_hex: Vec<String> = prog_buf[..prog_read].iter().map(|b| format!("{:02X}", b)).collect();
        emit_log("rx", &format!("{} [{}]", prog_formatted, prog_hex.join(" ")));
    }

    // Step 5: Send P{level} password command
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

    // CRITICAL: Clear serial input buffer before sending password command.
    // The P0 response or baud rate change may leave stale bytes (including ACK)
    // that could be falsely interpreted as password acceptance.
    {
        // Step A: OS-level input buffer clear
        let _ = port.clear(serialport::ClearBuffer::Input);

        // Step B: Wait for any in-flight data to arrive from the meter
        std::thread::sleep(Duration::from_millis(50));

        // Step C: Manual drain to catch anything that arrived after clear
        let mut drain_buf = [0u8; 256];
        let old_timeout = port.timeout();
        let _ = port.set_timeout(Duration::from_millis(50));
        let mut drained = 0;
        loop {
            match port.read(&mut drain_buf[drained..]) {
                Ok(n) if n > 0 => {
                    drained += n;
                    if drained >= drain_buf.len() { break; }
                }
                _ => break,
            }
        }
        let _ = port.set_timeout(old_timeout);
        if drained > 0 {
            let stale_hex: Vec<String> = drain_buf[..drained].iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("warn", &format!("UYARI: Buffer'dan {} byte eski veri temizlendi: [{}]", drained, stale_hex.join(" ")));
        } else {
            emit_log("info", "Buffer temiz, şifre gönderiliyor...");
        }
    }

    // Use P{level} command for authentication (IEC 62056-21).
    // P1 = Reader (96.96.1), P2 = Operator (96.96.2), P3 = Master (96.96.3)
    // The meter checks the HMAC against the password stored at the given level.
    // Sending P1 for a P3 password causes B0 rejection — the level in the frame must match.
    let cmd = if let Some(ref seed) = seed_opt {
        let seed_str = String::from_utf8_lossy(seed).to_string();
        emit_log("info", &format!("HMAC-SHA256 hesaplanıyor (seed: {}, seviye: P{})", seed_str, password_level));
        // Seed is used as-is (raw base64 ASCII string), NOT decoded to binary.
        // MASS meters compute: HMAC-SHA256(key=password_utf8, msg=seed_string_utf8)
        let hmac_result = iec62056::encrypt_password_hmac_sha256(&password, seed);
        let hmac_hex: String = hmac_result.iter().map(|b| format!("{:02X}", b)).collect();
        emit_log("info", &format!("HMAC sonucu: {}", hmac_hex));
        iec62056::build_hmac_password_command(&hmac_result, password_level)
    } else {
        emit_log("info", &format!("P0 seed yok, şifre düz metin gönderiliyor (P{})", password_level));
        iec62056::build_password_command_with_level(&password, password_level)
    };

    // Log the command bytes (for debugging)
    let cmd_hex: Vec<String> = cmd.iter().map(|b| format!("{:02X}", b)).collect();
    emit_log("info", &format!("Şifre komutu: {} byte [{}]", cmd.len(), cmd_hex.join(" ")));
    emit_log("tx", &format!("{} (********)", if seed_opt.is_some() { format!("P{} HMAC", password_level) } else { format!("P{}", password_level) }));

    port.write_all(&cmd).map_err(|e| format!("Write failed: {}", e))?;
    port.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Wait for meter to process the password command
    std::thread::sleep(Duration::from_millis(150));

    // Read response — collect ALL available bytes to detect stale data
    let mut buf = [0u8; 128];
    let mut total_resp = 0;
    let resp_start = std::time::Instant::now();

    // Phase 1: Wait for first byte(s) to arrive
    loop {
        match port.read(&mut buf[total_resp..]) {
            Ok(n) if n > 0 => {
                total_resp += n;
                break;
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                if resp_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // Phase 2: After first byte(s), keep reading briefly to get complete response
    if total_resp > 0 {
        let old_timeout = port.timeout();
        let _ = port.set_timeout(Duration::from_millis(200));
        loop {
            match port.read(&mut buf[total_resp..]) {
                Ok(n) if n > 0 => {
                    total_resp += n;
                    if total_resp >= buf.len() { break; }
                }
                _ => break,
            }
        }
        let _ = port.set_timeout(old_timeout);
    }

    if total_resp > 0 {
        let n = total_resp;
        let response_hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
        let response_formatted = iec62056::format_bytes_for_display(&buf[..n]);
        emit_log("rx", &format!("Şifre yanıtı ({} byte): {} [{}]", n, response_formatted, response_hex.join(" ")));

        // CRITICAL: If we received more than 1 byte and the first byte is ACK,
        // the ACK might be stale data. Check for NAK or B0 in the remaining bytes.
        if n > 1 && buf[0] == control::ACK {
            emit_log("warn", &format!("UYARI: {} byte alındı, ilk byte ACK ama ek veri var - ek veriler kontrol ediliyor", n));
            // Look for NAK or SOH (B0 Break) in the remaining bytes
            let has_nak = buf[1..n].iter().any(|&b| b == control::NAK);
            let has_soh = buf[1..n].iter().any(|&b| b == control::SOH);
            let remaining_str = String::from_utf8_lossy(&buf[1..n]);
            if has_nak {
                emit_log("error", &format!("P{} şifre reddedildi - ACK'den sonra NAK bulundu (eski buffer verisi ACK olabilir)", password_level));
                drop(port);
                return Ok(false);
            }
            if has_soh && remaining_str.contains("B0") {
                emit_log("error", &format!("P{} şifre reddedildi - ACK'den sonra B0 Break bulundu", password_level));
                drop(port);
                return Ok(false);
            }
        }

        // Check first byte for ACK/NAK
        if buf[0] == control::ACK {
            // NOTE: MASS meters ACK the P command regardless of password correctness.
            // ACK = "command received". Actual password verification happens on first write/read.
            // We cannot use an R2 probe here because it also fails in normal (non-factory) mode.
            emit_log("success", &format!("P{} komutu onaylandı (ACK) - Programlama modu aktif", password_level));

            // Store port and password in CONNECTION_STATE for subsequent operations
            let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
            manager.port = Some(port);
            manager.connected = true;
            manager.in_programming_mode = true;
            manager.negotiated_baud = target_baud;
            manager.last_password = Some(password.clone());
            Ok(true)
        } else if buf[0] == control::NAK {
            emit_log("error", &format!("P{} şifre reddedildi (NAK)!", password_level));
            drop(port);
            Ok(false)
        } else if buf[0] == control::SOH {
            // Meter sent SOH — check if it's a B0 (Break) command meaning password rejected
            let response_str = String::from_utf8_lossy(&buf[1..n]);
            if response_str.contains("B0") {
                emit_log("error", &format!("P{} şifre reddedildi - Sayaç oturumu sonlandırdı (B0 Break)", password_level));
                emit_log("warn", "DİKKAT: 3 hatalı şifre girişinde sayaç 6 saat kilitlenir! Sayaç kilitliyse lütfen bekleyin.");
            } else {
                emit_log("error", &format!("P{} şifre reddedildi - Sayaç yanıtı: {}", password_level, response_str.trim()));
            }
            drop(port);
            Ok(false)
        } else {
            emit_log("error", &format!("Beklenmeyen yanıt: 0x{:02X} ({} byte)", buf[0], n));
            drop(port);
            Err(format!("Unexpected response from meter: 0x{:02X}", buf[0]))
        }
    } else if resp_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
        emit_log("error", "Şifre yanıtı zaman aşımı");
        drop(port);
        Err("Timeout waiting for password response".to_string())
    } else {
        emit_log("error", "Şifre yanıtı boş");
        drop(port);
        Err("Empty response from meter".to_string())
    }
}

/// Change meter password (atomic: authenticate + W2 write)
/// Per MASS/TEDAS spec: password changes use W2 96.96.{level}(new_password)
/// level: target password level to change (1=96.96.1, 2=96.96.2, 3=96.96.3). Default 3.
#[tauri::command]
pub async fn change_password(current_password: String, new_password: String, level: Option<u8>, window: tauri::Window) -> Result<String, String> {
    let password_level = level.unwrap_or(3);
    let auth_level: u8 = 3; // Always authenticate with P3 (highest level) for password changes
    log::info!("Changing meter password at level {} (auth: P{})", password_level, auth_level);

    let emit_log = |log_type: &str, message: &str| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
    };

    // Validate passwords (8 digits per TEDAS/MASS spec, e.g. "12345678")
    if current_password.len() != 8 || !current_password.chars().all(|c| c.is_ascii_digit()) {
        return Err("Mevcut şifre tam olarak 8 rakam olmalıdır".to_string());
    }
    if new_password.len() != 8 || !new_password.chars().all(|c| c.is_ascii_digit()) {
        return Err("Yeni şifre tam olarak 8 rakam olmalıdır".to_string());
    }

    // OBIS code for target password level: 96.96.1 (P1), 96.96.2 (P2), 96.96.3 (P3)
    let obis_code = format!("96.96.{}", password_level);

    // P1/P2 use W1, P3 uses W2 (per MASS spec and MMS reference logs)
    let w_type = if password_level == 3 { b'2' } else { b'1' };
    let w_label = if password_level == 3 { "W2" } else { "W1" };

    emit_log("info", &format!("P{} kimlik doğrulama → {} {} ile şifre değiştirme başlatılıyor", auth_level, w_label, obis_code));

    // Authenticate at normal baud (no forced 19200 for auth step).
    // AUTH_FORCE_19200 caused slow P0 response (~5s) and P3 timeout due to garbled B0 pre-break at 19200.
    // W1/W2 write is attempted at the session baud; if W2 NAKs at 9600 we may need a different approach.
    let auth_ok = authenticate(current_password.clone(), Some(auth_level), window.clone()).await;
    let auth_ok = auth_ok.map_err(|e| format!("Kimlik doğrulama hatası: {}", e))?;

    if !auth_ok {
        emit_log("error", &format!("P{} kimlik doğrulama başarısız — şifre yanlış, sayaç kilitli (3 hatalı giriş = 6 saat kilit) veya baud hızı uyumsuzluğu", auth_level));
        let _ = end_session(window.clone()).await;
        return Err("Kimlik doğrulama başarısız — şifre yanlış, sayaç kilitli veya baud hızı uyumsuzluğu".to_string());
    }

    let result: Result<String, String> = {
        let mut manager = CONNECTION_STATE.lock().map_err(|e| e.to_string())?;
        let port = manager.port.as_mut().ok_or("Port mevcut değil")?;
        let _ = port.clear(serialport::ClearBuffer::Input);

        let cmd = iec62056::build_write_command_with_type(&obis_code, &new_password, w_type);
        let cmd_hex: Vec<String> = cmd.iter().map(|b| format!("{:02X}", b)).collect();
        let cmd_display = iec62056::format_bytes_for_display(&cmd);
        emit_log("tx", &format!("{} {} [{}]", w_label, cmd_display, cmd_hex.join(" ")));
        port.write_all(&cmd).map_err(|e| format!("{} gönderme hatası: {}", w_label, e))?;
        let _ = port.flush();
        std::thread::sleep(Duration::from_millis(500));

        let mut buf = [0u8; 64];
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                let hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
                let formatted = iec62056::format_bytes_for_display(&buf[..n]);
                emit_log("rx", &format!("{} yanıt ({} byte): {} [{}]", w_label, n, formatted, hex.join(" ")));
                match buf[0] {
                    b if b == control::ACK => {
                        emit_log("success", &format!("Şifre başarıyla değiştirildi! (OBIS: {})", obis_code));
                        Ok(format!("Şifre {} ile başarıyla değiştirildi", obis_code))
                    }
                    b if b == control::NAK => {
                        emit_log("error", &format!("{} reddedildi (NAK) — TEDAŞ MLZ/2017-062.B Madde 154 gereği RS-485 portu üzerinden değiştirilebilir parametreler ayda en fazla 2 kez değiştirilebilir. Aylık limit aşılmış olabilir. Alternatif: klemens kapağı açıldıktan sonra optik port üzerinden sınırsız değiştirilebilir (Madde 153).", w_label));
                        Err("Şifre değiştirme reddedildi (NAK) — TEDAŞ şartnamesine göre RS-485 üzerinden ayda en fazla 2 kez değişiklik yapılabilir (Madde 154). Aylık limit dolmuş olabilir.".to_string())
                    }
                    b if b == control::SOH => {
                        emit_log("error", "Sayaç B0 ile oturumu kapattı");
                        manager.in_programming_mode = false;
                        manager.connected = false;
                        Err("Sayaç oturumu sonlandırdı (B0)".to_string())
                    }
                    b => Err(format!("{} beklenmeyen yanıt: 0x{:02X}", w_label, b)),
                }
            }
            Ok(_) => { emit_log("warn", &format!("{} boş yanıt", w_label)); Err("empty".to_string()) }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                emit_log("warn", &format!("{} 96.96 zaman aşımı", w_label)); Err("timeout".to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    };

    let _ = end_session(window.clone()).await;
    result
}

/// Sync meter time to computer time
#[tauri::command]
pub async fn sync_time(window: tauri::Window) -> Result<(), String> {
    log::info!("Syncing meter time");

    let emit_log = |log_type: &str, message: &str| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
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
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: None,
        });
        logger::write_log(&timestamp, log_type, message);
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

/// Read load profile data from meter (Mode 1 - Programming mode, with P3 authentication)
/// Atomic operation: opens port → handshake → Mode 1 → P3 auth → R1 P.01/P.02/P.03
#[tauri::command]
pub async fn read_load_profile(
    profile_number: u8,
    start_time: Option<String>,
    end_time: Option<String>,
    password: Option<String>,
    window: tauri::Window,
) -> Result<LoadProfileResult, String> {
    log::info!("Reading load profile {} with range: {:?} - {:?}", profile_number, start_time, end_time);

    let emit_progress = |step: u32, total: u32, message: &str| {
        let _ = window.emit("read-progress", ProgressEvent {
            step,
            total,
            message: message.to_string(),
        });
    };

    let emit_log = |log_type: &str, message: &str, data: Option<&str>| {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let _ = window.emit("comm-log", LogEvent {
            timestamp: timestamp.clone(),
            log_type: log_type.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        });
        let full_msg = match data {
            Some(d) => format!("{} | {}", message, d),
            None => message.to_string(),
        };
        logger::write_log(&timestamp, log_type, &full_msg);
        if let Some(d) = data {
            logger::write_hex_dump(d);
        }
    };

    let total_steps = 7;

    // Step 1: Get connection parameters from stored state
    emit_progress(1, total_steps, "Bağlantı hazırlanıyor...");
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
            emit_log("info", "Mevcut bağlantı kapatılıyor...", None);
            let break_cmd = iec62056::build_break_command();
            let _ = port.write_all(&break_cmd);
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(300));
        }
        manager.disconnect();
    }

    // Step 3: Open port and handshake
    // B0 is sent per-baud inside the loop to avoid sending garbage at wrong baud rates.
    emit_progress(2, total_steps, "Sayaç ile iletişim kuruluyor...");
    let baud_rates = io::resolve_initial_bauds(&connection_type, configured_baud);
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut ident: Option<iec62056::MeterIdent> = None;
    let mut initial_baud: u32 = 0;

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        // Send B0 at this baud to clear any stuck session at this specific baud rate
        if let Ok(mut bp) = iec62056::open_port(&port_name, try_baud, 200) {
            let break_cmd = iec62056::build_break_command();
            let _ = bp.write_all(&break_cmd);
            let _ = bp.flush();
            std::thread::sleep(Duration::from_millis(200));
        }

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

    // Decide mode based on password: Mode 0 (Readout, no auth) vs Mode 1 (Programming, with auth)
    let password_str = password.unwrap_or_default();
    let has_password = !password_str.is_empty() && password_str.len() == 8;

    let (target_baud, baud_char) = io::resolve_target_baud(
        &connection_type, configured_baud, ident.max_baud_rate, ident.baud_char
    );

    if has_password {
        // === MODE 1: Programming mode with P3 HMAC auth + R1 selective read ===
        emit_progress(3, total_steps, "Programlama moduna geçiliyor...");
        let ack = iec62056::build_ack_message(ProtocolMode::Programming, baud_char);
        let ack_formatted = iec62056::format_bytes_for_display(&ack);
        emit_log("tx", &ack_formatted, None);

        port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
        let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
        let _ = port.flush();

        emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud), None);
        std::thread::sleep(Duration::from_millis(300));

        if target_baud != initial_baud {
            port.set_baud_rate(target_baud).map_err(|e| {
                emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
                format!("Baud hızı değiştirilemedi: {}", e)
            })?;
        }

        std::thread::sleep(Duration::from_millis(500));

        // Read P0 seed
        let mut prog_buf = vec![0u8; 256];
        let mut prog_read = 0;
        let prog_start = std::time::Instant::now();

        loop {
            match port.read(&mut prog_buf[prog_read..]) {
                Ok(n) if n > 0 => {
                    prog_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    if prog_buf[..prog_read].iter().any(|&b| b == control::ETX) {
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
            emit_log("rx", &prog_formatted, None);
        }

        emit_log("success", "Programlama moduna geçildi", None);

        // P3 HMAC authentication
        emit_progress(4, total_steps, "Kimlik doğrulama yapılıyor...");

        let seed_opt = if prog_read > 0 {
            iec62056::parse_p0_seed(&prog_buf[..prog_read])
        } else {
            None
        };

        let auth_cmd = if let Some(ref seed) = seed_opt {
            let seed_str = String::from_utf8_lossy(seed).to_string();
            emit_log("info", &format!("P0 seed: {}", seed_str), None);
            // Seed used as raw base64 string, not decoded
            let hmac_result = iec62056::encrypt_password_hmac_sha256(&password_str, seed);
            let hmac_hex: String = hmac_result.iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("info", &format!("P3 HMAC: {}", hmac_hex), None);
            iec62056::build_hmac_password_command(&hmac_result, 3)
        } else {
            emit_log("warn", "P0 seed alınamadı, düz metin şifre gönderiliyor", None);
            iec62056::build_password_command_with_level(&password_str, 3)
        };

        emit_log("tx", "P3 HMAC (********)", None);
        port.write_all(&auth_cmd).map_err(|e| format!("Şifre gönderilemedi: {}", e))?;
        let _ = port.flush();

        std::thread::sleep(Duration::from_millis(500));

        let mut auth_buf = [0u8; 64];
        let mut auth_read = 0;
        let auth_start = std::time::Instant::now();

        loop {
            match port.read(&mut auth_buf[auth_read..]) {
                Ok(n) if n > 0 => {
                    auth_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    break;
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if auth_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if auth_read > 0 {
            let auth_formatted = iec62056::format_bytes_for_display(&auth_buf[..auth_read]);
            emit_log("rx", &auth_formatted, None);
            if auth_buf[0] == control::ACK {
                emit_log("success", "P3 komutu kabul edildi", None);
            }
        }

        let _ = port.clear(serialport::ClearBuffer::Input);
        std::thread::sleep(Duration::from_millis(200));

        // Send R1 command for selective load profile read
        emit_progress(5, total_steps, &format!("P.{:02} yük profili sorgulanıyor...", profile_number));

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
    } else {
        // === MODE 1: Programming mode with P1 HMAC auto-auth + R1 selective read ===
        // LP data (P.01/P.02/P.03) is only accessible via R1/R2 in programming mode.
        // P1 (Reader) level is sufficient for reading load profiles.
        let auto_password = "12345678";

        emit_progress(3, total_steps, "Programlama moduna geçiliyor...");
        let ack = iec62056::build_ack_message(ProtocolMode::Programming, baud_char);
        let ack_formatted = iec62056::format_bytes_for_display(&ack);
        emit_log("tx", &ack_formatted, None);

        port.write_all(&ack).map_err(|e| format!("ACK gönderilemedi: {}", e))?;
        let _ = window.emit("comm-activity", serde_json::json!({"type": "tx"}));
        let _ = port.flush();

        emit_log("info", &format!("Baud hızı değiştiriliyor: {} -> {}", initial_baud, target_baud), None);
        std::thread::sleep(Duration::from_millis(300));

        if target_baud != initial_baud {
            port.set_baud_rate(target_baud).map_err(|e| {
                emit_log("error", &format!("Baud hızı değiştirilemedi: {}", e), None);
                format!("Baud hızı değiştirilemedi: {}", e)
            })?;
        }

        std::thread::sleep(Duration::from_millis(500));

        // Read P0 seed
        let mut prog_buf = vec![0u8; 256];
        let mut prog_read = 0;
        let prog_start = std::time::Instant::now();

        loop {
            match port.read(&mut prog_buf[prog_read..]) {
                Ok(n) if n > 0 => {
                    prog_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    if prog_buf[..prog_read].iter().any(|&b| b == control::ETX) {
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
            emit_log("rx", &prog_formatted, None);
        }

        emit_log("success", "Programlama moduna geçildi", None);

        // P1 HMAC authentication (automatic)
        emit_progress(4, total_steps, "P1 kimlik doğrulama yapılıyor...");

        let seed_opt = if prog_read > 0 {
            iec62056::parse_p0_seed(&prog_buf[..prog_read])
        } else {
            None
        };

        let auth_cmd = if let Some(ref seed) = seed_opt {
            let seed_str = String::from_utf8_lossy(seed).to_string();
            emit_log("info", &format!("P0 seed: {}", seed_str), None);
            // Seed used as raw base64 string, not decoded
            let hmac_result = iec62056::encrypt_password_hmac_sha256(auto_password, seed);
            let hmac_hex: String = hmac_result.iter().map(|b| format!("{:02X}", b)).collect();
            emit_log("info", &format!("P1 HMAC: {}", hmac_hex), None);
            iec62056::build_hmac_password_command(&hmac_result, 1)
        } else {
            emit_log("warn", "P0 seed alınamadı, düz metin şifre gönderiliyor", None);
            iec62056::build_password_command_with_level(auto_password, 1)
        };

        emit_log("tx", "P1 HMAC (otomatik)", None);
        port.write_all(&auth_cmd).map_err(|e| format!("Şifre gönderilemedi: {}", e))?;
        let _ = port.flush();

        std::thread::sleep(Duration::from_millis(500));

        let mut auth_buf = [0u8; 64];
        let mut auth_read = 0;
        let auth_start = std::time::Instant::now();

        loop {
            match port.read(&mut auth_buf[auth_read..]) {
                Ok(n) if n > 0 => {
                    auth_read += n;
                    let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));
                    break;
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if auth_start.elapsed() > Duration::from_millis(timeout_ms as u64) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if auth_read > 0 {
            let auth_formatted = iec62056::format_bytes_for_display(&auth_buf[..auth_read]);
            emit_log("rx", &auth_formatted, None);
            if auth_buf[0] == control::ACK {
                emit_log("success", "P1 kimlik doğrulama kabul edildi", None);
            } else {
                emit_log("error", "P1 kimlik doğrulama reddedildi", None);
                return Err("P1 kimlik doğrulama başarısız — yük profili okunamıyor".to_string());
            }
        }

        let _ = port.clear(serialport::ClearBuffer::Input);
        std::thread::sleep(Duration::from_millis(200));

        // Send R1 command for selective load profile read
        emit_progress(5, total_steps, &format!("P.{:02} yük profili sorgulanıyor...", profile_number));

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
    }

    // Read data from meter (Mode 1 R1 response)
    emit_progress(5, total_steps, &format!("P.{:02} yük profili verisi bekleniyor...", profile_number));
    emit_log("info", "Yük profili verisi bekleniyor (bu işlem uzun sürebilir)...", None);
    std::thread::sleep(Duration::from_millis(500));

    let mut data_buf: Vec<u8> = Vec::with_capacity(1048576);
    let mut total_read = 0;
    let read_start = std::time::Instant::now();
    let mut last_read_time = std::time::Instant::now();
    let mut block_count: usize = 0;

    loop {
        let mut chunk = [0u8; 8192];
        match port.read(&mut chunk) {
            Ok(n) if n > 0 => {
                data_buf.extend_from_slice(&chunk[..n]);
                total_read += n;
                last_read_time = std::time::Instant::now();
                let _ = window.emit("comm-activity", serde_json::json!({"type": "rx"}));

                // Check for ETX (last block)
                if chunk[..n].contains(&control::ETX) {
                    block_count += 1;
                    emit_log("info", &format!("Veri alımı tamamlandı: {} byte, süre: {:.1}s",
                        total_read, read_start.elapsed().as_secs_f32()), None);
                    break;
                }

                // Check for EOT (more blocks) - send ACK to continue
                if chunk[..n].contains(&control::EOT) {
                    block_count += 1;
                    let _ = port.write_all(&[control::ACK]);
                    let _ = port.flush();
                    if block_count % 50 == 0 {
                        emit_log("info", &format!("{} blok alındı ({} byte, {:.1}s)...",
                            block_count, total_read, read_start.elapsed().as_secs_f32()), None);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            Ok(_) => { std::thread::sleep(Duration::from_millis(100)); }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                emit_log("error", &format!("Okuma hatası: {}", e), None);
                break;
            }
        }

        // Idle timeout: 15 seconds
        if last_read_time.elapsed() > Duration::from_millis(15000) {
            if total_read == 0 {
                emit_log("error", "Zaman aşımı: Hiç veri alınamadı (15s)", None);
            } else {
                emit_log("warn", &format!("Boşta kalma zaman aşımı: {} byte alındı", total_read), None);
            }
            break;
        }
    }

    emit_log("success", &format!("Veri alımı tamamlandı: {} blok, {} byte, süre: {:.1}s",
        block_count, total_read, read_start.elapsed().as_secs_f32()), None);

    // Cleanup: close port
    emit_log("info", "Oturum sonlandırılıyor...", None);
    let break_cmd = iec62056::build_break_command();
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(100));
    drop(port);

    if total_read == 0 {
        return Err("Sayaçtan yük profili verisi alınamadı".to_string());
    }

    emit_progress(6, total_steps, "Yük profili verileri ayrıştırılıyor...");

    // Extract data content: strip STX/ETX/EOT/BCC framing bytes
    let raw_data = {
        let text = String::from_utf8_lossy(&data_buf).to_string();
        text.chars().filter(|c| {
            *c != '\x01' && *c != '\x02' && *c != '\x03' && *c != '\x04'
        }).collect::<String>()
    };
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

    emit_progress(4, total_steps, "Tamamlandı!");

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
