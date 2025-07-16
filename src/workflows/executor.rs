use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid; // Import Uuid for generating prompt IDs

use super::workflow::{Workflow, WorkflowStep, WorkflowOutputFormat, WorkflowStepType};
use crate::command::{CommandManager, Command};
use crate::virtual_fs::VirtualFileSystem;
use crate::agent_mode_eval::{AgentModeEvaluator, ai_client::ChatMessage, AgentMessage}; // Import AgentMessage
use crate::resources::ResourceManager;
use crate::plugins::plugin_manager::PluginManager;
use crate::shell::ShellManager;
use crate::drive::DriveManager;
use crate::watcher::Watcher;
use crate::websocket::WebSocketServer;
use crate::lpc::LpcEngine;
use crate::mcq::McqManager;
use crate::natural_language_detection::NaturalLanguageDetector;
use crate::syntax_tree::SyntaxTreeManager;
use crate::string_offset::StringOffsetManager;
use crate::sum_tree::SumTreeManager;
use crate::fuzzy_match::FuzzyMatchManager;
use crate::markdown_parser::MarkdownParser;
use crate::languages::LanguageManager;
use crate::settings::SettingsManager;
use crate::collaboration::session_sharing::SessionSharingManager;
use crate::cloud::sync_manager::SyncManager;
use crate::serve_wasm::WasmServer;
use crate::agent_mode_eval::AgentMode; // Import AgentMode

/// Events generated during workflow execution.
#[derive(Debug, Clone)]
pub enum WorkflowExecutionEvent {
    Started { workflow_id: String, name: String },
    StepStarted { workflow_id: String, step_id: String, name: String },
    StepCompleted { workflow_id: String, step_id: String, name: String, output: String },
    StepFailed { workflow_id: String, step_id: String, name: String, error: String },
    Completed { workflow_id: String, name: String, success: bool },
    Error { workflow_id: String, message: String },
    AgentPromptRequest { // Event for interactive agent prompts
        workflow_id: String,
        step_id: String,
        prompt_id: String,
        message: String,
    },
}

pub struct WorkflowExecutor {
    event_sender: mpsc::Sender<WorkflowExecutionEvent>,
    command_manager: Arc<CommandManager>,
    virtual_file_system: Arc<VirtualFileSystem>,
    agent_mode: Arc<tokio::sync::RwLock<AgentMode>>, // Use AgentMode directly
    resource_manager: Arc<ResourceManager>,
    plugin_manager: Arc<PluginManager>,
    shell_manager: Arc<ShellManager>,
    drive_manager: Arc<DriveManager>,
    watcher: Arc<Watcher>,
    websocket_server: Arc<WebSocketServer>,
    lpc_engine: Arc<LpcEngine>,
    mcq_manager: Arc<McqManager>,
    natural_language_detector: Arc<NaturalLanguageDetector>,
    syntax_tree_manager: Arc<SyntaxTreeManager>,
    string_offset_manager: Arc<StringOffsetManager>,
    sum_tree_manager: Arc<SumTreeManager>,
    fuzzy_match_manager: Arc<FuzzyMatchManager>,
    markdown_parser: Arc<MarkdownParser>,
    language_manager: Arc<LanguageManager>,
    settings_manager: Arc<SettingsManager>,
    collaboration_manager: Arc<SessionSharingManager>,
    sync_manager: Arc<SyncManager>,
    wasm_server: Arc<WasmServer>,
}

