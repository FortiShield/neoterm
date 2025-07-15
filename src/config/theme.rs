use serde::{Deserialize, Serialize};
use iced::{Color, Font};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: ColorScheme,
    pub typography: Typography,
    pub spacing: Spacing,
    pub effects: Effects,
    pub custom_themes: HashMap<String, CustomTheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    // Background colors
    pub background: ColorValue,
    pub surface: ColorValue,
    pub surface_variant: ColorValue,
    
    // Text colors
    pub text: ColorValue,
    pub text_secondary: ColorValue,
    pub text_disabled: ColorValue,
    
    // Terminal colors
    pub terminal_background: ColorValue,
    pub terminal_foreground: ColorValue,
    pub terminal_cursor: ColorValue,
    pub terminal_selection: ColorValue,
    
    // ANSI colors (16 colors)
    pub ansi_colors: AnsiColors,
    
    // UI element colors
    pub primary: ColorValue,
    pub secondary: ColorValue,
    pub accent: ColorValue,
    pub success: ColorValue,
    pub warning: ColorValue,
    pub error: ColorValue,
    
    // Interactive states
    pub hover: ColorValue,
    pub active: ColorValue,
    pub focus: ColorValue,
    pub disabled: ColorValue,
    
    // Borders and dividers
    pub border: ColorValue,
    pub divider: ColorValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsiColors {
    // Normal colors (0-7)
    pub black: ColorValue,
    pub red: ColorValue,
    pub green: ColorValue,
    pub yellow: ColorValue,
    pub blue: ColorValue,
    pub magenta: ColorValue,
    pub cyan: ColorValue,
    pub white: ColorValue,
    
    // Bright colors (8-15)
    pub bright_black: ColorValue,
    pub bright_red: ColorValue,
    pub bright_green: ColorValue,
    pub bright_yellow: ColorValue,
    pub bright_blue: ColorValue,
    pub bright_magenta: ColorValue,
    pub bright_cyan: ColorValue,
    pub bright_white: ColorValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
    
    // Terminal-specific typography
    pub terminal_font_family: String,
    pub terminal_font_size: f32,
    pub terminal_line_height: f32,
    
    // UI typography
    pub ui_font_family: String,
    pub ui_font_size: f32,
    
    // Font weights
    pub font_weight_normal: u16,
    pub font_weight_bold: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    
    // Component-specific spacing
    pub block_padding: f32,
    pub input_padding: f32,
    pub button_padding: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effects {
    pub border_radius: f32,
    pub shadow_blur: f32,
    pub shadow_offset: (f32, f32),
    pub shadow_color: ColorValue,
    
    // Animation settings
    pub animation_duration: u64, // milliseconds
    pub animation_easing: String,
    
    // Terminal effects
    pub cursor_blink: bool,
    pub cursor_blink_rate: u64, // milliseconds
    pub text_smoothing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorValue {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<ColorValue> for Color {
    fn from(color: ColorValue) -> Self {
        Color::from_rgba(color.r, color.g, color.b, color.a)
    }
}

impl From<Color> for ColorValue {
    fn from(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub colors: ColorScheme,
    pub typography: Option<Typography>,
    pub effects: Option<Effects>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Default Dark".to_string(),
            colors: ColorScheme::default_dark(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            effects: Effects::default(),
            custom_themes: HashMap::new(),
        }
    }
}

impl ColorScheme {
    pub fn default_dark() -> Self {
        Self {
            background: ColorValue { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            surface: ColorValue { r: 0.15, g: 0.15, b: 0.15, a: 1.0 },
            surface_variant: ColorValue { r: 0.2, g: 0.2, b: 0.2, a: 1.0 },
            
            text: ColorValue { r: 0.9, g: 0.9, b: 0.9, a: 1.0 },
            text_secondary: ColorValue { r: 0.7, g: 0.7, b: 0.7, a: 1.0 },
            text_disabled: ColorValue { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
            
            terminal_background: ColorValue { r: 0.05, g: 0.05, b: 0.05, a: 1.0 },
            terminal_foreground: ColorValue { r: 0.95, g: 0.95, b: 0.95, a: 1.0 },
            terminal_cursor: ColorValue { r: 0.0, g: 0.8, b: 1.0, a: 1.0 },
            terminal_selection: ColorValue { r: 0.3, g: 0.5, b: 0.8, a: 0.3 },
            
            ansi_colors: AnsiColors::default(),
            
            primary: ColorValue { r: 0.0, g: 0.6, b: 1.0, a: 1.0 },
            secondary: ColorValue { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },
            accent: ColorValue { r: 0.8, g: 0.2, b: 0.8, a: 1.0 },
            success: ColorValue { r: 0.0, g: 0.8, b: 0.4, a: 1.0 },
            warning: ColorValue { r: 1.0, g: 0.6, b: 0.0, a: 1.0 },
            error: ColorValue { r: 1.0, g: 0.2, b: 0.2, a: 1.0 },
            
            hover: ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 0.1 },
            active: ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 0.2 },
            focus: ColorValue { r: 0.0, g: 0.6, b: 1.0, a: 0.5 },
            disabled: ColorValue { r: 0.5, g: 0.5, b: 0.5, a: 0.5 },
            
            border: ColorValue { r: 0.3, g: 0.3, b: 0.3, a: 1.0 },
            divider: ColorValue { r: 0.25, g: 0.25, b: 0.25, a: 1.0 },
        }
    }

    pub fn default_light() -> Self {
        Self {
            background: ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
            surface: ColorValue { r: 0.98, g: 0.98, b: 0.98, a: 1.0 },
            surface_variant: ColorValue { r: 0.95, g: 0.95, b: 0.95, a: 1.0 },
            
            text: ColorValue { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            text_secondary: ColorValue { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },
            text_disabled: ColorValue { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
            
            terminal_background: ColorValue { r: 0.98, g: 0.98, b: 0.98, a: 1.0 },
            terminal_foreground: ColorValue { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            terminal_cursor: ColorValue { r: 0.0, g: 0.4, b: 0.8, a: 1.0 },
            terminal_selection: ColorValue { r: 0.0, g: 0.4, b: 0.8, a: 0.2 },
            
            ansi_colors: AnsiColors::default_light(),
            
            primary: ColorValue { r: 0.0, g: 0.4, b: 0.8, a: 1.0 },
            secondary: ColorValue { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
            accent: ColorValue { r: 0.6, g: 0.0, b: 0.6, a: 1.0 },
            success: ColorValue { r: 0.0, g: 0.6, b: 0.2, a: 1.0 },
            warning: ColorValue { r: 0.8, g: 0.4, b: 0.0, a: 1.0 },
            error: ColorValue { r: 0.8, g: 0.0, b: 0.0, a: 1.0 },
            
            hover: ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 0.05 },
            active: ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 0.1 },
            focus: ColorValue { r: 0.0, g: 0.4, b: 0.8, a: 0.3 },
            disabled: ColorValue { r: 0.5, g: 0.5, b: 0.5, a: 0.5 },
            
            border: ColorValue { r: 0.8, g: 0.8, b: 0.8, a: 1.0 },
            divider: ColorValue { r: 0.85, g: 0.85, b: 0.85, a: 1.0 },
        }
    }
}

impl Default for AnsiColors {
    fn default() -> Self {
        Self {
            black: ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            red: ColorValue { r: 0.8, g: 0.0, b: 0.0, a: 1.0 },
            green: ColorValue { r: 0.0, g: 0.8, b: 0.0, a: 1.0 },
            yellow: ColorValue { r: 0.8, g: 0.8, b: 0.0, a: 1.0 },
            blue: ColorValue { r: 0.0, g: 0.0, b: 0.8, a: 1.0 },
            magenta: ColorValue { r: 0.8, g: 0.0, b: 0.8, a: 1.0 },
            cyan: ColorValue { r: 0.0, g: 0.8, b: 0.8, a: 1.0 },
            white: ColorValue { r: 0.8, g: 0.8, b: 0.8, a: 1.0 },
            
            bright_black: ColorValue { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },
            bright_red: ColorValue { r: 1.0, g: 0.4, b: 0.4, a: 1.0 },
            bright_green: ColorValue { r: 0.4, g: 1.0, b: 0.4, a: 1.0 },
            bright_yellow: ColorValue { r: 1.0, g: 1.0, b: 0.4, a: 1.0 },
            bright_blue: ColorValue { r: 0.4, g: 0.4, b: 1.0, a: 1.0 },
            bright_magenta: ColorValue { r: 1.0, g: 0.4, b: 1.0, a: 1.0 },
            bright_cyan: ColorValue { r: 0.4, g: 1.0, b: 1.0, a: 1.0 },
            bright_white: ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
        }
    }
}

impl AnsiColors {
    pub fn default_light() -> Self {
        Self {
            black: ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            red: ColorValue { r: 0.6, g: 0.0, b: 0.0, a: 1.0 },
            green: ColorValue { r: 0.0, g: 0.6, b: 0.0, a: 1.0 },
            yellow: ColorValue { r: 0.6, g: 0.6, b: 0.0, a: 1.0 },
            blue: ColorValue { r: 0.0, g: 0.0, b: 0.6, a: 1.0 },
            magenta: ColorValue { r: 0.6, g: 0.0, b: 0.6, a: 1.0 },
            cyan: ColorValue { r: 0.0, g: 0.6, b: 0.6, a: 1.0 },
            white: ColorValue { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
            
            bright_black: ColorValue { r: 0.3, g: 0.3, b: 0.3, a: 1.0 },
            bright_red: ColorValue { r: 0.8, g: 0.2, b: 0.2, a: 1.0 },
            bright_green: ColorValue { r: 0.2, g: 0.8, b: 0.2, a: 1.0 },
            bright_yellow: ColorValue { r: 0.8, g: 0.8, b: 0.2, a: 1.0 },
            bright_blue: ColorValue { r: 0.2, g: 0.2, b: 0.8, a: 1.0 },
            bright_magenta: ColorValue { r: 0.8, g: 0.2, b: 0.8, a: 1.0 },
            bright_cyan: ColorValue { r: 0.2, g: 0.8, b: 0.8, a: 1.0 },
            bright_white: ColorValue { r: 0.9, g: 0.9, b: 0.9, a: 1.0 },
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(),
            font_size: 14.0,
            line_height: 1.4,
            letter_spacing: 0.0,
            
            terminal_font_family: "JetBrains Mono".to_string(),
            terminal_font_size: 14.0,
            terminal_line_height: 1.2,
            
            ui_font_family: "Inter".to_string(),
            ui_font_size: 13.0,
            
            font_weight_normal: 400,
            font_weight_bold: 600,
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            
            block_padding: 16.0,
            input_padding: 12.0,
            button_padding: 8.0,
        }
    }
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            border_radius: 8.0,
            shadow_blur: 4.0,
            shadow_offset: (0.0, 2.0),
            shadow_color: ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 0.1 },
            
            animation_duration: 200,
            animation_easing: "ease-out".to_string(),
            
            cursor_blink: true,
            cursor_blink_rate: 500,
            text_smoothing: true,
        }
    }
}

impl ThemeConfig {
    pub fn builtin_themes() -> Vec<Self> {
        vec![
            Self::default(), // Default Dark
            Self {
                name: "Default Light".to_string(),
                colors: ColorScheme::default_light(),
                ..Self::default()
            },
            Self::dracula(),
            Self::monokai(),
            Self::solarized_dark(),
            Self::solarized_light(),
        ]
    }

    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            colors: ColorScheme {
                background: ColorValue { r: 0.16, g: 0.16, b: 0.21, a: 1.0 },
                surface: ColorValue { r: 0.20, g: 0.20, b: 0.25, a: 1.0 },
                surface_variant: ColorValue { r: 0.24, g: 0.24, b: 0.29, a: 1.0 },
                
                text: ColorValue { r: 0.95, g: 0.95, b: 0.95, a: 1.0 },
                text_secondary: ColorValue { r: 0.75, g: 0.75, b: 0.75, a: 1.0 },
                text_disabled: ColorValue { r: 0.55, g: 0.55, b: 0.55, a: 1.0 },
                
                terminal_background: ColorValue { r: 0.16, g: 0.16, b: 0.21, a: 1.0 },
                terminal_foreground: ColorValue { r: 0.95, g: 0.95, b: 0.95, a: 1.0 },
                terminal_cursor: ColorValue { r: 1.0, g: 0.47, b: 0.78, a: 1.0 },
                terminal_selection: ColorValue { r: 0.27, g: 0.21, b: 0.31, a: 1.0 },
                
                primary: ColorValue { r: 0.74, g: 0.58, b: 1.0, a: 1.0 },
                secondary: ColorValue { r: 0.55, g: 0.55, b: 0.55, a: 1.0 },
                accent: ColorValue { r: 1.0, g: 0.47, b: 0.78, a: 1.0 },
                success: ColorValue { r: 0.31, g: 0.98, b: 0.48, a: 1.0 },
                warning: ColorValue { r: 0.95, g: 0.76, b: 0.06, a: 1.0 },
                error: ColorValue { r: 1.0, g: 0.34, b: 0.33, a: 1.0 },
                
                ..ColorScheme::default_dark()
            },
            ..Self::default()
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "Monokai".to_string(),
            colors: ColorScheme {
                background: ColorValue { r: 0.16, g: 0.16, b: 0.16, a: 1.0 },
                surface: ColorValue { r: 0.20, g: 0.20, b: 0.20, a: 1.0 },
                surface_variant: ColorValue { r: 0.24, g: 0.24, b: 0.24, a: 1.0 },
                
                primary: ColorValue { r: 0.65, g: 0.89, b: 0.18, a: 1.0 },
                accent: ColorValue { r: 0.96, g: 0.26, b: 0.21, a: 1.0 },
                success: ColorValue { r: 0.65, g: 0.89, b: 0.18, a: 1.0 },
                warning: ColorValue { r: 0.99, g: 0.59, b: 0.12, a: 1.0 },
                error: ColorValue { r: 0.96, g: 0.26, b: 0.21, a: 1.0 },
                
                ..ColorScheme::default_dark()
            },
            ..Self::default()
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            colors: ColorScheme {
                background: ColorValue { r: 0.0, g: 0.17, b: 0.21, a: 1.0 },
                surface: ColorValue { r: 0.03, g: 0.21, b: 0.26, a: 1.0 },
                surface_variant: ColorValue { r: 0.07, g: 0.26, b: 0.31, a: 1.0 },
                
                text: ColorValue { r: 0.51, g: 0.58, b: 0.59, a: 1.0 },
                text_secondary: ColorValue { r: 0.42, g: 0.48, b: 0.51, a: 1.0 },
                
                primary: ColorValue { r: 0.15, g: 0.55, b: 0.82, a: 1.0 },
                accent: ColorValue { r: 0.83, g: 0.21, b: 0.51, a: 1.0 },
                success: ColorValue { r: 0.52, g: 0.60, b: 0.0, a: 1.0 },
                warning: ColorValue { r: 0.71, g: 0.54, b: 0.0, a: 1.0 },
                error: ColorValue { r: 0.86, g: 0.20, b: 0.18, a: 1.0 },
                
                ..ColorScheme::default_dark()
            },
            ..Self::default()
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            name: "Solarized Light".to_string(),
            colors: ColorScheme {
                background: ColorValue { r: 0.99, g: 0.96, b: 0.89, a: 1.0 },
                surface: ColorValue { r: 0.93, g: 0.91, b: 0.84, a: 1.0 },
                surface_variant: ColorValue { r: 0.87, g: 0.85, b: 0.78, a: 1.0 },
                
                text: ColorValue { r: 0.40, g: 0.48, b: 0.51, a: 1.0 },
                text_secondary: ColorValue { r: 0.51, g: 0.58, b: 0.59, a: 1.0 },
                
                primary: ColorValue { r: 0.15, g: 0.55, b: 0.82, a: 1.0 },
                accent: ColorValue { r: 0.83, g: 0.21, b: 0.51, a: 1.0 },
                success: ColorValue { r: 0.52, g: 0.60, b: 0.0, a: 1.0 },
                warning: ColorValue { r: 0.71, g: 0.54, b: 0.0, a: 1.0 },
                error: ColorValue { r: 0.86, g: 0.20, b: 0.18, a: 1.0 },
                
                ..ColorScheme::default_light()
            },
            ..Self::default()
        }
    }
}
