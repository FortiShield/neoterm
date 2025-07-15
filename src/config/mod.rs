use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use iced::Color;
use preferences::Preferences;

pub mod theme;
pub mod preferences;
pub mod storage;
pub mod yaml_theme;
pub mod yaml_theme_manager;

pub use theme::*;
pub use preferences::*;
pub use storage::*;
pub use yaml_theme::*;
pub use yaml_theme_manager::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub preferences: Preferences,
    pub env_profiles: EnvProfiles,
    // Add other configuration sections here
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            preferences: Preferences::default(),
            env_profiles: EnvProfiles::default(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: Self = serde_json::from_str(&config_str)?;
            Ok(config)
        } else {
            let default_config = Self::default();
            default_config.save()?;
            Ok(default_config)
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        let config_str = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, config_str)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut config_dir = dirs::config_dir().ok_or("Could not find config directory")?;
        config_dir.push("neoterm");
        std::fs::create_dir_all(&config_dir)?;
        config_dir.push("config.json");
        Ok(config_dir)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfiles {
    pub active_profile: Option<String>,
    pub profiles: std::collections::HashMap<String, EnvProfile>,
}

impl Default for EnvProfiles {
    fn default() -> Self {
        let mut profiles = std::collections::HashMap::new();
        profiles.insert(
            "default".to_string(),
            EnvProfile {
                variables: std::collections::HashMap::new(),
            },
        );
        Self {
            active_profile: Some("default".to_string()),
            profiles,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfile {
    pub variables: std::collections::HashMap<String, String>,
}
