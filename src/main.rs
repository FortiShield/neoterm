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
use clap::Parser; // Import Parser for CLI
use anyhow::Result;
use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use ratatui::text::{Line, Span, Style};

// Import modules
mod ai; // New AI module
mod api;
mod asset_macro;
mod block;
mod cli;
mod cloud;
mod collaboration;
mod command;
mod config;
mod drive;
mod fuzzy_match;
mod graphql;
mod input;
mod integration;
mod languages;
mod lpc;
mod markdown_parser;
mod mcq;
mod natural_language_detection;
mod performance;
mod plugins;
mod renderer;
mod resources;
mod serve_wasm;
mod settings;
mod shell;
mod string_offset;
mod sum_tree;
mod syntax_tree;
mod ui;
mod virtual_fs;
mod watcher;
mod websocket;
mod workflows;
mod agent_mode_eval; // Keep agent_mode_eval for AgentMode struct

use ai::assistant::Assistant; // Use the new AI Assistant
use agent_mode_eval::{AgentMode, AgentConfig, AgentMessage, AgentToolCall}; // Keep AgentMode and related types
use block::{Block, BlockContent, BlockManager, BlockType}; // Updated import path
use shell::ShellManager;
use input::{EnhancedTextInput, Message as InputMessage, HistoryDirection, Direction, InputManager, InputEvent};
use config::{AppConfig, ConfigManager, preferences::Preferences};
use crate::{
    ui::{
        command_palette::{CommandPalette, CommandAction},
        ai_sidebar::AISidebar,
    },
    command::pty::{PtyManager, CommandStatus},
    workflows::debugger::WorkflowDebugger,
    plugins::plugin_manager::PluginManager,
    collaboration::session_sharing::SessionSharingManager,
    cloud::sync_manager::{CloudSyncManager as SyncManager, SyncEvent, SyncConfig}, // Renamed to SyncManager to avoid conflict
    performance::benchmarks::{PerformanceBenchmarks, BenchmarkSuite, BenchmarkResult}, // Import BenchmarkSuite
    cli::{Cli, Commands, ConfigCommands, AiCommands, PluginCommands, WorkflowCommands}, // Import CLI components
};
use command::{CommandManager, CommandEvent};
use drive::{DriveManager, DriveConfig, DriveEvent};
use fuzzy_match::FuzzyMatchManager;
use graphql::build_schema;
use languages::LanguageManager;
use lpc::LpcEngine;
use markdown_parser::MarkdownParser;
use mcq::McqManager;
use natural_language_detection::NaturalLanguageDetector;
use resources::ResourceManager;
use settings::SettingsManager;
use shell::ShellManager;
use string_offset::StringOffsetManager;
use sum_tree::SumTreeManager;
use syntax_tree::SyntaxTreeManager;
use virtual_fs::VirtualFileSystem;
use watcher::{Watcher, WatcherEvent};
use websocket::WebSocketServer;
use workflows::executor::WorkflowExecutor;
use workflows::manager::WorkflowManager;
use collaboration::session_sharing::CollaborationEvent; // Import CollaborationEvent

#[derive(Debug, Clone)]
pub struct NeoTerm {
   blocks: Vec<Block>, // Changed from CommandBlock to Block
   input_bar: EnhancedTextInput,
   shell_manager: ShellManager,
   
   // Agent mode
   agent_mode: Arc<RwLock<AgentMode>>, // Still use AgentMode, but it will wrap the new Assistant
   agent_enabled: bool,
   agent_streaming_rx: Option<mpsc::Receiver<AgentMessage>>, // Receiver for agent messages
   
   // Configuration
   config: AppConfig,
   settings_open: bool,

   // Channels for PTY communication
   pty_tx: mpsc::Sender<PtyMessage>,
   pty_rx: mpsc::Receiver<PtyMessage>,
}

