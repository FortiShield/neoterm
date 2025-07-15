use iced::{Element, widget::{text_input, column, row, container}};
use std::collections::VecDeque;

use crate::Message;

#[derive(Debug, Clone)]
pub struct EnhancedTextInput {
    value: String,
    suggestions: Vec<Suggestion>,
    active_suggestion: Option<usize>,
    history: VecDeque<String>,
    history_index: Option<usize>,
    syntax_tree: Option<SyntaxTree>,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    text: String,
    description: Option<String>,
    suggestion_type: SuggestionType,
    score: f32,
}

#[derive(Debug, Clone)]
pub enum SuggestionType {
    Command,
    File,
    Directory,
    Flag,
    History,
    Alias,
}

#[derive(Debug, Clone)]
pub struct SyntaxTree {
    tokens: Vec<Token>,
    errors: Vec<SyntaxError>,
}

#[derive(Debug, Clone)]
pub struct Token {
    text: String,
    token_type: TokenType,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
pub enum TokenType {
    Command,
    Argument,
    Flag,
    String,
    Number,
    Operator,
    Pipe,
    Redirect,
    Variable,
    Comment,
}

#[derive(Debug, Clone)]
pub struct SyntaxError {
    message: String,
    position: usize,
    length: usize,
}

impl EnhancedTextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            suggestions: Vec::new(),
            active_suggestion: None,
            history: VecDeque::new(),
            history_index: None,
            syntax_tree: None,
        }
    }

    pub fn update_value(&mut self, value: String) {
        self.value = value;
        self.update_syntax_tree();
        self.update_suggestions();
    }

    pub fn add_to_history(&mut self, command: String) {
        if !command.trim().is_empty() && self.history.front() != Some(&command) {
            self.history.push_front(command);
            if self.history.len() > 1000 {
                self.history.pop_back();
            }
        }
        self.history_index = None;
    }

    pub fn navigate_history(&mut self, direction: HistoryDirection) -> Option<String> {
        match direction {
            HistoryDirection::Up => {
                let new_index = match self.history_index {
                    None => Some(0),
                    Some(i) if i < self.history.len() - 1 => Some(i + 1),
                    Some(i) => Some(i),
                };
                
                if let Some(index) = new_index {
                    self.history_index = new_index;
                    self.history.get(index).cloned()
                } else {
                    None
                }
            }
            HistoryDirection::Down => {
                match self.history_index {
                    Some(0) => {
                        self.history_index = None;
                        Some(String::new())
                    }
                    Some(i) => {
                        self.history_index = Some(i - 1);
                        self.history.get(i - 1).cloned()
                    }
                    None => None,
                }
            }
        }
    }

    fn update_syntax_tree(&mut self) {
        self.syntax_tree = Some(self.parse_command(&self.value));
    }

    fn parse_command(&self, input: &str) -> SyntaxTree {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut current_pos = 0;

        // Simple tokenizer - in a real implementation, you'd use a proper parser
        let words: Vec<&str> = input.split_whitespace().collect();
        
        for (i, word) in words.iter().enumerate() {
            let token_type = if i == 0 {
                TokenType::Command
            } else if word.starts_with('-') {
                TokenType::Flag
            } else if word.starts_with('$') {
                TokenType::Variable
            } else if word.contains('|') {
                TokenType::Pipe
            } else if word.contains('>') || word.contains('<') {
                TokenType::Redirect
            } else if word.starts_with('"') || word.starts_with('\'') {
                TokenType::String
            } else if word.parse::<f64>().is_ok() {
                TokenType::Number
            } else {
                TokenType::Argument
            };

            tokens.push(Token {
                text: word.to_string(),
                token_type,
                start: current_pos,
                end: current_pos + word.len(),
            });

            current_pos += word.len() + 1; // +1 for space
        }

        SyntaxTree { tokens, errors }
    }

    fn update_suggestions(&mut self) {
        let mut suggestions = Vec::new();
        
        // Generate suggestions based on current input
        if let Some(last_word) = self.value.split_whitespace().last() {
            // Command suggestions
            if self.value.split_whitespace().count() <= 1 {
                suggestions.extend(self.get_command_suggestions(last_word));
            }
            
            // File/directory suggestions
            suggestions.extend(self.get_file_suggestions(last_word));
            
            // History suggestions
            suggestions.extend(self.get_history_suggestions(last_word));
        }

        // Sort by score
        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(10);

        self.suggestions = suggestions;
    }

    fn get_command_suggestions(&self, prefix: &str) -> Vec<Suggestion> {
        let common_commands = [
            "ls", "cd", "pwd", "mkdir", "rmdir", "rm", "cp", "mv", "cat", "less", "more",
            "grep", "find", "which", "whereis", "man", "info", "help", "history",
            "ps", "top", "htop", "kill", "killall", "jobs", "bg", "fg", "nohup",
            "git", "npm", "yarn", "cargo", "docker", "kubectl", "ssh", "scp", "rsync",
        ];

        common_commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| Suggestion {
                text: cmd.to_string(),
                description: Some(self.get_command_description(cmd)),
                suggestion_type: SuggestionType::Command,
                score: self.calculate_fuzzy_score(cmd, prefix),
            })
            .collect()
    }

    fn get_file_suggestions(&self, prefix: &str) -> Vec<Suggestion> {
        // In a real implementation, you'd scan the filesystem
        // For now, return empty suggestions
        Vec::new()
    }

    fn get_history_suggestions(&self, prefix: &str) -> Vec<Suggestion> {
        self.history
            .iter()
            .filter(|cmd| cmd.contains(prefix))
            .take(5)
            .map(|cmd| Suggestion {
                text: cmd.clone(),
                description: Some("From history".to_string()),
                suggestion_type: SuggestionType::History,
                score: self.calculate_fuzzy_score(cmd, prefix),
            })
            .collect()
    }

    fn get_command_description(&self, command: &str) -> String {
        match command {
            "ls" => "List directory contents".to_string(),
            "cd" => "Change directory".to_string(),
            "pwd" => "Print working directory".to_string(),
            "git" => "Git version control".to_string(),
            "npm" => "Node package manager".to_string(),
            "cargo" => "Rust package manager".to_string(),
            "docker" => "Container management".to_string(),
            _ => format!("Execute {}", command),
        }
    }

    fn calculate_fuzzy_score(&self, text: &str, query: &str) -> f32 {
        if text.starts_with(query) {
            1.0
        } else if text.contains(query) {
            0.7
        } else {
            // Simple fuzzy matching - in a real implementation, use a proper fuzzy matching library
            let mut score = 0.0;
            let mut query_chars = query.chars().peekable();
            
            for ch in text.chars() {
                if let Some(&query_ch) = query_chars.peek() {
                    if ch.to_lowercase().eq(query_ch.to_lowercase()) {
                        score += 0.1;
                        query_chars.next();
                    }
                }
            }
            
            score
        }
    }

    pub fn view(&self) -> Element<Message> {
        let input = text_input("Enter command...", &self.value)
            .on_input(Message::InputChanged)
            .on_submit(Message::ExecuteCommand)
            .padding(12)
            .size(16);

        let suggestions_view = if !self.suggestions.is_empty() {
            let suggestion_elements: Vec<Element<Message>> = self.suggestions
                .iter()
                .enumerate()
                .map(|(i, suggestion)| {
                    let is_active = self.active_suggestion == Some(i);
                    
                    container(
                        row![
                            iced::widget::text(&suggestion.text).size(14),
                            if let Some(desc) = &suggestion.description {
                                iced::widget::text(desc)
                                    .size(12)
                                    .style(|theme| iced::widget::text::Appearance {
                                        color: Some(theme.palette().text.scale_alpha(0.7)),
                                    })
                            } else {
                                iced::widget::text("")
                            }
                        ]
                        .spacing(8)
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
                    .into()
                })
                .collect();

            container(column(suggestion_elements).spacing(2))
                .padding(4)
                .style(|theme| container::Appearance {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        color: theme.palette().text.scale_alpha(0.2),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .into()
        } else {
            column![].into()
        };

        column![input, suggestions_view].spacing(4).into()
    }
}

#[derive(Debug, Clone)]
pub enum HistoryDirection {
    Up,
    Down,
}
