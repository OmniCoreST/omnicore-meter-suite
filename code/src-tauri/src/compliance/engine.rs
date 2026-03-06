//! Category-based compliance rule evaluation engine
//!
//! Evaluates rules against CommunicationLog data, dispatching to
//! category-specific checkers.

use super::config::{ComplianceConfig, Rule};
use super::types::*;
use std::collections::HashMap;

/// Main evaluation entry point.
/// Returns all issues found across all sessions.
pub fn evaluate(
    config: &ComplianceConfig,
    log: &CommunicationLog,
    profile_id: &str,
) -> (Vec<ComplianceIssue>, Vec<SessionCheckResult>, usize) {
    let rules = super::config::rules_for_profile(config, profile_id);
    let total_rules = rules.len();

    // Build OBIS lookup per session type for fast access
    let session_obis = build_session_obis_map(log);

    let mut all_issues = Vec::new();
    let mut session_results = Vec::new();

    // Evaluate session-level results
    for session in &log.sessions {
        let session_type_str = session.session_type.as_str();
        let mut session_issues = Vec::new();

        for rule in &rules {
            // Check session_type filter
            if !rule_applies_to_session(rule, session_type_str) {
                continue;
            }

            let issues = evaluate_rule(rule, session, &session_obis, log);
            session_issues.extend(issues);
        }

        session_results.push(SessionCheckResult {
            session_type: session_type_str.to_string(),
            success: session.success,
            issues: session_issues.clone(),
            obis_count: session.obis_lines.len(),
            duration_ms: session.duration_ms,
        });
        all_issues.extend(session_issues);
    }

    // Also evaluate rules with no specific session requirement ("any")
    // against the combined data from all sessions
    for rule in &rules {
        let session_filter = rule.session_type.as_deref().unwrap_or("any");
        if session_filter != "any" {
            continue; // Already handled per-session above
        }

        // For "any" rules, find the best matching session
        // (prefer full_read > short_read > others)
        let best_session = log.sessions.iter()
            .find(|s| s.session_type == SessionType::FullRead && s.success)
            .or_else(|| log.sessions.iter().find(|s| s.session_type == SessionType::ShortRead && s.success))
            .or_else(|| log.sessions.iter().find(|s| s.success));

        if let Some(session) = best_session {
            let issues = evaluate_rule(rule, session, &session_obis, log);
            all_issues.extend(issues);
        }
    }

    // Deduplicate issues by code (same rule can match in multiple sessions)
    all_issues.sort_by(|a, b| a.code.cmp(&b.code));
    all_issues.dedup_by(|a, b| a.code == b.code);

    (all_issues, session_results, total_rules)
}

fn rule_applies_to_session(rule: &Rule, session_type: &str) -> bool {
    match rule.session_type.as_deref() {
        None | Some("any") => false, // Handled separately in the combined pass
        Some(st) => st == session_type,
    }
}

/// Dispatch rule evaluation to the appropriate category handler
fn evaluate_rule(
    rule: &Rule,
    session: &SessionLog,
    _session_obis: &HashMap<String, Vec<&ObisLine>>,
    log: &CommunicationLog,
) -> Vec<ComplianceIssue> {
    match rule.category.as_str() {
        "obis_existence" => check_obis_existence(rule, session),
        "obis_format"    => check_obis_format(rule, session),
        "obis_value"     => check_obis_value(rule, session),
        "cross_value"    => check_cross_value(rule, session),
        "protocol"       => check_protocol(rule, session, log),
        "session"        => check_session(rule, session, log),
        "load_profile"   => check_load_profile(rule, session),
        "full_read"      => check_full_read(rule, session),
        other => {
            log::warn!("Unknown rule category: {} (rule {})", other, rule.code);
            vec![]
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Build a map: session_type_str -> Vec<&ObisLine> for all sessions
fn build_session_obis_map<'a>(log: &'a CommunicationLog) -> HashMap<String, Vec<&'a ObisLine>> {
    let mut map = HashMap::new();
    for session in &log.sessions {
        let key = session.session_type.as_str().to_string();
        let lines: Vec<&ObisLine> = session.obis_lines.iter().collect();
        map.entry(key).or_insert_with(Vec::new).extend(lines);
    }
    map
}

/// Find an OBIS line by code in a session
fn find_obis<'a>(session: &'a SessionLog, code: &str) -> Option<&'a ObisLine> {
    session.obis_lines.iter().find(|l| l.code == code)
}

