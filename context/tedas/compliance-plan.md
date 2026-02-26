# TEDAŞ Uygunluk Kontrol Sistemi — Uygulama Planı

## Genel Bakış

Sayaca bağlandığında TEDAŞ MLZ/2017-062.B şartnamesindeki maddeleri otomatik olarak kontrol eden,
her madde için detaylı sonuç üreten bir uygunluk denetim sistemi.

---

## Çıktı Formatı

Her kontrol için şu yapıda sonuç üretilecek:

```
✅ / ❌ / ⚠️  Madde X.X.X — Başlık
   Beklenen : ...
   Bulunan  : ...
   Sonuç    : Uygun / Uygun Değil / Kontrol Edilemedi
```

**Durum ikonları:**
- ✅ `pass` — Şartnameye uygun
- ❌ `fail` — Şartnameye aykırı
- ⚠️ `warn` — Kontrol edilemedi veya kısmi uyum

---

## Kontrol Kategorileri ve Maddeleri

### 1. İletişim Protokolü

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 1.1 | Mad. 2.1.8–2.1.10 | Baud rate desteği (300 / 9600 / 19200) | Handshake testi |
| 1.2 | Mad. 2.1.9 | IEC 62056-21 Mode C protokol uyumu | `/?!` yanıtı |
| 1.3 | Mad. 4.2 (161) | Paket başlığında seri no + tarih + saat | `0.2.1`, `0.9.1`, `0.9.2` |

---

### 2. Zaman / Saat (RTC)

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 2.1 | Mad. 2.1.11 | RTC okunabilirliği | `0.9.1`, `0.9.2` |
| 2.2 | Mad. 2.1.11 | Saat sapması ±30 saniye içinde mi | `0.9.1` vs sistem saati |
| 2.3 | Mad. 2.1.11 | Tarih formatı doğru mu (YY-MM-DD) | `0.9.2` |
| 2.4 | Mad. 2.1.11 | Saat formatı doğru mu (hh:mm:ss) | `0.9.1` |

---

### 3. Yaz/Kış Saati (DST)

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 3.1 | Mad. 2.1.12 | DST özelliği etkin mi | `96.90.0` = 1 |
| 3.2 | Mad. 2.1.12 | En az 1 DST dönemi yapılandırılmış mı | `96.90.1` |
| 3.3 | Mad. 2.1.12 | 12 döneme kadar yapılandırma desteği | `96.90.1`–`96.90.12` |
| 3.4 | Mad. 2.1.12 | DST dönem formatı doğru mu | `+01:00,YY-MM-DD,hh:mm;...` |

---

### 4. Tarife Yapısı

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 4.1 | Mad. 2.2 (84) | En az 4 tarife dilimi (T1/T2/T3/T4) | `1.8.1`–`1.8.4` |
| 4.2 | Mad. 2.2 (84) | Günlük 8 zaman dilimi desteği | `96.50`–`96.52` |
| 4.3 | Mad. 2.2 (85) | Hafta içi / Cumartesi / Pazar ayrı tarife | `96.50`, `96.51`, `96.52` |
| 4.4 | Mad. 2.2 (85) | Tarife saatleri okunabilir mi | `96.60`, `96.61`, `96.62` |

---

### 5. Enerji Ölçümü

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 5.1 | Mad. 2.2 (31) | Toplam aktif enerji ithalat | `1.8.0` |
| 5.2 | Mad. 2.2 (31) | T1/T2/T3/T4 tarife bazlı enerji | `1.8.1`, `1.8.2`, `1.8.3`, `1.8.4` |
| 5.3 | Mad. 2.2 (31) | Aktif enerji ihracat (çift yönlü sayaçlar) | `2.8.0` |
| 5.4 | Mad. 2.2 (31) | Reaktif enerji endüktif (kombi sayaçlar) | `5.8.0` |
| 5.5 | Mad. 2.2 (31) | Reaktif enerji kapasitif (kombi sayaçlar) | `6.8.0` |
| 5.6 | Mad. 2.2 (31) | Değer formatı: 6 tam + 3 ondalık hane | Regex kontrolü |
| 5.7 | Mad. 2.2 (52) | 12 aylık enerji tarihçesi | `1.8.0*1`–`1.8.0*12` |

