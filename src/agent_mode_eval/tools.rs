use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
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
    pub handler: ToolHandler,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: HashMap<String, ParameterProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: String,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolHandler {
    ExecuteCommand,
    ListFiles,
    ReadFile,
    WriteFile,
    SearchFiles,
    GetSystemInfo,
    NetworkRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
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

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        registry.register_default_tools();
        registry
    }

    fn register_default_tools(&mut self) {
        // Execute command tool
        self.register_tool(Tool {
            name: "execute_command".to_string(),
            description: "Execute a shell command and return the output".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("command".to_string(), ParameterProperty {
                        prop_type: "string".to_string(),
                        description: "The shell command to execute".to_string(),
                        enum_values: None,
                    });
                    props.insert("working_directory".to_string(), ParameterProperty {
                        prop_type: "string".to_string(),
                        description: "Working directory for the command (optional)".to_string(),
                        enum_values: None,
                    });
                    props
                },
                required: vec!["command".to_string()],
            },
            handler: ToolHandler::ExecuteCommand,
        });

        // List files tool
        self.register_tool(Tool {
            name: "list_files".to_string(),
            description: "List files and directories in a given path".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("path".to_string(), ParameterProperty {
                        prop_type: "string".to_string(),
                        description: "Path to list (defaults to current directory)".to_string(),
                        enum_values: None,
                    });
                    props.insert("show_hidden".to_string(), ParameterProperty {
                        prop_type: "boolean".to_string(),
                        description: "Whether to show hidden files".to_string(),
                        enum_values: None,
                    });
                    props
                },
                required: vec![],
            },
            handler: ToolHandler::ListFiles,
        });

        // Read file tool
        self.register_tool(Tool {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("path".to_string(), ParameterProperty {
                        prop_type: "string".to_string(),
                        description: "Path to the file to read".to_string(),
                        enum_values: None,
                    });
                    props.insert("max_lines".to_string(), ParameterProperty {
                        prop_type: "integer".to_string(),
                        description: "Maximum number of lines to read (optional)".to_string(),
                        enum_values: None,
                    });
                    props
                },
                required: vec!["path".to_string()],
            },
            handler: ToolHandler::ReadFile,
        });

        // Write file tool
        self.register_tool(Tool {
            name: "write_file".to_string(),
            description: "Write content to a file".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("path".to_string(), ParameterProperty {
                        prop_type: "string".to_string(),
                        description: "Path to the file to write".to_string(),
                        enum_values: None,
                    });
                    props.insert("content".to_string(), ParameterProperty {
                        prop_type: "string".to_string(),
                        description: "Content to write to the file".to_string(),
                        enum_values: None,
                    });
                    props.insert("append".to_string(), ParameterProperty {
                        prop_type: "boolean".to_string(),
                        description: "Whether to append to the file instead of overwriting".to_string(),
                        enum_values: None,
                    });
                    props
                },
                required: vec!["path".to_string(), "content".to_string()],
            },
            handler: ToolHandler::WriteFile,
        });

        // Get system info tool
        self.register_tool(Tool {
            name: "get_system_info".to_string(),
            description: "Get system information like OS, architecture, etc.".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: HashMap::new(),
                required: vec![],
            },
            handler: ToolHandler::GetSystemInfo,
        });
    }

    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get_available_tools(&self) -> Vec<Tool> {
        self.tools.values().cloned().collect()
    }

    pub async fn execute_tool(&self, tool_call: ToolCall) -> Result<ToolResult, ToolError> {
        let tool = self.tools.get(&tool_call.name)
            .ok_or_else(|| ToolError::ToolNotFound(tool_call.name.clone()))?;

        let result = match &tool.handler {
            ToolHandler::ExecuteCommand => self.execute_command_tool(&tool_call).await,
            ToolHandler::ListFiles => self.list_files_tool(&tool_call).await,
            ToolHandler::ReadFile => self.read_file_tool(&tool_call).await,
            ToolHandler::WriteFile => self.write_file_tool(&tool_call).await,
            ToolHandler::GetSystemInfo => self.get_system_info_tool(&tool_call).await,
            _ => Err(ToolError::NotImplemented(tool_call.name.clone())),
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
        let command = tool_call.arguments["command"].as_str()
            .ok_or(ToolError::InvalidArguments("Missing command".to_string()))?;

        let working_dir = tool_call.arguments.get("working_directory")
            .and_then(|v| v.as_str());

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = AsyncCommand::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = AsyncCommand::new("sh");
            c.args(["-c", command]);
            c
        };

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Ok(format!("Command failed with exit code {:?}\nStdout: {}\nStderr: {}", 
                output.status.code(), stdout, stderr))
        }
    }

    async fn list_files_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let path = tool_call.arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let show_hidden = tool_call.arguments.get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let entries = std::fs::read_dir(path)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata()
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
            
            let file_type = if metadata.is_dir() { "dir" } else { "file" };
            let size = if metadata.is_file() { 
                format!(" ({}B)", metadata.len()) 
            } else { 
                String::new() 
            };

            files.push(format!("{} [{}]{}", name, file_type, size));
        }

        files.sort();
        Ok(files.join("\n"))
    }

    async fn read_file_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let path = tool_call.arguments["path"].as_str()
            .ok_or(ToolError::InvalidArguments("Missing path".to_string()))?;

        let max_lines = tool_call.arguments.get("max_lines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if let Some(max) = max_lines {
            let lines: Vec<&str> = content.lines().take(max).collect();
            let result = lines.join("\n");
            if content.lines().count() > max {
                Ok(format!("{}\n... (truncated, showing first {} lines)", result, max))
            } else {
                Ok(result)
            }
        } else {
            Ok(content)
        }
    }

    async fn write_file_tool(&self, tool_call: &ToolCall) -> Result<String, ToolError> {
        let path = tool_call.arguments["path"].as_str()
            .ok_or(ToolError::InvalidArguments("Missing path".to_string()))?;

        let content = tool_call.arguments["content"].as_str()
            .ok_or(ToolError::InvalidArguments("Missing content".to_string()))?;

        let append = tool_call.arguments.get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if append {
            std::fs::write(path, format!("{}\n{}", 
                std::fs::read_to_string(path).unwrap_or_default(), content))
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        } else {
            std::fs::write(path, content)
                .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        }

        Ok(format!("Successfully {} {} bytes to {}", 
            if append { "appended" } else { "wrote" }, 
            content.len(), 
            path))
    }

    async fn get_system_info_tool(&self, _tool_call: &ToolCall) -> Result<String, ToolError> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let family = std::env::consts::FAMILY;
        
        let current_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(format!(
            "OS: {}\nArchitecture: {}\nFamily: {}\nCurrent Directory: {}\nUser: {}",
            os, arch, family, current_dir, user
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Tool not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(!registry.tools.is_empty());
        assert!(registry.tools.contains_key("execute_command"));
        assert!(registry.tools.contains_key("list_files"));
    }

    #[tokio::test]
    async fn test_system_info_tool() {
        let registry = ToolRegistry::new();
        let tool_call = ToolCall {
            id: "test".to_string(),
            name: "get_system_info".to_string(),
            arguments: serde_json::json!({}),
        };

        let result = registry.execute_tool(tool_call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("OS:"));
    }
}
