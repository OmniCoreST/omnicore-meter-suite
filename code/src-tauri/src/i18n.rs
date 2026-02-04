//! Internationalization support for backend messages

pub enum Lang {
    Turkish,
    English,
}

impl Lang {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tr" | "turkish" => Lang::Turkish,
            "en" | "english" => Lang::English,
            _ => Lang::Turkish, // Default to Turkish
        }
    }
}

pub struct I18n {
    lang: Lang,
}

impl I18n {
    pub fn new(lang: Lang) -> Self {
        Self { lang }
    }

    pub fn t(&self, key: &str) -> String {
        match &self.lang {
            Lang::Turkish => self.tr(key),
            Lang::English => self.en(key),
        }
    }

    fn tr(&self, key: &str) -> String {
        match key {
            // Connection messages
            "opening_port" => format!("Seri port açılıyor"),
            "port_opened" => format!("Port açıldı"),
            "port_open_failed" => format!("Port açılamadı"),
            "handshake_failed" => format!("Handshake gönderilemedi"),
            "waiting_response" => format!("Yanıt bekleniyor..."),
            "response_received" => format!("Yanıt alındı"),
            "no_response" => format!("Yanıt alınamadı"),
            "meter_identified" => format!("Sayaç tanımlandı"),
            "identification_failed" => format!("Sayaç tanımlama yanıtı ayrıştırılamadı"),
            "switching_baud" => format!("Baud hızı değiştiriliyor"),
            "baud_switched" => format!("Baud hızı olarak ayarlandı"),
            "baud_switch_failed" => format!("Baud hızı değiştirilemedi"),
            "connection_successful" => format!("Bağlantı başarılı!"),

            // Reading messages
            "waiting_short_packet" => format!("Kısa okuma paketi bekleniyor (Paket 6)..."),
            "waiting_full_packet" => format!("Tam okuma paketi bekleniyor (Mod 0 - Tüm veriler)..."),
            "data_received" => format!("Veri alımı tamamlandı"),
            "data_receive_failed" => format!("Veri alınamadı"),
            "incomplete_data" => format!("Veri tam alınamadı: ETX bulunamadı"),
            "verifying_data" => format!("Veriler alındı, doğrulanıyor..."),
            "parsing_obis" => format!("OBIS kodları çözümleniyor..."),
            "obis_parsed" => format!("OBIS kodu ayrıştırıldı"),
            "read_complete" => format!("Tam okuma başarıyla tamamlandı"),
            "short_read_complete" => format!("Kısa okuma başarıyla tamamlandı"),
            "bcc_mismatch" => format!("BCC uyuşmazlığı: beklenen"),
            "bcc_verified" => format!("BCC doğrulaması başarılı"),

            // Timeout messages
            "timeout_no_data" => format!("Zaman aşımı: Hiç veri alınamadı"),
            "timeout_no_etx" => format!("Zaman aşımı: ETX bulunamadı (alınan"),
            "idle_timeout" => format!("Boşta kalma zaman aşımı"),
            "absolute_timeout" => format!("Mutlak zaman aşımı"),
            "data_flow_stopped" => format!("Veri akışı durdu"),

            // Error messages
            "read_error" => format!("Okuma hatası"),
            "write_error" => format!("Yazma hatası"),
            "flush_error" => format!("Flush hatası"),

            // Progress messages
            "checking_connection" => format!("Bağlantı kontrol ediliyor..."),
            "requesting_packet" => format!("Paket alınıyor..."),
            "receiving_data" => format!("Veriler alınıyor..."),
            "complete" => format!("Tamamlandı!"),

            _ => key.to_string(),
        }
    }

    fn en(&self, key: &str) -> String {
        match key {
            // Connection messages
            "opening_port" => format!("Opening serial port"),
            "port_opened" => format!("Port opened"),
            "port_open_failed" => format!("Failed to open port"),
            "handshake_failed" => format!("Failed to send handshake"),
            "waiting_response" => format!("Waiting for response..."),
            "response_received" => format!("Response received"),
            "no_response" => format!("No response received"),
            "meter_identified" => format!("Meter identified"),
            "identification_failed" => format!("Failed to parse meter identification"),
            "switching_baud" => format!("Switching baud rate"),
            "baud_switched" => format!("Baud rate set to"),
            "baud_switch_failed" => format!("Failed to switch baud rate"),
            "connection_successful" => format!("Connection successful!"),

            // Reading messages
            "waiting_short_packet" => format!("Waiting for short read packet (Packet 6)..."),
            "waiting_full_packet" => format!("Waiting for full readout (Mode 0 - All data)..."),
            "data_received" => format!("Data reception completed"),
            "data_receive_failed" => format!("No data received"),
            "incomplete_data" => format!("Incomplete data: ETX not found"),
            "verifying_data" => format!("Data received, verifying..."),
            "parsing_obis" => format!("Parsing OBIS codes..."),
            "obis_parsed" => format!("OBIS code parsed"),
            "read_complete" => format!("Full read completed successfully"),
            "short_read_complete" => format!("Short read completed successfully"),
            "bcc_mismatch" => format!("BCC mismatch: expected"),
            "bcc_verified" => format!("BCC verification successful"),

            // Timeout messages
            "timeout_no_data" => format!("Timeout: No data received"),
            "timeout_no_etx" => format!("Timeout: ETX not found (received"),
            "idle_timeout" => format!("Idle timeout"),
            "absolute_timeout" => format!("Absolute timeout"),
            "data_flow_stopped" => format!("Data flow stopped"),

            // Error messages
            "read_error" => format!("Read error"),
            "write_error" => format!("Write error"),
            "flush_error" => format!("Flush error"),

            // Progress messages
            "checking_connection" => format!("Checking connection..."),
            "requesting_packet" => format!("Requesting packet..."),
            "receiving_data" => format!("Receiving data..."),
            "complete" => format!("Completed!"),

            _ => key.to_string(),
        }
    }
}
