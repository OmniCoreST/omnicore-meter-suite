# CLAUDE.md — Omnicore Meter Suite

## Project Overview

**Omnicore Meter Suite** is a desktop application for reading, programming, and managing Turkish MASS-compliant electricity meters. Built with **Tauri 2 + Svelte**, the entire UI is in both **Turkish and English**. The primary goal is an extremely intuitive interface usable by field technicians of all computer literacy levels.

> **Design Philosophy:** Every interaction must feel obvious. If a user needs to think about what to do next, the design has failed. Visual feedback for every action, progress indication for every operation, and clear error messages for every failure.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Framework | Tauri 2 (Rust backend + Webview frontend) |
| Frontend | Svelte (SvelteKit NOT required — use Svelte standalone with Vite) |
| Styling | Tailwind CSS (with custom config below) |
| Icons | Material Symbols Outlined (Google Fonts) |
| Font | Spline Sans (Google Fonts) |
| Serial | Tauri serialport plugin (`tauri-plugin-serialport`) or custom Rust serial via `serialport` crate |
| Storage | Local JSON/SQLite for session history, reports, and settings |
| Charts | Chart.js or similar for load profile graphs |
| Language | Turkish — all UI labels, messages, tooltips, and errors |

---

## Design System

### Colors

```js
// tailwind.config.js extend.colors
{
  "primary": "#279EA7",        // Brand teal — buttons, active states, links
  "secondary": "#1F3244",      // Dark blue — sidebar bg in dark mode, card accents
  "background-light": "#f6f8f7",
  "background-dark": "#0f1821",
  "surface-dark": "#1F3244",
  "surface-light": "#ffffff",
}
```

Additional palette tokens used in the design template:
- **Border light:** `slate-200` / **Border dark:** `#334a5e`
- **Sidebar dark bg:** `#111c26`
- **Input dark bg:** `#1a2632`
- **Hover dark bg:** `#131d27`
- **Status green:** `emerald-500` (connected, success)
- **Status amber:** `amber-500` (warning, retry)
- **Status red:** `red-500` (error, disconnect)
- **Status blue:** `blue-500` (info, previous sessions)
- **Text primary dark:** `white` / **Text secondary dark:** `slate-400`
- **Text primary light:** `slate-900` / **Text secondary light:** `slate-500`

### Typography

```js
// tailwind.config.js extend.fontFamily
{ "display": ["Spline Sans", "sans-serif"] }
```

- **Body:** `text-sm` (14px) regular
- **Labels:** `text-[10px]` or `text-xs` bold uppercase tracking-wider
- **Headings:** `text-2xl` to `text-3xl` bold
- **Mono data:** `font-mono` for serial numbers, OBIS codes, hex values, COM ports

### Border Radius

```js
// tailwind.config.js extend.borderRadius
{ "DEFAULT": "1rem", "lg": "1.5rem", "xl": "2rem", "2xl": "3rem", "full": "9999px" }
```

### Icons

Use **Material Symbols Outlined** from Google Fonts. Key icons used:
- `dashboard` — Home/Ana Sayfa
- `electric_meter` — Meter/Sayaç
- `cable` / `usb` — Serial ports
- `lan` / `wifi_off` — Network interfaces
- `tune` — Parameters/Ayarlar
- `terminal` — Communication log
- `bar_chart` — Load profiles/Charts
- `description` — Reports
- `warning` — Alerts/Alarms
- `settings` — Settings
- `dark_mode` / `light_mode` — Theme toggle
- `notifications` — Notifications
- `replay` — Retry/Reconnect
- `download` — Export
- `visibility` — View
- `block` — Clear console
- `expand_less` / `expand_more` — Collapse/Expand

### Dark Mode

- Supported with `class` strategy (manual toggle)
- Apply `transition-colors duration-300` on major containers
- Persist preference in local storage

### Component Patterns (from design template)

**Cards:** `bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl shadow-sm`

**Active nav item:** `bg-primary/10 text-primary rounded-full`

**Hover nav item:** `hover:bg-slate-100 dark:hover:bg-[#334a5e] rounded-full`

**Buttons (primary):** `bg-primary text-white font-bold rounded-full px-6 py-3 hover:bg-primary/90 shadow-lg shadow-primary/20`

**Inputs:** `bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-xs focus:border-primary focus:ring-1 focus:ring-primary`

**Status badge (connected):** `bg-emerald-500/10 border border-emerald-500/20 text-emerald-500` with animated ping dot

**Status badge (disconnected):** `bg-red-500/10 border border-red-500/20 text-red-500`

**Glassmorphism header:** `bg-white/80 dark:bg-[#0f1821]/80 backdrop-blur-md`

**Glow effect on cards:** `absolute inset-0 bg-primary/20 blur-3xl rounded-3xl opacity-20`

---

## Application Architecture

### Layout Structure

```
┌─────────────────────────────────────────────────┐
│ [Sidebar Nav]  │  [Header Bar]                  │
│                │─────────────────────────────────│
│  OMNICORE      │  [Main Content Area]            │
│  MASS v2.4     │                                 │
│                │  Scrollable content area         │
│  ─────────     │  with max-w-7xl mx-auto         │
│  Ana Sayfa     │                                 │
│  Kısa Okuma    │                                 │
│  Tam Okuma     │                                 │
│  Yük Profili   │                                 │
│  Olaylar       │                                 │
│  Alarmlar      │                                 │
│  ─────────     │                                 │
│  AYARLAR       │                                 │
│  Saat Ayarı    │                                 │
│  Şifre Değiş.  │                                 │
│  Yaz Saati     │                                 │
│  Periyot Ayar. │                                 │
│  Tarife Ayar.  │                                 │
│                │                                 │
│  ─────────     │                                 │
│  [Status Card] │                                 │
│  Online/Offline│                                 │
│                ├─────────────────────────────────│
│                │ [Collapsible Communication Log] │
└─────────────────────────────────────────────────┘
```

### Sidebar Navigation (Left, w-72, fixed)

**Top section:**
- Logo + "OMNICORE" title + "MASS UTILITY v2.x" subtitle
- Nav items with Material icons, rounded-full hover states

**Nav Groups (Turkish labels):**

