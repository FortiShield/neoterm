use iced::{Element, widget::{column, row, text, button, text_input, slider, color_picker}};
use crate::config::{ThemeConfig, ColorScheme, ColorValue};

#[derive(Debug, Clone)]
pub struct ThemeEditor {
    theme: ThemeConfig,
    editing_color: Option<String>,
    preview_text: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ColorChanged(String, ColorValue),
    FontFamilyChanged(String),
    FontSizeChanged(f32),
    LineHeightChanged(f32),
    BorderRadiusChanged(f32),
    CreateTheme(String),
    LoadTheme(String),
    SaveTheme,
    ResetTheme,
    PreviewTextChanged(String),
}

impl ThemeEditor {
    pub fn new(theme: ThemeConfig) -> Self {
        Self {
            theme,
            editing_color: None,
            preview_text: "echo 'Hello, World!'\nls -la\ngit status".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Option<ThemeConfig> {
        match message {
            Message::ColorChanged(color_name, color) => {
                self.update_color(&color_name, color);
                Some(self.theme.clone())
            }
            Message::FontFamilyChanged(family) => {
                self.theme.typography.font_family = family;
                Some(self.theme.clone())
            }
            Message::FontSizeChanged(size) => {
                self.theme.typography.font_size = size;
                Some(self.theme.clone())
            }
            Message::LineHeightChanged(height) => {
                self.theme.typography.line_height = height;
                Some(self.theme.clone())
            }
            Message::BorderRadiusChanged(radius) => {
                self.theme.effects.border_radius = radius;
                Some(self.theme.clone())
            }
            Message::PreviewTextChanged(text) => {
                self.preview_text = text;
                None
            }
            Message::ResetTheme => {
                self.theme = ThemeConfig::default();
                Some(self.theme.clone())
            }
            _ => None,
        }
    }

    fn update_color(&mut self, color_name: &str, color: ColorValue) {
        match color_name {
            "background" => self.theme.colors.background = color,
            "surface" => self.theme.colors.surface = color,
            "text" => self.theme.colors.text = color,
            "primary" => self.theme.colors.primary = color,
            "secondary" => self.theme.colors.secondary = color,
            "accent" => self.theme.colors.accent = color,
            "success" => self.theme.colors.success = color,
            "warning" => self.theme.colors.warning = color,
            "error" => self.theme.colors.error = color,
            _ => {}
        }
    }

    pub fn view(&self) -> Element<Message> {
        column![
            text("Theme Editor").size(18),
            
            // Typography section
            text("Typography").size(16),
            row![
                text("Font Family:").width(iced::Length::Fixed(120.0)),
                text_input("Font name...", &self.theme.typography.font_family)
                    .on_input(Message::FontFamilyChanged)
            ].spacing(8),
            
            row![
                text("Font Size:").width(iced::Length::Fixed(120.0)),
                slider(8.0..=24.0, self.theme.typography.font_size, Message::FontSizeChanged)
            ].spacing(8),
            
            row![
                text("Line Height:").width(iced::Length::Fixed(120.0)),
                slider(1.0..=2.0, self.theme.typography.line_height, Message::LineHeightChanged)
            ].spacing(8),
            
            // Colors section
            text("Colors").size(16),
            self.create_color_section(),
            
            // Effects section
            text("Effects").size(16),
            row![
                text("Border Radius:").width(iced::Length::Fixed(120.0)),
                slider(0.0..=20.0, self.theme.effects.border_radius, Message::BorderRadiusChanged)
            ].spacing(8),
            
            // Preview section
            text("Preview").size(16),
            self.create_preview(),
            
            // Actions
            row![
                button("Reset Theme").on_press(Message::ResetTheme),
                button("Save Theme").on_press(Message::SaveTheme),
            ].spacing(8),
        ]
        .spacing(12)
        .into()
    }

    fn create_color_section(&self) -> Element<Message> {
        let colors = vec![
            ("Background", "background", &self.theme.colors.background),
            ("Surface", "surface", &self.theme.colors.surface),
            ("Text", "text", &self.theme.colors.text),
            ("Primary", "primary", &self.theme.colors.primary),
            ("Secondary", "secondary", &self.theme.colors.secondary),
            ("Accent", "accent", &self.theme.colors.accent),
            ("Success", "success", &self.theme.colors.success),
            ("Warning", "warning", &self.theme.colors.warning),
            ("Error", "error", &self.theme.colors.error),
        ];

        column(
            colors.into_iter()
                .map(|(label, key, color)| {
                    row![
                        text(label).width(iced::Length::Fixed(80.0)),
                        self.create_color_picker(key, color),
                        text(format!("#{:02X}{:02X}{:02X}", 
                            (color.r * 255.0) as u8,
                            (color.g * 255.0) as u8,
                            (color.b * 255.0) as u8
                        )).size(12)
                    ]
                    .spacing(8)
                    .into()
                })
                .collect::<Vec<_>>()
        )
        .spacing(8)
        .into()
    }

    fn create_color_picker(&self, key: &str, color: &ColorValue) -> Element<Message> {
        // In a real implementation, you'd use a proper color picker widget
        // For now, we'll use sliders for RGB values
        let key = key.to_string();
        
        row![
            slider(0.0..=1.0, color.r, move |r| {
                Message::ColorChanged(key.clone(), ColorValue { r, g: color.g, b: color.b, a: color.a })
            }),
            slider(0.0..=1.0, color.g, move |g| {
                Message::ColorChanged(key.clone(), ColorValue { r: color.r, g, b: color.b, a: color.a })
            }),
            slider(0.0..=1.0, color.b, move |b| {
                Message::ColorChanged(key.clone(), ColorValue { r: color.r, g: color.g, b, a: color.a })
            }),
        ]
        .spacing(4)
        .into()
    }

    fn create_preview(&self) -> Element<Message> {
        column![
            text_input("Preview text...", &self.preview_text)
                .on_input(Message::PreviewTextChanged),
            
            // Preview terminal block
            iced::widget::container(
                column![
                    text("$ echo 'Hello, World!'")
                        .style(|_| iced::widget::text::Appearance {
                            color: Some(self.theme.colors.primary.into()),
                        }),
                    text("Hello, World!")
                        .style(|_| iced::widget::text::Appearance {
                            color: Some(self.theme.colors.text.into()),
                        }),
                    text("$ ls -la")
                        .style(|_| iced::widget::text::Appearance {
                            color: Some(self.theme.colors.primary.into()),
                        }),
                    text("drwxr-xr-x  5 user user 4096 Jan 15 10:30 .")
                        .style(|_| iced::widget::text::Appearance {
                            color: Some(self.theme.colors.text.into()),
                        }),
                ]
                .spacing(4)
            )
            .padding(self.theme.spacing.block_padding)
            .style(move |_| iced::widget::container::Appearance {
                background: Some(self.theme.colors.terminal_background.into()),
                border: iced::Border {
                    color: self.theme.colors.border.into(),
                    width: 1.0,
                    radius: self.theme.effects.border_radius.into(),
                },
                ..Default::default()
            })
        ]
        .spacing(8)
        .into()
    }
}
