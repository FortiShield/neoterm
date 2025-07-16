use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tui_textarea::{TextArea, Input, Key};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_mode_eval::{AgentModeEvaluator, ai_client::ChatMessage};

pub struct AiSidebar {
    evaluator: Arc<AgentModeEvaluator>,
    chat_history: Vec<ChatMessage>,
    input_area: TextArea<'static>,
    is_active: bool,
    scroll_offset: usize,
    error_message: Option<String>,
}

impl AiSidebar {
    pub fn new(evaluator: Arc<AgentModeEvaluator>) -> Self {
        Self {
            evaluator,
            chat_history: Vec::new(),
            input_area: TextArea::default(),
            is_active: false,
            scroll_offset: 0,
            error_message: None,
        }
    }

    pub async fn init(&mut self) {
        log::info!("AI sidebar initialized.");
        self.chat_history = self.evaluator.get_conversation_history().await;
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    pub async fn handle_input(&mut self, input: Input) {
        self.error_message = None; // Clear error on new input

        match input {
            Input { key: Key::Enter, .. } => {
                let user_message = self.input_area.lines().join("\n");
                self.input_area = TextArea::default(); // Clear input

                if user_message.trim().is_empty() {
                    return;
                }

                log::info!("Sending message to AI: {}", user_message);
                let evaluator_clone = self.evaluator.clone();
                let user_message_clone = user_message.clone();
                let self_arc = Arc::new(Mutex::new(self)); // Temporarily wrap self in Arc<Mutex>

                tokio::spawn(async move {
                    let mut locked_self = self_arc.lock().await;
                    match evaluator_clone.handle_user_input(user_message_clone).await {
                        Ok(response_messages) => {
                            locked_self.chat_history = evaluator_clone.get_conversation_history().await;
                            locked_self.scroll_to_bottom();
                        },
                        Err(e) => {
                            log::error!("Error from AI: {:?}", e);
                            locked_self.error_message = Some(format!("AI Error: {}", e));
                        }
                    }
                });
            },
            Input { key: Key::Up, .. } => {
                if self.scroll_offset < self.chat_history.len() {
                    self.scroll_offset += 1;
                }
            },
            Input { key: Key::Down, .. } => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            },
            _ => {
                self.input_area.input(input);
            }
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0; // Reset scroll to show latest messages
    }

    pub async fn update_chat_history(&mut self) {
        self.chat_history = self.evaluator.get_conversation_history().await;
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1), // Chat history
                ratatui::layout::Constraint::Length(self.input_area.lines().len() as u128 + 2), // Input area + borders
                ratatui::layout::Constraint::Length(1), // Error message
            ])
            .split(area);

        // Chat History Block
        let chat_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled("AI Assistant", Style::default().fg(Color::LightGreen)));

        let mut chat_lines: Vec<Line> = Vec::new();
        for msg in self.chat_history.iter().rev().skip(self.scroll_offset) {
            let role_style = match msg.role.as_str() {
                "user" => Style::default().fg(Color::LightBlue),
                "assistant" => Style::default().fg(Color::LightYellow),
                "system" => Style::default().fg(Color::LightMagenta),
                "tool" => Style::default().fg(Color::LightCyan),
                _ => Style::default().fg(Color::White),
            };
            chat_lines.push(Line::from(vec![
                Span::styled(format!("{}: ", msg.role), role_style),
                Span::raw(msg.content.clone()),
            ]));
            if chat_lines.len() as u16 >= chunks[0].height - 2 { // -2 for borders
                break;
            }
        }
        chat_lines.reverse(); // Display in chronological order

        let chat_paragraph = Paragraph::new(chat_lines)
            .block(chat_block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(chat_paragraph, chunks[0]);

        // Input Area Block
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled("Your Message", Style::default().fg(Color::LightGreen)));
        
        let mut input_widget = self.input_area.widget();
        input_widget = input_widget.block(input_block);
        frame.render_widget(input_widget, chunks[1]);

        // Error Message Block
        if let Some(err) = &self.error_message {
            let error_paragraph = Paragraph::new(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red),
            )));
            frame.render_widget(error_paragraph, chunks[2]);
        }
    }
}
