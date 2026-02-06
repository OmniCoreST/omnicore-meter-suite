//! Quick test binary for serial meter reading
//! Run with: cargo run --bin test_read -- <port> <mode> <baud> [password] [date_range]
//! Examples:
//!   cargo run --bin test_read -- COM5 short 9600
//!   cargo run --bin test_read -- COM5 full auto
//!   cargo run --bin test_read -- COM5 short auto-serial
//!   cargo run --bin test_read -- COM5 profile1 9600 00000000
//!
//! Baud argument:
//!   <number>      - specific baud (serial + explicit)
//!   auto          - connectionType=auto, baudRate=auto -> tries [9600, 300]
//!   auto-serial   - connectionType=serial, baudRate=auto -> tries [9600, 300, 19200]
//!   300           - optical mode -> tries [300] only

use std::io::{Read, Write};
use std::time::{Duration, Instant};

/// Determine which initial baud rates to try (mirrors io::resolve_initial_bauds)
fn resolve_bauds(baud_arg: &str) -> Vec<u32> {
    match baud_arg {
        "auto" => vec![9600, 300],           // connectionType=auto, baudRate=auto
        "auto-serial" => vec![9600, 300, 19200], // connectionType=serial, baudRate=auto
        other => {
            let baud: u32 = other.parse().unwrap_or(9600);
            vec![baud]  // specific baud
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let port_name = args.get(1).map(|s| s.as_str()).unwrap_or("/dev/ttyS4");
    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("short");
    let baud_arg = args.get(3).map(|s| s.as_str()).unwrap_or("9600");
    let password = args.get(4).and_then(|s| if s == "-" || s == "none" { None } else { Some(s.as_str()) });
    let date_range = args.get(5).map(|s| s.as_str()); // e.g. "yesterday", "26-02-05", "26-02-05,00:00;26-02-06,00:00"

    let baud_rates = resolve_bauds(baud_arg);

    let is_profile = mode.starts_with("profile");
    let profile_number: u8 = if is_profile {
        mode.strip_prefix("profile").and_then(|n| n.parse().ok()).unwrap_or(1)
    } else {
        1
    };

    println!("=== Omnicore Meter Test ===");
    println!("Port: {}", port_name);
    println!("Mode: {}", mode);
    println!("Baud arg: {} -> try {:?}", baud_arg, baud_rates);
    if is_profile {
        println!("Profile: P.{:02}", profile_number);
        println!("Password: {}", password.unwrap_or("(none - will try without)"));
        println!("Date range: {}", date_range.unwrap_or("(all data)"));
    }
    println!("Usage: test_read <port> <mode:short|full|profile1> <baud:9600|auto|auto-serial> [password] [date_range]");
    println!("  baud: number=explicit, auto=try [9600,300], auto-serial=try [9600,300,19200]");
    println!("  date_range: 'yesterday', 'yy-mm-dd', or 'yy-mm-dd,hh:mm;yy-mm-dd,hh:mm'");
    println!();

    // Step 1: List available ports
    println!("[1] Listing available serial ports...");
    match serialport::available_ports() {
        Ok(ports) => {
            if ports.is_empty() {
                println!("    No ports found!");
            }
            for p in &ports {
                println!("    - {} ({:?})", p.port_name, p.port_type);
            }
        }
        Err(e) => println!("    Error listing ports: {}", e),
    }
    println!();

    // Step 2-4: Open port, handshake, read identification (with baud retry)
    let mut port: Option<Box<dyn serialport::SerialPort>> = None;
    let mut initial_baud: u32 = 0;
    let mut ident_response = String::new();
    #[allow(unused_assignments)]
    let mut baud_char: char = '0';

    for (attempt, &try_baud) in baud_rates.iter().enumerate() {
        println!("[2] Opening port {} @ {} baud (7E1) [Attempt {}/{}]...",
            port_name, try_baud, attempt + 1, baud_rates.len());

        let mut current_port = match serialport::new(port_name, try_baud)
            .data_bits(serialport::DataBits::Seven)
            .parity(serialport::Parity::Even)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(Duration::from_millis(2000))
            .open()
        {
            Ok(p) => {
                println!("    Port opened successfully!");
                p
            }
            Err(e) => {
                eprintln!("    FAILED to open port @ {} baud: {}", try_baud, e);
                if attempt == baud_rates.len() - 1 {
                    eprintln!("    If on WSL2, COM5 = /dev/ttyS4");
                    eprintln!("    You may need to run from Windows or use usbipd.");
                    std::process::exit(1);
                }
                continue;
            }
        };

        // Send handshake
        println!("[3] Sending handshake request /?!\\r\\n ...");
        let request = b"/?!\r\n";
        print!("    TX: ");
        print_hex(request);

        if let Err(e) = current_port.write_all(request) {
            eprintln!("    FAILED to send: {}", e);
            continue;
        }
        let _ = current_port.flush();
        println!("    Sent OK, waiting for response...");

        // Read identification response
        std::thread::sleep(Duration::from_millis(500));
        println!("[4] Reading identification response...");

        let mut response_buf = vec![0u8; 256];
        let mut ident_read = 0;
        let start = Instant::now();

        loop {
            match current_port.read(&mut response_buf[ident_read..]) {
                Ok(n) if n > 0 => {
                    ident_read += n;
                    if ident_read >= 2
                        && response_buf[ident_read - 2] == b'\r'
                        && response_buf[ident_read - 1] == b'\n'
                    {
                        break;
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if ident_read > 0 {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("    Read error: {}", e);
                    break;
                }
            }
            if start.elapsed() > Duration::from_secs(5) {
                println!("    Timeout after 5s");
                break;
            }
        }

        if ident_read > 0 {
            let response = String::from_utf8_lossy(&response_buf[..ident_read]);
            print!("    RX ({} bytes): ", ident_read);
            print_hex(&response_buf[..ident_read]);
            println!("    ASCII: {}", response.trim());

            let resp_str = response.trim();
            if resp_str.starts_with('/') && resp_str.len() >= 5 {
                initial_baud = try_baud;
                ident_response = resp_str.to_string();
                port = Some(current_port);
                println!("    -> Response received @ {} baud!", try_baud);
                break;
            } else {
                println!("    -> Invalid identification response, trying next baud...");
            }
        } else {
            println!("    -> No response @ {} baud", try_baud);
        }
        println!();
    }

    let mut port = match port {
        Some(p) => p,
        None => {
            eprintln!("    No response from meter at any baud rate!");
            std::process::exit(1);
        }
    };

    let resp_str = &ident_response;
    let manufacturer = &resp_str[1..4.min(resp_str.len())];
    baud_char = resp_str.chars().nth(4).unwrap_or('0');
    let model_info = if resp_str.len() > 5 { &resp_str[5..] } else { "" };

    println!("    Manufacturer: {}", manufacturer);
    println!("    Baud char: {} (max baud: {})", baud_char, baud_from_char(baud_char));
    println!("    Model: {}", model_info);
    println!();

    // Determine target baud: for auto modes, negotiate to meter's max; for explicit, keep it
    let target_baud = if baud_arg.starts_with("auto") || baud_arg == "300" {
        baud_from_char(baud_char) // negotiate to meter's max
    } else {
        let explicit: u32 = baud_arg.parse().unwrap_or(0);
        if explicit > 0 { explicit } else { baud_from_char(baud_char) }
    };

    // Step 5: Send ACK with mode selection
    // IEC 62056-21 ACK format: ACK V Z Y CR LF
    // V = protocol control (mode), Z = baud rate, Y = mode control
    let mode_char = if is_profile {
        b'1'  // Mode 1 - Programming
    } else {
        match mode {
            "full" => b'0',
            "short" => b'6',
            _ => b'0',
        }
    };

    let target_baud_char = char_from_baud(target_baud);
    println!("[5] Sending ACK (Mode {} @ {} baud, char '{}')...", mode_char - b'0', target_baud, target_baud_char);
    let ack = [0x06, mode_char, target_baud_char as u8, mode_char, b'\r', b'\n'];
    print!("    TX: ");
    print_hex(&ack);

    if let Err(e) = port.write_all(&ack) {
        eprintln!("    FAILED to send ACK: {}", e);
        std::process::exit(1);
    }
    let _ = port.flush();

    // Wait and switch baud rate
    std::thread::sleep(Duration::from_millis(300));

    if target_baud != initial_baud {
        println!("    Switching baud rate: {} -> {}", initial_baud, target_baud);
        if let Err(e) = port.set_baud_rate(target_baud) {
            eprintln!("    FAILED to set baud rate: {}", e);
        }
    } else {
        println!("    Keeping baud rate at {}", initial_baud);
    }
    println!();

    // For profile mode: handle programming mode setup
    if is_profile {
        // Read meter's response (may be password request)
        std::thread::sleep(Duration::from_millis(500));
        println!("[5a] Reading programming mode response...");

        let mut prog_buf = vec![0u8; 256];
        let mut prog_read = 0;
        let prog_start = Instant::now();

        loop {
            match port.read(&mut prog_buf[prog_read..]) {
                Ok(n) if n > 0 => {
                    prog_read += n;
                    // Look for complete message
                    if prog_read >= 2 {
                        // Check for BCC (byte after ETX)
                        if let Some(etx_pos) = prog_buf[..prog_read].iter().position(|&b| b == 0x03) {
                            if etx_pos + 1 < prog_read {
                                break; // Got full message with BCC
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if prog_read > 0 { break; }
                }
                Err(_) => break,
            }
            if prog_start.elapsed() > Duration::from_secs(3) {
                break;
            }
        }

        if prog_read > 0 {
            print!("    RX ({} bytes): ", prog_read);
            print_hex(&prog_buf[..prog_read]);
            let prog_ascii = String::from_utf8_lossy(&prog_buf[..prog_read]);
            println!("    ASCII: {}", prog_ascii.escape_default());

            // Check if it's a password request (SOH P0 STX () ETX BCC)
            if prog_read >= 2 && prog_buf[0] == 0x01 && prog_buf[1] == b'P' {
                println!("    -> Meter is requesting password");
            }
        } else {
            println!("    No response (meter may not require password)");
        }
        println!();

        // Send password if provided
        if let Some(pwd) = password {
            println!("[5b] Sending password: {} ...", pwd);
            let pwd_cmd = build_password_command(pwd);
            print!("    TX: ");
            print_hex(&pwd_cmd);

            if let Err(e) = port.write_all(&pwd_cmd) {
                eprintln!("    FAILED to send password: {}", e);
                send_break_and_exit(&mut port);
            }
            let _ = port.flush();

            // Wait for ACK/NAK
            std::thread::sleep(Duration::from_millis(500));

            let mut ack_buf = [0u8; 1];
            match port.read(&mut ack_buf) {
                Ok(1) if ack_buf[0] == 0x06 => {
                    println!("    -> Password ACCEPTED (ACK)");
                }
                Ok(1) if ack_buf[0] == 0x15 => {
                    eprintln!("    -> Password REJECTED (NAK)");
                    send_break_and_exit(&mut port);
                }
                Ok(n) => {
                    print!("    -> Unexpected response ({} bytes): ", n);
                    print_hex(&ack_buf[..n]);
                    println!("    Continuing anyway...");
                }
                Err(e) => {
                    eprintln!("    -> No response: {}", e);
                    println!("    Continuing anyway...");
                }
            }
            println!();
        } else {
            println!("[5b] No password provided, skipping authentication");
            println!();
        }

        // Resolve date range
        let resolved_range = resolve_date_range(date_range);
        if let Some(ref range) = resolved_range {
            println!("[5c] Sending R2 command for P.{:02} with range: {}...", profile_number, range);
        } else {
            println!("[5c] Sending R2 command for P.{:02} (all data)...", profile_number);
        }
        let r2_cmd = build_load_profile_command(profile_number, resolved_range.as_deref());
        print!("    TX: ");
        print_hex(&r2_cmd);
        let r2_ascii = String::from_utf8_lossy(&r2_cmd);
        println!("    ASCII: {}", r2_ascii.escape_default());

        if let Err(e) = port.write_all(&r2_cmd) {
            eprintln!("    FAILED to send R2 command: {}", e);
            send_break_and_exit(&mut port);
        }
        let _ = port.flush();
        println!();
    }

    // Step 6: Read data until ETX
    let (buf_size, idle_timeout) = if is_profile {
        (524288usize, 15000u64) // 512KB, 15s for profiles
    } else {
        match mode {
            "full" => (131072, 5000),
            "short" => (8192, 3000),
            _ => (131072, 5000),
        }
    };

    println!("[6] Reading data (waiting for ETX, idle timeout: {}s)...", idle_timeout / 1000);
    let mut data_buf = vec![0u8; buf_size];
    let mut total_read = 0;
    let mut found_etx = false;
    let read_start = Instant::now();
    let mut last_read_time = Instant::now();

    std::thread::sleep(Duration::from_millis(500));

    loop {
        match port.read(&mut data_buf[total_read..]) {
            Ok(n) if n > 0 => {
                total_read += n;
                last_read_time = Instant::now();
                // Only print every 100+ bytes to reduce spam
                if total_read < 1000 || total_read % 1000 < n {
                    print!("\r    Received: {} bytes ({:.1}s)    ", total_read, read_start.elapsed().as_secs_f32());
                }

                // Check for ETX (0x03)
                for i in (total_read.saturating_sub(n)..total_read).rev() {
                    if data_buf[i] == 0x03 {
                        found_etx = true;
                        break;
                    }
                }
                if found_etx {
                    println!();
                    println!("    ETX found! Data complete.");
                    break;
                }
            }
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                println!();
                eprintln!("    Read error: {}", e);
                break;
            }
        }

        if last_read_time.elapsed() > Duration::from_millis(idle_timeout) {
            println!();
            if total_read == 0 {
                eprintln!("    Timeout: No data received ({}ms idle)", idle_timeout);
            } else {
                println!("    Idle timeout: {} bytes received but no ETX", total_read);
            }
            break;
        }

        // Overall timeout: 5 minutes for profiles
        if is_profile && read_start.elapsed() > Duration::from_secs(300) {
            println!();
            println!("    Overall timeout: 5 minutes");
            break;
        }
    }
    println!();

    // Step 7: Send Break command
    println!("[7] Sending Break command...");
    let break_cmd = build_break_command();
    print!("    TX: ");
    print_hex(&break_cmd);
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(100));
    drop(port);
    println!("    Port closed.");
    println!();

    // Step 8: Display results
    println!("=== RESULTS ===");
    println!("Total bytes: {}", total_read);
    println!("ETX found: {}", found_etx);
    println!("Duration: {:.2}s", read_start.elapsed().as_secs_f32());
    println!();

    if total_read > 0 {
        let stx_pos = data_buf[..total_read].iter().position(|&b| b == 0x02);
        let etx_pos = data_buf[..total_read].iter().position(|&b| b == 0x03);

        if let (Some(stx), Some(etx)) = (stx_pos, etx_pos) {
            let data_slice = &data_buf[stx + 1..etx];
            let data_str = String::from_utf8_lossy(data_slice);

            // BCC check
            if etx + 1 < total_read {
                let received_bcc = data_buf[etx + 1];
                let mut calculated_bcc: u8 = 0;
                for &b in &data_buf[stx + 1..=etx] {
                    calculated_bcc ^= b;
                }
                println!("BCC: received=0x{:02X}, calculated=0x{:02X} -> {}",
                    received_bcc, calculated_bcc,
                    if received_bcc == calculated_bcc { "OK" } else { "MISMATCH!" });
            }

            println!();
            println!("--- Data ({} bytes) ---", data_slice.len());
            let mut line_count = 0;
            for line in data_str.lines() {
                let line = line.trim();
                if line.is_empty() || line == "!" {
                    continue;
                }
                line_count += 1;
                // For profiles, limit output to first 100 lines
                if is_profile && line_count > 100 {
                    println!("  ... (truncated at 100 lines)");
                    // Count remaining
                    let remaining = data_str.lines()
                        .filter(|l| !l.trim().is_empty() && l.trim() != "!")
                        .count();
                    println!("  Total lines: {}", remaining);
                    break;
                }
                println!("  {}", line);
            }
            if !is_profile || line_count <= 100 {
                println!("--- End ({} lines) ---", line_count);
            }
        } else {
            println!("Could not find STX/ETX markers in data");
            println!("First 500 bytes (hex):");
            let show = total_read.min(500);
            print_hex(&data_buf[..show]);
            println!();
            println!("First 500 bytes (ASCII):");
            let ascii = String::from_utf8_lossy(&data_buf[..show]);
            println!("{}", ascii.escape_default());
        }
    }
}

/// Calculate BCC (XOR of all bytes)
fn calculate_bcc(data: &[u8]) -> u8 {
    let mut bcc: u8 = 0;
    for &b in data {
        bcc ^= b;
    }
    bcc
}

/// Build password command: SOH P1 STX (password) ETX BCC
fn build_password_command(password: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(0x01); // SOH
    msg.push(b'P');
    msg.push(b'1');
    msg.push(0x02); // STX
    msg.push(b'(');
    msg.extend_from_slice(password.as_bytes());
    msg.push(b')');
    msg.push(0x03); // ETX

    let bcc = calculate_bcc(&msg[3..]); // BCC from STX to ETX
    msg.push(bcc);
    msg
}

/// Resolve date range shorthand into IEC format
/// "yesterday" -> "26-02-05,00:00;26-02-06,00:00"
/// "26-02-05"  -> "26-02-05,00:00;26-02-06,00:00"
/// "26-02-05,00:00;26-02-06,00:00" -> pass through
fn resolve_date_range(input: Option<&str>) -> Option<String> {
    let input = input?;

    if input == "yesterday" {
        // Calculate yesterday's date
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let yesterday = now - 86400;
        let today = now;

        let fmt = |ts: u64| -> String {
            // Simple date calculation (UTC-based, good enough for testing)
            let days = ts / 86400;
            let mut y = 1970u32;
            let mut remaining = days;

            loop {
                let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
                if remaining < days_in_year { break; }
                remaining -= days_in_year;
                y += 1;
            }

            let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
            let days_in_month = [31, if leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            let mut m = 0u32;
            for &dim in &days_in_month {
                if remaining < dim { break; }
                remaining -= dim;
                m += 1;
            }

            format!("{:02}-{:02}-{:02},00:00", y % 100, m + 1, remaining + 1)
        };

        Some(format!("{};{}", fmt(yesterday), fmt(today)))
    } else if input.len() == 8 && input.chars().nth(2) == Some('-') {
        // Single date like "26-02-05" -> start at 00:00, end next day
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() == 3 {
            let yy: u32 = parts[0].parse().unwrap_or(0);
            let mm: u32 = parts[1].parse().unwrap_or(0);
            let dd: u32 = parts[2].parse().unwrap_or(0);

            // Simple next day (doesn't handle month boundaries perfectly, but good enough)
            let (ny, nm, nd) = if dd >= 28 {
                if mm >= 12 { (yy + 1, 1, 1) } else { (yy, mm + 1, 1) }
            } else {
                (yy, mm, dd + 1)
            };

            Some(format!("{:02}-{:02}-{:02},00:00;{:02}-{:02}-{:02},00:00",
                yy, mm, dd, ny, nm, nd))
        } else {
            Some(input.to_string())
        }
    } else {
        // Pass through as-is (full range format)
        Some(input.to_string())
    }
}

/// Build load profile R2 command: SOH R2 STX P.XX(range) ETX BCC
fn build_load_profile_command(profile_number: u8, date_range: Option<&str>) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(0x01); // SOH
    msg.push(b'R');
    msg.push(b'2');
    msg.push(0x02); // STX
    let profile_str = format!("P.{:02}", profile_number);
    msg.extend_from_slice(profile_str.as_bytes());
    msg.push(b'(');
    match date_range {
        Some(range) if !range.is_empty() => {
            msg.extend_from_slice(range.as_bytes());
        }
        _ => {
            msg.push(b';'); // Read all data
        }
    }
    msg.push(b')');
    msg.push(0x03); // ETX

    let bcc = calculate_bcc(&msg[3..]); // BCC from STX to ETX
    msg.push(bcc);
    msg
}

/// Build break command: SOH B0 ETX BCC
fn build_break_command() -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(0x01); // SOH
    msg.push(b'B');
    msg.push(b'0');
    msg.push(0x03); // ETX
    let bcc = calculate_bcc(&msg[3..]); // BCC from ETX only
    msg.push(bcc);
    msg
}

fn send_break_and_exit(port: &mut Box<dyn serialport::SerialPort>) -> ! {
    let break_cmd = build_break_command();
    let _ = port.write_all(&break_cmd);
    let _ = port.flush();
    std::thread::sleep(Duration::from_millis(100));
    std::process::exit(1);
}

fn baud_from_char(c: char) -> u32 {
    match c {
        '0' => 300,
        '1' => 600,
        '2' => 1200,
        '3' => 2400,
        '4' => 4800,
        '5' => 9600,
        '6' => 19200,
        _ => 300,
    }
}

fn char_from_baud(baud: u32) -> char {
    match baud {
        300 => '0',
        600 => '1',
        1200 => '2',
        2400 => '3',
        4800 => '4',
        9600 => '5',
        19200 => '6',
        _ => '0',
    }
}

fn print_hex(data: &[u8]) {
    for b in data {
        print!("{:02X} ", b);
    }
    println!();
}