```
── ANA MENÜ ──
📊 Ana Sayfa              (dashboard)      → Home/Connection page
📖 Kısa Okuma             (menu_book)      → Short reading
📋 Tam Okuma              (assignment)     → Full reading
📈 Yük Profili            (bar_chart)      → Load profiles
📅 Olaylar                (event_note)     → Meter events
🔔 Alarmlar               (notifications)  → Meter alarms

── SAYAÇ AYARLARI ──
🕐 Saat Senkronizasyonu   (schedule)       → Time sync
🔑 Şifre Değiştir         (lock_reset)     → Password change
☀️ Yaz Saati Ayarı        (wb_sunny)       → DST settings
⏱️ Periyot Ayarları       (timer)          → Demand/LP periods
💰 Tarife Ayarları        (payments)       → Tariff settings
```

**Bottom section:**
- System status card (gradient dark bg, pulse dot for online status)
- No user login — just show "Sistem Durumu: Çevrimiçi" or "Çevrimdışı"

### Header Bar

- Page title (dynamic based on current view)
- Connection status badge with animated pulse (Bağlı / Bağlı Değil)
- Dark/light mode toggle
- Notification bell (for alarm events)
- No language toggle (Turkish only)

### Communication Log Panel (Bottom, collapsible)

Always present at the bottom of the screen. Uses `<details>` element with smooth transition.

**Header row:**
- Terminal icon + "Haberleşme Logu" title
- Live monitoring indicator (green dot + "Canlı İzleme")
- Clear console button, Export log button
- Collapse/Expand chevron

**Log table (grid layout):**
```
[Zaman]    [Tür]      [Detay]
[14:30:01] INFO       Seri bağlantı başlatılıyor COM3...
[14:30:02] INFO       Port başarıyla açıldı. Baud: 9600
[14:30:02] UYARI      Cihaz el sıkışması gecikti (Deneme 1/3)
[14:30:05] BAŞARILI   Cihaz el sıkışması onaylandı
[14:30:06] TX         /?MKS123456789!\r\n
[14:30:07] RX         /MKS6<2>ADM(M550.2251)\r\n
[14:30:08] HATA       Zaman aşımı: 2000ms içinde cevap alınamadı
```

**Log type colors:**
- `INFO` → `text-blue-600 dark:text-blue-400`
- `UYARI` → `text-amber-600 dark:text-amber-500`
- `BAŞARILI` → `text-emerald-600 dark:text-emerald-500`
- `HATA` → `text-red-600 dark:text-red-500`
- `TX` → `text-violet-600 dark:text-violet-400`
- `RX` → `text-emerald-600 dark:text-emerald-500`

Font: `font-mono text-xs` for the entire log area. Show raw bytes/ASCII for TX/RX.

---

## Pages & Features

### 1. Ana Sayfa (Home / Connection Dashboard)

This is the landing page. It combines connection setup with session history.

**Quick Connect Card** (hero section, full width):
- Gradient background with subtle mesh pattern
- "Hızlı Bağlantı" heading
- Status badge: "Bağlanmaya Hazır" (ready) / "Bağlı" (connected)
- Last connected meter serial shown

**Connection Parameters Panel** (inside Quick Connect card):
- **Bağlantı Türü** (Connection Type): Dropdown → `Optik Prob (IEC 62056-21)`, `RS485 Direkt`
- **COM Port**: Dropdown → Auto-detected serial ports (COM1, COM2... or /dev/ttyUSB0...)
- **Baud Hızı** (Baud Rate): Dropdown → `300 (Mod C Başlangıç)`, `600`, `1200`, `2400`, `4800`, `9600`, `19200`
  - Note: IEC 62056-21 Mode C always starts at 300 baud then negotiates up
- **Zaman Aşımı (ms)** (Timeout): Number input, default `2000`
- **Sayaç Adresi** (Meter Address): Text input, optional (for `/?ADDRESS!\r\n` request)
- **Şifre** (Password): Password input for programming mode (8 digits)
- "Gelişmiş Ayarlar" (Advanced Settings) toggle link:
  - Data bits: 7 (fixed for IEC 62056-21)
  - Parity: Even (fixed)
  - Stop bits: 1 (fixed)
  - Max retries: 3

**Port Selection Visual** (from design template):
- Show detected COM ports as clickable cards
- Active port: primary border with glow dot + baud rate display
- Idle ports: subtle border, hover highlight
- Show port description (USB Serial, etc.)

**Bağlan Button:** Large, primary colored, centered: `"🔌 Bağlan"` / `"⏹️ Bağlantıyı Kes"`

**Previous Sessions** (left column below):
- List of previous meter readings with:
  - Meter icon (blue for success, amber for failure)
  - Meter serial number
  - Date/time + result ("Okuma Başarılı", "Bağlantı Başarısız", "Yapılandırıldı")
  - Reconnect/Retry button on hover

**Recent Reports** (right column below):
- List of exported files with:
  - File icon
  - Filename + size + date
  - View and Download buttons

### 2. Kısa Okuma (Short Meter Reading)

Reads the "Kısa Okuma Paketi (6)" — essential meter data.

**Pre-read UI:**
- "Kısa Okuma Başlat" button (only enabled when connected)
- Brief explanation: "Sayacın temel enerji ve durum bilgilerini okur"

**Progress Bar** (during read):
- Multi-step progress with descriptive labels:
  1. "El sıkışma başlatılıyor..." (Handshake)
  2. "Baud hızı değiştiriliyor..." (Baud switch)
  3. "Şifre gönderiliyor..." (Password if needed)
  4. "Kısa okuma paketi isteniyor..." (Requesting packet 6)
  5. "Veriler alınıyor..." (Receiving data)
  6. "Veriler işleniyor..." (Processing)
  7. "Tamamlandı ✓" (Complete)
- Animated progress bar with percentage
- Current step highlighted, completed steps show checkmark
- Elapsed time display

**Results Display** (after successful read):

Organized in visual card groups:

**Sayaç Kimliği (Meter Identity):**
| Label | OBIS | Value |
|---|---|---|
| Seri Numarası | 0.0.0 | 123456789 |
| Program Versiyonu | 0.2.0 | V01.00 |
| Üretim Tarihi | 96.1.3 | 2024-06-30 |
| Kalibrasyon Tarihi | 96.2.5 | 2024-06-30 |

