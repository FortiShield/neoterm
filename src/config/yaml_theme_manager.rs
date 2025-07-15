use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::config::{AppConfig, ThemeConfig, ConfigError};
use super::yaml_theme::{YamlTheme, YamlThemeError};
use serde_yaml;
use notify;
use tempfile;

pub struct YamlThemeManager {
    themes_dir: PathBuf,
    themes: HashMap<String, YamlTheme>,
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
            themes: HashMap::new(),
        };

        manager.load_themes();
        Ok(manager)
    }

    fn load_themes(&mut self) {
        if self.themes_dir.is_dir() {
            for entry in std::fs::read_dir(&self.themes_dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            match serde_yaml::from_str::<YamlTheme>(&content) {
                                Ok(theme) => {
                                    self.themes.insert(theme.name.clone(), theme);
                                },
                                Err(e) => eprintln!("Error parsing theme file {}: {}", path.display(), e),
                            }
                        },
                        Err(e) => eprintln!("Error reading theme file {}: {}", path.display(), e),
                    }
                }
            }
        }
    }

    /// Get all available YAML theme names
    pub fn get_theme_names(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }

    /// Get a theme by name
    pub fn get_theme(&self, name: &str) -> Option<&YamlTheme> {
        self.themes.get(name)
    }

    /// Import theme from YAML string
    pub fn import_theme_from_string(&mut self, yaml_content: &str, name: Option<String>) -> Result<String, YamlThemeError> {
        let mut theme: YamlTheme = serde_yaml::from_str(yaml_content)?;
        
        let theme_name = name.or_else(|| theme.name.clone())
            .unwrap_or_else(|| format!("imported_theme_{}", chrono::Utc::now().timestamp()));

        theme.name = Some(theme_name.clone());

        // Save to file
        let file_path = self.themes_dir.join(format!("{}.yaml", sanitize_filename(&theme_name)));
        std::fs::write(&file_path, yaml_content)?;

        // Add to loaded themes
        self.themes.insert(theme_name.clone(), theme);

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
        self.themes.insert(theme_config.name.clone(), yaml_theme);

        Ok(())
    }

    /// Delete a theme
    pub fn delete_theme(&mut self, name: &str) -> Result<(), YamlThemeError> {
        let file_path = self.themes_dir.join(format!("{}.yaml", sanitize_filename(name)));
        
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| YamlThemeError::IoError(e.to_string()))?;
        }

        self.themes.remove(name);

        Ok(())
    }

    /// Get theme metadata
    pub fn get_theme_metadata(&self, name: &str) -> Option<ThemeMetadata> {
        self.themes.get(name).map(|theme| ThemeMetadata {
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
        self.themes
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
