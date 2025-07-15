use std::process::Stdio;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

pub struct ShellManager {
    // In a real application, you might manage multiple shell sessions
    // For now, we'll keep it simple.
}

impl ShellManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Executes a command and returns its output and exit code.
    /// This is a simplified version for the Iced GUI.
    pub async fn execute_command(&self, command: String, env_vars: Option<HashMap<String, String>>) -> (String, i32) {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut cmd = Command::new(&shell);
        cmd.arg("-c").arg(&command);

        if let Some(vars) = env_vars {
            cmd.envs(vars);
        }

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(1);
                (format!("{}\n{}", stdout, stderr), exit_code)
            }
            Err(e) => {
                (format!("Failed to execute command: {}", e), 1)
            }
        }
    }

    /// Creates a new interactive shell session.
    pub fn create_session(&self, initial_env: Option<HashMap<String, String>>) -> Result<ShellSession, String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut command = Command::new(&shell);
        command.stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());

        if let Some(vars) = initial_env {
            command.envs(vars);
        }

        let child = command.spawn().map_err(|e| format!("Failed to spawn shell: {}", e))?;

        Ok(ShellSession {
            child,
            environment: initial_env.unwrap_or_default(), // Store the environment for the session
        })
    }
}

pub struct ShellSession {
    child: Child,
    pub environment: HashMap<String, String>, // Environment variables for this session
}

impl ShellSession {
    /// Sends a command to the interactive shell session.
    pub async fn send_command(&mut self, command: &str) -> Result<(), String> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(command.as_bytes()).await.map_err(|e| format!("Failed to write to stdin: {}", e))?;
            stdin.write_all(b"\n").await.map_err(|e| format!("Failed to write newline to stdin: {}", e))?;
            Ok(())
        } else {
            Err("Stdin not available for shell session".to_string())
        }
    }

    /// Streams output from the shell session.
    pub async fn execute_command_stream(&mut self, command: String, tx: mpsc::Sender<String>) -> Result<(), String> {
        self.send_command(&command).await?;

        let stdout = self.child.stdout.take().ok_or("Stdout not available")?;
        let stderr = self.child.stderr.take().ok_or("Stderr not available")?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout_reader.next_line() => {
                    if tx.send(line).await.is_err() {
                        break; // Receiver dropped
                    }
                }
                Ok(Some(line)) = stderr_reader.next_line() => {
                    if tx.send(format!("[STDERR] {}", line)).await.is_err() {
                        break; // Receiver dropped
                    }
                }
                status = self.child.wait() => {
                    match status {
                        Ok(exit_status) => {
                            if tx.send(format!("[Command exited with status: {}]", exit_status)).await.is_err() {
                                // Receiver dropped
                            }
                        }
                        Err(e) => {
                            if tx.send(format!("[Command failed: {}]", e)).await.is_err() {
                                // Receiver dropped
                            }
                        }
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}
