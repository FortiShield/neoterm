use std::collections::HashMap;
use serde_json::{Value, json};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use tokio::process::Command;
use std::path::PathBuf;
use std::env;
use std::fs;

/// Trait for defining a tool that the AI can use.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> serde_json::Value; // JSON schema for parameters
    async fn execute(&self, args: Value) -> Result<String>;
}

pub struct ToolManager {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    pub async fn register_default_tools(&mut self) -> Result<()> {
        self.register_tool(Box::new(ListFilesTool));
        self.register_tool(Box::new(ReadFileTool));
        self.register_tool(Box::new(WriteFileTool));
        self.register_tool(Box::new(ExecuteCommandTool));
        self.register_tool(Box::new(ChangeDirectoryTool));
        Ok(())
    }

    pub fn get_all_tools_schema(&self) -> serde_json::Value {
        let functions: Vec<serde_json::Value> = self.tools.values()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters(),
                    }
                })
            })
            .collect();
        json!(functions)
    }
}

// --- Concrete Tool Implementations ---

pub struct ListFilesTool;

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str { "list_files" }
    fn description(&self) -> &'static str { "Lists files and directories in a given path." }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to list. Defaults to current directory if not provided."
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"].as_str().unwrap_or(".");
        let path = PathBuf::from(path_str);

        if !path.exists() {
            return Ok(format!("Error: Path '{}' does not exist.", path_str));
        }
        if !path.is_dir() {
            return Ok(format!("Error: Path '{}' is not a directory.", path_str));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let file_name = entry.file_name().into_string().unwrap_or_default();
            let metadata = entry.metadata()?;
            let entry_type = if metadata.is_dir() { "DIR" } else if metadata.is_file() { "FILE" } else { "OTHER" };
            entries.push(format!("{} ({})", file_name, entry_type));
        }
        Ok(entries.join("\n"))
    }
}

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str { "read_file" }
    fn description(&self) -> &'static str { "Reads the content of a specified file." }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"].as_str().ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let path = PathBuf::from(path_str);

        if !path.exists() {
            return Ok(format!("Error: File '{}' does not exist.", path_str));
        }
        if !path.is_file() {
            return Ok(format!("Error: Path '{}' is not a file.", path_str));
        }

        let content = tokio::fs::read_to_string(&path).await?;
        Ok(content)
    }
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str { "write_file" }
    fn description(&self) -> &'static str { "Writes content to a specified file, overwriting if it exists." }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to write."
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file."
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"].as_str().ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let content = args["content"].as_str().ok_or_else(|| anyhow!("Missing 'content' argument"))?;
        let path = PathBuf::from(path_str);

        tokio::fs::write(&path, content).await?;
        Ok(format!("Successfully wrote to file: {}", path_str))
    }
}

pub struct ExecuteCommandTool;

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn name(&self) -> &'static str { "execute_command" }
    fn description(&self) -> &'static str { "Executes a shell command and returns its stdout and stderr." }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute."
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Arguments for the command."
                },
                "dir": {
                    "type": "string",
                    "description": "The working directory for the command. Defaults to current."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let command_str = args["command"].as_str().ok_or_else(|| anyhow!("Missing 'command' argument"))?;
        let command_args: Vec<String> = args["args"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        let dir = args["dir"].as_str();

        let mut cmd = Command::new(command_str);
        cmd.args(&command_args);
        if let Some(d) = dir {
            cmd.current_dir(d);
        }

        let output = cmd.output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(format!("Command executed successfully.\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr))
        } else {
            Err(anyhow!("Command failed with exit code {:?}.\nSTDOUT:\n{}\nSTDERR:\n{}", output.status.code(), stdout, stderr))
        }
    }
}

pub struct ChangeDirectoryTool;

#[async_trait]
impl Tool for ChangeDirectoryTool {
    fn name(&self) -> &'static str { "change_directory" }
    fn description(&self) -> &'static str { "Changes the current working directory of the shell environment." }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to change to."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"].as_str().ok_or_else(|| anyhow!("Missing 'path' argument"))?;
        let path = PathBuf::from(path_str);

        if !path.exists() {
            return Ok(format!("Error: Path '{}' does not exist.", path_str));
        }
        if !path.is_dir() {
            return Ok(format!("Error: Path '{}' is not a directory.", path_str));
        }

        match env::set_current_dir(&path) {
            Ok(_) => Ok(format!("Successfully changed directory to: {}", path.display())),
            Err(e) => Err(anyhow!("Failed to change directory to '{}': {:?}", path.display(), e)),
        }
    }
}
