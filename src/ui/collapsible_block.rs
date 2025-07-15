use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapsibleBlock {
    pub id: String,
    pub title: String,
    pub content: Vec<String>,
    pub is_collapsed: bool,
    pub block_type: BlockType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // Added PartialEq for comparison
pub enum BlockType {
    Command,
    Output,
    Error,
    Info,
    Warning,
}

impl CollapsibleBlock {
    pub fn new(title: String, block_type: BlockType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content: Vec::new(),
            is_collapsed: false,
            block_type,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_line(&mut self, line: String) {
        self.content.push(line);
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn get_display_lines(&self) -> Vec<String> {
        if self.is_collapsed {
            if self.content.is_empty() {
                vec!["(empty)".to_string()]
            } else {
                vec![format!("... {} lines", self.content.len())]
            }
        } else {
            self.content.clone()
        }
    }

    pub fn get_style(&self) -> Style {
        match self.block_type {
            BlockType::Command => Style::default().fg(Color::Cyan),
            BlockType::Output => Style::default().fg(Color::White),
            BlockType::Error => Style::default().fg(Color::Red),
            BlockType::Info => Style::default().fg(Color::Blue),
            BlockType::Warning => Style::default().fg(Color::Yellow),
        }
    }

    pub fn get_border_style(&self) -> Style {
        let base_style = match self.block_type {
            BlockType::Command => Style::default().fg(Color::Cyan),
            BlockType::Output => Style::default().fg(Color::Gray),
            BlockType::Error => Style::default().fg(Color::Red),
            BlockType::Info => Style::default().fg(Color::Blue),
            BlockType::Warning => Style::default().fg(Color::Yellow),
        };

        if self.is_collapsed {
            base_style.add_modifier(Modifier::DIM)
        } else {
            base_style
        }
    }

    pub fn get_title_with_indicator(&self) -> String {
        let indicator = if self.is_collapsed { "▶" } else { "▼" };
        let timestamp = self.timestamp.format("%H:%M:%S");
        format!("{} {} [{}]", indicator, self.title, timestamp)
    }
}

pub struct CollapsibleBlockRenderer {
    pub blocks: Vec<CollapsibleBlock>,
    pub selected_index: usize,
    pub scroll_offset: usize,
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
        self.selected_index = self.blocks.len().saturating_sub(1);
        // Ensure scroll_offset keeps the new block in view if it's the last one
        // This logic might need refinement based on actual viewport height
        // For now, simply set scroll_offset to show the last block
        self.scroll_offset = self.blocks.len().saturating_sub(1);
    }

    pub fn toggle_selected_block(&mut self) {
        if let Some(block) = self.blocks.get_mut(self.selected_index) {
            block.toggle_collapse();
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.blocks.len().saturating_sub(1) {
            self.selected_index += 1;
            // Adjust scroll_offset to keep selected item in view
            // This needs to consider the actual height of the rendered blocks
            // For simplicity, let's assume a fixed block height for now or adjust based on visible area
            // A more robust solution would calculate visible lines/blocks
            let visible_height = 10; // Placeholder, should be derived from actual render area
            if self.selected_index >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected_index - visible_height + 1;
            }
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let mut list_items: Vec<ListItem> = Vec::new();
        let mut current_height = 0;
        let mut block_start_indices = Vec::new(); // Store (block_index, start_line_in_list_items)

        for (i, block) in self.blocks.iter().enumerate() {
            block_start_indices.push((i, list_items.len()));

            let is_selected = i == self.selected_index;
            let style = if is_selected {
                block.get_style().add_modifier(Modifier::REVERSED)
            } else {
                block.get_style()
            };

            let title_line = Line::from(vec![
                Span::styled(block.get_title_with_indicator(), style)
            ]);
            list_items.push(ListItem::new(title_line));
            current_height += 1;
            
            if !block.is_collapsed {
                for content_line in &block.content {
                    list_items.push(ListItem::new(Line::from(vec![
                        Span::styled(format!("  {}", content_line), block.get_style())
                    ])));
                    current_height += 1;
                }
            }
        }

        // Calculate the actual scroll offset based on selected_index and visible area
        let mut effective_scroll_offset = 0;
        if let Some((_, start_line)) = block_start_indices.iter().find(|(idx, _)| *idx == self.selected_index) {
            let visible_lines_in_area = area.height as usize;
            if *start_line >= effective_scroll_offset + visible_lines_in_area {
                effective_scroll_offset = start_line.saturating_sub(visible_lines_in_area - 1);
            } else if *start_line < effective_scroll_offset {
                effective_scroll_offset = *start_line;
            }
        }
        self.scroll_offset = effective_scroll_offset;


        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Blocks ({}/{})", self.selected_index + 1, self.blocks.len()))
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut ratatui::widgets::ListState::new(Some(self.scroll_offset)));
    }

    pub fn collapse_all(&mut self) {
        for block in &mut self.blocks {
            block.is_collapsed = true;
        }
    }

    pub fn expand_all(&mut self) {
        for block in &mut self.blocks {
            block.is_collapsed = false;
        }
    }

    pub fn filter_by_type(&self, block_type: BlockType) -> Vec<&CollapsibleBlock> {
        self.blocks.iter()
            .filter(|block| std::mem::discriminant(&block.block_type) == std::mem::discriminant(&block_type))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| {
                block.title.to_lowercase().contains(&query.to_lowercase()) ||
                block.content.iter().any(|line| line.to_lowercase().contains(&query.to_lowercase()))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapsible_block_creation() {
        let mut block = CollapsibleBlock::new("Test Block".to_string(), BlockType::Command);
        assert!(!block.is_collapsed);
        assert_eq!(block.title, "Test Block");
        
        block.add_line("Line 1".to_string());
        block.add_line("Line 2".to_string());
        assert_eq!(block.content.len(), 2);
        
        block.toggle_collapse();
        assert!(block.is_collapsed);
        
        let display_lines = block.get_display_lines();
        assert_eq!(display_lines, vec!["... 2 lines"]);
    }

    #[test]
    fn test_block_renderer() {
        let mut renderer = CollapsibleBlockRenderer::new();
        
        let block1 = CollapsibleBlock::new("Block 1".to_string(), BlockType::Command);
        let block2 = CollapsibleBlock::new("Block 2".to_string(), BlockType::Output);
        
        renderer.add_block(block1);
        renderer.add_block(block2);
        
        assert_eq!(renderer.blocks.len(), 2);
        assert_eq!(renderer.selected_index, 1);
        
        renderer.move_selection_up();
        assert_eq!(renderer.selected_index, 0);
    }
}
