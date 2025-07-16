use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tui_textarea::{TextArea, Input, Key};
use crate::fuzzy_match::{FuzzyMatchManager, FuzzyMatchResult};
use std::collections::HashMap;

pub struct CommandPalette {
    input_area: TextArea<'static>,
    is_open: bool,
    commands: HashMap<String, String>, // command_id -> command_description
    filtered_commands: Vec<FuzzyMatchResult>,
    selected_index: usize,
    fuzzy_matcher: FuzzyMatchManager,
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut commands = HashMap::new();
        commands.insert("app.quit".to_string(), "Quit NeoTerm".to_string());
        commands.insert("ui.next_block".to_string(), "Switch to next UI block".to_string());
        commands.insert("app.run_benchmarks".to_string(), "Run performance benchmarks".to_string());
        commands.insert("ai.chat".to_string(), "Send message to AI assistant".to_string());
        commands.insert("config.edit".to_string(), "Edit preferences".to_string());
        commands.insert("theme.edit".to_string(), "Edit current theme".to_string());
        commands.insert("workflow.run".to_string(), "Run a workflow".to_string());
        commands.insert("plugin.list".to_string(), "List installed plugins".to_string());

        Self {
            input_area: TextArea::default(),
            is_open: false,
            commands,
            filtered_commands: Vec::new(),
            selected_index: 0,
            fuzzy_matcher: FuzzyMatchManager::new(),
        }
    }

    pub fn init(&self) {
        log::info!("Command palette initialized.");
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.input_area.set_cursor_line_style(Style::default());
        self.input_area.set_cursor_style(Style::default().bg(Color::White).fg(Color::Black));
        self.update_filtered_commands();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.input_area.set_lines(vec!["".to_string()]);
        self.selected_index = 0;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn handle_input(&mut self, input: Input) -> Option<String> {
        match input {
            Input { key: Key::Esc, .. } => {
                self.close();
                None
            },
            Input { key: Key::Enter, .. } => {
                if !self.filtered_commands.is_empty() {
                    let selected_command_id = self.filtered_commands[self.selected_index].text.clone();
                    self.close();
                    Some(selected_command_id)
                } else {
                    None
                }
            },
            Input { key: Key::Up, .. } => {
                if !self.filtered_commands.is_empty() {
                    self.selected_index = self.selected_index.saturating_sub(1);
                }
                None
            },
            Input { key: Key::Down, .. } => {
                if !self.filtered_commands.is_empty() {
                    self.selected_index = (self.selected_index + 1).min(self.filtered_commands.len() - 1);
                }
                None
            },
            _ => {
                self.input_area.input(input);
                self.update_filtered_commands();
                None
            }
        }
    }

    fn update_filtered_commands(&mut self) {
        let query = self.input_area.lines().join("").to_lowercase();
        let candidate_ids: Vec<String> = self.commands.keys().cloned().collect();
        
        self.filtered_commands = self.fuzzy_matcher.fuzzy_match(&query, &candidate_ids);
        self.selected_index = 0; // Reset selection on filter change
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }

        let popup_area = CommandPalette::centered_rect(60, 40, area); // 60% width, 40% height

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input area
                Constraint::Min(0),    // Results area
            ])
            .split(popup_area);

        // Input area
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Command Palette", Style::default().fg(Color::LightYellow)));
        
        let mut input_widget = self.input_area.widget();
        input_widget = input_widget.block(input_block);
        frame.render_widget(input_widget, chunks[0]);

        // Results area
        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Results", Style::default().fg(Color::LightGreen)));

        let mut result_lines: Vec<Line> = Vec::new();
        for (i, result) in self.filtered_commands.iter().enumerate() {
            let command_id = &result.text;
            let description = self.commands.get(command_id).unwrap_or(&command_id);
            let line_content = format!("{}: {}", command_id, description);
            
            let mut spans = Vec::new();
            let mut last_idx = 0;
            for &match_idx in &result.indices {
                if match_idx >= line_content.len() { continue; }
                spans.push(Span::raw(&line_content[last_idx..match_idx]));
                spans.push(Span::styled(
                    line_content.chars().nth(match_idx).unwrap().to_string(),
                    Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
                ));
                last_idx = match_idx + line_content.chars().nth(match_idx).unwrap().len_utf8();
            }
            spans.push(Span::raw(&line_content[last_idx..]));

            let mut line = Line::from(spans);
            if i == self.selected_index {
                line = line.style(Style::default().bg(Color::DarkGray));
            }
            result_lines.push(line);
        }

        let results_paragraph = Paragraph::new(result_lines)
            .block(results_block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(results_paragraph, chunks[1]);
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
}
