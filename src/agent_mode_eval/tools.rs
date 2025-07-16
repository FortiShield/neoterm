use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::fmt;
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use thiserror::Error;

pub type ToolFunction = Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON Schema for parameters
    #[serde(skip)]
    pub function: ToolFunction,
}

impl fmt::Debug for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .field("function", &"Fn(...)") // Cannot print the function itself
            .finish()
    }
}

impl Tool {
    pub fn new(name: String, description: String, parameters: Value, function: ToolFunction) -> Self {
        Tool {
            name,
            description,
            parameters,
            function,
        }
    }

    pub fn to_openai_format(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }

    pub fn to_claude_format(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": self.parameters,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value, // Parsed JSON arguments
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Clone, Debug)]
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, tool: Tool) -> Result<(), String> {
        if self.tools.contains_key(&tool.name) {
            return Err(format!("Tool with name '{}' already registered.", tool.name));
        }
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn get_all_tools(&self) -> Vec<Tool> {
        self.tools.values().cloned().collect()
    }

    pub async fn execute_tool(&self, tool_call: &ToolCall) -> Result<String, String> {
        if let Some(tool) = self.tools.get(&tool_call.name) {
            // Validate arguments against schema (simplified for now)
            // In a real application, you'd use a JSON schema validator here.
            if !tool_call.arguments.is_object() {
                return Err(format!("Invalid arguments for tool '{}': Expected object, got {:?}", tool_call.name, tool_call.arguments));
            }

            (tool.function)(tool_call.arguments.clone()).await
        } else {
            Err(format!("Tool '{}' not found.", tool_call.name))
        }
    }
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Missing required argument: {0}")]
    MissingArgument(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl ToolRegistry {
    pub fn register_default_tools(&mut self) {
        // Execute Command Tool
        self.register_tool(Tool::new(
            "execute_command".to_string(),
            "Executes a shell command and returns its output.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Optional working directory for the command."
                    }
                },
                "required": ["command"]
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let command = args.get("command")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required argument: command".to_string())?;

                    let working_directory = args.get("working_directory")
                        .and_then(|v| v.as_str());

                    let mut cmd = AsyncCommand::new("sh");
                    cmd.arg("-c").arg(command);

                    if let Some(wd) = working_directory {
                        cmd.current_dir(wd);
                    }

                    let output = cmd.output().await
                        .map_err(|e| format!("Execution error: {}", e.to_string()))?;

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    if output.status.success() {
                        Ok(stdout.to_string())
                    } else {
                        Err(format!("Command failed: {}", stderr))
                    }
                })
            }),
        ).unwrap();

        // Read File Tool
        self.register_tool(Tool::new(
            "read_file".to_string(),
            "Reads the content of a file.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file."
                    }
                },
                "required": ["path"]
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let path = args.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required argument: path".to_string())?;

                    fs::read_to_string(path).await
                        .map_err(|e| format!("IO error: {}", e.to_string()))
                })
            }),
        ).unwrap();

        // Write File Tool
        self.register_tool(Tool::new(
            "write_file".to_string(),
            "Writes content to a file.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file."
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file."
                    }
                },
                "required": ["path", "content"]
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let path = args.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required argument: path".to_string())?;

                    let content = args.get("content")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required argument: content".to_string())?;

                    fs::write(path, content).await
                        .map_err(|e| format!("IO error: {}", e.to_string()))?;

                    Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
                })
            }),
        ).unwrap();

        // List Directory Tool
        self.register_tool(Tool::new(
            "list_directory".to_string(),
            "Lists contents of a directory.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to list."
                    },
                    "show_hidden": {
                        "type": "boolean",
                        "description": "Whether to show hidden files."
                    }
                },
                "required": ["path"]
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let path = args.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required argument: path".to_string())?;

                    let show_hidden = args.get("show_hidden")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let mut entries = fs::read_dir(path).await
                        .map_err(|e| format!("IO error: {}", e.to_string()))?;

                    let mut result = Vec::new();
                    while let Some(entry) = entries.next_entry().await
                        .map_err(|e| format!("IO error: {}", e.to_string()))? {
                        
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        
                        if !show_hidden && file_name.starts_with('.') {
                            continue;
                        }

                        let metadata = entry.metadata().await
                            .map_err(|e| format!("IO error: {}", e.to_string()))?;

                        let file_type = if metadata.is_dir() { "DIR" } else { "FILE" };
                        let size = if metadata.is_file() { 
                            format!(" ({}B)", metadata.len()) 
                        } else { 
                            String::new() 
                        };

                        result.push(format!("{} {}{}", file_type, file_name, size));
                    }

                    Ok(result.join("\n"))
                })
            }),
        ).unwrap();

        // Get System Info Tool
        self.register_tool(Tool::new(
            "get_system_info".to_string(),
            "Gets system information including OS, CPU, memory, etc.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            Box::new(|_| {
                Box::pin(async move {
                    let mut info = Vec::new();
                    
                    // OS Info
                    info.push(format!("OS: {}", std::env::consts::OS));
                    info.push(format!("Architecture: {}", std::env::consts::ARCH));
                    
                    // Current directory
                    if let Ok(current_dir) = std::env::current_dir() {
                        info.push(format!("Current Directory: {}", current_dir.display()));
                    }

                    // Environment variables (selected)
                    if let Ok(user) = std::env::var("USER") {
                        info.push(format!("User: {}", user));
                    }
                    if let Ok(shell) = std::env::var("SHELL") {
                        info.push(format!("Shell: {}", shell));
                    }
                    if let Ok(home) = std::env::var("HOME") {
                        info.push(format!("Home: {}", home));
                    }

                    Ok(info.join("\n"))
                })
            }),
        ).unwrap();

        // Search Files Tool
        self.register_tool(Tool::new(
            "search_files".to_string(),
            "Searches for files matching a pattern.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (glob or regex)."
                    },
                    "directory": {
                        "type": "string",
                        "description": "Directory to search in (default: current)."
                    }
                },
                "required": ["pattern"]
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let pattern = args.get("pattern")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required argument: pattern".to_string())?;

                    let directory = args.get("directory")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");

                    let output = AsyncCommand::new("find")
                        .arg(directory)
                        .arg("-name")
                        .arg(pattern)
                        .output()
                        .await
                        .map_err(|e| format!("Execution error: {}", e.to_string()))?;

                    if output.status.success() {
                        Ok(String::from_utf8_lossy(&output.stdout).to_string())
                    } else {
                        Err(String::from_utf8_lossy(&output.stderr).to_string())
                    }
                })
            }),
        ).unwrap();

        // Git Status Tool
        self.register_tool(Tool::new(
            "git_status".to_string(),
            "Gets git repository status.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "repository_path": {
                        "type": "string",
                        "description": "Path to git repository (default: current directory)."
                    }
                }
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let repo_path = args.get("repository_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");

                    let mut cmd = AsyncCommand::new("git");
                    cmd.arg("status").arg("--porcelain").current_dir(repo_path);

                    let output = cmd.output().await
                        .map_err(|e| format!("Execution error: {}", e.to_string()))?;

                    if output.status.success() {
                        let status = String::from_utf8_lossy(&output.stdout);
                        if status.trim().is_empty() {
                            Ok("Repository is clean".to_string())
                        } else {
                            Ok(status.to_string())
                        }
                    } else {
                        Err(String::from_utf8_lossy(&output.stderr).to_string())
                    }
                })
            }),
        ).unwrap();

        // Process List Tool
        self.register_tool(Tool::new(
            "process_list".to_string(),
            "Lists running processes.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Optional filter for process names."
                    }
                }
            }),
            Box::new(|args| {
                Box::pin(async move {
                    let filter = args.get("filter")
                        .and_then(|v| v.as_str());

                    let mut cmd = AsyncCommand::new("ps");
                    cmd.arg("aux");

                    let output = cmd.output().await
                        .map_err(|e| format!("Execution error: {}", e.to_string()))?;

                    if output.status.success() {
                        let mut result = String::from_utf8_lossy(&output.stdout).to_string();
                        
                        if let Some(filter_term) = filter {
                            let lines: Vec<&str> = result.lines()
                                .filter(|line| line.contains(filter_term))
                                .collect();
                            result = lines.join("\n");
                        }

                        Ok(result)
                    } else {
                        Err(String::from_utf8_lossy(&output.stderr).to_string())
                    }
                })
            }),
        ).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(!registry.tools.is_empty());
        assert!(registry.get_tool("execute_command").is_some());
        assert!(registry.get_tool("read_file").is_some());
    }

    #[test]
    fn test_tool_registration() {
        let mut registry = ToolRegistry::new();
        let initial_count = registry.tools.len();

        let custom_tool = Tool::new(
            "custom_tool".to_string(),
            "A custom tool".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            Box::new(|_| {
                Box::pin(async move {
                    Ok("Custom tool executed".to_string())
                })
            }),
        );

        registry.register_tool(custom_tool).unwrap();
        assert_eq!(registry.tools.len(), initial_count + 1);
        assert!(registry.get_tool("custom_tool").is_some());
    }

    #[tokio::test]
    async fn test_system_info_tool() {
        let registry = ToolRegistry::new();
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "get_system_info".to_string(),
            arguments: serde_json::json!({}),
        };

        let result = registry.execute_tool(&tool_call).await.unwrap();
        assert!(result.contains("OS:"));
        assert!(result.contains("Architecture:"));
    }
}
