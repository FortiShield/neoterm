use iced::{Element, widget::{column, row, text, button, container, scrollable, pick_list, slider, checkbox, text_input}};
use crate::{Message, config::*};
use std::collections::HashMap;

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

pub struct SettingsView {
    pub active_tab: SettingsTab,
    pub config: AppConfig,
    pub theme_editor: ThemeEditor,
    pub keybinding_editor: KeyBindingEditor,
    pub yaml_theme_ui: YamlThemeUI, // Changed to YamlThemeUI
    pub unsaved_changes: bool,
    
    // Environment Profile Editor State
    pub editing_env_profile: Option<EnvProfile>, // Changed from EnvironmentProfile to EnvProfile
    pub editing_env_profile_original_name: Option<String>, // To track if it's a new profile or an edit
    pub env_profile_error: Option<String>, // For validation errors
}

impl SettingsView {
    pub fn new(config: AppConfig) -> Self {
        Self {
            active_tab: SettingsTab::General,
            theme_editor: ThemeEditor::new(config.theme.clone()),
            keybinding_editor: KeyBindingEditor::new(config.keybindings.clone()),
            yaml_theme_ui: YamlThemeUI::new().unwrap_or_else(|e| { // Handle error for YamlThemeUI
                eprintln!("Failed to initialize YamlThemeUI: {}", e);
                YamlThemeUI::new_empty() // Provide a fallback empty instance
            }),
            config,
            unsaved_changes: false,
            editing_env_profile: None,
            editing_env_profile_original_name: None,
            env_profile_error: None,
        }
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::TabChanged(tab) => {
                self.active_tab = tab;
                self.editing_env_profile = None; // Close editor when changing tabs
                self.env_profile_error = None;
            }
            SettingsMessage::ConfigChanged(change) => {
                self.apply_config_change(change);
                self.unsaved_changes = true;
            }
            SettingsMessage::ThemeChanged(theme_name) => {
                if let Some(theme) = ThemeConfig::builtin_themes()
                    .into_iter()
                    .find(|t| t.name == theme_name)
                {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
            }
            SettingsMessage::Save => {
                if let Err(e) = self.config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
                self.unsaved_changes = false;
            }
            SettingsMessage::Cancel => {
                // Reload config from disk
                if let Ok(config) = AppConfig::load() {
                    self.config = config.clone();
                    self.unsaved_changes = false;
                    self.editing_env_profile = None; // Reset editor state
                    self.env_profile_error = None;
                } else {
                    // Handle error loading config
                }
            }
            SettingsMessage::ResetToDefaults => {
                self.config = AppConfig::default();
                self.unsaved_changes = true;
                self.editing_env_profile = None; // Reset editor state
                self.env_profile_error = None;
            }
            SettingsMessage::ThemeEditor(msg) => {
                if let Some(theme) = self.theme_editor.update(msg) {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
            }
            SettingsMessage::KeyBindingEditor(msg) => {
                if let Some(keybindings) = self.keybinding_editor.update(msg) {
                    self.config.keybindings = keybindings;
                    self.unsaved_changes = true;
                }
            }
            SettingsMessage::YamlThemeUI(msg) => { // Changed to YamlThemeUI
                if let Some(theme) = self.yaml_theme_ui.update(msg) {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
            }
            // Environment Profile Messages
            SettingsMessage::SelectEnvironmentProfile(name) => {
                self.config.env_profiles.active_profile = Some(name);
                self.unsaved_changes = true;
            }
            SettingsMessage::EditEnvironmentProfile(name_opt) => {
                self.env_profile_error = None;
                if let Some(name) = name_opt {
                    if let Some(profile) = self.config.env_profiles.profiles.get(&name) {
                        self.editing_env_profile = Some(profile.clone());
                        self.editing_env_profile_original_name = Some(name);
                    }
                } else {
                    // New profile
                    self.editing_env_profile = Some(EnvProfile { // Changed to EnvProfile
                        variables: HashMap::new(),
                    });
                    self.editing_env_profile_original_name = None;
                }
            }
            SettingsMessage::SaveEnvironmentProfile => {
                if let Some(mut profile_to_save) = self.editing_env_profile.take() {
                    // Validate name
                    let profile_name = self.editing_env_profile_original_name.as_ref().unwrap_or(&profile_to_save.variables.get("name").cloned().unwrap_or_default()).clone();
                    if profile_name.trim().is_empty() {
                        self.env_profile_error = Some("Profile name cannot be empty.".to_string());
                        self.editing_env_profile = Some(profile_to_save); // Restore for editing
                        return;
                    }
                    
                    // Check for duplicate name if it's a new profile or name changed
                    if let Some(original_name) = &self.editing_env_profile_original_name {
                        if original_name != &profile_name && self.config.env_profiles.profiles.contains_key(&profile_name) {
                            self.env_profile_error = Some(format!("Profile with name '{}' already exists.", profile_name));
                            self.editing_env_profile = Some(profile_to_save);
                            return;
                        }
                        // Remove old entry if name changed
                        if original_name != &profile_name {
                            self.config.env_profiles.profiles.remove(original_name);
                            // If the old profile was active, switch to the new one
                            if self.config.env_profiles.active_profile.as_ref() == Some(original_name) {
                                self.config.env_profiles.active_profile = Some(profile_name.clone());
                            }
                        }
                    } else { // It's a new profile
                        if self.config.env_profiles.profiles.contains_key(&profile_name) {
                            self.env_profile_error = Some(format!("Profile with name '{}' already exists.", profile_name));
                            self.editing_env_profile = Some(profile_to_save);
                            return;
                        }
                    }

                    // Clean up empty variables
                    profile_to_save.variables.retain(|k, v| !k.trim().is_empty() || !v.trim().is_empty());

                    self.config.env_profiles.profiles.insert(profile_name.clone(), profile_to_save);
                    self.editing_env_profile_original_name = None;
                    self.unsaved_changes = true;
                    self.env_profile_error = None;
                }
            }
            SettingsMessage::CancelEditEnvironmentProfile => {
                self.editing_env_profile = None;
                self.editing_env_profile_original_name = None;
                self.env_profile_error = None;
            }
            SettingsMessage::DeleteEnvironmentProfile(name) => {
                if self.config.env_profiles.profiles.remove(&name).is_some() {
                    if self.config.env_profiles.active_profile.as_ref() == Some(&name) {
                        self.config.env_profiles.active_profile = self.config.env_profiles.profiles.keys().next().cloned();
                    }
                    self.unsaved_changes = true;
                }
            }
            SettingsMessage::EnvironmentProfileNameChanged(new_name) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    profile.variables.insert("name".to_string(), new_name);
                }
            }
            SettingsMessage::EnvironmentVariableKeyChanged(index, new_key) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<(String, String)> = profile.variables.drain().collect();
                    if let Some((key, _)) = vars.get_mut(index) {
                        *key = new_key;
                    }
                    profile.variables = vars.into_iter().collect();
                }
            }
            SettingsMessage::EnvironmentVariableValueChanged(index, new_value) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<(String, String)> = profile.variables.drain().collect();
                    if let Some((_, value)) = vars.get_mut(index) {
                        *value = new_value;
                    }
                    profile.variables = vars.into_iter().collect();
                }
            }
            SettingsMessage::AddEnvironmentVariable => {
                if let Some(profile) = &mut self.editing_env_profile {
                    profile.variables.insert("".to_string(), "".to_string());
                }
            }
            SettingsMessage::RemoveEnvironmentVariable(index) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<(String, String)> = profile.variables.drain().collect();
                    if index < vars.len() {
                        vars.remove(index);
                    }
                    profile.variables = vars.into_iter().collect();
                }
            }
        }
    }

    fn apply_config_change(&mut self, change: ConfigChange) {
        match change {
            ConfigChange::StartupBehavior(behavior) => self.config.preferences.general.startup_behavior = behavior,
            ConfigChange::DefaultShell(shell) => self.config.preferences.general.default_shell = Some(shell),
            ConfigChange::WorkingDirectory(behavior) => self.config.preferences.general.working_directory = behavior,
            ConfigChange::AutoUpdate(enabled) => self.config.preferences.general.auto_update = enabled,
            ConfigChange::TelemetryEnabled(enabled) => self.config.preferences.general.telemetry_enabled = enabled,
            ConfigChange::ScrollbackLines(lines) => self.config.preferences.terminal.scrollback_lines = lines,
            ConfigChange::ScrollSensitivity(sensitivity) => self.config.preferences.terminal.scroll_sensitivity = sensitivity,
            ConfigChange::MouseReporting(enabled) => self.config.preferences.terminal.mouse_reporting = enabled,
            ConfigChange::CopyOnSelect(enabled) => self.config.preferences.terminal.copy_on_select = enabled,
            ConfigChange::PasteOnRightClick(enabled) => self.config.preferences.terminal.paste_on_right_click = enabled,
            ConfigChange::ConfirmBeforeClosing(enabled) => self.config.preferences.terminal.confirm_before_closing = enabled,
            ConfigChange::BellBehavior(behavior) => self.config.preferences.terminal.bell_behavior = behavior,
            ConfigChange::CursorStyle(style) => self.config.preferences.terminal.cursor_style = style,
            ConfigChange::CursorBlink(enabled) => self.config.preferences.terminal.cursor_blink = enabled,
            ConfigChange::VimMode(enabled) => self.config.preferences.editor.vim_mode = enabled,
            ConfigChange::AutoSuggestions(enabled) => self.config.preferences.editor.auto_suggestions = enabled,
            ConfigChange::SyntaxHighlighting(enabled) => self.config.preferences.editor.syntax_highlighting = enabled,
            ConfigChange::AutoCompletion(enabled) => self.config.preferences.editor.auto_completion = enabled,
            ConfigChange::IndentSize(size) => self.config.preferences.editor.indent_size = size,
            ConfigChange::TabWidth(width) => self.config.preferences.editor.tab_width = width,
            ConfigChange::InsertSpaces(enabled) => self.config.preferences.editor.insert_spaces = enabled,
            ConfigChange::ShowTabBar(visibility) => self.config.preferences.ui.show_tab_bar = visibility,
            ConfigChange::ShowTitleBar(enabled) => self.config.preferences.ui.show_title_bar = enabled,
            ConfigChange::CompactMode(enabled) => self.config.preferences.ui.compact_mode = enabled,
            ConfigChange::Transparency(value) => self.config.preferences.ui.transparency = value,
            ConfigChange::BlurBackground(enabled) => self.config.preferences.ui.blur_background = enabled,
            ConfigChange::AnimationsEnabled(enabled) => self.config.preferences.ui.animations_enabled = enabled,
            ConfigChange::ZoomLevel(level) => self.config.preferences.ui.zoom_level = level,
            ConfigChange::GpuAcceleration(enabled) => self.config.preferences.performance.gpu_acceleration = enabled,
            ConfigChange::Vsync(enabled) => self.config.preferences.performance.vsync = enabled,
            ConfigChange::MaxFps(fps) => self.config.preferences.performance.max_fps = fps,
            ConfigChange::MemoryLimit(limit) => self.config.preferences.performance.memory_limit = limit,
            ConfigChange::HistoryEnabled(enabled) => self.config.preferences.privacy.history_enabled = enabled,
            ConfigChange::HistoryLimit(limit) => self.config.preferences.privacy.history_limit = limit,
            ConfigChange::ClearHistoryOnExit(enabled) => self.config.preferences.privacy.clear_history_on_exit = enabled,
            ConfigChange::IncognitoMode(enabled) => self.config.preferences.privacy.incognito_mode = enabled,
            ConfigChange::LogLevel(level) => self.config.preferences.privacy.log_level = level,
        }
    }

    pub fn view(&mut self) -> Element<SettingsMessage> {
        let tabs = row![
            button(text("General")).on_press(SettingsMessage::TabChanged(SettingsTab::General)),
            button(text("Appearance")).on_press(SettingsMessage::TabChanged(SettingsTab::Appearance)),
            button(text("Terminal")).on_press(SettingsTab::Terminal),
            button(text("Editor")).on_press(SettingsTab::Editor),
            button(text("Key Bindings")).on_press(SettingsTab::KeyBindings),
            button(text("Performance")).on_press(SettingsTab::Performance),
            button(text("Privacy")).on_press(SettingsTab::Privacy),
            button(text("Plugins")).on_press(SettingsTab::Plugins),
            button(text("Environment Profiles")).on_press(SettingsTab::EnvironmentProfiles),
        ]
        .spacing(10);

        let content = match self.active_tab {
            SettingsTab::General => self.create_general_settings(),
            SettingsTab::Appearance => self.create_appearance_settings(),
            SettingsTab::Terminal => self.create_terminal_settings(),
            SettingsTab::Editor => self.create_editor_settings(),
            SettingsTab::KeyBindings => self.create_keybinding_settings(),
            SettingsTab::Performance => self.create_performance_settings(),
            SettingsTab::Privacy => self.create_privacy_settings(),
            SettingsTab::Plugins => self.create_plugin_settings(),
            SettingsTab::EnvironmentProfiles => self.create_environment_profiles_settings(),
        };

        column![
            text("Settings").size(30),
            tabs,
            content,
            row![
                button(text("Save")).on_press(SettingsMessage::Save),
                button(text("Cancel")).on_press(SettingsMessage::Cancel),
                button(text("Reset to Defaults")).on_press(SettingsMessage::ResetToDefaults),
            ].spacing(10)
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    fn create_general_settings(&self) -> Element<SettingsMessage> {
        column![
            text("General Settings").size(20),
            
            row![
                text("Startup Behavior:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    vec![
                        StartupBehavior::NewSession,
                        StartupBehavior::RestoreLastSession,
                        StartupBehavior::CustomCommand("".to_string()), // Placeholder for custom command
                    ],
                    Some(self.config.preferences.general.startup_behavior.clone()),
                    |behavior| SettingsMessage::ConfigChanged(ConfigChange::StartupBehavior(behavior))
                )
            ].spacing(8),
            
            row![
                text("Default Shell:").width(iced::Length::Fixed(150.0)),
                text_input(
                    "Shell path...",
                    self.config.preferences.general.default_shell.as_deref().unwrap_or("")
                )
                .on_input(|shell| SettingsMessage::ConfigChanged(ConfigChange::DefaultShell(shell)))
            ].spacing(8),
            
            row![
                checkbox(
                    "Auto Update",
                    self.config.preferences.general.auto_update,
                    |enabled| SettingsMessage::ConfigChanged(ConfigChange::AutoUpdate(enabled))
                ),
                text("Automatically check for and install updates")
            ].spacing(8),
            
            row![
                checkbox(
                    "Telemetry",
                    self.config.preferences.general.telemetry_enabled,
                    |enabled| SettingsMessage::ConfigChanged(ConfigChange::TelemetryEnabled(enabled))
                ),
                text("Help improve NeoTerm by sharing anonymous usage data")
            ].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn create_appearance_settings(&mut self) -> Element<SettingsMessage> {
        let theme_names: Vec<String> = ThemeConfig::builtin_themes()
            .into_iter()
            .map(|t| t.name)
            .collect();

        column![
            text("Appearance Settings").size(20),
            
            row![
                text("Theme:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    theme_names,
                    Some(self.config.theme.name.clone()),
                    SettingsMessage::ThemeChanged
                )
            ].spacing(8),
            
            row![
                text("Font Family:").width(iced::Length::Fixed(150.0)),
                text_input(
                    "Font name...",
                    &self.config.theme.typography.font_family
                )
                .on_input(|_font| SettingsMessage::ConfigChanged(ConfigChange::AutoUpdate(true))) // Placeholder
            ].spacing(8),
            
            row![
                text("Font Size:").width(iced::Length::Fixed(150.0)),
                slider(8.0..=24.0, self.config.preferences.font_size as f32, |size| {
                    // This would need to be handled differently in a real implementation
                    SettingsMessage::ConfigChanged(ConfigChange::AutoUpdate(true)) // Placeholder
                })
            ].spacing(8),
            
            row![
                text("Transparency:").width(iced::Length::Fixed(150.0)),
                slider(0.0..=1.0, self.config.preferences.ui.transparency, |value| {
                    SettingsMessage::ConfigChanged(ConfigChange::Transparency(value))
                })
            ].spacing(8),
            
            checkbox(
                "Blur Background",
                self.config.preferences.ui.blur_background,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::BlurBackground(enabled))
            ),
            
            checkbox(
                "Enable Animations",
                self.config.preferences.ui.animations_enabled,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::AnimationsEnabled(enabled))
            ),
            
            // Theme editor section
            text("Custom Theme Editor").size(16),
            self.theme_editor.view().map(SettingsMessage::ThemeEditor),
            text("YAML Theme Management").size(16),
            self.yaml_theme_ui.view().map(SettingsMessage::YamlThemeUI), // Changed to YamlThemeUI
        ]
        .spacing(16)
        .into()
    }

    fn create_terminal_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Terminal Settings").size(20),
            
            row![
                text("Scrollback Lines:").width(iced::Length::Fixed(150.0)),
                slider(1000.0..=50000.0, self.config.preferences.terminal.scrollback_lines as f32, |lines| {
                    SettingsMessage::ConfigChanged(ConfigChange::ScrollbackLines(lines as usize))
                })
            ].spacing(8),
            
            row![
                text("Scroll Sensitivity:").width(iced::Length::Fixed(150.0)),
                slider(0.1..=5.0, self.config.preferences.terminal.scroll_sensitivity, |sensitivity| {
                    SettingsMessage::ConfigChanged(ConfigChange::ScrollSensitivity(sensitivity))
                })
            ].spacing(8),
            
            checkbox(
                "Copy on Select",
                self.config.preferences.terminal.copy_on_select,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::CopyOnSelect(enabled))
            ),
            
            checkbox(
                "Paste on Right Click",
                self.config.preferences.terminal.paste_on_right_click,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::PasteOnRightClick(enabled))
            ),
            
            checkbox(
                "Confirm Before Closing",
                self.config.preferences.terminal.confirm_before_closing,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::ConfirmBeforeClosing(enabled))
            ),
            
            row![
                text("Cursor Style:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    vec![CursorStyle::Block, CursorStyle::Underline, CursorStyle::Bar],
                    Some(self.config.preferences.terminal.cursor_style.clone()),
                    |style| SettingsMessage::ConfigChanged(ConfigChange::CursorStyle(style))
                )
            ].spacing(8),
            
            checkbox(
                "Cursor Blink",
                self.config.preferences.terminal.cursor_blink,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::CursorBlink(enabled))
            ),
        ]
        .spacing(16)
        .into()
    }

    fn create_editor_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Editor Settings").size(20),
            
            checkbox(
                "Vim Mode",
                self.config.preferences.editor.vim_mode,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::VimMode(enabled))
            ),
            
            checkbox(
                "Auto Suggestions",
                self.config.preferences.editor.auto_suggestions,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::AutoSuggestions(enabled))
            ),
            
            checkbox(
                "Syntax Highlighting",
                self.config.preferences.editor.syntax_highlighting,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::SyntaxHighlighting(enabled))
            ),
            
            checkbox(
                "Auto Completion",
                self.config.preferences.editor.auto_completion,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::AutoCompletion(enabled))
            ),
            
            row![
                text("Indent Size:").width(iced::Length::Fixed(150.0)),
                slider(1.0..=8.0, self.config.preferences.editor.indent_size as f32, |size| {
                    SettingsMessage::ConfigChanged(ConfigChange::IndentSize(size as usize))
                })
            ].spacing(8),
            
            row![
                text("Tab Width:").width(iced::Length::Fixed(150.0)),
                slider(1.0..=8.0, self.config.preferences.editor.tab_width as f32, |width| {
                    SettingsMessage::ConfigChanged(ConfigChange::TabWidth(width as usize))
                })
            ].spacing(8),
            
            checkbox(
                "Insert Spaces",
                self.config.preferences.editor.insert_spaces,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::InsertSpaces(enabled))
            ),
        ]
        .spacing(16)
        .into()
    }

    fn create_keybinding_settings(&mut self) -> Element<SettingsMessage> {
        column![
            text("Key Bindings").size(20),
            self.keybinding_editor.view().map(SettingsMessage::KeyBindingEditor),
        ]
        .spacing(16)
        .into()
    }

    fn create_performance_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Performance Settings").size(20),
            
            checkbox(
                "GPU Acceleration",
                self.config.preferences.performance.gpu_acceleration,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::GpuAcceleration(enabled))
            ),
            
            checkbox(
                "VSync",
                self.config.preferences.performance.vsync,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::Vsync(enabled))
            ),
            
            row![
                text("Max FPS:").width(iced::Length::Fixed(150.0)),
                slider(30.0..=144.0, self.config.preferences.performance.max_fps.unwrap_or(60) as f32, |fps| {
                    SettingsMessage::ConfigChanged(ConfigChange::MaxFps(Some(fps as u32)))
                })
            ].spacing(8),
            
            row![
                text("Memory Limit (MB):").width(iced::Length::Fixed(150.0)),
                slider(256.0..=4096.0, self.config.preferences.performance.memory_limit.unwrap_or(1024) as f32, |mb| {
                    SettingsMessage::ConfigChanged(ConfigChange::MemoryLimit(Some(mb as usize)))
                })
            ].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn create_privacy_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Privacy Settings").size(20),
            
            checkbox(
                "Enable History",
                self.config.preferences.privacy.history_enabled,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::HistoryEnabled(enabled))
            ),
            
            row![
                text("History Limit:").width(iced::Length::Fixed(150.0)),
                slider(100.0..=50000.0, self.config.preferences.privacy.history_limit as f32, |limit| {
                    SettingsMessage::ConfigChanged(ConfigChange::HistoryLimit(limit as usize))
                })
            ].spacing(8),
            
            checkbox(
                "Clear History on Exit",
                self.config.preferences.privacy.clear_history_on_exit,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::ClearHistoryOnExit(enabled))
            ),
            
            checkbox(
                "Incognito Mode",
                self.config.preferences.privacy.incognito_mode,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::IncognitoMode(enabled))
            ),
        ]
        .spacing(16)
        .into()
    }

    fn create_plugin_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Plugin Settings").size(20),
            text("Plugin management coming soon..."),
        ]
        .spacing(16)
        .into()
    }

    fn create_environment_profiles_settings(&self) -> Element<SettingsMessage> {
        let mut content = column![
            text("Environment Profiles").size(20),
            text("Manage sets of environment variables for different sessions.").size(14),
        ].spacing(16);

        // List of profiles
        let mut profile_names: Vec<String> = self.config.env_profiles.profiles.keys().cloned().collect();
        profile_names.sort(); // Sort profiles alphabetically
        let active_profile_name = self.config.env_profiles.active_profile.clone();

        content = content.push(
            row![
                text("Active Profile:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    profile_names.clone(),
                    active_profile_name,
                    SettingsMessage::SelectEnvironmentProfile
                )
            ].spacing(8)
        );

        // Profile list and actions
        let profiles_list = column(
            profile_names.into_iter().map(|name| {
                let is_active = self.config.env_profiles.active_profile.as_ref() == Some(&name);
                let profile_text = if is_active {
                    format!("{} (Active)", name)
                } else {
                    name.clone()
                };
                
                row![
                    text(profile_text).width(iced::Length::Fill),
                    button("Edit").on_press(SettingsMessage::EditEnvironmentProfile(Some(name.clone()))),
                    button("Delete").on_press(SettingsMessage::DeleteEnvironmentProfile(name.clone())),
                ].spacing(8).into()
            }).collect::<Vec<_>>()
        ).spacing(4);

        content = content.push(profiles_list);
        content = content.push(
            button("Add New Profile")
                .on_press(SettingsMessage::EditEnvironmentProfile(None))
        );

        // Environment Profile Editor
        if let Some(editing_profile) = &self.editing_env_profile {
            let profile_name_val = editing_profile.variables.get("name").cloned().unwrap_or_default();
            let mut editor_column = column![
                text(format!("Editing Profile: {}", profile_name_val)).size(18),
                row![
                    text("Profile Name:").width(iced::Length::Fixed(120.0)),
                    text_input(
                        "Profile Name",
                        &profile_name_val
                    )
                    .on_input(SettingsMessage::EnvironmentProfileNameChanged)
                ].spacing(8),
                text("Variables:").size(16),
            ].spacing(10);

            // Sort variables for consistent display, excluding "name"
            let mut sorted_vars: Vec<(&String, &String)> = editing_profile.variables.iter()
                .filter(|(k, _)| k.as_str() != "name")
                .collect();
            sorted_vars.sort_by_key(|(k, _)| *k);

            for (index, (key, value)) in sorted_vars.iter().enumerate() {
                editor_column = editor_column.push(
                    row![
                        text_input("Key", key)
                            .on_input(move |s| SettingsMessage::EnvironmentVariableKeyChanged(index, s))
                            .width(iced::Length::FillPortion(1)),
                        text_input("Value", value)
                            .on_input(move |s| SettingsMessage::EnvironmentVariableValueChanged(index, s))
                            .width(iced::Length::FillPortion(2)),
                        button("Remove").on_press(SettingsMessage::RemoveEnvironmentVariable(index)),
                    ].spacing(8)
                );
            }

            editor_column = editor_column.push(
                button("Add Variable").on_press(SettingsMessage::AddEnvironmentVariable)
            );

            editor_column = editor_column.push(
                row![
                    button("Save Profile").on_press(SettingsMessage::SaveEnvironmentProfile),
                    button("Cancel").on_press(SettingsMessage::CancelEditEnvironmentProfile),
                ].spacing(8)
            );

            if let Some(error) = &self.env_profile_error {
                editor_column = editor_column.push(text(error).color(iced::Color::from_rgb(1.0, 0.0, 0.0)));
            }

            content = content.push(container(editor_column).padding(16).style(container::Style::Box));
        }

        content.into()
    }
}
