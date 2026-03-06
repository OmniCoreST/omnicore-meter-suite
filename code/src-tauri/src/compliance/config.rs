//! Compliance configuration file parsing and management
//!
//! Handles loading/saving of the v3 TOML config with profiles, test plan, and rules.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Meter Profile ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub phases: u8,
    pub connection: String,
    #[serde(default)]
    pub description: String,
}

// ─── Test Plan ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestPlan {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub steps: Vec<TestStep>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestStep {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub mode: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    #[serde(default)]
    pub retry_count: u32,
    /// For obis_read mode: which codes to read
    #[serde(default)]
    pub obis_codes: Vec<String>,
    /// For packet_read mode: which packet mode (7, 8, 9)
    #[serde(default)]
    pub packet_mode: Option<u32>,
}

fn default_true() -> bool { true }
fn default_timeout() -> u32 { 60 }

// ─── Rule ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rule {
    pub code: String,
    pub category: String,
    pub check: String,
    pub severity: String,
    pub description: String,

    // ─── Common optional fields ───
    #[serde(default)]
    pub spec_ref: Option<String>,
    #[serde(default)]
    pub cause: Option<String>,
    #[serde(default)]
    pub remedy: Option<String>,
    /// Profile IDs this rule applies to. Empty = all profiles.
    #[serde(default)]
    pub profile: Vec<String>,
    /// Session type this rule applies to: "short_read", "full_read", "load_profile", "any"
    #[serde(default)]
    pub session_type: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,

    // ─── obis_existence fields ───
    #[serde(default)]
    pub obis_codes: Vec<String>,

    // ─── obis_format fields ───
    #[serde(default)]
    pub obis_code: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,

    // ─── obis_value fields ───
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub bit: Option<u32>,
    #[serde(default = "default_unit_strip")]
    pub unit_strip: bool,

    // ─── cross_value fields ───
    #[serde(default)]
    pub obis_total: Option<String>,
    #[serde(default)]
    pub obis_parts: Vec<String>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub obis_date: Option<String>,
    #[serde(default)]
    pub obis_time: Option<String>,
    #[serde(default)]
    pub max_drift: Option<i64>,

    // ─── protocol fields ───
    #[serde(default)]
    pub max_ms: Option<u64>,
    #[serde(default)]
    pub modes: Vec<String>,

    // ─── session fields ───
    #[serde(default)]
    pub min_lines: Option<usize>,

    // ─── load_profile fields ───
    #[serde(default)]
    pub expected_minutes: Option<u32>,
    #[serde(default)]
    pub max_gap: Option<u32>,

    // ─── full_read fields ───
    #[serde(default)]
    pub obis_pattern: Option<String>,
    #[serde(default)]
    pub min_count: Option<usize>,

    // ─── Legacy v2 compat (auto-migrated) ───
    /// v2 field name, mapped to obis_code during migration
    #[serde(default)]
    pub field: Option<String>,
    /// v2 phases filter, mapped to profile during migration
    #[serde(default)]
    pub phases: Option<u8>,
}

fn default_unit_strip() -> bool { true }

// ─── Config File Root ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComplianceConfig {
    pub config_version: String,
    #[serde(default)]
    pub update_url: Option<String>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub test_plan: Option<TestPlan>,
    #[serde(default)]
    pub rules: Vec<Rule>,

    // ─── Legacy v2 fields (for auto-migration) ───
    #[serde(default)]
    pub rules_version: Option<String>,
}

// ─── File Operations ────────────────────────────────────────────────────────

/// Get the config file path.
/// Production: next to the exe. Development: CARGO_MANIFEST_DIR.
pub fn get_config_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("compliance_rules.toml");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("compliance_rules.toml")
}

/// Load config from TOML file, auto-migrating v2 format if detected.
pub fn load_config() -> Result<ComplianceConfig, Box<dyn std::error::Error + Send + Sync>> {
    let path = get_config_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Config dosyası okunamadı ({}): {}", path.display(), e))?;

    // Try v3 format first
    let mut config: ComplianceConfig = toml::from_str(&content)
        .map_err(|e| format!("Config dosyası ayrıştırılamadı: {}", e))?;

    // Auto-migrate v2: if config_version is missing but rules_version exists
    if config.config_version.is_empty() {
        if let Some(ref rv) = config.rules_version {
            config.config_version = rv.clone();
        }
    }

    // Migrate v2 rules that have `field` but no `category`
    for rule in &mut config.rules {
        migrate_v2_rule(rule);
    }

    // If no profiles defined, add defaults
    if config.profiles.is_empty() {
        config.profiles = default_profiles();
    }

    Ok(config)
}

/// Save config back to TOML
pub fn save_config(config: &ComplianceConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = get_config_path();
    let content = toml::to_string_pretty(config)
        .map_err(|e| format!("Config seri hale getirilemedi: {}", e))?;
    std::fs::write(&path, content.as_bytes())
        .map_err(|e| format!("Config dosyası yazılamadı: {}", e))?;
    Ok(())
}

