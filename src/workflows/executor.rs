use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::workflow::{Workflow, WorkflowStep, WorkflowOutputFormat};
use crate::command::{CommandManager, Command};
use crate::virtual_fs::VirtualFileSystem;
use crate::agent_mode_eval::AgentModeEvaluator;
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

/// Events generated during workflow execution.
#[derive(Debug, Clone)]
pub enum WorkflowExecutionEvent {
    Started { workflow_id: String, name: String },
    StepStarted { workflow_id: String, step_id: String, name: String },
    StepCompleted { workflow_id: String, step_id: String, name: String, output: String },
    StepFailed { workflow_id: String, step_id: String, name: String, error: String },
    Completed { workflow_id: String, name: String, success: bool },
    Error { workflow_id: String, message: String },
}

pub struct WorkflowExecutor {
    event_sender: mpsc::Sender<WorkflowExecutionEvent>,
    command_manager: Arc<CommandManager>,
    virtual_file_system: Arc<VirtualFileSystem>,
    agent_evaluator: Arc<AgentModeEvaluator>,
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
        agent_evaluator: Arc<AgentModeEvaluator>,
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
            agent_evaluator,
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

        for step in workflow.steps {
            let step_id = step.id.clone();
            let step_name = step.name.clone();
            log::info!("Executing step: {} (ID: {})", step_name, step_id);
            self.event_sender.send(WorkflowExecutionEvent::StepStarted {
                workflow_id: workflow.id.clone(),
                step_id: step_id.clone(),
                name: step_name.clone(),
            }).await?;

            match self.execute_step(&step, &mut context).await {
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
    pub async fn execute_step(&self, step: &WorkflowStep, context: &mut HashMap<String, Value>) -> Result<String> {
        log::info!("Executing workflow step: {}", step.name);

        // 1. Command Execution
        let command_output = self.execute_command_step(step).await?;

        // 2. Output Handling (based on format)
        let output = match &step.output_format {
            WorkflowOutputFormat::PlainText => command_output,
            WorkflowOutputFormat::Json => {
                // Attempt to parse as JSON and add to context
                match serde_json::from_str::<Value>(&command_output) {
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
                if let Some(capture) = re.captures(&command_output) {
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

    async fn execute_command_step(&self, step: &WorkflowStep) -> Result<String> {
        let cmd_id = Uuid::new_v4().to_string();
        let cmd = Command {
            id: cmd_id.clone(),
            name: step.name.clone(),
            description: format!("Workflow step: {}", step.name),
            executable: step.command.clone(),
            args: step.args.clone(),
            env: step.environment.clone(),
            working_dir: step.working_directory.clone(),
            output_format: command::CommandOutputFormat::PlainText, // Or derive from step
        };

        self.command_manager.execute_command(cmd).await?;
        // In a real implementation, you'd capture the output and handle errors
        Ok(format!("Command '{}' executed (awaiting real output).", step.command))
    }
}
