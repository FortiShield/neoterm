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
use iced::{Element, widget::{column, text_input, text, button, row}};

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

#[derive(Debug, Clone, PartialEq)]
pub enum CommandAction {
    ToggleAgentMode,
    ToggleSettings,
    RunBenchmarks,
    ClearHistory,
    // Add more commands here
    CustomCommand(String), // For dynamic commands
}

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    InputChanged(String),
    ExecuteSelected,
    Navigate(Direction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub struct CommandPalette {
    is_open: bool,
    input_value: String,
    filtered_commands: Vec<CommandAction>,
    active_selection: Option<usize>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            input_value: String::new(),
            filtered_commands: Self::get_all_commands(),
            active_selection: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<CommandAction> {
        match message {
            Message::Toggle => {
                self.is_open = !self.is_open;
                if self.is_open {
                    self.input_value.clear();
                    self.filter_commands();
                    self.active_selection = None;
                }
                None
            }
            Message::InputChanged(value) => {
                self.input_value = value;
                self.filter_commands();
                self.active_selection = None; // Reset selection on input change
                None
            }
            Message::ExecuteSelected => {
                if let Some(index) = self.active_selection {
                    if let Some(action) = self.filtered_commands.get(index) {
                        self.is_open = false; // Close palette on execution
                        return Some(action.clone());
                    }
                }
                None
            }
            Message::Navigate(direction) => {
                if self.filtered_commands.is_empty() {
                    return None;
                }
                let new_index = match self.active_selection {
                    Some(i) => match direction {
                        Direction::Up => i.checked_sub(1).unwrap_or(self.filtered_commands.len() - 1),
                        Direction::Down => (i + 1) % self.filtered_commands.len(),
                    },
                    None => match direction {
                        Direction::Up => self.filtered_commands.len() - 1,
                        Direction::Down => 0,
                    },
                };
                self.active_selection = Some(new_index);
                None
            }
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        if !self.is_open {
            return column![].into();
        }

        let input = text_input("Search commands...", &self.input_value)
            .on_input(Message::InputChanged)
            .on_submit(Message::ExecuteSelected)
            .padding(10)
            .size(16);

        let command_list: Vec<Element<Message>> = self.filtered_commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let is_active = self.active_selection == Some(i);
                let cmd_text = match cmd {
                    CommandAction::ToggleAgentMode => "Toggle AI Agent".to_string(),
                    CommandAction::ToggleSettings => "Open Settings".to_string(),
                    CommandAction::RunBenchmarks => "Run Performance Benchmarks".to_string(),
                    CommandAction::ClearHistory => "Clear Command History".to_string(),
                    CommandAction::CustomCommand(s) => s.clone(),
                };

                container(
                    text(cmd_text).size(14)
                )
                .padding(8)
                .style(move |theme| {
                    if is_active {
                        container::Appearance {
                            background: Some(theme.palette().primary.scale_alpha(0.1).into()),
                            ..Default::default()
                        }
                    } else {
                        container::Appearance::default()
                    }
                })
                .on_press(Message::ExecuteSelected) // Clicking selects and executes
                .into()
            })
            .collect();

        column![
            input,
            column(command_list).spacing(2)
        ]
        .spacing(5)
        .padding(10)
        .into()
    }

    fn get_all_commands() -> Vec<CommandAction> {
        vec![
            CommandAction::ToggleAgentMode,
            CommandAction::ToggleSettings,
            CommandAction::RunBenchmarks,
            CommandAction::ClearHistory,
            // Add more static commands here
        ]
    }

    fn filter_commands(&mut self) {
        let query = self.input_value.to_lowercase();
        self.filtered_commands = Self::get_all_commands()
            .into_iter()
            .filter(|cmd| {
                let cmd_name = match cmd {
                    CommandAction::ToggleAgentMode => "toggle ai agent",
                    CommandAction::ToggleSettings => "open settings",
                    CommandAction::RunBenchmarks => "run performance benchmarks",
                    CommandAction::ClearHistory => "clear command history",
                    CommandAction::CustomCommand(s) => s.to_lowercase().as_str(),
                };
                cmd_name.contains(&query)
            })
            .collect();
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }
}
