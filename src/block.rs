use iced::{
    widget::{column, container, row, text, button, scrollable},
    Element, Length, Color, alignment,
};
use uuid::Uuid;
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub enum BlockContent {
    Command {
        input: String,
        output: Vec<(String, bool)>, // (content, is_stdout)
        status: String,
        error: bool,
        start_time: DateTime<Local>,
        end_time: Option<DateTime<Local>>,
    },
    AgentMessage {
        content: String,
        is_user: bool,
        timestamp: DateTime<Local>,
    },
    Info {
        title: String,
        message: String,
        timestamp: DateTime<Local>,
    },
    Error {
        message: String,
        timestamp: DateTime<Local>,
    },
    // Add other block types as needed (e.g., Code, Image, Workflow)
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: String,
    pub content: BlockContent,
    pub collapsed: bool,
}

impl Block {
    pub fn new_command(input: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Command {
                input,
                output: Vec::new(),
                status: "Running...".to_string(),
                error: false,
                start_time: Local::now(),
                end_time: None,
            },
            collapsed: false,
        }
    }

    pub fn new_agent_message(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::AgentMessage {
                content,
                is_user: false,
                timestamp: Local::now(),
            },
            collapsed: false,
        }
    }

    pub fn new_user_message(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::AgentMessage {
                content,
                is_user: true,
                timestamp: Local::now(),
            },
            collapsed: false,
        }
    }

    pub fn new_info(title: String, message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Info {
                title,
                message,
                timestamp: Local::now(),
            },
            collapsed: false,
        }
    }

    pub fn new_error(message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Error {
                message,
                timestamp: Local::now(),
            },
            collapsed: false,
        }
    }

    pub fn new_output(initial_output: String) -> Self {
        let mut block = Self::new_command("".to_string()); // Use command block for output
        if let BlockContent::Command { output, .. } = &mut block.content {
            output.push((initial_output, true));
        }
        block
    }

    pub fn add_output_line(&mut self, line: String, is_stdout: bool) {
        if let BlockContent::Command { output, .. } = &mut self.content {
            output.push((line, is_stdout));
        }
    }

    pub fn set_status(&mut self, status: String) {
        if let BlockContent::Command { status: s, end_time, .. } = &mut self.content {
            *s = status;
            *end_time = Some(Local::now());
        }
    }

    pub fn set_error(&mut self, error: bool) {
        if let BlockContent::Command { error: e, .. } = &mut self.content {
            *e = error;
        }
    }

    pub fn toggle_collapse(&mut self) {
        self.collapsed = !self.collapsed;
    }

    pub fn view(&self) -> Element<crate::Message> {
        let id_text = text(format!("#{}", &self.id[0..8])).size(12).color(Color::from_rgb(0.5, 0.5, 0.5));
        let toggle_button = button(text(if self.collapsed { "▶" } else { "▼" }))
            .on_press(crate::Message::BlockAction(self.id.clone(), crate::block::BlockMessage::ToggleCollapse))
            .style(iced::widget::button::text::Style::Text);

        let header = row![
            toggle_button,
            id_text,
            // Add other block actions here (copy, rerun, delete, export)
            button(text("📋")).on_press(crate::Message::BlockAction(self.id.clone(), crate::block::BlockMessage::Copy)).style(iced::widget::button::text::Style::Text),
            button(text("🔄")).on_press(crate::Message::BlockAction(self.id.clone(), crate::block::BlockMessage::Rerun)).style(iced::widget::button::text::Style::Text),
            button(text("🗑️")).on_press(crate::Message::BlockAction(self.id.clone(), crate::block::BlockMessage::Delete)).style(iced::widget::button::text::Style::Text),
            button(text("📤")).on_press(crate::Message::BlockAction(self.id.clone(), crate::block::BlockMessage::Export)).style(iced::widget::button::text::Style::Text),
        ]
        .spacing(5)
        .align_items(alignment::Horizontal::Center);

        let content_view: Element<crate::Message> = if self.collapsed {
            match &self.content {
                BlockContent::Command { input, status, error, .. } => {
                    row![
                        text(input).size(16).color(Color::BLACK),
                        text(format!("Status: {}", status)).size(14).color(if *error { Color::from_rgb(1.0, 0.0, 0.0) } else { Color::from_rgb(0.0, 0.5, 0.0) }),
                    ].spacing(10).into()
                }
                BlockContent::AgentMessage { content, is_user, .. } => {
                    row![
                        text(if *is_user { "You:" } else { "Agent:" }).size(14).color(Color::from_rgb(0.2, 0.2, 0.8)),
                        text(content.lines().next().unwrap_or("...")).size(16),
                    ].spacing(10).into()
                }
                BlockContent::Info { title, .. } => {
                    row![
                        text(format!("Info: {}", title)).size(16).color(Color::from_rgb(0.0, 0.5, 0.8)),
                    ].spacing(10).into()
                }
                BlockContent::Error { message, .. } => {
                    row![
                        text(format!("Error: {}", message.lines().next().unwrap_or("..."))).size(16).color(Color::from_rgb(1.0, 0.0, 0.0)),
                    ].spacing(10).into()
                }
            }
        } else {
            match &self.content {
                BlockContent::Command { input, output, status, error, start_time, end_time } => {
                    let output_text = output.iter().map(|(line, is_stdout)| {
                        text(line).size(14).color(if *is_stdout { Color::BLACK } else { Color::from_rgb(0.8, 0.0, 0.0) })
                    }).fold(column![], |col, txt| col.push(txt));

                    let duration = end_time.map(|e| e - *start_time).map(|d| format!("Duration: {}ms", d.num_milliseconds())).unwrap_or_default();

                    column![
                        text(input).size(16).color(Color::from_rgb(0.2, 0.2, 0.8)),
                        scrollable(output_text).height(Length::Shrink).width(Length::Fill),
                        row![
                            text(format!("Status: {}", status)).size(14).color(if *error { Color::from_rgb(1.0, 0.0, 0.0) } else { Color::from_rgb(0.0, 0.5, 0.0) }),
                            text(duration).size(14).color(Color::from_rgb(0.5, 0.5, 0.5)),
                        ].spacing(10)
                    ].spacing(5).into()
                }
                BlockContent::AgentMessage { content, is_user, timestamp } => {
                    column![
                        text(if *is_user { "You:" } else { "Agent:" }).size(14).color(Color::from_rgb(0.2, 0.2, 0.8)),
                        text(content).size(16),
                        text(timestamp.format("%H:%M:%S").to_string()).size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
                    ].spacing(5).into()
                }
                BlockContent::Info { title, message, timestamp } => {
                    column![
                        text(title).size(18).color(Color::from_rgb(0.0, 0.5, 0.8)),
                        text(message).size(16),
                        text(timestamp.format("%H:%M:%S").to_string()).size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
                    ].spacing(5).into()
                }
                BlockContent::Error { message, timestamp } => {
                    column![
                        text("Error!").size(18).color(Color::from_rgb(1.0, 0.0, 0.0)),
                        text(message).size(16),
                        text(timestamp.format("%H:%M:%S").to_string()).size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
                    ].spacing(5).into()
                }
            }
        };

        container(
            column![
                header,
                content_view,
            ]
            .spacing(5)
        )
        .padding(10)
        .style(iced::widget::container::Appearance {
            background: Some(iced::Background::Color(Color::WHITE)),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.8, 0.8, 0.8),
            ..Default::default()
        })
        .into()
    }
}
