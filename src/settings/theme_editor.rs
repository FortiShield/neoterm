use iced::{Element, widget::{column, row, text, button, text_input, pick_list, container, scrollable}, Length};
use iced::Color;
use std::collections::HashMap;
use crate::config::{ThemeConfig, TerminalColors};
use crate::config::yaml_theme_manager::YamlThemeManager;
use crate::settings::Settings;

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
    settings: Settings,
    yaml_theme_manager: YamlThemeManager,
    available_themes: Vec<String>,
    selected_theme_name: Option<String>,
    current_theme_config: ThemeConfig, // The theme currently being edited
    new_custom_color_name: String,
    new_custom_color_value: String,
}

impl ThemeEditor {
    pub fn new(settings: Settings, theme_dir: std::path::PathBuf) -> Self {
        let mut manager = YamlThemeManager::new(theme_dir);
        let _ = manager.load_themes(); // Load themes on startup
        let available_themes = manager.get_available_theme_names();

        let default_theme_name = settings.theme.name.clone();
        let current_theme_config = manager.get_theme_config(&default_theme_name)
            .unwrap_or_else(ThemeConfig::default);

        Self {
            settings,
            yaml_theme_manager: manager,
            available_themes,
            selected_theme_name: Some(default_theme_name),
            current_theme_config,
            new_custom_color_name: String::new(),
            new_custom_color_value: String::new(),
        }
    }

    pub fn update(&mut self, message: ThemeEditorMessage) -> Command<ThemeEditorMessage> {
        match message {
            ThemeEditorMessage::LoadThemes => {
                let _ = self.yaml_theme_manager.load_themes();
                self.available_themes = self.yaml_theme_manager.get_available_theme_names();
                Command::none()
            }
            ThemeEditorMessage::SelectTheme(name) => {
                if let Some(theme_config) = self.yaml_theme_manager.get_theme_config(&name) {
                    self.selected_theme_name = Some(name);
                    self.current_theme_config = theme_config;
                    // Update main settings struct as well
                    self.settings.theme = self.current_theme_config.clone();
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
                self.settings.theme = self.current_theme_config.clone();
                Command::none()
            }
            ThemeEditorMessage::SaveTheme => {
                // In a real application, you'd save self.current_theme_config
                // to a YAML file in the themes directory.
                println!("Saving theme: {}", self.current_theme_config.name);
                // For now, just update the settings struct
                self.settings.theme = self.current_theme_config.clone();
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
                    self.settings.theme = self.current_theme_config.clone();
                }
                Command::none()
            }
            ThemeEditorMessage::DeleteCustomColor(color_name) => {
                self.current_theme_config.colors.remove(&color_name);
                // Update main settings struct as well
                self.settings.theme = self.current_theme_config.clone();
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

    pub fn get_updated_settings(&self) -> &Settings {
        &self.settings
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
    println!("settings/theme_editor module loaded");
}
