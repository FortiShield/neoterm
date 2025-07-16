use iced::{
    widget::{column, container, row, text, button, scrollable},
    Element, Length, Color, alignment,
};
use uuid::Uuid;
use chrono::{DateTime, Local};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color as TuiColor, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block as TuiBlock, Borders, Paragraph};
use ratatui::Frame;

/// Represents a generic UI block in the terminal.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Terminal,
    Editor,
    Info,
    Welcome,
    BenchmarkResults,
    // Add more block types as needed
}

/// State for a UI block.
#[derive(Debug, Clone)]
pub struct BlockState {
    pub block_type: BlockType,
    pub title: String,
    pub content: Vec<Line<'static>>,
    pub is_active: bool,
    pub area: Rect,
}

impl BlockState {
    pub fn new(block_type: BlockType, title: String, content: Vec<Line<'static>>) -> Self {
        Self {
            block_type,
            title,
            content,
            is_active: false,
            area: Rect::default(),
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    pub fn render(&self, frame: &mut Frame) {
        let border_style = if self.is_active {
            Style::default().fg(TuiColor::Cyan)
        } else {
            Style::default().fg(TuiColor::White)
        };

        let block = TuiBlock::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(self.title.clone(), Style::default().fg(TuiColor::LightGreen)));

        let paragraph = Paragraph::new(self.content.clone())
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: false });

        frame.render_widget(paragraph, self.area);
    }
}

/// Manages multiple UI blocks.
#[derive(Debug, Clone)]
pub struct BlockManager {
    pub blocks: Vec<BlockState>,
    pub active_block_index: usize,
}

impl BlockManager {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            active_block_index: 0,
        }
    }

    pub fn add_block(&mut self, block: BlockState) {
        self.blocks.push(block);
        // If this is the first block, make it active
        if self.blocks.len() == 1 {
            self.blocks[0].set_active(true);
        }
    }

    pub fn get_active_block_mut(&mut self) -> Option<&mut BlockState> {
        self.blocks.get_mut(self.active_block_index)
    }

    pub fn get_active_block(&self) -> Option<&BlockState> {
        self.blocks.get(self.active_block_index)
    }

    pub fn next_block(&mut self) {
        if !self.blocks.is_empty() {
            self.blocks[self.active_block_index].set_active(false);
            self.active_block_index = (self.active_block_index + 1) % self.blocks.len();
            self.blocks[self.active_block_index].set_active(true);
        }
    }

    pub fn previous_block(&mut self) {
        if !self.blocks.is_empty() {
            self.blocks[self.active_block_index].set_active(false);
            self.active_block_index = (self.active_block_index + self.blocks.len() - 1) % self.blocks.len();
            self.blocks[self.active_block_index].set_active(true);
        }
    }

    pub fn update_layout(&mut self, area: Rect) {
        if self.blocks.is_empty() {
            return;
        }

        let constraints: Vec<Constraint> = self.blocks.iter()
            .map(|_| Constraint::Percentage(100 / self.blocks.len() as u16))
            .collect();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, block_area) in layout.iter().enumerate() {
            if let Some(block) = self.blocks.get_mut(i) {
                block.set_area(*block_area);
            }
        }
    }

    pub fn set_block_content(&mut self, block_type: BlockType, content: Vec<Line<'static>>) {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.block_type == block_type) {
            block.content = content;
        } else {
            log::warn!("Block of type {:?} not found to update content.", block_type);
        }
    }

    pub fn get_block_content(&self, block_type: BlockType) -> Option<&Vec<Line<'static>>> {
        self.blocks.iter().find(|b| b.block_type == block_type).map(|b| &b.content)
    }
}

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
    pub status: Option<String>, // For streaming updates
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
            status: Some("Running...".to_string()),
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
            status: None, // Status will be set during streaming
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
            status: None,
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
            status: None,
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
            status: Some("Error".to_string()),
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
        match &mut self.content {
            BlockContent::Command { status: s, end_time, .. } => {
                *s = status.clone();
                *end_time = Some(Local::now());
            },
            BlockContent::AgentMessage { .. } | BlockContent::Info { .. } | BlockContent::Error { .. } => {
                // For other block types, update the general status field
            }
        }
        self.status = Some(status);
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
            .on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::ToggleCollapse))
            .style(iced::widget::button::text::Style::Text);

        let mut actions_row = row![
            toggle_button,
            id_text,
            button(text("📋")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::Copy)).style(iced::widget::button::text::Style::Text),
            button(text("🔄")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::Rerun)).style(iced::widget::button::text::Style::Text),
            button(text("🗑️")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::Delete)).style(iced::widget::button::text::Style::Text),
            button(text("📤")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::Export)).style(iced::widget::button::text::Style::Text),
            button(text("🤖")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::SendToAI)).style(iced::widget::button::text::Style::Text),
        ];

        // Conditionally show "Fix" button for failed command blocks
        if let BlockContent::Command { error: true, .. } = self.content {
            actions_row = actions_row.push(
                button(text("🛠️ Fix")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::SuggestFix)).style(iced::widget::button::text::Style::Text)
            );
        }

        // Conditionally show "Explain Output" button for command and error blocks
        match self.content {
            BlockContent::Command { .. } | BlockContent::Error { .. } => {
                actions_row = actions_row.push(
                    button(text("❓ Explain")).on_press(crate::Message::BlockAction(self.id.clone(), crate::main::BlockMessage::ExplainOutput)).style(iced::widget::button::text::Style::Text)
                );
            }
            _ => {}
        }

        let header = actions_row.spacing(5).align_items(alignment::Horizontal::Center);

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

pub fn init() {
    log::info!("Block module initialized.");
}
