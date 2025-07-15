use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Command,
    Output,
    Info,
    Error,
    AgentMessage,
    UserMessage,
}

#[derive(Debug, Clone)]
pub struct CollapsibleBlock {
    pub id: String,
    pub title: String,
    pub content: Vec<String>,
    pub is_collapsed: bool,
    pub block_type: BlockType,
}

impl CollapsibleBlock {
    pub fn new(title: String, block_type: BlockType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            content: Vec::new(),
            is_collapsed: false,
            block_type,
        }
    }

    pub fn add_line(&mut self, line: String) {
        self.content.push(line);
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn render<B: Backend>(&self, f: &mut Frame<B>, area: Rect, is_selected: bool) {
        let border_color = if is_selected {
            Color::Cyan
        } else {
            match self.block_type {
                BlockType::Command => Color::Blue,
                BlockType::Output => Color::Green,
                BlockType::Info => Color::LightCyan,
                BlockType::Error => Color::Red,
                BlockType::AgentMessage => Color::Magenta,
                BlockType::UserMessage => Color::Yellow,
            }
        };

        let title_style = Style::default().fg(border_color).add_modifier(Modifier::BOLD);
        let content_style = Style::default().fg(Color::White);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_color)
            .title(Span::styled(self.title.clone(), title_style));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        if !self.is_collapsed {
            let lines: Vec<ListItem> = self.content.iter()
                .map(|line| ListItem::new(Span::styled(line.clone(), content_style)))
                .collect();

            let list = List::new(lines)
                .block(Block::default()) // No borders for inner list
                .style(content_style);

            f.render_widget(list, inner_area);
        } else {
            let collapsed_text = Paragraph::new(Span::styled("... (collapsed) ...", content_style))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(collapsed_text, inner_area);
        }
    }
}

pub struct CollapsibleBlockRenderer {
    pub blocks: Vec<CollapsibleBlock>,
    pub selected_index: usize,
    pub scroll_offset: usize, // For scrolling the list of blocks
}

impl CollapsibleBlockRenderer {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    pub fn add_block(&mut self, block: CollapsibleBlock) {
        self.blocks.push(block);
        self.selected_index = self.blocks.len().saturating_sub(1); // Select the new block
        self.scroll_offset = self.selected_index; // Adjust scroll to show new block
    }

    pub fn toggle_selected_block(&mut self) {
        if let Some(block) = self.blocks.get_mut(self.selected_index) {
            block.toggle_collapse();
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.adjust_scroll_offset();
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.blocks.len().saturating_sub(1) {
            self.selected_index += 1;
            self.adjust_scroll_offset();
        }
    }

    fn adjust_scroll_offset(&mut self) {
        // Ensure selected_index is within the visible scroll area
        // This is a simplified adjustment. A more robust solution would consider
        // the height of each block. For now, assuming uniform height or just
        // keeping the selected index in view.
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.get_visible_height() {
            self.scroll_offset = self.selected_index - self.get_visible_height() + 1;
        }
    }

    fn get_visible_height(&self) -> usize {
        // Placeholder: In a real TUI, this would depend on the actual rendering area height
        // and average block height. For now, a fixed number.
        10 // Assume 10 blocks are visible at a time
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if self.blocks.is_empty() {
            let no_blocks_text = Paragraph::new(Span::styled(
                "No blocks yet. Type a command or press 'p' for command palette.",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(no_blocks_text, area);
            return;
        }

        let mut current_y = area.y;
        let max_height = area.height;

        // Calculate total height needed for all blocks
        let mut block_heights: Vec<u16> = Vec::new();
        for block in &self.blocks {
            let title_height = 1; // For the title bar
            let content_height = if block.is_collapsed {
                1 // For "(collapsed)" text
            } else {
                block.content.len() as u16
            };
            block_heights.push(title_height + content_height + 2); // +2 for top/bottom borders
        }

        // Adjust scroll_offset to keep selected block in view
        let mut visible_start_index = self.scroll_offset;
        let mut visible_end_index = self.scroll_offset;
        let mut current_visible_height = 0;

        for (i, &height) in block_heights.iter().enumerate().skip(self.scroll_offset) {
            if current_visible_height + height <= max_height {
                current_visible_height += height;
                visible_end_index = i;
            } else {
                break;
            }
        }

        // If selected block is below visible area, scroll down
        if self.selected_index > visible_end_index {
            let mut new_scroll_offset = self.selected_index;
            let mut temp_height = 0;
            while new_scroll_offset > 0 && temp_height + block_heights[new_scroll_offset] <= max_height {
                temp_height += block_heights[new_scroll_offset];
                new_scroll_offset -= 1;
            }
            self.scroll_offset = new_scroll_offset.saturating_add(1); // Adjust back one if it overshot
        }
        // If selected block is above visible area, scroll up
        else if self.selected_index < visible_start_index {
            self.scroll_offset = self.selected_index;
        }


        current_y = area.y;
        for (i, block) in self.blocks.iter().enumerate().skip(self.scroll_offset) {
            let block_height = block_heights[i];
            if current_y + block_height > area.y + max_height {
                break; // No more space
            }

            let block_area = Rect::new(area.x, current_y, area.width, block_height);
            block.render(f, block_area, i == self.selected_index);
            current_y += block_height;
        }
    }
}