---

### 6. Demant (Güç) Ölçümü

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 6.1 | Mad. 4.1 (90) | Max demant okunabilir mi | `1.6.0` |
| 6.2 | Mad. 4.1 (90) | Demant zaman damgası var mı | `1.6.0` formatı: `(kW)(YY-MM-DD,hh:mm)` |
| 6.3 | Mad. 4.1 (91) | 12 aylık demant tarihçesi | `1.6.0*1`–`1.6.0*12` |
| 6.4 | Mad. 4.1 (91) | Demant periyodu geçerli mi (15/30/60 dk) | `0.8.0` |
| 6.5 | Mad. 4.1 (93) | Son sıfırlama tarihi mevcut mu | `0.1.2` |

---

### 7. Yük Profili

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 7.1 | Mad. 4.5 (104) | LP1 okunabilir mi | `97.1.0` |
| 7.2 | Mad. 4.5 (104) | Kayıt aralığı şartnameye uygun mu | `0.8.4` ≤ 15 dk |
| 7.3 | Mad. 4.5 (107) | En az 180 günlük veri mevcut mu | LP kayıt sayısı |
| 7.4 | Mad. 4.5 (104) | Zaman damgası formatı doğru mu | `(YY-MM-DD,hh:mm)` |

---

### 8. Kesinti Kayıtları

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 8.1 | Mad. 4.7 (119) | 3 faz kesinti sayacı mevcut mu | `96.7.0` |
| 8.2 | Mad. 4.7 (125) | Faz bazlı kesinti sayaçları mevcut mu | `96.7.1`, `96.7.2`, `96.7.3` |
| 8.3 | Mad. 4.7 (119) | Kesinti kaydı formatı saniye hassasiyetli mi | `96.7.10*1` formatı |
| 8.4 | Mad. 4.7 (119) | En az 200 kayıt kapasitesi (3 faz) | `96.7.10*1`–`96.7.10*200` |
| 8.5 | Mad. 4.7 (125) | Faz başına en az 200 kayıt | `96.7.11`–`96.7.13` |

---

### 9. Uyarı ve Müdahale Kayıtları

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 9.1 | Mad. 4.8 (127) | Gerilim bağlantı hatası kayıtları | `96.77.2`, `96.77.20*1`–`*10` |
| 9.2 | Mad. 4.9 (134) | Akım bağlantı hatası kayıtları | `96.77.3`, `96.77.30*1`–`*10` |
| 9.3 | Mad. 4.10 (141) | Manyetik alan algılama kayıtları | `96.20.15`, `96.20.16*1`–`*10` |
| 9.4 | Mad. 4.6.1 (110) | Üst kapak açılma kayıtları | `96.20.0`, `96.20.1*1`–`*10` |
| 9.5 | Mad. 4.6.2 (111) | Klemens kapağı açılma kayıtları | `96.20.5`, `96.20.6*1`–`*24` |
| 9.6 | Mad. 4.11 (144) | Sayaç reset kayıtları | `96.11.0`, `96.11.1*1`–`*10` |
| 9.7 | Mad. 4.12 (148) | Nötr gerilim uyarı kayıtları | `96.20.26*1`–`*10` |
| 9.8 | Mad. 4.5 (103) | Tarife değişiklik kayıtları | `96.2.2*1`–`*10` |

---

### 10. Şifre ve Güvenlik

| # | Madde | Kontrol | OBIS / Parametre |
|---|-------|---------|-----------------|
| 10.1 | Mad. 4.3 (156) | P1 kimlik doğrulama çalışıyor mu | `authenticate(P1)` |
| 10.2 | Mad. 4.3 (156) | P3 kimlik doğrulama çalışıyor mu | `authenticate(P3)` |
| 10.3 | Mad. 4.3 (157) | 3 yanlış şifre → kilit mekanizması | Hata kodu kontrolü |

