# TEDAŞ Uyumluluk Denetim Sistemi (Compliance Engine)

**Modül:** `code/src-tauri/src/compliance/`
**Yapılandırma:** `code/src-tauri/compliance_rules.toml`
**Ön Yüz:** `code/src/lib/pages/Compliance.svelte`
**Versiyon:** v3.0.0

---

## 1. Amaç

OmniCore Meter Suite uygulamasında **TEDAŞ MLZ/2017-062.B** ve **MASS** şartnamelerine göre elektrik sayaçlarının uyumluluk denetimini otomatik olarak gerçekleştiren bir altyapı bulunmaktadır.

Bu sistem:

- Sayaçtan farklı modlarda veri okur (kısa, tam, yük profili, paket okumaları)
- Okunan verileri yapılandırılabilir kurallara göre değerlendirir
- Hataları, uyarıları ve bilgilendirmeleri kullanıcıya raporlar
- Kuralları harici bir TOML dosyasından yükler; güncelleme, ekleme, silme destekler

---

## 2. Mimari Genel Bakış

```
┌─────────────────────────────────────────────────────────┐
│                  Compliance.svelte                        │
│                 (Ön Yüz / Test Runner)                   │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │ Kısa     │  │ Tam      │  │ Yük      │  │ Paket   │ │
│  │ Okuma    │→ │ Okuma    │→ │ Profili  │→ │ Okuma   │ │
│  │ Mode 6   │  │ Mode 0   │  │ Mode 1   │  │ 7/8/9   │ │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │
│         │              │            │             │       │
│         └──────────────┴────────────┴─────────────┘      │
│                         │                                 │
│                    Tauri invoke                           │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                   Rust Backend                           │
│                                                          │
│  ┌────────────────┐    ┌──────────────────────────────┐ │
│  │ compliance/     │    │ commands/                     │ │
│  │   config.rs     │    │   mod.rs (read_short,         │ │
│  │   engine.rs     │    │          read_full,           │ │
│  │   types.rs      │    │          read_load_profile,   │ │
│  │   mod.rs        │    │          read_packet)         │ │
│  │   updater.rs    │    │                               │ │
│  └────────────────┘    └──────────────────────────────┘ │
│         │                                                │
│  ┌──────▼─────────────────────┐                         │
│  │ compliance_rules.toml       │                         │
│  │ (Kurallar + Profiller +     │                         │
│  │  Test Planı)                │                         │
│  └────────────────────────────┘                         │
└─────────────────────────────────────────────────────────┘
```

---

## 3. Test Planı ve Okuma Modları

Uyumluluk testi, TEDAŞ şartnamesinde tanımlı tüm okuma modlarını sırayla çalıştırır:

| Adım | Mod | Açıklama | TEDAŞ Referansı |
|------|-----|----------|-----------------|
| 1 | Mode 6 | Kısa Okuma Paketi — temel sayaç bilgileri | TEDAŞ §5.x |
| 2 | Mode 0 | Uzun Okuma (Tam Bilgiler) — tüm OBIS kodları | TEDAŞ §5.x |
| 3 | Mode 1 | Yük Profili Okuma (P.01 üzerinden) | TEDAŞ §5.x |
| 4 | Mode 7 | Geçmiş Bilgiler Paketi | TEDAŞ §5.x |
| 5 | Mode 8 | Uyarı Paketi | TEDAŞ §5.x |
| 6 | Mode 9 | Kesinti Kayıtları Paketi (Son 10 Kesinti) | TEDAŞ §5.x |
| 7 | OBIS | Saat Kontrolü (0.9.1 / 0.9.2 okuma) | TEDAŞ §2.1.x |

Her adım bağımsızdır; bir adım başarısız olursa diğerleri yine çalışır. Tüm okumalar tamamlandıktan sonra toplanan veriler uyumluluk motoruna gönderilir.

Test planı `compliance_rules.toml` dosyasındaki `[test_plan]` bölümünde tanımlanır ve adımlar `[[test_plan.steps]]` olarak listelenir. Her adım `enabled = false` yapılarak devre dışı bırakılabilir.

---

## 4. Sayaç Profilleri

Farklı sayaç tipleri farklı kural setlerine tabidir. Sistemde üç öntanımlı profil bulunur:

| Profil ID | Ad | Faz | Bağlantı | Açıklama |
|-----------|----|-----|----------|----------|
| `single_phase` | Tek Fazlı Sayaç | 1 | Direkt | Monofaze sayaçlar |
| `three_phase_direct` | Üç Fazlı Direkt | 3 | Direkt | Trifaze direkt bağlantı |
| `three_phase_ct` | Üç Fazlı Akım Trafolu | 3 | CT | Akım trafosu bağlantılı |

