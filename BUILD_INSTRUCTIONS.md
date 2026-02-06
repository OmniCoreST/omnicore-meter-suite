# Build Instructions

## Issue: Cannot Compile

**Error**: `cargo: command not found` or `program not found`

**Cause**: Rust/Cargo is not installed or not in PATH.

## Solution: Install Rust

### Windows Installation

1. **Download Rust installer**:
   - Go to: https://rustup.rs/
   - Download `rustup-init.exe`

2. **Run the installer**:
   ```
   rustup-init.exe
   ```
   - Choose option 1 (default installation)
   - Wait for installation to complete

3. **Restart your terminal/PowerShell**:
   - Close current terminal
   - Open a new PowerShell or CMD window
   - This ensures PATH is updated

4. **Verify installation**:
   ```bash
   cargo --version
   rustc --version
   ```

   Expected output:
   ```
   cargo 1.xx.x
   rustc 1.xx.x
   ```

5. **Navigate to project and build**:
   ```bash
   cd D:\projects\omnicore-meter-suite\code
   npm run tauri dev
   ```

## Alternative: Development Without Building

If you can't install Rust right now, you can:

1. **Review the changes** in the documentation:
   - `SESSION_CYCLE_FIX.md` - Complete explanation
   - `test_load_profile.py` - Python test script

2. **Install on another machine** that has Rust

3. **Use pre-built version** if available

## Build Commands

### Development (with hot reload)
```bash
cd code
npm run tauri dev
```

### Production Build
```bash
cd code
npm run tauri build
```

### Check Rust Code Only
```bash
cd code/src-tauri
cargo check
```

### Run Tests
```bash
cd code/src-tauri
cargo test
```

## What Was Fixed

Even without compiling, here's what the code changes do:

### 1. **Each Operation is Independent**

**Before**:
```
Connect (9600 → 19200)
  ↓
Short Read (at 19200) ✅
  ↓
Load Profile (tries at 19200) ❌ Meter expects 9600!
```

**After**:
```
Connect (9600 → 19200)
  ↓
Short Read (at 19200) ✅
  ↓ Send Break, reset to 9600
Load Profile:
  - Disconnect
  - Reconnect at 9600 ✅
  - Negotiate → 19200
  - Send Mode 1 request ✅
```

### 2. **Break Command Added**

After `read_short()` and `read_full()`:
```rust
let break_cmd = iec62056::build_break_command(); // <SOH>B0<ETX>BCC
port.write_all(&break_cmd);
```

### 3. **Load Profile Full Cycle**

```rust
// Save params
let (port_name, baud, ...) = { /* extract from manager */ };

// Disconnect
manager.disconnect();

// Reconnect from initial baud
connect(params, window).await?;

// Re-handshake for Mode 1
// ... (fresh handshake starting from initial baud)
```

## Code Changes Summary

| File | Lines | Change |
|------|-------|--------|
| `commands/mod.rs` | 371-417 | Added `end_and_reset_session()` |
| `commands/mod.rs` | 419-490 | Modified `read_full()` - adds disconnect/reconnect |
| `commands/mod.rs` | 618-628 | Added Break command at end of `read_full()` |
| `commands/mod.rs` | 846-856 | Added Break command at end of `read_short()` |
| `commands/mod.rs` | 1176-1260 | Modified `read_load_profile()` - full cycle |

## Verification Without Compiling

You can verify the logic is correct by:

1. **Reading the code changes** in `commands/mod.rs`
2. **Following the protocol flow** in `SESSION_CYCLE_FIX.md`
3. **Testing with Python script** (`test_load_profile.py`)

## Next Steps

1. ✅ Install Rust (see above)
2. ✅ Restart terminal
3. ✅ Run `npm run tauri dev`
4. ✅ Test with meter on COM5
5. ✅ Check communication log for Break commands

## Common Issues

### "cargo: command not found" after installing Rust
- **Solution**: Restart terminal/PowerShell

### "linker 'link.exe' not found"
- **Solution**: Install Visual Studio C++ Build Tools
- Download: https://visualstudio.microsoft.com/downloads/
- Select "Desktop development with C++"

### Port in use
- **Solution**: Close any other serial terminal (PuTTY, etc.)

### Permission denied on COM5
- **Solution**: Run as Administrator or close app using the port

---

**TL;DR**: Install Rust from https://rustup.rs/, restart terminal, then `npm run tauri dev`
