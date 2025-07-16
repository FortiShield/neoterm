use iced::{Element, widget::{text_input, column, row, container, button, text}};
use iced::keyboard::{self, KeyCode, Modifiers};
use iced::{keyboard::Event as KeyEvent, Event as IcedEvent};
use std::collections::{VecDeque, HashMap};
use crossterm::event::{self, Event as CrosstermEvent, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent, KeyModifiers}; // Renamed KeyCode and KeyEvent to avoid conflict
use tokio::sync::mpsc;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct EnhancedTextInput {
    value: String,
    suggestions: Vec<Suggestion>,
    active_suggestion: Option<usize>,
    history: VecDeque<String>,
    history_index: Option<usize>,
    live_preview: String, // New field for live preview
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub description: Option<String>,
    pub suggestion_type: SuggestionType,
    pub score: f32,
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
pub enum Message {
    InputChanged(String),
    Submit,
    SuggestionSelected(usize),
    NavigateSuggestions(Direction),
    ApplySuggestion,
    HistoryNavigated(HistoryDirection),
    CommandPaletteToggle,
    AISidebarToggle,
    RunBenchmark,
    KeyInput(CrosstermKeyEvent), // Use CrosstermKeyEvent
    Resize(u16, u16),
    MouseInput(event::MouseEvent),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keybinding {
    Single(KeyCode, Modifiers),
    Chord(KeyCode, Modifiers, KeyCode, Modifiers), // e.g., Ctrl+K, Ctrl+L
}

pub struct InputHandler {
    keybindings: HashMap<Keybinding, Message>,
    // State for chord detection if needed
    // last_key_event: Option<KeyEvent>,
}

impl InputHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            keybindings: HashMap::new(),
            // last_key_event: None,
        };
        handler.setup_default_keybindings();
        handler
    }

    fn setup_default_keybindings(&mut self) {
        self.keybindings.insert(
            Keybinding::Single(KeyCode::P, Modifiers::COMMAND), // Cmd+P on macOS, Ctrl+P on others
            Message::CommandPaletteToggle,
        );
        self.keybindings.insert(
            Keybinding::Single(KeyCode::A, Modifiers::COMMAND), // Cmd+A on macOS, Ctrl+A on others
            Message::AISidebarToggle,
        );
        self.keybindings.insert(
            Keybinding::Single(KeyCode::F1, Modifiers::NONE),
            Message::RunBenchmark,
        );
        // Add more default keybindings here
    }

    pub fn process_iced_event(&self, event: &IcedEvent) -> Option<Message> {
        match event {
            IcedEvent::Keyboard(key_event) => self.process_keyboard_event(key_event),
            IcedEvent::Mouse(mouse_event) => Some(Message::MouseInput(mouse_event.clone())),
            IcedEvent::Text(text) => Some(Message::InputChanged(text.clone())),
            _ => None,
        }
    }

    fn process_keyboard_event(&self, event: &KeyEvent) -> Option<Message> {
        match event {
            KeyEvent::KeyPressed { key_code, modifiers, .. } => {
                let current_keybinding = Keybinding::Single(*key_code, *modifiers);
                self.keybindings.get(&current_keybinding).cloned()
            }
            // Handle KeyReleased or other keyboard events if necessary
            _ => None,
        }
    }

    pub fn register_keybinding(&mut self, keybinding: Keybinding, message: Message) {
        self.keybindings.insert(keybinding, message);
    }

    pub fn remove_keybinding(&mut self, keybinding: &Keybinding) {
        self.keybindings.remove(keybinding);
    }
}

