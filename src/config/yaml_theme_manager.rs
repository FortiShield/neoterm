use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use std::sync::Arc;
use super::theme::Theme;
use super::yaml_theme::YamlTheme;
use super::CONFIG_DIR;

pub struct YamlThemeManager {
    themes: RwLock<HashMap<String, Theme>>,
    theme_dir: PathBuf,
}

impl YamlThemeManager {
    pub fn new() -> Self {
        let theme_dir = CONFIG_DIR.join("themes");
        Self {
            themes: RwLock::new(HashMap::new()),
            theme_dir,
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Initializing YAML theme manager. Theme directory: {:?}", self.theme_dir);
        fs::create_dir_all(&self.theme_dir).await?;
        self.load_themes_from_disk().await?;
        self.ensure_default_themes_exist().await?;
        Ok(())
    }

    async fn load_themes_from_disk(&self) -> Result<()> {
        let mut themes = self.themes.write().await;
        themes.clear(); // Clear existing themes before reloading

        let mut entries = fs::read_dir(&self.theme_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                log::debug!("Loading theme from file: {:?}", path);
                match fs::read_to_string(&path).await {
                    Ok(contents) => {
                        match serde_yaml::from_str::<YamlTheme>(&contents) {
                            Ok(yaml_theme) => {
                                let theme: Theme = yaml_theme.into();
                                themes.insert(theme.name.clone(), theme);
                                log::info!("Loaded theme: {}", themes.len());
                            },
                            Err(e) => log::error!("Failed to parse theme file {:?}: {}", path, e),
                        }
                    },
                    Err(e) => log::error!("Failed to read theme file {:?}: {}", path, e),
                }
            }
        }
        log::info!("Finished loading themes. Total themes loaded: {}", themes.len());
        Ok(())
    }

    async fn ensure_default_themes_exist(&self) -> Result<()> {
        let default_themes = vec![
            ("nord", include_str!("../../themes/nord.yaml")),
            ("gruvbox-dark", include_str!("../../themes/gruvbox-dark.yaml")),
        ];

        for (name, content) in default_themes {
            let theme_path = self.theme_dir.join(format!("{}.yaml", name));
            if !theme_path.exists() {
                log::info!("Writing default theme '{}' to {:?}", name, theme_path);
                fs::write(&theme_path, content).await?;
                // After writing, reload to ensure it's picked up by the manager
                self.load_themes_from_disk().await?;
            }
        }
        Ok(())
    }

    pub async fn get_theme(&self, name: &str) -> Result<Theme> {
        let themes = self.themes.read().await;
        themes.get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Theme '{}' not found.", name))
    }

    pub async fn list_themes(&self) -> Vec<String> {
        let themes = self.themes.read().await;
        themes.keys().cloned().collect()
    }

    pub async fn save_theme(&self, theme: Theme) -> Result<()> {
        let yaml_theme: YamlTheme = theme.into();
        let contents = serde_yaml::to_string(&yaml_theme)?;
        let path = self.theme_dir.join(format!("{}.yaml", yaml_theme.name));
        fs::write(&path, contents).await?;
        log::info!("Theme '{}' saved to {:?}", yaml_theme.name, path);
        self.load_themes_from_disk().await?; // Reload to update internal state
        Ok(())
    }

    pub async fn delete_theme(&self, name: &str) -> Result<()> {
        let path = self.theme_dir.join(format!("{}.yaml", name));
        if path.exists() {
            fs::remove_file(&path).await?;
            log::info!("Theme '{}' deleted from {:?}", name, path);
            self.load_themes_from_disk().await?; // Reload to update internal state
            Ok(())
        } else {
            Err(anyhow!("Theme file for '{}' not found at {:?}", name, path))
        }
    }
}
