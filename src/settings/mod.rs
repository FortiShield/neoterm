use iced::{Element, widget::{column, row, text, button, container, scrollable, pick_list, slider, checkbox, text_input}, Length, Color};
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
use crate::config::AppConfig;

pub mod theme_editor;
pub mod keybinding_editor;
pub mod yaml_theme_ui;
pub mod appearance_settings; // New module

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    General,
    Appearance, // New tab
    Terminal,
    Editor,
    KeyBindings,
    Themes,
    Plugins,
    AI,
    Privacy,
    Performance,
    Collaboration,
    CloudSync,
    Drive,
    Workflows,
    About,
}

impl Default for SettingsTab {
    fn default() -> Self {
        SettingsTab::General
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    TabSelected(SettingsTab),
    KeybindingEditor(keybinding_editor::Message),
    ThemeEditor(theme_editor::Message),
    AppearanceSettings(appearance_settings::AppearanceSettingsMessage), // New message variant
    // Add other settings messages here
}

#[derive(Debug)]
pub struct SettingsView {
    pub config: AppConfig,
    selected_tab: SettingsTab,
    keybinding_editor: keybinding_editor::KeybindingEditor,
    theme_editor: theme_editor::ThemeEditor,
    appearance_settings: appearance_settings::AppearanceSettings, // New field
    yaml_theme_ui: yaml_theme_ui::YamlThemeUi, // Assuming this is for advanced YAML editing
    config_manager: Arc<ConfigManager>,
}

impl SettingsView {
    pub fn new(config: AppConfig) -> Self {
        let config_manager = Arc::new(ConfigManager::new_dummy()); // Dummy for now, will be replaced by actual
        Self {
            keybinding_editor: keybinding_editor::KeybindingEditor::new(config.preferences.keybindings_file.clone()),
            theme_editor: theme_editor::ThemeEditor::new(config_manager.clone()),
            appearance_settings: appearance_settings::AppearanceSettings::new(config.clone()), // Initialize new settings view
            yaml_theme_ui: yaml_theme_ui::YamlThemeUi::new(config_manager.clone()),
            config_manager, // Store the dummy config manager
            config,
            selected_tab: SettingsTab::default(),
        }
    }

    pub async fn init(&mut self) {
        // Re-initialize components that need async setup
        self.theme_editor = theme_editor::ThemeEditor::new(self.config_manager.clone());
        self.theme_editor.init().await.unwrap();
        self.yaml_theme_ui.init().await.unwrap();
        // No async init needed for AppearanceSettings currently
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::TabSelected(tab) => {
                self.selected_tab = tab;
            }
            SettingsMessage::KeybindingEditor(msg) => {
                self.keybinding_editor.update(msg);
                // Update config from editor if necessary
                // self.config.preferences.keybindings_file = self.keybinding_editor.get_keybindings_file();
            }
            SettingsMessage::ThemeEditor(msg) => {
                self.theme_editor.update(msg);
                // The theme editor updates the config_manager's theme directly
            }
            SettingsMessage::AppearanceSettings(msg) => {
                self.appearance_settings.update(msg);
                self.config.preferences = self.appearance_settings.config.preferences.clone(); // Sync preferences back
            }
        }
        // In a real application, you'd save preferences here or on app exit
        // For now, we'll just update the in-memory config
        // self.config.preferences.save().await.unwrap(); // This would be async
    }

    pub fn view(&self) -> Element<SettingsMessage> {
        let sidebar = column![
            self.nav_button(SettingsTab::General, "General"),
            self.nav_button(SettingsTab::Appearance, "Appearance"), // New nav button
            self.nav_button(SettingsTab::Terminal, "Terminal"),
            self.nav_button(SettingsTab::Editor, "Editor"),
            self.nav_button(SettingsTab::KeyBindings, "Keyboard shortcuts"),
            self.nav_button(SettingsTab::Themes, "Themes"),
            self.nav_button(SettingsTab::Plugins, "Plugins"),
            self.nav_button(SettingsTab::AI, "AI"),
            self.nav_button(SettingsTab::Privacy, "Privacy"),
            self.nav_button(SettingsTab::Performance, "Performance"),
            self.nav_button(SettingsTab::Collaboration, "Collaboration"),
            self.nav_button(SettingsTab::CloudSync, "Cloud Sync"),
            self.nav_button(SettingsTab::Drive, "Drive"),
            self.nav_button(SettingsTab::Workflows, "Workflows"),
            self.nav_button(SettingsTab::About, "About"),
        ]
        .spacing(10)
        .width(Length::Units(200));

        let content: Element<SettingsMessage> = match self.selected_tab {
            SettingsTab::General => column![text("General Settings")].into(),
            SettingsTab::Appearance => self.appearance_settings.view().map(SettingsMessage::AppearanceSettings), // Render new view
            SettingsTab::Terminal => column![text("Terminal Settings")].into(),
            SettingsTab::Editor => column![text("Editor Settings")].into(),
            SettingsTab::KeyBindings => self.keybinding_editor.view().map(SettingsMessage::KeybindingEditor),
            SettingsTab::Themes => self.theme_editor.view().map(SettingsMessage::ThemeEditor),
            SettingsTab::Plugins => column![text("Plugin Settings")].into(),
            SettingsTab::AI => column![text("AI Settings")].into(),
            SettingsTab::Privacy => column![text("Privacy Settings")].into(),
            SettingsTab::Performance => column![text("Performance Settings")].into(),
            SettingsTab::Collaboration => column![text("Collaboration Settings")].into(),
            SettingsTab::CloudSync => column![text("Cloud Sync Settings")].into(),
            SettingsTab::Drive => column![text("Drive Integration Settings")].into(),
            SettingsTab::Workflows => column![text("Workflow Settings")].into(),
            SettingsTab::About => column![text("About NeoTerm")].into(),
        };

        row![
            sidebar,
            iced::widget::vertical_rule(1),
            scrollable(content).width(Length::Fill),
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    fn nav_button(&self, tab: SettingsTab, label: &str) -> Element<SettingsMessage> {
        let is_selected = self.selected_tab == tab;
        button(text(label).size(16).color(if is_selected { Color::BLACK } else { Color::WHITE }))
            .on_press(SettingsMessage::TabSelected(tab))
            .style(if is_selected {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Text
            })
            .width(Length::Fill)
            .into()
    }
}

pub fn init() {
    log::info!("settings module loaded");
}
