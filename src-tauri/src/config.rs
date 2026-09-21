use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
}

fn default_view_mode() -> String {
    "list".to_string()
}

fn default_opacity() -> f64 {
    0.45
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            x: 100.0,
            y: 100.0,
            width: 360.0,
            height: 500.0,
            view_mode: default_view_mode(),
            opacity: default_opacity(),
        }
    }
}

pub fn config_file_path() -> PathBuf {
    super::data::data_dir().join("config.json")
}

pub fn read_config() -> WindowConfig {
    let path = config_file_path();
    if !path.exists() {
        return WindowConfig::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => WindowConfig::default(),
    }
}

pub fn write_config(config: &WindowConfig) -> Result<(), String> {
    let dir = super::data::data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize: {}", e))?;
    let path = config_file_path();
    fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}