impl WorkflowExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_manager: Arc<CommandManager>,
        virtual_file_system: Arc<VirtualFileSystem>,
        agent_mode: Arc<tokio::sync::RwLock<AgentMode>>, // Changed to AgentMode
        resource_manager: Arc<ResourceManager>,
        plugin_manager: Arc<PluginManager>,
        shell_manager: Arc<ShellManager>,
        drive_manager: Arc<DriveManager>,
        watcher: Arc<Watcher>,
        websocket_server: Arc<WebSocketServer>,
        lpc_engine: Arc<LpcEngine>,
        mcq_manager: Arc<McqManager>,
        natural_language_detector: Arc<NaturalLanguageDetector>,
        syntax_tree_manager: Arc<SyntaxTreeManager>,
        string_offset_manager: Arc<StringOffsetManager>,
        sum_tree_manager: Arc<SumTreeManager>,
        fuzzy_match_manager: Arc<FuzzyMatchManager>,
        markdown_parser: Arc<MarkdownParser>,
        language_manager: Arc<LanguageManager>,
        settings_manager: Arc<SettingsManager>,
        collaboration_manager: Arc<SessionSharingManager>,
        sync_manager: Arc<SyncManager>,
        wasm_server: Arc<WasmServer>,
    ) -> Self {
        let (tx, _) = mpsc::channel(100); // Dummy sender, will be replaced if needed
        Self {
            event_sender: tx,
            command_manager,
            virtual_file_system,
            agent_mode,
            resource_manager,
            plugin_manager,
            shell_manager,
            drive_manager,
            watcher,
            websocket_server,
            lpc_engine,
            mcq_manager,
            natural_language_detector,
            syntax_tree_manager,
            string_offset_manager,
            sum_tree_manager,
            fuzzy_match_manager,
            markdown_parser,
            language_manager,
            settings_manager,
            collaboration_manager,
            sync_manager,
            wasm_server,
        }
    }

    pub fn set_event_sender(&mut self, sender: mpsc::Sender<WorkflowExecutionEvent>) {
        self.event_sender = sender;
    }

    pub async fn execute_workflow(&self, workflow: Workflow, args: Vec<String>) -> Result<()> {
        log::info!("Executing workflow: {} (ID: {})", workflow.name, workflow.id);
        self.event_sender.send(WorkflowExecutionEvent::Started {
            workflow_id: workflow.id.clone(),
            name: workflow.name.clone(),
        }).await?;

        let mut success = true;
        let mut context: HashMap<String, Value> = HashMap::new();
        // Populate initial context from args
        for (i, arg) in args.iter().enumerate() {
            context.insert(format!("arg{}", i), Value::String(arg.clone()));
        }
        // Populate context from workflow arguments with default values
        for arg_def in &workflow.arguments {
            if let Some(default_val) = &arg_def.default_value {
                context.insert(arg_def.name.clone(), Value::String(default_val.clone()));
            }
        }


        for step in workflow.steps {
            let step_id = step.id.clone();
            let step_name = step.name.clone();
            log::info!("Executing step: {} (ID: {})", step_name, step_id);
            self.event_sender.send(WorkflowExecutionEvent::StepStarted {
                workflow_id: workflow.id.clone(),
                step_id: step_id.clone(),
                name: step_name.clone(),
            }).await?;

            match self.execute_step(&step, &mut context, &workflow.id).await {
                Ok(output) => {
                    log::info!("Step '{}' completed. Output: {}", step_name, output);
                    self.event_sender.send(WorkflowExecutionEvent::StepCompleted {
                        workflow_id: workflow.id.clone(),
                        step_id,
                        name: step_name,
                        output,
                    }).await?;
                },
                Err(e) => {
                    log::error!("Step '{}' failed: {:?}", step_name, e);
                    self.event_sender.send(WorkflowExecutionEvent::StepFailed {
                        workflow_id: workflow.id.clone(),
                        step_id,
                        name: step_name,
                        error: e.to_string(),
                    }).await?;
                    success = false;
                    break; // Stop on first error
                }
            }
        }

        self.event_sender.send(WorkflowExecutionEvent::Completed {
            workflow_id: workflow.id.clone(),
            name: workflow.name.clone(),
            success,
        }).await?;

        if success {
            log::info!("Workflow '{}' completed successfully.", workflow.name);
        } else {
            log::error!("Workflow '{}' failed.", workflow.name);
        }
        Ok(())
    }

    /// Executes a single workflow step.
    pub async fn execute_step(&self, step: &WorkflowStep, context: &mut HashMap<String, Value>, workflow_id: &str) -> Result<String> {
        log::info!("Executing workflow step: {}", step.name);

        let raw_output = match &step.step_type {
            WorkflowStepType::Command { command, args, working_directory } => {
                let resolved_command = self.resolve_placeholders(command, context)?;
                let resolved_args = args.iter()
                    .map(|arg| self.resolve_placeholders(arg, context))
                    .collect::<Result<Vec<String>>>()?;
                let resolved_working_dir = working_directory.as_ref()
                    .map(|dir| self.resolve_placeholders(dir, context))
                    .transpose()?;
                
                self.execute_command_step(&resolved_command, &resolved_args, resolved_working_dir.as_deref()).await?
            },
            WorkflowStepType::AgentPrompt { message, input_variable } => {
                let resolved_message = self.resolve_placeholders(message, context)?;
                let prompt_id = Uuid::new_v4().to_string();

                // Request input from the user via AgentMode
                let mut agent_mode_lock = self.agent_mode.write().await;
                let mut response_rx = agent_mode_lock.request_agent_prompt_input(prompt_id.clone(), resolved_message).await?;
                drop(agent_mode_lock); // Release lock

                // Wait for user's response
                let user_response = response_rx.recv().await
                    .ok_or_else(|| anyhow!("Agent prompt response channel closed unexpectedly for prompt ID: {}", prompt_id))?;
                
                if let Some(var_name) = input_variable {
                    context.insert(var_name.clone(), Value::String(user_response.clone()));
                }
                user_response
            },
            WorkflowStepType::ToolCall { tool_name, arguments } => {
                let agent_mode_lock = self.agent_mode.read().await;
                let tool_manager_lock = agent_mode_lock.assistant.read().await.tool_manager.lock().await; // Access tool_manager via assistant
                
                let tool = tool_manager_lock.get_tool(tool_name)
                    .ok_or_else(|| anyhow!("Tool '{}' not found.", tool_name))?;
                
                // Resolve arguments if they contain placeholders
                let resolved_arguments = self.resolve_json_placeholders(arguments.clone(), context)?;

                tool.execute(resolved_arguments).await?
            },
            WorkflowStepType::SubWorkflow { workflow_name, args } => {
                // TODO: Implement sub-workflow execution
                return Err(anyhow!("SubWorkflow step type not yet implemented."));
            },
            WorkflowStepType::PluginAction { plugin_name, action_name, arguments } => {
                // TODO: Implement plugin action execution
                return Err(anyhow!("PluginAction step type not yet implemented."));
            },
        };

        // 2. Output Handling (based on format)
        let output = match &step.output_format {
            WorkflowOutputFormat::PlainText => raw_output,
            WorkflowOutputFormat::Json => {
                // Attempt to parse as JSON and add to context
                match serde_json::from_str::<Value>(&raw_output) {
                    Ok(json_value) => {
                        if let Some(var_name) = &step.output_variable {
                            context.insert(var_name.clone(), json_value);
                            format!("Parsed JSON and stored in variable: {}", var_name)
                        } else {
                            "Parsed JSON but no output variable specified.".to_string()
                        }
                    }
                    Err(e) => return Err(anyhow!("Failed to parse command output as JSON: {}", e)),
                }
            }
            WorkflowOutputFormat::Regex { pattern } => {
                // Extract a specific part of the output using a regex
                let re = regex::Regex::new(pattern)?;
                if let Some(capture) = re.captures(&raw_output) {
                    if capture.len() > 1 {
                        let extracted_value = capture.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                        if let Some(var_name) = &step.output_variable {
                            context.insert(var_name.clone(), Value::String(extracted_value.clone()));
                            format!("Extracted value using regex and stored in variable: {}", var_name)
                        } else {
                            format!("Extracted value using regex: {}", extracted_value)
                        }
                    } else {
                        "Regex matched but no capture group found.".to_string()
                    }
                } else {
                    "Regex did not match the output.".to_string()
                }
            }
        };

        Ok(output)
    }

    async fn execute_command_step(&self, command: &str, args: &[String], working_dir: Option<&str>) -> Result<String> {
        let cmd_id = Uuid::new_v4().to_string();
        let cmd = Command {
            id: cmd_id.clone(),
            name: command.to_string(),
            description: format!("Workflow command: {}", command),
            executable: command.to_string(),
            args: args.to_vec(),
            env: HashMap::new(), // Environment variables from step will be handled by PTY
            working_dir: working_dir.map(|s| s.to_string()),
            output_format: crate::command::CommandOutputFormat::PlainText, // Always plain text for raw output
        };

        // Execute the command and capture its output
        let (tx, mut rx) = mpsc::channel(100);
        self.command_manager.execute_command_with_output_channel(cmd, tx).await?;

        let mut full_output = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                crate::command::CommandEvent::Output { data, .. } => {
                    full_output.push_str(&String::from_utf8_lossy(&data));
                },
                crate::command::CommandEvent::Completed { exit_code, .. } => {
                    if exit_code != 0 {
                        return Err(anyhow!("Command '{}' failed with exit code: {}", command, exit_code));
                    }
                    break;
                },
                crate::command::CommandEvent::Error { message, .. } => {
                    return Err(anyhow!("Command '{}' execution error: {}", command, message));
                },
                _ => {}
            }
        }
        Ok(full_output)
    }

    /// Resolves `{{variable}}` placeholders in a string using the provided context.
    fn resolve_placeholders(&self, text: &str, context: &HashMap<String, Value>) -> Result<String> {
        let mut resolved_text = text.to_string();
        let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap(); // Regex to find {{variable}}

        for captures in re.captures_iter(text) {
            let placeholder = captures.get(0).unwrap().as_str(); // e.g., "{{user_name}}"
            let var_name = captures.get(1).unwrap().as_str();    // e.g., "user_name"

            if let Some(value) = context.get(var_name) {
                let replacement = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => return Err(anyhow!("Unsupported value type for placeholder '{}'", var_name)),
                };
                resolved_text = resolved_text.replace(placeholder, &replacement);
            } else {
                return Err(anyhow!("Missing context variable for placeholder: {}", var_name));
            }
        }
        Ok(resolved_text)
    }

    /// Recursively resolves placeholders within a JSON Value.
    fn resolve_json_placeholders(&self, value: Value, context: &HashMap<String, Value>) -> Result<Value> {
        match value {
            Value::String(s) => Ok(Value::String(self.resolve_placeholders(&s, context)?)),
            Value::Array(arr) => {
                let mut new_arr = Vec::new();
                for item in arr {
                    new_arr.push(self.resolve_json_placeholders(item, context)?);
                }
                Ok(Value::Array(new_arr))
            },
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (key, val) in obj {
                    new_obj.insert(key, self.resolve_json_placeholders(val, context)?);
                }
                Ok(Value::Object(new_obj))
            },
            _ => Ok(value), // Numbers, Booleans, Null don't have placeholders
        }
    }
}
