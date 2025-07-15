use iced::{executor, Application, Command, Element, Settings, Theme};
use iced::widget::{column, container, scrollable, text_input, button, row, text};
use std::path::PathBuf;
use tokio::sync::mpsc;
use uuid::Uuid;

mod block;
mod shell;
mod input;
mod renderer;
mod agent_mode_eval;
mod config;
mod settings;
mod syntax_tree;
mod string_offset;
mod websocket;
mod watcher;
mod virtual_fs;
mod integration;
mod resources;
mod sum_tree;
mod workflows;
mod lpc;
mod mcq;
mod markdown_parser;
mod serve_wasm;
mod languages;
mod natural_language_detection;
mod graphql;
mod command;
mod drive;
mod fuzzy_match;
mod asset_macro;

use block::{Block, BlockContent};
use shell::ShellManager;
use input::EnhancedTextInput;
use agent_mode_eval::{AgentMode, AgentConfig, AgentMessage};
use config::AppConfig;

#[derive(Debug, Clone)]
pub struct NeoTerm {
    blocks: Vec<Block>,
    current_input: String,
    input_history: Vec<String>,
    history_index: Option<usize>,
    shell_manager: ShellManager,
    input_state: text_input::State,
    suggestions: Vec<String>,
    active_suggestion: Option<usize>,
    
    // Agent mode
    agent_mode: Option<AgentMode>,
    agent_enabled: bool,
    agent_streaming: bool,
    
    // Configuration
    config: AppConfig,
    settings_open: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    ExecuteCommand,
    CommandOutput(String, i32), // output, exit_code
    KeyPressed(iced::keyboard::Key),
    HistoryUp,
    HistoryDown,
    SuggestionSelected(usize),
    BlockAction(Uuid, BlockMessage),
    Tick,
    
    // Agent mode messages
    ToggleAgentMode,
    AgentMessage(AgentMessage),
    AgentStreamingChunk(String),
    AgentError(String),
    
    // Settings messages
    ToggleSettings,
    SettingsMessage(settings::SettingsMessage),
    
    // Configuration
    ConfigLoaded(AppConfig),
    ConfigSaved,
}

#[derive(Debug, Clone)]
pub enum BlockMessage {
    Copy,
    Rerun,
    Delete,
    Export,
}

impl Application for NeoTerm {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let shell_manager = ShellManager::new();
        
        // Load configuration
        let config = AppConfig::load().unwrap_or_default();
        
        // Initialize agent mode if configured
        let agent_mode = if let Some(api_key) = std::env::var("OPENAI_API_KEY").ok() {
            let mut agent_config = AgentConfig::default();
            agent_config.api_key = Some(api_key);
            AgentMode::new(agent_config).ok()
        } else {
            None
        };
        