**Tarih/Saat (Date/Time):**
| Label | OBIS | Value |
|---|---|---|
| Sayaç Tarihi | 0.9.2 | 2024-12-15 |
| Sayaç Saati | 0.9.1 | 14:30:35 |
| Haftanın Günü | 0.9.5 | 4 (Perşembe) |

**Aktif Enerji (+) (Active Energy Import):**
| Label | OBIS | Value |
|---|---|---|
| Toplam | 1.8.0 | 123456.789 kWh |
| T1 Gündüz | 1.8.1 | xxx.xxx kWh |
| T2 Puant | 1.8.2 | xxx.xxx kWh |
| T3 Gece | 1.8.3 | xxx.xxx kWh |
| T4 | 1.8.4 | xxx.xxx kWh |

**Aktif Enerji (−) (Active Energy Export):** (only for bidirectional meters)
| Label | OBIS | Value |
|---|---|---|
| Toplam | 2.8.0 | xxx.xxx kWh |
| T1-T4 | 2.8.1-4 | xxx.xxx kWh |

**Reaktif Enerji (Reactive Energy):** (only for Kombi meters)
| Label | OBIS | Value |
|---|---|---|
| Endüktif (+) | 5.8.0 | xxx.xxx kVARh |
| Kapasitif (+) | 6.8.0 | xxx.xxx kVARh |
| Endüktif (−) | 7.8.0 | xxx.xxx kVARh |
| Kapasitif (−) | 8.8.0 | xxx.xxx kVARh |

**Demant (Maximum Demand):**
| Label | OBIS | Value |
|---|---|---|
| Maks. Aktif Güç (+) | 1.6.0 | 123.456 kW @ 2024-02-01 13:30 |
| Maks. Aktif Güç (−) | 2.6.0 | xxx.xxx kW @ date |

**Anlık Değerler (Instantaneous Values):**
| Label | OBIS | Value |
|---|---|---|
| Vrms L1 | 32.7.0 | 220.5 V |
| Vrms L2 | 52.7.0 | 220.5 V |
| Vrms L3 | 72.7.0 | 220.5 V |
| Irms L1 | 31.7.0 | 16.5 A |
| Irms L2 | 51.7.0 | 16.5 A |
| Irms L3 | 71.7.0 | 16.5 A |
| Frekans | 14.7.0 | 49.9 Hz |
| Cos φ L1 | 33.7.0 | 0.97 |
| Cos φ L2 | 53.7.0 | 0.97 |
| Cos φ L3 | 73.7.0 | 0.97 |

**Durum Kodları (Status Codes):**
| Label | OBIS | Value |
|---|---|---|
| FF Hata/Durum Kodu | F.F.0 | 00000000 (decoded bits shown) |
| GF Coğrafi Durum Kodu | F.F.1 | 00000000 (decoded fields shown) |
| Pil Durumu | 96.6.1 | Dolu / Zayıf |
| Röle Durumu | 96.3.10 | Aktif / Pasif |

**Action buttons:** "Dışa Aktar" (Export to CSV/PDF), "Yeniden Oku" (Re-read), "Rapor Oluştur" (Generate report)

### 3. Tam Okuma (Full Meter Reading)

Reads ALL available packets: Kısa Okuma (6), Geçmiş Bilgiler (7), Uyarı (8), Kesinti Kayıtları (9), and Teknik Kalite (5).

**Same progress bar system** but with more steps and longer duration.

**Results organized in tabs:**
- Tab 1: Güncel Veriler (Current Data) — same as Kısa Okuma results
- Tab 2: Geçmiş Bilgiler (Historical Data) — 12 months of:
  - Monthly energy by tariff (1.8.1*1 through 1.8.4*12, 2.8.1*1 through 2.8.4*12)
  - Monthly reactive energy (5.8.0*1-12, 6.8.0*1-12, 7.8.0*1-12, 8.8.0*1-12)
  - Monthly demand values with timestamps (1.6.0*1-12, 2.6.0*1-12)
  - Demand reset dates (0.1.2*1-12)
  - Monthly terminal cover openings (96.71*1-12)
  - Tariff schedule history
  - Tariff change dates (96.2.2*1-10)
- Tab 3: Uyarılar (Alerts) — 
  - Voltage warnings (96.7.4, 96.77.4*1-10)
  - Current warnings (96.7.5, 96.77.5*1-10)
  - Magnetic field warnings (96.7.6, 96.77.6*1-10)
  - Cover opening (96.70, 96.71)
  - Battery status (96.6.1)
  - DST settings (96.90.0-12)
- Tab 4: Kesinti Kayıtları (Outage Records) —
  - Three-phase long outages (96.7.0, 96.77.0*1-99)
  - Three-phase short outages (96.7.00, 96.77.00*1-99)
  - Per-phase long/short outages for L1, L2, L3
- Tab 5: Teknik Kalite (Technical Quality) — Packet 5 parameters

### 4. Yük Profili (Load Profile)

**Profile Selection:**
- Radio buttons: Yük Profili 1, Yük Profili 2 (Kombi only), Yük Profili 3 (Kombi only)
- Show profile content description (from 97.1.0, 97.2.0, 97.3.0 OBIS codes)
- Date range picker: Start date/time, End date/time
- "Tümünü Oku" (Read All) checkbox for entire profile
- Load profile period display: "Kayıt Periyodu: 15 dk" (from 0.8.4)

**Query format:**
- Date range: `<SOH>R2<STX>P.01(yy-mm-dd,hh:mm;yy-mm-dd,hh:mm)<ETX><BCC>`
- All data: `<SOH>R2<STX>P.01(;)<ETX><BCC>`

**Profile 1 contains** (varies by meter type — see Ek-E):
- Single Phase Uni: Total Active Energy, Active Power, Max Vrms L1, Min Vrms L1
- Single Phase Bi: + Reverse Active Energy, Reverse Power
- Three Phase Uni: + Vrms L2/L3 max/min
- Three Phase Bi: + Reverse values
- Kombi Uni: Active, Inductive, Capacitive, Active Power
- Kombi Bi: + Reverse of all

**Profile 2** (Kombi only): Tariff-specific energy T1-T3 (and reverse for bidirectional)

**Profile 3** (Kombi only): Vrms L1-L3 max/min, Irms L1-L3, Frequency, Avg Cos φ

