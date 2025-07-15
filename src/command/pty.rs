use portable_pty::{PtySize, CommandBuilder, PtyPair, MasterPty, SlavePty};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use vte::{Parser, Perform};
use std::collections::HashMap;

#[derive(Debug)]
pub enum CommandStatus {
    Running,
    Completed(i32),
    Failed(String),
    Killed,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String, // Plain text from VTE
    pub stderr: String, // Plain text from VTE
    pub status: CommandStatus,
}

// A simple VTE event handler that strips ANSI and collects plain text
struct VteOutputHandler {
    buffer: String,
}

impl VteOutputHandler {
    fn new() -> Self {
        VteOutputHandler {
            buffer: String::new(),
        }
    }

    fn take_buffer(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }
}

impl Perform for VteOutputHandler {
    fn print(&mut self, c: char) {
        self.buffer.push(c);
    }

    // Implement other Perform methods as no-ops or for debugging if needed
    fn execute(&mut self, _byte: u8) {}
    fn hook(&mut self, _params: &[i64], _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(&mut self, _params: &[i64], _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn set_mode(&mut self, _mode: vte::ansi::Mode) {}
    fn unset_mode(&mut self, _mode: vte::ansi::Mode) {}
    fn set_dec_private_mode(&mut self, _mode: vte::ansi::DecPrivateMode) {}
    fn unset_dec_private_mode(&mut self, _mode: vte::ansi::DecPrivateMode) {}
    fn csi_dispatch_ext(&mut self, _params: &[i64], _intermediates: &[u8], _ignore: bool, _c: char, _ext: vte::ext::Ext) {}
}

pub struct PtyManager {
    // No fields needed for now, as it's stateless for command execution
}

impl PtyManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<mpsc::Receiver<CommandOutput>, anyhow::Error> {
        let pty_system = portable_pty::PtySystem::native()?;
        let pty_pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);

        if let Some(vars) = env_vars {
            for (key, value) in vars {
                cmd.env(key, value);
            }
        }

        let mut child = pty_pair.slave.spawn_command(cmd)?;

        let (tx, rx) = mpsc::channel(100); // Channel to send output chunks

        let mut reader = pty_pair.master.try_clone_reader()?;
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let mut buf = vec![0; 4096];
            let mut parser = Parser::new();
            let mut handler = VteOutputHandler::new();

            loop {
                tokio::select! {
                    read_result = reader.read(&mut buf) => {
                        match read_result {
                            Ok(0) => {
                                // EOF, reader closed
                                break;
                            }
                            Ok(bytes_read) => {
                                for &byte in &buf[..bytes_read] {
                                    parser.advance(&mut handler, byte);
                                }
                                let plain_text = handler.take_buffer();
                                if !plain_text.is_empty() {
                                    let _ = tx_clone.send(CommandOutput {
                                        stdout: plain_text,
                                        stderr: String::new(), // For simplicity, assume all output is stdout for now
                                        status: CommandStatus::Running,
                                    }).await;
                                }
                            }
                            Err(e) => {
                                eprintln!("PTY read error: {:?}", e);
                                let _ = tx_clone.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: String::new(),
                                    status: CommandStatus::Failed(e.to_string()),
                                }).await;
                                break;
                            }
                        }
                    }
                    _ = child.wait() => {
                        // Child process exited
                        break;
                    }
                }
            }

            // After loop, check child exit status
            match child.wait().await {
                Ok(exit_status) => {
                    let _ = tx_clone.send(CommandOutput {
                        stdout: String::new(),
                        stderr: String::new(),
                        status: CommandStatus::Completed(exit_status.code().unwrap_or(1)),
                    }).await;
                }
                Err(e) => {
                    let _ = tx_clone.send(CommandOutput {
                        stdout: String::new(),
                        stderr: String::new(),
                        status: CommandStatus::Failed(e.to_string()),
                    }).await;
                }
            }
        });

        Ok(rx)
    }
}
