//! I/O utilities for serial communication
//!
//! Provides helper functions for reading data from the meter,
//! verifying BCC, and sending commands.

use crate::serial::iec62056::{self, control};
use serialport::SerialPort;
use std::io::Read;
use std::time::{Duration, Instant};
use tauri::{Emitter, Window};

/// Determine which initial baud rates to try based on connection type and configured baud.
///
/// Turkish MASS standard behavior:
/// - optical: always 300 bps (IEC 62056-21 requirement), negotiate up
/// - auto + specific baud: use that baud
/// - auto + auto (0): try 9600, then 300
/// - serial + specific baud: use that baud
/// - serial + auto (0): try 9600, then 300, then 19200
pub fn resolve_initial_bauds(connection_type: &str, configured_baud: u32) -> Vec<u32> {
    match connection_type {
        "optical" => vec![300],
        "auto" => {
            if configured_baud > 0 {
                vec![configured_baud]
            } else {
                vec![9600, 300]
            }
        }
        _ => {
            // "serial", "rs485", "rs232", or any other
            if configured_baud > 0 {
                vec![configured_baud]
            } else {
                vec![9600, 300, 19200]
            }
        }
    }
}

/// Determine the target baud rate for data transfer after handshake.
///
/// - optical or auto baud (0): negotiate to meter's max supported baud
/// - serial with explicit baud: keep that baud (no switch needed)
pub fn resolve_target_baud(connection_type: &str, configured_baud: u32, meter_max_baud: u32, meter_baud_char: char) -> (u32, char) {
    let target = if connection_type == "optical" || configured_baud == 0 {
        meter_max_baud
    } else {
        configured_baud
    };
    let baud_char = crate::serial::iec62056::char_from_baud_rate(target)
        .unwrap_or(meter_baud_char);
    (target, baud_char)
}

/// Result of reading until ETX
#[derive(Debug)]
pub struct ReadResult {
    /// The data buffer with received bytes
    pub data: Vec<u8>,
    /// Number of bytes actually read
    pub bytes_read: usize,
    /// Whether ETX was found
    pub found_etx: bool,
    /// Total read duration
    pub duration: Duration,
}

/// Read configuration
#[derive(Debug, Clone)]
pub struct ReadConfig {
    /// Buffer size in bytes
    pub buffer_size: usize,
    /// Idle timeout in milliseconds (time since last received byte)
    pub idle_timeout_ms: u64,
    /// Initial delay before starting to read (ms)
    pub initial_delay_ms: u64,
    /// Sleep duration between read attempts (ms)
    pub read_interval_ms: u64,
}

impl Default for ReadConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,      // 8KB default
            idle_timeout_ms: 3000,  // 3 seconds idle timeout
            initial_delay_ms: 300,  // 300ms initial delay
            read_interval_ms: 100,  // 100ms between reads
        }
    }
}

impl ReadConfig {
    /// Configuration for short read (Mode 6)
    pub fn short_read() -> Self {
        Self {
            buffer_size: 8192,       // 8KB
            idle_timeout_ms: 3000,   // 3 seconds
            initial_delay_ms: 300,
            read_interval_ms: 100,
        }
    }

    /// Configuration for full read (Mode 0)
    pub fn full_read() -> Self {
        Self {
            buffer_size: 131072,     // 128KB
            idle_timeout_ms: 5000,   // 5 seconds
            initial_delay_ms: 300,
            read_interval_ms: 100,
        }
    }

    /// Configuration for load profile read
    pub fn load_profile() -> Self {
        Self {
            buffer_size: 524288,     // 512KB
            idle_timeout_ms: 15000,  // 15 seconds for initial response
            initial_delay_ms: 500,
            read_interval_ms: 100,
        }
    }
}

