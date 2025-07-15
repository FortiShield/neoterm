use iced::{Element, widget::{column, row, text, button, container, scrollable, pick_list, slider, checkbox, text_input}};
use crate::{Message, config::*};
use std::collections::HashMap; // Added for HashMap

pub mod theme_editor;
pub mod keybinding_editor;

use theme_editor::ThemeEditor;
use keybinding_editor::KeyBindingEditor;

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};
use crate::config::{AppConfig, EnvironmentProfile};

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    General,
    Keybindings,
    Theme,
    EnvironmentProfiles, // New tab for environment profiles
    About,
}

impl SettingsTab {
    pub fn all() -> Vec<SettingsTab> {
        vec![
            SettingsTab::General,
            SettingsTab::Keybindings,
            SettingsTab::Theme,
            SettingsTab::EnvironmentProfiles,
            SettingsTab::About,
        ]
    }

    pub fn title(&self) -> &str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Keybindings => "Keybindings",
            SettingsTab::Theme => "Theme",
            SettingsTab::EnvironmentProfiles => "Environment Profiles",
            SettingsTab::About => "About",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    TabSelected(SettingsTab),
    // Environment Profile Messages
    SelectEnvironmentProfile(String),
    EditEnvironmentProfile(Option<String>), // None for new, Some(name) for existing
    SaveEnvironmentProfile,
    DeleteEnvironmentProfile(String),
    EnvironmentProfileNameChanged(String),
    EnvironmentVariableKeyChanged(usize, String),
    EnvironmentVariableValueChanged(usize, String),
    AddEnvironmentVariable,
    RemoveEnvironmentVariable(usize),
    CancelEditEnvironmentProfile,
    TabChanged(iced::settings::SettingsTab),
    ConfigChanged(ConfigChange),
    ThemeChanged(String),
    CustomThemeCreated(String),
    KeyBindingChanged(String, KeyBinding),
    ResetToDefaults,
    ImportConfig,
    ExportConfig,
    Save,
    Cancel,
    ThemeEditor(theme_editor::Message),
    KeyBindingEditor(keybinding_editor::Message),

    // Environment Profile Management
    //SelectEnvironmentProfile(String), // For setting active profile
    //EditEnvironmentProfile(Option<String>), // None for new, Some(name) for existing
    //SaveEnvironmentProfile,
    //CancelEditEnvironmentProfile,
    //DeleteEnvironmentProfile(String),
    //EnvironmentProfileNameChanged(String),
    //EnvironmentVariableKeyChanged(usize, String), // index, new_key
    //EnvironmentVariableValueChanged(usize, String), // index, new_value
    //AddEnvironmentVariable,
    //RemoveEnvironmentVariable(usize), // index
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

    // Environment Profiles (handled directly by SettingsView update logic)
    // No direct ConfigChange for these, as they are managed through specific SettingsMessages
}

pub struct SettingsView {
    config: AppConfig,
    selected_tab: SettingsTab,
    env_profile_list_state: ListState,
    
    // State for editing environment profiles
    editing_env_profile: Option<EnvironmentProfile>,
    editing_env_profile_original_name: Option<String>, // To track renames
    env_profile_error: Option<String>,
    pub active_tab: iced::settings::SettingsTab,
    pub theme_editor: ThemeEditor,
    pub keybinding_editor: KeyBindingEditor,
    pub unsaved_changes: bool,
    
    // Environment Profile Editor State
    //pub editing_env_profile: Option<EnvironmentProfile>,
    //pub editing_env_profile_original_name: Option<String>, // To track if it's a new profile or an edit
    //pub env_profile_error: Option<String>, // For validation errors
}

