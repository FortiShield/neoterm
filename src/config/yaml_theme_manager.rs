use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::config::theme::{ThemeConfig, TerminalColors};
use crate::config::yaml_theme::YamlTheme;
use serde_yaml;
use notify;
use tempfile;

pub struct YamlThemeManager {
    theme_dir: PathBuf,
    loaded_themes: HashMap<String, YamlTheme>,
}

impl YamlThemeManager {
    pub fn new(theme_dir: PathBuf) -> Self {
        Self {
            theme_dir,
            loaded_themes: HashMap::new(),
        }
    }

    pub fn load_themes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.loaded_themes.clear();
        if !self.theme_dir.exists() {
            fs::create_dir_all(&self.theme_dir)?;
        }

        for entry in fs::read_dir(&self.theme_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_yaml::from_str::<YamlTheme>(&content) {
                            Ok(theme) => {
                                println!("Loaded theme: {}", theme.name);
                                self.loaded_themes.insert(theme.name.clone(), theme);
                            },
                            Err(e) => eprintln!("Error parsing theme file {}: {}", path.display(), e),
                        }
                    },
                    Err(e) => eprintln!("Error reading theme file {}: {}", path.display(), e),
                }
            }
        }
        Ok(())
    }

    pub fn get_theme_config(&self, name: &str) -> Option<ThemeConfig> {
        self.loaded_themes.get(name).map(|yaml_theme| {
            let mut colors = HashMap::new();
            colors.insert("primary".to_string(), yaml_theme.colors.custom.get("primary").cloned().unwrap_or_else(|| "#007bff".to_string()));
            colors.insert("secondary".to_string(), yaml_theme.colors.custom.get("secondary").cloned().unwrap_or_else(|| "#6c757d".to_string()));
            colors.insert("success".to_string(), yaml_theme.colors.custom.get("success").cloned().unwrap_or_else(|| "#28a745".to_string()));
            colors.insert("danger".to_string(), yaml_theme.colors.custom.get("danger").cloned().unwrap_or_else(|| "#dc3545".to_string()));
            colors.insert("warning".to_string(), yaml_theme.colors.custom.get("warning").cloned().unwrap_or_else(|| "#ffc107".to_string()));
            colors.insert("info".to_string(), yaml_theme.colors.custom.get("info").cloned().unwrap_or_else(|| "#17a2b8".to_string()));
            // Add other custom colors from yaml_theme.colors.custom
            for (key, value) in &yaml_theme.colors.custom {
                colors.insert(key.clone(), value.clone());
            }

            ThemeConfig {
                name: yaml_theme.name.clone(),
                colors,
                syntax_highlighting: yaml_theme.syntax_highlighting.clone(),
                terminal_colors: TerminalColors {
                    background: yaml_theme.colors.background.clone(),
                    foreground: yaml_theme.colors.foreground.clone(),
                    cursor: yaml_theme.colors.cursor.clone(),
                    selection: yaml_theme.colors.selection.clone(),
                    black: yaml_theme.colors.black.clone(),
                    red: yaml_theme.colors.red.clone(),
                    green: yaml_theme.colors.green.clone(),
                    yellow: yaml_theme.colors.yellow.clone(),
                    blue: yaml_theme.colors.blue.clone(),
                    magenta: yaml_theme.colors.magenta.clone(),
                    cyan: yaml_theme.colors.cyan.clone(),
                    white: yaml_theme.colors.white.clone(),
                    bright_black: yaml_theme.colors.bright_black.clone(),
                    bright_red: yaml_theme.colors.bright_red.clone(),
                    bright_green: yaml_theme.colors.bright_green.clone(),
                    bright_yellow: yaml_theme.colors.bright_yellow.clone(),
                    bright_blue: yaml_theme.colors.bright_blue.clone(),
                    bright_magenta: yaml_theme.colors.bright_magenta.clone(),
                    bright_cyan: yaml_theme.colors.bright_cyan.clone(),
                    bright_white: yaml_theme.colors.bright_white.clone(),
                },
            }
        })
    }

    pub fn get_available_theme_names(&self) -> Vec<String> {
        self.loaded_themes.keys().cloned().collect()
    }

    /// Import theme from YAML string
    pub fn import_theme_from_string(&mut self, yaml_content: &str, name: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
        let mut theme: YamlTheme = serde_yaml::from_str(yaml_content)?;
        
        let theme_name = name.or_else(|| theme.name.clone())
            .unwrap_or_else(|| format!("imported_theme_{}", chrono::Utc::now().timestamp()));

        theme.name = Some(theme_name.clone());

        // Save to file
        let file_path = self.theme_dir.join(format!("{}.yaml", sanitize_filename(&theme_name)));
        fs::write(&file_path, yaml_content)?;

        // Add to loaded themes
        self.loaded_themes.insert(theme_name.clone(), theme);

        Ok(theme_name)
    }

    /// Import theme from file
    pub fn import_theme_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)
            .map_err(|e| e.to_string())?;
        
        self.import_theme_from_string(&content, None)
    }

    /// Export theme to YAML string
    pub fn export_theme_to_string(&self, theme_config: &ThemeConfig) -> Result<String, Box<dyn std::error::Error>> {
        let yaml_theme = YamlTheme::from_theme_config(theme_config);
        yaml_theme.to_yaml()
    }

    /// Export theme to file
    pub fn export_theme_to_file<P: AsRef<Path>>(&self, theme_config: &ThemeConfig, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let yaml_theme = YamlTheme::from_theme_config(theme_config);
        yaml_theme.to_file(path)
    }

    /// Save a custom theme
    pub fn save_custom_theme(&mut self, theme_config: &ThemeConfig) -> Result<(), Box<dyn std::error::Error>> {
        let yaml_theme = YamlTheme::from_theme_config(theme_config);
        let file_path = self.theme_dir.join(format!("{}.yaml", sanitize_filename(&theme_config.name)));
        
        yaml_theme.to_file(&file_path)?;
        
        // Add to loaded themes
        self.loaded_themes.insert(theme_config.name.clone(), yaml_theme);

        Ok(())
    }

    /// Delete a theme
    pub fn delete_theme(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = self.theme_dir.join(format!("{}.yaml", sanitize_filename(name)));
        
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }

        self.loaded_themes.remove(name);

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

    /// Watch for theme file changes
    pub fn start_watching(&self) -> Result<notify::RecommendedWatcher, Box<dyn std::error::Error>> {
        use notify::{Watcher, RecursiveMode, Event, EventKind};
        
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(&self.theme_dir, RecursiveMode::NonRecursive)?;

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
        assert!(YamlThemeManager::new(themes_dir).is_ok());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("My Theme"), "My Theme");
        assert_eq!(sanitize_filename("My/Theme"), "My_Theme");
        assert_eq!(sanitize_filename("My:Theme*"), "My_Theme_");
    }
}

pub fn init() {
    println!("config/yaml_theme_manager module loaded");
}