/// Get the numeric value from an OBIS line (unit-stripped)
fn obis_numeric(session: &SessionLog, code: &str) -> Option<f64> {
    find_obis(session, code).and_then(|l| l.value.trim().parse::<f64>().ok())
}

fn make_issue(rule: &Rule, expected: String, actual: String, session_type: Option<&str>) -> ComplianceIssue {
    ComplianceIssue {
        code: rule.code.clone(),
        category: rule.category.clone(),
        severity: rule.severity.clone(),
        description: rule.description.clone(),
        expected,
        actual,
        spec_ref: rule.spec_ref.clone(),
        cause: rule.cause.clone(),
        remedy: rule.remedy.clone(),
        session_type: session_type.map(|s| s.to_string()),
    }
}

// ─── Category 1: OBIS Existence ─────────────────────────────────────────────

fn check_obis_existence(rule: &Rule, session: &SessionLog) -> Vec<ComplianceIssue> {
    if rule.check != "must_exist" || rule.obis_codes.is_empty() {
        return vec![];
    }

    let present_codes: std::collections::HashSet<&str> = session.obis_lines.iter()
        .map(|l| l.code.as_str())
        .collect();

    let missing: Vec<&str> = rule.obis_codes.iter()
        .map(|s| s.as_str())
        .filter(|code| !present_codes.contains(code))
        .collect();

    if missing.is_empty() {
        return vec![];
    }

    vec![make_issue(
        rule,
        format!("Tüm OBIS kodları mevcut olmalı: {}", rule.obis_codes.join(", ")),
        format!("Eksik kodlar: {}", missing.join(", ")),
        Some(session.session_type.as_str()),
    )]
}

// ─── Category 2: OBIS Format ────────────────────────────────────────────────

fn check_obis_format(rule: &Rule, session: &SessionLog) -> Vec<ComplianceIssue> {
    if rule.check != "value_format" {
        return vec![];
    }

    let obis_code = match rule.obis_code.as_deref() {
        Some(c) => c,
        None => return vec![],
    };

    let pattern = match rule.pattern.as_deref() {
        Some(p) => p,
        None => return vec![],
    };

    let line = match find_obis(session, obis_code) {
        Some(l) => l,
        None => return vec![], // Code not present — handled by obis_existence rules
    };

    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Invalid regex in rule {}: {} — {}", rule.code, pattern, e);
            return vec![];
        }
    };

    if !re.is_match(&line.raw_value) {
        vec![make_issue(
            rule,
            format!("{}({}) formatı: {}", obis_code, pattern, pattern),
            format!("{}({})", obis_code, line.raw_value),
            Some(session.session_type.as_str()),
        )]
    } else {
        vec![]
    }
}

// ─── Category 3: OBIS Value ─────────────────────────────────────────────────

fn check_obis_value(rule: &Rule, session: &SessionLog) -> Vec<ComplianceIssue> {
    let obis_code = match rule.obis_code.as_deref() {
        Some(c) => c,
        None => return vec![],
    };

    let st = Some(session.session_type.as_str());

    match rule.check.as_str() {
        "range" => {
            let val = match obis_numeric(session, obis_code) {
                Some(v) => v,
                None => return vec![],
            };
            let min = rule.min.unwrap_or(f64::NEG_INFINITY);
            let max = rule.max.unwrap_or(f64::INFINITY);
            if val < min || val > max {
                let expected = match (rule.min, rule.max) {
                    (Some(mn), Some(mx)) => format!("{mn}–{mx}"),
                    (Some(mn), None)     => format!("≥{mn}"),
                    (None, Some(mx))     => format!("≤{mx}"),
                    (None, None)         => "herhangi".to_string(),
                };
                vec![make_issue(rule, expected, format!("{:.3}", val), st)]
            } else {
                vec![]
            }
        }
        "equals" => {
            let line = match find_obis(session, obis_code) {
                Some(l) => l,
                None => return vec![],
            };
            let expected_val = rule.value.as_deref().unwrap_or("");
            if line.value.trim() != expected_val {
                vec![make_issue(rule, expected_val.to_string(), line.value.clone(), st)]
            } else {
                vec![]
            }
        }
        "not_equals" => {
            let line = match find_obis(session, obis_code) {
                Some(l) => l,
                None => return vec![],
            };
            let forbidden = rule.value.as_deref().unwrap_or("");
            if line.value.trim() == forbidden {
                vec![make_issue(rule, format!("≠ {forbidden}"), line.value.clone(), st)]
            } else {
                vec![]
            }
        }
        "not_empty" => {
            let line = match find_obis(session, obis_code) {
                Some(l) => l,
                None => {
                    // Code not found = effectively empty
                    return vec![make_issue(rule, "Boş olmayan değer".to_string(), "(bulunamadı)".to_string(), st)];
                }
            };
            if line.value.trim().is_empty() {
                vec![make_issue(rule, "Boş olmayan değer".to_string(), "(boş)".to_string(), st)]
            } else {
                vec![]
            }
        }
        "bit_zero" | "bit_one" => {
            check_bit(rule, session, rule.check == "bit_one")
        }
        "regex_match" => {
            let line = match find_obis(session, obis_code) {
                Some(l) => l,
                None => return vec![],
            };
            let pattern = match rule.pattern.as_deref() {
                Some(p) => p,
                None => return vec![],
            };
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(&line.value) {
                        vec![make_issue(rule, format!("Desen: {pattern}"), line.value.clone(), st)]
                    } else {
                        vec![]
                    }
                }
                Err(_) => vec![],
            }
        }
        _ => {
            log::warn!("Unknown obis_value check: {} (rule {})", rule.check, rule.code);
            vec![]
        }
    }
}

