use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, Clear},
    Frame,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use std::collections::VecDeque;
use crate::agent_mode_eval::{AIClient, Conversation, Message, MessageRole};
use iced::{Element, widget::{column, text, container}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

pub struct AISidebar {
    pub is_open: bool,
    pub width_percentage: u16,
    pub messages: VecDeque<ChatMessage>,
    pub input_buffer: String,
    pub is_loading: bool,
    pub scroll_offset: usize,
    pub ai_client: Option<AIClient>,
    pub conversation: Option<Conversation>,
    pub response_receiver: Option<mpsc::UnboundedReceiver<String>>,
    pub current_response: String,
}

impl AISidebar {
    pub fn new() -> Self {
        Self {
            is_open: false,
            width_percentage: 30,
            messages: VecDeque::new(),
            input_buffer: String::new(),
            is_loading: false,
            scroll_offset: 0,
            ai_client: None,
            conversation: None,
            response_receiver: None,
            current_response: String::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open && self.ai_client.is_none() {
            // Initialize AI client when first opened
            if let Ok(client) = AIClient::new() {
                self.ai_client = Some(client);
                self.conversation = Some(Conversation::new());
                self.add_system_message("AI Assistant ready. How can I help you with your terminal session?");
            }
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) {
        let message = ChatMessage {
            role,
            content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        
        self.messages.push_back(message.clone());
        
        // Keep only last 100 messages to prevent memory issues
        if self.messages.len() > 100 {
            self.messages.pop_front();
        }

        // Add to conversation if available
        if let Some(ref mut conversation) = self.conversation {
            conversation.add_message(Message {
                role: message.role,
                content: message.content,
                timestamp: message.timestamp,
                metadata: message.metadata.unwrap_or_default(),
            });
        }

        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    pub fn add_system_message(&mut self, content: &str) {
        self.add_message(MessageRole::System, content.to_string());
    }

    pub fn add_user_message(&mut self, content: String) {
        self.add_message(MessageRole::User, content);
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.add_message(MessageRole::Assistant, content);
    }

    pub fn send_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.input_buffer.trim().is_empty() || self.is_loading {
            return Ok(());
        }

        let user_message = self.input_buffer.clone();
        self.input_buffer.clear();
        
        // Add user message to chat
        self.add_user_message(user_message.clone());
        
        // Send to AI if client is available
        if let (Some(ref mut client), Some(ref mut conversation)) = 
            (&mut self.ai_client, &mut self.conversation) {
            
            self.is_loading = true;
            self.current_response.clear();
            
            // Create a channel for streaming response
            let (tx, rx) = mpsc::unbounded_channel();
            self.response_receiver = Some(rx);
            
            // Send message asynchronously
            let conversation_clone = conversation.clone();
            let client_clone = client.clone();
            
            tokio::spawn(async move {
                match client_clone.send_message_stream(&conversation_clone, &user_message).await {
                    Ok(mut stream) => {
                        while let Some(chunk) = stream.recv().await {
                            if tx.send(chunk).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("Error: {}", e));
                    }
                }
            });
        }

        Ok(())
    }

    pub fn process_ai_response(&mut self) {
        if let Some(ref mut receiver) = self.response_receiver {
            while let Ok(chunk) = receiver.try_recv() {
                if chunk.starts_with("Error:") {
                    self.add_assistant_message(chunk);
                    self.is_loading = false;
                    self.response_receiver = None;
                    return;
                }
                
                self.current_response.push_str(&chunk);
                
                // Check if response is complete (this is a simple heuristic)
                if chunk.ends_with('\n') || chunk.ends_with('.') || chunk.ends_with('!') || chunk.ends_with('?') {
                    if !self.current_response.trim().is_empty() {
                        self.add_assistant_message(self.current_response.clone());
                        self.current_response.clear();
                    }
                    self.is_loading = false;
                    self.response_receiver = None;
                    break;
                }
            }
        }
    }

    pub fn add_char(&mut self, c: char) {
        if !self.is_loading {
            self.input_buffer.push(c);
        }
    }

    pub fn remove_char(&mut self) {
        if !self.is_loading {
            self.input_buffer.pop();
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = self.messages.len().saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        if let Some(ref mut conversation) = self.conversation {
            *conversation = Conversation::new();
        }
        self.add_system_message("Chat cleared. How can I help you?");
    }

    pub fn inject_terminal_context(&mut self, context: &str) {
        let context_message = format!("Terminal context: {}", context);
        self.add_system_message(&context_message);
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_open {
            return;
        }

        // Calculate sidebar width
        let sidebar_width = (area.width * self.width_percentage) / 100;
        let main_width = area.width - sidebar_width;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(main_width),
                Constraint::Length(sidebar_width),
            ])
            .split(area);

        let sidebar_area = chunks[1];

        // Split sidebar into messages and input
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(sidebar_area);

        // Render messages
        self.render_messages(f, sidebar_chunks[0]);
        
        // Render input
        self.render_input(f, sidebar_chunks[1]);
    }

    fn render_messages<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        
        let items: Vec<ListItem> = self.messages
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_height)
            .map(|(_, message)| {
                let (prefix, style) = match message.role {
                    MessageRole::User => ("You: ", Style::default().fg(Color::Cyan)),
                    MessageRole::Assistant => ("AI: ", Style::default().fg(Color::Green)),
                    MessageRole::System => ("System: ", Style::default().fg(Color::Yellow)),
                };

                let timestamp = message.timestamp.format("%H:%M");
                let content = format!("{}{}", prefix, message.content);
                
                // Wrap long messages
                let wrapped_lines = self.wrap_text(&content, area.width.saturating_sub(4) as usize);
                let lines: Vec<Line> = wrapped_lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, line)| {
                        if i == 0 {
                            Line::from(vec![
                                Span::styled(line, style),
                                Span::styled(format!(" [{}]", timestamp), Style::default().fg(Color::Gray)),
                            ])
                        } else {
                            Line::from(vec![Span::styled(line, style)])
                        }
                    })
                    .collect();

                ListItem::new(lines)
            })
            .collect();

        let messages_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("AI Assistant")
                    .border_style(Style::default().fg(Color::Magenta))
            );

        f.render_widget(messages_list, area);
    }

    fn render_input<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let input_text = if self.is_loading {
            "AI is thinking...".to_string()
        } else {
            self.input_buffer.clone()
        };

        let input_style = if self.is_loading {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(Color::White)
        };

        let input = Paragraph::new(input_text)
            .style(input_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Message")
                    .border_style(Style::default().fg(Color::Magenta))
            )
            .wrap(Wrap { trim: true });

        f.render_widget(input, area);
    }

    fn wrap_text(&self, text: &str, width: usize) -> Vec<String> {
        if width == 0 {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > width {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        lines
    }

    pub fn resize(&mut self, new_width_percentage: u16) {
        self.width_percentage = new_width_percentage.clamp(20, 50);
    }

    pub fn get_conversation_summary(&self) -> String {
        let message_count = self.messages.len();
        let user_messages = self.messages.iter()
            .filter(|m| matches!(m.role, MessageRole::User))
            .count();
        
        format!("Chat: {} messages ({} from user)", message_count, user_messages)
    }

    pub fn view(&self) -> Element<crate::Message> {
        container(
            column![
                text("AI Sidebar (Iced Placeholder)").size(20),
                text("Chat with AI here...").size(16),
            ]
            .spacing(10)
            .padding(10)
        )
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(iced::widget::container::Appearance {
            background: Some(iced::Color::from_rgb(0.15, 0.15, 0.15).into()),
            border_radius: 5.0.into(),
            border_width: 1.0,
            border_color: iced::Color::from_rgb(0.3, 0.3, 0.3),
            ..Default::default()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_sidebar_creation() {
        let sidebar = AISidebar::new();
        assert!(!sidebar.is_open);
        assert_eq!(sidebar.width_percentage, 30);
        assert!(sidebar.messages.is_empty());
    }

    #[test]
    fn test_message_handling() {
        let mut sidebar = AISidebar::new();
        sidebar.add_user_message("Hello".to_string());
        sidebar.add_assistant_message("Hi there!".to_string());
        
        assert_eq!(sidebar.messages.len(), 2);
        assert!(matches!(sidebar.messages[0].role, MessageRole::User));
        assert!(matches!(sidebar.messages[1].role, MessageRole::Assistant));
    }

    #[test]
    fn test_text_wrapping() {
        let sidebar = AISidebar::new();
        let wrapped = sidebar.wrap_text("This is a very long line that should be wrapped", 10);
        assert!(wrapped.len() > 1);
    }
}
