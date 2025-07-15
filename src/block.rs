use iced::{Element, widget::{column, row, text, button, container}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Block {
    pub id: Uuid,
    pub content: BlockContent,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum BlockContent {
    Command {
        input: String,
        output: Option<String>,
        exit_code: Option<i32>,
        working_directory: String,
    },
    AgentMessage {
        content: String,
        role: AgentRole,
    },
    UserMessage {
        content: String,
    },
    Error {
        message: String,
    },
    Separator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    Assistant,
    User,
    System,
}

impl Block {
    pub fn new_command(input: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: BlockContent::Command {
                input,
                output: None,
                exit_code: None,
                working_directory: std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "~".to_string()),
            },
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_agent_message(content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: BlockContent::AgentMessage {
                content,
                role: AgentRole::Assistant,
            },
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_user_message(content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: BlockContent::UserMessage { content },
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_error(message: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: BlockContent::Error { message },
            created_at: now,
            updated_at: now,
        }
    }

    pub fn set_output(&mut self, output: String, exit_code: i32) {
        if let BlockContent::Command { ref mut output: cmd_output, ref mut exit_code: cmd_exit_code, .. } = self.content {
            *cmd_output = Some(output);
            *cmd_exit_code = Some(exit_code);
            self.updated_at = Utc::now();
        }
    }

    pub fn view(&self) -> Element<crate::Message> {
        match &self.content {
            BlockContent::Command { input, output, exit_code, working_directory } => {
                self.view_command_block(input, output, exit_code, working_directory)
            }
            BlockContent::AgentMessage { content, role } => {
                self.view_agent_message_block(content, role)
            }
            BlockContent::UserMessage { content } => {
                self.view_user_message_block(content)
            }
            BlockContent::Error { message } => {
                self.view_error_block(message)
            }
            BlockContent::Separator => {
                container(text("─".repeat(80)))
                    .padding(8)
                    .into()
            }
        }
    }

    fn view_command_block(
        &self,
        input: &str,
        output: &Option<String>,
        exit_code: &Option<i32>,
        working_directory: &str,
    ) -> Element<crate::Message> {
        let header = row![
            text(format!("$ {}", input)).size(14),
            button("⟲").on_press(crate::Message::BlockAction(self.id, crate::BlockMessage::Rerun)),
            button("📋").on_press(crate::Message::BlockAction(self.id, crate::BlockMessage::Copy)),
            button("🗑").on_press(crate::Message::BlockAction(self.id, crate::BlockMessage::Delete)),
        ]
        .spacing(8);

        let mut content = vec![header.into()];

        if let Some(output_text) = output {
            let output_style = match exit_code {
                Some(0) => iced::theme::Text::Color(iced::Color::from_rgb(0.0, 0.8, 0.0)),
                Some(_) => iced::theme::Text::Color(iced::Color::from_rgb(0.8, 0.0, 0.0)),
                None => iced::theme::Text::Default,
            };

            content.push(
                container(
                    text(output_text)
                        .size(12)
                        .style(output_style)
                )
                .padding(8)
                .style(container::Appearance {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(0.05, 0.05, 0.05))),
                    border: iced::Border {
                        color: iced::Color::from_rgb(0.2, 0.2, 0.2),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .into()
            );
        }

        container(column(content).spacing(4))
            .padding(8)
            .style(container::Appearance {
                background: Some(iced::Background::Color(iced::Color::from_rgb(0.98, 0.98, 0.98))),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.9, 0.9, 0.9),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_agent_message_block(&self, content: &str, role: &AgentRole) -> Element<crate::Message> {
        let (icon, bg_color) = match role {
            AgentRole::Assistant => ("🤖", iced::Color::from_rgb(0.95, 0.98, 1.0)),
            AgentRole::User => ("👤", iced::Color::from_rgb(0.98, 1.0, 0.95)),
            AgentRole::System => ("⚙️", iced::Color::from_rgb(1.0, 0.98, 0.95)),
        };

        let header = row![
            text(format!("{} {:?}", icon, role)).size(12),
            button("📋").on_press(crate::Message::BlockAction(self.id, crate::BlockMessage::Copy)),
            button("🗑").on_press(crate::Message::BlockAction(self.id, crate::BlockMessage::Delete)),
        ]
        .spacing(8);

        let message_content = container(
            text(content).size(14)
        )
        .padding(12);

        container(
            column![header, message_content]
                .spacing(8)
        )
        .padding(8)
        .style(container::Appearance {
            background: Some(iced::Background::Color(bg_color)),
            border: iced::Border {
                color: iced::Color::from_rgb(0.8, 0.8, 0.8),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn view_user_message_block(&self, content: &str) -> Element<crate::Message> {
        container(
            row![
                text("👤").size(16),
                text(content).size(14)
            ]
            .spacing(8)
        )
        .padding(8)
        .style(container::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.98, 1.0, 0.95))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.8, 0.9, 0.8),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn view_error_block(&self, message: &str) -> Element<crate::Message> {
        container(
            row![
                text("❌").size(16),
                text(message).size(14)
            ]
            .spacing(8)
        )
        .padding(8)
        .style(container::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(1.0, 0.95, 0.95))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.9, 0.7, 0.7),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new_command("ls -la".to_string());
        assert!(matches!(block.content, BlockContent::Command { .. }));
        
        let agent_block = Block::new_agent_message("Hello!".to_string());
        assert!(matches!(agent_block.content, BlockContent::AgentMessage { .. }));
    }

    #[test]
    fn test_set_output() {
        let mut block = Block::new_command("echo test".to_string());
        block.set_output("test\n".to_string(), 0);
        
        if let BlockContent::Command { output, exit_code, .. } = block.content {
            assert_eq!(output, Some("test\n".to_string()));
            assert_eq!(exit_code, Some(0));
        } else {
            panic!("Expected command block");
        }
    }
}
