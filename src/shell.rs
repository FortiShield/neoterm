use tokio::sync::mpsc;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use portable_pty::{CommandBuilder, PtySize, PtySystem, MasterPty, Child, ChildKiller};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use vte::{Parser, Perform};

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

/// Represents output from the shell's PTY.
#[derive(Debug, Clone)]
pub struct ShellOutput {
    pub data: Vec<u8>,
    pub is_stderr: bool, // PTYs typically don't separate stdout/stderr, but this could be for future parsing
}

/// Events related to the shell session.
#[derive(Debug, Clone)]
pub enum ShellEvent {
    /// New output from the shell.
    Output(ShellOutput),
    /// The shell process has exited.
    Exited(Option<i32>),
    /// An error occurred in the shell session.
    Error(String),
    /// The shell's current working directory changed.
    CwdChanged(String),
    /// The shell's title changed.
    TitleChanged(String),
}

/// Manages a shell session (e.g., bash, zsh, powershell).
pub struct ShellManager {
    pty_session: Arc<Mutex<Option<PtySession>>>,
    event_sender: mpsc::Sender<ShellEvent>,
    // Add state for current working directory, shell history, etc.
}

impl ShellManager {
    pub fn new() -> Self {
        let (tx, _) = mpsc::channel(100); // Dummy sender, will be replaced
        Self {
            pty_session: Arc::new(Mutex::new(None)),
            event_sender: tx,
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Shell manager initialized.");
        Ok(())
    }

    pub fn set_event_sender(&mut self, sender: mpsc::Sender<ShellEvent>) {
        self.event_sender = sender;
    }

    /// Spawns a new shell session.
    pub async fn spawn_shell(&self, shell_path: &str, initial_dir: Option<&str>) -> Result<()> {
        let mut pty_session_guard = self.pty_session.lock().await;
        if pty_session_guard.is_some() {
            log::warn!("A shell session is already active. Terminating existing one.");
            self.terminate_shell().await?;
        }

        log::info!("Spawning shell: {}", shell_path);

        let mut cmd = CommandBuilder::new(shell_path);
        if let Some(dir) = initial_dir {
            cmd.cwd(dir);
        }
        // Set environment variables if needed
        // cmd.env("TERM", "xterm-256color");

        let pty_system = portable_pty::PtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let child = pair.slave.spawn_command(cmd)?;
        let master = pair.master;
        let mut reader = master.try_clone_reader()?;
        let mut writer = master.try_clone_writer()?;

        let (output_tx, mut output_rx) = mpsc::channel(100);
        let (input_tx, mut input_rx) = mpsc::channel(100);

        // Reader task: Reads from PTY and sends to output_tx
        let output_sender_clone = self.event_sender.clone();
        tokio::spawn(async move {
            let mut buf = vec![0; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        log::debug!("Shell PTY reader got EOF.");
                        break;
                    },
                    Ok(n) => {
                        if output_sender_clone.send(ShellEvent::Output(ShellOutput {
                            data: buf[..n].to_vec(),
                            is_stderr: false, // PTYs don't distinguish stdout/stderr
                        })).await.is_err() {
                            log::warn!("Shell output receiver dropped.");
                            break;
                        }
                    },
                    Err(e) => {
                        log::error!("Error reading from shell PTY: {:?}", e);
                        let _ = output_sender_clone.send(ShellEvent::Error(format!("PTY read error: {}", e))).await;
                        break;
                    }
                }
            }
        });

        // Writer task: Reads from input_rx and writes to PTY
        tokio::spawn(async move {
            while let Some(input_data) = input_rx.recv().await {
                if let Err(e) = writer.write_all(&input_data).await {
                    log::error!("Error writing to shell PTY: {:?}", e);
                    break;
                }
            }
        });

        // Child waiter task: Waits for the shell process to exit
        let exit_sender_clone = self.event_sender.clone();
        let child_killer = child.clone_killer();
        tokio::spawn(async move {
            let exit_status = tokio::task::spawn_blocking(move || child.wait())
                .await
                .expect("Failed to join child wait task")
                .expect("Child process wait failed");
            log::info!("Shell process exited with status: {:?}", exit_status);
            let _ = exit_sender_clone.send(ShellEvent::Exited(exit_status.code())).await;
            // Ensure child is killed if it hasn't already
            if let Some(killer) = child_killer {
                let _ = killer.kill(); // Best effort kill
            }
        });

