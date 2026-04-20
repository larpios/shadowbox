use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub stores: HashMap<String, StoreConfig>,
    #[serde(default)]
    pub mappings: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoreConfig {
    pub url: String,
}

impl Config {
    pub fn load() -> std::io::Result<Self> {
        let config_path = config_dir()?.join("config.toml");
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            Ok(toml::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = config_dir()?;
        fs::create_dir_all(&dir)?;
        let config_path = dir.join("config.toml");
        let content = toml::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        fs::write(config_path, content)
    }
}

pub fn config_dir() -> std::io::Result<PathBuf> {
    if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(val).join("shadowbox"));
    }
    let home = std::env::var("HOME").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "HOME environment variable not set",
        )
    })?;
    Ok(PathBuf::from(home).join(".config").join("shadowbox"))
}

pub fn data_dir() -> std::io::Result<PathBuf> {
    if let Ok(val) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(val).join("shadowbox"));
    }
    let home = std::env::var("HOME").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "HOME environment variable not set",
        )
    })?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("shadowbox"))
}
