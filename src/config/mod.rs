use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use iced::Color;

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
    pub theme: ThemeConfig,
    pub preferences: UserPreferences,
    pub keybindings: KeyBindings,
    pub plugins: PluginConfig,
    
    // YAML theme settings
    pub yaml_themes_enabled: bool,
    pub active_yaml_theme: Option<String>,

    // Environment profiles
    pub env_profiles: EnvironmentProfiles, // Added for environment profiles
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            preferences: UserPreferences::default(),
            keybindings: KeyBindings::default(),
            plugins: PluginConfig::default(),
            yaml_themes_enabled: true,
            active_yaml_theme: None,
            env_profiles: EnvironmentProfiles::default(), // Initialize default environment profiles
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
            
            let mut config: AppConfig = toml::from_str(&content)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?;
            
            // Load YAML theme if specified
            if let Some(yaml_theme_name) = &config.active_yaml_theme {
                if let Ok(mut theme_manager) = YamlThemeManager::new() {
                    if let Some(yaml_theme) = theme_manager.get_theme(yaml_theme_name) {
                        config.theme = yaml_theme;
                    }
                }
            }
            
            Ok(config)
        } else {
            // Create default config and save it
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path()?;
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;
        
        std::fs::write(&config_path, content)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir()
            .ok_or(ConfigError::ConfigDirNotFound)?
            .join("neoterm");
        
        Ok(config_dir.join("config.toml"))
    }

    pub fn themes_dir() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir()
            .ok_or(ConfigError::ConfigDirNotFound)?
            .join("neoterm")
            .join("themes");
        
        Ok(config_dir)
    }

    /// Set active YAML theme
    pub fn set_yaml_theme(&mut self, theme_name: Option<String>) -> Result<(), ConfigError> {
        if let Some(name) = &theme_name {
            let mut theme_manager = YamlThemeManager::new()?;
            if let Some(yaml_theme) = theme_manager.get_theme(name) {
                self.theme = yaml_theme;
                self.active_yaml_theme = theme_name;
            } else {
                return Err(ConfigError::ThemeNotFound(name.clone()));
            }
        } else {
            self.active_yaml_theme = None;
            self.theme = ThemeConfig::default();
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config directory not found")]
    ConfigDirNotFound,
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Serialize error: {0}")]
    SerializeError(String),
    #[error("Theme not found: {0}")]
    ThemeNotFound(String),
    #[error("YAML theme error: {0}")]
    YamlThemeError(#[from] YamlThemeError),
}
