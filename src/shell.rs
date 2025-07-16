use tokio::sync::mpsc;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

/// Represents a shell session.
/// This struct manages the lifecycle of a shell process and its I/O.
pub struct Shell {
    pub id: Uuid,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
    pub environment: HashMap<String, String>,
    // Sender to send shell output/events back to the main application
    output_sender: mpsc::UnboundedSender<ShellOutput>,
    // Child process handle
    // child: Option<tokio::process::Child>, // Managed internally by spawn_and_stream
}

/// Messages sent from the shell to the main application.
#[derive(Debug, Clone)]
pub enum ShellOutput {
    Stdout(String),
    Stderr(String),
    Exit(i32),
    Error(String),
    Started,
}

impl Shell {
    pub fn new(
        command: String,
        args: Vec<String>,
        working_directory: Option<String>,
        environment: HashMap<String, String>,
        output_sender: mpsc::UnboundedSender<ShellOutput>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            command,
            args,
            working_directory,
            environment,
            output_sender,
        }
    }

    /// Spawns the shell process and streams its output.
    pub async fn spawn_and_stream(mut self) {
        let _ = self.output_sender.send(ShellOutput::Started);
        println!("Spawning shell: {} {:?}", self.command, self.args);

        let mut cmd = TokioCommand::new(&self.command);
        cmd.args(&self.args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.envs(&self.environment);

        if let Some(wd) = &self.working_directory {
            cmd.current_dir(wd);
        }

        match cmd.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("Failed to take stdout");
                let stderr = child.stderr.take().expect("Failed to take stderr");

                let mut stdout_reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                loop {
                    tokio::select! {
                        Ok(Some(line)) = stdout_reader.next_line() => {
                            let _ = self.output_sender.send(ShellOutput::Stdout(line + "\n"));
                        }
                        Ok(Some(line)) = stderr_reader.next_line() => {
                            let _ = self.output_sender.send(ShellOutput::Stderr(line + "\n"));
                        }
                        status = child.wait() => {
                            match status {
                                Ok(exit_status) => {
                                    let code = exit_status.code().unwrap_or(-1);
                                    let _ = self.output_sender.send(ShellOutput::Exit(code));
                                }
                                Err(e) => {
                                    let _ = self.output_sender.send(ShellOutput::Error(format!("Failed to wait for shell: {}", e)));
                                }
                            }
                            break;
                        }
                        else => break, // All streams closed and child exited
                    }
                }
            }
            Err(e) => {
                let _ = self.output_sender.send(ShellOutput::Error(format!("Failed to spawn shell '{}': {}", self.command, e)));
            }
        }
    }

    // Potentially add methods for sending input to the shell (if using PTY)
    // pub async fn send_input(&self, input: &str) -> Result<(), String> { ... }
}

pub fn init() {
    println!("shell module loaded");
}
