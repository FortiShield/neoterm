use anyhow::Result;
use std::collections::HashMap;
use std::env;

pub struct AIContext {
    pub cwd: String,
    pub env_vars: Option<HashMap<String, String>>,
    pub recent_commands: Vec<String>,
    pub selected_text: Option<String>,
    // Add more context fields as needed, e.g., open files, terminal output, etc.
}

impl AIContext {
    pub fn new() -> Self {
        Self {
            cwd: String::new(),
            env_vars: None,
            recent_commands: Vec::new(),
            selected_text: None,
        }
    }

    pub async fn update_current_state(&mut self) -> Result<()> {
        // Update Current Working Directory
        self.cwd = env::current_dir()?
            .to_string_lossy()
            .into_owned();

        // Update Environment Variables (consider filtering sensitive ones)
        let env_map: HashMap<String, String> = env::vars().collect();
        self.env_vars = Some(env_map);

        // TODO: Implement mechanisms to get recent commands and selected text from the terminal UI/shell
        // For now, these are placeholders.
        // self.recent_commands = get_recent_commands_from_shell_history().await?;
        // self.selected_text = get_selected_text_from_ui().await?;

        Ok(())
    }

    pub fn add_recent_command(&mut self, command: String) {
        self.recent_commands.push(command);
        // Keep history limited, e.g., to last 10 commands
        if self.recent_commands.len() > 10 {
            self.recent_commands.remove(0);
        }
    }

    pub fn set_selected_text(&mut self, text: Option<String>) {
        self.selected_text = text;
    }
}
