use std::process::{Command, Stdio};
use std::io::{self, Write, BufReader, BufRead};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::process::Command as TokioCommand;
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};

use crate::shell::ShellSession; // Import ShellSession

#[derive(Debug)]
pub enum CommandStatus {
    Running,
    Completed(i32),
    Failed(String),
    Killed,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: CommandStatus,
}

pub struct PtyManager {
    // In a full PTY implementation, this would manage the pseudo-terminal
    // For now, it wraps `ShellSession` for command execution.
}

impl PtyManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute_command(
        &self,
        command: &str,
        args: Vec<&str>,
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<mpsc::Receiver<CommandOutput>, String> {
        let (tx, rx) = mpsc::channel(100); // Channel for sending CommandOutput chunks

        let session = ShellSession::new(env_vars); // Create a session with provided env vars

        let command_str = command.to_string();
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        tokio::spawn(async move {
            let mut cmd = TokioCommand::new(&command_str);
            cmd.args(&args_vec);
            
            // Apply environment variables from the session
            for (key, value) in &session.environment {
                cmd.env(key, value);
            }

            cmd.stdout(Stdio::piped())
               .stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(CommandOutput {
                        stdout: String::new(),
                        stderr: String::new(),
                        status: CommandStatus::Failed(format!("Failed to spawn command: {}", e)),
                    }).await;
                    return;
                }
            };

            let stdout = child.stdout.take().expect("Child did not have stdout");
            let stderr = child.stderr.take().expect("Child did not have stderr");

            let mut stdout_reader = TokioBufReader::new(stdout).lines();
            let mut stderr_reader = TokioBufReader::new(stderr).lines();

            loop {
                tokio::select! {
                    stdout_line = stdout_reader.next_line() => {
                        match stdout_line {
                            Ok(Some(line)) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: line + "\n",
                                    stderr: String::new(),
                                    status: CommandStatus::Running,
                                }).await;
                            }
                            Ok(None) => { /* stdout stream closed */ }
                            Err(e) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: String::new(),
                                    status: CommandStatus::Failed(format!("Stdout read error: {}", e)),
                                }).await;
                                break;
                            }
                        }
                    }
                    stderr_line = stderr_reader.next_line() => {
                        match stderr_line {
                            Ok(Some(line)) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: line + "\n",
                                    status: CommandStatus::Running,
                                }).await;
                            }
                            Ok(None) => { /* stderr stream closed */ }
                            Err(e) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: String::new(),
                                    status: CommandStatus::Failed(format!("Stderr read error: {}", e)),
                                }).await;
                                break;
                            }
                        }
                    }
                    child_status = child.wait() => {
                        match child_status {
                            Ok(status) => {
                                let exit_code = status.code().unwrap_or(1);
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
                                    status: CommandStatus::Failed(format!("Child process error: {}", e)),
                                }).await;
                            }
                        }
                        break; // Child exited, so break the loop
                    }
                }
            }
        });

        Ok(rx)
    }
}
