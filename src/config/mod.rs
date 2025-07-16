use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::config::preferences::Preferences;
use crate::config::theme::{Theme, ThemeConfig};
use crate::config::yaml_theme_manager::YamlThemeManager;
use crate::agent_mode_eval::ai_client::AiConfig;

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
pub struct Config {
    pub preferences: Preferences,
    pub theme: ThemeConfig,
    pub ai: AiConfig,
    pub environment_profiles: HashMap<String, HashMap<String, String>>, // New field
    // Add other top-level configuration sections here
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preferences: Preferences::default(),
            theme: ThemeConfig::default(),
            ai: AiConfig::default(),
            environment_profiles: HashMap::new(),
        }
    }
}

impl Config {
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_default_config_path() -> PathBuf {
        // Example: ~/.config/neoterm/config.yaml or %APPDATA%/neoterm/config.yaml
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("neoterm")
            .join("config.yaml")
    }

    pub fn get_themes_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("neoterm")
            .join("themes")
    }

    pub fn get_workflows_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("neoterm")
            .join("workflows")
    }

    pub fn get_environment_profiles_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("neoterm")
            .join("env_profiles")
    }

    pub fn ensure_config_dirs_exist() -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = Self::get_default_config_path().parent().unwrap().to_path_buf();
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(Self::get_themes_dir())?;
        fs::create_dir_all(Self::get_workflows_dir())?;
        fs::create_dir_all(Self::get_environment_profiles_dir())?;
        Ok(())
    }
}

pub fn init() {
    println!("config module loaded");
}
