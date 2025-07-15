use iced::{Element, widget::{column, row, text, button, text_input, scrollable, pick_list, container}};
use crate::config::yaml_theme_manager::{YamlThemeManager, ThemeMetadata};
use crate::config::{ThemeConfig, yaml_theme::YamlThemeError};

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
}

impl YamlThemeUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let theme_manager = YamlThemeManager::new()?;
        let theme_metadata = theme_manager.get_all_metadata();

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
            _ => None,
        }
    }

    fn refresh_metadata(&mut self) {
        self.theme_metadata = self.theme_manager.get_all_metadata();
    }

    pub fn view(&self) -> Element<Message> {
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
            main_content.into()
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
