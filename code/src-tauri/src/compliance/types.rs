//! Compliance system type definitions
//!
//! Core data structures for the log-based compliance engine.

use serde::{Deserialize, Serialize};

// ─── Communication Log ──────────────────────────────────────────────────────

/// A single OBIS line as received from the meter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObisLine {
    /// OBIS code, e.g. "1.8.0", "F.F.0"
    pub code: String,
    /// Raw value string (with unit), e.g. "000123.456*kWh"
    pub raw_value: String,
    /// Parsed numeric or string value (unit stripped), e.g. "000123.456"
    pub value: String,
    /// Unit if present, e.g. "kWh", "V"
    pub unit: Option<String>,
}

/// Protocol-level event captured during communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolEvent {
    pub event_type: String,
    pub timestamp_ms: u64,
    pub detail: String,
    pub success: bool,
}

/// Handshake details for a single session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeLog {
    pub request_sent: bool,
    pub identification_received: bool,
    pub identification_raw: String,
    pub identification_format_valid: bool,
    pub ack_sent: bool,
    pub ack_mode: Option<String>,
    pub ack_baud_char: Option<String>,
    pub baud_negotiation_success: bool,
    pub initial_baud: u32,
    pub target_baud: u32,
    pub response_time_ms: u64,
}

/// Type of communication session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    ShortRead,
    FullRead,
    LoadProfile,
    ObisRead,
    TimeSync,
    ObisWrite,
}

impl SessionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "short_read" => Some(Self::ShortRead),
            "full_read" => Some(Self::FullRead),
            "load_profile" => Some(Self::LoadProfile),
            "obis_read" => Some(Self::ObisRead),
            "time_sync" => Some(Self::TimeSync),
            "obis_write" => Some(Self::ObisWrite),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::ShortRead => "short_read",
            Self::FullRead => "full_read",
            Self::LoadProfile => "load_profile",
            Self::ObisRead => "obis_read",
            Self::TimeSync => "time_sync",
            Self::ObisWrite => "obis_write",
        }
    }
}

/// Log of a single communication session (one read/write operation)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLog {
    pub session_type: SessionType,
    pub timestamp: String,
    pub handshake: HandshakeLog,
    pub obis_lines: Vec<ObisLine>,
    pub protocol_events: Vec<ProtocolEvent>,
    pub bcc_valid: Option<bool>,
    pub etx_found: bool,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    /// Which modes were successfully used (for mode_supported checks)
    pub modes_used: Vec<String>,
}

impl Default for SessionLog {
    fn default() -> Self {
        Self {
            session_type: SessionType::ShortRead,
            timestamp: String::new(),
            handshake: HandshakeLog::default(),
            obis_lines: Vec::new(),
            protocol_events: Vec::new(),
            bcc_valid: None,
            etx_found: false,
            duration_ms: 0,
            success: false,
            error: None,
            modes_used: Vec::new(),
        }
    }
}

/// Complete communication log for compliance evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunicationLog {
    pub meter_serial: String,
    pub meter_manufacturer: String,
    pub meter_model: String,
    pub profile_id: String,
    pub sessions: Vec<SessionLog>,
}

impl Default for CommunicationLog {
    fn default() -> Self {
        Self {
            meter_serial: String::new(),
            meter_manufacturer: String::new(),
            meter_model: String::new(),
            profile_id: String::new(),
            sessions: Vec::new(),
        }
    }
}

// ─── Compliance Results ─────────────────────────────────────────────────────

/// A single compliance violation
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceIssue {
    pub code: String,
    pub category: String,
    pub severity: String,
    pub description: String,
    pub expected: String,
    pub actual: String,
    pub spec_ref: Option<String>,
    pub cause: Option<String>,
    pub remedy: Option<String>,
    /// Which session type this issue was found in
    pub session_type: Option<String>,
    /// OBIS code(s) used for this check (e.g. "32.7.0" or "1.8.1, 1.8.2")
    pub obis_code: Option<String>,
}

/// Result of a single session's compliance check
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionCheckResult {
    pub session_type: String,
    pub success: bool,
    pub issues: Vec<ComplianceIssue>,
    pub obis_count: usize,
    pub duration_ms: u64,
}

/// Overall compliance check result
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceResult {
    pub issues: Vec<ComplianceIssue>,
    pub session_results: Vec<SessionCheckResult>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub total_rules_checked: usize,
    pub config_version: String,
    pub profile_id: String,
    pub profile_name: String,
    pub checked_at: String,
    pub config_file_path: String,
    /// Version status
    pub latest_version: Option<String>,
    pub rules_status: RulesStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RulesStatus {
    Ok,
    Offline,
    TooOld,
}
