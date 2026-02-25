//! Type definitions for Tauri commands
//!
//! Contains data structures used in meter communication commands.

use serde::{Deserialize, Serialize};

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

    // Active Energy Export
    pub active_energy_export_total: f64,
    pub active_energy_export_t1: f64,
    pub active_energy_export_t2: f64,
    pub active_energy_export_t3: f64,
    pub active_energy_export_t4: f64,

    // Reactive Energy
    pub reactive_energy_inductive_import: f64,
    pub reactive_energy_capacitive_import: f64,
    pub reactive_energy_inductive_export: f64,
    pub reactive_energy_capacitive_export: f64,

    // Export Maximum Demand
    pub max_demand_export: f64,
    pub max_demand_export_timestamp: String,

    // Instantaneous Power Values
    pub total_active_power: f64,
    pub active_power_l1: f64,
    pub active_power_l2: f64,
    pub active_power_l3: f64,
    pub total_reactive_power: f64,
    pub neutral_current: f64,

    // Configuration
    pub demand_period: String,
    pub lp_period: String,
    pub load_limit_threshold: String,
    pub load_limit_period: String,

    // Status Codes
    pub ff_code: String,
    pub gf_code: String,
    pub battery_status: String,
    pub relay_status: String,

    // Raw data for debugging
    pub raw_data: Option<String>,

    // Timestamp (epoch ms) captured when 0.9.1 and 0.9.2 were received from meter
    pub time_of_09x_read: Option<u64>,
}

/// Mode-specific packet read result (Modes 5, 7, 8, 9)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketReadResult {
    pub mode: u8,
    pub raw_data: String,
    pub bytes_read: usize,
    pub read_duration_ms: u64,
    pub bcc_valid: bool,
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

/// Load profile entry parsed from response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadProfileEntry {
    pub timestamp: String,
    pub values: Vec<f64>,
    pub status: Option<String>,
}

/// Load profile read result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadProfileResult {
    pub profile_number: u8,
    pub entries: Vec<LoadProfileEntry>,
    pub raw_data: String,
}

/// Session data structure for saving
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionData {
    pub flag: String,
    pub serial_number: String,
    pub model: String,
    pub saved_at: String,
    pub note: String,
    pub meter_data: serde_json::Value,
    pub connection_info: serde_json::Value,
}
