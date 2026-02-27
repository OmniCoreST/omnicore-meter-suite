//! Kural değerlendirme motoru

use crate::commands::types::ShortReadResult;
use super::rules::{Rule, RulesFile};
use super::ComplianceIssue;

pub fn evaluate(rules_file: &RulesFile, data: &ShortReadResult, meter_phases: u8) -> Vec<ComplianceIssue> {
    rules_file.rules.iter()
        .filter_map(|rule| {
            // phases kısıtı varsa uyuşmayan sayaçlarda atla
            if let Some(required_phases) = rule.phases {
                if required_phases != meter_phases {
                    return None;
                }
            }
            evaluate_rule(rule, data)
        })
        .collect()
}

fn evaluate_rule(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    match rule.check.as_str() {
        "range"              => check_range(rule, data),
        "equals"             => check_equals(rule, data),
        "not_equals"         => check_not_equals(rule, data),
        "not_empty"          => check_not_empty(rule, data),
        "bit_zero"           => check_bit(rule, data, false),
        "bit_one"            => check_bit(rule, data, true),
        "tariff_balance"     => check_tariff_balance(rule, data),
        "time_drift_minutes" => check_time_drift(rule, data),
        other => {
            log::warn!("Bilinmeyen kural tipi: {}", other);
            None
        }
    }
}

/// ShortReadResult'tan alan değerini string olarak alır
fn get_field_str(field: &str, data: &ShortReadResult) -> Option<String> {
    match field {
        "serial_number"              => Some(data.serial_number.clone()),
        "program_version"            => Some(data.program_version.clone()),
        "production_date"            => Some(data.production_date.clone()),
        "calibration_date"           => Some(data.calibration_date.clone()),
        "meter_date"                 => Some(data.meter_date.clone()),
        "meter_time"                 => Some(data.meter_time.clone()),
        "voltage_l1"                 => Some(data.voltage_l1.to_string()),
        "voltage_l2"                 => Some(data.voltage_l2.to_string()),
        "voltage_l3"                 => Some(data.voltage_l3.to_string()),
        "current_l1"                 => Some(data.current_l1.to_string()),
        "current_l2"                 => Some(data.current_l2.to_string()),
        "current_l3"                 => Some(data.current_l3.to_string()),
        "frequency"                  => Some(data.frequency.to_string()),
        "power_factor_l1"            => Some(data.power_factor_l1.to_string()),
        "power_factor_l2"            => Some(data.power_factor_l2.to_string()),
        "power_factor_l3"            => Some(data.power_factor_l3.to_string()),
        "active_energy_import_total" => Some(data.active_energy_import_total.to_string()),
        "active_energy_import_t1"    => Some(data.active_energy_import_t1.to_string()),
        "active_energy_import_t2"    => Some(data.active_energy_import_t2.to_string()),
        "active_energy_import_t3"    => Some(data.active_energy_import_t3.to_string()),
        "active_energy_import_t4"    => Some(data.active_energy_import_t4.to_string()),
        "ff_code"                    => Some(data.ff_code.clone()),
        "gf_code"                    => Some(data.gf_code.clone()),
        "battery_status"             => Some(data.battery_status.clone()),
        "relay_status"               => Some(data.relay_status.clone()),
        "demand_period"              => Some(data.demand_period.clone()),
        "lp_period"                  => Some(data.lp_period.clone()),
        // Özel kontroller için alan çözümlemesi ayrı fonksiyonlarda
        "tariff_sum" | "time_drift"  => None,
        _ => {
            log::warn!("Bilinmeyen alan: {}", field);
            None
        }
    }
}

/// Sayısal aralık kontrolü
fn check_range(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    let value_str = get_field_str(&rule.field, data)?;
    let value: f64 = value_str.parse().ok()?;

    let min = rule.min.unwrap_or(f64::NEG_INFINITY);
    let max = rule.max.unwrap_or(f64::INFINITY);

    if value < min || value > max {
        let expected = match (rule.min, rule.max) {
            (Some(mn), Some(mx)) => format!("{mn}–{mx}"),
            (Some(mn), None)     => format!("≥{mn}"),
            (None, Some(mx))     => format!("≤{mx}"),
            (None, None)         => "herhangi".to_string(),
        };
        Some(make_issue(rule, expected, format!("{:.3}", value)))
    } else {
        None
    }
}

/// String eşitliği kontrolü
fn check_equals(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    let value_str = get_field_str(&rule.field, data)?;
    let expected_val = rule.value.as_deref().unwrap_or("");
    if value_str.trim() != expected_val {
        Some(make_issue(rule, expected_val.to_string(), value_str))
    } else {
        None
    }
}

