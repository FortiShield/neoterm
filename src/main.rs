use iced::{executor, Application, Command, Element, Settings, Theme};
use iced::widget::{column, container, scrollable, text_input, button, row, text};
use iced::keyboard::{self, KeyCode, Modifiers};
use std::path::PathBuf;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use futures_util::StreamExt; // For consuming reqwest response stream
use chrono::{DateTime, Local};

mod block; // Updated import path
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
mod api; // New API module

use block::{Block, BlockContent}; // Updated import
use shell::ShellManager;
use input::{EnhancedTextInput, Message as InputMessage, HistoryDirection, Direction};
use agent_mode_eval::{AgentMode, AgentConfig, AgentMessage, AgentToolCall};
use config::AppConfig;
use crate::{
    ui::{
        command_palette::{CommandPalette, CommandAction},
        ai_sidebar::AISidebar,
    },
    command::pty::{PtyManager, CommandStatus},
    workflows::debugger::WorkflowDebugger,
    plugins::plugin_manager::PluginManager,
    collaboration::session_sharing::SessionSharingManager,
    cloud::sync_manager::CloudSyncManager,
    performance::benchmarks::{PerformanceBenchmarks, BenchmarkSuite}, // Import BenchmarkSuite
};

#[derive(Debug, Clone)]
pub struct NeoTerm {
    blocks: Vec<Block>, // Changed from CommandBlock to Block
    input_bar: EnhancedTextInput,
    shell_manager: ShellManager,
    
    // Agent mode
    agent_mode: Arc<RwLock<AgentMode>>,
    agent_enabled: bool,
    agent_streaming_rx: Option<mpsc::Receiver<AgentMessage>>, // Receiver for agent messages
    
    // Configuration
    config: AppConfig,
    settings_open: bool,

    // Channels for PTY communication
    pty_tx: mpsc::Sender<PtyMessage>,
    pty_rx: mpsc::Receiver<PtyMessage>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Input(InputMessage),
    ExecuteCommand,
    PtyOutput(PtyMessage),
    KeyboardEvent(keyboard::Event),
    BlockAction(String, BlockMessage), // Block ID is String now
    Tick,
    
    // Agent mode messages
    ToggleAgentMode,
    AgentStream(AgentMessage), // Streamed messages from agent
    AgentStreamEnded,
    AgentError(String),
    
    // Settings messages
    ToggleSettings,
    SettingsMessage(settings::SettingsMessage),
    
    // Configuration
    ConfigLoaded(AppConfig),
    ConfigSaved,

    // Performance Benchmarks
    RunBenchmarks,
    BenchmarkResults(BenchmarkSuite), // New message for benchmark results
}

#[derive(Debug, Clone)]
pub enum BlockMessage {
    Copy,
    Rerun,
    Delete,
    Export,
    ToggleCollapse,
    SendToAI, // New: Send this block's content to AI as context
}

