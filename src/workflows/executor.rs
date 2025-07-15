use super::{Workflow, WorkflowExecution, WorkflowError, Shell, ArgumentType};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use regex::Regex;

pub struct WorkflowExecutor {
    current_shell: Shell,
    environment: HashMap<String, String>,
}

impl WorkflowExecutor {
    pub fn new(shell: Shell) -> Self {
        Self {
            current_shell: shell,
            environment: std::env::vars().collect(),
        }
    }

    /// Prepare workflow for execution by resolving arguments
    pub fn prepare_execution(
        &self,
        workflow: &Workflow,
        arguments: HashMap<String, String>,
    ) -> Result<WorkflowExecution, WorkflowError> {
        // Validate shell compatibility
        if !workflow.is_compatible_with_shell(&self.current_shell) {
            return Err(WorkflowError::UnsupportedShell(self.current_shell.clone()));
        }

        // Validate and resolve arguments
        let resolved_args = self.validate_and_resolve_arguments(workflow, arguments)?;
        
        // Substitute arguments in command
        let resolved_command = self.substitute_arguments(&workflow.command, &resolved_args)?;

        Ok(WorkflowExecution {
            workflow: workflow.clone(),
            arguments: resolved_args,
            resolved_command,
            shell: self.current_shell.clone(),
        })
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        execution: &WorkflowExecution,
    ) -> Result<WorkflowExecutionResult, WorkflowError> {
        let start_time = std::time::Instant::now();

        let output = match self.current_shell {
            Shell::Bash => self.execute_bash(&execution.resolved_command).await?,
            Shell::Zsh => self.execute_zsh(&execution.resolved_command).await?,
            Shell::Fish => self.execute_fish(&execution.resolved_command).await?,
        };

        let execution_time = start_time.elapsed();

        Ok(WorkflowExecutionResult {
            workflow_name: execution.workflow.name.clone(),
            command: execution.resolved_command.clone(),
            output,
            execution_time,
            success: true, // This would be determined by the actual execution
        })
    }

    /// Execute workflow in dry-run mode (show what would be executed)
    pub fn dry_run(&self, execution: &WorkflowExecution) -> WorkflowDryRun {
        WorkflowDryRun {
            workflow_name: execution.workflow.name.clone(),
            original_command: execution.workflow.command.clone(),
            resolved_command: execution.resolved_command.clone(),
            arguments: execution.arguments.clone(),
            shell: execution.shell.clone(),
            environment_vars: self.get_relevant_env_vars(&execution.resolved_command),
        }
    }

    fn validate_and_resolve_arguments(
        &self,
        workflow: &Workflow,
        mut arguments: HashMap<String, String>,
    ) -> Result<HashMap<String, String>, WorkflowError> {
        let mut resolved = HashMap::new();

        for arg_def in &workflow.arguments {
            let value = if let Some(provided_value) = arguments.remove(&arg_def.name) {
                provided_value
            } else if let Some(default_value) = &arg_def.default_value {
                default_value.clone()
            } else if arg_def.required {
                return Err(WorkflowError::MissingArgument(arg_def.name.clone()));
            } else {
                String::new()
            };

            // Validate argument value
            self.validate_argument_value(arg_def, &value)?;
            resolved.insert(arg_def.name.clone(), value);
        }

        // Check for unexpected arguments
        if !arguments.is_empty() {
            let unexpected: Vec<_> = arguments.keys().collect();
            return Err(WorkflowError::ArgumentError(
                format!("Unexpected arguments: {:?}", unexpected)
            ));
        }

        Ok(resolved)
    }