fn check_bit(rule: &Rule, session: &SessionLog, expect_one: bool) -> Vec<ComplianceIssue> {
    let obis_code = match rule.obis_code.as_deref() {
        Some(c) => c,
        None => return vec![],
    };

    let line = match find_obis(session, obis_code) {
        Some(l) => l,
        None => return vec![],
    };

    let bit_index = rule.bit.unwrap_or(0) as u64;
    let code_str = &line.value;

    let code_value: u64 = if code_str.chars().all(|c| c == '0' || c == '1') && code_str.len() > 8 {
        u64::from_str_radix(&code_str.chars().take(64).collect::<String>(), 2).unwrap_or(0)
    } else {
        let trimmed = code_str.trim_start_matches("0x").trim_start_matches("0X");
        if trimmed.is_empty() { 0 } else { u64::from_str_radix(trimmed, 16).unwrap_or(0) }
    };

    let bit_is_set = (code_value >> bit_index) & 1 == 1;
    let st = Some(session.session_type.as_str());

    if expect_one && !bit_is_set {
        vec![make_issue(rule, format!("Bit {bit_index} = 1"), format!("Bit {bit_index} = 0"), st)]
    } else if !expect_one && bit_is_set {
        vec![make_issue(rule, format!("Bit {bit_index} = 0"), format!("Bit {bit_index} = 1"), st)]
    } else {
        vec![]
    }
}

// ─── Category 4: Cross-Value ────────────────────────────────────────────────

fn check_cross_value(rule: &Rule, session: &SessionLog) -> Vec<ComplianceIssue> {
    let st = Some(session.session_type.as_str());

    match rule.check.as_str() {
        "tariff_balance" => {
            let total_code = rule.obis_total.as_deref().unwrap_or("1.8.0");
            let total = match obis_numeric(session, total_code) {
                Some(v) => v,
                None => return vec![],
            };

            let parts_codes = if rule.obis_parts.is_empty() {
                vec!["1.8.1", "1.8.2", "1.8.3", "1.8.4"]
            } else {
                rule.obis_parts.iter().map(|s| s.as_str()).collect()
            };

            let sum: f64 = parts_codes.iter()
                .filter_map(|code| obis_numeric(session, code))
                .sum();

            if total == 0.0 && sum == 0.0 {
                return vec![];
            }

            let tolerance = rule.tolerance.unwrap_or(0.01);
            let diff = (total - sum).abs();

            if diff > tolerance {
                vec![make_issue(
                    rule,
                    format!("Fark ≤ {tolerance} kWh"),
                    format!("T={total:.3}, Σ={sum:.3}, Δ={diff:.3} kWh"),
                    st,
                )]
            } else {
                vec![]
            }
        }
        "time_drift_minutes" => {
            let date_code = rule.obis_date.as_deref().unwrap_or("0.9.2");
            let time_code = rule.obis_time.as_deref().unwrap_or("0.9.1");

            let date_val = match find_obis(session, date_code) {
                Some(l) => l.value.trim().to_string(),
                None => return vec![],
            };
            let time_val = match find_obis(session, time_code) {
                Some(l) => l.value.trim().to_string(),
                None => return vec![],
            };

            if date_val.is_empty() || time_val.is_empty() {
                return vec![];
            }

            let meter_dt_str = format!("{} {}", date_val, time_val);
            let meter_naive = chrono::NaiveDateTime::parse_from_str(&meter_dt_str, "%y-%m-%d %H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&meter_dt_str, "%d.%m.%Y %H:%M:%S"))
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&meter_dt_str, "%d.%m.%Y %H:%M"));

            let meter_naive = match meter_naive {
                Ok(dt) => dt,
                Err(_) => return vec![],
            };

            let now = chrono::Local::now().naive_local();
            let drift_secs = (now - meter_naive).num_seconds().abs();
            let drift_minutes = drift_secs / 60;
            let max_drift = rule.max_drift.unwrap_or(5);

            if drift_minutes > max_drift {
                vec![make_issue(rule, format!("≤ {max_drift} dakika"), format!("{drift_minutes} dakika"), st)]
            } else {
                vec![]
            }
        }
        _ => {
            log::warn!("Unknown cross_value check: {} (rule {})", rule.check, rule.code);
            vec![]
        }
    }
}

