use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Represents a chunk of output from a command, including its status.
#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: CommandStatus,
}

/// The current status of a command being executed.
#[derive(Debug)]
pub enum CommandStatus {
    Running,
    Completed(i32), // Exit code
    Failed(String), // Error message
    Killed,
}

pub struct PtyManager {
    // In a real PTY manager, you would manage actual PTYs (pseudo-terminals)
    // For this simplified example, we're just simulating command execution.
}

impl PtyManager {
    pub fn new() -> Self {
        PtyManager {}
    }

    /// Executes a command and streams its output.
    /// Returns a receiver to get `CommandOutput` chunks.
    pub async fn execute_command(
        &self,
        cmd: &str,
        args: Vec<&str>,
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<mpsc::Receiver<CommandOutput>, String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut command = Command::new(&shell);
        command.arg("-c").arg(format!("{} {}", cmd, args.join(" ")));
        
        // Apply environment variables if provided
        if let Some(vars) = env_vars {
            command.envs(vars);
        }

        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command.spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let (tx, rx) = mpsc::channel(100); // Channel for sending output chunks

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    stdout_line = stdout_reader.next_line() => {
                        match stdout_line {
                            Ok(Some(line)) => {
                                if tx.send(CommandOutput {
                                    stdout: line + "\n",
                                    stderr: String::new(),
                                    status: CommandStatus::Running,
                                }).await.is_err() {
                                    break; // Receiver dropped
                                }
                            }
                            Ok(None) => { /* stdout closed */ }
                            Err(e) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: format!("Error reading stdout: {}\n", e),
                                    status: CommandStatus::Failed(e.to_string()),
                                }).await;
                                break;
                            }
                        }
                    }
                    stderr_line = stderr_reader.next_line() => {
                        match stderr_line {
                            Ok(Some(line)) => {
                                if tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: line + "\n",
                                    status: CommandStatus::Running,
                                }).await.is_err() {
                                    break; // Receiver dropped
                                }
                            }
                            Ok(None) => { /* stderr closed */ }
                            Err(e) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: format!("Error reading stderr: {}\n", e),
                                    status: CommandStatus::Failed(e.to_string()),
                                }).await;
                                break;
                            }
                        }
                    }
                    status = child.wait() => {
                        match status {
                            Ok(exit_status) => {
                                let exit_code = exit_status.code().unwrap_or(1);
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: String::new(),
                                    status: CommandStatus::Completed(exit_code),
                                }).await;
                            }
                            Err(e) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: String::new(),
                                    status: CommandStatus::Failed(e.to_string()),
                                }).await;
                            }
                        }
                        break; // Command finished
                    }
                }
            }
        });

        Ok(rx)
    }
}
