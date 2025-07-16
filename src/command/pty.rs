use anyhow::{Result, anyhow};
use portable_pty::{CommandBuilder, PtySize, PtySystem, MasterPty, Child, ChildKiller};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents output from the PTY.
#[derive(Debug, Clone)]
pub struct PtyOutput {
    pub data: Vec<u8>,
    pub is_stderr: bool, // PTYs typically don't separate stdout/stderr, but this could be for future parsing
}

/// Manages a pseudo-terminal session.
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    _child: Box<dyn Child + Send>, // Keep child handle to prevent it from being dropped
    _child_killer: Option<Arc<dyn ChildKiller + Send + Sync>>, // For graceful termination
    output_receiver: mpsc::Receiver<PtyOutput>,
    input_sender: mpsc::Sender<Vec<u8>>,
}

impl PtySession {
    pub async fn spawn(
        executable: &str,
        args: &[String],
        working_dir: Option<&str>,
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let pty_system = portable_pty::PtySystem::default();

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(executable);
        cmd.args(args);
        if let Some(dir) = working_dir {
            cmd.cwd(dir);
        }
        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd)?;
        let child_killer = child.clone_killer();

        let master = pair.master;
        let mut reader = master.try_clone_reader()?;
        let mut writer = master.try_clone_writer()?;

        let (output_tx, output_rx) = mpsc::channel(100);
        let (input_tx, mut input_rx) = mpsc::channel(100);

        // Read from PTY and send to output_tx
        tokio::spawn(async move {
            let mut buf = vec![0; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        log::debug!("PTY reader got EOF.");
                        break;
                    },
                    Ok(n) => {
                        if output_tx.send(PtyOutput {
                            data: buf[..n].to_vec(),
                            is_stderr: false, // PTYs don't distinguish stdout/stderr
                        }).await.is_err() {
                            log::warn!("PTY output receiver dropped.");
                            break;
                        }
                    },
                    Err(e) => {
                        log::error!("Error reading from PTY: {:?}", e);
                        break;
                    }
                }
            }
        });

        // Read from input_rx and write to PTY
        tokio::spawn(async move {
            while let Some(input_data) = input_rx.recv().await {
                if let Err(e) = writer.write_all(&input_data).await {
                    log::error!("Error writing to PTY: {:?}", e);
                    break;
                }
            }
        });

        Ok(Self {
            master,
            _child: child,
            _child_killer: child_killer.map(Arc::new),
            output_receiver: output_rx,
            input_sender: input_tx,
        })
    }

    /// Reads output from the PTY.
    pub async fn read_output(&mut self) -> Option<PtyOutput> {
        self.output_receiver.recv().await
    }

    /// Writes input to the PTY.
    pub async fn write_input(&self, data: &[u8]) -> Result<()> {
        self.input_sender.send(data.to_vec()).await
            .map_err(|e| anyhow!("Failed to send input to PTY: {:?}", e))
    }

    /// Resizes the PTY.
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    /// Waits for the child process to exit.
    pub async fn wait(&mut self) -> Result<portable_pty::ExitStatus> {
        // portable-pty's Child::wait is blocking, so we need to spawn it.
        let child_handle = self._child.take().ok_or_else(|| anyhow!("Child handle already taken"))?;
        let exit_status = tokio::task::spawn_blocking(move || child_handle.wait())
            .await??;
        Ok(exit_status)
    }

    /// Sends a signal to the child process (e.g., SIGINT).
    pub async fn signal(&self, signal: portable_pty::Signal) -> Result<()> {
        if let Some(killer) = &self._child_killer {
            killer.signal(signal)?;
            Ok(())
        } else {
            Err(anyhow!("No child killer available for signaling."))
        }
    }

    /// Kills the child process.
    pub async fn kill(&self) -> Result<()> {
        if let Some(killer) = &self._child_killer {
            killer.kill()?;
            Ok(())
        } else {
            Err(anyhow!("No child killer available for killing."))
        }
    }
}

pub fn init() {
    log::info!("PTY module initialized.");
}