**Graphical Display:**
- Line chart with time axis (x) and value axis (y)
- Multiple series toggle (energy, power, voltage, etc.)
- Zoom and pan controls
- Hover tooltip with exact values
- Color-coded lines per parameter
- Vertical lines for outage periods

**Data Table:**
- Sortable table below chart
- Columns: Tarih/Saat, then one column per profile parameter
- Highlight anomalies (missing periods, outage markers)

**Export:** CSV, PDF (with graph), JSON

### 5. Olaylar (Meter Events)

Display parsed event records from Uyarı Paketi (8) and Kesinti Kayıtları Paketi (9).

**Event Categories (filterable):**
- Gerilim Uyarıları (Voltage Warnings) — 96.7.4, 96.77.4*
- Akım Uyarıları (Current Warnings) — 96.7.5, 96.77.5*
- Manyetik Alan (Magnetic Field) — 96.7.6, 96.77.6*
- Üst Kapak Açılma (Top Cover Open) — 96.70
- Klemens Kapağı (Terminal Cover) — 96.71
- Tarife Değişikliği (Tariff Change) — 96.2.2
- Demant Sıfırlama (Demand Reset) — 0.1.2

**Event Table:**
| # | Tür | Başlangıç | Bitiş | Süre | Detay |
|---|---|---|---|---|---|
| 1 | Gerilim Uyarısı | 2024-06-30 13:30 | 2024-06-30 13:35 | 5 dk | Faz sırası hatası |

- Color-coded rows by event type
- Filter by type, date range
- Search functionality
- Export to CSV

### 6. Alarmlar (Meter Alarms)

**FF Hata/Durum Kodları (F.F.0):**

Display as a visual bitfield. 64-bit value decoded into human-readable cards.

Each bit gets a card showing:
- Bit number and name
- Current state (green = OK, red = Active)
- Brief description

**FF Bits (all 64 — from Ek-C):**
```
Bit 0:  Saat Hatası (RTC Error)
Bit 1:  Ölçüm Entegresi Arızası (Measurement IC Fault)
Bit 2:  Kritik Ölçüm Hatası (Critical Measurement Error)
Bit 3:  RS485 Port Hatası (RS485 Port Error)
Bit 4:  Kalibrasyon Durumu (0=Done, 1=Not Done)
Bit 5:  Klemens Kapağı Açık - Fiziksel (Terminal Cover Open)
Bit 6:  Üst Kapak Açık - Fiziksel (Top Cover Open) [URGENT]
Bit 7:  Üst Kapak Açılma Bilgisi Mevcut (Top Cover Open History)
Bit 8:  Akım Var Gerilim Yok R Fazı (Current w/o Voltage R) [URGENT]
Bit 9:  Akım Var Gerilim Yok S Fazı [URGENT]
Bit 10: Akım Var Gerilim Yok T Fazı [URGENT]
Bit 11: Manyetik+Akım+Cos+Gerilim R (Magnetic Tampering R) [URGENT]
Bit 12: Manyetik+Akım+Cos+Gerilim S [URGENT]
Bit 13: Manyetik+Akım+Cos+Gerilim T [URGENT]
Bit 14: T1 Endeks Durma R (T1 Index Stuck) [URGENT]
Bit 15: T2 Endeks Durma [URGENT]
Bit 16: T3 Endeks Durma [URGENT]
Bit 17: R Fazı Endeks İlerleyişi Sıfır (R Phase No Progress)
Bit 18: S Fazı Endeks İlerleyişi Sıfır
Bit 19: T Fazı Endeks İlerleyişi Sıfır
Bit 20: R Faz Kesilmesi Devam Ediyor (R Phase Outage Active)
Bit 21: S Faz Kesilmesi Devam Ediyor
Bit 22: T Faz Kesilmesi Devam Ediyor
Bit 23: 3 Faz Kesilmesi Devam Ediyor (3-Phase Outage Active)
Bit 24: Akım Hata Uyarısı Devam (Current Error Active)
Bit 25: Gerilim Hata Uyarısı Devam (Voltage Error Active)
Bit 26: Aktif Endekslerde Gerileme (Active Index Regression)
Bit 27: Reaktif Endekslerde Gerileme
Bit 28: Kapasitif Endekslerde Gerileme
Bit 29: Demant Var Endeks İlerlemiyor (Demand w/o Index Progress)
Bit 30: T0 ile T1+T2+T3+T4 Farkı > 200W (Tariff Sum Mismatch)
Bit 31: T4'de Endeks Var (T4 Has Index)
Bit 32: Tarife Dilimleri Arızalı (Tariff Slots Faulty)
Bit 33: Tarife Değişiklik Yılı ≠ Üretim Yılı
Bit 34: Üretim Yılı ≠ Kalibrasyon Yılı
Bit 35: 3 Aydır Sabit Demant + Gerilim Var
Bit 36: İki Hafıza Bölgesinde Eşzamanlı Hata
Bit 37: Sistem Pili Zayıf (0=Low, 1=OK)
Bit 38: Zaman Saati Pili Zayıf (0=Low, 1=OK)
Bit 39: R Fazı Saatte 20+ Kesinti (R Phase 20+ Outages/Hour)
Bit 40: S Fazı Saatte 20+ Kesinti
Bit 41: T Fazı Saatte 20+ Kesinti
Bit 42: Saatte 20+ Akım Uyarısı (20+ Current Warnings/Hour)
Bit 43: Saatte 20+ Gerilim Uyarısı
Bit 44: Yüksek Demant (MF>20kW, TF>60kW)
Bit 45: R Yüksek Gerilim >253V (10sn)
Bit 46: S Yüksek Gerilim >253V (10sn)
Bit 47: T Yüksek Gerilim >253V (10sn)
Bit 48: R Düşük Gerilim <195.5V (10sn)
Bit 49: S Düşük Gerilim <195.5V (10sn)
Bit 50: T Düşük Gerilim <195.5V (10sn)
Bit 51: R Yüksek Akım >Imax+10% (60sn)
Bit 52: S Yüksek Akım >Imax+10% (60sn)
Bit 53: T Yüksek Akım >Imax+10% (60sn)
Bit 54: Faz-Nötr Akım Dengesizliği [URGENT]
Bit 55: Kesme-Açma Rölesi Arızalı
Bit 56-63: Rezerve (Reserved)
```

**GF Coğrafi Durum Kodları (F.F.1):**

