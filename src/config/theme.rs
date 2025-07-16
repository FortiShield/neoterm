use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub description: String,
    pub colors: HashMap<String, String>, // e.g., "background": "#1e1e1e", "foreground": "#d4d4d4"
    pub syntax_highlighting: HashMap<String, String>, // e.g., "keyword": "#569cd6"
    pub ui_elements: HashMap<String, String>, // e.g., "border": "#444444", "selection": "#264f78"
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: "A default dark theme.".to_string(),
            colors: [
                ("background".to_string(), "#1e1e1e".to_string()),
                ("foreground".to_string(), "#d4d4d4".to_string()),
                ("cursor".to_string(), "#ffffff".to_string()),
                ("black".to_string(), "#000000".to_string()),
                ("red".to_string(), "#cd3131".to_string()),
                ("green".to_string(), "#0dbc79".to_string()),
                ("yellow".to_string(), "#e5e510".to_string()),
                ("blue".to_string(), "#2472c8".to_string()),
                ("magenta".to_string(), "#bc3fbc".to_string()),
                ("cyan".to_string(), "#03babc".to_string()),
                ("white".to_string(), "#e0e0e0".to_string()),
                ("bright_black".to_string(), "#666666".to_string()),
                ("bright_red".to_string(), "#f14c4c".to_string()),
                ("bright_green".to_string(), "#23d18b".to_string()),
                ("bright_yellow".to_string(), "#f5f543".to_string()),
                ("bright_blue".to_string(), "#3b8eea".to_string()),
                ("bright_magenta".to_string(), "#d670d6".to_string()),
                ("bright_cyan".to_string(), "#29b8bd".to_string()),
                ("bright_white".to_string(), "#e6e6e6".to_string()),
            ].iter().cloned().collect(),
            syntax_highlighting: [
                ("keyword".to_string(), "#569cd6".to_string()),
                ("string".to_string(), "#ce9178".to_string()),
                ("comment".to_string(), "#6a9955".to_string()),
                ("number".to_string(), "#b5cea8".to_string()),
                ("function".to_string(), "#dcdcaa".to_string()),
                ("variable".to_string(), "#9cdcfe".to_string()),
            ].iter().cloned().collect(),
            ui_elements: [
                ("border".to_string(), "#444444".to_string()),
                ("selection".to_string(), "#264f78".to_string()),
                ("tab_active".to_string(), "#333333".to_string()),
                ("tab_inactive".to_string(), "#252526".to_string()),
                ("status_bar".to_string(), "#007acc".to_string()),
                ("command_palette_bg".to_string(), "#252526".to_string()),
                ("command_palette_fg".to_string(), "#d4d4d4".to_string()),
            ].iter().cloned().collect(),
        }
    }
}

impl Theme {
    pub fn get_color(&self, name: &str) -> Option<&String> {
        self.colors.get(name)
            .or_else(|| self.syntax_highlighting.get(name))
            .or_else(|| self.ui_elements.get(name))
    }
}
