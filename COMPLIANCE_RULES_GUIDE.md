# compliance_rules.toml — Kural Yönetim Kılavuzu

Bu kılavuz, OmniCore Meter Suite'in uyumluluk kurallarını nasıl düzenleyeceğinizi adım adım açıklar. Programlama bilgisi gerekmez.

---

## Dosyanın Konumu

| İşletim Sistemi | Yol |
|-----------------|-----|
| **Windows**     | `C:\Users\<kullanıcı adı>\AppData\Roaming\omnicore\compliance_rules.toml` |
| **Linux**       | `~/.local/share/omnicore/compliance_rules.toml` |

> Uygulama ilk çalıştığında bu dosya otomatik oluşturulur. Daha sonra kendiniz düzenleyebilirsiniz.

---

## Dosya Yapısı

```toml
rules_version = "2.0.0"
update_url = ""

[[rules]]
code        = "XX-001"
field       = "alan_adi"
check       = "kural_tipi"
severity    = "error"
description = "Uyumsuzluk açıklaması"
# ... kural tipine göre ek alanlar
```

Her `[[rules]]` bloğu bir kuraldır. Dosyada dilediğiniz kadar blok olabilir.

---

## Zorunlu Alanlar

| Alan | Açıklama | Örnek |
|------|----------|-------|
| `code` | Benzersiz kural kodu — raporda bu kod görünür | `"EL-005"` |
| `field` | Sayaçtan okunan hangi değer kontrol edilecek | `"voltage_l1"` |
| `check` | Kontrol tipi | `"range"` |
| `severity` | Önem derecesi: `"error"`, `"warning"`, `"info"` | `"warning"` |
| `description` | Uyumsuzluk durumunda gösterilecek mesaj | `"Gerilim aralık dışı"` |

---

## Opsiyonel Alanlar