        *pty_session_guard = Some(PtySession {
            master,
            output_receiver: output_rx,
            input_sender: input_tx,
        });

        Ok(())
    }

    /// Sends input to the active shell session.
    pub async fn send_input(&self, input: &[u8]) -> Result<()> {
        let pty_session_guard = self.pty_session.lock().await;
        if let Some(session) = pty_session_guard.as_ref() {
            session.input_sender.send(input.to_vec()).await
                .map_err(|e| anyhow!("Failed to send input to shell: {:?}", e))
        } else {
            Err(anyhow!("No active shell session."))
        }
    }

    /// Reads output from the active shell session.
    pub async fn read_output(&self) -> Option<ShellOutput> {
        let mut pty_session_guard = self.pty_session.lock().await;
        if let Some(session) = pty_session_guard.as_mut() {
            session.output_receiver.recv().await
        } else {
            None
        }
    }

    /// Resizes the active shell's PTY.
    pub async fn resize_pty(&self, rows: u16, cols: u16) -> Result<()> {
        let pty_session_guard = self.pty_session.lock().await;
        if let Some(session) = pty_session_guard.as_ref() {
            session.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
            Ok(())
        } else {
            Err(anyhow!("No active shell session to resize."))
        }
    }

    /// Terminates the active shell session.
    pub async fn terminate_shell(&self) -> Result<()> {
        let mut pty_session_guard = self.pty_session.lock().await;
        if let Some(session) = pty_session_guard.take() {
            log::info!("Terminating shell session.");
            // Portable-pty doesn't expose a direct `kill` on MasterPty,
            // but dropping the MasterPty and Child handles should clean up.
            // If a ChildKiller was captured, use it here.
            // For now, just drop the session.
            drop(session);
            Ok(())
        } else {
            Err(anyhow!("No active shell session to terminate."))
        }
    }
}

/// Internal struct to hold PTY components.
struct PtySession {
    master: Box<dyn MasterPty + Send>,
    output_receiver: mpsc::Receiver<ShellOutput>,
    input_sender: mpsc::Sender<Vec<u8>>,
}

/// A simple VTE (Virtual Terminal Emulator) performer to parse ANSI escape codes.
struct VtePerformer {
    // This struct would hold the terminal buffer state, cursor position, etc.
    // For this stub, we'll just log.
}

impl VtePerformer {
    fn new() -> Self {
        Self {}
    }
}

impl Perform for VtePerformer {
    fn print(&mut self, c: char) {
        // In a real VTE, this would write `c` to the terminal buffer at the current cursor position.
        // print!("{}", c); // For debugging, print directly
    }

    fn execute(&mut self, byte: u8) {
        // Handle C0 control characters
        // log::debug!("Execute: {}", byte);
    }

    fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        // Handle CSI, OSC, DCS, APC, PM, SOS sequences
        // log::debug!("Hook: params={:?}, intermediates={:?}, ignore={}, char={}", params, intermediates, ignore, c);
    }

    fn put(&mut self, byte: u8) {
        // Handle byte directly (e.g., for UTF-8 decoding)
        // log::debug!("Put: {}", byte);
    }

    fn unhook(&mut self) {
        // End of a sequence
        // log::debug!("Unhook");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        // Operating System Command (OSC) sequences
        // Used for changing window title, setting clipboard, etc.
        // Example: OSC 0;title ST (set window title)
        if params.len() > 0 && params[0] == b"0" {
            if let Some(title_bytes) = params.get(1) {
                if let Ok(title) = String::from_utf8(title_bytes.to_vec()) {
                    log::info!("Shell title changed to: {}", title);
                    // Here you would send a ShellEvent::TitleChanged
                }
            }
        }
    }

    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        // Control Sequence Introducer (CSI) sequences
        // Used for cursor movement, text formatting, screen clearing, etc.
        // log::debug!("CSI: params={:?}, intermediates={:?}, ignore={}, char={}", params, intermediates, ignore, c);
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        // Escape sequences
        // log::debug!("ESC: intermediates={:?}, ignore={}, byte={}", intermediates, ignore, byte);
    }
}

pub fn init() {
    println!("shell module loaded");
}
