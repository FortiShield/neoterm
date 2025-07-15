use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandStatus {
    Running,
    Completed(i32), // exit code
    Failed(String),
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: CommandStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct PtyManager {
    active_commands: Arc<Mutex<Vec<ActiveCommand>>>,
}

struct ActiveCommand {
    id: String,
    process: std::process::Child,
    output_sender: mpsc::UnboundedSender<CommandOutput>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            active_commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn execute_command(
        &self,
        command: &str,
        args: Vec<&str>,
        working_dir: Option<&str>,
    ) -> Result<mpsc::UnboundedReceiver<CommandOutput>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let command_id = uuid::Uuid::new_v4().to_string();

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Handle stdout
        let tx_stdout = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let output = CommandOutput {
                        stdout: line,
                        stderr: String::new(),
                        status: CommandStatus::Running,
                        timestamp: chrono::Utc::now(),
                    };
                    if tx_stdout.send(output).is_err() {
                        break;
                    }
                }
            }
        });

        // Handle stderr
        let tx_stderr = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let output = CommandOutput {
                        stdout: String::new(),
                        stderr: line,
                        status: CommandStatus::Running,
                        timestamp: chrono::Utc::now(),
                    };
                    if tx_stderr.send(output).is_err() {
                        break;
                    }
                }
            }
        });

        // Store active command
        {
            let mut commands = self.active_commands.lock().unwrap();
            commands.push(ActiveCommand {
                id: command_id.clone(),
                process: child,
                output_sender: tx.clone(),
            });
        }

        // Monitor process completion
        let commands_ref = Arc::clone(&self.active_commands);
        let tx_completion = tx.clone();
        tokio::spawn(async move {
            // Wait for process completion in a separate thread
            let (completion_tx, mut completion_rx) = mpsc::unbounded_channel();
            
            thread::spawn(move || {
                let mut commands = commands_ref.lock().unwrap();
                if let Some(pos) = commands.iter().position(|cmd| cmd.id == command_id) {
                    let mut cmd = commands.remove(pos);
                    match cmd.process.wait() {
                        Ok(status) => {
                            let exit_code = status.code().unwrap_or(-1);
                            let _ = completion_tx.send(CommandStatus::Completed(exit_code));
                        }
                        Err(e) => {
                            let _ = completion_tx.send(CommandStatus::Failed(e.to_string()));
                        }
                    }
                }
            });

            if let Some(status) = completion_rx.recv().await {
                let final_output = CommandOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    status,
                    timestamp: chrono::Utc::now(),
                };
                let _ = tx_completion.send(final_output);
            }
        });

        Ok(rx)
    }

    pub fn kill_command(&self, command_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut commands = self.active_commands.lock().unwrap();
        if let Some(pos) = commands.iter().position(|cmd| cmd.id == command_id) {
            let mut cmd = commands.remove(pos);
            cmd.process.kill()?;
            
            let killed_output = CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                status: CommandStatus::Killed,
                timestamp: chrono::Utc::now(),
            };
            let _ = cmd.output_sender.send(killed_output);
        }
        Ok(())
    }

    pub fn get_active_commands(&self) -> Vec<String> {
        let commands = self.active_commands.lock().unwrap();
        commands.iter().map(|cmd| cmd.id.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_execution() {
        let pty = PtyManager::new();
        let mut rx = pty.execute_command("echo", vec!["hello world"], None).await.unwrap();
        
        let mut outputs = Vec::new();
        while let Some(output) = rx.recv().await {
            outputs.push(output.clone());
            if matches!(output.status, CommandStatus::Completed(_)) {
                break;
            }
        }
        
        assert!(!outputs.is_empty());
        assert!(outputs.iter().any(|o| o.stdout.contains("hello world")));
    }
}