#[derive(Debug, Clone)]
pub enum PtyMessage {
    OutputChunk {
        block_id: String,
        content: String,
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
        
        let config = AppConfig::load().unwrap_or_default();
        
        let agent_config = {
            let mut cfg = AgentConfig::default();
            if let Some(api_key) = std::env::var("OPENAI_API_KEY").ok() {
                cfg.ai_config = agent_mode_eval::ai_client::AIClientConfig::OpenAI { api_key: Some(api_key) };
            }
            cfg
        };
        let agent_mode = Arc::new(RwLock::new(AgentMode::new(agent_config).unwrap()));

        // Start the API server
        let agent_mode_clone = agent_mode.clone();
        tokio::spawn(async move {
            api::start_api_server(agent_mode_clone).await;
        });

        let (pty_tx, pty_rx) = mpsc::channel(100);
       
        (
            Self {
                blocks: Vec::new(),
                input_bar: EnhancedTextInput::new(),
                shell_manager,
                agent_mode,
                agent_enabled: false,
                agent_streaming_rx: None,
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
            Message::Input(input_message) => {
                match input_message {
                    InputMessage::Submit => {
                        let command = self.input_bar.value().to_string();
                        self.input_bar.update(InputMessage::Submit);
                        if !command.trim().is_empty() {
                            if command.starts_with("#") || command.starts_with("/ai") {
                                self.handle_ai_command(command, None)
                            } else {
                                self.execute_command(command)
                            }
                        } else {
                            Command::none()
                        }
                    }
                    _ => {
                        self.input_bar.update(input_message);
                        Command::none()
                    }
                }
            }
            Message::ExecuteCommand => {
                Command::none()
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
                let agent_mode_arc_clone = self.agent_mode.clone();
                Command::perform(
                    async move {
                        let mut agent_mode = agent_mode_arc_clone.write().await;
                        let enabled = agent_mode.toggle();
                        if enabled {
                            if let Ok(_) = agent_mode.start_conversation() {
                                Some("Agent mode activated. How can I help you?".to_string())
                            } else {
                                None
                            }
                        } else {
                            Some("Agent mode deactivated.".to_string())
                        }
                    },
                    |msg| {
                        if let Some(content) = msg {
                            Message::AgentStream(AgentMessage::SystemMessage(content))
                        } else {
                            Message::AgentError("Failed to start agent conversation.".to_string())
                        }
                    }
                )
            }
            Message::AgentStream(agent_msg) => {
                match agent_msg {
                    AgentMessage::UserMessage(content) => {
                        let block = Block::new_user_message(content);
                        self.blocks.push(block);
                    }
                    AgentMessage::AgentResponse(content) => {
                        if let Some(last_block) = self.blocks.last_mut() {
                            if let BlockContent::AgentMessage { ref mut content: block_content, .. } = last_block.content {
                                block_content.push_str(&content);
                            } else {
                                // If the last block isn't an agent message, create a new one
                                let mut new_block = Block::new_agent_message(content);
                                new_block.set_status("Streaming...".to_string());
                                self.blocks.push(new_block);
                            }
                        } else {
                            // No blocks yet, create a new agent message block
                            let mut new_block = Block::new_agent_message(content);
                            new_block.set_status("Streaming...".to_string());
                            self.blocks.push(new_block);
                        }
                    }
                    AgentMessage::ToolCall(tool_call) => {
                        let block = Block::new_info(
                            format!("AI Tool Call: {}", tool_call.name),
                            format!("Arguments: {}", tool_call.arguments.to_string())
                        );
                        self.blocks.push(block);
                    }
                    AgentMessage::ToolResult(result) => {
                        let block = Block::new_info(
                            "AI Tool Result".to_string(),
                            result
                        );
                        self.blocks.push(block);
                    }
                    AgentMessage::SystemMessage(content) => {
                        let block = Block::new_info("System Message".to_string(), content);
                        self.blocks.push(block);
                    }
                    AgentMessage::Error(error) => {
                        let block = Block::new_error(format!("Agent error: {}", error));
                        self.blocks.push(block);
                    }
                    AgentMessage::Done => {
                        if let Some(last_block) = self.blocks.last_mut() {
                            if let BlockContent::AgentMessage { .. } = last_block.content {
                                last_block.set_status("Completed".to_string());
                            }
                        }
                        self.agent_streaming_rx = None; // Mark stream as ended
                    }
                }
                Command::none()
            }
            Message::AgentStreamEnded => {
                self.agent_streaming_rx = None;
                Command::none()
            }
            Message::AgentError(error) => {
                let block = Block::new_error(format!("Agent error: {}", error)); // Changed from CommandBlock to Block
                self.blocks.push(block);
                self.agent_streaming_rx = None;
                Command::none()
            }
            Message::ToggleSettings => {
                self.settings_open = !self.settings_open;
                Command::none()
            }
            Message::BlockAction(block_id, action) => {
                self.handle_block_action(block_id, action)
            }
            Message::Tick => {
                Command::none()
            }
            Message::KeyboardEvent(event) => {
                match event {
                    keyboard::Event::KeyPressed { key_code, modifiers, .. } => {
                        match key_code {
                            KeyCode::Up => {
                                self.input_bar.update(InputMessage::HistoryNavigated(HistoryDirection::Up));
                            }
                            KeyCode::Down => {
                                self.input_bar.update(InputMessage::HistoryNavigated(HistoryDirection::Down));
                            }
                            KeyCode::Tab => {
                                self.input_bar.update(InputMessage::NavigateSuggestions(Direction::Down));
                                self.input_bar.update(InputMessage::ApplySuggestion);
                            }
                            KeyCode::F1 => { // Handle F1 key for benchmarks
                                return Command::perform(async {}, |_| Message::RunBenchmarks);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::ConfigLoaded(_) => Command::none(),
            Message::ConfigSaved => Command::none(),
            Message::SettingsMessage(msg) => {
                let mut settings_view = settings::SettingsView::new(self.config.clone());
                settings_view.update(msg);
                self.config = settings_view.config;
                Command::none()
            }
            Message::RunBenchmarks => {
                // Create a new instance of PerformanceBenchmarks to run the tests
                Command::perform(
                    async move {
                        let mut benchmarks_runner = PerformanceBenchmarks::new();
                        benchmarks_runner.run_all_benchmarks().await
                    },
                    Message::BenchmarkResults,
                )
            }
            Message::BenchmarkResults(suite) => {
                let summary = suite.get_performance_summary();
                let block = Block::new_info("Performance Benchmark Results".to_string(), summary);
                self.blocks.push(block);
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

        let input_view = self.input_bar.view(prompt_indicator, placeholder).map(Message::Input);

        let toolbar = self.create_toolbar();

        column![toolbar, blocks_view, input_view]
            .spacing(8)
            .padding(16)
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let agent_stream_sub = if let Some(rx) = self.agent_streaming_rx.clone() {
            iced::Subscription::unfold(
                "agent_stream",
                rx,
                |mut receiver| async move {
                    match receiver.recv().await {
                        Some(msg) => (Message::AgentStream(msg), receiver),
                        None => (Message::AgentStreamEnded, receiver),
                    }
                },
            )
        } else {
            iced::Subscription::none()
        };

        iced::Subscription::batch(vec![
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick),
            self.pty_manager_subscription(),
            keyboard::Event::all().map(Message::KeyboardEvent),
            agent_stream_sub,
        ])
    }
}

impl NeoTerm {
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

    fn handle_ai_command(&mut self, command: String, context_block_id: Option<String>) -> Command<Message> {
        let prompt = command.trim_start_matches('#').trim_start_matches("/ai").trim().to_string();
        
        let user_block = Block::new_user_message(command.clone());
        self.blocks.push(user_block);
        
        // Prepare context blocks
        let mut context_blocks = Vec::new();
        if let Some(id) = context_block_id {
            if let Some(block) = self.blocks.iter().find(|b| b.id == id) {
                context_blocks.push(block.clone());
            }
        }

        let agent_mode_arc_clone = self.agent_mode.clone();
        let (tx, rx) = mpsc::channel(100); // Channel for agent messages
        self.agent_streaming_rx = Some(rx); // Set the receiver for the subscription

        Command::perform(
            async move {
                let mut agent_mode = agent_mode_arc_clone.write().await;
                match agent_mode.send_message(prompt, context_blocks).await {
                    Ok(mut stream_rx) => {
                        while let Some(msg) = stream_rx.recv().await {
                            if tx.send(msg).await.is_err() {
                                break; // Receiver dropped
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AgentMessage::Error(format!("Failed to send message to agent: {}", e))).await;
                    }
                }
            },
            |_| Message::Tick // Just a dummy message to trigger update
        )
    }

    fn handle_block_action(&mut self, block_id: String, action: BlockMessage) -> Command<Message> {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            match action {
                BlockMessage::Rerun => {
                    if let BlockContent::Command { input, .. } = &block.content {
                        let command = input.clone();
                        self.execute_command(command)
                    } else {
                        Command::none()
                    }
                }
                BlockMessage::Delete => {
                    self.blocks.retain(|b| b.id != block_id);
                    Command::none()
                }
                BlockMessage::Copy => {
                    // TODO: Implement copy to clipboard
                    Command::none()
                }
                BlockMessage::Export => {
                    // TODO: Implement export functionality
                    Command::none()
                }
                BlockMessage::ToggleCollapse => {
                    block.toggle_collapse();
                    Command::none()
                }
                BlockMessage::SendToAI => {
                    // Send the content of this block to the AI
                    let block_to_send = block.clone();
                    let prompt = format!("Please analyze the following block:\n{}", match &block_to_send.content {
                        BlockContent::Command { input, output, status, error, .. } => {
                            format!("Command: `{}`\nOutput:\n\`\`\`\n{}\n\`\`\`\nStatus: {}\nError: {}", input, output.iter().map(|(s, _)| s.clone()).collect::<Vec<String>>().join("\n"), status, error)
                        },
                        BlockContent::AgentMessage { content, is_user, .. } => {
                            format!("{}: {}", if *is_user { "User" } else { "Agent" }, content)
                        },
                        BlockContent::Info { title, message, .. } => {
                            format!("Info ({}): {}", title, message)
                        },
                        BlockContent::Error { message, .. } => {
                            format!("Error: {}", message)
                        },
                    });
                    self.handle_ai_command(prompt, Some(block_id.clone()))
                }
            }
        } else {
            Command::none()
        }
    }

    fn execute_command(&mut self, command: String) -> Command<Message> {
        let command_block = Block::new_command(command.clone()); // Changed from CommandBlock to Block
        let block_id = command_block.id.clone();
        self.blocks.push(command_block);
        
        let env_vars = self.config.env_profiles.active_profile
            .as_ref()
            .and_then(|name| self.config.env_profiles.profiles.get(name))
            .map(|profile| profile.variables.clone());

        let pty_tx = self.pty_tx.clone();
        Command::perform(
            async move {
                let parts: Vec<&str> = command.split_whitespace().collect();
                let cmd = parts[0];
                let args = &parts[1..];

                let mut output_receiver = PtyManager::new().execute_command(cmd, args, env_vars).await.unwrap();
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
    }

    fn add_sample_blocks(&mut self) {
        let welcome_block = Block::new_info( // Changed from CommandBlock to Block
            "Welcome to NeoPilot Terminal".to_string(),
            "This is a next-generation terminal with AI assistance.\nPress 'p' to open the command palette.\nPress 'a' to toggle the AI sidebar.\nPress 'F1' to run performance benchmarks.\nUse Up/Down arrows for history, Tab for autocomplete.\nType # or /ai followed by your query to ask the AI."
        );
        
        let sample_command = Block::new_command("$ echo 'Hello, NeoPilot!'".to_string()); // Changed from CommandBlock to Block
        let mut sample_output = Block::new_output("".to_string()); // Changed from CommandBlock to Block
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
