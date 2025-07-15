use iced::{
    widget::{column, container, row, scrollable, text, Button, Space},
    Alignment, Element, Length, Color,
};
use uuid::Uuid;
use crate::main::BlockMessage; // Import BlockMessage from main

#[derive(Debug, Clone)]
pub enum BlockType {
    Command,
    Output,
    Error,
    Info,
    AgentMessage,
    UserMessage,
}

#[derive(Debug, Clone)]
pub struct CommandBlock {
    pub id: String,
    pub block_type: BlockType,
    pub command_input: Option<String>,
    pub output_lines: Vec<String>,
    pub status_message: Option<String>,
    pub is_collapsed: bool,
    pub is_error: bool,
}

impl CommandBlock {
    pub fn new_command(input: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            block_type: BlockType::Command,
            command_input: Some(input),
            output_lines: Vec::new(),
            status_message: None,
            is_collapsed: false,
            is_error: false,
        }
    }

    pub fn new_output(initial_output: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            block_type: BlockType::Output,
            command_input: None,
            output_lines: vec![initial_output],
            status_message: None,
            is_collapsed: false,
            is_error: false,
        }
    }

    pub fn new_error(message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            block_type: BlockType::Error,
            command_input: None,
            output_lines: vec![message],
            status_message: Some("Error".to_string()),
            is_collapsed: false,
            is_error: true,
        }
    }

    pub fn new_info(title: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            block_type: BlockType::Info,
            command_input: Some(title),
            output_lines: content.lines().map(|s| s.to_string()).collect(),
            status_message: None,
            is_collapsed: false,
            is_error: false,
        }
    }

    pub fn new_agent_message(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            block_type: BlockType::AgentMessage,
            command_input: Some("Agent Response".to_string()),
            output_lines: vec![content],
            status_message: None,
            is_collapsed: false,
            is_error: false,
        }
    }

    pub fn new_user_message(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            block_type: BlockType::UserMessage,
            command_input: Some("User Input".to_string()),
            output_lines: vec![content],
            status_message: None,
            is_collapsed: false,
            is_error: false,
        }
    }

    pub fn add_output_line(&mut self, line: String, _is_stdout: bool) {
        self.output_lines.push(line);
    }

    pub fn set_status(&mut self, status: String) {
        self.status_message = Some(status);
    }

    pub fn set_error(&mut self, is_error: bool) {
        self.is_error = is_error;
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn view(&self) -> Element<BlockMessage> {
        let header_color = match self.block_type {
            BlockType::Command => Color::from_rgb(0.2, 0.6, 0.8), // Cyan-ish
            BlockType::Output => Color::from_rgb(0.2, 0.8, 0.6), // Green-ish
            BlockType::Error => Color::from_rgb(0.8, 0.2, 0.2), // Red
            BlockType::Info => Color::from_rgb(0.8, 0.8, 0.2), // Yellow-ish
            BlockType::AgentMessage => Color::from_rgb(0.6, 0.2, 0.8), // Magenta-ish
            BlockType::UserMessage => Color::from_rgb(0.8, 0.6, 0.2), // Orange-ish
        };

        let header_text = match self.block_type {
            BlockType::Command => self.command_input.as_deref().unwrap_or("Command"),
            BlockType::Output => self.status_message.as_deref().unwrap_or("Output"),
            BlockType::Error => self.status_message.as_deref().unwrap_or("Error"),
            BlockType::Info => self.command_input.as_deref().unwrap_or("Info"),
            BlockType::AgentMessage => self.command_input.as_deref().unwrap_or("Agent Response"),
            BlockType::UserMessage => self.command_input.as_deref().unwrap_or("User Input"),
        };

        let collapse_icon = if self.is_collapsed { "▶" } else { "▼" };

        let header = row![
            Button::new(text(collapse_icon))
                .on_press(BlockMessage::ToggleCollapse)
                .style(iced::widget::button::text()),
            text(header_text)
                .size(18)
                .color(header_color)
                .width(Length::Fill),
            text(self.status_message.as_deref().unwrap_or(""))
                .size(14)
                .color(if self.is_error { Color::from_rgb(1.0, 0.0, 0.0) } else { Color::from_rgb(0.7, 0.7, 0.7) }),
            Button::new(text("Rerun"))
                .on_press(BlockMessage::Rerun)
                .style(iced::widget::button::text()),
            Button::new(text("Copy"))
                .on_press(BlockMessage::Copy)
                .style(iced::widget::button::text()),
            Button::new(text("Delete"))
                .on_press(BlockMessage::Delete)
                .style(iced::widget::button::text()),
        ]
        .align_items(Alignment::Center)
        .spacing(10);

        let content = if self.is_collapsed {
            column![text(format!("... {} lines ...", self.output_lines.len())).size(14).color(Color::from_rgb(0.5, 0.5, 0.5))]
                .align_items(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Units(30))
                .into()
        } else {
            scrollable(
                column(
                    self.output_lines
                        .iter()
                        .map(|line| text(line).size(14).into())
                        .collect()
                )
                .spacing(2)
            )
            .height(Length::Shrink)
            .into()
        };

        container(
            column![
                header,
                Space::with_height(Length::Units(5)),
                content
            ]
            .spacing(5)
        )
        .padding(10)
        .width(Length::Fill)
        .style(move |theme| container::Appearance {
            background: Some(Color::from_rgb(0.1, 0.1, 0.1).into()),
            border_radius: 5.0.into(),
            border_width: 1.0,
            border_color: header_color,
            ..Default::default()
        })
        .into()
    }
}