| Alan | Açıklama | Kullanıldığı Kural Tipi |
|------|----------|------------------------|
| `min` | Minimum izin verilen sayısal değer | `range` |
| `max` | Maksimum izin verilen sayısal değer | `range` |
| `value` | Eşitlik için beklenen değer (string) | `equals`, `not_equals` |
| `bit` | Kontrol edilecek bit numarası (0'dan başlar) | `bit_zero`, `bit_one` |
| `tolerance` | Tarife dengesi toleransı (kWh) | `tariff_balance` |
| `max_drift` | Maksimum saat sapması (dakika) | `time_drift_minutes` |
| `phases` | Sadece `1` veya `3` fazlı sayaçlara uygula | Tüm tipler |
| `spec_ref` | Şartname referansı (bilgi amaçlı) | Tüm tipler |

---

## Kural Tipleri

### `range` — Sayısal Aralık Kontrolü

Alan değeri `min` ile `max` arasında değilse hata üretir.

```toml
[[rules]]
code        = "EL-005"
field       = "power_factor_l1"
check       = "range"
min         = 0.85
max         = 1.0
severity    = "warning"
description = "L1 güç faktörü düşük (beklenen: ≥ 0,85)"
spec_ref    = "TEDAŞ Şartname 2.2.3"
```

> `min` veya `max`'tan birini yazmazsanız o yönde sınır olmaz.

---

### `equals` — Eşitlik Kontrolü

Alan değeri `value`'ya tam eşit değilse hata üretir.

```toml
[[rules]]
code        = "DEM-002"
field       = "lp_period"
check       = "equals"
value       = "15"
severity    = "warning"
description = "Yük profili kayıt periyodu 15 dakika olmalı"
spec_ref    = "TEDAŞ Şartname 4.3"
```

---

### `not_equals` — Eşit Olmamalı Kontrolü

Alan değeri `value`'ya eşitse hata üretir.

```toml
[[rules]]
code        = "BAT-002"
field       = "relay_status"
check       = "not_equals"
value       = "error"
severity    = "error"
description = "Röle hata durumunda — müdahale gerekiyor"
```

---

### `not_empty` — Boş Olmamalı Kontrolü

Alan boş string ise hata üretir. Ek parametre gerekmez.

```toml
[[rules]]
code        = "ID-004"
field       = "program_version"
check       = "not_empty"
severity    = "info"
description = "Program versiyonu okunamadı"
```

---

### `bit_zero` — Bit 0 Olmalı Kontrolü

`ff_code` veya `gf_code` içindeki belirtilen bit `1` ise hata üretir (o bit sıfır olmalıdır).

```toml
[[rules]]
code        = "FF-057"
field       = "ff_code"
check       = "bit_zero"
bit         = 56
severity    = "error"
description = "Yeni hata biti aktif (FF Bit 56)"
spec_ref    = "TEDAŞ Şartname 5.3 Ek-F"
```

---

### `bit_one` — Bit 1 Olmalı Kontrolü

Belirtilen bit `0` ise hata üretir (o bit bir olmalıdır).

```toml
[[rules]]
code        = "GF-001"
field       = "gf_code"
check       = "bit_one"
bit         = 0
severity    = "info"
description = "GF Bit 0 beklendiği gibi 1 değil"
```

---

### `tariff_balance` — Tarife Dengesi Kontrolü

`T1 + T2 + T3 + T4` toplamı `toplam` değerinden `tolerance` kWh'den fazla sapıyorsa hata üretir.
`field` değeri sabit `"tariff_sum"` olmalıdır.

```toml
[[rules]]
code        = "TRF-002"
field       = "tariff_sum"
check       = "tariff_balance"
tolerance   = 0.5
severity    = "info"
description = "Tarife toplamı 0,5 kWh toleransla uyumsuz"
```

---

### `time_drift_minutes` — Saat Sapması Kontrolü

Sayaç saati ile sistem saati arasındaki fark `max_drift` dakikayı aşarsa hata üretir.
`field` değeri sabit `"time_drift"` olmalıdır.

```toml
[[rules]]
code        = "TIME-002"
field       = "time_drift"
check       = "time_drift_minutes"
max_drift   = 2
severity    = "error"
description = "Sayaç saati 2 dakikadan fazla sapıyor — acil senkronizasyon gerekiyor"
```

---

## Kullanılabilir `field` Değerleri

### Kimlik Bilgileri
| Alan | Açıklama |
|------|----------|
| `serial_number` | Seri numarası |
| `program_version` | Program versiyonu |
| `production_date` | Üretim tarihi |
| `calibration_date` | Kalibrasyon tarihi |
| `meter_date` | Sayaç tarihi |
| `meter_time` | Sayaç saati |

### Elektriksel Ölçümler
| Alan | Açıklama |
|------|----------|
| `voltage_l1` | L1 (R faz) gerilimi (V) |
| `voltage_l2` | L2 (S faz) gerilimi (V) |
| `voltage_l3` | L3 (T faz) gerilimi (V) |
| `current_l1` | L1 akımı (A) |
| `current_l2` | L2 akımı (A) |
| `current_l3` | L3 akımı (A) |
| `frequency` | Şebeke frekansı (Hz) |
| `power_factor_l1` | L1 güç faktörü |
| `power_factor_l2` | L2 güç faktörü |
| `power_factor_l3` | L3 güç faktörü |

### Enerji Endeksleri
| Alan | Açıklama |
|------|----------|
| `active_energy_import_total` | Toplam aktif enerji (kWh) |
| `active_energy_import_t1` | T1 tarifesi aktif enerji (kWh) |
| `active_energy_import_t2` | T2 tarifesi aktif enerji (kWh) |
| `active_energy_import_t3` | T3 tarifesi aktif enerji (kWh) |
| `active_energy_import_t4` | T4 tarifesi aktif enerji (kWh) |

### Durum Bilgileri
| Alan | Açıklama |
|------|----------|
| `ff_code` | FF hata/durum kodu (hex veya binary) |
| `gf_code` | GF durum kodu (hex veya binary) |
| `battery_status` | Pil durumu |
| `relay_status` | Röle durumu |
| `demand_period` | Talep ölçüm periyodu (dakika) |
| `lp_period` | Yük profili kayıt periyodu (dakika) |

### Özel Kontroller (sabit field adı)
| Alan | Açıklama |
|------|----------|
| `tariff_sum` | `tariff_balance` kontrolü için (T1+T2+T3+T4 vs toplam) |
| `time_drift` | `time_drift_minutes` kontrolü için (sayaç vs sistem saati) |

---

## Yeni Kural Ekleme — Adım Adım

1. Dosyayı bir metin editörüyle açın (Not Defteri, VS Code vb.)

2. Dosyanın sonuna yeni bir `[[rules]]` bloğu ekleyin:

```toml
[[rules]]
code        = "EL-006"
field       = "current_l1"
check       = "range"
min         = 0.0
max         = 100.0
phases      = 3
severity    = "warning"
description = "L1 akımı 100 A sınırını aşıyor"
spec_ref    = "TEDAŞ Şartname 2.2.2"
```

3. Dosyayı kaydedin.

4. Uygulamada Uyumluluk sayfasına gidin ve **Yenile** butonuna tıklayın.

> **Önemli:** Her kuralın `code` değeri dosya içinde **benzersiz** olmalıdır. Aynı kodu iki farklı kurala vermeyin.

---

## Mevcut Kural Güncelleme — Adım Adım

1. Dosyayı açın ve güncellemek istediğiniz `[[rules]]` bloğunu bulun. Kodu (`code`) referans alarak arayabilirsiniz.

2. İlgili değeri değiştirin. Örneğin gerilim üst sınırını 253 V'a indirmek için:

```toml
# ÖNCE:
[[rules]]
code = "EL-001"
field = "voltage_l1"
check = "range"
min = 90.0
max = 265.0

# SONRA:
[[rules]]
code = "EL-001"
field = "voltage_l1"
check = "range"
min = 90.0
max = 253.0
```

3. Dosyayı kaydedin.

4. Uygulamada **Yenile** butonuna tıklayın — değişiklik anında devreye girer.

---

## Kural Devre Dışı Bırakma

Bir kuralı silmek yerine geçici olarak devre dışı bırakmak isterseniz, ilgili `[[rules]]` bloğunun tamamını `#` ile yorum satırına alın:

```toml
# [[rules]]
# code = "TIME-001"
# field = "time_drift"
# check = "time_drift_minutes"
# max_drift = 5
# severity = "warning"
# description = "Saat sapması kontrolü (geçici devre dışı)"
```

---

## Değişikliklerin Uygulanması

Dosyayı düzenledikten sonra yapmanız gereken tek şey:

1. Uygulamayı açın (zaten açıksa gerek yok)
2. **Uyumluluk** sayfasına gidin
3. Sağ üstteki **Yenile** (↻) butonuna tıklayın
4. Tekrar **Denetim Çalıştır** yapın

Rust kodunda veya başka bir dosyada değişiklik **gerekmez**. Kural dosyası çalışma zamanında okunur.

---

## `rules_version` Alanı

Dosyanın en üstündeki `rules_version` değeri bilgi amaçlıdır. Değiştirmeniz şart değildir, ancak büyük güncellemeleri takip etmek için artırabilirsiniz:

```toml
rules_version = "2.1.0"
```

---

## Sık Yapılan Hatalar

| Hata | Çözüm |
|------|-------|
| Yenile sonrası kural görünmüyor | TOML söz dizimi hatalı olabilir — `=` işaretini ve tırnak işaretlerini kontrol edin |
| `code` çakışması | Her kurala farklı bir `code` verin |
| `bit_zero` çalışmıyor | `bit` değerinin 0'dan başladığını unutmayın (Bit 1 → `bit = 0` değil, `bit = 1`) |
| `range` her zaman hata veriyor | `min` ve `max`'ın sayı (tırnak olmadan) girildiğinden emin olun: `min = 90.0` |
| `phases` alanı yok sayılıyor | Değer `1` veya `3` olmalı, string değil: `phases = 3` (tırnak yok) |

---

## Sahadaki Cihazlara Dağıtım

Kural dosyasını güncelledikten sonra değişikliklerin sahadaki tüm bilgisayarlara ulaşması için üç yöntem kullanılabilir.

---

### Nasıl Çalışır?

Uygulama her uyumluluk denetiminde `compliance_rules.toml` içindeki `update_url` adresine bir istek atar. Bu adres şu yapıda bir JSON döndürmelidir:

```json
{
  "version": "2.1.0",
  "url": "https://.../compliance_rules.toml"
}
```

- `version`: Sunucudaki güncel kural versiyonu
- `url`: Güncel TOML dosyasının indirme adresi

Uygulama yerel `rules_version` ile sunucudaki `version`'ı karşılaştırır:

| Fark | Sonuç |
|------|-------|
| Aynı veya 1 major geride | Denetim çalışır, güncelleme önerilir |
| 2+ major geride | Denetim **kilitlenir**, güncelleme zorunludur |
| Sunucuya ulaşılamıyor | Denetim çalışır, uyarı gösterilir |

> **Önemli:** `update_url` boş bırakılırsa otomatik güncelleme tamamen devre dışıdır. Sadece elle dağıtım yapılır.

---

### Yöntem 1: Elle Kopyalama (En Basit)

İnternet bağlantısı olmayan ortamlar veya az sayıda bilgisayar için uygundur.

**Adımlar:**

1. Güncel `compliance_rules.toml` dosyasını hazırlayın
2. USB bellek veya ağ paylaşımı üzerinden her bilgisayara kopyalayın
3. Her bilgisayarda şu konuma yapıştırın:
   - **Windows:** `C:\Users\<kullanıcı adı>\AppData\Roaming\omnicore\compliance_rules.toml`
   - **Linux:** `~/.local/share/omnicore/compliance_rules.toml`
4. Uygulamada **Yenile** butonuna tıklayın

**Ne zaman kullanılır:** Saha ekibiniz az sayıda bilgisayarda çalışıyorsa veya ağ erişimi kısıtlıysa.

---

### Yöntem 2: GitHub ile Otomatik Dağıtım (Önerilen)

GitHub ücretsiz ve kamuya açık dosyalar için idealdir. Güncelleme yapıldığında tüm sahalar otomatik olarak bilgilendirilir.

#### Kurulum (Bir kez yapılır)

**1. GitHub reposuna iki dosya ekleyin:**

`version.json` — Güncel versiyon bilgisi:
```json
{
  "version": "2.1.0",
  "url": "https://raw.githubusercontent.com/KULLANICI/REPO/main/compliance_rules.toml"
}
```

`compliance_rules.toml` — Kural dosyasının kendisi.

> `KULLANICI` ve `REPO` kısımlarını kendi GitHub kullanıcı adı ve repo adıyla değiştirin.

**2. `update_url`'i ayarlayın** — `compliance_rules.toml` dosyasının en üstüne ekleyin:

```toml
rules_version = "2.1.0"
update_url = "https://raw.githubusercontent.com/KULLANICI/REPO/main/version.json"
```

**3. Bu `update_url`'li dosyayı sahadaki tüm bilgisayarlara bir kez elle kopyalayın** (bundan sonra güncellemeler otomatik olur).

#### Güncelleme (Her seferinde)

Kural dosyasını değiştirdiğinizde:

1. `compliance_rules.toml`'daki `rules_version` değerini artırın (ör. `2.1.0` → `2.2.0`)
2. `version.json`'daki `version` alanını da aynı değere güncelleyin
3. Her iki dosyayı GitHub'a push edin:

```bash
git add compliance_rules.toml version.json
git commit -m "Uyumluluk kuralları güncellendi: v2.2.0"
git push
```

4. Sahadaki bilgisayarlar bir sonraki denetimde güncellemeyi otomatik indirir.

> **Not:** GitHub'ın `raw.githubusercontent.com` adresleri herkese açıktır. Gizli tutmak istiyorsanız private repo kullanın ve Yöntem 3'e geçin.

---

### Yöntem 3: Kendi Sunucunuz ile Otomatik Dağıtım

Şirket içi ağda bir HTTP sunucusu (IIS, Nginx, Apache vb.) varsa bu yöntem tercih edilir.

#### Sunucuya Eklenecek Dosyalar

Sunucunuzda erişilebilir bir dizine iki dosya koyun:

`version.json`:
```json
{
  "version": "2.1.0",
  "url": "http://sunucu-ip/kurallar/compliance_rules.toml"
}
```

`compliance_rules.toml` — güncel kural dosyası.

#### `update_url`'i Ayarlayın

```toml
rules_version = "2.1.0"
update_url = "http://sunucu-ip/kurallar/version.json"
```

#### Güncelleme (Her seferinde)

1. `compliance_rules.toml`'daki `rules_version`'ı artırın
2. `version.json`'daki `version`'ı aynı değere güncelleyin
3. Her iki dosyayı sunucuya kopyalayın (FTP, SCP, ağ paylaşımı vb.)
4. Sahadaki bilgisayarlar bir sonraki denetimde güncellemeyi otomatik indirir

> **IIS için not:** `version.json` dosyasının MIME türünün `application/json` olarak tanımlı olduğundan emin olun. Tanımlı değilse IIS 404 döner.

---

### Yöntem Karşılaştırması

| | Elle Kopyalama | GitHub | Kendi Sunucu |
|-|----------------|--------|--------------|
| Kurulum zorluğu | Çok kolay | Kolay | Orta |
| Otomatik güncelleme | Hayır | Evet | Evet |
| İnternet gereksinimi | Hayır | Evet | Hayır (iç ağ) |
| Maliyet | Yok | Yok | Sunucu altyapısı |
| Gizlilik | Tam | Public repo = açık | Tam |
| Önerilen durum | Az cihaz / offline | Genel kullanım | Kurumsal / intranet |

---

### Manuel Kurulum (Otomatik Güncelleme Çalışmazsa)

Güncelle butonu internet bağlantısı gerektirdiğinden bazen çalışmayabilir. Bu durumda dosyayı elle kurabilirsiniz.

**1. Güncel dosyayı indirin:**

GitHub Releases sayfasına gidin:
```
https://github.com/KULLANICI/REPO/releases/latest
```
`compliance_rules.toml` dosyasını indirin.

**2. İndirilen dosyayı doğru konuma kopyalayın:**

| İşletim Sistemi | Konum |
|-----------------|-------|
| **Windows** | `C:\Users\<kullanıcı adı>\AppData\Roaming\omnicore\compliance_rules.toml` |
| **Linux** | `~/.local/share/omnicore/compliance_rules.toml` |

Mevcut dosyanın üzerine yapıştırın.

**3. Uygulamada Yenile butonuna tıklayın.**

---

### Versiyon Numarası Kuralı

Güncelleme mekanizması **major versiyon** farkına göre çalışır (`X`.y.z formatında ilk sayı):

- `2.0.0` → `2.5.0`: Fark 0 — denetim çalışır, uyarı yok
- `2.0.0` → `3.0.0`: Fark 1 — denetim çalışır, güncelleme önerilir
- `2.0.0` → `4.0.0`: Fark 2 — **denetim kilitlenir**, güncelleme zorunludur

Rutin kural düzenlemelerinde (alan ekleme, eşik değeri değiştirme) sadece minor veya patch versiyonu artırın:
```
2.0.0 → 2.1.0   (yeni kural eklendi)
2.1.0 → 2.1.1   (küçük düzeltme)
```

Major versiyonu yalnızca mevcut tüm kuralları sıfırlayan ya da format değişikliği içeren büyük güncellemelerde artırın.
