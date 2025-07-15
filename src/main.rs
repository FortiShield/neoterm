mod agent_mode_eval;
mod asset_macro;
mod block;
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
mod renderer;
mod resources;
mod serve_wasm;
mod settings;
mod shell;
mod string_offset;
mod sum_tree;
mod syntax_tree;
mod virtual_fs;
mod watcher;
mod websocket;
mod workflows;

// New modules for Phase implementations
mod ui;
mod plugins;
mod collaboration;
mod cloud;
mod performance;

use std::io;
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
use tokio::sync::mpsc;

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
}

impl App {
    fn new() -> Self {
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
        self.block_renderer.render(