// ─── Category 5: Protocol ───────────────────────────────────────────────────

fn check_protocol(rule: &Rule, session: &SessionLog, log: &CommunicationLog) -> Vec<ComplianceIssue> {
    let st = Some(session.session_type.as_str());

    match rule.check.as_str() {
        "handshake_complete" => {
            if !session.handshake.request_sent || !session.handshake.identification_received {
                vec![make_issue(rule, "El sıkışma tamamlanmalı".to_string(),
                    "El sıkışma tamamlanamadı".to_string(), st)]
            } else {
                vec![]
            }
        }
        "identification_format" => {
            let pattern = rule.pattern.as_deref().unwrap_or(r"^/[A-Z]{3}\d");
            let raw = &session.handshake.identification_raw;
            if raw.is_empty() {
                return vec![];
            }
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(raw) {
                        vec![make_issue(rule, format!("Format: {pattern}"), raw.clone(), st)]
                    } else {
                        vec![]
                    }
                }
                Err(_) => vec![],
            }
        }
        "baud_negotiation" => {
            if !session.handshake.baud_negotiation_success {
                vec![make_issue(rule, "Baud değişimi başarılı".to_string(),
                    "Baud değişimi başarısız".to_string(), st)]
            } else {
                vec![]
            }
        }
        "bcc_valid" => {
            match session.bcc_valid {
                Some(false) => {
                    vec![make_issue(rule, "BCC geçerli".to_string(), "BCC geçersiz".to_string(), st)]
                }
                _ => vec![],
            }
        }
        "response_time" => {
            let max_ms = rule.max_ms.unwrap_or(1500);
            let actual = session.handshake.response_time_ms;
            if actual > max_ms {
                vec![make_issue(rule, format!("≤ {max_ms} ms"), format!("{actual} ms"), st)]
            } else {
                vec![]
            }
        }
        "etx_present" => {
            if !session.etx_found {
                vec![make_issue(rule, "ETX mevcut".to_string(), "ETX bulunamadı".to_string(), st)]
            } else {
                vec![]
            }
        }
        "mode_supported" => {
            if rule.modes.is_empty() {
                return vec![];
            }
            // Collect all modes used across all sessions
            let all_modes: std::collections::HashSet<&str> = log.sessions.iter()
                .flat_map(|s| s.modes_used.iter().map(|m| m.as_str()))
                .collect();

            let unsupported: Vec<&str> = rule.modes.iter()
                .map(|m| m.as_str())
                .filter(|m| !all_modes.contains(m))
                .collect();

            if !unsupported.is_empty() {
                vec![make_issue(
                    rule,
                    format!("Desteklenen modlar: {}", rule.modes.join(", ")),
                    format!("Desteklenmeyen: {}", unsupported.join(", ")),
                    st,
                )]
            } else {
                vec![]
            }
        }
        _ => {
            log::warn!("Unknown protocol check: {} (rule {})", rule.check, rule.code);
            vec![]
        }
    }
}

// ─── Category 6: Session ────────────────────────────────────────────────────

