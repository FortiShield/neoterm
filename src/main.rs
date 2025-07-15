use iced::{executor, Application, Command, Element, Settings, Theme};
use iced::widget::{column, container, scrollable, text_input, button, row, text};
use std::path::PathBuf;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::collections::HashMap;

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
mod serve_wasm;
mod languages;
mod natural_language_detection;
mod graphql;
mod command;
mod drive;
mod fuzzy_match;
mod asset_macro;
mod ui;
mod plugins;
mod collaboration;
mod cloud;
mod performance;
mod sum_tree;
mod workflows;
mod lpc;
mod mcq;
mod markdown_parser;

use block::{Block, BlockContent}; // This Block is for Iced
use shell::ShellManager;
use input::EnhancedTextInput;
use agent_mode_eval::{AgentMode, AgentConfig, AgentMessage};
use config::AppConfig;
use crate::{
    ui::{
        block::CommandBlock, // Use the new Iced CommandBlock
        command_palette::{CommandPalette, CommandAction},
        ai_sidebar::AISidebar,
    },
    command::pty::{PtyManager, CommandStatus},
    workflows::debugger::WorkflowDebugger,
    plugins::plugin_manager::PluginManager,
    collaboration::session_sharing::SessionSharingManager,
    cloud::sync_manager::CloudSyncManager,
    performance::benchmarks::PerformanceBenchmarks,
};

#[derive(Debug, Clone)]
pub struct NeoTerm {
    blocks: Vec<CommandBlock>, // Now stores Iced CommandBlock
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

    // Channels for PTY communication
    pty_tx: mpsc::Sender<PtyMessage>,
    pty_rx: mpsc::Receiver<PtyMessage>,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    ExecuteCommand,
    PtyOutput(PtyMessage), // Message from PTY async task
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
    ToggleCollapse, // Added for Iced CommandBlock
}

// Messages for PTY communication
#[derive(Debug, Clone)]
pub enum PtyMessage {
    OutputChunk {
        block_id: String,
        content: String, // Plain string from VTE
        is_stdout: bool,
    },
    Completed {
        block_id: String,
        exit_code: i32,
    },
    Failed {
        block_id: String,
        error: String,
    },
    Killed {
        block_id: String,
    },
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