/// Main application state.
pub struct App {
    should_quit: bool,
    input_manager: InputManager,
    renderer: Renderer,
    block_manager: BlockManager,
    config_manager: Arc<ConfigManager>,
    ai_assistant: Arc<RwLock<Assistant>>, // Use the new AI Assistant
    workflow_manager: Arc<WorkflowManager>,
    plugin_manager: Arc<PluginManager>,
    sync_manager: Arc<SyncManager>,
    collaboration_manager: Arc<SessionSharingManager>,
    command_manager: Arc<CommandManager>,
    drive_manager: Arc<DriveManager>,
    fuzzy_match_manager: Arc<FuzzyMatchManager>,
    graphql_schema: Arc<graphql::AppSchema>,
    language_manager: Arc<LanguageManager>,
    lpc_engine: Arc<LpcEngine>,
    markdown_parser: Arc<MarkdownParser>,
    mcq_manager: Arc<McqManager>,
    natural_language_detector: Arc<NaturalLanguageDetector>,
    resource_manager: Arc<ResourceManager>,
    settings_manager: Arc<SettingsManager>,
    shell_manager: Arc<ShellManager>,
    string_offset_manager: Arc<StringOffsetManager>,
    sum_tree_manager: Arc<SumTreeManager>,
    syntax_tree_manager: Arc<SyntaxTreeManager>,
    virtual_file_system: Arc<VirtualFileSystem>,
    watcher: Arc<Watcher>,
    websocket_server: Arc<WebSocketServer>,
    wasm_server: Arc<WasmServer>,
    preferences: Preferences,
    benchmark_results: Option<Vec<BenchmarkResult>>,
    // Channels for inter-module communication
    sync_event_rx: mpsc::Receiver<SyncEvent>,
    collaboration_event_rx: mpsc::Receiver<CollaborationEvent>,
    command_event_rx: mpsc::Receiver<CommandEvent>,
    drive_event_rx: mpsc::Receiver<DriveEvent>,
    watcher_event_rx: mpsc::Receiver<watcher::WatcherEvent>,
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
   CommandGenerated(String), // New: A shell command generated by AI
   SuggestedFix(String), // New: A suggested command fix from AI
   
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

/// Messages that can be sent to the main application loop.
#[derive(Debug)]
pub enum AppMessage {
    Quit,
    Tick,
    Render,
    Input(input::InputEvent),
    RunBenchmarks,
    BenchmarkResults(Vec<BenchmarkResult>),
    SyncEvent(SyncEvent),
    CollaborationEvent(CollaborationEvent),
    CommandEvent(CommandEvent),
    DriveEvent(DriveEvent),
    WatcherEvent(watcher::WatcherEvent),
    // Add more messages for other events (e.g., plugin events, workflow events)
}

#[derive(Debug, Clone)]
pub enum BlockMessage {
    Copy,
    Rerun,
    Delete,
    Export,
    ToggleCollapse,
    SendToAI, // New: Send this block's content to AI as context
    SuggestFix, // New: Ask AI to suggest a fix for this block's command
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
            // Load API key from environment variable for AgentConfig
            if let Some(api_key) = std::env::var("OPENAI_API_KEY").ok() {
                cfg.api_key = Some(api_key);
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
                        PtyMessage::Completed { exit_code, block_id } => {
                            block.set_status(format!("Completed with exit code: {}", exit_code));
                            if exit_code != 0 {
                                block.set_error(true);
                                // Trigger AI fix suggestion
                                if let BlockContent::Command { input, output, .. } = &block.content {
                                    let original_command = input.clone();
                                    let error_output = output.iter()
                                        .filter(|(_, is_stdout)| !is_stdout) // Filter for stderr
                                        .map(|(s, _)| s.clone())
                                        .collect::<Vec<String>>()
                                        .join("\n");
                                    let error_msg = if error_output.is_empty() {
                                        format!("Command exited with non-zero code: {}", exit_code)
                                    } else {
                                        format!("Error output:\n{}", error_output)
                                    };

                                    let agent_mode_arc_clone = self.agent_mode.clone();
                                    return Command::perform(
                                        async move {
                                            let mut agent_mode = agent_mode_arc_clone.write().await;
                                            match agent_mode.fix(&original_command, &error_msg).await {
                                                Ok(suggested_command) => Message::SuggestedFix(suggested_command),
                                                Err(e) => Message::AgentError(format!("Failed to get fix suggestion: {}", e)),
                                            }
                                        },
                                        |msg| msg
                                    );
                                }
                            }
                        }
                        PtyMessage::Failed { error, block_id } => {
                            block.set_status(format!("Failed: {}", error));
                            block.set_error(true);
                            // Trigger AI fix suggestion
                            if let BlockContent::Command { input, .. } = &block.content {
                                let original_command = input.clone();
                                let agent_mode_arc_clone = self.agent_mode.clone();
                                return Command::perform(
                                    async move {
                                        let mut agent_mode = agent_mode_arc_clone.write().await;
                                        match agent_mode.fix(&original_command, &error).await {
                                            Ok(suggested_command) => Message::SuggestedFix(suggested_command),
                                            Err(e) => Message::AgentError(format!("Failed to get fix suggestion: {}", e)),
                                        }
                                    },
                                    |msg| msg
                                );
                            }
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
                            if let Ok(_) = agent_mode.start_conversation().await { // Await start_conversation
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
            Message::CommandGenerated(generated_command) => {
                // Auto-fill the input bar with the generated command
                self.input_bar.update(InputMessage::InputChanged(generated_command.clone()));
                // Optionally, add an info block that the command was generated
                let info_block = Block::new_info(
                    "AI Generated Command".to_string(),
                    format!("The command has been auto-filled into the input bar: `{}`. Press Enter to execute.", generated_command)
                );
                self.blocks.push(info_block);
                Command::none()
            }
            Message::SuggestedFix(suggested_command) => {
                // Auto-fill the input bar with the suggested command
                self.input_bar.update(InputMessage::InputChanged(suggested_command.clone()));
                let info_block = Block::new_info(
                    "AI Suggested Fix".to_string(),
                    format!("AI suggested a fix for the last failed command. It has been auto-filled into the input bar: `{}`. Press Enter to execute.", suggested_command)
                );
                self.blocks.push(info_block);
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
        let prompt_content = command.trim_start_matches('#').trim_start_matches("/ai").trim().to_string();
        
        let user_block = Block::new_user_message(command.clone());
        self.blocks.push(user_block);
        
        // Prepare context blocks (currently unused for command generation, but kept for consistency)
        let mut context_blocks = Vec::new();
        if let Some(id) = context_block_id {
            if let Some(block) = self.blocks.iter().find(|b| b.id == id) {
                context_blocks.push(block.clone());
            }
        }

        let agent_mode_arc_clone = self.agent_mode.clone();
        let (tx, rx) = mpsc::channel(100); // Channel for agent messages
        self.agent_streaming_rx = Some(rx); // Set the receiver for the subscription

        // Check if the prompt is a command generation request
        if prompt_content.to_lowercase().starts_with("generate command for") ||
           prompt_content.to_lowercase().starts_with("create command to") ||
           prompt_content.to_lowercase().starts_with("command to") {
            
            let natural_language_query = prompt_content
                .trim_start_matches("generate command for")
                .trim_start_matches("create command to")
                .trim_start_matches("command to")
                .trim()
                .to_string();

            Command::perform(
                async move {
                    let mut agent_mode = agent_mode_arc_clone.write().await; // Lock the agent_mode
                    match agent_mode.generate_command(&natural_language_query).await {
                        Ok(generated_command) => {
                            // Send the generated command back to the main loop
                            Message::CommandGenerated(generated_command)
                        }
                        Err(e) => {
                            // Send an error message if command generation fails
                            Message::AgentError(format!("Failed to generate command: {}", e))
                        }
                    }
                },
                |msg| msg // The async block directly returns a Message
            )
        } else {
            // Existing general chat logic
            Command::perform(
                async move {
                    let mut agent_mode = agent_mode_arc_clone.write().await;
                    match agent_mode.send_message(prompt_content, context_blocks).await {
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
                BlockMessage::SuggestFix => {
                    if let BlockContent::Command { input, output, status, error, .. } = &block.content {
                        let original_command = input.clone();
                        let error_output = output.iter()
                            .filter(|(_, is_stdout)| !is_stdout) // Filter for stderr
                            .map(|(s, _)| s.clone())
                            .collect::<Vec<String>>()
                            .join("\n");
                        let error_msg = if *error { // If block is marked as error
                            if error_output.is_empty() {
                                status.clone() // Use the status message if no stderr
                            } else {
                                format!("Error output:\n{}", error_output)
                            }
                        } else {
                            "No specific error message available, but command failed.".to_string()
                        };

                        let agent_mode_arc_clone = self.agent_mode.clone();
                        return Command::perform(
                            async move {
                                let mut agent_mode = agent_mode_arc_clone.write().await;
                                match agent_mode.fix(&original_command, &error_msg).await {
                                    Ok(suggested_command) => Message::SuggestedFix(suggested_command),
                                    Err(e) => Message::AgentError(format!("Failed to get fix suggestion: {}", e)),
                                }
                            },
                            |msg| msg
                        );
                    }
                    Command::none()
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

impl App {
    pub async fn new() -> Result<(App, mpsc::Sender<AppMessage>)> {
        let (app_tx, app_rx) = mpsc::channel(100);

        // Initialize ConfigManager first to load preferences
        let config_manager = Arc::new(ConfigManager::new().await?);
        config_manager.init().await?;
        let preferences = config_manager.get_preferences().await;

        // Initialize other managers with their respective channels
        let (sync_event_tx, sync_event_rx) = mpsc::channel(100);
        let sync_manager = Arc::new(SyncManager::new(SyncConfig::default(), sync_event_tx));
        sync_manager.init().await?;

        let (collab_event_tx, collab_event_rx) = mpsc::channel(100);
        let collaboration_manager = Arc::new(SessionSharingManager::new(collab_event_tx));
        collaboration_manager.init().await?;

        let (command_event_tx, command_event_rx) = mpsc::channel(100);
        let command_manager = Arc::new(CommandManager::new(command_event_tx));
        command_manager.init().await?;

        let (drive_event_tx, drive_event_rx) = mpsc::channel(100);
        let drive_manager = Arc::new(DriveManager::new(DriveConfig::default(), drive_event_tx));
        drive_manager.init().await?;

        let (watcher_event_tx, watcher_event_rx) = mpsc::channel(100);
        let watcher = Arc::new(Watcher::new(watcher_event_tx));
        watcher.init().await?;

        // Initialize AI Assistant
        let ai_assistant = Arc::new(RwLock::new(Assistant::new(
            &preferences.ai_provider_type,
            preferences.ai_api_key.clone(),
            preferences.ai_model.clone(),
        )?));
        // No explicit init for Assistant, its internal components are ready on new()

        // Initialize other modules
        let workflow_manager = Arc::new(WorkflowManager::new());
        workflow_manager.init().await?;
        let plugin_manager = Arc::new(PluginManager::new());
        plugin_manager.init().await?;
        let fuzzy_match_manager = Arc::new(FuzzyMatchManager::new());
        fuzzy_match_manager.init();
        let graphql_schema = Arc::new(build_schema());
        graphql::init(); // Initialize GraphQL module
        let language_manager = Arc::new(LanguageManager::new());
        language_manager.init().await?;
        let lpc_engine = Arc::new(LpcEngine::new(mpsc::channel(100).0)); // Dummy sender for now
        lpc_engine.init().await?;
        let markdown_parser = Arc::new(MarkdownParser::new());
        markdown_parser.init();
        let mcq_manager = Arc::new(McqManager::new());
        mcq_manager.init().await?;
        let natural_language_detector = Arc::new(NaturalLanguageDetector::new());
        natural_language_detector.init().await?;
        let resource_manager = Arc::new(ResourceManager::new());
        resource_manager.init().await?;
        let settings_manager = Arc::new(SettingsManager::new(config_manager.clone()));
        settings_manager.init().await?;
        let shell_manager = Arc::new(ShellManager::new());
        shell_manager.init().await?;
        let string_offset_manager = Arc::new(StringOffsetManager::new());
        string_offset_manager.init();
        let sum_tree_manager = Arc::new(SumTreeManager::new());
        sum_tree_manager.init();
        let syntax_tree_manager = Arc::new(SyntaxTreeManager::new());
        syntax_tree_manager.init().await?;
        let virtual_file_system = Arc::new(VirtualFileSystem::new());
        virtual_file_system.init().await?;
        let websocket_server = Arc::new(WebSocketServer::new());
        websocket_server.init().await?;
        let wasm_server = Arc::new(WasmServer::new());
        wasm_server.init().await?;

        // Initialize BlockManager and Renderer
        let mut block_manager = BlockManager::new();
        block_manager.add_block(BlockType::Welcome.into());
        block_manager.add_block(BlockType::Terminal.into());
        block_manager.add_block(BlockType::Info.into());

        let renderer = Renderer::new(config_manager.clone()).await?;

        let app = App {
            should_quit: false,
            input_manager: InputManager::new(),
            renderer,
            block_manager,
            config_manager,
            ai_assistant, // Use the new AI Assistant
            workflow_manager,
            plugin_manager,
            sync_manager,
            collaboration_manager,
            command_manager,
            drive_manager,
            fuzzy_match_manager,
            graphql_schema,
            language_manager,
            lpc_engine,
            markdown_parser,
            mcq_manager,
            natural_language_detector,
            resource_manager,
            settings_manager,
            shell_manager,
            string_offset_manager,
            sum_tree_manager,
            syntax_tree_manager,
            virtual_file_system,
            watcher,
            websocket_server,
            wasm_server,
            preferences,
            benchmark_results: None,
            sync_event_rx,
            collaboration_event_rx,
            command_event_rx,
            drive_event_rx,
            watcher_event_rx,
        };

        Ok((app, app_tx))
    }

    pub async fn run(&mut self, mut app_rx: mpsc::Receiver<AppMessage>) -> Result<()> {
        self.input_manager.init().await?;
        self.input_manager.start_event_loop(); // Start polling events in a separate task

        // Start API server if enabled
        if self.preferences.enable_graphql_api {
            let schema_clone = self.graphql_schema.clone();
            tokio::spawn(async move {
                graphql::run_graphql_server().await;
            });
        }
        if self.preferences.enable_ai_assistant {
            let assistant_clone = self.ai_assistant.clone(); // Use the new assistant
            tokio::spawn(async move {
                api::run_api_server(assistant_clone).await; // Pass the new assistant
            });
        }

        // Initial render
        self.renderer.render(&mut self.block_manager).await?;

        while !self.should_quit {
            tokio::select! {
                // Prioritize app messages
                msg = app_rx.recv() => {
                    if let Some(msg) = msg {
                        self.update(msg).await?;
                    } else {
                        // Sender dropped, quit
                        break;
                    }
                }
                // Handle input events
                input_event = self.input_manager.next_event() => {
                    if let Some(event) = input_event {
                        self.update(AppMessage::Input(event)).await?;
                    }
                }
                // Handle sync events
                sync_event = self.sync_event_rx.recv(), if self.preferences.enable_cloud_sync => {
                    if let Some(event) = sync_event {
                        self.update(AppMessage::SyncEvent(event)).await?;
                    }
                }
                // Handle collaboration events
                collab_event = self.collaboration_event_rx.recv(), if self.preferences.enable_collaboration => {
                    if let Some(event) = collab_event {
                        self.update(AppMessage::CollaborationEvent(event)).await?;
                    }
                }
                // Handle command events
                command_event = self.command_event_rx.recv() => {
                    if let Some(event) = command_event {
                        self.update(AppMessage::CommandEvent(event)).await?;
                    }
                }
                // Handle drive events
                drive_event = self.drive_event_rx.recv(), if self.preferences.enable_drive_integration => {
                    if let Some(event) = drive_event {
                        self.update(AppMessage::DriveEvent(event)).await?;
                    }
                }
                // Handle watcher events
                watcher_event = self.watcher_event_rx.recv(), if self.preferences.enable_watcher => {
                    if let Some(event) = watcher_event {
                        self.update(AppMessage::WatcherEvent(event)).await?;
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Regular tick for UI updates or background tasks
                    self.update(AppMessage::Tick).await?;
                }
            }

            // Re-render after processing messages
            self.renderer.render(&mut self.block_manager).await?;
        }

        self.shutdown().await?;
        Ok(())
    }

    async fn update(&mut self, message: AppMessage) -> Result<()> {
        match message {
            AppMessage::Quit => self.should_quit = true,
            AppMessage::Tick => {
                // Update any time-sensitive UI elements or trigger background checks
            }
            AppMessage::Render => { /* Triggered by other updates, handled by main loop */ }
            AppMessage::Input(event) => {
                match event {
                    input::InputEvent::Key(key) => {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.should_quit = true;
                            }
                            KeyCode::Tab => {
                                self.block_manager.next_block();
                            }
                            KeyCode::BackTab => {
                                self.block_manager.previous_block();
                            }
                            KeyCode::F(1) => {
                                // Trigger performance benchmarks
                                log::info!("F1 pressed: Running performance benchmarks.");
                                let app_tx_clone = self.renderer.get_app_sender();
                                tokio::spawn(async move {
                                    let benchmarks = PerformanceBenchmarks::new();
                                    let results = benchmarks.run_all_benchmarks().await;
                                    if let Err(e) = app_tx_clone.send(AppMessage::BenchmarkResults(results)).await {
                                        log::error!("Failed to send benchmark results: {:?}", e);
                                    }
                                });
                                self.block_manager.set_block_content(
                                    BlockType::BenchmarkResults,
                                    vec![Line::from(Span::raw("Running benchmarks... Please wait."))],
                                );
                            }
                            _ => {
                                // Handle other key presses, e.g., send to active block
                                if let Some(active_block) = self.block_manager.get_active_block_mut() {
                                    match active_block.block_type {
                                        BlockType::Terminal => {
                                            // Simulate sending input to terminal
                                            let current_content = active_block.content.clone();
                                            let mut new_content_str = current_content.iter()
                                                .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect::<String>())
                                                .collect::<Vec<String>>()
                                                .join("\n");

                                            match key.code {
                                                KeyCode::Char(c) => new_content_str.push(c),
                                                KeyCode::Backspace => { new_content_str.pop(); },
                                                KeyCode::Enter => { new_content_str.push('\n'); /* Process command */ },
                                                _ => {}
                                            }
                                            active_block.content = new_content_str.lines().map(|s| Line::from(Span::raw(s.to_string()))).collect();
                                        }
                                        _ => {
                                            // Generic input handling for other blocks
                                            log::debug!("Key {:?} pressed in block {:?}", key.code, active_block.block_type);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    input::InputEvent::Resize(w, h) => {
                        self.renderer.resize(w, h).await?;
                        self.block_manager.update_layout(self.renderer.get_terminal_area());
                    }
                    _ => {} // Handle other input events like mouse
                }
            }
            AppMessage::BenchmarkResults(results) => {
                self.benchmark_results = Some(results.clone());
                let mut content = vec![Line::from(Span::styled("Benchmark Results:", Style::default().fg(ratatui::style::Color::LightCyan)))];
                for res in results {
                    content.push(Line::from(Span::raw(format!("  {}: {:.2?} ({} iterations)", res.name, res.duration, res.iterations))));
                }
                self.block_manager.set_block_content(BlockType::BenchmarkResults, content);
            }
            AppMessage::SyncEvent(event) => {
                log::info!("Sync Event: {:?}", event);
                let content = match event {
                    SyncEvent::Start => vec![Line::from("Cloud Sync: Starting...")],
                    SyncEvent::Progress(msg) => vec![Line::from(format!("Cloud Sync: {}", msg))],
                    SyncEvent::Complete => vec![Line::from("Cloud Sync: Complete!")],
                    SyncEvent::Error(e) => vec![Line::from(format!("Cloud Sync Error: {}", e))],
                };
                self.block_manager.set_block_content(BlockType::Info, content);
            }
            AppMessage::CollaborationEvent(event) => {
                log::info!("Collaboration Event: {:?}", event);
                let content = match event {
                    CollaborationEvent::SessionStarted { address } => vec![Line::from(format!("Collab: Session started on {}", address))],
                    CollaborationEvent::SessionEnded => vec![Line::from("Collab: Session ended.")],
                    CollaborationEvent::PeerConnected { peer_id } => vec![Line::from(format!("Collab: Peer {} connected.", peer_id))],
                    CollaborationEvent::PeerDisconnected { peer_id } => vec![Line::from(format!("Collab: Peer {} disconnected.", peer_id))],
                    CollaborationEvent::TextUpdate { content, cursor_pos } => vec![Line::from(format!("Collab: Text update (len={}, cursor={})", content.len(), cursor_pos))],
                    CollaborationEvent::CommandExecuted { command } => vec![Line::from(format!("Collab: Command executed: {}", command))],
                    CollaborationEvent::Error(e) => vec![Line::from(format!("Collab Error: {}", e))],
                };
                self.block_manager.set_block_content(BlockType::Info, content);
            }
            AppMessage::CommandEvent(event) => {
                log::info!("Command Event: {:?}", event);
                let content = match event {
                    CommandEvent::Started { id, command_line } => vec![Line::from(format!("Cmd {}: Started: {}", id, command_line))],
                    CommandEvent::Output { data, is_stderr } => {
                        let output_str = String::from_utf8_lossy(&data);
                        vec![Line::from(format!("Cmd {}: {}{}", id, if is_stderr { "[ERR] " } else { "" }, output_str))]
                    },
                    CommandEvent::Completed { id, exit_code } => vec![Line::from(format!("Cmd {}: Completed with exit code {:?}", id, exit_code))],
                    CommandEvent::Error { id, message } => vec![Line::from(format!("Cmd {}: Error: {}", id, message))],
                };
                self.block_manager.set_block_content(BlockType::Info, content);
            }
            AppMessage::DriveEvent(event) => {
                log::info!("Drive Event: {:?}", event);
                let content = match event {
                    DriveEvent::Connected(provider) => vec![Line::from(format!("Drive: Connected to {:?}", provider))],
                    DriveEvent::Disconnected(provider) => vec![Line::from(format!("Drive: Disconnected from {:?}", provider))],
                    DriveEvent::FileListed { path, entries } => vec![Line::from(format!("Drive: Listed {} entries in {:?}", entries.len(), path))],
                    DriveEvent::FileDownloaded { path, local_path } => vec![Line::from(format!("Drive: Downloaded {:?} to {:?}", path, local_path))],
                    DriveEvent::FileUploaded { local_path, remote_path } => vec![Line::from(format!("Drive: Uploaded {:?} to {:?}", local_path, remote_path))],
                    DriveEvent::Error(e) => vec![Line::from(format!("Drive Error: {}", e))],
                };
                self.block_manager.set_block_content(BlockType::Info, content);
            }
            AppMessage::WatcherEvent(event) => {
                log::info!("Watcher Event: {:?}", event);
                let content = match event {
                    watcher::WatcherEvent::FileChanged { path } => vec![Line::from(format!("Watcher: File changed: {:?}", path))],
                    watcher::WatcherEvent::FileCreated { path } => vec![Line::from(format!("Watcher: File created: {:?}", path))],
                    watcher::WatcherEvent::FileDeleted { path } => vec![Line::from(format!("Watcher: File deleted: {:?}", path))],
                    watcher::WatcherEvent::Error(e) => vec![Line::from(format!("Watcher Error: {}", e))],
                };
                self.block_manager.set_block_content(BlockType::Info, content);
            }
            _ => {}
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.input_manager.shutdown().await?;
        disable_raw_mode()?;
        execute!(self.renderer.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.renderer.terminal.show_cursor()?;
        log::info!("NeoTerm shutdown complete.");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    // Initialize Sentry for crash reporting
    let _guard = sentry::init((
        "https://example.com/sentry/42", // Replace with your DSN
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    log::info!("Starting NeoTerm...");

    // Parse CLI arguments
    let cli = Cli::parse();

    if cli.verbose {
        log::set_max_level(log::LevelFilter::Debug);
        log::info!("Verbose logging enabled.");
    }

    // Initialize core modules
    config::init();
    asset_macro::init();
    block::init();
    cli::init();
    command::pty::init();
    fuzzy_match::init();
    graphql::init();
    input::init();
    languages::init();
    lpc::init();
    markdown_parser::init();
    string_offset::init();
    sum_tree::init();
    syntax_tree::init();
    ui::init();
    virtual_fs::init();
    watcher::init();
    websocket::init();
    workflows::init();
    workflows::debugger::init();
    workflows::executor::init();
    workflows::manager::init();
    workflows::ui::init();
    plugins::init();
    plugins::lua_engine::init();
    plugins::plugin_manager::init();
    plugins::wasm_runtime::init();
    resources::init();
    serve_wasm::init();
    settings::init();
    settings::keybinding_editor::init();
    settings::theme_editor::init();
    settings::yaml_theme_ui::init();
    shell::init();
    ai::init(); // Initialize the new AI module

    let (mut app, app_tx) = App::new().await?;

    // Handle CLI commands
    match cli.command {
        Some(cli::Commands::Gui { path }) => {
            if let Some(p) = path {
                log::info!("Starting GUI with initial path: {}", p);
                // TODO: Pass initial path to shell manager or virtual FS
            }
            //app.run(app_tx).await?;
            //TODO: Remove iced and use ratatui
        }
        Some(cli::Commands::Run { command, args }) => {
            log::info!("Running command in headless mode: {} {:?}", command, args);
            let cmd_id = uuid::Uuid::new_v4().to_string();
            let cmd = command::Command {
                id: cmd_id.clone(),
                name: "cli_run".to_string(),
                description: format!("CLI run: {} {}", command, args.join(" ")),
                executable: command,
                args,
                env: std::collections::HashMap::new(),
                working_dir: None,
                output_format: command::CommandOutputFormat::PlainText,
            };
            app.command_manager.execute_command(cmd).await?;

            // Wait for command to complete (simplified for CLI mode)
            let mut rx = app.command_event_rx;
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Output { data, .. } => {
                        print!("{}", String::from_utf8_lossy(&data));
                    }
                    CommandEvent::Completed { id, exit_code } => {
                        if id == cmd_id {
                            log::info!("Headless command completed with exit code {:?}", exit_code);
                            break;
                        }
                    }
                    CommandEvent::Error { id, message } => {
                        if id == cmd_id {
                            log::error!("Headless command error: {}", message);
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        Some(cli::Commands::Config { action }) => {
            let config_manager = app.config_manager.clone();
            match action {
                cli::ConfigCommands::Show => {
                    let prefs = config_manager.get_preferences().await;
                    println!("Current Preferences:\n{:#?}", prefs);
                    let current_theme = config_manager.get_current_theme().await?;
                    println!("\nCurrent Theme: {}", current_theme.name);
                }
                cli::ConfigCommands::Set { key, value } => {
                    let mut prefs = config_manager.get_preferences().await;
                    // This is a simplified example; a real implementation would use reflection or a match statement
                    // to update specific fields based on `key`.
                    println!("Attempting to set {} = {} (Not fully implemented)", key, value);
                    // Example: if key == "terminal_font_size" { prefs.terminal_font_size = value.parse()?; }
                    config_manager.update_preferences(prefs).await?;
                    println!("Configuration updated (simulated).");
                }
                cli::ConfigCommands::Edit => {
                    println!("Opening configuration file in default editor (Not implemented).");
                    // In a real app, you'd use `edit::edit_file(Preferences::path())`
                }
            }
        }
        Some(cli::Commands::Ai { action }) => {
            let assistant = app.ai_assistant.clone(); // Use the new assistant
            match action {
                cli::AiCommands::Chat { message } => {
                    println!("Sending message to AI: {}", message);
                    let mut assistant_lock = assistant.write().await;
                    match assistant_lock.stream_chat(&message).await { // Use stream_chat
                        Ok(mut rx) => {
                            while let Some(msg) = rx.recv().await {
                                match msg.role.as_str() {
                                    "assistant" => print!("{}", msg.content.unwrap_or_default()),
                                    "tool_calls" => println!("\nAI Tool Call: {:?}", msg.tool_calls),
                                    _ => {}
                                }
                            }
                            println!("\n"); // Newline after stream
                        }
                        Err(e) => eprintln!("Error from AI: {}", e),
                    }
                }
                cli::AiCommands::History => {
                    println!("AI Conversation History:");
                    let assistant_lock = assistant.read().await;
                    for msg in assistant_lock.get_history() {
                        println!("{}: {}", msg.role, msg.content.clone().unwrap_or_default());
                    }
                }
                cli::AiCommands::Reset => {
                    let mut assistant_lock = assistant.write().await;
                    assistant_lock.clear_history();
                    println!("AI conversation reset.");
                }
            }
        }
        Some(cli::Commands::Benchmark) => {
            println!("Running performance benchmarks...");
            let benchmarks = PerformanceBenchmarks::new();
            let results = benchmarks.run_all_benchmarks().await;
            println!("\nBenchmark Results:");
            for res in results {
                println!("  {}: {:.2?} ({} iterations)", res.name, res.duration, res.iterations);
            }
        }
        Some(cli::Commands::Sync { force }) => {
            println!("Triggering cloud sync (force: {})...", force);
            let sync_manager = app.sync_manager.clone();
            tokio::spawn(async move {
                if let Err(e) = sync_manager.trigger_manual_sync(force).await {
                    eprintln!("Sync failed: {}", e);
                }
            });
            // In CLI mode, we might want to wait for sync events or just exit
            println!("Sync initiated. Check logs for progress.");
        }
        Some(cli::Commands::Plugin { action }) => {
            let plugin_manager = app.plugin_manager.clone();
            match action {
                cli::PluginCommands::List => {
                    println!("Installed Plugins:");
                    for plugin in plugin_manager.list_plugins().await {
                        println!("- {} (Version: {})", plugin.name, plugin.version);
                    }
                }
                cli::PluginCommands::Install { source } => {
                    println!("Installing plugin from: {}", source);
                    match plugin_manager.install_plugin(&source).await {
                        Ok(_) => println!("Plugin installed successfully."),
                        Err(e) => eprintln!("Failed to install plugin: {}", e),
                    }
                }
                cli::PluginCommands::Uninstall { name } => {
                    println!("Uninstalling plugin: {}", name);
                    match plugin_manager.uninstall_plugin(&name).await {
                        Ok(_) => println!("Plugin uninstalled successfully."),
                        Err(e) => eprintln!("Failed to uninstall plugin: {}", e),
                    }
                }
                cli::PluginCommands::Update => {
                    println!("Updating all plugins (Not fully implemented).");
                    // plugin_manager.update_all_plugins().await?;
                }
            }
        }
        Some(cli::Commands::Workflow { action }) => {
            let workflow_manager = app.workflow_manager.clone();
            match action {
                cli::WorkflowCommands::List => {
                    println!("Available Workflows:");
                    for workflow in workflow_manager.list_workflows().await {
                        println!("- {} (Description: {})", workflow.name, workflow.description);
                    }
                }
                cli::WorkflowCommands::Run { name, args } => {
                    println!("Running workflow: {} with args: {:?}", name, args);
                    let executor = WorkflowExecutor::new(
                        app.command_manager.clone(),
                        app.virtual_file_system.clone(),
                        app.ai_assistant.clone(), // Pass the new assistant
                        app.resource_manager.clone(),
                        app.plugin_manager.clone(),
                        app.shell_manager.clone(),
                        app.drive_manager.clone(),
                        app.watcher.clone(),
                        app.websocket_server.clone(),
                        app.lpc_engine.clone(),
                        app.mcq_manager.clone(),
                        app.natural_language_detector.clone(),
                        app.syntax_tree_manager.clone(),
                        app.string_offset_manager.clone(),
                        app.sum_tree_manager.clone(),
                        app.fuzzy_match_manager.clone(),
                        app.markdown_parser.clone(),
                        app.language_manager.clone(),
                        app.settings_manager.clone(),
                        app.collaboration_manager.clone(),
                        app.sync_manager.clone(),
                        app.wasm_server.clone(),
                    );
                    match workflow_manager.get_workflow(&name).await {
                        Ok(workflow) => {
                            tokio::spawn(async move {
                                if let Err(e) = executor.execute_workflow(workflow, args).await {
                                    eprintln!("Workflow execution failed: {}", e);
                                }
                            });
                            println!("Workflow '{}' started in background.", name);
                        }
                        Err(e) => eprintln!("Workflow '{}' not found: {}", name, e),
                    }
                }
                cli::WorkflowCommands::Edit { name } => {
                    println!("Opening workflow '{}' for editing (Not implemented).", name);
                }
                cli::WorkflowCommands::Import { source } => {
                    println!("Importing workflow from: {}", source);
                    match workflow_manager.import_workflow(&source).await {
                        Ok(wf_name) => println!("Workflow '{}' imported successfully.", wf_name),
                        Err(e) => eprintln!("Failed to import workflow: {}", e),
                    }
                }
            }
        }
        None => {
            // No subcommand, run GUI by default
            //app.run(app_tx).await?;
            //TODO: Remove iced and use ratatui
        }
    }

    Ok(())
}
