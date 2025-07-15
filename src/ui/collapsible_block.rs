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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let visible_height = area.height as usize;
        let total_blocks = self.blocks.len();

        // Adjust scroll offset to keep selected item visible
        if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index.saturating_sub(visible_height - 1);
        } else if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }

        let visible_blocks: Vec<ListItem> = self.blocks
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_height)
            .map(|(i, block)| {
                let is_selected = i == self.selected_index;
                let style = if is_selected {
                    block.get_style().add_modifier(Modifier::REVERSED)
                } else {
                    block.get_style()
                };

                let title_line = Line::from(vec![
                    Span::styled(block.get_title_with_indicator(), style)
                ]);

                let mut lines = vec![title_line];
                
                if !block.is_collapsed {
                    for content_line in &block.content {
                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", content_line), block.get_style())
                        ]));
                    }
                }

                ListItem::new(lines)
            })
            .collect();

        let list = List::new(visible_blocks)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Blocks ({}/{})", self.selected_index + 1, total_blocks))
            );

        f.render_widget(list, area);
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