impl SettingsView {
    pub fn new(config: AppConfig) -> Self {
        let mut env_profile_list_state = ListState::default();
        if !config.env_profiles.profiles.is_empty() {
            env_profile_list_state.select(Some(0));
        }
        Self {
            config,
            selected_tab: SettingsTab::General,
            env_profile_list_state,
            editing_env_profile: None,
            editing_env_profile_original_name: None,
            env_profile_error: None,
            active_tab: iced::settings::SettingsTab::General,
            theme_editor: ThemeEditor::new(config.theme.clone()),
            keybinding_editor: KeyBindingEditor::new(config.keybindings.clone()),
            unsaved_changes: false,
            //editing_env_profile: None,
            //editing_env_profile_original_name: None,
            //env_profile_error: None,
        }
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::TabSelected(tab) => {
                self.selected_tab = tab;
                // Reset editing state when switching tabs
                self.editing_env_profile = None;
                self.editing_env_profile_original_name = None;
                self.env_profile_error = None;
            }
            SettingsMessage::SelectEnvironmentProfile(name) => {
                self.config.env_profiles.active_profile = Some(name);
                self.config.save().unwrap_or_else(|e| eprintln!("Failed to save config: {}", e));
            }
            SettingsMessage::EditEnvironmentProfile(name_opt) => {
                self.env_profile_error = None;
                if let Some(name) = name_opt {
                    if let Some(profile) = self.config.env_profiles.profiles.get(&name) {
                        self.editing_env_profile = Some(profile.clone());
                        self.editing_env_profile_original_name = Some(name.clone());
                    }
                } else {
                    // New profile
                    self.editing_env_profile = Some(EnvironmentProfile {
                        name: "New Profile".to_string(),
                        variables: HashMap::new(),
                    });
                    self.editing_env_profile_original_name = None;
                }
            }
            SettingsMessage::SaveEnvironmentProfile => {
                if let Some(mut profile_to_save) = self.editing_env_profile.take() {
                    self.env_profile_error = None;
                    if profile_to_save.name.trim().is_empty() {
                        self.env_profile_error = Some("Profile name cannot be empty.".to_string());
                        self.editing_env_profile = Some(profile_to_save); // Put it back for editing
                        return;
                    }

                    // Check for duplicate name if it's a new profile or a rename
                    let is_new_profile = self.editing_env_profile_original_name.is_none();
                    let is_renamed = self.editing_env_profile_original_name.as_ref() != Some(&profile_to_save.name);

                    if (is_new_profile || is_renamed) && self.config.env_profiles.profiles.contains_key(&profile_to_save.name) {
                        self.env_profile_error = Some(format!("Profile with name '{}' already exists.", profile_to_save.name));
                        self.editing_env_profile = Some(profile_to_save);
                        return;
                    }

                    // If it was a rename, remove the old entry
                    if let Some(original_name) = self.editing_env_profile_original_name.take() {
                        if original_name != profile_to_save.name {
                            self.config.env_profiles.profiles.remove(&original_name);
                            // If the renamed profile was active, update active_profile
                            if self.config.env_profiles.active_profile == Some(original_name) {
                                self.config.env_profiles.active_profile = Some(profile_to_save.name.clone());
                            }
                        }
                    }
                    
                    self.config.env_profiles.profiles.insert(profile_to_save.name.clone(), profile_to_save);
                    self.config.save().unwrap_or_else(|e| eprintln!("Failed to save config: {}", e));
                }
            }
            SettingsMessage::DeleteEnvironmentProfile(name) => {
                self.config.env_profiles.profiles.remove(&name);
                if self.config.env_profiles.active_profile == Some(name) {
                    self.config.env_profiles.active_profile = None; // Clear active if deleted
                }
                self.config.save().unwrap_or_else(|e| eprintln!("Failed to save config: {}", e));
                // Reset selection if the selected profile was deleted
                if let Some(selected) = self.env_profile_list_state.selected() {
                    if selected >= self.config.env_profiles.profiles.len() && !self.config.env_profiles.profiles.is_empty() {
                        self.env_profile_list_state.select(Some(self.config.env_profiles.profiles.len() - 1));
                    } else if self.config.env_profiles.profiles.is_empty() {
                        self.env_profile_list_state.select(None);
                    }
                }
            }
            SettingsMessage::EnvironmentProfileNameChanged(name) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    profile.name = name;
                }
            }
            SettingsMessage::EnvironmentVariableKeyChanged(index, key) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<_> = profile.variables.drain().collect();
                    if let Some((old_key, value)) = vars.get_mut(index) {
                        *old_key = key;
                    }
                    profile.variables = vars.into_iter().collect();
                }
            }
            SettingsMessage::EnvironmentVariableValueChanged(index, value) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<_> = profile.variables.drain().collect();
                    if let Some((key, old_value)) = vars.get_mut(index) {
                        *old_value = value;
                    }
                    profile.variables = vars.into_iter().collect();
                }
            }
            SettingsMessage::AddEnvironmentVariable => {
                if let Some(profile) = &mut self.editing_env_profile {
                    profile.variables.insert("NEW_VAR".to_string(), "".to_string());
                }
            }
            SettingsMessage::RemoveEnvironmentVariable(index) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<_> = profile.variables.drain().collect();
                    if index < vars.len() {
                        vars.remove(index);
                    }
                    profile.variables = vars.into_iter().collect();
                }
            }
            SettingsMessage::CancelEditEnvironmentProfile => {
                self.editing_env_profile = None;
                self.editing_env_profile_original_name = None;
                self.env_profile_error = None;
            }
            SettingsMessage::TabChanged(tab) => {
                self.active_tab = tab;
                //self.editing_env_profile = None; // Close editor when changing tabs
                //self.env_profile_error = None;
                //None
            }
            SettingsMessage::ConfigChanged(change) => {
                self.apply_config_change(change);
                self.unsaved_changes = true;
                //None
            }
            SettingsMessage::ThemeChanged(theme_name) => {
                if let Some(theme) = ThemeConfig::builtin_themes()
                    .into_iter()
                    .find(|t| t.name == theme_name)
                {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
                //None
            }
            SettingsMessage::Save => {
                if let Err(e) = self.config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
                self.unsaved_changes = false;
                //Some(self.config.clone())
            }
            SettingsMessage::Cancel => {
                // Reload config from disk
                if let Ok(config) = AppConfig::load() {
                    self.config = config.clone();
                    self.unsaved_changes = false;
                    //self.editing_env_profile = None; // Reset editor state
                    //self.env_profile_error = None;
                    //Some(config)
                } else {
                    //None
                }
            }
            SettingsMessage::ResetToDefaults => {
                self.config = AppConfig::default();
                self.unsaved_changes = true;
                //self.editing_env_profile = None; // Reset editor state
                //self.env_profile_error = None;
                //None
            }
            SettingsMessage::ThemeEditor(msg) => {
                if let Some(theme) = self.theme_editor.update(msg) {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
                //None
            }
            SettingsMessage::KeyBindingEditor(msg) => {
                if let Some(keybindings) = self.keybinding_editor.update(msg) {
                    self.config.keybindings = keybindings;
                    self.unsaved_changes = true;
                }
                //None
            }
            // Environment Profile Messages
            /*SettingsMessage::SelectEnvironmentProfile(name) => {
                self.config.env_profiles.active_profile = Some(name);
                self.unsaved_changes = true;
                None
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
                    self.editing_env_profile = Some(EnvironmentProfile {
                        name: "New Profile".to_string(),
                        variables: HashMap::new(),
                    });
                    self.editing_env_profile_original_name = None;
                }
                None
            }
            SettingsMessage::SaveEnvironmentProfile => {
                if let Some(mut profile_to_save) = self.editing_env_profile.take() {
                    // Validate name
                    if profile_to_save.name.trim().is_empty() {
                        self.env_profile_error = Some("Profile name cannot be empty.".to_string());
                        self.editing_env_profile = Some(profile_to_save); // Restore for editing
                        return None;
                    }
                    
                    // Check for duplicate name if it's a new profile or name changed
                    if let Some(original_name) = &self.editing_env_profile_original_name {
                        if original_name != &profile_to_save.name && self.config.env_profiles.profiles.contains_key(&profile_to_save.name) {
                            self.env_profile_error = Some(format!("Profile with name '{}' already exists.", profile_to_save.name));
                            self.editing_env_profile = Some(profile_to_save);
                            return None;
                        }
                        // Remove old entry if name changed
                        if original_name != &profile_to_save.name {
                            self.config.env_profiles.profiles.remove(original_name);
                            // If the old profile was active, switch to the new one
                            if self.config.env_profiles.active_profile.as_ref() == Some(original_name) {
                                self.config.env_profiles.active_profile = Some(profile_to_save.name.clone());
                            }
                        }
                    } else { // It's a new profile
                        if self.config.env_profiles.profiles.contains_key(&profile_to_save.name) {
                            self.env_profile_error = Some(format!("Profile with name '{}' already exists.", profile_to_save.name));
                            self.editing_env_profile = Some(profile_to_save);
                            return None;
                        }
                    }

                    // Clean up empty variables
                    profile_to_save.variables.retain(|k, v| !k.trim().is_empty() || !v.trim().is_empty());

                    self.config.env_profiles.profiles.insert(profile_to_save.name.clone(), profile_to_save);
                    self.editing_env_profile_original_name = None;
                    self.unsaved_changes = true;
                    self.env_profile_error = None;
                }
                None
            }
            SettingsMessage::CancelEditEnvironmentProfile => {
                self.editing_env_profile = None;
                self.editing_env_profile_original_name = None;
                self.env_profile_error = None;
                None
            }
            SettingsMessage::DeleteEnvironmentProfile(name) => {
                if self.config.env_profiles.profiles.remove(&name).is_some() {
                    if self.config.env_profiles.active_profile.as_ref() == Some(&name) {
                        self.config.env_profiles.active_profile = self.config.env_profiles.profiles.keys().next().cloned();
                    }
                    self.unsaved_changes = true;
                }
                None
            }
            SettingsMessage::EnvironmentProfileNameChanged(new_name) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    profile.name = new_name;
                }
                None
            }
            SettingsMessage::EnvironmentVariableKeyChanged(index, new_key) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<(String, String)> = profile.variables.drain().collect();
                    if let Some((key, value)) = vars.get_mut(index) {
                        *key = new_key;
                    }
                    profile.variables = vars.into_iter().collect();
                }
                None
            }
            SettingsMessage::EnvironmentVariableValueChanged(index, new_value) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<(String, String)> = profile.variables.drain().collect();
                    if let Some((key, value)) = vars.get_mut(index) {
                        *value = new_value;
                    }
                    profile.variables = vars.into_iter().collect();
                }
                None
            }
            SettingsMessage::AddEnvironmentVariable => {
                if let Some(profile) = &mut self.editing_env_profile {
                    profile.variables.insert("".to_string(), "".to_string());
                }
                None
            }
            SettingsMessage::RemoveEnvironmentVariable(index) => {
                if let Some(profile) = &mut self.editing_env_profile {
                    let mut vars: Vec<(String, String)> = profile.variables.drain().collect();
                    if index < vars.len() {
                        vars.remove(index);
                    }
                    profile.variables = vars.into_iter().collect();
                }
                None
            }*/
        }
    }

    fn apply_config_change(&mut self, change: ConfigChange) {
        match change {
            ConfigChange::StartupBehavior(behavior) => {
                self.config.preferences.general.startup_behavior = behavior;
            }
            ConfigChange::DefaultShell(shell) => {
                self.config.preferences.general.default_shell = Some(shell);
            }
            ConfigChange::AutoUpdate(enabled) => {
                self.config.preferences.general.auto_update = enabled;
            }
            ConfigChange::ScrollbackLines(lines) => {
                self.config.preferences.terminal.scrollback_lines = lines;
            }
            ConfigChange::ScrollSensitivity(sensitivity) => {
                self.config.preferences.terminal.scroll_sensitivity = sensitivity;
            }
            ConfigChange::CopyOnSelect(enabled) => {
                self.config.preferences.terminal.copy_on_select = enabled;
            }
            ConfigChange::VimMode(enabled) => {
                self.config.preferences.editor.vim_mode = enabled;
            }
            ConfigChange::AutoSuggestions(enabled) => {
                self.config.preferences.editor.auto_suggestions = enabled;
            }
            ConfigChange::Transparency(value) => {
                self.config.preferences.ui.transparency = value;
            }
            ConfigChange::GpuAcceleration(enabled) => {
                self.config.preferences.performance.gpu_acceleration = enabled;
            }
            ConfigChange::ShowTabBar(visibility) => {
                self.config.preferences.ui.show_tab_bar = visibility;
            }
            ConfigChange::ShowTitleBar(enabled) => {
                self.config.preferences.ui.show_title_bar = enabled;
            }
            ConfigChange::CompactMode(enabled) => {
                self.config.preferences.ui.compact_mode = enabled;
            }
            ConfigChange::BlurBackground(enabled) => {
                self.config.preferences.ui.blur_background = enabled;
            }
            ConfigChange::AnimationsEnabled(enabled) => {
                self.config.preferences.ui.animations_enabled = enabled;
            }
            ConfigChange::ZoomLevel(level) => {
                self.config.preferences.ui.zoom_level = level;
            }
            ConfigChange::Vsync(enabled) => {
                self.config.preferences.performance.vsync = enabled;
            }
            ConfigChange::MaxFps(fps) => {
                self.config.preferences.performance.max_fps = fps;
            }
            ConfigChange::MemoryLimit(limit) => {
                self.config.preferences.performance.memory_limit = limit;
            }
            ConfigChange::HistoryEnabled(enabled) => {
                self.config.preferences.privacy.history_enabled = enabled;
            }
            ConfigChange::HistoryLimit(limit) => {
                self.config.preferences.privacy.history_limit = limit;
            }
            ConfigChange::ClearHistoryOnExit(enabled) => {
                self.config.preferences.privacy.clear_history_on_exit = enabled;
            }
            ConfigChange::IncognitoMode(enabled) => {
                self.config.preferences.privacy.incognito_mode = enabled;
            }
            ConfigChange::LogLevel(level) => {
                self.config.preferences.privacy.log_level = level;
            }
            ConfigChange::PasteOnRightClick(enabled) => {
                self.config.preferences.terminal.paste_on_right_click = enabled;
            }
            ConfigChange::ConfirmBeforeClosing(enabled) => {
                self.config.preferences.terminal.confirm_before_closing = enabled;
            }
            ConfigChange::BellBehavior(behavior) => {
                self.config.preferences.terminal.bell_behavior = behavior;
            }
            ConfigChange::CursorStyle(style) => {
                self.config.preferences.terminal.cursor_style = style;
            }
            ConfigChange::CursorBlink(enabled) => {
                self.config.preferences.terminal.cursor_blink = enabled;
            }
            ConfigChange::IndentSize(size) => {
                self.config.preferences.editor.indent_size = size;
            }
            ConfigChange::TabWidth(width) => {
                self.config.preferences.editor.tab_width = width;
            }
            ConfigChange::InsertSpaces(enabled) => {
                self.config.preferences.editor.insert_spaces = enabled;
            }
            ConfigChange::SyntaxHighlighting(enabled) => {
                self.config.preferences.editor.syntax_highlighting = enabled;
            }
            ConfigChange::AutoCompletion(enabled) => {
                self.config.preferences.editor.auto_completion = enabled;
            }
            _ => {} // Catch-all for unhandled ConfigChange variants
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Content
            ])
            .split(area);

        self.render_tabs(f, chunks[0]);

        let content_area = chunks[1];
        match self.selected_tab {
            SettingsTab::General => self.create_general_settings(f, content_area),
            SettingsTab::Keybindings => self.create_keybindings_settings(f, content_area),
            SettingsTab::Theme => self.create_theme_settings(f, content_area),
            SettingsTab::EnvironmentProfiles => self.create_environment_profiles_settings(f, content_area),
            SettingsTab::About => self.create_about_settings(f, content_area),
        }
    }

    fn render_tabs<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let titles: Vec<Line> = SettingsTab::all()
            .iter()
            .map(|tab| {
                let is_selected = *tab == self.selected_tab;
                let style = if is_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(tab.title(), style))
            })
            .collect();

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Settings"))
            .highlight_style(Style::default().fg(Color::Yellow))
            .select(self.selected_tab as usize);

        f.render_widget(tabs, area);
    }

    fn create_general_settings<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("General Settings");
        let paragraph = Paragraph::new("General settings content goes here.").block(block).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn create_keybindings_settings<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Keybindings");
        let paragraph = Paragraph::new("Keybindings settings content goes here.").block(block).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn create_theme_settings<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Theme");
        let paragraph = Paragraph::new("Theme settings content goes here.").block(block).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn create_about_settings<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("About");
        let paragraph = Paragraph::new("NeoTerm v0.1.0\n\nDeveloped by Vercel AI").block(block).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn create_environment_profiles_settings<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let main_block = Block::default().borders(Borders::ALL).title("Environment Profiles");
        let inner_area = main_block.inner(area);
        f.render_widget(main_block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Profile list
                Constraint::Percentage(70), // Editor/Details
            ])
            .split(inner_area);

        // Left pane: Profile List
        let profiles: Vec<ListItem> = self.config.env_profiles.profiles.keys()
            .map(|name| {
                let is_active = self.config.env_profiles.active_profile.as_ref() == Some(name);
                let style = if is_active {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Span::styled(name, style))
            })
            .collect();

        let list = List::new(profiles)
            .block(Block::default().borders(Borders::ALL).title("Profiles"))
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

        f.render_stateful_widget(list, chunks[0], &mut self.env_profile_list_state);

        // Right pane: Editor or Actions
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Active profile / Add button
                Constraint::Min(0),    // Editor or details
            ])
            .split(chunks[1]);

        // Top right: Active profile and Add button
        let active_profile_text = format!("Active: {}", self.config.env_profiles.active_profile.as_deref().unwrap_or("None"));
        let active_profile_paragraph = Paragraph::new(active_profile_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(active_profile_paragraph, right_chunks[0]);

        let editor_area = right_chunks[1];

        if let Some(profile) = &mut self.editing_env_profile {
            // Render editor for the selected profile
            let editor_block = Block::default().borders(Borders::ALL).title(format!("Editing: {}", profile.name));
            let editor_inner_area = editor_block.inner(editor_area);
            f.render_widget(editor_block, editor_area);

            let editor_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Name input
                    Constraint::Min(0),    // Variables list
                    Constraint::Length(1), // Error message
                    Constraint::Length(3), // Action buttons
                ])
                .split(editor_inner_area);

            // Profile Name Input
            let name_input_block = Block::default().borders(Borders::ALL).title("Profile Name");
            let name_input_paragraph = Paragraph::new(profile.name.as_str())
                .block(name_input_block);
            f.render_widget(name_input_paragraph, editor_chunks[0]);

            // Environment Variables List
            let vars_items: Vec<ListItem> = profile.variables.iter()
                .enumerate()
                .map(|(i, (key, value))| {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::DarkGray)),
                        Span::styled(key, Style::default().fg(Color::LightCyan)),
                        Span::raw("="),
                        Span::styled(value, Style::default().fg(Color::LightGreen)),
                    ]))
                })
                .collect();
            let vars_list = List::new(vars_items)
                .block(Block::default().borders(Borders::ALL).title("Variables"));
            f.render_widget(vars_list, editor_chunks[1]);

            // Error message
            if let Some(error) = &self.env_profile_error {
                let error_paragraph = Paragraph::new(Span::styled(error, Style::default().fg(Color::Red)));
                f.render_widget(error_paragraph, editor_chunks[2]);
            }

            // Action Buttons (Save, Cancel, Add Var, Remove Var)
            let button_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ])
                .split(editor_chunks[3]);

            // These buttons are placeholders for now, as direct button interaction
            // in Ratatui requires more complex event handling (e.g., mouse clicks or specific keybinds).
            // For a real implementation, you'd map key presses to these actions.
            let save_button = Paragraph::new(" [S]ave ").block(Block::default().borders(Borders::ALL));
            let cancel_button = Paragraph::new(" [C]ancel ").block(Block::default().borders(Borders::ALL));
            let add_var_button = Paragraph::new(" [A]dd Var ").block(Block::default().borders(Borders::ALL));
            let remove_var_button = Paragraph::new(" [R]emove Var ").block(Block::default().borders(Borders::ALL));

            f.render_widget(save_button, button_chunks[0]);
            f.render_widget(cancel_button, button_chunks[1]);
            f.render_widget(add_var_button, button_chunks[2]);
            f.render_widget(remove_var_button, button_chunks[3]);

        } else {
            // No profile being edited, show actions for selected profile
            let selected_profile_name = self.env_profile_list_state.selected()
                .and_then(|i| self.config.env_profiles.profiles.keys().nth(i).cloned());

            let action_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Add New Profile
                    Constraint::Length(3), // Set Active
                    Constraint::Length(3), // Edit
                    Constraint::Length(3), // Delete
                ])
                .split(editor_area);

            let add_new_button = Paragraph::new(" [N]ew Profile ").block(Block::default().borders(Borders::ALL));
            f.render_widget(add_new_button, action_chunks[0]);

            if let Some(name) = selected_profile_name {
                let set_active_button = Paragraph::new(format!(" [Set] Active: {}", name)).block(Block::default().borders(Borders::ALL));
                let edit_button = Paragraph::new(format!(" [E]dit: {}", name)).block(Block::default().borders(Borders::ALL));
                let delete_button = Paragraph::new(format!(" [D]elete: {}", name)).block(Block::default().borders(Borders::ALL));

                f.render_widget(set_active_button, action_chunks[1]);
                f.render_widget(edit_button, action_chunks[2]);
                f.render_widget(delete_button, action_chunks[3]);
            }
        }
    }
}
