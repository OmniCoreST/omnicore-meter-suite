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
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub value: Option<String>,
    pub bit: Option<u32>,
    pub tolerance: Option<f64>,
    pub max_drift: Option<i64>,
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
/// Windows: `%APPDATA%\omnicore\compliance_rules.toml`
/// Linux:   `~/.local/share/omnicore/compliance_rules.toml`
pub fn get_rules_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("omnicore");
    path.push("compliance_rules.toml");
    path
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

/// Kural dosyası yoksa varsayılan içeriği yazar
pub fn ensure_default_rules() {
    let path = get_rules_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&path, DEFAULT_RULES) {
            log::warn!("Varsayılan kural dosyası yazılamadı: {}", e);
        } else {
            log::info!("Varsayılan uyumluluk kural dosyası oluşturuldu: {}", path.display());
        }
    }
}

/// Varsayılan TOML kural içeriği (binary'e gömülü)
pub const DEFAULT_RULES: &str = r#"# compliance_rules.toml
# =====================
# TEDAŞ MLZ/2017-062.B Uyumluluk Kuralları
#
# Bu dosyayı düzenleyerek uyumluluk kurallarını güncelleyebilirsiniz.
# Programlama bilgisi gerekmez — mevcut bir [[rules]] bloğunu kopyalayıp değiştirin.
#
# Desteklenen kural tipleri (check değerleri):
#   range              → Sayısal aralık (min, max)
#   equals             → Eşitlik (value)
#   not_equals         → Eşit olmamalı (value)
#   not_empty          → Boş olmamalı
#   bit_zero           → ff_code/gf_code'da bit 0 olmalı (bit)
#   bit_one            → Bit 1 olmalı (bit)
#   tariff_balance     → T1+T2+T3+T4 = toplam (tolerance, kWh)
#   time_drift_minutes → Sayaç/sistem saat farkı (max_drift, dakika)
#
# severity: "error" | "warning" | "info"
# update_url: GitHub raw URL — boş bırakılırsa otomatik güncelleme devre dışı

rules_version = "1.0.0"
update_url = ""

# ── Gerilim ───────────────────────────────────────────────────────────────────

[[rules]]
code = "EL-001"
field = "voltage_l1"
check = "range"
min = 90.0
max = 265.0
severity = "error"
description = "L1 gerilimi çalışma aralığı dışında (90–265V)"

[[rules]]
code = "EL-002"
field = "voltage_l2"
check = "range"
min = 90.0
max = 265.0
severity = "error"
description = "L2 gerilimi çalışma aralığı dışında (90–265V)"

[[rules]]
code = "EL-003"
field = "voltage_l3"
check = "range"
min = 90.0
max = 265.0
severity = "error"
description = "L3 gerilimi çalışma aralığı dışında (90–265V)"

# ── Frekans ───────────────────────────────────────────────────────────────────

[[rules]]
code = "EL-004"
field = "frequency"
check = "range"
min = 49.0
max = 51.0
severity = "warning"
description = "Şebeke frekansı ±%2 tolerans dışında (49–51 Hz)"

# ── Kimlik ────────────────────────────────────────────────────────────────────

[[rules]]
code = "ID-001"
field = "serial_number"
check = "not_empty"
severity = "error"
description = "Seri numarası boş"

# ── Pil ───────────────────────────────────────────────────────────────────────

[[rules]]
code = "BAT-001"
field = "battery_status"
check = "not_equals"
value = "low"
severity = "warning"
description = "Pil seviyesi düşük"

# ── Demant Periyodu ───────────────────────────────────────────────────────────

[[rules]]
code = "DEM-001"
field = "demand_period"
check = "equals"
value = "15*min"
severity = "warning"
description = "Demant periyodu 15 dakika olmalı (TEDAŞ standardı)"

# ── FF Durum Kodu ─────────────────────────────────────────────────────────────

[[rules]]
code = "FF-001"
field = "ff_code"
check = "bit_zero"
bit = 0
severity = "error"
description = "RTC (saat modülü) arızası tespit edildi (FF Bit 0)"

[[rules]]
code = "FF-002"
field = "ff_code"
check = "bit_zero"
bit = 5
severity = "warning"
description = "Klemens kapağı açık veya açılma geçmişi var (FF Bit 5)"

[[rules]]
code = "FF-003"
field = "ff_code"
check = "bit_zero"
bit = 6
severity = "warning"
description = "Üst kapak açık veya açılma geçmişi var (FF Bit 6)"

[[rules]]
code = "FF-004"
field = "ff_code"
check = "bit_zero"
bit = 11
severity = "error"
description = "R fazında manyetik müdahale tespit edildi (FF Bit 11)"

[[rules]]
code = "FF-005"
field = "ff_code"
check = "bit_zero"
bit = 12
severity = "error"
description = "S fazında manyetik müdahale tespit edildi (FF Bit 12)"

[[rules]]
code = "FF-006"
field = "ff_code"
check = "bit_zero"
bit = 13
severity = "error"
description = "T fazında manyetik müdahale tespit edildi (FF Bit 13)"

# ── Tarife Dengesi ────────────────────────────────────────────────────────────

[[rules]]
code = "TRF-001"
field = "tariff_sum"
check = "tariff_balance"
tolerance = 0.01
severity = "warning"
description = "T1+T2+T3+T4 toplamı genel toplama eşit değil"

# ── Saat Sapması ──────────────────────────────────────────────────────────────

[[rules]]
code = "TIME-001"
field = "time_drift"
check = "time_drift_minutes"
max_drift = 5
severity = "warning"
description = "Sayaç saati sistem saatinden 5 dakikadan fazla sapıyor"
"#;