---

## Mimari Tasarım

### Backend — Rust (`src-tauri/src/compliance/`)

```
compliance/
├── mod.rs           → run_compliance_check() Tauri komutu
├── checks/
│   ├── communication.rs   → Kategori 1
│   ├── time.rs            → Kategori 2-3
│   ├── tariff.rs          → Kategori 4
│   ├── energy.rs          → Kategori 5
│   ├── demand.rs          → Kategori 6
│   ├── load_profile.rs    → Kategori 7
│   ├── outages.rs         → Kategori 8
│   ├── warnings.rs        → Kategori 9
│   └── security.rs        → Kategori 10
└── types.rs         → CheckResult, CheckStatus struct'ları
```

**CheckResult Yapısı:**
```rust
pub struct CheckResult {
    pub id: String,           // "2.1", "5.3" vb.
    pub article: String,      // "Mad. 2.1.11"
    pub category: String,     // "Zaman / Saat"
    pub title: String,        // "RTC okunabilirliği"
    pub expected: String,     // "0.9.1 ve 0.9.2 yanıt vermeli"
    pub actual: String,       // "0.9.1 = 14:32:05, 0.9.2 = 26-02-25"
    pub status: CheckStatus,  // Pass / Fail / Warn
    pub note: Option<String>, // Ek açıklama
}

pub enum CheckStatus { Pass, Fail, Warn }
```

---

### Frontend — Svelte (`src/lib/pages/ComplianceCheck.svelte`)

```
ComplianceCheck.svelte
├── Header: "TEDAŞ Uygunluk Kontrolü" + Başlat butonu
├── Progress: Gerçek zamanlı kontrol adımları (Tauri event ile)
├── Sonuçlar:
│   ├── Özet kart: X/Y uygun, Z uyumsuz, W kontrol edilemedi
│   ├── Kategori bazlı accordion listesi
│   └── Her madde: ikon + madde no + başlık + beklenen/bulunan
└── Rapor: Excel export butonu
```

---

### Tauri Event Akışı

```
Frontend                          Backend
   │                                 │
   │── run_compliance_check() ──────>│
   │                                 │ Her kontrol tamamlandıkça:
   │<── compliance-progress ─────────│ { current, total, latest_result }
   │                                 │
   │<── (return) Vec<CheckResult> ───│
```

---

## Uygulama Sırası

1. **`types.rs`** — CheckResult ve CheckStatus tanımları
2. **`communication.rs`** — En basit kontroller (bağlantı sırasında zaten bilgi var)
3. **`time.rs`** — RTC ve DST kontrolleri
4. **`energy.rs`** — Zorunlu OBIS kodlarının varlığı
5. **`tariff.rs`** — Tarife yapısı
6. **`demand.rs`** — Demant kontrolleri
7. **`load_profile.rs`** — Yük profili
8. **`outages.rs`** — Kesinti kayıtları
9. **`warnings.rs`** — Uyarı kayıtları
10. **`security.rs`** — Şifre/güvenlik
11. **`mod.rs`** — Tauri komutu ve event gönderimi
12. **`ComplianceCheck.svelte`** — Frontend

---

## Açık Sorular (Uygulamaya Başlamadan Netleşmeli)

1. **Şifre gerektiren kontroller (P2/P3):**
   - Sadece P1 (şifresiz) ile yapılabilenler mi kontrol edilsin?
   - Yoksa kullanıcıdan P2/P3 şifresi de sorulsun mu?

2. **Rapor formatı:**
   - Excel mi?
   - PDF mi?
   - Her ikisi de mi?

3. **Tetikleyici:**
   - Sayaç bağlandığında otomatik mı çalışsın?
   - Yoksa kullanıcı manuel mi başlatsın?

4. **Entegrasyon noktası:**
   - Sidebar'a yeni menü maddesi olarak mı eklensin?
   - Home sayfasına "Uygunluk Kontrol Et" butonu mu eklensin?
