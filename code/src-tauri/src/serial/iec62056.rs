//! IEC 62056-21 Mode C Protocol Implementation
//!
//! This module implements the IEC 62056-21 Mode C protocol for communication
//! with MASS-compliant electricity meters.

#![allow(dead_code)]

#[allow(unused_imports)]
use std::io::{Read, Write};
use std::time::Duration;
use serialport::{DataBits, Parity, StopBits, SerialPort};
use serde::{Deserialize, Serialize};

/// Control characters used in IEC 62056-21 protocol
pub mod control {
    pub const SOH: u8 = 0x01; // Start of header
    pub const STX: u8 = 0x02; // Start of text
    pub const ETX: u8 = 0x03; // End of text
    pub const EOT: u8 = 0x04; // End of transmission
    pub const ACK: u8 = 0x06; // Acknowledge
    pub const NAK: u8 = 0x15; // Not acknowledged
    pub const CR: u8 = 0x0D;  // Carriage return
    pub const LF: u8 = 0x0A;  // Line feed
}

/// Protocol modes for IEC 62056-21
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolMode {
    Readout = 0,        // Full readout (Mode 0)
    Programming = 1,    // Programming mode (Mode 1)
    TechQuality = 5,    // Technical quality (Packet 5)
    ShortRead = 6,      // Short read (Packet 6)
    Historical = 7,     // Historical data (Packet 7)
    Warnings = 8,       // Warning/alert packet (Packet 8)
    Outages = 9,        // Outage records (Packet 9)
}

impl ProtocolMode {
    pub fn as_char(&self) -> char {
        match self {
            ProtocolMode::Readout => '0',
            ProtocolMode::Programming => '1',
            ProtocolMode::TechQuality => '5',
            ProtocolMode::ShortRead => '6',
            ProtocolMode::Historical => '7',
            ProtocolMode::Warnings => '8',
            ProtocolMode::Outages => '9',
        }
    }
}

/// Meter identification information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterIdent {
    pub manufacturer: String,
    pub baud_char: char,
    pub generation: String,
    pub edas_id: String,
    pub model: String,
    pub max_baud_rate: u32,
}

/// OBIS code representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObisCode {
    pub a: u8,  // Media
    pub b: u8,  // Channel
    pub c: u8,  // Physical quantity
    pub d: u8,  // Processing type
    pub e: u8,  // Tariff
    pub f: u8,  // Billing period
}

impl ObisCode {
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 {
            return None;
        }

        Some(Self {
            a: parts.get(0).and_then(|p| p.parse().ok()).unwrap_or(0),
            b: parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0),
            c: parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
            d: parts.get(3).and_then(|p| p.parse().ok()).unwrap_or(0),
            e: parts.get(4).and_then(|p| p.parse().ok()).unwrap_or(0),
            f: parts.get(5).and_then(|p| p.parse().ok()).unwrap_or(0),
        })
    }

    pub fn to_string(&self) -> String {
        if self.f > 0 {
            format!("{}.{}.{}*{}", self.c, self.d, self.e, self.f)
        } else if self.e > 0 {
            format!("{}.{}.{}", self.c, self.d, self.e)
        } else {
            format!("{}.{}", self.c, self.d)
        }
    }
}

/// Parsed OBIS data item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObisDataItem {
    pub code: String,
    pub value: String,
    pub unit: Option<String>,
}

/// Short read data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortReadData {
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

    // Active Energy Export (for bidirectional meters)
    pub active_energy_export_total: Option<f64>,
    pub active_energy_export_t1: Option<f64>,
    pub active_energy_export_t2: Option<f64>,
    pub active_energy_export_t3: Option<f64>,
    pub active_energy_export_t4: Option<f64>,

    // Reactive Energy (for Kombi meters)
    pub reactive_inductive_import: Option<f64>,
    pub reactive_capacitive_import: Option<f64>,
    pub reactive_inductive_export: Option<f64>,
    pub reactive_capacitive_export: Option<f64>,

    // Maximum Demand
    pub max_demand_import: f64,
    pub max_demand_import_timestamp: String,
    pub max_demand_export: Option<f64>,
    pub max_demand_export_timestamp: Option<String>,

    // Instantaneous Values
    pub voltage_l1: f64,
    pub voltage_l2: Option<f64>,
    pub voltage_l3: Option<f64>,
    pub current_l1: f64,
    pub current_l2: Option<f64>,
    pub current_l3: Option<f64>,
    pub frequency: f64,
    pub power_factor_l1: f64,
    pub power_factor_l2: Option<f64>,
    pub power_factor_l3: Option<f64>,

    // Status Codes
    pub ff_code: String,
    pub gf_code: String,
    pub battery_status: String,
    pub relay_status: String,
}

