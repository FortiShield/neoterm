use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::fuzzy_match;
use iced::{Element, widget::{column, text, container, text_input}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: CommandCategory,
    pub keybinding: Option<String>,
    pub action: CommandAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandCategory {
    Navigation,
    Editing,
    Terminal,
    AI,
    Workflow,
    Settings,
    System,
}

#[derive(Debug, Clone)]
pub enum CommandAction {
    ToggleCollapse,
    NewTab,
    CloseTab,
    SearchBlocks,
    ClearHistory,
    AIChat,
    AIExplain,
    OpenSettings,
    ShowHelp,
    RunWorkflow(String),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub is_open: bool,
    pub input_buffer: String,
    pub commands: Vec<Command>,
    pub filtered_commands: Vec<(Command, f64)>, // (command, score)
    pub selected_index: usize,
    pub scroll_offset: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut palette = Self {
            is_open: false,
            input_buffer: String::new(),
            commands: Vec::new(),
            filtered_commands: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        };
        
        palette.load_default_commands();
        palette.update_filtered_commands();
        palette
    }

    fn load_default_commands(&mut self) {
        let default_commands = vec![
            Command {
                id: "toggle_collapse".to_string(),
                name: "Toggle Block Collapse".to_string(),
                description: "Collapse or expand the selected block".to_string(),
                category: CommandCategory::Navigation,
                keybinding: Some("Space".to_string()),
                action: CommandAction::ToggleCollapse,
            },
            Command {
                id: "new_tab".to_string(),
                name: "New Tab".to_string(),
                description: "Open a new terminal tab".to_string(),
                category: CommandCategory::Terminal,
                keybinding: Some("Ctrl+T".to_string()),
                action: CommandAction::NewTab,
            },
            Command {
                id: "close_tab".to_string(),
                name: "Close Tab".to_string(),
                description: "Close the current tab".to_string(),
                category: CommandCategory::Terminal,
                keybinding: Some("Ctrl+W".to_string()),
                action: CommandAction::CloseTab,
            },
            Command {
                id: "search_blocks".to_string(),
                name: "Search Blocks".to_string(),
                description: "Search through command blocks".to_string(),
                category: CommandCategory::Navigation,
                keybinding: Some("Ctrl+F".to_string()),
                action: CommandAction::SearchBlocks,
            },
            Command {
                id: "clear_history".to_string(),
                name: "Clear History".to_string(),
                description: "Clear all command history".to_string(),
                category: CommandCategory::Terminal,
                keybinding: None,
                action: CommandAction::ClearHistory,
            },
            Command {
                id: "ai_chat".to_string(),
                name: "AI Chat".to_string(),
                description: "Open AI assistant sidebar".to_string(),
                category: CommandCategory::AI,
                keybinding: Some("Ctrl+A".to_string()),
                action: CommandAction::AIChat,
            },
            Command {
                id: "ai_explain".to_string(),
                name: "AI Explain Output".to_string(),
                description: "Ask AI to explain the selected output".to_string(),
                category: CommandCategory::AI,
                keybinding: Some("Ctrl+E".to_string()),
                action: CommandAction::AIExplain,
            },
            Command {
                id: "open_settings".to_string(),
                name: "Open Settings".to_string(),
                description: "Open application settings".to_string(),
                category: CommandCategory::Settings,
                keybinding: Some("Ctrl+,".to_string()),
                action: CommandAction::OpenSettings,
            },
            Command {
                id: "show_help".to_string(),
                name: "Show Help".to_string(),
                description: "Display help and keybindings".to_string(),
                category: CommandCategory::System,
                keybinding: Some("F1".to_string()),
                action: CommandAction::ShowHelp,
            },
        ];

        self.commands = default_commands;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if !self.is_open {
            self.input_buffer.clear(); // Clear input when closing
        }
        if self.is_open {
            self.input_buffer.clear();
            self.selected_index = 0;
            self.scroll_offset = 0;
            self.update_filtered_commands();
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.input_buffer.push(c);
        self.update_filtered_commands();
    }

    pub fn remove_char(&mut self) {
        self.input_buffer.pop();
        self.update_filtered_commands();
    }

    fn update_filtered_commands(&mut self) {
        if self.input_buffer.is_empty() {
            self.filtered_commands = self.commands
                .iter()
                .map(|cmd| (cmd.clone(), 1.0))
                .collect();
        } else {
            let mut scored_commands: Vec<(Command, f64)> = self.commands
                .iter()
                .filter_map(|cmd| {
                    let name_score = fuzzy_match::fuzzy_match(&self.input_buffer, &cmd.name);
                    let desc_score = fuzzy_match::fuzzy_match(&self.input_buffer, &cmd.description);
                    
                    let best_score = name_score.max(desc_score);
                    if best_score > 0.3 {
                        Some((cmd.clone(), best_score))
                    } else {
                        None
                    }
                })
                .collect();

            scored_commands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            self.filtered_commands = scored_commands;
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
        if self.selected_index < self.filtered_commands.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn get_selected_command(&self) -> Option<&Command> {
        self.filtered_commands.get(self.selected_index).map(|(cmd, _)| cmd)
    }

    pub fn execute_selected(&mut self) -> Option<CommandAction> {
        if let Some(command) = self.get_selected_command() {
            let action = command.action.clone();
            self.is_open = false;
            Some(action)
        } else {
            None
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.is_open {
            return;
        }

        // Create centered popup
        let popup_area = self.centered_rect(60, 70, area);
        
        // Clear the area
        f.render_widget(Clear, popup_area);

        // Split into input and list areas
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(popup_area);

        // Render input box
        let input = Paragraph::new(self.input_buffer.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Palette")
                    .border_style(Style::default().fg(Color::Cyan))
            );
        f.render_widget(input, chunks[0]);

        // Render command list
        let visible_height = chunks[1].height as usize;
        
        // Adjust scroll offset
        if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index.saturating_sub(visible_height - 1);
        } else if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }

        let items: Vec<ListItem> = self.filtered_commands
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_height)
            .map(|(i, (cmd, score))| {
                let is_selected = i + self.scroll_offset == self.selected_index;
                let style = if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };

                let keybinding = cmd.keybinding
                    .as_ref()
                    .map(|k| format!(" ({})", k))
                    .unwrap_or_default();

                let category_color = match cmd.category {
                    CommandCategory::Navigation => Color::Green,
                    CommandCategory::Editing => Color::Yellow,
                    CommandCategory::Terminal => Color::Cyan,
                    CommandCategory::AI => Color::Magenta,
                    CommandCategory::Workflow => Color::Blue,
                    CommandCategory::Settings => Color::Gray,
                    CommandCategory::System => Color::Red,
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled(format!("{}{}", cmd.name, keybinding), style),
                    ]),
                    Line::from(vec![
                        Span::styled("  ", style),
                        Span::styled(cmd.description.clone(), style.fg(Color::Gray)),
                        Span::styled(format!(" [{:?}]", cmd.category), style.fg(category_color)),
                    ]),
                ];

                ListItem::new(lines).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Commands ({}/{})", 
                        self.filtered_commands.len().min(1), 
                        self.filtered_commands.len()))
                    .border_style(Style::default().fg(Color::Cyan))
            );

        f.render_widget(list, chunks[1]);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    pub fn add_custom_command(&mut self, command: Command) {
        self.commands.push(command);
        self.update_filtered_commands();
    }

    pub fn remove_command(&mut self, id: &str) {
        self.commands.retain(|cmd| cmd.id != id);
        self.update_filtered_commands();
    }

    pub fn view(&self) -> Element<crate::Message> {
        if !self.is_open {
            return column![].into();
        }

        container(
            column![
                text("Command Palette (Iced Placeholder)").size(20),
                text_input("Type command...", &self.input_buffer)
                    .on_input(|s| {
                        // This would typically be handled by a specific message for the palette
                        // For now, just update the internal buffer
                        crate::Message::InputChanged(s) // This is a hack, needs proper message routing
                    })
                    .padding(8),
                text("Suggestions...").size(14),
            ]
            .spacing(10)
            .padding(10)
        )
        .width(iced::Length::FillPortion(1))
        .height(iced::Length::FillPortion(1))
        .style(iced::widget::container::Appearance {
            background: Some(iced::Color::from_rgb(0.1, 0.1, 0.1).into()),
            border_radius: 8.0.into(),
            border_width: 2.0,
            border_color: iced::Color::from_rgb(0.5, 0.5, 0.5),
            ..Default::default()
        })
        .center_x()
        .center_y()
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_palette_creation() {
        let palette = CommandPalette::new();
        assert!(!palette.is_open);
        assert!(!palette.commands.is_empty());
        assert!(!palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_command_filtering() {
        let mut palette = CommandPalette::new();
        palette.add_char('t');
        palette.add_char('o');
        palette.add_char('g');
        palette.add_char('g');
        palette.add_char('l');
        palette.add_char('e');
        
        assert!(palette.filtered_commands.iter()
            .any(|(cmd, _)| cmd.name.to_lowercase().contains("toggle")));
    }

    #[test]
    fn test_command_selection() {
        let mut palette = CommandPalette::new();
        palette.toggle();
        
        let initial_selection = palette.selected_index;
        palette.move_selection_down();
        assert!(palette.selected_index > initial_selection || palette.filtered_commands.len() <= 1);
    }
}