Kurallar `profile` alanı ile hangi profillerde çalışacağını belirler. `profile` boş bırakılırsa kural tüm profillerde geçerlidir.

---

## 5. Kural Kategorileri

Uyumluluk motoru 8 farklı kural kategorisini destekler:

### 5.1 `obis_existence` — OBIS Kod Varlığı
Belirtilen OBIS kodunun sayaç verisinde bulunup bulunmadığını kontrol eder.

**Örnek:** `1.8.0` (toplam aktif enerji) okumada mevcut olmalıdır.

### 5.2 `obis_format` — OBIS Değer Formatı
OBIS değerinin belirli bir formata (regex) uygunluğunu kontrol eder.

**Örnek:** Seri numarası `^\d{9,12}$` formatında olmalıdır.

### 5.3 `obis_value` — OBIS Değer Kontrolü
OBIS değerinin sayısal veya mantıksal olarak geçerliliğini kontrol eder.

Desteklenen kontroller:
- `range` — min/max aralık kontrolü
- `equals` / `not_equals` — eşitlik/eşitsizlik
- `not_empty` — boş olmamalı
- `bit_zero` / `bit_one` — bit düzeyi kontrol (FF/GF kodları için)
- `regex_match` — regex ile eşleşme

**Örnek:** L1 gerilimi 90–265 V aralığında olmalıdır.

### 5.4 `cross_value` — Çapraz Değer Kontrolü
Birden fazla OBIS değeri arasındaki ilişkiyi kontrol eder.

Desteklenen kontroller:
- `tariff_balance` — T1+T2+T3+T4 ≈ Toplam enerji (tolerans dahilinde)
- `time_drift_minutes` — Sayaç saati ile sistem saati arasındaki fark

### 5.5 `protocol` — Protokol Uyumluluğu
IEC 62056-21 protokol davranışlarını kontrol eder.

Desteklenen kontroller:
- `handshake_complete` — El sıkışma tamamlandı mı
- `identification_format` — Kimlik yanıtı format kontrolü
- `baud_negotiation` — Baud hızı müzakeresi başarılı mı
- `bcc_valid` — BCC (Block Check Character) doğrulama
- `response_time` — Yanıt süresi kontrolü
- `etx_present` — ETX (End of Text) karakteri mevcut mu
- `mode_supported` — Belirtilen mod destekleniyor mu

### 5.6 `session` — Oturum Kontrolü
Okuma/yazma oturumlarının genel başarısını kontrol eder.

### 5.7 `load_profile` — Yük Profili Kontrolü
Yük profili verisinin yapısal bütünlüğünü kontrol eder.

### 5.8 `full_read` — Tam Okuma Kontrolü
Tam okuma verisinin yapısal bütünlüğünü kontrol eder (satır sayısı, ETX varlığı vb.).

---

## 6. TOML Yapılandırma Dosyası

`compliance_rules.toml` dosyası üç ana bölümden oluşur:

### 6.1 Genel Ayarlar
```toml
config_version = "3.0.0"
update_url = ""
```

### 6.2 Profiller
```toml
[[profiles]]
id = "single_phase"
name = "Tek Fazlı Sayaç"
phases = 1
connection = "direct"
description = "Tek fazlı direkt bağlantı sayacı"
```

### 6.3 Test Planı
```toml
[test_plan]
name = "TEDAŞ Tam Uyumluluk Testi"
description = "..."

[[test_plan.steps]]
id = "short_read"
name = "Kısa Okuma (Mode 6)"
mode = "short_read"
enabled = true
timeout_seconds = 30
retry_count = 1

[[test_plan.steps]]
id = "history_packet"
name = "Geçmiş Bilgiler Paketi (Mode 7)"
mode = "packet_read"
packet_mode = 7
enabled = true
timeout_seconds = 60
retry_count = 1
```

### 6.4 Kurallar
```toml
[[rules]]
code = "EL-001"
category = "obis_value"
obis_code = "32.7.0"
check = "range"
severity = "error"
min = 90.0
max = 265.0
description = "L1 fazı gerilimi 90–265 V aralığında olmalıdır"
spec_ref = "TEDAŞ §2.2.2 Çizelge 4"
cause = "Gerilim düşüklüğü veya aşırı gerilim"
remedy = "Şebeke gerilimini ve bağlantıları kontrol edin"
profile = ["three_phase_direct", "three_phase_ct"]
session_type = "full_read"
enabled = true
```

---

## 7. Rust Backend Modül Yapısı