Decoded as structured fields:
```
Bits 0-4:   EDAŞ ID (5 bit, 0-31) — map to company name from table
Bits 5-19:  Trafo Merkez ID (15 bit, 0-32767)
Bits 20-23: Trafo ID (4 bit, 0-15)
Bits 24-29: Depar ID (6 bit, 0-63)
Bits 30-31: Faz ID (2 bit: 1=R, 2=S, 3=T)
Bits 32-33: Kol ID (2 bit, 0-3)
Bits 34-43: Maksimum Akım (10 bit, 0-1023)
Bits 44-63: Rezerve
```

**EDAŞ ID Table:**
```
01: AKDENİZ EDAŞ (AKD)     08: ÇAMLIBEL EDAŞ (CMB)    15: OSMANGAZİ EDAŞ (OED)
02: AKEDAŞ (AKE)            09: ÇORUH EDAŞ (CRH)       16: SAKARYA EDAŞ (SED)
03: ARAS EDAŞ (ARS)         10: DİCLE EDAŞ (DCL)       17: TOROSLAR EDAŞ (TRS)
04: AYDEM (ADM)             11: FIRAT EDAŞ (FRT)       18: TRAKYA EDAŞ (TRK)
05: AYEDAŞ (AYE)            12: GEDİZ EDAŞ (GDZ)       19: ULUDAĞ EDAŞ (UED)
06: BAŞKENT EDAŞ (BSK)      13: KCETAŞ (KCE)           20: VANGÖLÜ EDAŞ (VAN)
07: BOĞAZİÇİ EDAŞ (BGZ)    14: MERAM EDAŞ (MER)       21: YEŞİLIRMAK EDAŞ (YED)
```

**Visual display:** Show as info cards with decoded values and colored severity indicators.

### 7. Kesinti Kayıtları (Outage Records)

Part of Olaylar or a sub-tab within full reading. Shows detailed outage records from Packet 9.

**Categories:**
- Üç Faz Uzun Kesinti (3-Phase Long: 96.7.0, 96.77.0*1-99)
- Üç Faz Kısa Kesinti (3-Phase Short: 96.7.00, 96.77.00*1-99)
- 1. Faz Uzun/Kısa Kesinti (L1: 96.7.1/96.7.10, 96.77.1*/96.77.10*)
- 2. Faz Uzun/Kısa Kesinti (L2: 96.7.2/96.7.20, 96.77.2*/96.77.20*)
- 3. Faz Uzun/Kısa Kesinti (L3: 96.7.3/96.7.30, 96.77.3*/96.77.30*)

**Threshold:** ≥180 seconds = Uzun (Long), <180 seconds = Kısa (Short)

**Display:** Sortable table with start/end timestamps, duration calculation, type badge.

---

## Meter Settings Pages

All settings require the meter to be connected AND in programming mode (password verified).

### 8. Saat Senkronizasyonu (Time Sync)

**Current meter time display** (read from 0.9.1, 0.9.2, 0.9.5):
- Show meter time vs. computer time
- Show drift in seconds

**Sync Options:**
- "Bilgisayar Saatine Senkronize Et" — sync to PC time
- Manual entry: Date picker + Time picker
- Day of week auto-calculated

**Write commands:**
```
W2 STX 0.9.1(HH:MM:SS)   — Set time
W2 STX 0.9.2(YY-MM-DD)   — Set date
W2 STX 0.9.5(N)           — Set day of week (1=Monday, 7=Sunday)
```

**Visual:** Side-by-side clock display (Sayaç Saati / Bilgisayar Saati) with difference highlighted.

### 9. Şifre Değiştir (Password Change)

**Current password display:** Masked (only visible in programming mode via 96.96)

**Change form:**
- Mevcut Şifre (Current Password): 8-digit input
- Yeni Şifre (New Password): 8-digit input
- Yeni Şifre Tekrar (Confirm): 8-digit input
- Validation: must be exactly 8 digits

**Write command:**
```
W2 STX 96.96(12345678)    — Set new password
```

**Security notes to display:**
- 3 wrong attempts = 6 hour lockout
- Password is per-meter (derived from serial number algorithm by manufacturer)
- Password OBIS code (96.96) only readable in programming mode

### 10. Yaz Saati Ayarı (DST Settings)

**DST Status:**
- Read from 96.90.0: Aktif (1) / Pasif (0)
- Toggle switch to enable/disable

**DST Periods** (up to 12):
- Each period has:
  - Period number (1-12)
  - Time offset: ±HH:MM (typically +01:00)
  - Forward date/time: YY-MM-DD,HH:MM (spring, e.g., 2024-03-31,03:00)
  - Backward date/time: YY-MM-DD,HH:MM (fall, e.g., 2024-10-27,04:00)

**Write commands:**
```
W2 STX 96.90.0(0)         — Disable DST
W2 STX 96.90.0(1)         — Enable DST
W2 STX 96.90.1(+01:00,YY-MM-DD,HH:MM;YY-MM-DD,HH:MM)  — Period 1
W2 STX 96.90.2(...)        — Period 2
...
W2 STX 96.90.12(...)       — Period 12
```

**UI:** Table with 12 rows, each showing period dates, editable inline. "Tümünü Kaydet" save button.

### 11. Periyot Ayarları (Demand & Load Profile Period Settings)

**Demand Period (0.8.0):**
- Dropdown: 15 / 30 / 60 dakika (minutes)
- Default: 15
- Command: `W2 STX 0.8.0(15*min)`

**Load Profile Period (0.8.4):**
- Dropdown: 15 / 30 / 60 dakika
- Default: 15
- Command: `W2 STX 0.8.4(15*min)`

**Short/Long Outage Threshold (0.9.9):**
- Number input, default: 180 seconds
- ≥ threshold = Long outage, < threshold = Short outage
- Command: `W2 STX 0.9.9(180*sec)`

**Manual Demand Reset:**
- Button: "Demant Sıfırla"
- Command: `E2 STX 1.6.0()`
- Confirm dialog: "Demant değeri sıfırlanacaktır. Devam etmek istiyor musunuz?"

### 12. Tarife Ayarları (Tariff Settings)

**Default tariff configuration:**
```
T1 – Gündüz  : 06:00 – 17:00
T2 – Puant   : 17:00 – 22:00
T3 – Gece    : 22:00 – 06:00
T4 – (unused, 9999 = disabled)
```

