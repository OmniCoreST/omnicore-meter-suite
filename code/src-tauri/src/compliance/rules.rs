//! Uyumluluk kural dosyası yükleme ve yönetimi

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Tek bir kural tanımı
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rule {
    pub code: String,
    pub field: String,
    pub check: String,
    pub severity: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_drift: Option<i64>,
    /// Kaç fazlı sayaçlara uygulanır (1 veya 3). Belirtilmezse her sayaçta çalışır.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<u8>,
    /// Şartname referansı — ör. "TEDAŞ Şartname 2.2.2 / MASS Şartname 2.2.2"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
}

/// TOML kural dosyasının kök yapısı
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RulesFile {
    pub rules_version: String,
    /// Versiyon kontrol URL'si — boşsa otomatik güncelleme devre dışı
    pub update_url: Option<String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// Kural dosyasının disk konumu
///
/// Üretim: exe'nin yanındaki `compliance_rules.toml`
/// Geliştirme: `src-tauri/compliance_rules.toml` (CARGO_MANIFEST_DIR)
pub fn get_rules_path() -> PathBuf {
    // Üretim: exe'nin yanındaki dosyayı kullan
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("compliance_rules.toml");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    // Geliştirme: kaynak klasöründeki dosyayı kullan
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("compliance_rules.toml")
}

/// Kural dosyasını diske kaydeder (mevcut dosyanın üzerine yazar)
pub fn save_rules(rules: &RulesFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = get_rules_path();
    let content = toml::to_string_pretty(rules)
        .map_err(|e| format!("Kurallar seri hale getirilemedi: {}", e))?;
    std::fs::write(&path, content.as_bytes())
        .map_err(|e| format!("Kural dosyası yazılamadı: {}", e))?;
    Ok(())
}

/// Kural dosyasını TOML olarak yükler
pub fn load_rules() -> Result<RulesFile, Box<dyn std::error::Error + Send + Sync>> {
    let path = get_rules_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Kural dosyası okunamadı ({}): {}", path.display(), e))?;
    let rules: RulesFile = toml::from_str(&content)
        .map_err(|e| format!("Kural dosyası ayrıştırılamadı: {}", e))?;
    Ok(rules)
}

// Kural dosyası artık src-tauri/compliance_rules.toml olarak kaynak kodda
// yaşıyor ve kurulumda exe'nin yanına paketleniyor.

// Derleme zamanı kontrolü: dosya yoksa hata ver
const _: &str = include_str!("../../compliance_rules.toml");
