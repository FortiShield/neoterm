use portable_pty::{CommandBuilder, PtySize, PtySystem, MasterPty, Child};
use tokio::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vte::{Parser, Perform};
use std::collections::HashMap;

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: CommandStatus,
}

#[derive(Debug)]
pub enum CommandStatus {
    Running,
    Completed(i32),
    Failed(String),
    Killed,
}

pub struct PtyManager {
    pty_system: Box<dyn PtySystem + Send>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            pty_system: portable_pty::get_pty_system().unwrap(),
        }
    }

    pub async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<mpsc::Receiver<CommandOutput>, Box<dyn std::error::Error + Send + Sync>> {
        let (mut master, slave) = self.pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(std::env::current_dir().unwrap()); // Set current working directory

        if let Some(env) = env_vars {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let child = slave.spawn_command(cmd)?;

        // It is important to drop the slave, otherwise the main process can hang
        drop(slave);

        let (tx, rx) = mpsc::channel(100);
        let tx_clone = tx.clone();

        // Read output from the PTY in a separate task
        tokio::spawn(async move {
            let mut reader = master.try_clone_reader().unwrap();
            let mut parser = Parser::new();
            let mut performer = VtePerformer {
                tx: tx_clone,
                current_stdout: String::new(),
                current_stderr: String::new(),
            };

            let mut buf = vec![0; 4096];
            loop {
                tokio::select! {
                    read_result = reader.read(&mut buf) => {
                        match read_result {
                            Ok(0) => { // EOF
                                break;
                            }
                            Ok(n) => {
                                let bytes = &buf[..n];
                                parser.advance(&mut performer, bytes);
                            }
                            Err(e) => {
                                eprintln!("Error reading from PTY: {}", e);
                                let _ = tx.send(CommandOutput {
                                    stdout: String::new(),
                                    stderr: format!("Error reading PTY: {}", e),
                                    status: CommandStatus::Failed(e.to_string()),
                                }).await;
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {
                        // Periodically flush any buffered output
                        performer.flush_output().await;
                    }
                }
            }
            performer.flush_output().await; // Final flush
        });

        // Wait for the child process to complete in another task
        let tx_status = tx.clone();
        tokio::spawn(async move {
            match child.wait().await {
                Ok(exit_status) => {
                    let exit_code = exit_status.exit_code().unwrap_or(-1);
                    let _ = tx_status.send(CommandOutput {
                        stdout: String::new(),
                        stderr: String::new(),
                        status: CommandStatus::Completed(exit_code),
                    }).await;
                }
                Err(e) => {
                    let _ = tx_status.send(CommandOutput {
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

struct VtePerformer {
    tx: mpsc::Sender<CommandOutput>,
    current_stdout: String,
    current_stderr: String,
}

impl VtePerformer {
    async fn flush_output(&mut self) {
        if !self.current_stdout.is_empty() {
            let _ = self.tx.send(CommandOutput {
                stdout: self.current_stdout.drain(..).collect(),
                stderr: String::new(),
                status: CommandStatus::Running,
            }).await;
        }
        if !self.current_stderr.is_empty() {
            let _ = self.tx.send(CommandOutput {
                stdout: String::new(),
                stderr: self.current_stderr.drain(..).collect(),
                status: CommandStatus::Running,
            }).await;
        }
    }
}

impl Perform for VtePerformer {
    fn print(&mut self, c: char) {
        self.current_stdout.push(c);
    }

    fn execute(&mut self, byte: u8) {
        // Handle control characters if necessary
        // For now, just pass them through or ignore
    }

    fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        // Handle hooks
    }

    fn put(&mut self, byte: u8) {
        // Handle raw bytes
    }

    fn escape(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        // Handle escape sequences
    }

    fn csi(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        // Handle CSI sequences
    }

    fn osc(&mut self, params: &[&[u8]], bell_terminated: bool) {
        // Handle OSC sequences
    }

    fn set_mode(&mut self, mode: vte::ansi::Mode) {
        // Handle mode changes
    }

    fn unset_mode(&mut self, mode: vte::ansi::Mode) {
        // Handle mode changes
    }

    fn set_color(&mut self, color: vte::ansi::Color) {
        // Handle color changes
    }

    fn set_hyperlink(&mut self, hyperlink: vte::ansi::Hyperlink) {
        // Handle hyperlinks
    }

    fn set_title(&mut self, title: String) {
        // Handle title changes
    }

    fn set_icon_name(&mut self, name: String) {
        // Handle icon name changes
    }

    fn set_cursor_shape(&mut self, shape: vte::ansi::CursorShape) {
        // Handle cursor shape changes
    }

    fn set_cursor_style(&mut self, style: vte::ansi::CursorStyle) {
        // Handle cursor style changes
    }

    fn set_cursor_visibility(&mut self, visible: bool) {
        // Handle cursor visibility changes
    }

    fn set_keypad_application_mode(&mut self, enabled: bool) {
        // Handle keypad application mode changes
    }

    fn set_mouse_mode(&mut self, mode: vte::ansi::MouseMode) {
        // Handle mouse mode changes
    }

    fn set_charset(&mut self, charset: vte::ansi::Charset, g: vte::ansi::G) {
        // Handle charset changes
    }

    fn set_active_charset(&mut self, g: vte::ansi::G) {
        // Handle active charset changes
    }

    fn set_terminal_id(&mut self, id: vte::ansi::TerminalId) {
        // Handle terminal ID changes
    }

    fn set_device_attributes(&mut self, attrs: vte::ansi::DeviceAttributes) {
        // Handle device attributes changes
    }

    fn set_keyboard_locked(&mut self, locked: bool) {
        // Handle keyboard lock changes
    }

    fn set_bell_suppression(&mut self, enabled: bool) {
        // Handle bell suppression changes
    }

    fn set_bracketed_paste(&mut self, enabled: bool) {
        // Handle bracketed paste changes
    }

    fn set_modify_other_keys(&mut self, mode: vte::ansi::ModifyOtherKeys) {
        // Handle modify other keys changes
    }

    fn set_cursor_blinking(&mut self, enabled: bool) {
        // Handle cursor blinking changes
    }

    fn set_cursor_color(&mut self, color: vte::ansi::Color) {
        // Handle cursor color changes
    }

    fn set_cursor_text_color(&mut self, color: vte::ansi::Color) {
        // Handle cursor text color changes
    }

    fn set_selection_color(&mut self, color: vte::ansi::Color) {
        // Handle selection color changes
    }

    fn set_selection_text_color(&mut self, color: vte::ansi::Color) {
        // Handle selection text color changes
    }

    fn set_underline_color(&mut self, color: vte::ansi::Color) {
        // Handle underline color changes
    }

    fn set_underline_style(&mut self, style: vte::ansi::UnderlineStyle) {
        // Handle underline style changes
    }

    fn set_font_size(&mut self, size: f32) {
        // Handle font size changes
    }

    fn set_font_family(&mut self, family: String) {
        // Handle font family changes
    }

    fn set_font_weight(&mut self, weight: f32) {
        // Handle font weight changes
    }

    fn set_font_style(&mut self, style: vte::ansi::FontStyle) {
        // Handle font style changes
    }

    fn set_line_height(&mut self, height: f32) {
        // Handle line height changes
    }

    fn set_letter_spacing(&mut self, spacing: f32) {
        // Handle letter spacing changes
    }

    fn set_word_spacing(&mut self, spacing: f32) {
        // Handle word spacing changes
    }

    fn set_text_decoration(&mut self, decoration: vte::ansi::TextDecoration) {
        // Handle text decoration changes
    }

    fn set_text_transform(&mut self, transform: vte::ansi::TextTransform) {
        // Handle text transform changes
    }

    fn set_text_shadow(&mut self, shadow: vte::ansi::TextShadow) {
        // Handle text shadow changes
    }

    fn set_text_stroke(&mut self, stroke: vte::ansi::TextStroke) {
        // Handle text stroke changes
    }

    fn set_text_rendering(&mut self, rendering: vte::ansi::TextRendering) {
        // Handle text rendering changes
    }

    fn set_text_overflow(&mut self, overflow: vte::ansi::TextOverflow) {
        // Handle text overflow changes
    }

    fn set_text_wrap(&mut self, wrap: vte::ansi::TextWrap) {
        // Handle text wrap changes
    }

    fn set_text_align(&mut self, align: vte::ansi::TextAlign) {
        // Handle text align changes
    }

    fn set_text_justify(&mut self, justify: vte::ansi::TextJustify) {
        // Handle text justify changes
    }

    fn set_text_indent(&mut self, indent: f32) {
        // Handle text indent changes
    }

    fn set_text_orientation(&mut self, orientation: vte::ansi::TextOrientation) {
        // Handle text orientation changes
    }

    fn set_text_direction(&mut self, direction: vte::ansi::TextDirection) {
        // Handle text direction changes
    }

    fn set_text_combine_upright(&mut self, combine: vte::ansi::TextCombineUpright) {
        // Handle text combine upright changes
    }

    fn set_text_emphasis(&mut self, emphasis: vte::ansi::TextEmphasis) {
        // Handle text emphasis changes
    }

    fn set_text_underline_position(&mut self, position: vte::ansi::TextUnderlinePosition) {
        // Handle text underline position changes
    }

    fn set_text_underline_offset(&mut self, offset: f32) {
        // Handle text underline offset changes
    }

    fn set_text_decoration_line(&mut self, line: vte::ansi::TextDecorationLine) {
        // Handle text decoration line changes
    }

    fn set_text_decoration_style(&mut self, style: vte::ansi::TextDecorationStyle) {
        // Handle text decoration style changes
    }

    fn set_text_decoration_thickness(&mut self, thickness: f32) {
        // Handle text decoration thickness changes
    }

    fn set_text_decoration_skip_ink(&mut self, skip: bool) {
        // Handle text decoration skip ink changes
    }

    fn set_text_emphasis_style(&mut self, style: vte::ansi::TextEmphasisStyle) {
        // Handle text emphasis style changes
    }

    fn set_text_emphasis_position(&mut self, position: vte::ansi::TextEmphasisPosition) {
        // Handle text emphasis position changes
    }

    fn set_text_orientation_vertical(&mut self, vertical: bool) {
        // Handle text orientation vertical changes
    }

    fn set_text_orientation_sideways(&mut self, sideways: bool) {
        // Handle text orientation sideways changes
    }

    fn set_text_orientation_mixed(&mut self, mixed: bool) {
        // Handle text orientation mixed changes
    }

    fn set_text_orientation_upright(&mut self, upright: bool) {
        // Handle text orientation upright changes
    }

    fn set_text_orientation_rotate(&mut self, rotate: f32) {
        // Handle text orientation rotate changes
    }

    fn set_text_orientation_angle(&mut self, angle: f32) {
        // Handle text orientation angle changes
    }

    fn set_text_orientation_auto(&mut self, auto: bool) {
        // Handle text orientation auto changes
    }

    fn set_text_orientation_initial(&mut self, initial: bool) {
        // Handle text orientation initial changes
    }

    fn set_text_orientation_inherit(&mut self, inherit: bool) {
        // Handle text orientation inherit changes
    }

    fn set_text_orientation_unset(&mut self, unset: bool) {
        // Handle text orientation unset changes
    }

    fn set_text_orientation_revert(&mut self, revert: bool) {
        // Handle text orientation revert changes
    }

    fn set_text_orientation_revert_layer(&mut self, revert_layer: bool) {
        // Handle text orientation revert layer changes
    }

    fn set_text_orientation_from_font(&mut self, from_font: bool) {
        // Handle text orientation from font changes
    }

    fn set_text_orientation_sideways_right(&mut self, sideways_right: bool) {
        // Handle text orientation sideways right changes
    }

    fn set_text_orientation_sideways_left(&mut self, sideways_left: bool) {
        // Handle text orientation sideways left changes
    }

    fn set_text_orientation_upright_right(&mut self, upright_right: bool) {
        // Handle text orientation upright right changes
    }

    fn set_text_orientation_upright_left(&mut self, upright_left: bool) {
        // Handle text orientation upright left changes
    }

    fn set_text_orientation_mixed_right(&mut self, mixed_right: bool) {
        // Handle text orientation mixed right changes
    }

    fn set_text_orientation_mixed_left(&mut self, mixed_left: bool) {
        // Handle text orientation mixed left changes
    }

    fn set_text_orientation_auto_right(&mut self, auto_right: bool) {
        // Handle text orientation auto right changes
    }

    fn set_text_orientation_auto_left(&mut self, auto_left: bool) {
        // Handle text orientation auto left changes
    }

    fn set_text_orientation_initial_right(&mut self, initial_right: bool) {
        // Handle text orientation initial right changes
    }

    fn set_text_orientation_initial_left(&mut self, initial_left: bool) {
        // Handle text orientation initial left changes
    }

    fn set_text_orientation_inherit_right(&mut self, inherit_right: bool) {
        // Handle text orientation inherit right changes
    }

    fn set_text_orientation_inherit_left(&mut self, inherit_left: bool) {
        // Handle text orientation inherit left changes
    }

    fn set_text_orientation_unset_right(&mut self, unset_right: bool) {
        // Handle text orientation unset right changes
    }

    fn set_text_orientation_unset_left(&mut self, unset_left: bool) {
        // Handle text orientation unset left changes
    }

    fn set_text_orientation_revert_right(&mut self, revert_right: bool) {
        // Handle text orientation revert right changes
    }

    fn set_text_orientation_revert_left(&mut self, revert_left: bool) {
        // Handle text orientation revert left changes
    }

    fn set_text_orientation_revert_layer_right(&mut self, revert_layer_right: bool) {
        // Handle text orientation revert layer right changes
    }

    fn set_text_orientation_revert_layer_left(&mut self, revert_layer_left: bool) {
        // Handle text orientation revert layer left changes
    }

    fn set_text_orientation_from_font_right(&mut self, from_font_right: bool) {
        // Handle text orientation from font right changes
    }

    fn set_text_orientation_from_font_left(&mut self, from_font_left: bool) {
        // Handle text orientation from font left changes
    }
}
