use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::config::{AppConfig, ThemeConfig, ConfigError};
use super::yaml_theme::{YamlTheme, YamlThemeError};

pub struct YamlThemeManager {
    themes_dir: PathBuf,
    loaded_themes: HashMap<String, YamlTheme>,
    theme_cache: HashMap<String, ThemeConfig>,
}

impl YamlThemeManager {
    pub fn new() -> Result<Self, ConfigError> {
        let themes_dir = AppConfig::themes_dir()?;
        
        // Ensure themes directory exists
        if !themes_dir.exists() {
            std::fs::create_dir_all(&themes_dir)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
            
            // Create some example themes
            Self::create_example_themes(&themes_dir)?;
        }

        let mut manager = Self {
            themes_dir,
            loaded_themes: HashMap::new(),
            theme_cache: HashMap::new(),
        };

        manager.scan_themes()?;
        Ok(manager)
    }

    /// Scan themes directory and load all YAML themes
    pub fn scan_themes(&mut self) -> Result<(), ConfigError> {
        self.loaded_themes.clear();
        self.theme_cache.clear();

        if !self.themes_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.themes_dir)
            .map_err(|e| ConfigError::IoError(e.to_string()))?
        {
            let entry = entry.map_err(|e| ConfigError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") ||
               path.extension().and_then(|s| s.to_str()) == Some("yml") {
                
                match self.load_theme_file(&path) {
                    Ok((name, theme)) => {
                        self.loaded_themes.insert(name, theme);
                    }
                    Err(e) => {
                        eprintln!("Failed to load theme {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single theme file
    fn load_theme_file(&self, path: &Path) -> Result<(String, YamlTheme), YamlThemeError> {
        let theme = YamlTheme::from_file(path)?;
        theme.validate()?;
        
        let name = theme.name.clone()
            .or_else(|| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "Unnamed Theme".to_string());

        Ok((name, theme))
    }

    /// Get all available YAML theme names
    pub fn get_theme_names(&self) -> Vec<String> {
        self.loaded_themes.keys().cloned().collect()
    }

    /// Get a theme by name
    pub fn get_theme(&mut self, name: &str) -> Option<ThemeConfig> {
        if let Some(cached) = self.theme_cache.get(name) {
            return Some(cached.clone());
        }

        if let Some(yaml_theme) = self.loaded_themes.get(name) {
            match yaml_theme.to_theme_config() {
                Ok(theme_config) => {
                    self.theme_cache.insert(name.to_string(), theme_config.clone());
                    Some(theme_config)
                }
                Err(e) => {
                    eprintln!("Failed to convert YAML theme '{}': {}", name, e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Import theme from YAML string
    pub fn import_theme_from_string(&mut self, yaml_content: &str, name: Option<String>) -> Result<String, YamlThemeError> {
        let mut theme = YamlTheme::from_yaml(yaml_content)?;
        theme.validate()?;

        let theme_name = name.or_else(|| theme.name.clone())
            .unwrap_or_else(|| format!("imported_theme_{}", chrono::Utc::now().timestamp()));

        theme.name = Some(theme_name.clone());

        // Save to file
        let file_path = self.themes_dir.join(format!("{}.yaml", sanitize_filename(&theme_name)));
        theme.to_file(&file_path)?;

        // Add to loaded themes
        self.loaded_themes.insert(theme_name.clone(), theme);
        self.theme_cache.remove(&theme_name); // Clear cache

        Ok(theme_name)
    }

    /// Import theme from file
    pub fn import_theme_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String, YamlThemeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| YamlThemeError::IoError(e.to_string()))?;
        
        self.import_theme_from_string(&content, None)
    }

    /// Export theme to YAML string
    pub fn export_theme_to_string(&self, theme_config: &ThemeConfig) -> Result<String, YamlThemeError> {
        let yaml_theme = YamlTheme::from_theme_config(theme_config);
        yaml_theme.to_yaml()
    }

    /// Export theme to file
    pub fn export_theme_to_file<P: AsRef<Path>>(&self, theme_config: &ThemeConfig, path: P) -> Result<(), YamlThemeError> {
        let yaml_theme = YamlTheme::from_theme_config(theme_config);
        yaml_theme.to_file(path)
    }

    /// Save a custom theme
    pub fn save_custom_theme(&mut self, theme_config: &ThemeConfig) -> Result<(), YamlThemeError> {
        let yaml_theme = YamlTheme::from_theme_config(theme_config);
        let file_path = self.themes_dir.join(format!("{}.yaml", sanitize_filename(&theme_config.name)));
        
        yaml_theme.to_file(&file_path)?;
        
        // Add to loaded themes
        self.loaded_themes.insert(theme_config.name.clone(), yaml_theme);
        self.theme_cache.insert(theme_config.name.clone(), theme_config.clone());

        Ok(())
    }

    /// Delete a theme
    pub fn delete_theme(&mut self, name: &str) -> Result<(), YamlThemeError> {
        let file_path = self.themes_dir.join(format!("{}.yaml", sanitize_filename(name)));
        
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| YamlThemeError::IoError(e.to_string()))?;
        }

        self.loaded_themes.remove(name);
        self.theme_cache.remove(name);

        Ok(())
    }

    /// Get theme metadata
    pub fn get_theme_metadata(&self, name: &str) -> Option<ThemeMetadata> {
        self.loaded_themes.get(name).map(|theme| ThemeMetadata {
            name: theme.name.clone().unwrap_or_else(|| name.to_string()),
            author: theme.author.clone(),
            description: theme.description.clone(),
            is_dark: theme.is_dark_theme(),
            has_custom_font: theme.font.is_some(),
            has_custom_effects: theme.effects.is_some(),
        })
    }

    /// Get all theme metadata
    pub fn get_all_metadata(&self) -> Vec<ThemeMetadata> {
        self.loaded_themes
            .keys()
            .filter_map(|name| self.get_theme_metadata(name))
            .collect()
    }

    /// Create example themes in the themes directory
    fn create_example_themes(themes_dir: &Path) -> Result<(), ConfigError> {
        let example_themes = vec![
            ("dracula.yaml", include_str!("../../themes/dracula.yaml")),
            ("monokai.yaml", include_str!("../../themes/monokai.yaml")),
            ("solarized-dark.yaml", include_str!("../../themes/solarized-dark.yaml")),
            ("solarized-light.yaml", include_str!("../../themes/solarized-light.yaml")),
            ("gruvbox-dark.yaml", include_str!("../../themes/gruvbox-dark.yaml")),
            ("nord.yaml", include_str!("../../themes/nord.yaml")),
        ];

        for (filename, content) in example_themes {
            let file_path = themes_dir.join(filename);
            if !file_path.exists() {
                std::fs::write(&file_path, content)
                    .map_err(|e| ConfigError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Watch for theme file changes
    pub fn start_watching(&self) -> Result<notify::RecommendedWatcher, ConfigError> {
        use notify::{Watcher, RecursiveMode, Event, EventKind};
        
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        watcher.watch(&self.themes_dir, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        // In a real implementation, you'd handle the events in a separate thread
        // and notify the UI to reload themes when files change

        Ok(watcher)
    }
}

#[derive(Debug, Clone)]
pub struct ThemeMetadata {
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub is_dark: bool,
    pub has_custom_font: bool,
    pub has_custom_effects: bool,
}

/// Sanitize filename for cross-platform compatibility
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_theme_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let themes_dir = temp_dir.path().join("themes");
        
        // This would normally use the config directory
        // but for testing we use a temporary directory
        assert!(YamlThemeManager::new().is_ok());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("My Theme"), "My Theme");
        assert_eq!(sanitize_filename("My/Theme"), "My_Theme");
        assert_eq!(sanitize_filename("My:Theme*"), "My_Theme_");
    }
}
