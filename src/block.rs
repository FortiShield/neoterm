use iced::{Element, Length, widget::{column, row, text, container, button, scrollable}};
use iced::alignment::{Horizontal, Vertical};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: String,
    pub content: BlockContent,
    pub is_collapsed: bool,
    pub is_error: bool,
    scroll_offset: scrollable::State,
}

#[derive(Debug, Clone)]
pub enum BlockContent {
    Command {
        input: String,
        output_stdout: String,
        output_stderr: String,
        status: String,
    },
    Info {
        title: String,
        message: String,
    },
    AgentMessage {
        content: String,
    },
    UserMessage {
        content: String,
    },
    Output {
        output_stdout: String,
        output_stderr: String,
        status: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub enum BlockMessage {
    Copy,
    Rerun,
    Delete,
    Export,
    ToggleCollapse,
}

impl Block {
    pub fn new_command(input: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Command {
                input,
                output_stdout: String::new(),
                output_stderr: String::new(),
                status: "Running...".to_string(),
            },
            is_collapsed: false,
            is_error: false,
            scroll_offset: scrollable::State::new(),
        }
    }

    pub fn new_info(title: String, message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Info { title, message },
            is_collapsed: false,
            is_error: false,
            scroll_offset: scrollable::State::new(),
        }
    }

    pub fn new_output(initial_status: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Output {
                output_stdout: String::new(),
                output_stderr: String::new(),
                status: initial_status,
            },
            is_collapsed: false,
            is_error: false,
            scroll_offset: scrollable::State::new(),
        }
    }

    pub fn new_error(message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::Error { message },
            is_collapsed: false,
            is_error: true,
            scroll_offset: scrollable::State::new(),
        }
    }

    pub fn new_agent_message(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::AgentMessage { content },
            is_collapsed: false,
            is_error: false,
            scroll_offset: scrollable::State::new(),
        }
    }

    pub fn new_user_message(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: BlockContent::UserMessage { content },
            is_collapsed: false,
            is_error: false,
            scroll_offset: scrollable::State::new(),
        }
    }

    pub fn add_output_line(&mut self, line: String, is_stdout: bool) {
        match &mut self.content {
            BlockContent::Command { output_stdout, output_stderr, .. } => {
                if is_stdout {
                    output_stdout.push_str(&line);
                } else {
                    output_stderr.push_str(&line);
                }
            }
            BlockContent::Output { output_stdout, output_stderr, .. } => {
                if is_stdout {
                    output_stdout.push_str(&line);
                } else {
                    output_stderr.push_str(&line);
                }
            }
            _ => {}
        }
    }

    pub fn set_status(&mut self, status: String) {
        match &mut self.content {
            BlockContent::Command { status: s, .. } => *s = status,
            BlockContent::Output { status: s, .. } => *s = status,
            _ => {}
        }
    }

    pub fn set_error(&mut self, is_error: bool) {
        self.is_error = is_error;
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn view(&self) -> Element<BlockMessage> {
        let header = match &self.content {
            BlockContent::Command { input, status, .. } => {
                row![
                    text(input).size(16).width(Length::Fill),
                    text(status).size(14).style(|theme| {
                        if self.is_error {
                            iced::widget::text::Appearance { color: Some(theme.palette().danger) }
                        } else if status.contains("Running") {
                            iced::widget::text::Appearance { color: Some(theme.palette().primary) }
                        } else {
                            iced::widget::text::Appearance { color: Some(theme.palette().text) }
                        }
                    }),
                    button(text(if self.is_collapsed { "▶" } else { "▼" }))
                        .on_press(BlockMessage::ToggleCollapse)
                ]
            }
            BlockContent::Info { title, .. } => {
                row![
                    text(title).size(16).width(Length::Fill),
                    button(text(if self.is_collapsed { "▶" } else { "▼" }))
                        .on_press(BlockMessage::ToggleCollapse)
                ]
            }
            BlockContent::AgentMessage { .. } => {
                row![
                    text("🤖 Agent Response").size(16).width(Length::Fill),
                    button(text(if self.is_collapsed { "▶" } else { "▼" }))
                        .on_press(BlockMessage::ToggleCollapse)
                ]
            }
            BlockContent::UserMessage { .. } => {
                row![
                    text("👤 User Input").size(16).width(Length::Fill),
                    button(text(if self.is_collapsed { "▶" } else { "▼" }))
                        .on_press(BlockMessage::ToggleCollapse)
                ]
            }
            BlockContent::Output { status, .. } => {
                row![
                    text("Output").size(16).width(Length::Fill),
                    text(status).size(14).style(|theme| {
                        if self.is_error {
                            iced::widget::text::Appearance { color: Some(theme.palette().danger) }
                        } else {
                            iced::widget::text::Appearance { color: Some(theme.palette().text) }
                        }
                    }),
                    button(text(if self.is_collapsed { "▶" } else { "▼" }))
                        .on_press(BlockMessage::ToggleCollapse)
                ]
            }
            BlockContent::Error { .. } => {
                row![
                    text("Error").size(16).width(Length::Fill),
                    button(text(if self.is_collapsed { "▶" } else { "▼" }))
                        .on_press(BlockMessage::ToggleCollapse)
                ]
            }
        }
        .align_items(Vertical::Center)
        .spacing(8);

        let content_view = if !self.is_collapsed {
            match &self.content {
                BlockContent::Command { output_stdout, output_stderr, .. } => {
                    let output_text = format!("{}{}", output_stdout, output_stderr);
                    scrollable(
                        &mut self.scroll_offset,
                        text(output_text).size(14).width(Length::Fill)
                    )
                    .height(Length::Units(100))
                    .into()
                }
                BlockContent::Info { message, .. } => {
                    text(message).size(14).width(Length::Fill).into()
                }
                BlockContent::AgentMessage { content } => {
                    text(content).size(14).width(Length::Fill).into()
                }
                BlockContent::UserMessage { content } => {
                    text(content).size(14).width(Length::Fill).into()
                }
                BlockContent::Output { output_stdout, output_stderr, .. } => {
                    let output_text = format!("{}{}", output_stdout, output_stderr);
                    scrollable(
                        &mut self.scroll_offset,
                        text(output_text).size(14).width(Length::Fill)
                    )
                    .height(Length::Units(100))
                    .into()
                }
                BlockContent::Error { message } => {
                    text(message).size(14).style(|theme| iced::widget::text::Appearance { color: Some(theme.palette().danger) }).width(Length::Fill).into()
                }
            }
        } else {
            column![].into()
        };

        column![
            header,
            content_view
        ]
        .padding(10)
        .spacing(5)
        .style(|theme| container::Appearance {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                color: if self.is_error {
                    theme.palette().danger
                } else {
                    theme.palette().text.scale_alpha(0.1)
                },
                width: 1.0,
                radius: 5.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}
