//! This module defines the command-line interface (CLI) for NeoTerm.
//! It uses the `clap` crate to parse arguments and subcommands,
//! allowing for headless operations or scripting.

use clap::{Parser, Subcommand};

/// NeoTerm: A next-generation terminal with AI assistance and advanced features.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Turn on verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Executes a shell command directly without launching the full GUI.
    #[command(arg_required_else_help = true)]
    Exec {
        /// The command to execute.
        #[arg(required = true, index = 1)]
        command: String,
        /// Arguments for the command.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Runs a specific workflow by its name or ID.
    #[command(arg_required_else_help = true)]
    Workflow {
        /// The name or ID of the workflow to run.
        #[arg(required = true, index = 1)]
        name_or_id: String,
        /// Optional arguments to pass to the workflow.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Manages NeoTerm configurations (e.g., get, set, list).
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Runs performance benchmarks and prints results to stdout.
    Benchmark,
    /// Starts the NeoTerm API server in headless mode.
    ApiServer {
        /// The port to listen on.
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Gets the value of a specific configuration key.
    Get {
        /// The configuration key to retrieve (e.g., "ui.theme", "ai.provider").
        key: String,
    },
    /// Sets the value of a specific configuration key.
    Set {
        /// The configuration key to set.
        key: String,
        /// The value to set.
        value: String,
    },
    /// Lists all configuration keys and their current values.
    List,
}

/// Initializes the CLI module.
pub fn init() {
    // This function can be used for any global CLI setup if needed.
    // For now, it's a placeholder.
}
