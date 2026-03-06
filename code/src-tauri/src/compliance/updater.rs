//! Config file auto-update checker

use super::config::{get_config_path, load_config, ComplianceConfig};
use serde::Deserialize;

/// Version info from update server
#[derive(Deserialize, Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub url: String,
}

/// Fetch latest version info from update server.
/// Returns None silently if unavailable.
pub async fn fetch_latest_version() -> Option<VersionInfo> {
    let config = load_config().ok()?;
    let update_url = config.update_url.as_deref()?.trim().to_string();
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

/// Download new config file from URL and overwrite local copy.
/// Returns new version string.
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

    // Validate TOML
    let parsed: ComplianceConfig = toml::from_str(&content)
        .map_err(|e| format!("İndirilen dosya geçerli TOML değil: {}", e))?;

    let new_version = parsed.config_version.clone();

    let path = get_config_path();
    std::fs::write(&path, &content)
        .map_err(|e| format!("Dosya yazılamadı: {}", e))?;

    log::info!("Uyumluluk kuralları güncellendi: v{}", new_version);
    Ok(new_version)
}