/// Baud rate character mapping for IEC 62056-21
pub fn baud_rate_from_char(c: char) -> Option<u32> {
    match c {
        '0' => Some(300),
        '1' => Some(600),
        '2' => Some(1200),
        '3' => Some(2400),
        '4' => Some(4800),
        '5' => Some(9600),
        '6' => Some(19200),
        _ => None,
    }
}

/// Baud rate character for IEC 62056-21
pub fn char_from_baud_rate(baud: u32) -> Option<char> {
    match baud {
        300 => Some('0'),
        600 => Some('1'),
        1200 => Some('2'),
        2400 => Some('3'),
        4800 => Some('4'),
        9600 => Some('5'),
        19200 => Some('6'),
        _ => None,
    }
}

/// Calculate BCC (Block Check Character) for data integrity
/// XOR of all bytes from after SOH/STX up to and including ETX
pub fn calculate_bcc(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
}

/// Verify BCC of received data
pub fn verify_bcc(data: &[u8], expected_bcc: u8) -> bool {
    calculate_bcc(data) == expected_bcc
}

/// Open a serial port with IEC 62056-21 Mode C settings (7E1)
pub fn open_port(port_name: &str, baud_rate: u32, timeout_ms: u64) -> Result<Box<dyn SerialPort>, String> {
    serialport::new(port_name, baud_rate)
        .data_bits(DataBits::Seven)
        .parity(Parity::Even)
        .stop_bits(StopBits::One)
        .timeout(Duration::from_millis(timeout_ms))
        .open()
        .map_err(|e| format!("Failed to open port {}: {}", port_name, e))
}

/// Build the request message for handshake
/// Format: /?ADDRESS!\r\n or /?!\r\n (if no address)
pub fn build_request_message(address: Option<&str>) -> Vec<u8> {
    let mut msg = vec![b'/', b'?'];
    if let Some(addr) = address {
        msg.extend_from_slice(addr.as_bytes());
    }
    msg.push(b'!');
    msg.push(control::CR);
    msg.push(control::LF);
    msg
}

/// Build the acknowledgment message for mode selection
/// Format: ACK V Z Y CR LF
pub fn build_ack_message(mode: ProtocolMode, baud_char: char) -> Vec<u8> {
    vec![
        control::ACK,
        mode.as_char() as u8,
        baud_char as u8,
        mode.as_char() as u8,
        control::CR,
        control::LF,
    ]
}

/// Build password command for programming mode
/// Format: SOH P1 STX (PASSWORD) ETX BCC
pub fn build_password_command(password: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(control::SOH);
    msg.push(b'P');
    msg.push(b'1');
    msg.push(control::STX);
    msg.push(b'(');
    msg.extend_from_slice(password.as_bytes());
    msg.push(b')');
    msg.push(control::ETX);

    // Calculate BCC from STX to ETX
    let bcc_data = &msg[3..];
    let bcc = calculate_bcc(bcc_data);
    msg.push(bcc);

    msg
}

/// Build OBIS read command
/// Format: SOH R2 STX OBIS() ETX BCC
pub fn build_read_command(obis: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(control::SOH);
    msg.push(b'R');
    msg.push(b'2');
    msg.push(control::STX);
    msg.extend_from_slice(obis.as_bytes());
    msg.push(b'(');
    msg.push(b')');
    msg.push(control::ETX);

    let bcc_data = &msg[3..];
    let bcc = calculate_bcc(bcc_data);
    msg.push(bcc);

    msg
}