fn check_session(rule: &Rule, session: &SessionLog, log: &CommunicationLog) -> Vec<ComplianceIssue> {
    match rule.check.as_str() {
        "must_succeed" => {
            let target_type = rule.session_type.as_deref().unwrap_or("any");
            // Find the matching session in the log
            let matching = log.sessions.iter()
                .find(|s| s.session_type.as_str() == target_type);

            match matching {
                None => {
                    vec![make_issue(
                        rule,
                        format!("{} oturumu başarılı olmalı", target_type),
                        "Oturum çalıştırılmadı".to_string(),
                        Some(target_type),
                    )]
                }
                Some(s) if !s.success => {
                    let err = s.error.as_deref().unwrap_or("Bilinmeyen hata");
                    vec![make_issue(
                        rule,
                        format!("{} oturumu başarılı olmalı", target_type),
                        format!("Başarısız: {}", err),
                        Some(target_type),
                    )]
                }
                _ => vec![],
            }
        }
        "data_received" => {
            let min_lines = rule.min_lines.unwrap_or(1);
            let actual = session.obis_lines.len();
            if actual < min_lines {
                vec![make_issue(
                    rule,
                    format!("≥ {min_lines} satır"),
                    format!("{actual} satır"),
                    Some(session.session_type.as_str()),
                )]
            } else {
                vec![]
            }
        }
        _ => {
            log::warn!("Unknown session check: {} (rule {})", rule.check, rule.code);
            vec![]
        }
    }
}

// ─── Category 7: Load Profile ───────────────────────────────────────────────

fn check_load_profile(rule: &Rule, session: &SessionLog) -> Vec<ComplianceIssue> {
    let st = Some(session.session_type.as_str());

    match rule.check.as_str() {
        "period_equals" => {
            let obis_code = rule.obis_code.as_deref().unwrap_or("0.8.0");
            let expected = rule.expected_minutes.unwrap_or(15);
            match find_obis(session, obis_code) {
                Some(line) => {
                    let val_str = line.value.trim();
                    let val: u32 = val_str.parse().unwrap_or(0);
                    if val != expected {
                        vec![make_issue(rule, format!("{expected} dakika"), format!("{val} dakika"), st)]
                    } else {
                        vec![]
                    }
                }
                None => vec![],
            }
        }
        "channels_exist" => {
            check_obis_existence(rule, session) // Reuse existence check logic
        }
        "continuity" => {
            // Check for gaps in load profile data (simplified check)
            // Look at P.01 style data blocks: 96.7.10*N
            let max_gap = rule.max_gap.unwrap_or(4) as usize;
            let mut consecutive_empty = 0;
            let mut max_found = 0;

            for line in &session.obis_lines {
                if line.code.starts_with("96.7.10") {
                    if line.value.trim().is_empty() || line.value.trim() == "0" {
                        consecutive_empty += 1;
                        if consecutive_empty > max_found {
                            max_found = consecutive_empty;
                        }
                    } else {
                        consecutive_empty = 0;
                    }
                }
            }

            if max_found > max_gap {
                vec![make_issue(
                    rule,
                    format!("Ardışık boş ≤ {max_gap}"),
                    format!("Ardışık {max_found} boş periyot"),
                    st,
                )]
            } else {
                vec![]
            }
        }
        _ => {
            log::warn!("Unknown load_profile check: {} (rule {})", rule.check, rule.code);
            vec![]
        }
    }
}

// ─── Category 8: Full Read Structure ────────────────────────────────────────

fn check_full_read(rule: &Rule, session: &SessionLog) -> Vec<ComplianceIssue> {
    let st = Some(session.session_type.as_str());

    match rule.check.as_str() {
        "history_series" => {
            let pattern = match rule.obis_pattern.as_deref() {
                Some(p) => p,
                None => return vec![],
            };
            let min_count = rule.min_count.unwrap_or(12);

            // Pattern like "1.8.0*{n}" — expand {n} to 1..count and check presence
            let present_codes: std::collections::HashSet<&str> = session.obis_lines.iter()
                .map(|l| l.code.as_str())
                .collect();

            let mut found = 0;
            for n in 1..=min_count {
                let code = pattern.replace("{n}", &n.to_string());
                if present_codes.contains(code.as_str()) {
                    found += 1;
                }
            }

            if found < min_count {
                vec![make_issue(
                    rule,
                    format!("≥ {min_count} kayıt ({pattern})"),
                    format!("{found} kayıt bulundu"),
                    st,
                )]
            } else {
                vec![]
            }
        }
        _ => {
            log::warn!("Unknown full_read check: {} (rule {})", rule.check, rule.code);
            vec![]
        }
    }
}
