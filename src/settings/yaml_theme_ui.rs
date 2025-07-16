use iced::{Element, widget::{column, row, text, button, text_input, scrollable, pick_list, container}};
use crate::config::yaml_theme_manager::{YamlThemeManager, ThemeMetadata};
use crate::config::{ThemeConfig, yaml_theme::{YamlTheme, YamlColors, YamlThemeError}};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct YamlThemeUI {
    theme_manager: YamlThemeManager,
    selected_theme: Option<String>,
    import_text: String,
    export_text: String,
    theme_metadata: Vec<ThemeMetadata>,
    show_import_dialog: bool,
    show_export_dialog: bool,
    import_error: Option<String>,
    search_query: String,
    theme_editor: YamlThemeEditor,
}

#[derive(Debug, Clone)]
pub enum Message {
    ThemeSelected(String),
    ImportTheme,
    ExportTheme(ThemeConfig),
    ImportTextChanged(String),
    ImportFromText,
    ImportFromFile,
    ExportToFile(ThemeConfig),
    DeleteTheme(String),
    RefreshThemes,
    SearchChanged(String),
    ShowImportDialog(bool),
    ShowExportDialog(bool),
    ClearError,
    ThemeNameChanged(String),
    BackgroundColorChanged(String),
    ForegroundColorChanged(String),
    PrimaryColorChanged(String),
    SecondaryColorChanged(String),
    DangerColorChanged(String),
    TextColorChanged(String),
    BorderColorChanged(String),
    LoadTheme(String),
    SaveTheme,
}