**Schedule configuration (3 separate schedules):**

For each of: Hafta İçi (Weekdays) / Cumartesi (Saturday) / Pazar (Sunday):

**Time Slots** (up to 8 per day):
- Visual timeline/slider from 00:00 to 24:00
- Drag handles to set tariff boundaries
- Color-coded zones: T1=blue, T2=orange, T3=purple, T4=green

**OBIS Format:**
Tariff times stored as: `HHMM HHMM HHMM HHMM HHMM HHMM HHMM HHMM` (8 slots × 4 chars = 32 chars)
- `9999` means unused/disabled
- Example: `060017002200999999999999999999999` → 3 active zones

```
96.50 — Weekday times:    e.g., (060017002200999999999999999999999)
96.51 — Saturday times
96.52 — Sunday times
96.60 — Weekday zones:    e.g., (12340000) — position maps to tariff number
96.61 — Saturday zones
96.62 — Sunday zones
```

**Zone assignment format (96.60/61/62):**
8 digits, each representing which tariff (1-4) applies to that time slot. `0` = unused.
- Example: `31230000` → Slot1=T3, Slot2=T1, Slot3=T2, Slot4=T3, rest unused

**Write commands:**
```
W2 STX 96.50(0600170022009999999999999999999)
W2 STX 96.60(12300000)
... etc.
```

**Visual editor:** Interactive 24-hour timeline with colored blocks. Click to assign tariff to time range. Preview table shows the final configuration before writing.

---

## Communication Protocol Implementation

### IEC 62056-21 Mode C Handshake Sequence

```
STEP 1: Request Message (300 baud, 7E1)
  TX → /?ADDRESS!\r\n          (or /?!\r\n if no address)
  
STEP 2: Identification Message (300 baud)
  RX ← /XXXZ<generation>YYYYY(MODEL)\r\n
  
  Where:
    XXX = Flag code (e.g., "MKS" for manufacturer)
    Z   = Baud rate char: 0=300, 1=600, 2=1200, 3=2400, 4=4800, 5=9600, 6=19200
    <generation> = Protocol generation (e.g., <2>)
    YYYYY = EDAŞ ID (e.g., "ADM")
    (MODEL) = Meter model (e.g., "(M550.2251)")

STEP 3: Acknowledgment (300 baud)
  TX → ACK V Z Y \r\n
  
  Where:
    V = Protocol mode: 0=readout, 1=programming, 5=tech quality, 6=short read, 7=history, 8=warnings, 9=outages
    Z = Baud rate (same as received or negotiated)
    Y = Mode identifier

STEP 4: Baud Rate Switch
  Both sides switch to negotiated baud rate (still 7E1)
  Wait 250-1250ms for switch

STEP 5a: Data Readout (Mode 0, 5, 6, 7, 8, 9)
  RX ← STX <data block> ! \r\n ETX BCC
  
STEP 5b: Programming Mode (Mode 1)
  TX → SOH P1 STX (PASSWORD) ETX BCC    — Send password
  RX ← ACK                                — Password accepted
  
  Then read/write individual OBIS codes:
  TX → SOH R2 STX OBIS() ETX BCC         — Read specific OBIS
  RX ← STX OBIS(value) ETX BCC           — Response
  
  TX → SOH W2 STX OBIS(value) ETX BCC    — Write specific OBIS
  RX ← ACK                                — Write confirmed

STEP 6: Session End
  TX → SOH B0 ETX BCC                     — Break/logout
  RX ← ACK
```

### Baud Rate Character Map

| Char | Baud Rate |
|------|-----------|
| 0 | 300 |
| 1 | 600 |
| 2 | 1200 |
| 3 | 2400 |
| 4 | 4800 |
| 5 | 9600 |
| 6 | 19200 |

### Control Characters

| Char | Hex | Description |
|------|-----|-------------|
| SOH | 0x01 | Start of header |
| STX | 0x02 | Start of text |
| ETX | 0x03 | End of text |
| ACK | 0x06 | Acknowledge |
| NAK | 0x15 | Not acknowledged |
| CR | 0x0D | Carriage return |
| LF | 0x0A | Line feed |
| BCC | - | Block Check Character (XOR of bytes from after SOH/STX to and including ETX) |

### Communication Timing

| Parameter | Duration |
|---|---|
| Response after request (tr) | 250ms ≤ tr ≤ 1250ms |
| Baud rate switch delay | 250-1250ms |
| Default timeout | 2000ms |
| Retry attempts | 3 |
| Lockout after 3 wrong passwords | 6 hours |

---

## Progress Bar System

The progress bar is a critical UX element. It must feel informative and reassuring.

### Visual Design

```
┌─────────────────────────────────────────────────────────┐
│  📖 Kısa Okuma İşlemi                          %45     │
│  ═══════════════════════════░░░░░░░░░░░░░░░░░░░   2.3s  │
│                                                         │
│  ✅ El sıkışma başlatıldı                               │
│  ✅ Cihaz tanımlandı: MKS — ADM (M550.2251)             │
│  ✅ Baud hızı 9600'e yükseltildi                        │
│  🔄 Kısa okuma paketi alınıyor...                       │
│  ○  Veriler işleniyor                                   │
│  ○  Tamamlandı                                          │
└─────────────────────────────────────────────────────────┘
```

- Animated gradient bar (`bg-gradient-to-r from-primary to-emerald-400`)
- Percentage label on right
- Elapsed time bottom-right
- Step list: ✅ completed (green), 🔄 in progress (animated), ○ pending (gray)
- Smooth width transition on bar
- Cancel button ("İptal") available during operation

### Progress Steps Per Operation

**Kısa Okuma (Short Read):**
1. Seri port açılıyor... (Opening serial port)
2. El sıkışma gönderiliyor... (Sending handshake)
3. Cihaz tanımlanıyor... (Identifying device)
4. Baud hızı değiştiriliyor... (Switching baud rate)
5. Kısa okuma paketi isteniyor... (Requesting short read packet)
6. Veriler alınıyor... (Receiving data)
7. Veriler ayrıştırılıyor... (Parsing data)
8. Tamamlandı ✓ (Complete)

