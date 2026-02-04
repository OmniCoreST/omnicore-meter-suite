# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OmniCore Meter Suite is a Tauri 2 desktop application for reading and programming Turkish MASS-compliant electricity meters using the IEC 62056-21 Mode C protocol. The UI is in Turkish with English fallback.

## Build & Development Commands

```bash
# Install dependencies (run from code/ directory)
npm install

# Development mode (starts Vite dev server + Tauri)
npm run tauri dev

# Build for production
npm run tauri build

# TypeScript type checking
npm run check

# Frontend only (no Tauri)
npm run dev
```

### Rust Backend

```bash
# From code/src-tauri/
cargo build
cargo check
cargo test
cargo clippy
```

## Architecture

### Tech Stack
- **Frontend**: Svelte 5 + TypeScript + Tailwind CSS
- **Backend**: Rust (Tauri 2)
- **Serial**: `serialport` crate with IEC 62056-21 protocol
- **Storage**: SQLite via `rusqlite`
- **Charts**: Chart.js

### Frontend Structure (src/lib/)

**Stores** (`stores/`): Svelte stores for global state
- `connection.ts` - Serial port connection state and meter identity
- `meter.ts` - Current meter data from readings
- `logs.ts` - Communication log entries
- `progress.ts` - Progress bar state for read operations
- `navigation.ts` - Current page routing
- `theme.ts` - Dark/light mode
- `locale.ts` - i18n (Turkish/English)

**Pages** (`pages/`): Main views routed via `navigationStore`
- `Home.svelte` - Connection dashboard with port selection
- `ShortRead.svelte` / `FullRead.svelte` - Meter reading operations
- `LoadProfile.svelte` - Load profile charts and data
- Settings pages: `TimeSync`, `Password`, `DST`, `Periods`, `Tariffs`

**Layout** (`components/layout/`):
- `Layout.svelte` - Main layout orchestrator with page routing
- `Sidebar.svelte` - Navigation menu
- `Header.svelte` - Top bar with status
- `CommLog.svelte` - Bottom collapsible communication log panel

### Backend Structure (src-tauri/src/)

**Serial Module** (`serial/`):
- `iec62056.rs` - IEC 62056-21 Mode C protocol implementation (handshake, BCC, OBIS parsing)
- `port.rs` - Serial port enumeration

**Commands** (`commands/mod.rs`): Tauri commands exposed to frontend
- `list_serial_ports`, `connect`, `disconnect`
- `read_short`, `read_full`, `read_obis`
- `authenticate`, `write_obis`, `sync_time`

**Storage** (`storage/`): SQLite database for sessions, reports, settings

### IEC 62056-21 Protocol

Serial communication uses **7E1** (7 data bits, even parity, 1 stop bit). Mode C handshake:
1. Start at 300 baud
2. Send `/?ADDRESS!\r\n`
3. Parse identification response for baud rate negotiation
4. Send ACK with mode selection (0=readout, 1=programming, 6=short read)
5. Switch to negotiated baud rate

Control characters: SOH (0x01), STX (0x02), ETX (0x03), ACK (0x06), NAK (0x15)

### Tauri Events

Frontend listens for these events from backend:
- `comm-log` - Communication log entries (TX/RX data)
- `read-progress` - Step progress during meter readings
- `comm-activity` - LED indicator for TX/RX activity

## Design System

**Colors**: Primary `#279EA7`, Secondary `#1F3244`, Background dark `#0f1821`

**Dark mode**: Class-based (`darkMode: "class"` in Tailwind), toggled via `themeStore`

**Icons**: Material Symbols Outlined (bundled font)

**Font**: Oxanium (bundled)

## Releasing

See RELEASING.md. Releases are triggered by pushing version tags (e.g., `v0.2.0`). GitHub Actions builds Windows (.msi, .nsis) and Linux (.AppImage, .deb) artifacts.

Version must be updated in both:
- `src-tauri/tauri.conf.json`
- `package.json`
