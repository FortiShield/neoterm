use anyhow::Result;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::sync::Arc;
use crate::config::{ConfigManager, theme::Theme, yaml_theme::YamlTheme};
use tui_textarea::{TextArea, Input, Key};

pub struct YamlThemeUi {
    config_manager: Arc<ConfigManager>,
    text_area: TextArea<'static>,
    current_theme_name: String,
    is_editing: bool,
    error_message: Option<String>,
}

impl YamlThemeUi {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        let text_area = TextArea::default();
        Self {
            config_manager,
            text_area,
            current_theme_name: "default".to_string(), // Initial dummy
            is_editing: false,
            error_message: None,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        log::info!("YAML theme UI initialized.");
        let prefs = self.config_manager.get_preferences().await;
        self.current_theme_name = prefs.theme_name.clone();
        self.load_theme_into_editor(&self.current_theme_name).await?;
        Ok(())
    }

    pub async fn load_theme_into_editor(&mut self, theme_name: &str) -> Result<()> {
        match self.config_manager.get_theme_manager().await.get_theme(theme_name).await {
            Ok(theme) => {
                let yaml_theme: YamlTheme = theme.into();
                let contents = yaml_theme.to_string()?;
                self.text_area = TextArea::from(contents.lines().map(|s| s.to_string()).collect::<Vec<String>>());
                self.current_theme_name = theme_name.to_string();
                self.is_editing = true;
                self.error_message = None;
                log::info!("Loaded theme '{}' into editor.", theme_name);
                Ok(())
            },
            Err(e) => {
                self.error_message = Some(format!("Failed to load theme: {}", e));
                log::error!("Failed to load theme '{}' into editor: {}", theme_name, e);
                Err(e)
            }
        }
    }

    pub async fn save_current_theme(&mut self) -> Result<()> {
        let contents = self.text_area.lines().join("\n");
        match YamlTheme::load_from_str(&contents) {
            Ok(yaml_theme) => {
                let theme: Theme = yaml_theme.into();
                match self.config_manager.get_theme_manager().await.save_theme(theme).await {
                    Ok(_) => {
                        self.error_message = None;
                        log::info!("Theme '{}' saved successfully.", self.current_theme_name);
                        Ok(())
                    },
                    Err(e) => {
                        self.error_message = Some(format!("Failed to save theme: {}", e));
                        log::error!("Failed to save theme '{}': {}", self.current_theme_name, e);
                        Err(e)
                    }
                }
            },
            Err(e) => {
                self.error_message = Some(format!("YAML parsing error: {}", e));
                log::error!("YAML parsing error when saving theme: {}", e);
                Err(e)
            }
        }
    }

    pub fn handle_input(&mut self, input: Input) {
        self.text_area.input(input);
        self.error_message = None; // Clear error on new input
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // For title
                Constraint::Min(0),    // For text area
                Constraint::Length(1), // For error message
            ])
            .split(area);

        let title_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(Span::styled(
                format!("Editing Theme: {}", self.current_theme_name),
                Style::default().fg(Color::LightYellow),
            ));
        frame.render_widget(title_block, chunks[0]);

        let widget = self.text_area.widget();
        frame.render_widget(widget, chunks[1]);

        if let Some(err) = &self.error_message {
            let error_paragraph = Paragraph::new(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red),
            )));
            frame.render_widget(error_paragraph, chunks[2]);
        }
    }
}
