use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tokio::fs;
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub r#type: String,
    pub properties: HashMap<String, ParameterProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterProperty {
    pub r#type: String,
    pub description: String,
    pub r#enum: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolFunction {
    ExecuteCommand,
    ReadFile,
    WriteFile,
    ListDirectory,
    GetSystemInfo,
    SearchFiles,
    GitStatus,
    ProcessList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
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
        self.register_tool(Tool {
            name: "execute_command".to_string(),
            description: "Execute a shell command and return the output".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("command".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "The shell command to execute".to_string(),
                        r#enum: None,
                    });
                    props.insert("working_directory".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Optional working directory for the command".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec!["command".to_string()],
            },
            function: ToolFunction::ExecuteCommand,
        });

        // Read File Tool
        self.register_tool(Tool {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("path".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Path to the file to read".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec!["path".to_string()],
            },
            function: ToolFunction::ReadFile,
        });

        // Write File Tool
        self.register_tool(Tool {
            name: "write_file".to_string(),
            description: "Write content to a file".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("path".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Path to the file to write".to_string(),
                        r#enum: None,
                    });
                    props.insert("content".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Content to write to the file".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec!["path".to_string(), "content".to_string()],
            },
            function: ToolFunction::WriteFile,
        });

        // List Directory Tool
        self.register_tool(Tool {
            name: "list_directory".to_string(),
            description: "List contents of a directory".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("path".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Path to the directory to list".to_string(),
                        r#enum: None,
                    });
                    props.insert("show_hidden".to_string(), ParameterProperty {
                        r#type: "boolean".to_string(),
                        description: "Whether to show hidden files".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec!["path".to_string()],
            },
            function: ToolFunction::ListDirectory,
        });

        // Get System Info Tool
        self.register_tool(Tool {
            name: "get_system_info".to_string(),
            description: "Get system information including OS, CPU, memory, etc.".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: HashMap::new(),
                required: vec![],
            },
            function: ToolFunction::GetSystemInfo,
        });

        // Search Files Tool
        self.register_tool(Tool {
            name: "search_files".to_string(),
            description: "Search for files matching a pattern".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("pattern".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Search pattern (glob or regex)".to_string(),
                        r#enum: None,
                    });
                    props.insert("directory".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Directory to search in (default: current)".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec!["pattern".to_string()],
            },
            function: ToolFunction::SearchFiles,
        });

        // Git Status Tool
        self.register_tool(Tool {
            name: "git_status".to_string(),
            description: "Get git repository status".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("repository_path".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Path to git repository (default: current directory)".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec![],
            },
            function: ToolFunction::GitStatus,
        });

        // Process List Tool
        self.register_tool(Tool {
            name: "process_list".to_string(),
            description: "List running processes".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("filter".to_string(), ParameterProperty {
                        r#type: "string".to_string(),
                        description: "Optional filter for process names".to_string(),
                        r#enum: None,
                    });
                    props
                },
                required: vec![],
            },
            function: ToolFunction::ProcessList,
        });
    }

    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn get_available_tools(&self) -> Vec<Tool> {
        self.tools.values().cloned().collect()
    }

    pub async fn execute_tool(&self, tool_call: ToolCall) -> Result<ToolResult, ToolError> {
        let tool = self.get_tool(&tool_call.name)
            .ok_or_else(|| ToolError::ToolNotFound(tool_call.name.clone()))?;

        let result = match &tool.function {
            ToolFunction::ExecuteCommand => self.execute_command_tool(&tool_call).await,
            ToolFunction::ReadFile => self.read_file_tool(&tool_call).await,
            ToolFunction::WriteFile => self.write_file_tool(&tool_call).await,
            ToolFunction::ListDirectory => self.list_directory_tool(&tool_call).await,
            ToolFunction::GetSystemInfo => self.get_system_info_tool(&tool_call).await,
            ToolFunction::SearchFiles => self.search_files_tool(&tool_call).await,
            ToolFunction::GitStatus => self.git_status_tool(&tool_call).await,
            ToolFunction::ProcessList => self.process_list_tool(&tool_call).await,
        };

        match result {
            Ok(output) => Ok(ToolResult {
                tool_call_id: tool_call.id,
                success: true,
                output,
                error: None,
            }),
            Err(error) => Ok(ToolResult {
                tool_call_id: tool_call.id,
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

#[derive(Debug, thiserror::Error)]
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

        let custom_tool = Tool {
            name: "custom_tool".to_string(),
            description: "A custom tool".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: HashMap::new(),
                required: vec![],
            },
            function: ToolFunction::GetSystemInfo,
        };

        registry.register_tool(custom_tool);
        assert_eq!(registry.tools.len(), initial_count + 1);
        assert!(registry.get_tool("custom_tool").is_some());
    }

    #[tokio::test]
    async fn test_system_info_tool() {
        let registry = ToolRegistry::new();
        let tool_call = ToolCall {
            id: "test_id".to_string(),
            name: "get_system_info".to_string(),
            arguments: HashMap::new(),
        };

        let result = registry.execute_tool(tool_call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("OS:"));
        assert!(result.output.contains("Architecture:"));
    }
}