**Tam Okuma (Full Read):**
Same first 4 steps, then:
5. Uzun okuma paketi alınıyor... (Full readout)
6. Geçmiş bilgiler alınıyor... (Historical data — Packet 7)
7. Uyarı bilgileri alınıyor... (Warning data — Packet 8)
8. Kesinti kayıtları alınıyor... (Outage records — Packet 9)
9. Veriler işleniyor... (Processing)
10. Tamamlandı ✓

**Yük Profili Okuma (Load Profile Read):**
Steps 1-4, then:
5. Yük profili sorgulanıyor... (Querying load profile P.01/P.02/P.03)
6. Veri blokları alınıyor... (Receiving data blocks — show X/Y progress)
7. Grafik oluşturuluyor... (Generating chart)
8. Tamamlandı ✓

**Settings Write:**
Steps 1-4, then:
5. Şifre doğrulanıyor... (Verifying password)
6. Parametre yazılıyor: [OBIS code]... (Writing parameter)
7. Doğrulama okunuyor... (Read-back verification)
8. Tamamlandı ✓

---

## Data Storage & Export

### Local Storage (SQLite or JSON files)

**Sessions table:**
- id, meter_serial, meter_model, timestamp, connection_type, result_status, raw_data_json

**Reports table:**
- id, session_id, report_type, filename, filepath, created_at

**Settings table:**
- key, value (for app preferences: last COM port, baud, theme, etc.)

### Export Formats

**CSV:** Standard comma-separated with UTF-8 BOM for Turkish character support. One row per OBIS code or per load profile interval.

**PDF:** Formatted report with:
- Header: Omnicore logo, report title, date
- Meter info section
- Data tables
- Load profile charts (embedded)
- Footer: page numbers

**JSON:** Raw structured data for integration with other systems.

---

## Error Handling

### Connection Errors (Turkish messages)

| Error | Turkish Message |
|---|---|
| Port not found | "COM port bulunamadı. Lütfen bağlantıyı kontrol edin." |
| Port busy | "COM port başka bir uygulama tarafından kullanılıyor." |
| No response | "Sayaçtan cevap alınamadı. Optik probu kontrol edin." |
| Handshake failed | "El sıkışma başarısız oldu. Sayaç bağlantısını kontrol edin." |
| Password wrong | "Şifre hatalı. Kalan deneme: X (3 hatada 6 saat kilitleme!)" |
| Timeout | "Zaman aşımı: {X}ms içinde cevap alınamadı." |
| BCC mismatch | "Veri doğrulama hatası (BCC). Tekrar denenecek..." |
| Write failed | "Yazma işlemi başarısız. Sayaç programlama modunda mı?" |
| Port lost | "Seri port bağlantısı kesildi. USB kabloyu kontrol edin." |

### Error Display

- Toast notification (top-right) for non-critical warnings
- Modal dialog for critical errors (with Retry / Cancel buttons)
- Communication log always records full error detail
- Red highlight on progress bar step that failed

---

## Tauri Backend Commands

### Rust Command Interface

```rust
// Serial port management
#[tauri::command] fn list_serial_ports() -> Vec<PortInfo>
#[tauri::command] fn open_port(port: String, baud: u32) -> Result<(), String>
#[tauri::command] fn close_port() -> Result<(), String>
#[tauri::command] fn set_baud_rate(baud: u32) -> Result<(), String>

// IEC 62056-21 protocol
#[tauri::command] fn handshake(address: Option<String>) -> Result<MeterIdentity, String>
#[tauri::command] fn authenticate(password: String) -> Result<bool, String>
#[tauri::command] fn read_short() -> Result<ShortReadData, String>
#[tauri::command] fn read_full() -> Result<FullReadData, String>
#[tauri::command] fn read_load_profile(profile: u8, start: Option<String>, end: Option<String>) -> Result<LoadProfileData, String>
#[tauri::command] fn read_obis(code: String) -> Result<String, String>
#[tauri::command] fn write_obis(code: String, value: String) -> Result<(), String>
#[tauri::command] fn end_session() -> Result<(), String>

// Event emission for progress
// Use tauri::Emitter to emit events like:
// "progress" -> { step: number, total: number, message: String }
// "log" -> { timestamp, type, message }
// "rx_data" -> raw received data
// "tx_data" -> raw transmitted data
```

### Svelte Frontend Event Listeners

```svelte
<script>
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  
  let progress = { step: 0, total: 0, message: '' };
  let logs = [];
  
  listen('progress', (event) => { progress = event.payload; });
  listen('log', (event) => { logs = [...logs, event.payload]; });
</script>
```

---

## Meter Type Detection

After handshake, detect meter type from identification message:

```
/MKS6<2>ADM(M550.2251)
         ^^^  ^^^^^^^^^^
         |    Model string
         EDAŞ ID
```

The model string determines meter type and available features:
- **Tek Fazlı (Single Phase):** Packets 6, 7, 8, 9 only. Profile 1 only.
- **Üç Fazlı (Three Phase):** Same as single phase but with L2, L3 values.
- **Kombi (Active-Reactive):** All packets. Profiles 1, 2, 3 available.
- **Tek Yönlü (Unidirectional):** No reverse energy registers (2.8.x, 7.8.0, 8.8.0).
- **Çift Yönlü (Bidirectional):** All registers available.

**UI should adapt:** Hide unavailable fields/tabs based on detected meter type.

---

## Relay Control (Optional Feature, for meters with disconnect relay)

**Read relay status:** `96.3.10` → 0=Passive, 1=Active
**Write relay command:** `W2 STX 96.3.10(0)` to cut / `W2 STX 96.3.10(1)` to connect

Display with prominent ON/OFF toggle, requiring confirmation dialog:
"⚠️ DİKKAT: Bu işlem aboneye giden enerjiyi kesecektir. Devam etmek istiyor musunuz?"

---

## Feedback Activation (F.A.0)

Controls which FF status changes trigger modem notifications:
- `F.A.0(0000000000000000)` — 16 hex chars = 64 bits
- Each bit corresponds to an FF code bit
- `1` = active feedback, `0` = passive

Write: `W2 STX F.A.0(XXXXXXXXXXXXXXXX)`

---

## File Structure (Recommended)

