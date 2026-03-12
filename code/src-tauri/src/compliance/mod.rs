//! TEDAS MLZ/2017-062.B Compliance Checker v3
//!
//! Log-based compliance engine that evaluates meter communication logs
//! against configurable rules organized by category.

pub mod types;
pub mod config;
pub mod engine;
pub mod updater;

pub use types::*;

/// Main entry point: run compliance check on a CommunicationLog.
pub fn run_check(
    log: &CommunicationLog,
    profile_id: &str,
    latest_version: Option<String>,
) -> ComplianceResult {
    let config_file_path = config::get_config_path().display().to_string();

    let config = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Config dosyası yüklenemedi: {}", e);
            return ComplianceResult {
                issues: vec![ComplianceIssue {
                    code: "SYS-001".to_string(),
                    category: "system".to_string(),
                    severity: "error".to_string(),
                    description: format!("Config dosyası yüklenemedi: {}", e),
                    expected: "Geçerli config dosyası".to_string(),
                    actual: e.to_string(),
                    spec_ref: None,
                    cause: None,
                    remedy: None,
                    session_type: None,
                    obis_code: None,
                }],
                session_results: vec![],
                error_count: 1,
                warning_count: 0,
                info_count: 0,
                total_rules_checked: 0,
                config_version: "unknown".to_string(),
                profile_id: profile_id.to_string(),
                profile_name: String::new(),
                checked_at: chrono::Local::now().to_rfc3339(),
                config_file_path,
                latest_version,
                rules_status: RulesStatus::Offline,
            };
        }
    };

    let rules_status = compute_rules_status(
        &config.config_version,
        latest_version.as_deref(),
    );

    // If rules are too old, block the check
    if rules_status == RulesStatus::TooOld {
        return ComplianceResult {
            issues: vec![],
            session_results: vec![],
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            total_rules_checked: 0,
            config_version: config.config_version,
            profile_id: profile_id.to_string(),
            profile_name: String::new(),
            checked_at: chrono::Local::now().to_rfc3339(),
            config_file_path,
            latest_version,
            rules_status,
        };
    }

    let profile_name = config::find_profile(&config, profile_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| profile_id.to_string());

    let (issues, session_results, total_rules) = engine::evaluate(&config, log, profile_id);

    let error_count = issues.iter().filter(|i| i.severity == "error").count();
    let warning_count = issues.iter().filter(|i| i.severity == "warning").count();
    let info_count = issues.iter().filter(|i| i.severity == "info").count();

    ComplianceResult {
        issues,
        session_results,
        error_count,
        warning_count,
        info_count,
        total_rules_checked: total_rules,
        config_version: config.config_version,
        profile_id: profile_id.to_string(),
        profile_name,
        checked_at: chrono::Local::now().to_rfc3339(),
        config_file_path,
        latest_version,
        rules_status,
    }
}

/// Run compliance check using legacy ShortReadResult (backward compat).
/// Converts ShortReadResult to CommunicationLog format.
pub fn run_check_legacy(
    data: &crate::commands::types::ShortReadResult,
    latest_version: Option<String>,
    meter_phases: u8,
) -> ComplianceResult {
    let log = communication_log_from_short_read(data);
    let profile_id = match meter_phases {
        1 => "single_phase",
        _ => "three_phase_direct",
    };
    run_check(&log, profile_id, latest_version)
}

/// Convert a ShortReadResult into a CommunicationLog for backward compat.
fn communication_log_from_short_read(data: &crate::commands::types::ShortReadResult) -> CommunicationLog {
    use crate::serial::iec62056;

    let mut obis_lines = Vec::new();

    // Parse raw_data if available (best path — preserves all OBIS lines)
    if let Some(ref raw) = data.raw_data {
        let items = iec62056::parse_data_block(raw);
        for item in items {
            obis_lines.push(ObisLine {
                code: item.code.clone(),
                raw_value: match &item.unit {
                    Some(u) => format!("{}*{}", item.value, u),
                    None => item.value.clone(),
                },
                value: item.value,
                unit: item.unit,
            });
        }
    }

    // If no raw data or no OBIS lines parsed, build from struct fields
    if obis_lines.is_empty() {
        obis_lines = build_obis_from_struct(data);
    }

    let session = SessionLog {
        session_type: SessionType::ShortRead,
        timestamp: chrono::Local::now().to_rfc3339(),
        handshake: HandshakeLog {
            request_sent: true,
            identification_received: true,
            identification_format_valid: true,
            baud_negotiation_success: true,
            ..Default::default()
        },
        obis_lines,
        bcc_valid: Some(true),
        etx_found: true,
        success: true,
        modes_used: vec!["6".to_string()],
        ..Default::default()
    };

    CommunicationLog {
        meter_serial: data.serial_number.clone(),
        sessions: vec![session],
        ..Default::default()
    }
}

