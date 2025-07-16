use crate::command::CommandManager;
use crate::virtual_fs::VirtualFileSystem;
use crate::watcher::Watcher;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AIContext {
    command_manager: Arc<CommandManager>,
    virtual_file_system: Arc<VirtualFileSystem>,
    watcher: Arc<Watcher>,
    pub current_working_directory: Option<PathBuf>,
    pub recent_commands: Vec<String>,
    pub file_system_summary: String,
    pub redact_sensitive_info: bool, // New field to control redaction
}

impl AIContext {
    pub fn new(command_manager: Arc<CommandManager>, virtual_file_system: Arc<VirtualFileSystem>, watcher: Arc<Watcher>, redact_sensitive_info: bool) -> Self {
        Self {
            command_manager,
            virtual_file_system,
            watcher,
            current_working_directory: None,
            recent_commands: Vec::new(),
            file_system_summary: String::new(),
            redact_sensitive_info,
        }
    }

    pub async fn update_current_state(&mut self) -> Result<()> {
        // Update current working directory
        self.current_working_directory = std::env::current_dir().ok();

        // Update recent commands (simplified, assuming CommandManager tracks this)
        // In a real scenario, CommandManager would expose a method to get recent commands.
        // For now, let's simulate it or fetch from a history file if available.
        // self.recent_commands = self.command_manager.get_recent_commands().await?;

        // Update file system summary
        let fs_summary = self.virtual_file_system.get_summary().await?;
        self.file_system_summary = self.redact_file_system_summary(&fs_summary);

        Ok(())
    }

    fn redact_file_system_summary(&self, summary: &str) -> String {
        if !self.redact_sensitive_info {
            return summary.to_string();
        }

        let sensitive_patterns = [
            ".git",
            "node_modules",
            "target",
            "dist",
            "build",
            "__pycache__",
            ".env",
            ".ssh",
            ".aws",
            "secrets",
            "private",
            "temp",
            "tmp",
            "cache",
            "logs",
            "node_modules",
            "vendor",
            "bin",
            "obj",
            "out",
            "coverage",
            "test-results",
            "backup",
            "archive",
            "dump",
            "db",
            "database",
            "config.json",
            "credentials.json",
            "api_keys.txt",
            "password.txt",
            "token.txt",
            "id_rsa", // Common SSH key name
            "id_dsa", // Common SSH key name
            "id_ecdsa", // Common SSH key name
            "id_ed25519", // Common SSH key name
        ];

        let mut redacted_summary = summary.to_string();
        for pattern in &sensitive_patterns {
            // Replace full path segments or file names
            redacted_summary = redacted_summary.replace(
                &format!("/{}", pattern),
                &format!("/[REDACTED_{}]", pattern.to_uppercase().replace(".", "").replace("-", "_"))
            );
            redacted_summary = redacted_summary.replace(
                &format!("\\{}", pattern),
                &format!("\\[REDACTED_{}]", pattern.to_uppercase().replace(".", "").replace("-", "_"))
            );
            redacted_summary = redacted_summary.replace(
                &format!(" {}", pattern), // For file names at the end of a line
                &format!(" [REDACTED_{}]", pattern.to_uppercase().replace(".", "").replace("-", "_"))
            );
            redacted_summary = redacted_summary.replace(
                &format!("{}\n", pattern), // For file names at the end of a line
                &format!("[REDACTED_{}]\n", pattern.to_uppercase().replace(".", "").replace("-", "_"))
            );
            redacted_summary = redacted_summary.replace(
                &format!("{}\r\n", pattern), // For file names at the end of a line (Windows)
                &format!("[REDACTED_{}]\r\n", pattern.to_uppercase().replace(".", "").replace("-", "_"))
            );
        }
        redacted_summary
    }
}