        let (pty_tx, pty_rx) = mpsc::channel(100);
       
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
                pty_tx,
                pty_rx,
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
                        self.handle_agent_command(command)
                    } else {
                        let command_block = CommandBlock::new_command(command.clone());
                        let block_id = command_block.id.clone();
                        self.blocks.push(command_block);
                        self.current_input.clear();
                        
                        let env_vars = self.config.env_profiles.active_profile
                            .as_ref()
                            .and_then(|name| self.config.env_profiles.profiles.get(name))
                            .map(|profile| profile.variables.clone());

                        let pty_tx = self.pty_tx.clone();
                        Command::perform(
                            async move {
                                let mut output_receiver = PtyManager::new().execute_command(&command, &[], env_vars).await.unwrap();
                                while let Some(output) = output_receiver.recv().await {
                                    match output.status {
                                        CommandStatus::Running => {
                                            if !output.stdout.is_empty() {
                                                let _ = pty_tx.send(PtyMessage::OutputChunk {
                                                    block_id: block_id.clone(),
                                                    content: output.stdout,
                                                    is_stdout: true,
                                                }).await;
                                            }
                                            if !output.stderr.is_empty() {
                                                let _ = pty_tx.send(PtyMessage::OutputChunk {
                                                    block_id: block_id.clone(),
                                                    content: output.stderr,
                                                    is_stdout: false,
                                                }).await;
                                            }
                                        }
                                        CommandStatus::Completed(exit_code) => {
                                            let _ = pty_tx.send(PtyMessage::Completed {
                                                block_id: block_id.clone(),
                                                exit_code,
                                            }).await;
                                            break;
                                        }
                                        CommandStatus::Failed(error) => {
                                            let _ = pty_tx.send(PtyMessage::Failed {
                                                block_id: block_id.clone(),
                                                error,
                                            }).await;
                                            break;
                                        }
                                        CommandStatus::Killed => {
                                            let _ = pty_tx.send(PtyMessage::Killed {
                                                block_id: block_id.clone(),
                                            }).await;
                                            break;
                                        }
                                    }
                                }
                            },
                            |_| Message::Tick // Send a generic tick to trigger UI update
                        )
                    }
                } else {
                    Command::none()
                }
            }
            Message::PtyOutput(pty_msg) => {
                if let Some(block) = self.blocks.iter_mut().find(|b| b.id == pty_msg.get_block_id()) {
                    match pty_msg {
                        PtyMessage::OutputChunk { content, is_stdout, .. } => {
                            block.add_output_line(content, is_stdout);
                        }
                        PtyMessage::Completed { exit_code, .. } => {
                            block.set_status(format!("Completed with exit code: {}", exit_code));
                            if exit_code != 0 {
                                block.set_error(true);
                            }
                        }
                        PtyMessage::Failed { error, .. } => {
                            block.set_status(format!("Failed: {}", error));
                            block.set_error(true);
                        }
                        PtyMessage::Killed { .. } => {
                            block.set_status("Killed".to_string());
                            block.set_error(true);
                        }
                    }
                }
                Command::none()
            }
            Message::ToggleAgentMode => {
                if let Some(ref mut agent) = self.agent_mode {
                    self.agent_enabled = agent.toggle();
                    if self.agent_enabled {
                        if let Ok(_) = agent.start_conversation() {
                            let block = CommandBlock::new_agent_message("Agent mode activated. How can I help you?".to_string());
                            self.blocks.push(block);
                        }
                    } else {
                        let block = CommandBlock::new_agent_message("Agent mode deactivated.".to_string());
                        self.blocks.push(block);
                    }
                } else {
                    if let Some(api_key) = std::env::var("OPENAI_API_KEY").ok() {
                        let mut agent_config = AgentConfig::default();
                        agent_config.api_key = Some(api_key);
                        if let Ok(agent) = AgentMode::new(agent_config) {
                            self.agent_mode = Some(agent);
                            self.agent_enabled = true;
                            let block = CommandBlock::new_agent_message("Agent mode activated. How can I help you?".to_string());
                            self.blocks.push(block);
                        } else {
                            let block = CommandBlock::new_error("Failed to initialize agent mode. Check your API key.".to_string());
                            self.blocks.push(block);
                        }
                    } else {
                        let block = CommandBlock::new_error("Agent mode requires OPENAI_API_KEY environment variable.".to_string());
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
                let block = CommandBlock::new_error(format!("Agent error: {}", error));
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
            Message::Tick => {
                // Used to trigger UI redraws for streaming output
                Command::none()
            }
            Message::KeyPressed(_) => Command::none(), // Placeholder for keyboard events
            Message::SuggestionSelected(_) => Command::none(), // Placeholder
            Message::ConfigLoaded(_) => Command::none(), // Placeholder
            Message::ConfigSaved => Command::none(), // Placeholder
            Message::SettingsMessage(msg) => {
                // Handle settings messages
                let mut settings_view = settings::SettingsView::new(self.config.clone());
                settings_view.update(msg);
                self.config = settings_view.config; // Update main config from settings view
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        if self.settings_open {
            let mut settings_view = settings::SettingsView::new(self.config.clone());
            return settings_view.view().map(Message::SettingsMessage);
        }

        let blocks_view = scrollable(
            column(
                self.blocks
                    .iter()
                    .map(|block| block.view().map(|msg| Message::BlockAction(block.id.clone(), msg)))
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

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(vec![
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick),
            self.pty_manager_subscription(),
        ])
    }
}

impl NeoTerm {
    fn generate_suggestions(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for cmd in &self.input_history {
            if cmd.contains(input) && cmd != input {
                suggestions.push(cmd.clone());
            }
        }
        
        let common_commands = ["ls", "cd", "git", "npm", "cargo", "docker", "kubectl"];
        for cmd in &common_commands {
            if cmd.starts_with(input) && !input.is_empty() {
                suggestions.push(cmd.to_string());
            }
        }
        
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

        let active_profile_name = self.config.env_profiles.active_profile.as_deref().unwrap_or("None");
        let env_profile_indicator = text(format!("Env: {}", active_profile_name)).size(14);

        row![agent_button, settings_button, env_profile_indicator]
            .spacing(8)
            .into()
    }

    fn handle_agent_command(&mut self, command: String) -> Command<Message> {
        if let Some(ref mut agent) = self.agent_mode {
            self.current_input.clear();
            
            let user_block = CommandBlock::new_user_message(command.clone());
            self.blocks.push(user_block);
            
            let agent_block = CommandBlock::new_agent_message(String::new());
            self.blocks.push(agent_block);
            self.agent_streaming = true;
            
            let agent_clone = agent.clone();
            Command::perform(
                async move {
                    match agent_clone.send_message(command).await {
                        Ok(mut rx) => {
                            let mut full_response = String::new();
                            while let Some(chunk) = rx.recv().await {
                                full_response.push_str(&chunk);
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

    fn handle_block_action(&mut self, block_id: String, action: BlockMessage) -> Command<Message> {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            match action {
                BlockMessage::Rerun => {
                    if let BlockContent::Command { input, .. } = &block.content {
                        let command = input.clone();
                        let env_vars = self.config.env_profiles.active_profile
                            .as_ref()
                            .and_then(|name| self.config.env_profiles.profiles.get(name))
                            .map(|profile| profile.variables.clone());

                        let pty_tx = self.pty_tx.clone();
                        Command::perform(
                            async move {
                                let mut output_receiver = PtyManager::new().execute_command(&command, &[], env_vars).await.unwrap();
                                while let Some(output) = output_receiver.recv().await {
                                    match output.status {
                                        CommandStatus::Running => {
                                            if !output.stdout.is_empty() {
                                                let _ = pty_tx.send(PtyMessage::OutputChunk {
                                                    block_id: block_id.clone(),
                                                    content: output.stdout,
                                                    is_stdout: true,
                                                }).await;
                                            }
                                            if !output.stderr.is_empty() {
                                                let _ = pty_tx.send(PtyMessage::OutputChunk {
                                                    block_id: block_id.clone(),
                                                    content: output.stderr,
                                                    is_stdout: false,
                                                }).await;
                                            }
                                        }
                                        CommandStatus::Completed(exit_code) => {
                                            let _ = pty_tx.send(PtyMessage::Completed {
                                                block_id: block_id.clone(),
                                                exit_code,
                                            }).await;
                                            break;
                                        }
                                        CommandStatus::Failed(error) => {
                                            let _ = pty_tx.send(PtyMessage::Failed {
                                                block_id: block_id.clone(),
                                                error,
                                            }).await;
                                            break;
                                        }
                                        CommandStatus::Killed => {
                                            let _ = pty_tx.send(PtyMessage::Killed {
                                                block_id: block_id.clone(),
                                            }).await;
                                            break;
                                        }
                                    }
                                }
                            },
                            |_| Message::Tick
                        )
                    } else {
                        Command::none()
                    }
                }
                BlockMessage::Delete => {
                    self.blocks.retain(|b| b.id != block_id);
                    Command::none()
                }
                BlockMessage::Copy => {
                    Command::none() // TODO: Implement clipboard copy
                }
                BlockMessage::Export => {
                    Command::none() // TODO: Implement export functionality
                }
                BlockMessage::ToggleCollapse => {
                    block.toggle_collapse();
                    Command::none()
                }
            }
        } else {
            Command::none()
        }
    }

    fn add_sample_blocks(&mut self) {
        let welcome_block = CommandBlock::new_info(
            "Welcome to NeoPilot Terminal".to_string(),
            "This is a next-generation terminal with AI assistance.\nPress 'p' to open the command palette.\nPress 'a' to toggle the AI sidebar.\nPress 'F1' to run performance benchmarks.".to_string()
        );
        
        let sample_command = CommandBlock::new_command("$ echo 'Hello, NeoPilot!'".to_string());
        let mut sample_output = CommandBlock::new_output("".to_string());
        sample_output.add_output_line("Hello, NeoPilot!".to_string(), true);
        sample_output.set_status("Completed with exit code: 0".to_string());
        
        self.blocks.push(welcome_block);
        self.blocks.push(sample_command);
        self.blocks.push(sample_output);
    }

    fn pty_manager_subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::unfold(
            "pty_manager_events",
            self.pty_rx.clone(),
            |mut receiver| async move {
                let msg = receiver.recv().await.unwrap();
                (Message::PtyOutput(msg), receiver)
            },
        )
    }
}

impl PtyMessage {
    fn get_block_id(&self) -> &str {
        match self {
            PtyMessage::OutputChunk { block_id, .. } => block_id,
            PtyMessage::Completed { block_id, .. } => block_id,
            PtyMessage::Failed { block_id, .. } => block_id,
            PtyMessage::Killed { block_id, .. } => block_id,
        }
    }
}

fn main() -> iced::Result {
    NeoTerm::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}
