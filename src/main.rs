use iced::{executor, Application, Command, Element, Settings, Theme};
use iced::widget::{column, container, scrollable, text_input, button, row, text};
use std::path::PathBuf;
use tokio::sync::mpsc;
use uuid::Uuid;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io; // Added for terminal setup

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
mod sum_tree; // Added missing module import
mod workflows; // Added missing module import
mod lpc; // Added missing module import
mod mcq; // Added missing module import
mod markdown_parser; // Added missing module import

use block::{Block, BlockContent};
use shell::ShellManager;
use input::EnhancedTextInput;
use agent_mode_eval::{AgentMode, AgentConfig, AgentMessage};
use config::AppConfig;
use crate::{
    ui::{
        collapsible_block::{CollapsibleBlockRenderer, CollapsibleBlock, BlockType},
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
                        
                        // Get active environment profile variables
                        let env_vars = self.config.env_profiles.active_profile
                            .as_ref()
                            .and_then(|name| self.config.env_profiles.profiles.get(name))
                            .map(|profile| profile.variables.clone());

                        Command::perform(
                            self.shell_manager.execute_command(command, env_vars), // Pass env_vars
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

        // Display active environment profile
        let active_profile_name = self.config.env_profiles.active_profile.as_deref().unwrap_or("None");
        let env_profile_indicator = text(format!("Env: {}", active_profile_name)).size(14);

        row![agent_button, settings_button, env_profile_indicator]
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
                            // Get active environment profile variables for rerun
                            let env_vars = self.config.env_profiles.active_profile
                                .as_ref()
                                .and_then(|name| self.config.env_profiles.profiles.get(name))
                                .map(|profile| profile.variables.clone());

                            Command::perform(
                                self.shell_manager.execute_command(command, env_vars), // Pass env_vars
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

#[derive(Debug)]
enum AppMode {
    Normal,
    CommandPalette,
    AIChat,
    WorkflowDebug,
    Settings,
}

struct App {
    mode: AppMode,
    block_renderer: CollapsibleBlockRenderer,
    command_palette: CommandPalette,
    ai_sidebar: AISidebar,
    pty_manager: PtyManager,
    workflow_debugger: WorkflowDebugger,
    plugin_manager: PluginManager,
    session_manager: SessionSharingManager,
    sync_manager: Option<CloudSyncManager>,
    input_buffer: String,
    should_quit: bool,
    config: AppConfig, // Added AppConfig to App struct
}

impl App {
    fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default(); // Load config here
        Self {
            mode: AppMode::Normal,
            block_renderer: CollapsibleBlockRenderer::new(),
            command_palette: CommandPalette::new(),
            ai_sidebar: AISidebar::new(),
            pty_manager: PtyManager::new(),
            workflow_debugger: WorkflowDebugger::new(),
            plugin_manager: PluginManager::new(),
            session_manager: SessionSharingManager::new(),
            sync_manager: None,
            input_buffer: String::new(),
            should_quit: false,
            config, // Initialize config
        }
    }

    async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize cloud sync if configured
        if let (Ok(api_url), Ok(api_key)) = (std::env::var("NEOTERM_API_URL"), std::env::var("NEOTERM_API_KEY")) {
            self.sync_manager = Some(CloudSyncManager::new(api_url, api_key));
        }

        // Add some sample blocks for demonstration
        self.add_sample_blocks();

        loop {
            terminal.draw(|f| self.ui(f))?;

            // Process AI responses
            self.ai_sidebar.process_ai_response();

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.mode {
                        AppMode::Normal => self.handle_normal_mode_input(key.code).await?,
                        AppMode::CommandPalette => self.handle_command_palette_input(key.code).await?,
                        AppMode::AIChat => self.handle_ai_chat_input(key.code).await?,
                        AppMode::WorkflowDebug => self.handle_workflow_debug_input(key.code).await?,
                        AppMode::Settings => self.handle_settings_input(key.code).await?,
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn ui<B: Backend>(&mut self, f: &mut ratatui::Frame<B>) {
        let size = f.size();

        // Main layout
        let chunks = if self.ai_sidebar.is_open {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ])
                .split(size)
        } else {
            vec![size]
        };

        let main_area = chunks[0];

        // Split main area into blocks and input
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(main_area);

        // Render blocks
        self.block_renderer.render(f, main_chunks[0]);

        // Render input area
        let input_block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title("Input");
        
        let input_paragraph = ratatui::widgets::Paragraph::new(self.input_buffer.as_str())
            .block(input_block)
            .wrap(ratatui::widgets::Wrap { trim: true });
        
        f.render_widget(input_paragraph, main_chunks[1]);

        // Render AI sidebar if open
        if self.ai_sidebar.is_open {
            self.ai_sidebar.render(f, chunks[1]);
        }

        // Render command palette if open
        self.command_palette.render(f, size);

        // Show mode indicator and active environment profile
        let mode_text = match self.mode {
            AppMode::Normal => "NORMAL",
            AppMode::CommandPalette => "COMMAND",
            AppMode::AIChat => "AI CHAT",
            AppMode::WorkflowDebug => "DEBUG",
            AppMode::Settings => "SETTINGS",
        };

        let active_profile_name = self.config.env_profiles.active_profile.as_deref().unwrap_or("None");
        let status_text = format!("{} | Env: {}", mode_text, active_profile_name);

        let status_paragraph = ratatui::widgets::Paragraph::new(status_text)
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
            .alignment(ratatui::layout::Alignment::Right);
        
        let status_area = ratatui::layout::Rect {
            x: size.width.saturating_sub(status_text.len() as u16),
            y: 0,
            width: status_text.len() as u16,
            height: 1,
        };
        
        f.render_widget(status_paragraph, status_area);
    }

    async fn handle_normal_mode_input(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(' ') => self.block_renderer.toggle_selected_block(),
            KeyCode::Up => self.block_renderer.move_selection_up(),
            KeyCode::Down => self.block_renderer.move_selection_down(),
            KeyCode::Char('p') => {
                self.command_palette.toggle();
                self.mode = if self.command_palette.is_open {
                    AppMode::CommandPalette
                } else {
                    AppMode::Normal
                };
            }
            KeyCode::Char('a') => {
                self.ai_sidebar.toggle();
                if self.ai_sidebar.is_open {
                    self.mode = AppMode::AIChat;
                }
            }
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.execute_command().await?;
                }
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::F(1) => {
                // Run performance benchmarks
                let mut benchmarks = PerformanceBenchmarks::new();
                let suite = benchmarks.run_all_benchmarks().await;
                
                let mut benchmark_block = CollapsibleBlock::new(
                    "Performance Benchmarks".to_string(),
                    BlockType::Info
                );
                benchmark_block.add_line(benchmarks.get_performance_summary());
                self.block_renderer.add_block(benchmark_block);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_command_palette_input(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        match key {
            KeyCode::Esc => {
                self.command_palette.toggle();
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                if let Some(action) = self.command_palette.execute_selected() {
                    self.execute_command_action(action).await?;
                }
                self.mode = AppMode::Normal;
            }
            KeyCode::Up => self.command_palette.move_selection_up(),
            KeyCode::Down => self.command_palette.move_selection_down(),
            KeyCode::Char(c) => self.command_palette.add_char(c),
            KeyCode::Backspace => self.command_palette.remove_char(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_ai_chat_input(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                self.ai_sidebar.send_message()?;
            }
            KeyCode::Char(c) => self.ai_sidebar.add_char(c),
            KeyCode::Backspace => self.ai_sidebar.remove_char(),
            KeyCode::PageUp => self.ai_sidebar.scroll_up(),
            KeyCode::PageDown => self.ai_sidebar.scroll_down(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_workflow_debug_input(&mut self, _key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        // Workflow debugger input handling would go here
        Ok(())
    }

    async fn handle_settings_input(&mut self, _key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        // Settings input handling would go here
        Ok(())
    }

    async fn execute_command(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let command = self.input_buffer.clone();
        self.input_buffer.clear();

        // Add command to blocks
        let mut command_block = CollapsibleBlock::new(
            format!("$ {}", command),
            BlockType::Command
        );
        self.block_renderer.add_block(command_block);

        // Parse command
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let cmd = parts[0];
        let args = parts[1..].to_vec();

        // Get active environment profile variables
        let env_vars = self.config.env_profiles.active_profile
            .as_ref()
            .and_then(|name| self.config.env_profiles.profiles.get(name))
            .map(|profile| profile.variables.clone());

        // Execute command via PTY
        let mut output_receiver = self.pty_manager.execute_command(cmd, args, env_vars).await?;
        
        let mut output_block = CollapsibleBlock::new(
            "Output".to_string(),
            BlockType::Output
        );

        // Process command output
        tokio::spawn(async move {
            while let Some(output) = output_receiver.recv().await {
                match output.status {
                    CommandStatus::Running => {
                        if !output.stdout.is_empty() {
                            // In a real implementation, you'd send this back to the main thread
                            println!("STDOUT: {}", output.stdout);
                        }
                        if !output.stderr.is_empty() {
                            println!("STDERR: {}", output.stderr);
                        }
                    }
                    CommandStatus::Completed(exit_code) => {
                        println!("Command completed with exit code: {}", exit_code);
                        break;
                    }
                    CommandStatus::Failed(error) => {
                        println!("Command failed: {}", error);
                        break;
                    }
                    CommandStatus::Killed => {
                        println!("Command was killed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn execute_command_action(&mut self, action: CommandAction) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            CommandAction::ToggleCollapse => {
                self.block_renderer.toggle_selected_block();
            }
            CommandAction::NewTab => {
                // Implement new tab functionality
                let mut info_block = CollapsibleBlock::new(
                    "New Tab".to_string(),
                    BlockType::Info
                );
                info_block.add_line("New tab functionality would be implemented here".to_string());
                self.block_renderer.add_block(info_block);
            }
            CommandAction::CloseTab => {
                // Implement close tab functionality
            }
            CommandAction::SearchBlocks => {
                // Implement block search functionality
            }
            CommandAction::ClearHistory => {
                self.block_renderer = CollapsibleBlockRenderer::new();
            }
            CommandAction::AIChat => {
                self.ai_sidebar.toggle();
                if self.ai_sidebar.is_open {
                    self.mode = AppMode::AIChat;
                }
            }
            CommandAction::AIExplain => {
                // Get current block content and ask AI to explain
                if let Some(selected_block) = self.block_renderer.blocks.get(self.block_renderer.selected_index) {
                    let context = format!("Please explain this terminal output: {}", selected_block.content.join("\n"));
                    self.ai_sidebar.inject_terminal_context(&context);
                    self.ai_sidebar.toggle();
                    self.mode = AppMode::AIChat;
                }
            }
            CommandAction::OpenSettings => {
                self.mode = AppMode::Settings;
            }
            CommandAction::ShowHelp => {
                let mut help_block = CollapsibleBlock::new(
                    "Help".to_string(),
                    BlockType::Info
                );
                help_block.add_line("Keybindings:".to_string());
                help_block.add_line("  q - Quit".to_string());
                help_block.add_line("  Space - Toggle block collapse".to_string());
                help_block.add_line("  Up/Down - Navigate blocks".to_string());
                help_block.add_line("  p - Open command palette".to_string());
                help_block.add_line("  a - Toggle AI sidebar".to_string());
                help_block.add_line("  F1 - Run performance benchmarks".to_string());
                self.block_renderer.add_block(help_block);
            }
            CommandAction::RunWorkflow(workflow_name) => {
                let mut workflow_block = CollapsibleBlock::new(
                    format!("Running workflow: {}", workflow_name),
                    BlockType::Info
                );
                workflow_block.add_line("Workflow execution would be implemented here".to_string());
                self.block_renderer.add_block(workflow_block);
            }
            CommandAction::Custom(command) => {
                self.input_buffer = command;
                self.execute_command().await?;
            }
        }
        Ok(())
    }

    fn add_sample_blocks(&mut self) {
        let mut welcome_block = CollapsibleBlock::new(
            "Welcome to NeoPilot Terminal".to_string(),
            BlockType::Info
        );
        welcome_block.add_line("This is a next-generation terminal with AI assistance.".to_string());
        welcome_block.add_line("Press 'p' to open the command palette.".to_string());
        welcome_block.add_line("Press 'a' to toggle the AI sidebar.".to_string());
        welcome_block.add_line("Press 'F1' to run performance benchmarks.".to_string());
        
        let mut sample_command = CollapsibleBlock::new(
            "$ ls -la".to_string(),
            BlockType::Command
        );
        
        let mut sample_output = CollapsibleBlock::new(
            "Output".to_string(),
            BlockType::Output
        );
        sample_output.add_line("total 48".to_string());
        sample_output.add_line("drwxr-xr-x  8 user user 4096 Jan 15 10:30 .".to_string());
        sample_output.add_line("drwxr-xr-x  3 user user 4096 Jan 15 10:25 ..".to_string());
        sample_output.add_line("-rw-r--r--  1 user user 1234 Jan 15 10:30 README.md".to_string());
        
        self.block_renderer.add_block(welcome_block);
        self.block_renderer.add_block(sample_command);
        self.block_renderer.add_block(sample_output);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let res = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
