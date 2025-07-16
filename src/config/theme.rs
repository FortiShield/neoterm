use serde::{Deserialize, Serialize};
use iced::{Color, Font};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: HashMap<String, String>, // Hex codes or named colors
    pub syntax_highlighting: HashMap<String, String>, // For code blocks
    pub terminal_colors: TerminalColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        // A simple default light theme
        let mut colors = HashMap::new();
        colors.insert("primary".to_string(), "#007bff".to_string());
        colors.insert("secondary".to_string(), "#6c757d".to_string());
        colors.insert("success".to_string(), "#28a745".to_string());
        colors.insert("danger".to_string(), "#dc3545".to_string());
        colors.insert("warning".to_string(), "#ffc107".to_string());
        colors.insert("info".to_string(), "#17a2b8".to_string());

        let mut syntax_highlighting = HashMap::new();
        syntax_highlighting.insert("keyword".to_string(), "#0000ff".to_string());
        syntax_highlighting.insert("string".to_string(), "#a31515".to_string());
        syntax_highlighting.insert("comment".to_string(), "#008000".to_string());
        syntax_highlighting.insert("number".to_string(), "#098677".to_string());

        Self {
            name: "Default Light".to_string(),
            colors,
            syntax_highlighting,
            terminal_colors: TerminalColors {
                background: "#FFFFFF".to_string(),
                foreground: "#000000".to_string(),
                cursor: "#000000".to_string(),
                selection: "#B4D5FF".to_string(),
                black: "#000000".to_string(),
                red: "#CD3131".to_string(),
                green: "#0BCB0B".to_string(),
                yellow: "#E5E510".to_string(),
                blue: "#2472C8".to_string(),
                magenta: "#BC3FBC".to_string(),
                cyan: "#0ADBBF".to_string(),
                white: "#E5E5E5".to_string(),
                bright_black: "#666666".to_string(),
                bright_red: "#F14C4C".to_string(),
                bright_green: "#17A717".to_string(),
                bright_yellow: "#F5F543".to_string(),
                bright_blue: "#3B8EEA".to_string(),
                bright_magenta: "#D670D6".to_string(),
                bright_cyan: "#1ADCEF".to_string(),
                bright_white: "#FFFFFF".to_string(),
            },
        }
    }
}

pub fn init() {
    println!("config/theme module loaded");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub danger: Color,
    pub text: Color,
    pub border: Color,
    // Add more theme colors/styles as needed
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::from_rgb8(240, 240, 240),
            foreground: Color::BLACK,
            primary: Color::from_rgb8(0, 120, 215),
            secondary: Color::from_rgb8(100, 100, 100),
            danger: Color::from_rgb8(200, 0, 0),
            text: Color::BLACK,
            border: Color::from_rgb8(200, 200, 200),
        }
    }
}
