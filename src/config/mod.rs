pub mod preferences;
pub mod theme;
pub mod yaml_theme;
pub mod yaml_theme_manager;

use anyhow::Result;
use preferences::Preferences;
use theme::Theme;
use yaml_theme_manager::YamlThemeManager;
use std::path::PathBuf;
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use std::sync::Arc;

pub static PROJECT_DIRS: Lazy<Option<ProjectDirs>> = Lazy::new(|| {
    ProjectDirs::from("com", "NeoTerm", "NeoTerm")
});

pub static CONFIG_DIR: Lazy<PathBuf> = Lazy::new(|| {
    PROJECT_DIRS.as_ref()
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./config")) // Fallback for systems without standard dirs
});

pub static DATA_DIR: Lazy<PathBuf> = Lazy::new(|| {
    PROJECT_DIRS.as_ref()
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./data")) // Fallback
});

pub static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    PROJECT_DIRS.as_ref()
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./cache")) // Fallback
});

pub struct ConfigManager {
    preferences: Arc<RwLock<Preferences>>,
    theme_manager: Arc<YamlThemeManager>,
}

impl ConfigManager {
    pub async fn new() -> Result<Self> {
        // Ensure config directories exist
        tokio::fs::create_dir_all(&*CONFIG_DIR).await?;
        tokio::fs::create_dir_all(&*DATA_DIR).await?;
        tokio::fs::create_dir_all(&*CACHE_DIR).await?;

        let preferences = Arc::new(RwLock::new(Preferences::load().await?));
        let theme_manager = Arc::new(YamlThemeManager::new());

        Ok(Self {
            preferences,
            theme_manager,
        })
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Config manager initialized.");
        self.theme_manager.init().await?;
        Ok(())
    }

    pub async fn get_preferences(&self) -> Preferences {
        self.preferences.read().await.clone()
    }

    pub async fn update_preferences(&self, new_prefs: Preferences) -> Result<()> {
        let mut prefs = self.preferences.write().await;
        *prefs = new_prefs;
        prefs.save().await?;
        Ok(())
    }

    pub async fn get_current_theme(&self) -> Result<Theme> {
        let prefs = self.preferences.read().await;
        self.theme_manager.get_theme(&prefs.theme_name).await
    }

    pub async fn get_theme_manager(&self) -> Arc<YamlThemeManager> {
        self.theme_manager.clone()
    }
}

pub fn init() {
    log::info!("Config module initialized.");
    // Accessing lazy statics here to ensure they are initialized early
    let _ = &*CONFIG_DIR;
    let _ = &*DATA_DIR;
    let _ = &*CACHE_DIR;
}
