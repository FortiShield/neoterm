use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::block::{BlockManager, BlockType};
use crate::config::{ConfigManager, theme::Theme};
use crate::main::Message; // Import the main app's Message enum

pub struct Renderer {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    config_manager: Arc<ConfigManager>,
    app_sender: mpsc::Sender<Message>, // To send messages back to the app loop
}

impl Renderer {
    pub async fn new(config_manager: Arc<ConfigManager>) -> Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        
        // Create a dummy sender for now, will be replaced by the actual app_tx
        let (app_tx, _) = mpsc::channel(1); 

        Ok(Self {
            terminal,
            config_manager,
            app_sender: app_tx, // Placeholder
        })
    }

    // This method should be called by `main` to provide the actual app_tx
    pub fn set_app_sender(&mut self, sender: mpsc::Sender<Message>) {
        self.app_sender = sender;
    }

    pub fn get_app_sender(&self) -> mpsc::Sender<Message> {
        self.app_sender.clone()
    }

    pub async fn render(&mut self, block_manager: &mut BlockManager) -> Result<()> {
        let current_theme = self.config_manager.get_current_theme().await?;

        self.terminal.draw(|frame| {
            let size = frame.size();
            block_manager.update_layout(size); // Update block areas based on current terminal size

            for block_state in &block_manager.blocks {
                let border_style = if block_state.is_active {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(Span::styled(
                        block_state.title.clone(),
                        Style::default().fg(Self::hex_to_color(current_theme.get_color("bright_green").unwrap_or(&"#23d18b".to_string()))),
                    ));

                let paragraph = Paragraph::new(block_state.content.clone())
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false });

                frame.render_widget(paragraph, block_state.area);
            }
        })?;
        Ok(())
    }

    pub async fn resize(&mut self, width: u16, height: u16) -> Result<()> {
        self.terminal.resize(Rect::new(0, 0, width, height))?;
        Ok(())
    }

    fn hex_to_color(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        } else {
            Color::White // Default to white on error
        }
    }
}