```
src/compliance/
├── mod.rs        Giriş noktaları: run_check(), run_check_legacy()
├── config.rs     TOML dosya okuma, profiller, kurallar, v2→v3 göç
├── engine.rs     Kural değerlendirme motoru (8 kategori)
├── types.rs      Veri yapıları (ComplianceResult, ComplianceIssue, vb.)
└── updater.rs    Sunucudan kural güncellemesi indirme
```

### mod.rs
- `run_check(log, profile_id, latest_version)` — v3 API, CommunicationLog tabanlı
- `run_check_legacy(data, latest_version, phases)` — v2 uyumluluk, ShortReadResult tabanlı
- `communication_log_from_short_read()` — v2 verisini v3 formatına dönüştürür

### config.rs
- `load_config()` — TOML dosyasını okur, v2 kurallarını v3'e otomatik göç eder
- `rules_for_profile(rules, profile_id)` — Profile göre kuralları filtreler
- `migrate_v2_rule()` — Eski alan adlarını (ör. `voltage_l1`) OBIS kodlarına çevirir

### engine.rs
Her kural kategorisi için ayrı bir kontrol fonksiyonu:
- `check_obis_existence()` — OBIS kod varlığı
- `check_obis_format()` — Regex format kontrolü
- `check_obis_value()` — Değer aralığı, eşitlik, bit kontrolü
- `check_cross_value()` — Çapraz değer (tarife dengesi, saat sapması)
- `check_protocol()` — Protokol uyumluluğu
- `check_session()` — Oturum başarısı
- `check_load_profile()` — Yük profili yapısı
- `check_full_read()` — Tam okuma yapısı

---

## 8. Ön Yüz Yapısı

### Compliance.svelte — Ana Sayfa

Kullanıcı arayüzü üç ana bölümden oluşur:

1. **Başlık Çubuğu** — Kural dosyası yönetimi (aç, yenile, güncelle, kuralları yönet)
2. **Test Çalıştırıcı** — Profil seçimi ve otomatik test çalıştırma
   - Tüm okuma adımlarını sırayla çalıştırır
   - Her adımın durumunu (bekliyor/çalışıyor/tamamlandı/başarısız) gösterir
   - İlerleme çubuğu ve süre bilgisi
   - Ayrıntılı test günlüğü (daraltılabilir)
3. **Sonuçlar** — Hata/uyarı/bilgi özeti ve detaylı ihlal listesi

### stores/compliance.ts — Durum Yönetimi
- `ComplianceResult` — Kontrol sonuçları
- `ComplianceProfile` — Sayaç profilleri
- `TestPlan` / `TestStep` — Test planı tanımları
- `CommunicationLog` — v3 iletişim günlüğü yapısı

### utils/tauri.ts — Tauri Komut Bağlantıları
- `checkCompliance()` — v2 legacy kontrol
- `checkComplianceV3()` — v3 log tabanlı kontrol
- `getComplianceProfiles()` — Profil listesi
- `getComplianceTestPlan()` — Test planı
- `listComplianceRules()` — Kural listesi
- `updateComplianceRule()` / `deleteComplianceRule()` — Kural CRUD
- `reloadComplianceRules()` — Disk'ten yeniden yükleme
- `updateComplianceRules()` — Sunucudan güncelleme

---

## 9. Kural Ekleme / Düzenleme

Kullanıcılar ön yüz üzerinden kural ekleyebilir veya düzenleyebilir:

- **"Kuralları Yönet"** butonu ile mevcut kurallar listelenir
- Her kural düzenlenebilir veya silinebilir
- **"Yeni Kural Ekle"** ile TOML dosyasına yeni kural eklenir
- Kural alanları: kod, alan, kontrol tipi, şiddet, faz, açıklama, şartname referansı

Kural dosyası ayrıca:
- Sunucudan otomatik güncellenebilir (`update_url` tanımlıysa)
- Bilgisayardan `.toml` dosyası içe aktarılabilir
- Harici editörde açılıp elle düzenlenebilir

---

## 10. v2 → v3 Geriye Uyumluluk

Eski (v2) kural dosyaları alan adı tabanlıydı (ör. `field = "voltage_l1"`). Yeni (v3) sistem OBIS kodu tabanlıdır (ör. `obis_code = "32.7.0"`).

Geçiş otomatiktir:
- `config.rs` içindeki `migrate_v2_rule()` fonksiyonu eski alan adlarını OBIS kodlarına çevirir
- 27 alan adı → OBIS kodu eşlemesi tanımlıdır
- Eski `check_compliance` Tauri komutu korunmuştur; `run_check_legacy()` çağırır
- `communication_log_from_short_read()` eski ShortReadResult'ı yeni CommunicationLog formatına dönüştürür
