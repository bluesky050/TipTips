use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub email_address: String,
    /// QQ mail authorization code (stored with simple obfuscation, not plaintext)
    #[serde(default)]
    pub auth_code: String,
    #[serde(default)]
    pub gemini_api_key: String,
    #[serde(default = "default_scan_interval")]
    pub scan_interval_minutes: u64,
    #[serde(default)]
    pub last_scan_time: String,
    /// UIDs of emails that have already been processed (for deduplication)
    #[serde(default)]
    pub processed_uids: Vec<u32>,
    /// Gemini model name to use
    #[serde(default = "default_model")]
    pub gemini_model: String,
}

fn default_scan_interval() -> u64 {
    60
}

fn default_model() -> String {
    "gemini-3.8-flash".to_string()
}

impl Default for EmailConfig {
    fn default() -> Self {
        EmailConfig {
            enabled: false,
            email_address: String::new(),
            auth_code: String::new(),
            gemini_api_key: String::new(),
            scan_interval_minutes: default_scan_interval(),
            last_scan_time: String::new(),
            processed_uids: Vec::new(),
            gemini_model: default_model(),
        }
    }
}

/// Simple obfuscation for sensitive values (not cryptographic security, just prevents plaintext)
pub fn obfuscate(plaintext: &str) -> String {
    use base64::Engine;
    let bytes: Vec<u8> = plaintext.bytes().map(|b| b ^ 0x5A).collect();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

pub fn deobfuscate(encoded: &str) -> String {
    use base64::Engine;
    match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(bytes) => {
            let plaintext: Vec<u8> = bytes.iter().map(|b| b ^ 0x5A).collect();
            String::from_utf8(plaintext).unwrap_or_default()
        }
        Err(_) => encoded.to_string(), // If decoding fails, return as-is (maybe already plaintext)
    }
}

pub fn email_config_path() -> PathBuf {
    super::data::data_dir().join("email_config.json")
}

pub fn read_email_config() -> EmailConfig {
    let path = email_config_path();
    if !path.exists() {
        return EmailConfig::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => {
            let clean = content.trim_start_matches('\u{feff}');
            serde_json::from_str(clean).unwrap_or_default()
        }
        Err(_) => EmailConfig::default(),
    }
}

pub fn write_email_config(config: &EmailConfig) -> Result<(), String> {
    let dir = super::data::data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize email config: {}", e))?;
    let path = email_config_path();
    fs::write(&path, json).map_err(|e| format!("Failed to write email config: {}", e))?;
    Ok(())
}

/// Return a sanitized version of the config (hides sensitive fields)
pub fn sanitize_config(config: &EmailConfig) -> serde_json::Value {
    serde_json::json!({
        "enabled": config.enabled,
        "email_address": config.email_address,
        "has_auth_code": !config.auth_code.is_empty(),
        "has_gemini_api_key": !config.gemini_api_key.is_empty(),
        "scan_interval_minutes": config.scan_interval_minutes,
        "last_scan_time": config.last_scan_time,
        "processed_count": config.processed_uids.len(),
        "gemini_model": config.gemini_model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obfuscate_roundtrip() {
        let original = "my-secret-auth-code-12345";
        let obfuscated = obfuscate(original);
        assert_ne!(original, obfuscated);
        let recovered = deobfuscate(&obfuscated);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_email_config_default() {
        let config = EmailConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.scan_interval_minutes, 60);
        assert_eq!(config.gemini_model, "gemini-3.8-flash");
    }
}
