use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::{ThemeConfig, ColorScheme, ColorValue, AnsiColors, Typography, Effects, Spacing};
use iced;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlTheme {
    pub name: String,
    pub colors: YamlColors,
    // Add other theme properties like fonts, etc.
    pub font: Option<FontConfig>,
    pub effects: Option<EffectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlColors {
    pub background: String,
    pub foreground: String,
    pub primary: String,
    pub secondary: String,
    pub danger: String,
    pub text: String,
    pub border: String,
    // Add more specific colors if needed (e.g., syntax highlighting colors)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalColors {
    pub normal: AnsiColorSet,
    pub bright: AnsiColorSet,
    
    // Optional 256-color palette
    pub palette: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsiColorSet {
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiColors {
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
    pub info: Option<String>,
    pub surface: Option<String>,
    pub surface_variant: Option<String>,
    pub outline: Option<String>,
    pub shadow: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: Option<String>,
    pub size: Option<f32>,
    pub weight: Option<String>, // "normal", "bold", "light", etc.
    pub style: Option<String>,  // "normal", "italic"
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectConfig {
    pub border_radius: Option<f32>,
    pub shadow_blur: Option<f32>,
    pub shadow_offset: Option<(f32, f32)>,
    pub transparency: Option<f32>,
    pub blur: Option<bool>,
    pub animations: Option<bool>,
}

impl YamlTheme {
    /// Parse YAML theme from string
    pub fn from_yaml(yaml_str: &str) -> Result<Self, YamlThemeError> {
        serde_yaml::from_str(yaml_str)
            .map_err(|e| YamlThemeError::ParseError(e.to_string()))
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, YamlThemeError> {
        serde_yaml::to_string(self)
            .map_err(|e| YamlThemeError::SerializeError(e.to_string()))
    }

    /// Load from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, YamlThemeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| YamlThemeError::IoError(e.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Save to file
    pub fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), YamlThemeError> {
        let yaml_str = self.to_yaml()?;
        std::fs::write(path, yaml_str)
            .map_err(|e| YamlThemeError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Convert to internal ThemeConfig
    pub fn to_theme_config(&self) -> Result<ThemeConfig, YamlThemeError> {
        let colors = ColorScheme {
            background: parse_color(&self.colors.background)?,
            surface: self.derive_surface_color()?,
            surface_variant: self.derive_surface_variant_color()?,
            
            text: parse_color(&self.colors.foreground)?,
            text_secondary: self.derive_text_secondary()?,
            text_disabled: self.derive_text_disabled()?,
            
            terminal_background: parse_color(&self.colors.background)?,
            terminal_foreground: parse_color(&self.colors.foreground)?,
            terminal_cursor: parse_color(&self.colors.primary).unwrap_or_default(),
            terminal_selection: parse_color(&self.colors.secondary).unwrap_or_default(),
            
            ansi_colors: AnsiColors {
                black: parse_color(&self.colors.background)?,
                red: parse_color(&self.colors.danger)?,
                green: parse_color(&self.colors.success)?,
                yellow: parse_color(&self.colors.warning)?,
                blue: parse_color(&self.colors.primary)?,
                magenta: parse_color(&self.colors.secondary)?,
                cyan: parse_color(&self.colors.text)?,
                white: parse_color(&self.colors.foreground)?,
                
                bright_black: parse_color(&self.colors.background)?,
                bright_red: parse_color(&self.colors.danger)?,
                bright_green: parse_color(&self.colors.success)?,
                bright_yellow: parse_color(&self.colors.warning)?,
                bright_blue: parse_color(&self.colors.primary)?,
                bright_magenta: parse_color(&self.colors.secondary)?,
                bright_cyan: parse_color(&self.colors.text)?,
                bright_white: parse_color(&self.colors.foreground)?,
            },
            
            primary: parse_color(&self.colors.primary)?,
            secondary: parse_color(&self.colors.secondary)?,
            accent: parse_color(&self.colors.primary)?,
            success: parse_color(&self.colors.success)?,
            warning: parse_color(&self.colors.warning)?,
            error: parse_color(&self.colors.danger)?,
            
            hover: self.derive_hover_color()?,
            active: self.derive_active_color()?,
            focus: self.derive_focus_color()?,
            disabled: self.derive_disabled_color()?,
            
            border: parse_color(&self.colors.border)?,
            divider: self.derive_divider_color()?,
        };

        let typography = Typography {
            font_family: self.font.as_ref()
                .and_then(|f| f.family.clone())
                .unwrap_or_else(|| "JetBrains Mono".to_string()),
            font_size: self.font.as_ref()
                .and_then(|f| f.size)
                .unwrap_or(14.0),
            line_height: self.font.as_ref()
                .and_then(|f| f.line_height)
                .unwrap_or(1.4),
            letter_spacing: self.font.as_ref()
                .and_then(|f| f.letter_spacing)
                .unwrap_or(0.0),
            ..Typography::default()
        };

        let effects = Effects {
            border_radius: self.effects.as_ref()
                .and_then(|e| e.border_radius)
                .unwrap_or(8.0),
            shadow_blur: self.effects.as_ref()
                .and_then(|e| e.shadow_blur)
                .unwrap_or(4.0),
            shadow_offset: self.effects.as_ref()
                .and_then(|e| e.shadow_offset)
                .unwrap_or((0.0, 2.0)),
            ..Effects::default()
        };

        Ok(ThemeConfig {
            name: self.name.clone(),
            colors,
            typography,
            spacing: Spacing::default(),
            effects,
            custom_themes: HashMap::new(),
        })
    }

    /// Create from internal ThemeConfig
    pub fn from_theme_config(theme: &ThemeConfig) -> Self {
        Self {
            name: theme.name.clone(),
            colors: YamlColors {
                background: color_to_hex(&theme.colors.background),
                foreground: color_to_hex(&theme.colors.text),
                primary: color_to_hex(&theme.colors.primary),
                secondary: color_to_hex(&theme.colors.secondary),
                danger: color_to_hex(&theme.colors.error),
                text: color_to_hex(&theme.colors.text),
                border: color_to_hex(&theme.colors.border),
            },
            font: Some(FontConfig {
                family: Some(theme.typography.font_family.clone()),
                size: Some(theme.typography.font_size),
                weight: None,
                style: None,
                line_height: Some(theme.typography.line_height),
                letter_spacing: Some(theme.typography.letter_spacing),
            }),
            effects: Some(EffectConfig {
                border_radius: Some(theme.effects.border_radius),
                shadow_blur: Some(theme.effects.shadow_blur),
                shadow_offset: Some(theme.effects.shadow_offset),
                transparency: None,
                blur: None,
                animations: None,
            }),
        }
    }

    /// Validate theme completeness and correctness
    pub fn validate(&self) -> Result<(), YamlThemeError> {
        // Check required fields
        parse_color(&self.colors.background).map_err(|_| YamlThemeError::InvalidColor("background".to_string()))?;
        parse_color(&self.colors.foreground).map_err(|_| YamlThemeError::InvalidColor("foreground".to_string()))?;
        parse_color(&self.colors.primary).map_err(|_| YamlThemeError::InvalidColor("primary".to_string()))?;
        parse_color(&self.colors.secondary).map_err(|_| YamlThemeError::InvalidColor("secondary".to_string()))?;
        parse_color(&self.colors.danger).map_err(|_| YamlThemeError::InvalidColor("danger".to_string()))?;
        parse_color(&self.colors.text).map_err(|_| YamlThemeError::InvalidColor("text".to_string()))?;
        parse_color(&self.colors.border).map_err(|_| YamlThemeError::InvalidColor("border".to_string()))?;

        Ok(())
    }

    // Helper methods for deriving colors
    fn derive_surface_color(&self) -> Result<ColorValue, YamlThemeError> {
        let bg = parse_color(&self.colors.background)?;
        Ok(if self.is_dark_theme() {
            lighten_color(&bg, 0.05)
        } else {
            darken_color(&bg, 0.02)
        })
    }

    fn derive_surface_variant_color(&self) -> Result<ColorValue, YamlThemeError> {
        let bg = parse_color(&self.colors.background)?;
        Ok(if self.is_dark_theme() {
            lighten_color(&bg, 0.1)
        } else {
            darken_color(&bg, 0.05)
        })
    }

    fn derive_text_secondary(&self) -> Result<ColorValue, YamlThemeError> {
        let fg = parse_color(&self.colors.foreground)?;
        Ok(ColorValue {
            a: 0.7,
            ..fg
        })
    }

    fn derive_text_disabled(&self) -> Result<ColorValue, YamlThemeError> {
        let fg = parse_color(&self.colors.foreground)?;
        Ok(ColorValue {
            a: 0.5,
            ..fg
        })
    }

    fn derive_hover_color(&self) -> Result<ColorValue, YamlThemeError> {
        Ok(if self.is_dark_theme() {
            ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 0.1 }
        } else {
            ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 0.05 }
        })
    }

    fn derive_active_color(&self) -> Result<ColorValue, YamlThemeError> {
        Ok(if self.is_dark_theme() {
            ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 0.2 }
        } else {
            ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 0.1 }
        })
    }

    fn derive_focus_color(&self) -> Result<ColorValue, YamlThemeError> {
        let accent = parse_color(&self.colors.primary)?;
        Ok(ColorValue {
            a: 0.5,
            ..accent
        })
    }

    fn derive_disabled_color(&self) -> Result<ColorValue, YamlThemeError> {
        Ok(ColorValue { r: 0.5, g: 0.5, b: 0.5, a: 0.5 })
    }

    fn derive_divider_color(&self) -> Result<ColorValue, YamlThemeError> {
        let bg = parse_color(&self.colors.background)?;
        Ok(if self.is_dark_theme() {
            lighten_color(&bg, 0.15)
        } else {
            darken_color(&bg, 0.15)
        })
    }

    fn is_dark_theme(&self) -> bool {
        if let Ok(bg) = parse_color(&self.colors.background) {
            // Calculate luminance
            let luminance = 0.299 * bg.r + 0.587 * bg.g + 0.114 * bg.b;
            luminance < 0.5
        } else {
            true // Default to dark
        }
    }

    pub fn to_iced_theme(&self) -> crate::config::theme::Theme {
        crate::config::theme::Theme {
            background: parse_hex_color(&self.colors.background).unwrap_or(iced::Color::BLACK),
            foreground: parse_hex_color(&self.colors.foreground).unwrap_or(iced::Color::WHITE),
            primary: parse_hex_color(&self.colors.primary).unwrap_or(iced::Color::BLUE),
            secondary: parse_hex_color(&self.colors.secondary).unwrap_or(iced::Color::from_rgb8(100, 100, 100)),
            danger: parse_hex_color(&self.colors.danger).unwrap_or(iced::Color::RED),
            text: parse_hex_color(&self.colors.text).unwrap_or(iced::Color::BLACK),
            border: parse_hex_color(&self.colors.border).unwrap_or(iced::Color::from_rgb8(200, 200, 200)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum YamlThemeError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Serialize error: {0}")]
    SerializeError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid color: {0}")]
    InvalidColor(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid theme format: {0}")]
    InvalidFormat(String),
}

/// Parse color from various formats (hex, rgb, hsl, named)
pub fn parse_color(color_str: &str) -> Result<ColorValue, YamlThemeError> {
    let color_str = color_str.trim();
    
    if color_str.starts_with('#') {
        parse_hex_color(color_str)
    } else if color_str.starts_with("rgb(") {
        parse_rgb_color(color_str)
    } else if color_str.starts_with("rgba(") {
        parse_rgba_color(color_str)
    } else if color_str.starts_with("hsl(") {
        parse_hsl_color(color_str)
    } else {
        parse_named_color(color_str)
    }
}

fn parse_hex_color(hex: &str) -> Result<ColorValue, YamlThemeError> {
    let hex = hex.trim_start_matches('#');
    
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            (r, g, b, 255)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| YamlThemeError::InvalidColor(hex.to_string()))?;
            (r, g, b, a)
        }
        _ => return Err(YamlThemeError::InvalidColor(hex.to_string())),
    };

    Ok(ColorValue {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    })
}

fn parse_rgb_color(rgb: &str) -> Result<ColorValue, YamlThemeError> {
    let content = rgb.trim_start_matches("rgb(").trim_end_matches(')');
    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
    
    if parts.len() != 3 {
        return Err(YamlThemeError::InvalidColor(rgb.to_string()));
    }

    let r: f32 = parts[0].parse().map_err(|_| YamlThemeError::InvalidColor(rgb.to_string()))?;
    let g: f32 = parts[1].parse().map_err(|_| YamlThemeError::InvalidColor(rgb.to_string()))?;
    let b: f32 = parts[2].parse().map_err(|_| YamlThemeError::InvalidColor(rgb.to_string()))?;

    Ok(ColorValue {
        r: (r / 255.0).clamp(0.0, 1.0),
        g: (g / 255.0).clamp(0.0, 1.0),
        b: (b / 255.0).clamp(0.0, 1.0),
        a: 1.0,
    })
}

fn parse_rgba_color(rgba: &str) -> Result<ColorValue, YamlThemeError> {
    let content = rgba.trim_start_matches("rgba(").trim_end_matches(')');
    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
    
    if parts.len() != 4 {
        return Err(YamlThemeError::InvalidColor(rgba.to_string()));
    }

    let r: f32 = parts[0].parse().map_err(|_| YamlThemeError::InvalidColor(rgba.to_string()))?;
    let g: f32 = parts[1].parse().map_err(|_| YamlThemeError::InvalidColor(rgba.to_string()))?;
    let b: f32 = parts[2].parse().map_err(|_| YamlThemeError::InvalidColor(rgba.to_string()))?;
    let a: f32 = parts[3].parse().map_err(|_| YamlThemeError::InvalidColor(rgba.to_string()))?;

    Ok(ColorValue {
        r: (r / 255.0).clamp(0.0, 1.0),
        g: (g / 255.0).clamp(0.0, 1.0),
        b: (b / 255.0).clamp(0.0, 1.0),
        a: a.clamp(0.0, 1.0),
    })
}

fn parse_hsl_color(hsl: &str) -> Result<ColorValue, YamlThemeError> {
    let content = hsl.trim_start_matches("hsl(").trim_end_matches(')');
    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
    
    if parts.len() != 3 {
        return Err(YamlThemeError::InvalidColor(hsl.to_string()));
    }

    let h: f32 = parts[0].parse().map_err(|_| YamlThemeError::InvalidColor(hsl.to_string()))?;
    let s: f32 = parts[1].trim_end_matches('%').parse().map_err(|_| YamlThemeError::InvalidColor(hsl.to_string()))?;
    let l: f32 = parts[2].trim_end_matches('%').parse().map_err(|_| YamlThemeError::InvalidColor(hsl.to_string()))?;

    let (r, g, b) = hsl_to_rgb(h / 360.0, s / 100.0, l / 100.0);

    Ok(ColorValue { r, g, b, a: 1.0 })
}

fn parse_named_color(name: &str) -> Result<ColorValue, YamlThemeError> {
    match name.to_lowercase().as_str() {
        "black" => Ok(ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
        "white" => Ok(ColorValue { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
        "red" => Ok(ColorValue { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
        "green" => Ok(ColorValue { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }),
        "blue" => Ok(ColorValue { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }),
        "yellow" => Ok(ColorValue { r: 1.0, g: 1.0, b: 0.0, a: 1.0 }),
        "cyan" => Ok(ColorValue { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }),
        "magenta" => Ok(ColorValue { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }),
        "gray" | "grey" => Ok(ColorValue { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }),
        "transparent" => Ok(ColorValue { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
        _ => Err(YamlThemeError::InvalidColor(name.to_string())),
    }
}

/// Convert ColorValue to hex string
pub fn color_to_hex(color: &ColorValue) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8
    )
}

/// Lighten a color by a factor
pub fn lighten_color(color: &ColorValue, factor: f32) -> ColorValue {
    ColorValue {
        r: (color.r + (1.0 - color.r) * factor).clamp(0.0, 1.0),
        g: (color.g + (1.0 - color.g) * factor).clamp(0.0, 1.0),
        b: (color.b + (1.0 - color.b) * factor).clamp(0.0, 1.0),
        a: color.a,
    }
}

/// Darken a color by a factor
pub fn darken_color(color: &ColorValue, factor: f32) -> ColorValue {
    ColorValue {
        r: (color.r * (1.0 - factor)).clamp(0.0, 1.0),
        g: (color.g * (1.0 - factor)).clamp(0.0, 1.0),
        b: (color.b * (1.0 - factor)).clamp(0.0, 1.0),
        a: color.a,
    }
}

/// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 1.0 / 6.0 {
        (c, x, 0.0)
    } else if h < 2.0 / 6.0 {
        (x, c, 0.0)
    } else if h < 3.0 / 6.0 {
        (0.0, c, x)
    } else if h < 4.0 / 6.0 {
        (0.0, x, c)
    } else if h < 5.0 / 6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

fn parse_hex_color_iced(hex: &str) -> Option<iced::Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(iced::Color::from_rgb8(r, g, b))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_colors() {
        assert!(parse_color("#ff0000").is_ok());
        assert!(parse_color("#f00").is_ok());
        assert!(parse_color("#ff0000ff").is_ok());
    }

    #[test]
    fn test_parse_rgb_colors() {
        assert!(parse_color("rgb(255, 0, 0)").is_ok());
        assert!(parse_color("rgba(255, 0, 0, 0.5)").is_ok());
    }

    #[test]
    fn test_yaml_theme_conversion() {
        let yaml_str = r#"
name: "Test Theme"
colors:
  background: "#2f343f"
  foreground: "#d3dae3"
  primary: "#009688"
  secondary: "#61bc3b"
  danger: "#9c3528"
  text: "#f3b43a"
  border: "#0d68a8"
font:
  family: "JetBrains Mono"
  size: 14.0
  line_height: 1.4
  letter_spacing: 0.0
effects:
  border_radius: 8.0
  shadow_blur: 4.0
  shadow_offset: [0.0, 2.0]
"#;

        let theme = YamlTheme::from_yaml(yaml_str).unwrap();
        assert_eq!(theme.name, "Test Theme");
        assert!(theme.validate().is_ok());
        
        let theme_config = theme.to_theme_config().unwrap();
        assert_eq!(theme_config.name, "Test Theme");
    }
}