/// Eşit olmamalı kontrolü
fn check_not_equals(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    let value_str = get_field_str(&rule.field, data)?;
    let forbidden = rule.value.as_deref().unwrap_or("");
    if value_str.trim() == forbidden {
        Some(make_issue(rule, format!("≠ {forbidden}"), value_str))
    } else {
        None
    }
}

/// Boş olmamalı kontrolü
fn check_not_empty(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    let value_str = get_field_str(&rule.field, data)?;
    if value_str.trim().is_empty() {
        Some(make_issue(rule, "Boş olmayan değer".to_string(), "(boş)".to_string()))
    } else {
        None
    }
}

/// ff_code / gf_code bit kontrolü
/// ff_code hex string olarak gelir: "0000000000000000" (16 hex char = 64 bit)
fn check_bit(rule: &Rule, data: &ShortReadResult, expect_one: bool) -> Option<ComplianceIssue> {
    let code_str = get_field_str(&rule.field, data)?;
    let bit_index = rule.bit.unwrap_or(0) as u64;

    let code_value: u64 = if code_str.chars().all(|c| c == '0' || c == '1') && code_str.len() > 8 {
        // Binary string (OBIS'ten direkt gelen)
        u64::from_str_radix(&code_str.chars().take(64).collect::<String>(), 2).unwrap_or(0)
    } else {
        // Hex string
        let trimmed = code_str.trim_start_matches("0x").trim_start_matches("0X");
        if trimmed.is_empty() { 0 } else { u64::from_str_radix(trimmed, 16).unwrap_or(0) }
    };

    let bit_is_set = (code_value >> bit_index) & 1 == 1;

    if expect_one && !bit_is_set {
        Some(make_issue(rule, format!("Bit {bit_index} = 1"), format!("Bit {bit_index} = 0")))
    } else if !expect_one && bit_is_set {
        Some(make_issue(rule, format!("Bit {bit_index} = 0"), format!("Bit {bit_index} = 1")))
    } else {
        None
    }
}

/// Tarife dengesi: T1+T2+T3+T4 ≈ toplam
fn check_tariff_balance(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    let total = data.active_energy_import_total;
    let sum = data.active_energy_import_t1
        + data.active_energy_import_t2
        + data.active_energy_import_t3
        + data.active_energy_import_t4;

    if total == 0.0 && sum == 0.0 {
        return None; // Henüz veri yok
    }

    let tolerance = rule.tolerance.unwrap_or(0.01);
    let diff = (total - sum).abs();

    if diff > tolerance {
        Some(make_issue(
            rule,
            format!("Fark ≤ {tolerance} kWh"),
            format!("T={total:.3}, T1+T2+T3+T4={sum:.3}, Δ={diff:.3} kWh"),
        ))
    } else {
        None
    }
}

/// Sayaç/sistem saat farkı kontrolü
fn check_time_drift(rule: &Rule, data: &ShortReadResult) -> Option<ComplianceIssue> {
    let max_drift = rule.max_drift.unwrap_or(5);

    let date_str = data.meter_date.trim();
    let time_str = data.meter_time.trim();
    if date_str.is_empty() || time_str.is_empty() {
        return None;
    }

    let meter_dt_str = format!("{} {}", date_str, time_str);
    let meter_naive = chrono::NaiveDateTime::parse_from_str(&meter_dt_str, "%d.%m.%Y %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&meter_dt_str, "%d.%m.%Y %H:%M"))
        .ok()?;

    // Referans: 0.9.x okunduğundaki sistem zamanı veya şimdiki zaman
    let reference_epoch_ms = data.time_of_09x_read.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    });

    let reference_dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(reference_epoch_ms as i64)?;
    let reference_naive = reference_dt.with_timezone(&chrono::Local).naive_local();

    let drift_secs = (reference_naive - meter_naive).num_seconds().abs();
    let drift_minutes = drift_secs / 60;

    if drift_minutes > max_drift {
        Some(make_issue(rule, format!("≤ {max_drift} dakika"), format!("{drift_minutes} dakika")))
    } else {
        None
    }
}

fn make_issue(rule: &Rule, expected: String, actual: String) -> ComplianceIssue {
    ComplianceIssue {
        code: rule.code.clone(),
        severity: rule.severity.clone(),
        field: rule.field.clone(),
        expected,
        actual,
        description: rule.description.clone(),
        spec_ref: rule.spec_ref.clone(),
    }
}
