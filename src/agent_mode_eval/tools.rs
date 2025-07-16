use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema for parameters
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
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
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register_default_tools();
        registry
    }

    fn register_default_tools(&mut self) {
        // Execute Command Tool
        self.register_tool(ToolDefinition {
            name: "execute_command".to_string(),
            description: "Executes a shell command and returns its output.".to_string(),
            parameters: serde_json::json!({
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
        });

        // Read File Tool
        self.register_tool(ToolDefinition {
            name: "read_file".to_string(),
            description: "Reads the content of a file.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file."
                    }
                },
                "required": ["path"]
            }),
        });

        // Write File Tool
        self.register_tool(ToolDefinition {
            name: "write_file".to_string(),
            description: "Writes content to a file.".to_string(),
            parameters: serde_json::json!({
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
        });

        // List Directory Tool
        self.register_tool(ToolDefinition {
            name: "list_directory".to_string(),
            description: "Lists contents of a directory.".to_string(),
            parameters: serde_json::json!({
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
        });

        // Get System Info Tool
        self.register_tool(ToolDefinition {
            name: "get_system_info".to_string(),
            description: "Gets system information including OS, CPU, memory, etc.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        });

        // Search Files Tool
        self.register_tool(ToolDefinition {
            name: "search_files".to_string(),
            description: "Searches for files matching a pattern.".to_string(),
            parameters: serde_json::json!({
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
        });

        // Git Status Tool
        self.register_tool(ToolDefinition {
            name: "git_status".to_string(),
            description: "Gets git repository status.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "repository_path": {
                        "type": "string",
                        "description": "Path to git repository (default: current directory)."
                    }
                }
            }),
        });

        // Process List Tool
        self.register_tool(ToolDefinition {
            name: "process_list".to_string(),
            description: "Lists running processes.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Optional filter for process names."
                    }
                }
            }),
        });
    }

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    pub fn get_available_tools(&self) -> Vec<ToolDefinition> {
        self.tools.values().cloned().collect()
    }

    pub async fn execute_tool(&self, tool_call: ToolCall) -> Result<ToolResult, ToolError> {
        let tool = self.get_tool(&tool_call.name)
            .ok_or_else(|| ToolError::ToolNotFound(tool_call.name.clone()))?;

        let result = match tool.name.as_str() {
            "execute_command" => self.execute_command_tool(&tool_call).await,
            "read_file" => self.read_file_tool(&tool_call).await,
            "write_file" => self.write_file_tool(&tool_call).await,
            "list_directory" => self.list_directory_tool(&tool_call).await,
            "get_system_info" => self.get_system_info_tool(&tool_call).await,
            "search_files" => self.search_files_tool(&tool_call).await,
            "git_status" => self.git_status_tool(&tool_call).await,
            "process_list" => self.process_list_tool(&tool_call).await,
            _ => Err(ToolError::ToolNotFound(tool_call.name.clone())),
        };

        match result {
            Ok(output) => Ok(ToolResult {
                tool_call_id: tool_call.name.clone(),
                success: true,
                output,
                error: None,
            }),
            Err(error) => Ok(ToolResult {
                tool_call_id: tool_call.name.clone(),
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
            }),
        }
    }

    async fn execute_command_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let command = tool_call.arguments.get("command")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::MissingArgument("command".to_string()))?;

        let working_directory = tool_call.arguments.get("working_directory")
            .and_then(|v| v.as_str());

        let mut cmd = AsyncCommand::new("sh");
        cmd.arg("-c").arg(command);

        if let Some(wd) = working_directory {
            cmd.current_dir(wd);
        }

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(ToolError::ExecutionError(format!("Command failed: {}", stderr)))
        }
    }

    async fn read_file_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let path = tool_call.arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::MissingArgument("path".to_string()))?;

        fs::read_to_string(path).await
            .map_err(|e| ToolError::IoError(e.to_string()))
    }

    async fn write_file_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let path = tool_call.arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::MissingArgument("path".to_string()))?;

        let content = tool_call.arguments.get("content")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::MissingArgument("content".to_string()))?;

        fs::write(path, content).await
            .map_err(|e| ToolError::IoError(e.to_string()))?;

        Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
    }

    async fn list_directory_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let path = tool_call.arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::MissingArgument("path".to_string()))?;

        let show_hidden = tool_call.arguments.get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut entries = fs::read_dir(path).await
            .map_err(|e| ToolError::IoError(e.to_string()))?;

        let mut result = Vec::new();
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ToolError::IoError(e.to_string()))? {
            
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            if !show_hidden && file_name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().await
                .map_err(|e| ToolError::IoError(e.to_string()))?;

            let file_type = if metadata.is_dir() { "DIR" } else { "FILE" };
            let size = if metadata.is_file() { 
                format!(" ({}B)", metadata.len()) 
            } else { 
                String::new() 
            };

            result.push(format!("{} {}{}", file_type, file_name, size));
        }

        Ok(result.join("\n"))
    }

    async fn get_system_info_tool(&self, _tool_call: &ToolCall) -> Result<String, ToolError> {
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
    }

    async fn search_files_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let pattern = tool_call.arguments.get("pattern")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::MissingArgument("pattern".to_string()))?;

        let directory = tool_call.arguments.get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let output = AsyncCommand::new("find")
            .arg(directory)
            .arg("-name")
            .arg(pattern)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(ToolError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
    }

    async fn git_status_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let repo_path = tool_call.arguments.get("repository_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let mut cmd = AsyncCommand::new("git");
        cmd.arg("status").arg("--porcelain").current_dir(repo_path);

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if output.status.success() {
            let status = String::from_utf8_lossy(&output.stdout);
            if status.trim().is_empty() {
                Ok("Repository is clean".to_string())
            } else {
                Ok(status.to_string())
            }
        } else {
            Err(ToolError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
    }

    async fn process_list_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let filter = tool_call.arguments.get("filter")
            .and_then(|v| v.as_str());

        let mut cmd = AsyncCommand::new("ps");
        cmd.arg("aux");

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

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
            Err(ToolError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
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

        let custom_tool = ToolDefinition {
            name: "custom_tool".to_string(),
            description: "A custom tool".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        };

        registry.register_tool(custom_tool);
        assert_eq!(registry.tools.len(), initial_count + 1);
        assert!(registry.get_tool("custom_tool").is_some());
    }

    #[tokio::test]
    async fn test_system_info_tool() {
        let registry = ToolRegistry::new();
        let tool_call = ToolCall {
            name: "get_system_info".to_string(),
            arguments: serde_json::json!({}),
        };

        let result = registry.execute_tool(tool_call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("OS:"));
        assert!(result.output.contains("Architecture:"));
    }
}