/// Read from port until ETX byte is found or timeout occurs
///
/// This function implements the common pattern of reading data blocks
/// from the meter, looking for the ETX terminator.
///
/// # Arguments
/// * `port` - The serial port to read from
/// * `window` - Optional window for emitting activity events
/// * `config` - Read configuration
///
/// # Returns
/// * `Ok(ReadResult)` - The read data and metadata
/// * `Err(String)` - Error message if read failed
pub fn read_until_etx(
    port: &mut Box<dyn SerialPort>,
    window: Option<&Window>,
    config: &ReadConfig,
) -> Result<ReadResult, String> {
    let mut data_buf = vec![0u8; config.buffer_size];
    let mut total_read = 0;
    let mut found_etx = false;
    let read_start = Instant::now();
    let mut last_read_time = Instant::now();

    // Initial delay for data to start arriving
    std::thread::sleep(Duration::from_millis(config.initial_delay_ms));

    loop {
        match port.read(&mut data_buf[total_read..]) {
            Ok(n) if n > 0 => {
                total_read += n;
                last_read_time = Instant::now();

                // Emit activity if window is provided
                if let Some(w) = window {
                    let _ = w.emit("comm-activity", serde_json::json!({"type": "rx"}));
                }

                // Check for ETX
                if total_read >= 2 {
                    for i in 0..total_read - 1 {
                        if data_buf[i] == control::ETX && i + 1 < total_read {
                            found_etx = true;
                            break;
                        }
                    }
                    if found_etx {
                        break;
                    }
                }

                // Check if buffer is full
                if total_read >= config.buffer_size {
                    break;
                }
            }
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(config.read_interval_ms));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(config.read_interval_ms));
            }
            Err(e) => {
                return Err(format!("Okuma hatası: {}", e));
            }
        }

        // Check idle timeout
        if last_read_time.elapsed() > Duration::from_millis(config.idle_timeout_ms) {
            break;
        }
    }

    // Truncate buffer to actual size
    data_buf.truncate(total_read);

    Ok(ReadResult {
        data: data_buf,
        bytes_read: total_read,
        found_etx,
        duration: read_start.elapsed(),
    })
}

/// Verify BCC (Block Check Character) in the received data
///
/// Finds STX and ETX markers, calculates BCC from data between them,
/// and compares with the received BCC byte after ETX.
///
/// # Returns
/// * `Ok(true)` - BCC matches
/// * `Ok(false)` - BCC mismatch (with calculated and received values)
/// * `Err(String)` - Could not verify (missing markers)
pub fn verify_bcc(data: &[u8]) -> Result<bool, String> {
    let stx_idx = data.iter().position(|&b| b == control::STX)
        .ok_or("STX bulunamadı")?;
    let etx_idx = data.iter().position(|&b| b == control::ETX)
        .ok_or("ETX bulunamadı")?;

    if etx_idx + 1 >= data.len() {
        return Err("BCC byte'ı yok".to_string());
    }

    let received_bcc = data[etx_idx + 1];
    let calculated_bcc = iec62056::calculate_bcc(&data[stx_idx + 1..=etx_idx]);

    Ok(calculated_bcc == received_bcc)
}

/// Extract the data portion between STX and ETX
///
/// # Returns
/// The slice of data between STX (exclusive) and ETX (exclusive)
pub fn extract_data_block(data: &[u8]) -> Option<&[u8]> {
    let stx_idx = data.iter().position(|&b| b == control::STX)?;
    let etx_idx = data.iter().position(|&b| b == control::ETX)?;

    if stx_idx < etx_idx {
        Some(&data[stx_idx + 1..etx_idx])
    } else {
        None
    }
}

/// Send break command to end the session
pub fn send_break_command(port: &mut Box<dyn SerialPort>) -> Result<(), String> {
    use std::io::Write;

    let break_cmd = iec62056::build_break_command();
    port.write_all(&break_cmd).map_err(|e| format!("Break komutu gönderilemedi: {}", e))?;
    port.flush().map_err(|e| format!("Flush hatası: {}", e))?;
    std::thread::sleep(Duration::from_millis(100));
    Ok(())
}
