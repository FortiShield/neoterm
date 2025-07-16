use tokio::sync::mpsc;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use uuid::Uuid;
use anyhow::Result;
use serde::{Deserialize, Serialize};
pub mod pty;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub description: String,
    pub executable: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub output_format: CommandOutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandOutputFormat {
    PlainText,
    Markdown,
    Json,
    // Add more formats like HTML, YAML, etc.
}

#[derive(Debug, Clone)]
pub enum CommandEvent {
    /// A command has started execution.
    Started { id: String, command_line: String },
    /// New output (stdout or stderr) from a command.
    Output { id: String, data: Vec<u8>, is_stderr: bool },
    /// A command has completed execution.
    Completed { id: String, exit_code: Option<i32> },
    /// A command failed to start or encountered an error during execution.
    Error { id: String, message: String },
}

pub struct CommandManager {
    event_sender: mpsc::Sender<CommandEvent>,
    // Add state for tracking running commands, their PTYs, etc.
    // For example: HashMap<String, pty::PtySession>
}

impl CommandManager {
    pub fn new(event_sender: mpsc::Sender<CommandEvent>) -> Self {
        Self { event_sender }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Command manager initialized.");
        Ok(())
    }

    /// Executes a command.
    pub async fn execute_command(&self, command: Command) -> Result<()> {
        let command_id = command.id.clone();
        let command_line = format!("{} {}", command.executable, command.args.join(" "));
        log::info!("Executing command: {}", command_line);

        self.event_sender.send(CommandEvent::Started {
            id: command_id.clone(),
            command_line: command_line.clone(),
        }).await?;

        let sender_clone = self.event_sender.clone();
        tokio::spawn(async move {
            match pty::PtySession::spawn(
                &command.executable,
                &command.args,
                command.working_dir.as_deref(),
                &command.env,
            ).await {
                Ok(mut pty_session) => {
                    let command_id_clone = command_id.clone();
                    let sender_clone_output = sender_clone.clone();
                    tokio::spawn(async move {
                        while let Some(output) = pty_session.read_output().await {
                            let _ = sender_clone_output.send(CommandEvent::Output {
                                id: command_id_clone.clone(),
                                data: output.data,
                                is_stderr: output.is_stderr,
                            }).await;
                        }
                    });

                    match pty_session.wait().await {
                        Ok(exit_status) => {
                            let exit_code = exit_status.code();
                            log::info!("Command {} completed with exit code {:?}", command_id, exit_code);
                            let _ = sender_clone.send(CommandEvent::Completed {
                                id: command_id,
                                exit_code,
                            }).await;
                        },
                        Err(e) => {
                            log::error!("Command {} failed to wait: {:?}", command_id, e);
                            let _ = sender_clone.send(CommandEvent::Error {
                                id: command_id,
                                message: format!("Failed to wait for command: {}", e),
                            }).await;
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to spawn command {}: {:?}", command_id, e);
                    let _ = sender_clone.send(CommandEvent::Error {
                        id: command_id,
                        message: format!("Failed to spawn command: {}", e),
                    }).await;
                }
            }
        });

        Ok(())
    }

    /// Sends input to a running command's PTY.
    pub async fn send_input(&self, command_id: &str, input: &[u8]) -> Result<()> {
        // In a real implementation, you'd look up the PtySession by command_id
        // and then call its `write_input` method.
        // For this stub, we'll just log it.
        log::debug!("Sending input to command {}: {:?}", command_id, String::from_utf8_lossy(input));
        // Example: self.running_commands.get_mut(command_id).map(|pty| pty.write_input(input).await);
        Ok(())
    }

    /// Terminates a running command.
    pub async fn terminate_command(&self, command_id: &str) -> Result<()> {
        // In a real implementation, you'd look up the PtySession by command_id
        // and then call its `kill` method.
        log::info!("Terminating command: {}", command_id);
        // Example: self.running_commands.get_mut(command_id).map(|pty| pty.kill().await);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum CommandMessage {
    Output(String),
    Error(String),
    Exit(i32),
    Start(Uuid),
}

#[derive(Debug, Clone)]
pub struct CommandExecutor {
    // This struct might hold configuration for command execution,
    // like default working directory, environment variables, etc.
}

impl CommandExecutor {
    pub fn new() -> Self {
        CommandExecutor {}
    }

    pub fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_directory: Option<String>,
        tx: mpsc::UnboundedSender<String>, // Sender for output/error
    ) -> Uuid {
        let command_id = Uuid::new_v4();
        let cmd_str = format!("{} {}", command, args.join(" "));
        println!("Executing command [{}]: {}", command_id, cmd_str);

        let mut cmd = TokioCommand::new(command);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(wd) = working_directory {
            cmd.current_dir(wd);
        }

        tokio::spawn(async move {
            let _ = tx.send(format!("$ {}\n", cmd_str)); // Echo command to output

            match cmd.spawn() {
                Ok(mut child) => {
                    let stdout = child.stdout.take().expect("Failed to take stdout");
                    let stderr = child.stderr.take().expect("Failed to take stderr");

                    let mut stdout_reader = BufReader::new(stdout).lines();
                    let mut stderr_reader = BufReader::new(stderr).lines();

                    loop {
                        tokio::select! {
                            Ok(Some(line)) = stdout_reader.next_line() => {
                                let _ = tx.send(line + "\n");
                            }
                            Ok(Some(line)) = stderr_reader.next_line() => {
                                let _ = tx.send(format!("ERROR: {}\n", line));
                            }
                            status = child.wait() => {
                                match status {
                                    Ok(exit_status) => {
                                        let code = exit_status.code().unwrap_or(-1);
                                        let _ = tx.send(format!("Command exited with code: {}\n", code));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(format!("Failed to wait for command: {}\n", e));
                                    }
                                }
                                break;
                            }
                            else => break, // All streams closed and child exited
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to spawn command '{}': {}\n", command, e));
                }
            }
        });

        command_id
    }
}

pub fn init() {
    println!("command module loaded");
}
