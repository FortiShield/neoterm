pub mod ai_sidebar;
pub mod collapsible_block;
pub mod command_palette;
pub mod ratatui_block;

use anyhow::Result;

pub fn init() {
    log::info!("UI module initialized.");
}