/// Find a profile by ID
pub fn find_profile<'a>(config: &'a ComplianceConfig, profile_id: &str) -> Option<&'a Profile> {
    config.profiles.iter().find(|p| p.id == profile_id)
}

/// Get rules applicable to a given profile
pub fn rules_for_profile<'a>(config: &'a ComplianceConfig, profile_id: &str) -> Vec<&'a Rule> {
    config.rules.iter()
        .filter(|r| {
            if !r.enabled { return false; }
            if r.profile.is_empty() { return true; }
            r.profile.iter().any(|p| p == profile_id)
        })
        .collect()
}

// ─── V2 Migration ───────────────────────────────────────────────────────────

/// Map v2 field names to OBIS codes
fn field_to_obis(field: &str) -> Option<&'static str> {
    match field {
        "serial_number" => Some("0.0.0"),
        "program_version" => Some("0.2.0"),
        "production_date" => Some("0.1.0"),
        "calibration_date" => Some("96.2.5"),
        "meter_date" => Some("0.9.2"),
        "meter_time" => Some("0.9.1"),
        "voltage_l1" => Some("32.7.0"),
        "voltage_l2" => Some("52.7.0"),
        "voltage_l3" => Some("72.7.0"),
        "current_l1" => Some("31.7.0"),
        "current_l2" => Some("51.7.0"),
        "current_l3" => Some("71.7.0"),
        "frequency" => Some("14.7.0"),
        "power_factor_l1" => Some("33.7.0"),
        "power_factor_l2" => Some("53.7.0"),
        "power_factor_l3" => Some("73.7.0"),
        "active_energy_import_total" => Some("1.8.0"),
        "active_energy_import_t1" => Some("1.8.1"),
        "active_energy_import_t2" => Some("1.8.2"),
        "active_energy_import_t3" => Some("1.8.3"),
        "active_energy_import_t4" => Some("1.8.4"),
        "ff_code" => Some("F.F.0"),
        "gf_code" => Some("F.F.1"),
        "battery_status" => Some("96.5.0"),
        "relay_status" => Some("96.3.10"),
        "demand_period" => Some("0.8.0"),
        "lp_period" => Some("0.8.4"),
        _ => None,
    }
}

fn migrate_v2_rule(rule: &mut Rule) {
    // If category is already set, it's v3 — skip
    if !rule.category.is_empty() {
        return;
    }

    // Determine category from check type
    let check = rule.check.as_str();

    match check {
        "tariff_balance" => {
            rule.category = "cross_value".to_string();
            if rule.obis_total.is_none() {
                rule.obis_total = Some("1.8.0".to_string());
            }
            if rule.obis_parts.is_empty() {
                rule.obis_parts = vec![
                    "1.8.1".to_string(), "1.8.2".to_string(),
                    "1.8.3".to_string(), "1.8.4".to_string(),
                ];
            }
        }
        "time_drift_minutes" => {
            rule.category = "cross_value".to_string();
            if rule.obis_date.is_none() {
                rule.obis_date = Some("0.9.2".to_string());
            }
            if rule.obis_time.is_none() {
                rule.obis_time = Some("0.9.1".to_string());
            }
        }
        _ => {
            rule.category = "obis_value".to_string();
        }
    }

    // Map field to obis_code
    if rule.obis_code.is_none() {
        if let Some(ref field) = rule.field {
            if let Some(obis) = field_to_obis(field) {
                rule.obis_code = Some(obis.to_string());
            }
        }
    }

    // Map phases to profile
    if rule.profile.is_empty() {
        if let Some(phases) = rule.phases {
            match phases {
                1 => rule.profile = vec!["single_phase".to_string()],
                3 => rule.profile = vec!["three_phase_direct".to_string(), "three_phase_ct".to_string()],
                _ => {}
            }
        }
    }
}

fn default_profiles() -> Vec<Profile> {
    vec![
        Profile {
            id: "single_phase".to_string(),
            name: "Tek Fazlı Sayaç".to_string(),
            phases: 1,
            connection: "direct".to_string(),
            description: "Tek fazlı direkt bağlantı sayacı".to_string(),
        },
        Profile {
            id: "three_phase_direct".to_string(),
            name: "Üç Fazlı Direkt Bağlantı".to_string(),
            phases: 3,
            connection: "direct".to_string(),
            description: "Üç fazlı direkt bağlantı sayacı".to_string(),
        },
        Profile {
            id: "three_phase_ct".to_string(),
            name: "Üç Fazlı Akım Trafolu".to_string(),
            phases: 3,
            connection: "ct".to_string(),
            description: "Üç fazlı akım trafosu bağlantılı sayaç".to_string(),
        },
    ]
}

// Compile-time check: config file must exist
const _: &str = include_str!("../../compliance_rules.toml");