        (
            Self {
                blocks: Vec::new(),
                current_input: String::new(),
                input_history: Vec::new(),
                history_index: None,
                shell_manager,
                input_state: text_input::State::new(),
                suggestions: Vec::new(),
                active_suggestion: None,
                agent_mode,
                agent_enabled: false,
                agent_streaming: false,
                config,
                settings_open: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        if self.agent_enabled {
            "NeoTerm - Agent Mode".to_string()
        } else {
            "NeoTerm".to_string()
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::InputChanged(input) => {
                self.current_input = input.clone();
                self.suggestions = self.generate_suggestions(&input);
                Command::none()
            }
            Message::ExecuteCommand => {
                if !self.current_input.trim().is_empty() {
                    let command = self.current_input.clone();
                    self.input_history.push(command.clone());
                    self.history_index = None;
                    
                    if self.agent_enabled && self.agent_mode.is_some() {
                        // Send to agent mode
                        self.handle_agent_command(command)
                    } else {
                        // Regular command execution
                        let block = Block::new_command(command.clone());
                        self.blocks.push(block);
                        self.current_input.clear();
                        
                        Command::perform(
                            self.shell_manager.execute_command(command),
                            |(output, exit_code)| Message::CommandOutput(output, exit_code)
                        )
                    }
                } else {
                    Command::none()
                }
            }
            Message::CommandOutput(output, exit_code) => {
                if let Some(last_block) = self.blocks.last_mut() {
                    last_block.set_output(output, exit_code);
                }
                Command::none()
            }
            Message::ToggleAgentMode => {
                if let Some(ref mut agent) = self.agent_mode {
                    self.agent_enabled = agent.toggle();
                    if self.agent_enabled {
                        // Start new conversation
                        if let Ok(_) = agent.start_conversation() {
                            let block = Block::new_agent_message("Agent mode activated. How can I help you?".to_string());
                            self.blocks.push(block);
                        }
                    } else {
                        let block = Block::new_agent_message("Agent mode deactivated.".to_string());
                        self.blocks.push(block);
                    }
                } else {
                    // Try to initialize agent mode
                    if let Some(api_key) = std::env::var("OPENAI_API_KEY").ok() {
                        let mut agent_config = AgentConfig::default();
                        agent_config.api_key = Some(api_key);
                        if let Ok(agent) = AgentMode::new(agent_config) {
                            self.agent_mode = Some(agent);
                            self.agent_enabled = true;
                            let block = Block::new_agent_message("Agent mode activated. How can I help you?".to_string());
                            self.blocks.push(block);
                        } else {
                            let block = Block::new_error("Failed to initialize agent mode. Check your API key.".to_string());
                            self.blocks.push(block);
                        }
                    } else {
                        let block = Block::new_error("Agent mode requires OPENAI_API_KEY environment variable.".to_string());
                        self.blocks.push(block);
                    }
                }
                Command::none()
            }
            Message::AgentStreamingChunk(chunk) => {
                if let Some(last_block) = self.blocks.last_mut() {
                    if let BlockContent::AgentMessage { ref mut content, .. } = last_block.content {
                        content.push_str(&chunk);
                    }
                }
                Command::none()
            }
            Message::AgentError(error) => {
                let block = Block::new_error(format!("Agent error: {}", error));
                self.blocks.push(block);
                self.agent_streaming = false;
                Command::none()
            }
            Message::ToggleSettings => {
                self.settings_open = !self.settings_open;
                Command::none()
            }
            Message::HistoryUp => {
                if !self.input_history.is_empty() {
                    let new_index = match self.history_index {
                        None => Some(self.input_history.len() - 1),
                        Some(i) if i > 0 => Some(i - 1),
                        Some(i) => Some(i),
                    };
                    
                    if let Some(index) = new_index {
                        self.current_input = self.input_history[index].clone();
                        self.history_index = new_index;
                    }
                }
                Command::none()
            }
            Message::HistoryDown => {
                match self.history_index {
                    Some(i) if i < self.input_history.len() - 1 => {
                        self.history_index = Some(i + 1);
                        self.current_input = self.input_history[i + 1].clone();
                    }
                    Some(_) => {
                        self.history_index = None;
                        self.current_input.clear();
                    }
                    None => {}
                }
                Command::none()
            }
            Message::BlockAction(block_id, action) => {
                self.handle_block_action(block_id, action)
            }
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        if self.settings_open {
            // Show settings view
            let settings_view = settings::SettingsView::new(self.config.clone());
            return settings_view.view().map(Message::SettingsMessage);
        }

        let blocks_view = scrollable(
            column(
                self.blocks
                    .iter()
                    .map(|block| block.view())
                    .collect::<Vec<_>>()
            )
            .spacing(8)
        )
        .height(iced::Length::Fill);

        let input_view = self.create_input_view();
        let toolbar = self.create_toolbar();

        column![toolbar, blocks_view, input_view]
            .spacing(8)
            .padding(16)
            .into()
    }
}

impl NeoTerm {
    fn generate_suggestions(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Add command history matches
        for cmd in &self.input_history {
            if cmd.contains(input) && cmd != input {
                suggestions.push(cmd.clone());
            }
        }
        
        // Add common commands
        let common_commands = ["ls", "cd", "git", "npm", "cargo", "docker", "kubectl"];
        for cmd in &common_commands {
            if cmd.starts_with(input) && !input.is_empty() {
                suggestions.push(cmd.to_string());
            }
        }
        
        // Add agent mode suggestions
        if self.agent_enabled {
            let agent_suggestions = [
                "explain this command:",
                "help me with",
                "what does this error mean:",
                "how do I",
                "show me how to",
            ];
            for suggestion in &agent_suggestions {
                if suggestion.starts_with(input) && !input.is_empty() {
                    suggestions.push(suggestion.to_string());
                }
            }
        }
        
        suggestions.truncate(5);
        suggestions
    }

    fn create_input_view(&self) -> Element<Message> {
        let prompt_indicator = if self.agent_enabled {
            "🤖 "
        } else {
            "$ "
        };

        let placeholder = if self.agent_enabled {
            "Ask me anything or enter a command..."
        } else {
            "Enter command..."
        };

        let input = text_input(placeholder, &self.current_input)
            .on_input(Message::InputChanged)
            .on_submit(Message::ExecuteCommand)
            .padding(12)
            .size(16);

        let input_with_prompt = row![
            text(prompt_indicator).size(16),
            input
        ].spacing(8);

        let suggestions_view = if !self.suggestions.is_empty() {
            column(
                self.suggestions
                    .iter()
                    .enumerate()
                    .map(|(i, suggestion)| {
                        button(text(suggestion))
                            .on_press(Message::SuggestionSelected(i))
                            .width(iced::Length::Fill)
                            .into()
                    })
                    .collect::<Vec<_>>()
            )
            .spacing(2)
            .into()
        } else {
            column![].into()
        };

        column![input_with_prompt, suggestions_view].spacing(4).into()
    }

    fn create_toolbar(&self) -> Element<Message> {
        let agent_button = button(
            text(if self.agent_enabled { "🤖 Agent ON" } else { "🤖 Agent OFF" })
        )
        .on_press(Message::ToggleAgentMode);

        let settings_button = button(text("⚙️ Settings"))
            .on_press(Message::ToggleSettings);

        row![agent_button, settings_button]
            .spacing(8)
            .into()
    }

    fn handle_agent_command(&mut self, command: String) -> Command<Message> {
        if let Some(ref mut agent) = self.agent_mode {
            self.current_input.clear();
            
            // Add user message block
            let user_block = Block::new_user_message(command.clone());
            self.blocks.push(user_block);
            
            // Add streaming agent response block
            let agent_block = Block::new_agent_message(String::new());
            self.blocks.push(agent_block);
            self.agent_streaming = true;
            
            // Send message to agent
            let agent_clone = agent.clone();
            Command::perform(
                async move {
                    match agent_clone.send_message(command).await {
                        Ok(mut rx) => {
                            let mut full_response = String::new();
                            while let Some(chunk) = rx.recv().await {
                                full_response.push_str(&chunk);
                                // In a real implementation, you'd send streaming updates
                            }
                            Ok(full_response)
                        }
                        Err(e) => Err(e.to_string()),
                    }
                },
                |result| match result {
                    Ok(response) => Message::AgentStreamingChunk(response),
                    Err(error) => Message::AgentError(error),
                }
            )
        } else {
            Command::none()
        }
    }

    fn handle_block_action(&mut self, block_id: Uuid, action: BlockMessage) -> Command<Message> {
        match action {
            BlockMessage::Rerun => {
                if let Some(block) = self.blocks.iter().find(|b| b.id == block_id) {
                    match &block.content {
                        BlockContent::Command { input, .. } => {
                            let command = input.clone();
                            Command::perform(
                                self.shell_manager.execute_command(command),
                                |(output, exit_code)| Message::CommandOutput(output, exit_code)
                            )
                        }
                        _ => Command::none(),
                    }
                } else {
                    Command::none()
                }
            }
            BlockMessage::Delete => {
                self.blocks.retain(|b| b.id != block_id);
                Command::none()
            }
            BlockMessage::Copy => {
                // TODO: Implement clipboard copy
                Command::none()
            }
            BlockMessage::Export => {
                // TODO: Implement export functionality
                Command::none()
            }
        }
    }
}

fn main() -> iced::Result {
    // Initialize modules
    agent_mode_eval::init();
    
    NeoTerm::run(Settings::default())
}