/// Build OBIS write command
/// Format: SOH W2 STX OBIS(value) ETX BCC
pub fn build_write_command(obis: &str, value: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(control::SOH);
    msg.push(b'W');
    msg.push(b'2');
    msg.push(control::STX);
    msg.extend_from_slice(obis.as_bytes());
    msg.push(b'(');
    msg.extend_from_slice(value.as_bytes());
    msg.push(b')');
    msg.push(control::ETX);

    let bcc_data = &msg[3..];
    let bcc = calculate_bcc(bcc_data);
    msg.push(bcc);

    msg
}

/// Build break/logout command
/// Format: SOH B0 ETX BCC
pub fn build_break_command() -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(control::SOH);
    msg.push(b'B');
    msg.push(b'0');
    msg.push(control::ETX);

    let bcc_data = &msg[3..];
    let bcc = calculate_bcc(bcc_data);
    msg.push(bcc);

    msg
}

/// Parse the meter identification message
/// Format: /XXXZ<generation>YYYYY(MODEL)\r\n
/// Example: /MKS5<2>ADM(M550.2251)
pub fn parse_identification(response: &str) -> Option<MeterIdent> {
    if !response.starts_with('/') {
        return None;
    }

    let content = response.trim_start_matches('/').trim();

    // Extract manufacturer code (3 chars)
    if content.len() < 4 {
        return None;
    }

    let manufacturer = content[..3].to_string();
    let baud_char = content.chars().nth(3)?;
    let max_baud = baud_rate_from_char(baud_char)?;

    // Find generation marker
    let gen_start = content.find('<')?;
    let gen_end = content.find('>')?;
    let generation = content[gen_start + 1..gen_end].to_string();

    // EDAS ID is after the generation marker
    let after_gen = &content[gen_end + 1..];
    let model_start = after_gen.find('(')?;
    let edas_id = after_gen[..model_start].to_string();

    // Model is inside parentheses
    let model_end = after_gen.find(')')?;
    let model = after_gen[model_start + 1..model_end].to_string();

    Some(MeterIdent {
        manufacturer,
        baud_char,
        generation,
        edas_id,
        model,
        max_baud_rate: max_baud,
    })
}

/// Parse OBIS data from response
/// Format: OBIS(value*unit)\r\n or OBIS(value)\r\n
pub fn parse_obis_response(line: &str) -> Option<ObisDataItem> {
    let open_paren = line.find('(')?;
    let close_paren = line.rfind(')')?;

    if open_paren >= close_paren {
        return None;
    }

    let code = line[..open_paren].trim().to_string();
    let value_part = &line[open_paren + 1..close_paren];

    // Check for unit separator
    let (value, unit) = if let Some(star_pos) = value_part.find('*') {
        (
            value_part[..star_pos].to_string(),
            Some(value_part[star_pos + 1..].to_string()),
        )
    } else {
        (value_part.to_string(), None)
    };

    Some(ObisDataItem { code, value, unit })
}

/// Parse multiple OBIS lines from a data block
pub fn parse_data_block(data: &str) -> Vec<ObisDataItem> {
    data.lines()
        .filter_map(|line| parse_obis_response(line.trim()))
        .collect()
}

/// EDAŞ ID lookup table
pub fn edas_name_from_id(id: u8) -> &'static str {
    match id {
        1 => "AKDENİZ EDAŞ",
        2 => "AKEDAŞ",
        3 => "ARAS EDAŞ",
        4 => "AYDEM",
        5 => "AYEDAŞ",
        6 => "BAŞKENT EDAŞ",
        7 => "BOĞAZİÇİ EDAŞ",
        8 => "ÇAMLIBEL EDAŞ",
        9 => "ÇORUH EDAŞ",
        10 => "DİCLE EDAŞ",
        11 => "FIRAT EDAŞ",
        12 => "GEDİZ EDAŞ",
        13 => "KCETAŞ",
        14 => "MERAM EDAŞ",
        15 => "OSMANGAZİ EDAŞ",
        16 => "SAKARYA EDAŞ",
        17 => "TOROSLAR EDAŞ",
        18 => "TRAKYA EDAŞ",
        19 => "ULUDAĞ EDAŞ",
        20 => "VANGÖLÜ EDAŞ",
        21 => "YEŞİLIRMAK EDAŞ",
        _ => "Unknown",
    }
}