/// Build ObisLine list from ShortReadResult struct fields (fallback)
fn build_obis_from_struct(data: &crate::commands::types::ShortReadResult) -> Vec<ObisLine> {
    let mut lines = Vec::new();

    let add_str = |lines: &mut Vec<ObisLine>, code: &str, val: &str| {
        if !val.is_empty() {
            lines.push(ObisLine {
                code: code.to_string(),
                raw_value: val.to_string(),
                value: val.to_string(),
                unit: None,
            });
        }
    };
    let add_f64 = |lines: &mut Vec<ObisLine>, code: &str, val: f64, unit: &str| {
        lines.push(ObisLine {
            code: code.to_string(),
            raw_value: format!("{val}*{unit}"),
            value: format!("{val}"),
            unit: Some(unit.to_string()),
        });
    };

    add_str(&mut lines, "0.0.0", &data.serial_number);
    add_str(&mut lines, "0.2.0", &data.program_version);
    add_str(&mut lines, "0.9.2", &data.meter_date);
    add_str(&mut lines, "0.9.1", &data.meter_time);
    add_str(&mut lines, "0.1.0", &data.production_date);
    add_str(&mut lines, "96.2.5", &data.calibration_date);

    add_f64(&mut lines, "32.7.0", data.voltage_l1, "V");
    add_f64(&mut lines, "52.7.0", data.voltage_l2, "V");
    add_f64(&mut lines, "72.7.0", data.voltage_l3, "V");
    add_f64(&mut lines, "31.7.0", data.current_l1, "A");
    add_f64(&mut lines, "51.7.0", data.current_l2, "A");
    add_f64(&mut lines, "71.7.0", data.current_l3, "A");
    add_f64(&mut lines, "14.7.0", data.frequency, "Hz");
    add_f64(&mut lines, "33.7.0", data.power_factor_l1, "");
    add_f64(&mut lines, "53.7.0", data.power_factor_l2, "");
    add_f64(&mut lines, "73.7.0", data.power_factor_l3, "");

    add_f64(&mut lines, "1.8.0", data.active_energy_import_total, "kWh");
    add_f64(&mut lines, "1.8.1", data.active_energy_import_t1, "kWh");
    add_f64(&mut lines, "1.8.2", data.active_energy_import_t2, "kWh");
    add_f64(&mut lines, "1.8.3", data.active_energy_import_t3, "kWh");
    add_f64(&mut lines, "1.8.4", data.active_energy_import_t4, "kWh");

    add_f64(&mut lines, "2.8.0", data.active_energy_export_total, "kWh");
    add_f64(&mut lines, "5.8.0", data.reactive_energy_inductive_import, "kVArh");
    add_f64(&mut lines, "6.8.0", data.reactive_energy_capacitive_import, "kVArh");
    add_f64(&mut lines, "7.8.0", data.reactive_energy_inductive_export, "kVArh");
    add_f64(&mut lines, "8.8.0", data.reactive_energy_capacitive_export, "kVArh");

    add_str(&mut lines, "F.F.0", &data.ff_code);
    add_str(&mut lines, "F.F.1", &data.gf_code);
    add_str(&mut lines, "0.8.0", &data.demand_period);
    add_str(&mut lines, "0.8.4", &data.lp_period);

    lines
}

/// Version status calculation
pub fn compute_rules_status(local: &str, latest: Option<&str>) -> RulesStatus {
    let latest = match latest {
        Some(v) => v,
        None => return RulesStatus::Offline,
    };
    let local_major = parse_major(local);
    let latest_major = parse_major(latest);
    if latest_major.saturating_sub(local_major) >= 2 {
        RulesStatus::TooOld
    } else {
        RulesStatus::Ok
    }
}

fn parse_major(version: &str) -> u64 {
    version.split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}
