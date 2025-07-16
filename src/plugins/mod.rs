pub mod wasm_runtime;
pub mod lua_engine;
pub mod plugin_manager;
pub mod plugin_api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

pub trait PluginRuntime {
    fn load_plugin(&mut self, plugin: &Plugin) -> Result<(), Box<dyn std::error::Error>>;
    fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn execute_function(&mut self, plugin_id: &str, function: &str, args: &[serde_json::Value]) -> Result<serde_json::Value, Box<dyn std::error::Error>>;
    fn list_functions(&self, plugin_id: &str) -> Vec<String>;
}
