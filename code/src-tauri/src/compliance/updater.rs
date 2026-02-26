//! Kural dosyası otomatik güncelleme kontrolü

use super::rules::{get_rules_path, load_rules};
use serde::Deserialize;

/// GitHub'dan dönen versiyon JSON yapısı
#[derive(Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub url: String,
}

/// Sunucudan güncel versiyon bilgisini çeker.
/// Bağlanamazsa None döner (sessizce).
pub async fn fetch_latest_version() -> Option<VersionInfo> {
    let rules = load_rules().ok()?;
    let update_url = rules.update_url.as_deref()?.trim().to_string();
    if update_url.is_empty() {
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let response = client.get(&update_url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    response.json::<VersionInfo>().await.ok()
}

/// Yeni kural dosyasını URL'den indirir ve mevcut dosyanın üzerine yazar.
/// Döner: yeni versiyon string'i
pub async fn download_rules(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP istemcisi oluşturulamadı: {}", e))?;

    let response = client.get(url).send().await
        .map_err(|e| format!("İndirme isteği başarısız: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP hatası: {}", response.status()));
    }

    let content = response.text().await
        .map_err(|e| format!("İçerik okunamadı: {}", e))?;

    // Geçerli TOML mi kontrol et
    let parsed: super::rules::RulesFile = toml::from_str(&content)
        .map_err(|e| format!("İndirilen dosya geçerli TOML değil: {}", e))?;

    let new_version = parsed.rules_version.clone();

    let path = get_rules_path();
    std::fs::write(&path, &content)
        .map_err(|e| format!("Dosya yazılamadı: {}", e))?;

    log::info!("Uyumluluk kuralları güncellendi: v{}", new_version);
    Ok(new_version)
}
