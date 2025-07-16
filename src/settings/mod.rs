use iced::{Element, widget::{column, row, text, button, container, scrollable, pick_list, slider, checkbox, text_input}};
use crate::{Message, config::*};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::config::preferences::Preferences;
use crate::config::theme::ThemeConfig;
use crate::agent_mode_eval::ai_client::AiConfig;
use anyhow::Result;
use std::sync::Arc;
use crate::config::ConfigManager;

pub mod theme_editor;
pub mod keybinding_editor;
pub mod yaml_theme_ui;

use theme_editor::ThemeEditor;
use keybinding_editor::KeyBindingEditor;
use yaml_theme_ui::YamlThemeUI; // Changed from YamlThemeEditor to YamlThemeUI

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    General,
    Appearance,
    Terminal,
    Editor,
    KeyBindings,
    Performance,
    Privacy,
    Plugins,
    EnvironmentProfiles, // New tab for environment profiles
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    TabChanged(SettingsTab),
    ConfigChanged(ConfigChange),
    ThemeChanged(String), // For selecting built-in themes
    CustomThemeCreated(String), // For saving a new custom theme
    KeyBindingChanged(String, KeyBinding), // For individual keybinding changes
    ResetToDefaults,
    ImportConfig,
    ExportConfig,
    Save,
    Cancel,
    ThemeEditor(theme_editor::Message),
    KeyBindingEditor(keybinding_editor::Message),
    YamlThemeUI(yaml_theme_ui::Message), // Changed to YamlThemeUI

    // Environment Profile Management
    SelectEnvironmentProfile(String), // For setting active profile
    EditEnvironmentProfile(Option<String>), // None for new, Some(name) for existing
    SaveEnvironmentProfile,
    CancelEditEnvironmentProfile,
    DeleteEnvironmentProfile(String),
    EnvironmentProfileNameChanged(String),
    EnvironmentVariableKeyChanged(usize, String), // index, new_key
    EnvironmentVariableValueChanged(usize, String), // index, new_value
    AddEnvironmentVariable,
    RemoveEnvironmentVariable(usize), // index
}

#[derive(Debug, Clone)]
pub enum ConfigChange {
    // General
    StartupBehavior(StartupBehavior),
    DefaultShell(String),
    WorkingDirectory(WorkingDirectoryBehavior),
    AutoUpdate(bool),
    TelemetryEnabled(bool),
    
    // Terminal
    ScrollbackLines(usize),
    ScrollSensitivity(f32),
    MouseReporting(bool),
    CopyOnSelect(bool),
    PasteOnRightClick(bool),
    ConfirmBeforeClosing(bool),
    BellBehavior(BellBehavior),
    CursorStyle(CursorStyle),
    CursorBlink(bool),
    
    // Editor
    VimMode(bool),
    AutoSuggestions(bool),
    SyntaxHighlighting(bool),
    AutoCompletion(bool),
    IndentSize(usize),
    TabWidth(usize),
    InsertSpaces(bool),
    
    // UI
    ShowTabBar(TabBarVisibility),
    ShowTitleBar(bool),
    CompactMode(bool),
    Transparency(f32),
    BlurBackground(bool),
    AnimationsEnabled(bool),
    ZoomLevel(f32),
    
    // Performance
    GpuAcceleration(bool),
    Vsync(bool),
    MaxFps(Option<u32>),
    MemoryLimit(Option<usize>),
    
    // Privacy
    HistoryEnabled(bool),
    HistoryLimit(usize),
    ClearHistoryOnExit(bool),
    IncognitoMode(bool),
    LogLevel(LogLevel),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub preferences: Preferences,
    pub theme: ThemeConfig,
    pub ai: AiConfig,
    pub environment_profiles: HashMap<String, HashMap<String, String>>,
    // Add other top-level configuration sections here
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            preferences: Preferences::default(),
            theme: ThemeConfig::default(),
            ai: AiConfig::default(),
            environment_profiles: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let settings: Settings = serde_yaml::from_str(&content)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_default_settings_path() -> PathBuf {
        // Example: ~/.config/neoterm/settings.yaml or %APPDATA%/neoterm/settings.yaml
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("neoterm")
            .join("settings.yaml")
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

    pub fn ensure_settings_dirs_exist() -> Result<(), Box<dyn std::error::Error>> {
        let settings_dir = Self::get_default_settings_path().parent().unwrap().to_path_buf();
        fs::create_dir_all(&settings_dir)?;
        fs::create_dir_all(Self::get_themes_dir())?;
        fs::create_dir_all(Self::get_workflows_dir())?;
        fs::create_dir_all(Self::get_environment_profiles_dir())?;
        Ok(())
    }
}

pub struct SettingsManager {
    config_manager: Arc<ConfigManager>,
    keybinding_editor: keybinding_editor::KeybindingEditor,
    theme_editor: theme_editor::ThemeEditor,
    yaml_theme_ui: yaml_theme_ui::YamlThemeUI,
}

impl SettingsManager {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self {
            config_manager: config_manager.clone(),
            keybinding_editor: keybinding_editor::KeybindingEditor::new(),
            theme_editor: theme_editor::ThemeEditor::new(config_manager.clone()),
            yaml_theme_ui: yaml_theme_ui::YamlThemeUI::new(config_manager.clone()),
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Settings manager initialized.");
        self.keybinding_editor.init().await?;
        self.theme_editor.init().await?;
        self.yaml_theme_ui.init().await?;
        Ok(())
    }

    pub fn get_keybinding_editor(&self) -> &keybinding_editor::KeybindingEditor {
        &self.keybinding_editor
    }

    pub fn get_theme_editor(&self) -> &theme_editor::ThemeEditor {
        &self.theme_editor
    }

    pub fn get_yaml_theme_ui(&self) -> &yaml_theme_ui::YamlThemeUI {
        &self.yaml_theme_ui
    }
}

pub fn init() {
    log::info!("Settings module initialized.");
}
