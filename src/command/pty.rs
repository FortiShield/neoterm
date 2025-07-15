use portable_pty::{PtySize, CommandBuilder, PtyPair, MasterPty, Child};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use std::collections::HashMap;
use ratatui::text::{Line, Span, Style};
use ratatui::style::{Color, Modifier};
use vte::{Parser, Perform};

#[derive(Debug)]
pub enum CommandStatus {
    Running,
    Completed(i32),
    Failed(String),
    Killed,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: Vec<Line>, // Changed to Vec<Line>
    pub stderr: Vec<Line>, // Changed to Vec<Line>
    pub status: CommandStatus,
}

pub struct PtyManager {
    // We might want to store active PTYs if we support multiple concurrent commands
}

impl PtyManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute_command(
        &self,
        cmd: &str,
        args: Vec<&str>,
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<mpsc::Receiver<CommandOutput>, Box<dyn std::error::Error>> {
        let pty_system = portable_pty::PtySystem::native()?;
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd_builder = CommandBuilder::new(cmd);
        cmd_builder.args(args);

        if let Some(env) = env_vars {
            for (key, value) in env {
                cmd_builder.env(key, value);
            }
        }

        let child = pair.slave.spawn_command(cmd_builder)?;

        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.try_clone_writer()?;

        let (tx, rx) = mpsc::channel(100);

        // Spawn a task to read from the PTY and send output chunks
        tokio::spawn(async move {
            let mut buf = vec![0; 4096];
            let mut parser = Parser::new();
            let mut handler = VteEventHandler::new();

            loop {
                tokio::select! {
                    read_result = reader.read(&mut buf) => {
                        match read_result {
                            Ok(0) => { // EOF
                                break;
                            }
                            Ok(bytes_read) => {
                                for &byte in &buf[..bytes_read] {
                                    parser.advance(&mut handler, byte);
                                }
                                // Send accumulated lines
                                if !handler.buffer.is_empty() {
                                    let _ = tx.send(CommandOutput {
                                        stdout: handler.take_buffer(),
                                        stderr: Vec::new(), // stderr is not directly separated by PTY, but we can send it if needed
                                        status: CommandStatus::Running,
                                    }).await;
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(CommandOutput {
                                    stdout: Vec::new(),
                                    stderr: vec![Line::from(format!("Error reading from PTY: {}", e))],
                                    status: CommandStatus::Failed(e.to_string()),
                                }).await;
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        // Periodically send any buffered output even if no new bytes
                        if !handler.buffer.is_empty() {
                            let _ = tx.send(CommandOutput {
                                stdout: handler.take_buffer(),
                                stderr: Vec::new(),
                                status: CommandStatus::Running,
                            }).await;
                        }
                    }
                }
            }

            // Wait for the child process to exit
            match child.wait().await {
                Ok(status) => {
                    let exit_code = status.exit_code().unwrap_or(-1);
                    let _ = tx.send(CommandOutput {
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                        status: CommandStatus::Completed(exit_code),
                    }).await;
                }
                Err(e) => {
                    let _ = tx.send(CommandOutput {
                        stdout: Vec::new(),
                        stderr: vec![Line::from(format!("Error waiting for child process: {}", e))],
                        status: CommandStatus::Failed(e.to_string()),
                    }).await;
                }
            }
        });

        Ok(rx)
    }
}

// VteEventHandler to parse ANSI escape codes and build ratatui::text::Line
struct VteEventHandler {
    buffer: Vec<Line>,
    current_line_spans: Vec<Span<'static>>,
    current_style: Style,
    cursor_x: usize,
    cursor_y: usize,
    // We might need a full grid for more complex cursor movements and overwrites
    // For now, we'll just append lines.
}

impl VteEventHandler {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_line_spans: Vec::new(),
            current_style: Style::default(),
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    fn push_char(&mut self, c: char) {
        if self.current_line_spans.is_empty() || self.current_line_spans.last().unwrap().style != self.current_style {
            self.current_line_spans.push(Span::styled(String::from(c), self.current_style));
        } else {
            let last_span = self.current_line_spans.last_mut().unwrap();
            // This is a bit hacky, ideally Span::content should be mutable or we rebuild
            // For now, we'll convert to String, append, and convert back.
            // A better approach for performance would be to use a Rope or similar for the span content.
            let mut content = last_span.content.to_string();
            content.push(c);
            *last_span = Span::styled(content, self.current_style);
        }
        self.cursor_x += 1;
    }

    fn new_line(&mut self) {
        if !self.current_line_spans.is_empty() {
            self.buffer.push(Line::from(std::mem::take(&mut self.current_line_spans)));
        } else {
            self.buffer.push(Line::from("")); // Push an empty line
        }
        self.cursor_x = 0;
        self.cursor_y += 1;
    }

    fn take_buffer(&mut self) -> Vec<Line<'static>> {
        let mut temp_buffer = std::mem::take(&mut self.buffer);
        if !self.current_line_spans.is_empty() {
            // If there's partial line, add it to the buffer before clearing
            temp_buffer.push(Line::from(std::mem::take(&mut self.current_line_spans)));
        }
        temp_buffer
    }
}

impl Perform for VteEventHandler {
    fn print(&mut self, c: char) {
        self.push_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            vte::params::LF => self.new_line(),
            vte::params::CR => self.cursor_x = 0,
            _ => {} // Ignore other control characters for now
        }
    }

    fn csi(&mut self, params: &vte::Params, intermediates: &[u8], ignore: bool, c: char) {
        if ignore { return; }

        match c {
            'm' => { // SGR - Select Graphic Rendition
                for param in params.iter() {
                    match param {
                        0 => self.current_style = Style::default(), // Reset
                        1 => self.current_style = self.current_style.add_modifier(Modifier::BOLD),
                        2 => self.current_style = self.current_style.add_modifier(Modifier::DIM),
                        3 => self.current_style = self.current_style.add_modifier(Modifier::ITALIC),
                        4 => self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED),
                        5 => self.current_style = self.current_style.add_modifier(Modifier::SLOW_BLINK),
                        7 => self.current_style = self.current_style.add_modifier(Modifier::REVERSED),
                        8 => self.current_style = self.current_style.add_modifier(Modifier::HIDDEN),
                        9 => self.current_style = self.current_style.add_modifier(Modifier::CROSSED_OUT),
                        22 => self.current_style = self.current_style.remove_modifier(Modifier::BOLD | Modifier::DIM),
                        23 => self.current_style = self.current_style.remove_modifier(Modifier::ITALIC),
                        24 => self.current_style = self.current_style.remove_modifier(Modifier::UNDERLINED),
                        25 => self.current_style = self.current_style.remove_modifier(Modifier::SLOW_BLINK),
                        27 => self.current_style = self.current_style.remove_modifier(Modifier::REVERSED),
                        28 => self.current_style = self.current_style.remove_modifier(Modifier::HIDDEN),
                        29 => self.current_style = self.current_style.remove_modifier(Modifier::CROSSED_OUT),
                        // Foreground colors
                        30 => self.current_style = self.current_style.fg(Color::Black),
                        31 => self.current_style = self.current_style.fg(Color::Red),
                        32 => self.current_style = self.current_style.fg(Color::Green),
                        33 => self.current_style = self.current_style.fg(Color::Yellow),
                        34 => self.current_style = self.current_style.fg(Color::Blue),
                        35 => self.current_style = self.current_style.fg(Color::Magenta),
                        36 => self.current_style = self.current_style.fg(Color::Cyan),
                        37 => self.current_style = self.current_style.fg(Color::White),
                        38 => { // 256-color or truecolor foreground
                            if params.len() >= 3 && params[1] == 5 { // 256-color
                                self.current_style = self.current_style.fg(Color::Indexed(params[2] as u8));
                            } else if params.len() >= 5 && params[1] == 2 { // Truecolor
                                self.current_style = self.current_style.fg(Color::Rgb(params[2] as u8, params[3] as u8, params[4] as u8));
                            }
                        }
                        39 => self.current_style = self.current_style.fg(Color::Reset), // Default foreground
                        // Background colors
                        40 => self.current_style = self.current_style.bg(Color::Black),
                        41 => self.current_style = self.current_style.bg(Color::Red),
                        42 => self.current_style = self.current_style.bg(Color::Green),
                        43 => self.current_style = self.current_style.bg(Color::Yellow),
                        44 => self.current_style = self.current_style.bg(Color::Blue),
                        45 => self.current_style = self.current_style.bg(Color::Magenta),
                        46 => self.current_style = self.current_style.bg(Color::Cyan),
                        47 => self.current_style = self.current_style.bg(Color::White),
                        48 => { // 256-color or truecolor background
                            if params.len() >= 3 && params[1] == 5 { // 256-color
                                self.current_style = self.current_style.bg(Color::Indexed(params[2] as u8));
                            } else if params.len() >= 5 && params[1] == 2 { // Truecolor
                                self.current_style = self.current_style.bg(Color::Rgb(params[2] as u8, params[3] as u8, params[4] as u8));
                            }
                        }
                        49 => self.current_style = self.current_style.bg(Color::Reset), // Default background
                        // Bright colors (ANSI bright)
                        90 => self.current_style = self.current_style.fg(Color::DarkGray),
                        91 => self.current_style = self.current_style.fg(Color::LightRed),
                        92 => self.current_style = self.current_style.fg(Color::LightGreen),
                        93 => self.current_style = self.current_style.fg(Color::LightYellow),
                        94 => self.current_style = self.current_style.fg(Color::LightBlue),
                        95 => self.current_style = self.current_style.fg(Color::LightMagenta),
                        96 => self.current_style = self.current_style.fg(Color::LightCyan),
                        97 => self.current_style = self.current_style.fg(Color::White), // Light white
                        // Bright background colors
                        100 => self.current_style = self.current_style.bg(Color::DarkGray),
                        101 => self.current_style = self.current_style.bg(Color::LightRed),
                        102 => self.current_style = self.current_style.bg(Color::LightGreen),
                        103 => self.current_style = self.current_style.bg(Color::LightYellow),
                        104 => self.current_style = self.current_style.bg(Color::LightBlue),
                        105 => self.current_style = self.current_style.bg(Color::LightMagenta),
                        106 => self.current_style = self.current_style.bg(Color::LightCyan),
                        107 => self.current_style = self.current_style.bg(Color::White), // Light white
                        _ => {}
                    }
                }
            }
            'H' => { // CUP - Cursor Position
                let row = params.get(0).unwrap_or(&1).saturating_sub(1) as usize;
                let col = params.get(1).unwrap_or(&1).saturating_sub(1) as usize;
                // For now, we only support appending. More complex cursor movements
                // would require a full grid buffer.
                // If cursor moves to a new line, flush current line.
                if row > self.cursor_y {
                    self.new_line();
                }
                self.cursor_x = col;
                self.cursor_y = row;
            }
            'J' => { // ED - Erase in Display
                // Clear screen. For now, just clear buffer.
                let param = params.get(0).unwrap_or(&0);
                match param {
                    0 => { /* Erase from cursor to end of screen */ },
                    1 => { /* Erase from start of screen to cursor */ },
                    2 => { /* Erase entire screen */ self.buffer.clear(); self.current_line_spans.clear(); self.cursor_x = 0; self.cursor_y = 0; },
                    _ => {}
                }
            }
            'K' => { // EL - Erase in Line
                // Erase line. For now, just clear current line spans.
                let param = params.get(0).unwrap_or(&0);
                match param {
                    0 => { /* Erase from cursor to end of line */ },
                    1 => { /* Erase from start of line to cursor */ },
                    2 => { /* Erase entire line */ self.current_line_spans.clear(); self.cursor_x = 0; },
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn esc(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore { return; }
        // Handle simple escape sequences like `ESC c` (reset)
        if intermediates.is_empty() && byte == b'c' {
            self.current_style = Style::default();
        }
    }

    fn osc(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // Operating System Command (e.g., setting window title)
        // Not handled for now
    }
    
    // Other Perform methods can be left as no-ops for now
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn escape_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
    fn long_csi(&mut self, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn long_escape(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
    fn long_osc(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn long_dcs(&mut self, _params: &[&[u8]], _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn dcs(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _c: char) {}
    fn sos(&mut self) {}
    fn apc(&mut self) {}
    fn pm(&mut self) {}
}
