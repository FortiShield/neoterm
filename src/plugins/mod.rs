pub mod wasm_runtime;
pub mod lua_engine;
pub mod plugin_manager;
pub mod plugin_api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use anyhow::Result;
use log::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub plugin_type: PluginType,
    pub entry_point: String,
    pub permissions: Vec<Permission>,
    pub dependencies: Vec<String>,
    pub config_schema: Option<serde_json::Value>,
    pub install_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    WASM,
    Lua,
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    FileSystem(FileSystemPermission),
    Network(NetworkPermission),
    Terminal(TerminalPermission),
    System(SystemPermission),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemPermission {
    Read(PathBuf),
    Write(PathBuf),
    Execute(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPermission {
    HttpRequest(String), // URL pattern
    WebSocket(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalPermission {
    ExecuteCommand,
    ReadHistory,
    ModifyPrompt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemPermission {
    EnvironmentVariables,
    ProcessList,
    SystemInfo,
}

/// Represents a plugin event that can be sent from a plugin to the main application.
#[derive(Debug, Clone)]
pub enum PluginEvent {
    StatusUpdate(String, String), // (Plugin Name, Status Message)
    CommandExecuted(String, String), // (Plugin Name, Command Output)
    Data(String, serde_json::Value), // (Plugin Name, Arbitrary Data)
    Error(String, String), // (Plugin Name, Error Message)
}

/// Configuration for a specific plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginConfig {
    Lua {
        script_path: String,
        #[serde(default)]
        enabled: bool,
    },
    Wasm {
        wasm_path: String,
        #[serde(default)]
        enabled: bool,
    },
    // Add other plugin types here
}

/// A trait for defining common behavior for plugins.
/// Plugins can be implemented in different languages/runtimes (Lua, WebAssembly, native Rust).
pub trait Plugin: Send + Sync {
    /// Returns the name of the plugin.
    fn name(&self) -> &str;

    /// Initializes the plugin, potentially loading scripts or WASM modules.
    fn initialize(&mut self, event_sender: mpsc::UnboundedSender<PluginEvent>) -> Result<(), String>;

    /// Executes a specific function or command within the plugin.
    async fn execute_function(&self, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String>;

    /// Starts any background tasks for the plugin.
    fn start_background_tasks(&self, event_sender: mpsc::UnboundedSender<PluginEvent>);
}

/// Manages all active plugins.
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
    event_sender: mpsc::UnboundedSender<PluginEvent>,
}

impl PluginManager {
    pub fn new(event_sender: mpsc::UnboundedSender<PluginEvent>) -> Self {
        Self {
            plugins: HashMap::new(),
            event_sender,
        }
    }

    /// Registers a new plugin.
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<(), String> {
        let name = plugin.name().to_string();
        if self.plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' already registered.", name));
        }
        self.plugins.insert(name, plugin);
        Ok(())
    }

    /// Initializes all registered plugins.
    pub fn initialize_all(&mut self) {
        for (name, plugin) in self.plugins.iter_mut() {
            match plugin.initialize(self.event_sender.clone()) {
                Ok(_) => println!("Plugin '{}' initialized successfully.", name),
                Err(e) => eprintln!("Failed to initialize plugin '{}': {}", name, e),
            }
        }
    }

    /// Starts background tasks for all registered plugins.
    pub fn start_all_background_tasks(&self) {
        for plugin in self.plugins.values() {
            plugin.start_background_tasks(self.event_sender.clone());
        }
    }

    /// Executes a function in a specific plugin.
    pub async fn execute_plugin_function(&self, plugin_name: &str, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        if let Some(plugin) = self.plugins.get(plugin_name) {
            plugin.execute_function(function_name, args).await
        } else {
            Err(format!("Plugin '{}' not found.", plugin_name))
        }
    }
}

pub fn init() {
    info!("Plugins module initialized.");
}
