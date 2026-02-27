# Uyumluluk Kuralları Dokümantasyonu

**Versiyon:** 2.0.0
**Güncelleme:** 2026-02

Bu belge `compliance_rules.toml` dosyasındaki tüm kuralları, hangi şartnameye ve hangi maddeye dayandıklarını açıklar.

## Şartname Referansları

| Kısaltma | Belge |
|----------|-------|
| **TEDAŞ** | TEDAŞ MLZ/2017-062.B — Akıllı Sayaç Teknik Şartnamesi |
| **MASS** | MASS Protokol v11 — Milli Akıllı Sayaç Sistemleri Teknik Şartnamesi |

---

## Kural Kategorileri

- [KİMLİK — ID](#kimlik-kuralları-id)
- [ELEKTRİKSEL PARAMETRELER — EL](#elektriksel-parametreler-el)
- [PİL VE RÖLE — BAT](#pil-durumu-bat)
- [DEMANT PERİYODU — DEM](#demant-periyodu-dem)
- [TARİFE DENGESİ — TRF](#tarife-dengesi-trf)
- [SAAT SAPMASI — TIME](#saat-sapması-time)
- [FF HATA/DURUM KODU — FF](#ff-hatandurum-kodu-ff)

---

## Kimlik Kuralları (ID)

| Kod | Alan | Kontrol | Şartname | Madde | Açıklama |
|-----|------|---------|----------|-------|----------|
| ID-001 | `serial_number` | Boş olmamalı | TEDAŞ §3.1, MASS §3.1 | — | Seri numarası boş ise sayaç kimliği okunamıyor demektir. Bağlantı sorunu veya sayaç arızasına işaret eder. |
| ID-002 | `production_date` | Boş olmamalı | TEDAŞ §3.1, MASS §3.1 | — | Üretim tarihi sayaç sicili için zorunludur. |
| ID-003 | `calibration_date` | Boş olmamalı | TEDAŞ §2.1.11, MASS §2.1.11 | madde 47 | Kalibrasyon tarihi yasal metroloji için zorunludur. |

---

## Elektriksel Parametreler (EL)

### Gerilim

| Kod | Alan | Min (V) | Max (V) | Faz | Şartname | Madde |
|-----|------|---------|---------|-----|----------|-------|
| EL-001 | `voltage_l1` | 90 | 265 | Tümü | TEDAŞ §2.2.2 Çizelge 4, MASS §2.2.2 | — |
| EL-002 | `voltage_l2` | 90 | 265 | 3 Faz | TEDAŞ §2.2.2 Çizelge 4, MASS §2.2.2 | — |
| EL-003 | `voltage_l3` | 90 | 265 | 3 Faz | TEDAŞ §2.2.2 Çizelge 4, MASS §2.2.2 | — |

**Açıklama:** Şartname nominal gerilimi 230 V (tek faz), 400 V (üç faz) olarak tanımlar. Çalışma aralığı %−61 / +15 toleransla **90–265 V** olarak belirlenmiştir. Bu aralık dışındaki değerler sayacın ölçüm hassasiyetini doğrudan etkiler.

### Frekans

| Kod | Alan | Min (Hz) | Max (Hz) | Şartname | Madde |
|-----|------|---------|---------|----------|-------|
| EL-004 | `frequency` | 49 | 51 | TEDAŞ §1.4 Çizelge 2, MASS §1.4 | — |

**Açıklama:** Türkiye şebeke frekansı 50 Hz'dir. Şartname **±%2 tolerans** (49–51 Hz) tanımlar. Bu aralık dışındaki frekans değerleri şebeke arızasına veya yerel üretim/yük dengesizliğine işaret eder.

---

## Pil Durumu (BAT)

| Kod | Alan | Kontrol | Şartname | Açıklama |
|-----|------|---------|----------|----------|
| BAT-001 | `battery_status` | `"low"` olmamalı | TEDAŞ §4.11, MASS §4.11 | Pil seviyesi düşükse sayaç belleği ve RTC güç kesintisinde korunamamaz. |

**Olası değerler:** `"full"` (normal), `"low"` (uyarı), `""` (desteklenmiyor)

---

## Demant Periyodu (DEM)

| Kod | Alan | Beklenen | Şartname | Madde |
|-----|------|---------|----------|-------|
| DEM-001 | `demand_period` | `"15"` | TEDAŞ §4.3, MASS §4.3 | madde 91 (TEDAŞ), madde 80 (MASS) |

**Açıklama:** TEDAŞ ve MASS şartnameleri demant ölçüm periyodunu standart olarak **15 dakika** olarak belirler. Farklı bir değer (30, 60 dakika vb.) programlanmışsa uyumsuzluk oluşur.

---

## Tarife Dengesi (TRF)

| Kod | Alan | Kontrol | Tolerans | Şartname | Madde |
|-----|------|---------|---------|----------|-------|
| TRF-001 | `tariff_sum` | T0 ≈ T1+T2+T3+T4 | 0.01 kWh | TEDAŞ §4.2, MASS §4.2 | — |

**Açıklama:** Toplam aktif enerji endeksi (T0) ile tarife dilimlerinin toplamı (T1+T2+T3+T4) arasındaki fark 0.01 kWh'i geçmemeli. Daha büyük sapma, sayaç firmware veya tarife yapılandırma hatasına işaret eder.

---

## Saat Sapması (TIME)

| Kod | Alan | Maks Sapma | Şartname | Madde |
|-----|------|-----------|----------|-------|
| TIME-001 | `time_drift` | 5 dakika | TEDAŞ §2.1.11, MASS §2.1.11 | madde 47 |

**Açıklama:** TS EN 62054-21 standardı RTC sapmasını **maksimum 0.5 saniye/gün** olarak sınırlar. Uygulama eşiği olarak 5 dakika kullanılmıştır — bu değerin üzerinde zaman senkronizasyonu gerekir.

---

## FF Hata/Durum Kodu (FF)

FF kodu (`F.F.0` OBIS), 64 bitlik bir durum/hata kaydıdır. Her bit bir durum veya olayı temsil eder.

**Referans:** TEDAŞ §5.3 + Ek-F, MASS §5.3 + Ek-C

Her kural `bit_zero` kontrolü yapar: **ilgili bit 1 ise uyarı üretilir.**

### Donanım Hataları (Bit 0–4)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-001 | 0 | error | TEDAŞ Ek-F, MASS Ek-C | **RTC (Saat Modülü) Arızası** — Gerçek zamanlı saat çalışmıyor. Zaman damgalı ölçümler güvenilmez. |
| FF-002 | 1 | error | TEDAŞ Ek-F, MASS Ek-C | **Ölçüm Entegresi Arızası** — Enerji ölçüm devresi arızalı. Tüm enerji verileri şüpheli. |
| FF-003 | 2 | error | TEDAŞ Ek-F, MASS Ek-C | **Kritik Ölçüm Hatası** — Donanım düzeyinde ölçüm hatası. |
| FF-004 | 3 | warning | TEDAŞ Ek-F, MASS Ek-C | **RS-485 Port Hatası** — Haberleşme portu sorunlu. |
| FF-005 | 4 | error | TEDAŞ Ek-F, MASS Ek-C | **Kalibrasyon Yapılmamış** — Sayaç kalibre edilmemiş; ölçümler geçersiz. |

### Fiziksel Müdahale (Bit 5–7)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-006 | 5 | warning | TEDAŞ Ek-F, MASS Ek-C | **Klemens Kapağı Açık** — Akım veya bağlantı terminallerine erişim sağlanmış. |
| FF-007 | 6 | warning | TEDAŞ Ek-F, MASS Ek-C | **Üst Kapak Açık** — Sayaç üst kapağı açılmış veya açılma geçmişi var. |
| FF-008 | 7 | info | TEDAŞ Ek-F, MASS Ek-C | **Üst Kapak Açılma Bilgisi Mevcut** — Geçmişte kapak açılmış; şu an kapalı olabilir. |

> **Not:** Bit 5 fiziksel anlık durumu, Bit 7 geçmiş olayı yansıtır. Her ikisi de araziye çıkış raporuna alınmalıdır.

### Akım Var / Gerilim Yok (Bit 8–10)

| Kod | Bit | Faz | Şiddet | Şartname | Açıklama |
|-----|-----|-----|--------|----------|----------|
| FF-009 | 8 | Tümü | error | TEDAŞ Ek-F, MASS Ek-C | **R Fazı — Akım Var, Gerilim Yok** — Kaçak bağlantı veya hat manipülasyonu şüphesi. |
| FF-010 | 9 | 3 Faz | error | TEDAŞ Ek-F, MASS Ek-C | **S Fazı — Akım Var, Gerilim Yok** |
| FF-011 | 10 | 3 Faz | error | TEDAŞ Ek-F, MASS Ek-C | **T Fazı — Akım Var, Gerilim Yok** |

### Manyetik Alan Müdahalesi (Bit 11–13)

| Kod | Bit | Faz | Şiddet | Şartname | Açıklama |
|-----|-----|-----|--------|----------|----------|
| FF-012 | 11 | Tümü | error | TEDAŞ §4.10 madde 141, MASS §4.10 madde 128 | **R Fazı — Manyetik Alan >400 mT** — Güçlü mıknatıs ile ölçüm manipülasyonu girişimi. |
| FF-013 | 12 | 3 Faz | error | TEDAŞ §4.10 madde 141, MASS §4.10 madde 128 | **S Fazı — Manyetik Alan >400 mT** |
| FF-014 | 13 | 3 Faz | error | TEDAŞ §4.10 madde 141, MASS §4.10 madde 128 | **T Fazı — Manyetik Alan >400 mT** |

> **Şartname:** TEDAŞ §4.10 madde 141 ve MASS §4.10 madde 128, 400 mT üzerindeki manyetik alana karşı sayacın koruma işlevi görmesi gerektiğini belirtir.

### Endeks Durgunluğu (Bit 14–16)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-015 | 14 | warning | TEDAŞ Ek-F, MASS Ek-C | **Akım >20 mA — T1 Endeksi Durağan** — Akım geçiyor ama T1 tarifesi sayılmıyor. |
| FF-016 | 15 | warning | TEDAŞ Ek-F, MASS Ek-C | **Akım >20 mA — T2 Endeksi Durağan** |
| FF-017 | 16 | warning | TEDAŞ Ek-F, MASS Ek-C | **Akım >20 mA — T3 Endeksi Durağan** |

### Faz Endeks İlerleyişi Sıfır (Bit 17–19)

| Kod | Bit | Faz | Şiddet | Şartname | Açıklama |
|-----|-----|-----|--------|----------|----------|
| FF-018 | 17 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | **R Faz Endeksi İki Aydır İlerlemedi** — Tüketim kaydedilmiyor. |
| FF-019 | 18 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | **S Faz Endeksi İki Aydır İlerlemedi** |
| FF-020 | 19 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | **T Faz Endeksi İki Aydır İlerlemedi** |

### Faz Kesilmesi Devam Ediyor (Bit 20–23)

| Kod | Bit | Faz | Şiddet | Şartname | Açıklama |
|-----|-----|-----|--------|----------|----------|
| FF-021 | 20 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | **R Faz Kesilmesi Devam Ediyor** — Hat kesintisi alarm durumu sürmekte. |
| FF-022 | 21 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | **S Faz Kesilmesi Devam Ediyor** |
| FF-023 | 22 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | **T Faz Kesilmesi Devam Ediyor** |
| FF-024 | 23 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | **3 Faz Birden Kesilmesi Devam Ediyor** |

### Sonlanmamış Uyarılar (Bit 24–25)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-025 | 24 | warning | TEDAŞ Ek-F, MASS Ek-C | **Akım Hata Uyarısı Aktif** — Akım ile ilgili bir hata alarm durumu henüz sonlanmamış. |
| FF-026 | 25 | warning | TEDAŞ Ek-F, MASS Ek-C | **Gerilim Hata Uyarısı Aktif** — Gerilim ile ilgili bir hata alarm durumu henüz sonlanmamış. |

### Endeks Gerilemesi (Bit 26–28)

| Kod | Bit | Faz | Şiddet | Şartname | Açıklama |
|-----|-----|-----|--------|----------|----------|
| FF-027 | 26 | Tümü | error | TEDAŞ Ek-F, MASS Ek-C | **R Faz Enerji Endeksi Geriledi** — Endeks azalması enerji hırsızlığına işaret eder. |
| FF-028 | 27 | 3 Faz | error | TEDAŞ Ek-F, MASS Ek-C | **S Faz Enerji Endeksi Geriledi** |
| FF-029 | 28 | 3 Faz | error | TEDAŞ Ek-F, MASS Ek-C | **T Faz Enerji Endeksi Geriledi** |

### Demant Anomalisi (Bit 29–31)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-030 | 29 | warning | TEDAŞ Ek-F, MASS Ek-C | **Aktif Güç Varken Maks. Demant İlerlemedi** — Güç tüketimi var ama demant endeksi kaydedilmiyor. |
| FF-031 | 30 | warning | TEDAŞ Ek-F, MASS Ek-C | **T0 ile T1+T2+T3+T4 Farkı >200 W** — Tarife toplamları arasında büyük tutarsızlık. |
| FF-032 | 31 | info | TEDAŞ Ek-F, MASS Ek-C | **T4 Tarifesinde Endeks Var** — T4 tarifesi (isteğe bağlı) sayılıyor; sadece bilgi amaçlı. |

### Tarife Bozuklukları (Bit 32–34)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-033 | 32 | error | TEDAŞ Ek-F, MASS Ek-C | **Tarife Dilimi / Saat Ayarları Arızalı** — Tarife yapılandırması bozuk, enerji yanlış dilimlenebilir. |
| FF-034 | 33 | warning | TEDAŞ Ek-F, MASS Ek-C | **Tarife Değişiklik Yılı ≠ Üretim Yılı** — Tarife en son değiştirildiğinde yıl bilgisi tutarsız. |
| FF-035 | 34 | warning | TEDAŞ Ek-F, MASS Ek-C | **Üretim Yılı ≠ Kalibrasyon Yılı** — Üretim ve kalibrasyon yılları uyuşmuyor. |

### Uzun Süreli Anomaliler (Bit 35–38)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-036 | 35 | warning | TEDAŞ Ek-F, MASS Ek-C | **Son 3 Ayda Sabit Maks. Demant** — 3 ay boyunca demant değişmedi; sayaç veya tüketim sorunu olabilir. |
| FF-037 | 36 | error | TEDAŞ Ek-F, MASS Ek-C | **İki Bellek Bölgesi Hatası** — Sayaç dahili hafıza bütünlüğü bozuk; veri kaybı riski. |
| FF-038 | 37 | warning | TEDAŞ Ek-F, MASS Ek-C | **Sistem Pili Zayıf** — Sayaç ana sistem pili düşük. |
| FF-039 | 38 | warning | TEDAŞ Ek-F, MASS Ek-C | **RTC Pili Zayıf** — Zaman saati pili düşük; güç kesintisinde saat kaybı olur. |

### Sık Alarm Durumları (Bit 39–43)

| Kod | Bit | Faz | Şiddet | Şartname | Açıklama |
|-----|-----|-----|--------|----------|----------|
| FF-040 | 39 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | **Sık Kesinti Alarmı (>20 kesinti/saat)** — Hat bağlantısı istikrarsız. |
| FF-041 | 40 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | **Genel Sık Uyarı Alarmı Aktif** |
| FF-042 | 41 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | **R Faz Sık Uyarı Alarmı Aktif** |
| FF-043 | 42 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | **S Faz Sık Uyarı Alarmı Aktif** |
| FF-044 | 43 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | **T Faz Sık Uyarı Alarmı Aktif** |

### Güç ve Gerilim Aşımları (Bit 44–53)

| Kod | Bit | Faz | Şiddet | Şartname | Eşik | Açıklama |
|-----|-----|-----|--------|----------|------|----------|
| FF-045 | 44 | Tümü | info | TEDAŞ Ek-F, MASS Ek-C | MF=20 kW / TF=60 kW | **Yüksek Maks. Demant Aşımı** — Sözleşme gücü aşımı; sadece bilgi. |
| FF-046 | 45 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | >253 V | **R Faz Yüksek Gerilim Aşımı** |
| FF-047 | 46 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | >253 V | **S Faz Yüksek Gerilim Aşımı** |
| FF-048 | 47 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | >253 V | **T Faz Yüksek Gerilim Aşımı** |
| FF-049 | 48 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | <195.5 V | **R Faz Düşük Gerilim Aşımı** |
| FF-050 | 49 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | <195.5 V | **S Faz Düşük Gerilim Aşımı** |
| FF-051 | 50 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | <195.5 V | **T Faz Düşük Gerilim Aşımı** |
| FF-052 | 51 | Tümü | warning | TEDAŞ Ek-F, MASS Ek-C | — | **R Faz Yüksek Akım Aşımı** |
| FF-053 | 52 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | — | **S Faz Yüksek Akım Aşımı** |
| FF-054 | 53 | 3 Faz | warning | TEDAŞ Ek-F, MASS Ek-C | — | **T Faz Yüksek Akım Aşımı** |

> **Gerilim Eşikleri:** 230 V nominalinde %+10 = 253 V (yüksek), %−15 = 195.5 V (düşük). TEDAŞ §2.2.2 Çizelge 4.

### Dengesizlik ve Röle (Bit 54–55)

| Kod | Bit | Şiddet | Şartname | Açıklama |
|-----|-----|--------|----------|----------|
| FF-055 | 54 | warning | TEDAŞ Ek-F, MASS Ek-C | **Nötr-Faz Akım Dengesizliği** — Nötr hattındaki akım faz akımından belirgin şekilde farklı. |
| FF-056 | 55 | error | TEDAŞ Ek-F, MASS Ek-C | **Açma-Kesme Rölesi Arızası** — Sayaç devre kesme rölesi çalışmıyor. |

> **Not:** Bit 56–63 şartname tarafından **Rezerve** olarak ayrılmıştır; kural tanımlanmamıştır.

---

## GF Coğrafi Durum Kodu (F.F.1)

GF kodu (`F.F.1` OBIS) sayacın coğrafi konumlandırma bilgisini içerir. Bu kod şu an uyumluluk kurallarına dahil edilmemiştir — gelecek versiyonda eklenebilir.

| Bit Aralığı | Alan |
|-------------|------|
| 0–4 | EDAŞ ID |
| 5–19 | Trafo Merkez ID |
| 20–23 | Trafo ID |
| 24–29 | Depar ID |
| 30–31 | Faz ID |
| 32–33 | Kol ID |
| 34–43 | Maksimum Akım |

---

## Yeni Kural Ekleme Rehberi

`compliance_rules.toml` dosyasına yeni bir kural eklemek için:

```toml
[[rules]]
code = "XX-001"           # Benzersiz kod (kategori-numara)
field = "voltage_l1"      # Hangi alan kontrol edilecek
check = "range"           # Kontrol tipi
min = 90.0                # (range için) Alt sınır
max = 265.0               # (range için) Üst sınır
severity = "error"        # "error" | "warning" | "info"
description = "Açıklama"  # Kullanıcıya gösterilecek metin
# phases = 3              # Sadece 3 fazlı sayaçlarda çalıştır (isteğe bağlı)
```

### Desteklenen Alanlar

| Alan | Tip | Açıklama |
|------|-----|----------|
| `serial_number` | string | Seri numarası |
| `program_version` | string | Yazılım versiyonu |
| `production_date` | string | Üretim tarihi |
| `calibration_date` | string | Kalibrasyon tarihi |
| `voltage_l1/l2/l3` | sayı | Faz gerilimleri (V) |
| `current_l1/l2/l3` | sayı | Faz akımları (A) |
| `frequency` | sayı | Şebeke frekansı (Hz) |
| `power_factor_l1/l2/l3` | sayı | Güç faktörü |
| `active_energy_import_total` | sayı | Toplam aktif enerji (kWh) |
| `active_energy_import_t1/t2/t3/t4` | sayı | Tarife dilimi enerjisi (kWh) |
| `ff_code` | hex string | FF durum kodu (64 bit) |
| `gf_code` | hex string | GF coğrafi kodu (64 bit) |
| `battery_status` | string | `"full"`, `"low"`, `""` |
| `relay_status` | string | `"active"`, `"passive"`, `""` |
| `demand_period` | string | Demant periyodu (dakika) |
| `tariff_sum` | özel | T0 = T1+T2+T3+T4 dengesi |
| `time_drift` | özel | Sayaç/sistem saat farkı |