```
omnicore-meter-suite/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── serial/
│   │   │   ├── mod.rs           — Serial port management
│   │   │   ├── iec62056.rs      — IEC 62056-21 Mode C protocol
│   │   │   └── parser.rs        — OBIS data parsing
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs    — Port open/close/list
│   │   │   ├── reading.rs       — Short/Full/Profile reads
│   │   │   └── programming.rs   — Settings write operations
│   │   └── storage/
│   │       ├── mod.rs
│   │       └── database.rs      — SQLite session/report storage
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── lib/
│   │   ├── stores/
│   │   │   ├── connection.js    — Connection state store
│   │   │   ├── meter.js         — Current meter data store
│   │   │   ├── logs.js          — Communication log store
│   │   │   └── progress.js      — Progress bar state store
│   │   ├── components/
│   │   │   ├── layout/
│   │   │   │   ├── Sidebar.svelte
│   │   │   │   ├── Header.svelte
│   │   │   │   └── CommLog.svelte        — Bottom log panel
│   │   │   ├── common/
│   │   │   │   ├── ProgressBar.svelte
│   │   │   │   ├── StatusBadge.svelte
│   │   │   │   ├── DataCard.svelte
│   │   │   │   ├── DataTable.svelte
│   │   │   │   └── ConfirmDialog.svelte
│   │   │   ├── connection/
│   │   │   │   ├── QuickConnect.svelte
│   │   │   │   ├── PortSelector.svelte
│   │   │   │   └── ParameterForm.svelte
│   │   │   ├── reading/
│   │   │   │   ├── ShortReadResult.svelte
│   │   │   │   ├── FullReadResult.svelte
│   │   │   │   ├── EnergyCard.svelte
│   │   │   │   └── InstantValues.svelte
│   │   │   ├── loadprofile/
│   │   │   │   ├── ProfileSelector.svelte
│   │   │   │   ├── DateRangePicker.svelte
│   │   │   │   ├── ProfileChart.svelte
│   │   │   │   └── ProfileTable.svelte
│   │   │   ├── events/
│   │   │   │   ├── EventTable.svelte
│   │   │   │   ├── EventFilter.svelte
│   │   │   │   └── OutageRecords.svelte
│   │   │   ├── alarms/
│   │   │   │   ├── FFCodeDisplay.svelte
│   │   │   │   ├── GFCodeDisplay.svelte
│   │   │   │   └── AlarmBitCard.svelte
│   │   │   └── settings/
│   │   │       ├── TimeSync.svelte
│   │   │       ├── PasswordChange.svelte
│   │   │       ├── DSTSettings.svelte
│   │   │       ├── PeriodSettings.svelte
│   │   │       └── TariffEditor.svelte
│   │   └── utils/
│   │       ├── obis.js          — OBIS code definitions & labels
│   │       ├── ff-codes.js      — FF bit definitions
│   │       ├── gf-codes.js      — GF field definitions
│   │       ├── edas-ids.js      — EDAŞ ID lookup table
│   │       ├── formatters.js    — Number/date formatting (Turkish)
│   │       └── export.js        — CSV/PDF/JSON export helpers
│   ├── routes/
│   │   ├── +layout.svelte       — Main layout with sidebar + header + log
│   │   ├── +page.svelte         — Home / Connection (Ana Sayfa)
│   │   ├── short-read/
│   │   │   └── +page.svelte     — Kısa Okuma
│   │   ├── full-read/
│   │   │   └── +page.svelte     — Tam Okuma
│   │   ├── load-profile/
│   │   │   └── +page.svelte     — Yük Profili
│   │   ├── events/
│   │   │   └── +page.svelte     — Olaylar
│   │   ├── alarms/
│   │   │   └── +page.svelte     — Alarmlar
│   │   └── settings/
│   │       ├── time-sync/+page.svelte
│   │       ├── password/+page.svelte
│   │       ├── dst/+page.svelte
│   │       ├── periods/+page.svelte
│   │       └── tariffs/+page.svelte
│   ├── app.html
│   └── app.css                  — Tailwind imports + custom styles
├── package.json
├── svelte.config.js
├── tailwind.config.js
├── vite.config.js
└── CLAUDE.md                    ← This file
```

---

## Context Folders for AI Assistant

When implementing, provide these folders to the AI assistant for reference:

### `context/mass-protocol/`
Contains the full MASS specification PDF. Key sections:
- Ek-C: Complete OBIS code tables (pages 45-55)
- Ek-C: FF codes (pages 55-58), GF codes (page 58)
- Ek-D: Programmable parameters & defaults (pages 59-60)
- Ek-E: Load profile contents for P.01, P.02, P.03 (pages 61-64)
- Sections 5.1-5.2: Programming mode & security (pages 17-18)
- Section 2.1.8-2.1.9: Optical & RS485 port specs (pages 5-6)

### `context/design-guides/`
Contains the HTML design template (`code.html`) showing:
- Exact color values, spacing, and typography
- Sidebar navigation pattern
- Quick Connect card layout
- Communication log panel structure
- Previous sessions and reports list
- Dark/light mode implementation
- Responsive grid layouts

---

## Important Implementation Notes

1. **Serial communication is 7-bit Even parity 1 stop bit (7E1)** for IEC 62056-21. This is NOT the default 8N1.

2. **BCC calculation:** XOR all bytes from the byte immediately after SOH/STX up to and including ETX.

3. **Mode C always starts at 300 baud.** The meter responds with its max baud rate character, then both sides switch after ACK.

4. **Readout modes** are selected in the ACK message (Step 3):
   - Mode 0: Full readout (all data as stream)
   - Mode 1: Programming mode (individual OBIS read/write)
   - Mode 5: Technical quality parameters
   - Mode 6: Short read packet
   - Mode 7: Historical data
   - Mode 8: Warning/alert packet
   - Mode 9: Outage records

5. **All text is Turkish.** Use proper Turkish characters: ç, ğ, ı, İ, ö, ş, ü, Ç, Ğ, Ö, Ş, Ü.

6. **No user authentication.** The app is a standalone field tool. No login screen.

7. **Data persistence:** Store all reading sessions locally so users can review past readings without re-reading the meter.

8. **Responsive but desktop-first.** The app runs in Tauri (desktop). Sidebar should collapse on smaller windows but not be designed for mobile.

9. **Load profile can be very large** (180+ days × 96 intervals/day). Implement streaming/pagination for the data table and efficient chart rendering.

10. **3 wrong password attempts locks the meter for 6 hours.** Show a prominent warning counter in the password input UI.