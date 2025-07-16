use tokio::sync::mpsc;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

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
