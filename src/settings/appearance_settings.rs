use iced::{
    widget::{column, row, text, toggler, slider, pick_list, radio, container},
    Element, Length, Color, alignment,
};
use crate::config::{AppConfig, preferences::{InputType, InputPosition}};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AppearanceSettingsMessage {
    SyncWithOsThemeToggled(bool),
    AppIconSelected(String),
    OpenNewWindowsCustomSizeToggled(bool),
    WindowOpacityChanged(f32),
    WindowBlurRadiusChanged(f32),
    InputTypeSelected(InputType),
    InputPositionSelected(InputPosition),
    DimInactivePanesToggled(bool),
    FocusFollowsMouseToggled(bool),
}

#[derive(Debug)]
pub struct AppearanceSettings {
    config: AppConfig,
    // Add any local state needed for the UI, e.g., dropdown options
    app_icon_options: Vec<String>,
    input_position_options: Vec<InputPosition>,
}

impl AppearanceSettings {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            app_icon_options: vec!["Default".to_string(), "Custom1".to_string(), "Custom2".to_string()], // Example options
            input_position_options: vec![InputPosition::PinToBottom], // Only one option shown in UI
        }
    }

    pub fn update(&mut self, message: AppearanceSettingsMessage) {
        let mut prefs = self.config.preferences.clone();
        match message {
            AppearanceSettingsMessage::SyncWithOsThemeToggled(value) => {
                prefs.ui.sync_with_os_theme = value;
            }
            AppearanceSettingsMessage::AppIconSelected(value) => {
                prefs.ui.app_icon = value;
            }
            AppearanceSettingsMessage::OpenNewWindowsCustomSizeToggled(value) => {
                prefs.ui.open_new_windows_custom_size = value;
            }
            AppearanceSettingsMessage::WindowOpacityChanged(value) => {
                prefs.ui.window_opacity = value;
            }
            AppearanceSettingsMessage::WindowBlurRadiusChanged(value) => {
                prefs.ui.window_blur_radius = value;
            }
            AppearanceSettingsMessage::InputTypeSelected(value) => {
                prefs.ui.input_type = value;
            }
            AppearanceSettingsMessage::InputPositionSelected(value) => {
                prefs.ui.input_position = value;
            }
            AppearanceSettingsMessage::DimInactivePanesToggled(value) => {
                prefs.ui.dim_inactive_panes = value;
            }
            AppearanceSettingsMessage::FocusFollowsMouseToggled(value) => {
                prefs.ui.focus_follows_mouse = value;
            }
        }
        self.config.preferences = prefs;
    }

    pub fn view(&self) -> Element<AppearanceSettingsMessage> {
        let prefs = &self.config.preferences;

        let themes_section = column![
            text("Themes").size(24).style(Color::WHITE),
            row![
                text("Create your own custom theme").style(Color::from_rgb(0.2, 0.6, 1.0)), // Blue link color
            ].spacing(5),
            row![
                text("Sync with OS").style(Color::WHITE),
                toggler(
                    prefs.ui.sync_with_os_theme,
                    AppearanceSettingsMessage::SyncWithOsThemeToggled,
                ).width(Length::Shrink),
            ].spacing(10).align_items(alignment::Horizontal::Center),
            text("Automatically switch between light and dark themes when your system does.").size(14).style(Color::from_rgb(0.7, 0.7, 0.7)),
            row![
                column![
                    text("Light").style(Color::WHITE),
                    container(
                        text("ls\ndir executable file\n|")
                            .size(12)
                            .style(Color::BLACK)
                    )
                    .width(Length::Units(150))
                    .height(Length::Units(100))
                    .style(iced::widget::container::Appearance {
                        background: Some(iced::Background::Color(Color::WHITE)),
                        border_radius: 5.0,
                        border_width: 1.0,
                        border_color: Color::from_rgb(0.8, 0.8, 0.8),
                        ..Default::default()
                    })
                ].spacing(5),
                column![
                    text("Dark").style(Color::WHITE),
                    container(
                        text("ls\ndir executable file\n|")
                            .size(12)
                            .style(Color::WHITE)
                    )
                    .width(Length::Units(150))
                    .height(Length::Units(100))
                    .style(iced::widget::container::Appearance {
                        background: Some(iced::Background::Color(Color::BLACK)),
                        border_radius: 5.0,
                        border_width: 1.0,
                        border_color: Color::from_rgb(0.2, 0.2, 0.2),
                        ..Default::default()
                    })
                ].spacing(5),
            ].spacing(20).padding(10),
        ].spacing(15);

        let icon_section = column![
            text("Icon").size(24).style(Color::WHITE),
            text("Customize your app icon").size(14).style(Color::from_rgb(0.7, 0.7, 0.7)),
            pick_list(
                self.app_icon_options.clone(),
                Some(prefs.ui.app_icon.clone()),
                AppearanceSettingsMessage::AppIconSelected,
            ).width(Length::Units(200)),
        ].spacing(15);

        let window_section = column![
            text("Window").size(24).style(Color::WHITE),
            row![
                text("Open new windows with custom size").style(Color::WHITE),
                toggler(
                    prefs.ui.open_new_windows_custom_size,
                    AppearanceSettingsMessage::OpenNewWindowsCustomSizeToggled,
                ).width(Length::Shrink),
            ].spacing(10).align_items(alignment::Horizontal::Center),
            row![
                text(format!("Window Opacity: {:.0}%", prefs.ui.window_opacity * 100.0)).style(Color::WHITE),
                slider(
                    0.0..=1.0,
                    prefs.ui.window_opacity,
                    AppearanceSettingsMessage::WindowOpacityChanged,
                ).width(Length::Units(200)),
            ].spacing(10).align_items(alignment::Horizontal::Center),
            row![
                text(format!("Window Blur Radius: {:.0}", prefs.ui.window_blur_radius)).style(Color::WHITE),
                text("ⓘ").size(14).style(Color::from_rgb(0.7, 0.7, 0.7)), // Info icon placeholder
                slider(
                    0.0..=10.0, // Example range for blur radius
                    prefs.ui.window_blur_radius,
                    AppearanceSettingsMessage::WindowBlurRadiusChanged,
                ).width(Length::Units(200)),
            ].spacing(10).align_items(alignment::Horizontal::Center),
        ].spacing(15);

        let input_section = column![
            text("Input").size(24).style(Color::WHITE),
            text("Input type").style(Color::WHITE),
            row![
                radio(
                    "Universal",
                    InputType::Universal,
                    Some(prefs.ui.input_type),
                    InputType::Universal,
                    AppearanceSettingsMessage::InputTypeSelected,
                ).spacing(5),
                radio(
                    "Classic",
                    InputType::Classic,
                    Some(prefs.ui.input_type),
                    InputType::Classic,
                    AppearanceSettingsMessage::InputTypeSelected,
                ).spacing(5),
            ].spacing(20),
            row![
                text("Input position").style(Color::WHITE),
                pick_list(
                    self.input_position_options.clone(),
                    Some(prefs.ui.input_position),
                    AppearanceSettingsMessage::InputPositionSelected,
                ).width(Length::Units(200)),
            ].spacing(10).align_items(alignment::Horizontal::Center),
        ].spacing(15);

        let panes_section = column![
            text("Panes").size(24).style(Color::WHITE),
            row![
                text("Dim inactive panes").style(Color::WHITE),
                toggler(
                    prefs.ui.dim_inactive_panes,
                    AppearanceSettingsMessage::DimInactivePanesToggled,
                ).width(Length::Shrink),
            ].spacing(10).align_items(alignment::Horizontal::Center),
            row![
                text("Focus follows mouse").style(Color::WHITE),
                toggler(
                    prefs.ui.focus_follows_mouse,
                    AppearanceSettingsMessage::FocusFollowsMouseToggled,
                ).width(Length::Shrink),
            ].spacing(10).align_items(alignment::Horizontal::Center),
        ].spacing(15);

        let blocks_section = column![
            text("Blocks").size(24).style(Color::WHITE),
            // No specific controls shown in the image for this section
        ].spacing(15);

        column![
            themes_section,
            iced::widget::horizontal_rule(1),
            icon_section,
            iced::widget::horizontal_rule(1),
            window_section,
            iced::widget::horizontal_rule(1),
            input_section,
            iced::widget::horizontal_rule(1),
            panes_section,
            iced::widget::horizontal_rule(1),
            blocks_section,
        ]
        .spacing(20)
        .padding(20)
        .into()
    }
}

impl std::fmt::Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::fmt::Display for InputPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputPosition::PinToBottom => write!(f, "Pin to the bottom (Warp mode)"),
        }
    }
}
