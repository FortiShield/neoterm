use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span}, // Import Line and Span
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Command,
    Output,
    Error,
    Info,
    AgentMessage,
    UserMessage,
}

#[derive(Debug, Clone)]
pub struct CollapsibleBlock {
    pub id: String,
    pub title: Line<'static>, // Changed to Line
    pub content: Vec<Line<'static>>, // Changed to Vec<Line>
    pub block_type: BlockType,
    pub collapsed: bool,
    pub scroll_offset: u16,
}

impl CollapsibleBlock {
    pub fn new(title: Line<'static>, block_type: BlockType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            content: Vec::new(),
            block_type,
            collapsed: false,
            scroll_offset: 0,
        }
    }

    pub fn add_line(&mut self, line: Line<'static>) { // Accepts Line directly
        self.content.push(line);
        // Adjust scroll offset to keep new content in view if not collapsed
        if !self.collapsed {
            self.scroll_offset = self.content.len().saturating_sub(1) as u16;
        }
    }

    pub fn toggle_collapse(&mut self) {
        self.collapsed = !self.collapsed;
        if !self.collapsed {
            // Reset scroll offset when uncollapsed
            self.scroll_offset = self.content.len().saturating_sub(1) as u16;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, max_lines: u16) {
        self.scroll_offset = (self.scroll_offset + 1).min(self.content.len().saturating_sub(1).max(0) as u16);
        // Ensure scroll_offset doesn't go beyond the visible area if content is small
        if self.content.len() as u16 <= max_lines {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.scroll_offset.min(self.content.len().saturating_sub(max_lines).max(0) as u16);
        }
    }

    pub fn get_border_color(&self) -> Color {
        match self.block_type {
            BlockType::Command => Color::Blue,
            BlockType::Output => Color::Green,
            BlockType::Error => Color::Red,
            BlockType::Info => Color::Cyan,
            BlockType::AgentMessage => Color::Magenta,
            BlockType::UserMessage => Color::Yellow,
        }
    }
}

pub struct CollapsibleBlockRenderer {
    pub blocks: Vec<CollapsibleBlock>,
    pub selected_index: usize,
}

impl CollapsibleBlockRenderer {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn add_block(&mut self, block: CollapsibleBlock) {
        self.blocks.push(block);
        self.selected_index = self.blocks.len().saturating_sub(1); // Select the newly added block
    }

    pub fn toggle_selected_block(&mut self) {
        if let Some(block) = self.blocks.get_mut(self.selected_index) {
            block.toggle_collapse();
        }
    }

    pub fn move_selection_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn move_selection_down(&mut self) {
        self.selected_index = (self.selected_index + 1).min(self.blocks.len().saturating_sub(1));
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let mut constraints = Vec::new();
        let mut visible_block_indices = Vec::new();

        // Calculate constraints for visible blocks
        for (i, block) in self.blocks.iter().enumerate() {
            if block.collapsed {
                constraints.push(Constraint::Length(3)); // Title + borders
            } else {
                // Estimate content height + title + borders
                // This is a rough estimate, actual height depends on wrap
                let content_height = block.content.len() as u16;
                constraints.push(Constraint::Length(content_height.saturating_add(3).min(area.height)));
            }
            visible_block_indices.push(i);
        }

        if constraints.is_empty() {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .spacing(1)
            .split(area);

        for (i, chunk) in chunks.into_iter().enumerate() {
            if let Some(block_data) = self.blocks.get_mut(visible_block_indices[i]) {
                let is_selected = self.selected_index == visible_block_indices[i];
                let border_color = block_data.get_border_color();

                let block_widget = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(block_data.title.clone()) // Use Line as title
                    .border_type(if is_selected {
                        ratatui::widgets::BorderType::Thick
                    } else {
                        ratatui::widgets::BorderType::Plain
                    });

                let inner_area = block_widget.inner(chunk);
                f.render_widget(block_widget, chunk);

                if !block_data.collapsed {
                    let content_to_display = if block_data.content.len() as u16 > inner_area.height {
                        let start_index = block_data.scroll_offset as usize;
                        let end_index = (start_index + inner_area.height as usize).min(block_data.content.len());
                        block_data.content[start_index..end_index].to_vec()
                    } else {
                        block_data.content.clone()
                    };

                    let paragraph = Paragraph::new(content_to_display)
                        .wrap(Wrap { trim: true });
                    f.render_widget(paragraph, inner_area);
                }
            }
        }
    }
}