/// Parse GF code fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfCodeFields {
    pub edas_id: u8,
    pub edas_name: String,
    pub trafo_merkez_id: u16,
    pub trafo_id: u8,
    pub depar_id: u8,
    pub faz_id: u8,
    pub kol_id: u8,
    pub max_current: u16,
}

pub fn parse_gf_code(code: u64) -> GfCodeFields {
    let edas_id = (code & 0x1F) as u8;
    let trafo_merkez_id = ((code >> 5) & 0x7FFF) as u16;
    let trafo_id = ((code >> 20) & 0x0F) as u8;
    let depar_id = ((code >> 24) & 0x3F) as u8;
    let faz_id = ((code >> 30) & 0x03) as u8;
    let kol_id = ((code >> 32) & 0x03) as u8;
    let max_current = ((code >> 34) & 0x3FF) as u16;

    GfCodeFields {
        edas_id,
        edas_name: edas_name_from_id(edas_id).to_string(),
        trafo_merkez_id,
        trafo_id,
        depar_id,
        faz_id,
        kol_id,
        max_current,
    }
}

/// Format bytes for display with control character names
/// Example: [0x01, 0x50, 0x31, 0x02] -> "<SOH>P1<STX>"
pub fn format_bytes_for_display(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            control::SOH => result.push_str("<SOH>"),
            control::STX => result.push_str("<STX>"),
            control::ETX => result.push_str("<ETX>"),
            control::EOT => result.push_str("<EOT>"),
            control::ACK => result.push_str("<ACK>"),
            control::NAK => result.push_str("<NAK>"),
            control::CR => {
                result.push_str("<CR>");
                // Check if next byte is LF
                if i + 1 < bytes.len() && bytes[i + 1] == control::LF {
                    result.push_str("<LF>");
                    i += 1; // Skip the LF since we already handled it
                    // Only add newline if there's more content after CR+LF
                    if i + 1 < bytes.len() {
                        result.push('\n');
                    }
                }
            },
            control::LF => {
                result.push_str("<LF>");
                // Only add newline if there's more content after LF
                if i + 1 < bytes.len() {
                    result.push('\n');
                }
            },
            0x20..=0x7E => result.push(bytes[i] as char), // Printable ASCII
            _ => result.push_str(&format!("<0x{:02X}>", bytes[i])),
        }
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcc_calculation() {
        let data = b"test";
        let bcc = calculate_bcc(data);
        assert_eq!(bcc, b't' ^ b'e' ^ b's' ^ b't');
    }

    #[test]
    fn test_baud_rate_mapping() {
        assert_eq!(baud_rate_from_char('5'), Some(9600));
        assert_eq!(baud_rate_from_char('0'), Some(300));
        assert_eq!(char_from_baud_rate(9600), Some('5'));
    }

    #[test]
    fn test_parse_identification() {
        let response = "/MKS5<2>ADM(M550.2251)\r\n";
        let result = parse_identification(response);
        assert!(result.is_some());
        let ident = result.unwrap();
        assert_eq!(ident.manufacturer, "MKS");
        assert_eq!(ident.baud_char, '5');
        assert_eq!(ident.generation, "2");
        assert_eq!(ident.edas_id, "ADM");
        assert_eq!(ident.model, "M550.2251");
        assert_eq!(ident.max_baud_rate, 9600);
    }

    #[test]
    fn test_parse_obis_response() {
        let line = "1.8.0(00123.456*kWh)";
        let result = parse_obis_response(line);
        assert!(result.is_some());
        let item = result.unwrap();
        assert_eq!(item.code, "1.8.0");
        assert_eq!(item.value, "00123.456");
        assert_eq!(item.unit, Some("kWh".to_string()));
    }

    #[test]
    fn test_build_request_message() {
        let msg = build_request_message(Some("123456789"));
        assert_eq!(msg, b"/?123456789!\r\n");

        let msg_no_addr = build_request_message(None);
        assert_eq!(msg_no_addr, b"/?!\r\n");
    }

    #[test]
    fn test_obis_code() {
        let obis = ObisCode::new(1, 0, 1, 8, 0, 0);
        assert_eq!(obis.to_string(), "1.8.0");

        let obis_tariff = ObisCode::new(1, 0, 1, 8, 1, 0);
        assert_eq!(obis_tariff.to_string(), "1.8.1");
    }
}