    fn validate_argument_value(
        &self,
        arg_def: &super::WorkflowArgument,
        value: &str,
    ) -> Result<(), WorkflowError> {
        if value.is_empty() && arg_def.required {
            return Err(WorkflowError::MissingArgument(arg_def.name.clone()));
        }

        if value.is_empty() {
            return Ok(());
        }

        match arg_def.arg_type {
            ArgumentType::String => Ok(()),
            ArgumentType::Number => {
                value.parse::<f64>()
                    .map_err(|_| WorkflowError::InvalidArgumentValue(
                        format!("'{}' is not a valid number", value)
                    ))?;
                Ok(())
            }
            ArgumentType::Boolean => {
                match value.to_lowercase().as_str() {
                    "true" | "false" | "1" | "0" | "yes" | "no" => Ok(()),
                    _ => Err(WorkflowError::InvalidArgumentValue(
                        format!("'{}' is not a valid boolean", value)
                    )),
                }
            }
            ArgumentType::Path => {
                // Basic path validation
                if value.contains('\0') {
                    Err(WorkflowError::InvalidArgumentValue(
                        "Path cannot contain null characters".to_string()
                    ))
                } else {
                    Ok(())
                }
            }
            ArgumentType::Url => {
                url::Url::parse(value)
                    .map_err(|_| WorkflowError::InvalidArgumentValue(
                        format!("'{}' is not a valid URL", value)
                    ))?;
                Ok(())
            }
            ArgumentType::Email => {
                let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
                if email_regex.is_match(value) {
                    Ok(())
                } else {
                    Err(WorkflowError::InvalidArgumentValue(
                        format!("'{}' is not a valid email", value)
                    ))
                }
            }
            ArgumentType::Enum => {
                if let Some(options) = &arg_def.options {
                    if options.contains(&value.to_string()) {
                        Ok(())
                    } else {
                        Err(WorkflowError::InvalidArgumentValue(
                            format!("'{}' is not one of the valid options: {:?}", value, options)
                        ))
                    }
                } else {
                    Ok(())
                }
            }
        }
    }

    fn substitute_arguments(
        &self,
        command: &str,
        arguments: &HashMap<String, String>,
    ) -> Result<String, WorkflowError> {
        let mut result = command.to_string();

        for (name, value) in arguments {
            let placeholder = format!("{{{{{}}}}}", name);
            
            // Escape shell special characters in the value
            let escaped_value = self.escape_shell_value(value);
            result = result.replace(&placeholder, &escaped_value);
        }

        // Check for any remaining unresolved placeholders
        let placeholder_regex = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        if let Some(captures) = placeholder_regex.captures(&result) {
            let unresolved = captures.get(1).unwrap().as_str();
            return Err(WorkflowError::ArgumentError(
                format!("Unresolved placeholder: {}", unresolved)
            ));
        }

        Ok(result)
    }

    fn escape_shell_value(&self, value: &str) -> String {
        match self.current_shell {
            Shell::Bash | Shell::Zsh => {
                // For bash/zsh, quote the value and escape internal quotes
                format!("'{}'", value.replace('\'', "'\"'\"'"))
            }
            Shell::Fish => {
                // Fish uses different quoting rules
                if value.contains(' ') || value.contains('\t') || value.contains('\n') {
                    format!("'{}'", value.replace('\'', "\\'"))
                } else {
                    value.to_string()
                }
            }
        }
    }

    async fn execute_bash(&self, command: &str) -> Result<CommandOutput, WorkflowError> {
        let output = Command::new("bash")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    async fn execute_zsh(&self, command: &str) -> Result<CommandOutput, WorkflowError> {
        let output = Command::new("zsh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    async fn execute_fish(&self, command: &str) -> Result<CommandOutput, WorkflowError> {
        let output = Command::new("fish")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    fn get_relevant_env_vars(&self, command: &str) -> HashMap<String, String> {
        let mut relevant_vars = HashMap::new();
        
        // Common environment variables that might be relevant
        let common_vars = [
            "PATH", "HOME", "USER", "PWD", "SHELL", "TERM",
            "LANG", "LC_ALL", "EDITOR", "PAGER"
        ];

        for var in &common_vars {
            if let Some(value) = self.environment.get(*var) {
                relevant_vars.insert(var.to_string(), value.clone());
            }
        }

        // Look for environment variable references in the command
        let env_regex = Regex::new(r"\$([A-Z_][A-Z0-9_]*)").unwrap();
        for captures in env_regex.captures_iter(command) {
            if let Some(var_name) = captures.get(1) {
                let var_name = var_name.as_str();
                if let Some(value) = self.environment.get(var_name) {
                    relevant_vars.insert(var_name.to_string(), value.clone());
                }
            }
        }

        relevant_vars
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionResult {
    pub workflow_name: String,
    pub command: String,
    pub output: CommandOutput,
    pub execution_time: std::time::Duration,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct WorkflowDryRun {
    pub workflow_name: String,
    pub original_command: String,
    pub resolved_command: String,
    pub arguments: HashMap<String, String>,
    pub shell: Shell,
    pub environment_vars: HashMap<String, String>,
}

// This module would execute defined workflows.
// For now, it's a placeholder.
