//! This module defines the command-line interface (CLI) for NeoTerm.
//! It uses the `clap` crate to parse arguments and subcommands,
//! allowing for headless operations or scripting.

use clap::{Parser, Subcommand};
use log;

/// NeoTerm: A modern terminal emulator with AI integration and advanced features.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Turn on verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the NeoTerm GUI
    Gui {
        /// Initial directory to open
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Run a command in headless mode
    Run {
        /// Command to execute
        command: String,
        /// Arguments for the command
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Manage NeoTerm configurations
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Interact with the AI assistant
    Ai {
        #[command(subcommand)]
        action: AiCommands,
    },
    /// Run performance benchmarks
    Benchmark,
    /// Sync data with cloud services
    Sync {
        /// Force a full sync
        #[arg(short, long)]
        force: bool,
    },
    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// Manage workflows
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        key: String,
        value: String,
    },
    /// Edit configuration file in default editor
    Edit,
}

#[derive(Subcommand, Debug)]
pub enum AiCommands {
    /// Send a message to the AI assistant
    Chat {
        message: String,
    },
    /// Get the conversation history
    History,
    /// Reset the AI conversation
    Reset,
}

#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// List installed plugins
    List,
    /// Install a plugin from a path or URL
    Install {
        source: String,
    },
    /// Uninstall a plugin
    Uninstall {
        name: String,
    },
    /// Update all installed plugins
    Update,
}

#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    /// List available workflows
    List,
    /// Run a specific workflow
    Run {
        name: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Edit a workflow file in default editor
    Edit {
        name: String,
    },
    /// Import a workflow from a path or URL
    Import {
        source: String,
    },
}

/// Initializes the CLI module.
pub fn init() {
    log::info!("CLI module initialized.");
}
