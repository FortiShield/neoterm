// This file is kept for historical reference of the Ratatui implementation
// and is no longer actively used by the main application.
// This module is specifically for Ratatui (TUI) blocks, not the Iced GUI blocks.
// It was part of the previous Ratatui-focused implementation.
// For now, it's a placeholder to avoid breaking existing references if any.

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
pub enum BlockType {
    Command,
    Output,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct CollapsibleBlock {
    pub id: String,
    pub title: Line<'static>,
    pub content: VecDeque<Line<'static>>,
    pub block_type: BlockType,
    pub is_collapsed: bool,
    pub scroll_offset: u16,
    pub max_scroll_offset: u16,
    pub is_selected: bool,
}

impl CollapsibleBlock {
    pub fn new(title: Line<'static>, block_type: BlockType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            content: VecDeque::new(),
            block_type,
            is_collapsed: false,
            scroll_offset: 0,
            max_scroll_offset: 0,
            is_selected: false,
        }
    }

    pub fn add_line(&mut self, line: Line<'static>) {
        self.content.push_back(line);
        // Keep content buffer limited to avoid excessive memory usage
        if self.content.len() > 1000 {
            self.content.pop_front();
        }
        self.update_max_scroll_offset();
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
        self.scroll_offset = 0; // Reset scroll when collapsing/expanding
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1).min(self.max_scroll_offset);
    }

    fn update_max_scroll_offset(&mut self) {
        // This is a rough estimate; actual lines rendered depend on wrap
        self.max_scroll_offset = self.content.len().saturating_sub(1) as u16;
    }

    pub fn render<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let border_color = if self.is_selected {
            Color::LightCyan
        } else {
            match self.block_type {
                BlockType::Command => Color::Blue,
                BlockType::Output => Color::Green,
                BlockType::Error => Color::Red,
                BlockType::Info => Color::Yellow,
            }
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(self.title.clone());

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        if self.is_collapsed {
            let collapsed_text = Paragraph::new(Span::styled(
                format!("... {} lines ...", self.content.len()),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
            f.render_widget(collapsed_text, inner_area);
        } else {
            let content_to_display: Vec<Line> = self.content.iter().skip(self.scroll_offset as usize).cloned().collect();
            let paragraph = Paragraph::new(content_to_display)
                .wrap(Wrap { trim: true })
                .scroll((self.scroll_offset, 0)); // Apply scroll offset
            f.render_widget(paragraph, inner_area);
        }
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

    pub fn add_block(&mut self, mut block: CollapsibleBlock) {
        block.is_selected = false; // Ensure new block is not selected by default
        self.blocks.push(block);
        self.selected_index = self.blocks.len().saturating_sub(1); // Select the newest block
        self.update_selection();
    }

    pub fn toggle_selected_block(&mut self) {
        if let Some(block) = self.blocks.get_mut(self.selected_index) {
            block.toggle_collapse();
        }
    }

    pub fn move_selection_up(&mut self) {
        if !self.blocks.is_empty() {
            self.selected_index = self.selected_index.saturating_sub(1);
            self.update_selection();
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.blocks.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.blocks.len().saturating_sub(1));
            self.update_selection();
        }
    }

    fn update_selection(&mut self) {
        for (i, block) in self.blocks.iter_mut().enumerate() {
            block.is_selected = (i == self.selected_index);
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if self.blocks.is_empty() {
            return;
        }

        let constraints: Vec<Constraint> = self.blocks.iter().map(|block| {
            if block.is_collapsed {
                Constraint::Length(3) // Collapsed height (title + borders)
            } else {
                Constraint::Min(5) // Minimum height for expanded blocks
            }
        }).collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, block) in self.blocks.iter().enumerate() {
            if let Some(chunk) = chunks.get(i) {
                block.render(f, *chunk);
            }
        }
    }
}
