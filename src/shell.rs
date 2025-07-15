use std::process::{Command, Stdio};
use std::io::{self, Write, BufReader, BufRead};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::process::Command as TokioCommand;
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};

pub struct ShellManager {
    // In a real application, this might manage multiple shell sessions
}

impl ShellManager {
    pub fn new() -> Self {
        Self {}
    }

    // This function is for the Iced GUI path, which is not the primary focus for this request.
    pub async fn execute_command(&self, command: String, env_vars: Option<HashMap<String, String>>) -> (String, i32) {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&command);

        if let Some(vars) = env_vars {
            for (key, value) in vars {
                cmd.env(key, value);
            }
        }

        let output = cmd.output().expect("Failed to execute command");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(1);

        (format!("{}{}", stdout, stderr), exit_code)
    }

    // This function is for the Ratatui TUI path, used by PtyManager.
    pub fn create_session(&self, initial_env: Option<HashMap<String, String>>) -> ShellSession {
        ShellSession::new(initial_env)
    }
}

pub struct ShellSession {
    // This might hold a PTY master, or just manage environment for non-interactive commands
    pub environment: HashMap<String, String>,
}

impl ShellSession {
    pub fn new(initial_env: Option<HashMap<String, String>>) -> Self {
        let mut environment = std::env::vars().collect::<HashMap<String, String>>();
        if let Some(env_vars) = initial_env {
            environment.extend(env_vars);
        }
        Self { environment }
    }

    // This method is now primarily used by PtyManager for interactive/streaming commands
    pub async fn execute_command_stream(
        &self,
        command: &str,
        args: &[&str],
        tx: mpsc::Sender<String>, // Sender for streaming output
    ) -> Result<i32, String> {
        let mut cmd = TokioCommand::new(command);
        cmd.args(args);
        
        // Apply environment variables from the session
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;

        let stdout = child.stdout.take().ok_or("Child did not have stdout")?;
        let stderr = child.stderr.take().ok_or("Child did not have stderr")?;

        let mut stdout_reader = TokioBufReader::new(stdout).lines();
        let mut stderr_reader = TokioBufReader::new(stderr).lines();

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout_reader.next_line() => {
                    tx.send(line).await.map_err(|e| format!("Failed to send stdout: {}", e))?;
                }
                Ok(Some(line)) = stderr_reader.next_line() => {
                    tx.send(format!("ERROR: {}", line)).await.map_err(|e| format!("Failed to send stderr: {}", e))?;
                }
                status = child.wait() => {
                    let exit_code = status.map_err(|e| format!("Failed to wait for child: {}", e))?.code().unwrap_or(1);
                    return Ok(exit_code);
                }
                else => break, // Both streams closed and child not yet exited
            }
        }
        
        // In case streams close before child exits, wait for child
        let exit_code = child.wait().await.map_err(|e| format!("Failed to wait for child: {}", e))?.code().unwrap_or(1);
        Ok(exit_code)
    }
}
