// This file is kept for historical reference of the Ratatui implementation
// and is no longer used by src/main.rs in GUI mode.

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CollapsibleBlock {
    pub id: String,
    pub title: Line<'static>,
    pub content: VecDeque<Line<'static>>,
    pub is_collapsed: bool,
    pub block_type: BlockType,
    pub scroll_offset: usize, // Current scroll position
    pub max_scroll_offset: usize, // Maximum scrollable lines
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Command,
    Output,
    Info,
    Error,
    AgentMessage,
    UserMessage,
}

impl CollapsibleBlock {
    pub fn new(title: Line<'static>, block_type: BlockType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            content: VecDeque::new(),
            is_collapsed: false,
            block_type,
            scroll_offset: 0,
            max_scroll_offset: 0,
        }
    }

    pub fn add_line(&mut self, line: Line<'static>) {
        self.content.push_back(line);
        self.max_scroll_offset = self.content.len().saturating_sub(1); // Update max scroll
        // Keep scroll at bottom if it was already at bottom
        if self.scroll_offset == self.max_scroll_offset.saturating_sub(1) || self.scroll_offset == 0 {
            self.scroll_offset = self.max_scroll_offset;
        }
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = (self.scroll_offset + 1).min(self.max_scroll_offset);
    }
}

#[derive(Debug, Clone)]
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

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.blocks.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn toggle_selected_block(&mut self) {
        if let Some(block) = self.blocks.get_mut(self.selected_index) {
            block.toggle_collapse();
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let block_heights: Vec<Constraint> = self.blocks.iter().map(|block| {
            if block.is_collapsed {
                Constraint::Length(3) // Header + borders
            } else {
                // Estimate content height + header + borders
                let content_lines = block.content.len() as u16;
                Constraint::Length(content_lines.saturating_add(3).min(area.height / 3)) // Max 1/3 of screen height
            }
        }).collect();

        let total_height: u16 = block_heights.iter().map(|c| match c {
            Constraint::Length(l) => *l,
            _ => 0, // Should not happen with Length constraints
        }).sum();

        // If total height exceeds area, make blocks fill remaining space
        let constraints = if total_height > area.height {
            self.blocks.iter().map(|block| {
                if block.is_collapsed {
                    Constraint::Length(3)
                } else {
                    Constraint::Min(0) // Allow flexible height for non-collapsed blocks
                }
            }).collect()
        } else {
            block_heights
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, block) in self.blocks.iter_mut().enumerate() {
            if let Some(chunk) = chunks.get(i) {
                let is_selected = i == self.selected_index;
                let border_style = if is_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if block.block_type == BlockType::Error {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let inner_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(block.title.clone());

                f.render_widget(inner_block.clone(), *chunk);

                if !block.is_collapsed {
                    let inner_area = inner_block.inner(*chunk);
                    
                    // Calculate visible lines based on scroll offset
                    let start_index = block.scroll_offset;
                    let end_index = (start_index + inner_area.height as usize).min(block.content.len());
                    let visible_content: Vec<Line> = block.content.iter().skip(start_index).take(end_index - start_index).cloned().collect();

                    let paragraph = Paragraph::new(visible_content)
                        .wrap(Wrap { trim: true });
                    f.render_widget(paragraph, inner_area);
                }
            }
        }
    }
}