impl EnhancedTextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            suggestions: Vec::new(),
            active_suggestion: None,
            history: VecDeque::new(),
            history_index: None,
            live_preview: String::new(), // Initialize new field
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::InputChanged(value) => {
                self.value = value;
                self.update_suggestions();
                // Auto-select the first suggestion and update live preview
                self.active_suggestion = self.suggestions.first().map(|_| 0);
                self.update_live_preview();
            }
            Message::Submit => {
                self.add_to_history(self.value.clone());
                self.value.clear();
                self.suggestions.clear();
                self.active_suggestion = None;
                self.live_preview.clear(); // Clear live preview on submit
            }
            Message::SuggestionSelected(index) => {
                if let Some(suggestion) = self.suggestions.get(index) {
                    self.value = suggestion.text.clone();
                    self.suggestions.clear(); // Clear suggestions after selection
                    self.active_suggestion = None;
                    self.live_preview.clear(); // Clear live preview after selection
                }
            }
            Message::NavigateSuggestions(direction) => {
                if self.suggestions.is_empty() {
                    return;
                }
                let new_index = match self.active_suggestion {
                    Some(i) => match direction {
                        Direction::Up => i.checked_sub(1).unwrap_or(self.suggestions.len() - 1),
                        Direction::Down => (i + 1) % self.suggestions.len(),
                    },
                    None => match direction {
                        Direction::Up => self.suggestions.len() - 1,
                        Direction::Down => 0,
                    },
                };
                self.active_suggestion = Some(new_index);
                self.update_live_preview(); // Update live preview when navigating
            }
            Message::ApplySuggestion => {
                if let Some(index) = self.active_suggestion {
                    if let Some(suggestion) = self.suggestions.get(index) {
                        self.value = suggestion.text.clone();
                        self.suggestions.clear();
                        self.active_suggestion = None;
                        self.live_preview.clear(); // Clear live preview after applying
                    }
                }
            }
            Message::HistoryNavigated(direction) => {
                if let Some(cmd) = self.navigate_history(direction) {
                    self.value = cmd;
                    self.suggestions.clear(); // Clear suggestions when navigating history
                    self.active_suggestion = None;
                    self.live_preview.clear(); // Clear live preview when navigating history
                }
            }
            Message::CommandPaletteToggle => { /* Handled by parent */ }
            Message::AISidebarToggle => { /* Handled by parent */ }
            Message::RunBenchmark => { /* Handled by parent */ }
            Message::KeyInput(_) => { /* Handled by parent */ }
            Message::Resize(_, _) => { /* Handled by parent */ }
            Message::MouseInput(_) => { /* Handled by parent */ }
            Message::Error(_) => { /* Handled by parent */ }
        }
    }

    pub fn value(&self) -> &str {
        &self.value
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

    fn navigate_history(&mut self, direction: HistoryDirection) -> Option<String> {
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

    fn update_suggestions(&mut self) {
        let mut suggestions = Vec::new();
        let current_input = self.value.trim();

        if current_input.is_empty() {
            // Suggest common commands if input is empty
            suggestions.extend(self.get_command_suggestions(""));
        } else {
            // Get the last word for more targeted suggestions
            let last_word = current_input.split_whitespace().last().unwrap_or("");

            // Command suggestions (only if it's the first word or empty)
            if current_input.split_whitespace().count() <= 1 {
                suggestions.extend(self.get_command_suggestions(last_word));
            }
            
            // History suggestions (always relevant)
            suggestions.extend(self.get_history_suggestions(current_input));

            // File/directory suggestions (placeholder for now)
            // In a real implementation, you'd scan the filesystem based on the current path
            // suggestions.extend(self.get_file_suggestions(last_word));
        }

        // Sort by score (higher is better)
        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(10); // Limit to top 10 suggestions

        self.suggestions = suggestions;
    }

    fn update_live_preview(&mut self) {
        self.live_preview = if let Some(index) = self.active_suggestion {
            self.suggestions.get(index).map(|s| s.text.clone()).unwrap_or_default()
        } else {
            String::new()
        };
    }

    fn get_command_suggestions(&self, prefix: &str) -> Vec<Suggestion> {
        let common_commands = [
            "ls", "cd", "pwd", "mkdir", "rmdir", "rm", "cp", "mv", "cat", "less", "more",
            "grep", "find", "which", "whereis", "man", "info", "help", "history",
            "ps", "top", "htop", "kill", "killall", "jobs", "bg", "fg", "nohup",
            "git", "npm", "yarn", "cargo", "docker", "kubectl", "ssh", "scp", "rsync",
            "echo", "ping", "curl", "wget", "chmod", "chown", "df", "du", "tar", "zip", "unzip",
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

    fn get_file_suggestions(&self, _prefix: &str) -> Vec<Suggestion> {
        // In a real implementation, you'd scan the filesystem
        // For now, return empty suggestions
        Vec::new()
    }

    fn get_history_suggestions(&self, prefix: &str) -> Vec<Suggestion> {
        self.history
            .iter()
            .filter(|cmd| cmd.contains(prefix) && cmd != &self.value) // Don't suggest current input
            .take(5) // Limit history suggestions
            .map(|cmd| Suggestion {
                text: cmd.clone(),
                description: Some("From history".to_string()),
                suggestion_type: SuggestionType::History,
                score: self.calculate_fuzzy_score(cmd, prefix) * 0.9, // Slightly lower score for history
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
            "echo" => "Display a line of text".to_string(),
            "ping" => "Send ICMP ECHO_REQUEST packets to network hosts".to_string(),
            "curl" => "Transfer data from or to a server".to_string(),
            _ => format!("Execute {}", command),
        }
    }

    fn calculate_fuzzy_score(&self, text: &str, query: &str) -> f32 {
        if query.is_empty() {
            return 0.0;
        }
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        if text_lower.starts_with(&query_lower) {
            1.0 // Exact prefix match is highest score
        } else if text_lower.contains(&query_lower) {
            0.7 // Contains query
        } else {
            // Simple subsequence matching with position bonus
            let mut score = 0.0;
            let mut query_chars = query_lower.chars().peekable();
            
            for (i, ch) in text_lower.chars().enumerate() {
                if let Some(&query_ch) = query_chars.peek() {
                    if ch == query_ch {
                        score += 0.1;
                        // Add bonus for earlier matches
                        if i < 5 { score += 0.05; }
                        query_chars.next();
                    }
                }
            }
            
            score
        }
    }

    pub fn view(&self, prompt_indicator: &str, placeholder: &str) -> Element<Message> {
        let current_placeholder = if !self.live_preview.is_empty() && self.value.is_empty() {
            &self.live_preview // Show live preview as placeholder if input is empty
        } else if !self.live_preview.is_empty() && self.live_preview.starts_with(&self.value) {
            // If there's a live preview and it starts with the current value,
            // show the completion part as placeholder.
            // This is a simple way to simulate ghost text.
            &self.live_preview[self.value.len()..]
        }
        else {
            placeholder
        };

        let input = text_input(current_placeholder, &self.value)
            .on_input(Message::InputChanged)
            .on_submit(Message::Submit)
            .padding(12)
            .size(16);

        let input_with_prompt = row![
            text(prompt_indicator).size(16),
            input
        ].spacing(8);

        let suggestions_view = if !self.suggestions.is_empty() {
            let suggestion_elements: Vec<Element<Message>> = self.suggestions
                .iter()
                .enumerate()
                .map(|(i, suggestion)| {
                    let is_active = self.active_suggestion == Some(i);
                    
                    container(
                        row![
                            text(&suggestion.text).size(14),
                            if let Some(desc) = &suggestion.description {
                                text(desc)
                                    .size(12)
                                    .style(|theme| iced::widget::text::Appearance {
                                        color: Some(theme.palette().text.scale_alpha(0.7)),
                                    })
                            } else {
                                text("")
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
                    .on_press(Message::SuggestionSelected(i)) // Click to select
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

        column![input_with_prompt, suggestions_view].spacing(4).into()
    }
}

pub fn init() {
    println!("input module loaded");
}

/// Represents an input message from the terminal.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// A keyboard key was pressed.
    Key(CrosstermKeyEvent),
    /// The terminal was resized.
    Resize(u16, u16),
    /// A mouse event occurred.
    Mouse(event::MouseEvent),
    /// No event occurred within the timeout.
    Tick,
    /// An error occurred while reading input.
    Error(String),
}

/// Manages reading input events from the terminal.
pub struct InputManager {
    event_sender: mpsc::Sender<InputEvent>,
    event_receiver: mpsc::Receiver<InputEvent>,
}

impl InputManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100); // Buffer for input events
        Self {
            event_sender: tx,
            event_receiver: rx,
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Input manager initialized.");
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), event::EnableMouseCapture)?;
        Ok(())
    }

    /// Starts a background task to continuously read input events.
    pub fn start_event_loop(&self) {
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            loop {
                // Poll for events with a timeout to allow other async tasks to run
                if event::poll(std::time::Duration::from_millis(50)).unwrap() {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key_event)) => {
                            if sender.send(InputEvent::Key(key_event)).await.is_err() {
                                log::warn!("Input event receiver dropped, stopping event loop.");
                                break;
                            }
                        },
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if sender.send(InputEvent::Resize(w, h)).await.is_err() {
                                log::warn!("Input event receiver dropped, stopping event loop.");
                                break;
                            }
                        },
                        Ok(CrosstermEvent::Mouse(mouse_event)) => {
                            if sender.send(InputEvent::Mouse(mouse_event)).await.is_err() {
                                log::warn!("Input event receiver dropped, stopping event loop.");
                                break;
                            }
                        },
                        Ok(_) => { /* Ignore other event types for now */ },
                        Err(e) => {
                            log::error!("Error reading crossterm event: {:?}", e);
                            if sender.send(InputEvent::Error(e.to_string())).await.is_err() {
                                log::warn!("Input event receiver dropped, stopping event loop.");
                                break;
                            }
                        }
                    }
                } else {
                    // Send a tick event if no input was received
                    if sender.send(InputEvent::Tick).await.is_err() {
                        log::warn!("Input event receiver dropped, stopping event loop.");
                        break;
                    }
                }
            }
        });
    }

    /// Receives the next input event.
    pub async fn next_event(&mut self) -> Option<InputEvent> {
        self.event_receiver.recv().await
    }

    pub async fn shutdown(&self) -> Result<()> {
        log::info!("Shutting down input manager.");
        crossterm::execute!(std::io::stdout(), event::DisableMouseCapture)?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }
}
