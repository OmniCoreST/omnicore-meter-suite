//! TEDAŞ MLZ/2017-062.B Uyumluluk Kontrolcüsü

pub mod rules;
pub mod engine;
pub mod updater;

use serde::{Deserialize, Serialize};
use crate::commands::types::ShortReadResult;

/// Tek bir uyumluluk ihlali
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceIssue {
    pub code: String,
    pub severity: String, // "error" | "warning" | "info"
    pub field: String,
    pub expected: String,
    pub actual: String,
    pub description: String,
    /// Şartname referansı — ör. "TEDAŞ §2.2.2 / MASS §2.2.2"
    pub spec_ref: Option<String>,
    /// Sorunun olası nedeni
    pub cause: Option<String>,
    /// Önerilen düzeltme adımı
    pub remedy: Option<String>,
}

/// Tüm uyumluluk kontrolü sonucu
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceResult {
    pub issues: Vec<ComplianceIssue>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub rules_version: String,
    /// Sunucudaki güncel versiyon (bağlanılabildiyse)
    pub latest_version: Option<String>,
    /// Kurallar güncel mi? (major fark hesabına göre)
    pub rules_status: RulesStatus,
    pub checked_at: String,
    pub rules_file_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RulesStatus {
    /// Güncel veya 1 major geride — çalışır, uyarı gösterilir
    Ok,
    /// Sunucuya ulaşılamadı — mevcut kurallarla devam
    Offline,
    /// 2+ major geride — uyumluluk kontrolü kilitlenir
    TooOld,
}

/// Ana giriş noktası
pub fn run_check(data: &ShortReadResult, latest_version: Option<String>, meter_phases: u8) -> ComplianceResult {
    let rules_file_path = rules::get_rules_path().display().to_string();

    let rules_status = compute_rules_status(
        &rules::load_rules().map(|r| r.rules_version.clone()).unwrap_or_default(),
        latest_version.as_deref(),
    );

    // Kurallar çok eskiyse kontrol yapma
    if rules_status == RulesStatus::TooOld {
        return ComplianceResult {
            issues: vec![],
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            rules_version: rules::load_rules().map(|r| r.rules_version).unwrap_or_default(),
            latest_version,
            rules_status,
            checked_at: chrono::Local::now().to_rfc3339(),
            rules_file_path,
        };
    }

    match rules::load_rules() {
        Ok(rules_file) => {
            let issues = engine::evaluate(&rules_file, data, meter_phases);
            let error_count = issues.iter().filter(|i| i.severity == "error").count();
            let warning_count = issues.iter().filter(|i| i.severity == "warning").count();
            let info_count = issues.iter().filter(|i| i.severity == "info").count();
            let rules_version = rules_file.rules_version.clone();

            ComplianceResult {
                issues,
                error_count,
                warning_count,
                info_count,
                rules_version,
                latest_version,
                rules_status,
                checked_at: chrono::Local::now().to_rfc3339(),
                rules_file_path,
            }
        }
        Err(e) => {
            log::error!("Kural dosyası yüklenemedi: {}", e);
            ComplianceResult {
                issues: vec![ComplianceIssue {
                    code: "SYS-001".to_string(),
                    severity: "error".to_string(),
                    field: "rules_file".to_string(),
                    expected: "Geçerli kural dosyası".to_string(),
                    actual: e.to_string(),
                    description: format!("Kural dosyası yüklenemedi: {}", e),
                    spec_ref: None,
                    cause: None,
                    remedy: None,
                }],
                error_count: 1,
                warning_count: 0,
                info_count: 0,
                rules_version: "unknown".to_string(),
                latest_version,
                rules_status,
                checked_at: chrono::Local::now().to_rfc3339(),
                rules_file_path,
            }
        }
    }
}

/// Yerel versiyon ile sunucu versiyonunu karşılaştırır
/// Major fark 0-1 → Ok, 2+ → TooOld, sunucuya ulaşılamadı → Offline
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
    version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