impl YamlThemeUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let theme_manager = YamlThemeManager::new()?;
        let theme_metadata = theme_manager.get_all_metadata();
        let theme_editor = YamlThemeEditor::new();

        Ok(Self {
            theme_manager,
            selected_theme: None,
            import_text: String::new(),
            export_text: String::new(),
            theme_metadata,
            show_import_dialog: false,
            show_export_dialog: false,
            import_error: None,
            search_query: String::new(),
            theme_editor,
        })
    }

    pub fn update(&mut self, message: Message) -> Option<ThemeConfig> {
        match message {
            Message::ThemeSelected(name) => {
                self.selected_theme = Some(name.clone());
                self.theme_manager.get_theme(&name)
            }
            Message::ImportTextChanged(text) => {
                self.import_text = text;
                None
            }
            Message::ImportFromText => {
                match self.theme_manager.import_theme_from_string(&self.import_text, None) {
                    Ok(theme_name) => {
                        self.import_text.clear();
                        self.show_import_dialog = false;
                        self.import_error = None;
                        self.refresh_metadata();
                        self.theme_manager.get_theme(&theme_name)
                    }
                    Err(e) => {
                        self.import_error = Some(format!("Import failed: {}", e));
                        None
                    }
                }
            }
            Message::ExportTheme(theme) => {
                match self.theme_manager.export_theme_to_string(&theme) {
                    Ok(yaml_str) => {
                        self.export_text = yaml_str;
                        self.show_export_dialog = true;
                        None
                    }
                    Err(e) => {
                        self.import_error = Some(format!("Export failed: {}", e));
                        None
                    }
                }
            }
            Message::DeleteTheme(name) => {
                if let Err(e) = self.theme_manager.delete_theme(&name) {
                    self.import_error = Some(format!("Delete failed: {}", e));
                } else {
                    self.refresh_metadata();
                    if self.selected_theme.as_ref() == Some(&name) {
                        self.selected_theme = None;
                    }
                }
                None
            }
            Message::RefreshThemes => {
                if let Err(e) = self.theme_manager.scan_themes() {
                    self.import_error = Some(format!("Refresh failed: {}", e));
                } else {
                    self.refresh_metadata();
                }
                None
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                None
            }
            Message::ShowImportDialog(show) => {
                self.show_import_dialog = show;
                if !show {
                    self.import_text.clear();
                    self.import_error = None;
                }
                None
            }
            Message::ShowExportDialog(show) => {
                self.show_export_dialog = show;
                if !show {
                    self.export_text.clear();
                }
                None
            }
            Message::ClearError => {
                self.import_error = None;
                None
            }
            Message::ThemeNameChanged(name) => {
                self.theme_editor.update(Message::ThemeNameChanged(name));
                None
            }
            Message::BackgroundColorChanged(color) => {
                self.theme_editor.update(Message::BackgroundColorChanged(color));
                None
            }
            Message::ForegroundColorChanged(color) => {
                self.theme_editor.update(Message::ForegroundColorChanged(color));
                None
            }
            Message::PrimaryColorChanged(color) => {
                self.theme_editor.update(Message::PrimaryColorChanged(color));
                None
            }
            Message::SecondaryColorChanged(color) => {
                self.theme_editor.update(Message::SecondaryColorChanged(color));
                None
            }
            Message::DangerColorChanged(color) => {
                self.theme_editor.update(Message::DangerColorChanged(color));
                None
            }
            Message::TextColorChanged(color) => {
                self.theme_editor.update(Message::TextColorChanged(color));
                None
            }
            Message::BorderColorChanged(color) => {
                self.theme_editor.update(Message::BorderColorChanged(color));
                None
            }
            Message::LoadTheme(name) => {
                self.theme_editor.update(Message::LoadTheme(name));
                None
            }
            Message::SaveTheme => {
                self.theme_editor.update(Message::SaveTheme);
                None
            }
        }
    }

    fn refresh_metadata(&mut self) {
        self.theme_metadata = self.theme_manager.get_all_metadata();
    }

    pub fn view(&mut self) -> Element<Message> {
        let main_content = column![
            self.create_header(),
            self.create_theme_list(),
            self.create_actions(),
        ]
        .spacing(16);

        if self.show_import_dialog {
            self.create_import_dialog()
        } else if self.show_export_dialog {
            self.create_export_dialog()
        } else {
            column![
                main_content,
                self.theme_editor.view(),
            ]
            .into()
        }
    }

    fn create_header(&self) -> Element<Message> {
        row![
            text("YAML Themes").size(20),
            // Spacer
            iced::widget::horizontal_space(iced::Length::Fill),
            text_input("Search themes...", &self.search_query)
                .on_input(Message::SearchChanged)
                .width(iced::Length::Fixed(200.0)),
            button("Refresh")
                .on_press(Message::RefreshThemes),
            button("Import")
                .on_press(Message::ShowImportDialog(true)),
        ]
        .spacing(8)
        .align_items(iced::Alignment::Center)
        .into()
    }

    fn create_theme_list(&self) -> Element<Message> {
        let filtered_themes: Vec<_> = self.theme_metadata
            .iter()
            .filter(|metadata| {
                if self.search_query.is_empty() {
                    true
                } else {
                    metadata.name.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                    metadata.author.as_ref().map_or(false, |a| a.to_lowercase().contains(&self.search_query.to_lowercase()))
                }
            })
            .collect();

        if filtered_themes.is_empty() {
            return container(
                text("No themes found")
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text.scale_alpha(0.7)),
                    })
            )
            .center_x()
            .center_y()
            .height(iced::Length::Fixed(200.0))
            .into();
        }

        scrollable(
            column(
                filtered_themes
                    .into_iter()
                    .map(|metadata| self.create_theme_card(metadata))
                    .collect::<Vec<_>>()
            )
            .spacing(8)
        )
        .height(iced::Length::Fixed(400.0))
        .into()
    }

    fn create_theme_card(&self, metadata: &ThemeMetadata) -> Element<Message> {
        let is_selected = self.selected_theme.as_ref() == Some(&metadata.name);
        
        let card_content = column![
            row![
                text(&metadata.name)
                    .size(16)
                    .style(move |theme| iced::widget::text::Appearance {
                        color: Some(if is_selected {
                            theme.palette().primary
                        } else {
                            theme.palette().text
                        }),
                    }),
                // Spacer
                iced::widget::horizontal_space(iced::Length::Fill),
                if metadata.is_dark {
                    text("Dark").size(12)
                } else {
                    text("Light").size(12)
                }
                .style(|theme| iced::widget::text::Appearance {
                    color: Some(theme.palette().text.scale_alpha(0.7)),
                }),
            ]
            .align_items(iced::Alignment::Center),
            
            if let Some(author) = &metadata.author {
                row![
                    text("by").size(12),
                    text(author).size(12)
                ]
                .spacing(4)
                .into()
            } else {
                iced::widget::Space::new(0, 0).into()
            },
            
            if let Some(description) = &metadata.description {
                text(description)
                    .size(12)
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text.scale_alpha(0.8)),
                    })
                    .into()
            } else {
                iced::widget::Space::new(0, 0).into()
            },
            
            row![
                button("Select")
                    .on_press(Message::ThemeSelected(metadata.name.clone()))
                    .style(if is_selected {
                        button::primary
                    } else {
                        button::secondary
                    }),
                button("Delete")
                    .on_press(Message::DeleteTheme(metadata.name.clone()))
                    .style(button::danger),
            ]
            .spacing(8),
        ]
        .spacing(8);

        container(card_content)
            .padding(16)
            .style(move |theme| iced::widget::container::Appearance {
                background: Some(if is_selected {
                    theme.palette().primary.scale_alpha(0.1).into()
                } else {
                    theme.palette().background.into()
                }),
                border: iced::Border {
                    color: if is_selected {
                        theme.palette().primary
                    } else {
                        theme.palette().text.scale_alpha(0.1)
                    },
                    width: if is_selected { 2.0 } else { 1.0 },
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn create_import_dialog(&self) -> Element<Message> {
        column![
            text("Import Theme").size(20),
            text_input("Paste YAML here...", &self.import_text)
                .on_input(Message::ImportTextChanged)
                .width(iced::Length::Fill),
            button("Import from Text")
                .on_press(Message::ImportFromText),
            button("Import from File")
                .on_press(Message::ImportFromFile),
            button("Cancel")
                .on_press(Message::ShowImportDialog(false)),
        ]
        .spacing(10)
        .into()
    }

    fn create_export_dialog(&self) -> Element<Message> {
        column![
            text("Export Theme").size(20),
            text(&self.export_text)
                .width(iced::Length::Fill),
            button("Export to File")
                .on_press(Message::ExportToFile(ThemeConfig::new())),
            button("Cancel")
                .on_press(Message::ShowExportDialog(false)),
        ]
        .spacing(10)
        .into()
    }

    fn create_actions(&self) -> Element<Message> {
        row![
            button("Export")
                .on_press(Message::ShowExportDialog(true)),
            button("Edit")
                .on_press(Message::LoadTheme("Custom Theme".to_string())),
        ]
        .spacing(8)
        .into()
    }
}

#[derive(Debug, Clone)]
pub struct YamlThemeEditor {
    pub current_theme: YamlTheme,
    // Add fields for editing individual color strings
    background_input: String,
    foreground_input: String,
    primary_input: String,
    secondary_input: String,
    danger_input: String,
    text_input: String,
    border_input: String,
}

impl YamlThemeEditor {
    pub fn new() -> Self {
        let default_theme = YamlTheme {
            name: "Custom Theme".to_string(),
            colors: YamlColors {
                background: "#F0F0F0".to_string(),
                foreground: "#000000".to_string(),
                primary: "#0078D7".to_string(),
                secondary: "#646464".to_string(),
                danger: "#C80000".to_string(),
                text: "#000000".to_string(),
                border: "#C8C8C8".to_string(),
            },
        };
        Self::from_theme(default_theme)
    }

    fn from_theme(theme: YamlTheme) -> Self {
        Self {
            background_input: theme.colors.background.clone(),
            foreground_input: theme.colors.foreground.clone(),
            primary_input: theme.colors.primary.clone(),
            secondary_input: theme.colors.secondary.clone(),
            danger_input: theme.colors.danger.clone(),
            text_input: theme.colors.text.clone(),
            border_input: theme.colors.border.clone(),
            current_theme: theme,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ThemeNameChanged(name) => self.current_theme.name = name,
            Message::BackgroundColorChanged(color) => {
                self.background_input = color.clone();
                self.current_theme.colors.background = color;
            }
            Message::ForegroundColorChanged(color) => {
                self.foreground_input = color.clone();
                self.current_theme.colors.foreground = color;
            }
            Message::PrimaryColorChanged(color) => {
                self.primary_input = color.clone();
                self.current_theme.colors.primary = color;
            }
            Message::SecondaryColorChanged(color) => {
                self.secondary_input = color.clone();
                self.current_theme.colors.secondary = color;
            }
            Message::DangerColorChanged(color) => {
                self.danger_input = color.clone();
                self.current_theme.colors.danger = color;
            }
            Message::TextColorChanged(color) => {
                self.text_input = color.clone();
                self.current_theme.colors.text = color;
            }
            Message::BorderColorChanged(color) => {
                self.border_input = color.clone();
                self.current_theme.colors.border = color;
            }
            Message::LoadTheme(name) => {
                // In a real app, you'd load from YamlThemeManager
                // For now, just a placeholder
            }
            Message::SaveTheme => {
                // In a real app, you'd save to a file
                let yaml_string = serde_yaml::to_string(&self.current_theme).unwrap();
                println!("Saved Theme:\n{}", yaml_string);
            }
            _ => {}
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        let color_input = |label: &str, value: &str, on_change: fn(String) -> Message| {
            row![
                text(label).width(iced::Length::Units(100)),
                text_input("", value)
                    .on_input(on_change)
                    .padding(5)
                    .width(iced::Length::Fill),
            ]
            .spacing(10)
            .align_items(iced::Alignment::Center)
            .into()
        };

        column![
            text("YAML Theme Editor").size(20),
            row![
                text("Theme Name:").width(iced::Length::Units(100)),
                text_input("Custom Theme", &self.current_theme.name)
                    .on_input(Message::ThemeNameChanged)
                    .padding(5)
                    .width(iced::Length::Fill),
            ]
            .spacing(10)
            .align_items(iced::Alignment::Center),
            
            color_input("Background:", &self.background_input, Message::BackgroundColorChanged),
            color_input("Foreground:", &self.foreground_input, Message::ForegroundColorChanged),
            color_input("Primary:", &self.primary_input, Message::PrimaryColorChanged),
            color_input("Secondary:", &self.secondary_input, Message::SecondaryColorChanged),
            color_input("Danger:", &self.danger_input, Message::DangerColorChanged),
            color_input("Text:", &self.text_input, Message::TextColorChanged),
            color_input("Border:", &self.border_input, Message::BorderColorChanged),

            button(text("Save Theme")).on_press(Message::SaveTheme),
        ]
        .spacing(10)
        .into()
    }
}
