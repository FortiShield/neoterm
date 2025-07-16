use iced::{Element, widget::{column, row, text, button, text_input, pick_list, container, scrollable}, Length};
use iced::Color;
use std::collections::HashMap;
use crate::config::{ThemeConfig, TerminalColors, ConfigManager, theme::Theme, yaml_theme::YamlTheme};
use crate::settings::Settings;
use anyhow::{Result, anyhow};
use tokio::fs;
use std::sync::Arc;
use log::info;

#[derive(Debug, Clone)]
pub enum ThemeEditorMessage {
    SelectTheme(String),
    EditColor(String, String), // (Color Name, New Hex Value)
    SaveTheme,
    LoadThemes,
    // For adding custom colors
    NewCustomColorNameChanged(String),
    NewCustomColorValueChanged(String),
    AddCustomColor,
    DeleteCustomColor(String),
}

#[derive(Debug, Clone)]
pub struct ThemeEditor {
    config_manager: Arc<ConfigManager>,
    // No direct state needed here, as themes are managed by ConfigManager/YamlThemeManager
    available_themes: Vec<String>,
    selected_theme_name: Option<String>,
    current_theme_config: ThemeConfig, // The theme currently being edited
    new_custom_color_name: String,
    new_custom_color_value: String,
}

impl ThemeEditor {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        let mut editor = Self {
            config_manager,
            available_themes: Vec::new(),
            selected_theme_name: None,
            current_theme_config: ThemeConfig::default(),
            new_custom_color_name: String::new(),
            new_custom_color_value: String::new(),
        };
        editor.init().await.unwrap();
        editor
    }

    pub async fn init(&self) -> Result<()> {
        info!("Theme editor initialized.");
        self.available_themes = self.get_all_theme_names().await?;
        self.selected_theme_name = Some(self.get_active_theme_name().await);
        self.current_theme_config = self.get_theme_by_name(&self.selected_theme_name.clone().unwrap()).await?.into();
        Ok(())
    }

    pub async fn get_all_theme_names(&self) -> Vec<String> {
        self.config_manager.get_theme_manager().await.list_themes().await
    }

    pub async fn get_theme_by_name(&self, name: &str) -> Result<Theme> {
        self.config_manager.get_theme_manager().await.get_theme(name).await
    }

    pub async fn save_theme(&self, theme: Theme) -> Result<()> {
        self.config_manager.get_theme_manager().await.save_theme(theme).await
    }

    pub async fn delete_theme(&self, name: &str) -> Result<()> {
        self.config_manager.get_theme_manager().await.delete_theme(name).await
    }

    pub async fn set_active_theme(&self, name: &str) -> Result<()> {
        let mut prefs = self.config_manager.get_preferences().await;
        if prefs.theme_name != name {
            // Verify theme exists before setting
            self.config_manager.get_theme_manager().await.get_theme(name).await?;
            prefs.theme_name = name.to_string();
            self.config_manager.update_preferences(prefs).await?;
            info!("Active theme set to: {}", name);
        }
        Ok(())
    }

    pub async fn get_active_theme_name(&self) -> String {
        self.config_manager.get_preferences().await.theme_name
    }

    pub fn update(&mut self, message: ThemeEditorMessage) -> Command<ThemeEditorMessage> {
        match message {
            ThemeEditorMessage::LoadThemes => {
                // Update available themes
                self.available_themes = self.get_all_theme_names().await.unwrap();
                Command::none()
            }
            ThemeEditorMessage::SelectTheme(name) => {
                if let Some(theme_config) = self.get_theme_by_name(&name).await.ok().map(|t| t.into()) {
                    self.selected_theme_name = Some(name);
                    self.current_theme_config = theme_config;
                    // Update main settings struct as well
                    self.config_manager.get_preferences().await.theme = self.current_theme_config.clone();
                }
                Command::none()
            }
            ThemeEditorMessage::EditColor(color_name, new_hex) => {
                // Update terminal colors
                match color_name.as_str() {
                    "background" => self.current_theme_config.terminal_colors.background = new_hex,
                    "foreground" => self.current_theme_config.terminal_colors.foreground = new_hex,
                    "cursor" => self.current_theme_config.terminal_colors.cursor = new_hex,
                    "selection" => self.current_theme_config.terminal_colors.selection = new_hex,
                    "black" => self.current_theme_config.terminal_colors.black = new_hex,
                    "red" => self.current_theme_config.terminal_colors.red = new_hex,
                    "green" => self.current_theme_config.terminal_colors.green = new_hex,
                    "yellow" => self.current_theme_config.terminal_colors.yellow = new_hex,
                    "blue" => self.current_theme_config.terminal_colors.blue = new_hex,
                    "magenta" => self.current_theme_config.terminal_colors.magenta = new_hex,
                    "cyan" => self.current_theme_config.terminal_colors.cyan = new_hex,
                    "white" => self.current_theme_config.terminal_colors.white = new_hex,
                    "bright_black" => self.current_theme_config.terminal_colors.bright_black = new_hex,
                    "bright_red" => self.current_theme_config.terminal_colors.bright_red = new_hex,
                    "bright_green" => self.current_theme_config.terminal_colors.bright_green = new_hex,
                    "bright_yellow" => self.current_theme_config.terminal_colors.bright_yellow = new_hex,
                    "bright_blue" => self.current_theme_config.terminal_colors.bright_blue = new_hex,
                    "bright_magenta" => self.current_theme_config.terminal_colors.bright_magenta = new_hex,
                    "bright_cyan" => self.current_theme_config.terminal_colors.bright_cyan = new_hex,
                    "bright_white" => self.current_theme_config.terminal_colors.bright_white = new_hex,
                    _ => {
                        // Update general colors or custom colors
                        self.current_theme_config.colors.insert(color_name, new_hex);
                    }
                }
                // Update main settings struct as well
                self.config_manager.get_preferences().await.theme = self.current_theme_config.clone();
                Command::none()
            }
            ThemeEditorMessage::SaveTheme => {
                // Convert current_theme_config to Theme and save it
                let theme: Theme = self.current_theme_config.clone().into();
                self.save_theme(theme).await.unwrap();
                Command::none()
            }
            ThemeEditorMessage::NewCustomColorNameChanged(name) => {
                self.new_custom_color_name = name;
                Command::none()
            }
            ThemeEditorMessage::NewCustomColorValueChanged(value) => {
                self.new_custom_color_value = value;
                Command::none()
            }
            ThemeEditorMessage::AddCustomColor => {
                if !self.new_custom_color_name.is_empty() && !self.new_custom_color_value.is_empty() {
                    self.current_theme_config.colors.insert(
                        self.new_custom_color_name.clone(),
                        self.new_custom_color_value.clone(),
                    );
                    self.new_custom_color_name.clear();
                    self.new_custom_color_value.clear();
                    // Update main settings struct as well
                    self.config_manager.get_preferences().await.theme = self.current_theme_config.clone();
                }
                Command::none()
            }
            ThemeEditorMessage::DeleteCustomColor(color_name) => {
                self.current_theme_config.colors.remove(&color_name);
                // Update main settings struct as well
                self.config_manager.get_preferences().await.theme = self.current_theme_config.clone();
                Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<ThemeEditorMessage> {
        let theme_selector = row![
            text("Select Theme:"),
            pick_list(
                self.available_themes.clone(),
                self.selected_theme_name.clone(),
                ThemeEditorMessage::SelectTheme,
            )
        ].spacing(10).align_items(iced::Alignment::Center);

        let mut color_rows = column![].spacing(5);

        // Terminal Colors
        color_rows = color_rows.push(text("Terminal Colors").size(20).width(Length::Fill));
        let terminal_colors_map = self.current_theme_config.terminal_colors.to_map();
        for (name, hex) in terminal_colors_map.iter() {
            color_rows = color_rows.push(self.color_input_row(name.clone(), hex.clone()));
        }

        // General Colors
        color_rows = color_rows.push(text("General Colors").size(20).width(Length::Fill));
        for (name, hex) in self.current_theme_config.colors.iter() {
            color_rows = color_rows.push(self.color_input_row(name.clone(), hex.clone()));
        }

        // Add Custom Color section
        let add_custom_color_section = column![
            text("Add Custom Color").size(20),
            row![
                text_input("Color Name", &self.new_custom_color_name)
                    .on_input(ThemeEditorMessage::NewCustomColorNameChanged)
                    .width(Length::FillPortion(1)),
                text_input("Hex Value (#RRGGBB)", &self.new_custom_color_value)
                    .on_input(ThemeEditorMessage::NewCustomColorValueChanged)
                    .width(Length::FillPortion(1)),
                button(text("Add")).on_press(ThemeEditorMessage::AddCustomColor),
            ].spacing(10)
        ].spacing(10).padding(10).into();

        column![
            theme_selector,
            iced::widget::horizontal_rule(1),
            scrollable(color_rows).height(Length::FillPortion(1)),
            add_custom_color_section,
            button(text("Save Theme")).on_press(ThemeEditorMessage::SaveTheme),
        ].spacing(10).padding(20).into()
    }

    fn color_input_row(&self, name: String, hex_value: String) -> Element<ThemeEditorMessage> {
        let color_preview = container(text(" "))
            .width(Length::Units(20))
            .height(Length::Units(20))
            .style(iced::theme::Container::Custom(Box::new(ColorPreviewStyle {
                color: parse_hex_color(&hex_value).unwrap_or(Color::BLACK),
            })));

        row![
            text(name.clone()).width(Length::FillPortion(1)),
            color_preview,
            text_input("Hex", &hex_value)
                .on_input(move |s| ThemeEditorMessage::EditColor(name.clone(), s))
                .width(Length::FillPortion(2)),
            button(text("Delete")).on_press(ThemeEditorMessage::DeleteCustomColor(name.clone()))
                .style(iced::theme::Button::Destructive)
                .width(Length::Shrink)
                .into(),
        ].spacing(10).align_items(iced::Alignment::Center).into()
    }
}

// Helper to convert hex string to iced::Color
fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::from_rgb8(r, g, b))
    } else {
        None
    }
}

// Custom style for color preview
struct ColorPreviewStyle {
    color: Color,
}

impl iced::widget::container::StyleSheet for ColorPreviewStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(self.color.into()),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::BLACK,
            ..Default::default()
        }
    }
}

pub fn init() {
    info!("settings/theme_editor module loaded");
}
