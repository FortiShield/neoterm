use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ShellManager {
    active_sessions: HashMap<Uuid, ShellSession>,
    default_shell: String,
}

#[derive(Debug, Clone)]
pub struct ShellSession {
    id: Uuid,
    working_dir: std::path::PathBuf,
    environment: HashMap<String, String>,
}

impl ShellManager {
    pub fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
            default_shell: Self::detect_shell(),
        }
    }

    pub async fn execute_command(&self, command: String) -> (String, i32) {
        let mut cmd = Command::new(&self.default_shell);
        cmd.arg("-c")
           .arg(&command)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();

                let stdout_reader = BufReader::new(stdout);
                let stderr_reader = BufReader::new(stderr);

                let mut output = String::new();
                let mut error_output = String::new();

                // Read stdout
                let mut stdout_lines = stdout_reader.lines();
                while let Ok(Some(line)) = stdout_lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }

                // Read stderr
                let mut stderr_lines = stderr_reader.lines();
                while let Ok(Some(line)) = stderr_lines.next_line().await {
                    error_output.push_str(&line);
                    error_output.push('\n');
                }

                let exit_status = child.wait().await.unwrap_or_else(|_| {
                    std::process::ExitStatus::from_raw(1)
                });

                let exit_code = exit_status.code().unwrap_or(1);
                
                let combined_output = if !error_output.is_empty() {
                    format!("{}\n{}", output, error_output)
                } else {
                    output
                };

                (combined_output, exit_code)
            }
            Err(e) => {
                (format!("Failed to execute command: {}", e), 1)
            }
        }
    }

    pub async fn execute_interactive_command(&mut self, command: String) -> tokio::sync::mpsc::Receiver<String> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let shell = self.default_shell.clone();
        tokio::spawn(async move {
            let mut cmd = Command::new(shell);
            cmd.arg("-c")
               .arg(command)
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());

            if let Ok(mut child) = cmd.spawn() {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        if tx.send(line).await.is_err() {
                            break;
                        }
                    }
                }
                
                let _ = child.wait().await;
            }
        });

        rx
    }

    fn detect_shell() -> String {
        std::env::var("SHELL")
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "cmd".to_string()
                } else {
                    "/bin/sh".to_string()
                }
            })
    }

    pub fn create_session(&mut self) -> Uuid {
        let session = ShellSession {
            id: Uuid::new_v4(),
            working_dir: std::env::current_dir().unwrap_or_default(),
            environment: std::env::vars().collect(),
        };
        
        let id = session.id;
        self.active_sessions.insert(id, session);
        id
    }

    pub fn get_session(&self, id: &Uuid) -> Option<&ShellSession> {
        self.active_sessions.get(id)
    }
}

impl ShellSession {
    pub fn set_working_dir(&mut self, path: std::path::PathBuf) {
        self.working_dir = path;
    }

    pub fn set_env_var(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }

    pub fn get_working_dir(&self) -> &std::path::PathBuf {
        &self.working_dir
    }
}
