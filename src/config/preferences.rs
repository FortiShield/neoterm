use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use tokio::fs;
use super::CONFIG_DIR;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub general: GeneralPreferences,
    pub terminal: TerminalPreferences,
    pub editor: EditorPreferences,
    pub ui: UiPreferences,
    pub performance: PerformancePreferences,
    pub privacy: PrivacyPreferences,
    pub font_size: u16,
    pub shell: String,
    pub enable_ai_suggestions: bool,
    pub enable_telemetry: bool,
    pub terminal_rows: u16,
    pub terminal_cols: u16,
    pub enable_ligatures: bool,
    pub scrollback_lines: u32,
    pub default_working_directory: Option<String>,
    pub enable_transparency: bool,
    pub transparency_level: f32,
    pub enable_bell: bool,
    pub paste_on_middle_click: bool,
    pub confirm_exit: bool,
    pub auto_update_check: bool,
    pub max_history_size: u32,
    pub enable_fuzzy_search: bool,
    pub enable_ai_agent: bool,
    pub enable_cloud_sync: bool,
    pub enable_session_sharing: bool,
    pub enable_plugins: bool,
    pub default_environment_profile: Option<String>,
    pub terminal_font_size: u16,
    pub terminal_font_family: String,
    pub theme_name: String,
    pub enable_ai_assistant: bool,
    pub enable_collaboration: bool,
    pub ai_api_key: Option<String>,
    pub ai_model: String,
    pub ai_temperature: f32,
    pub ai_max_tokens: u32,
    pub ai_provider_type: String, // New field for AI provider type
    pub fallback_ai_provider_type: Option<String>, // New: Fallback AI provider type
    pub fallback_ai_model: Option<String>, // New: Fallback AI model
    pub redact_sensitive_info: bool, // New: Redact sensitive info before sending to AI
    pub local_only_ai_mode: bool, // New: Force AI to use local models only
    pub keybindings_file: String,
    pub enable_performance_profiling: bool,
    pub enable_wasm_plugins: bool,
    pub enable_lua_plugins: bool,
    pub enable_virtual_fs: bool,
    pub enable_graphql_api: bool,
    pub enable_natural_language_detection: bool,
    pub enable_markdown_preview: bool,
    pub enable_syntax_tree: bool,
    pub enable_fuzzy_match: bool,
    pub enable_lpc_support: bool,
    pub enable_mcq_support: bool,
    pub enable_asset_macro: bool,
    pub enable_workflow_engine: bool,
    pub enable_debugger: bool,
    pub enable_drive_integration: bool,
    pub enable_watcher: bool,
    pub enable_websocket_server: bool,
    pub enable_cli_integration: bool,
    pub enable_distribution_packaging: bool, // New field for Phase 10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralPreferences {
    pub startup_behavior: StartupBehavior,
    pub default_shell: Option<String>,
    pub working_directory: WorkingDirectoryBehavior,
    pub auto_update: bool,
    pub telemetry_enabled: bool,
    pub crash_reporting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StartupBehavior {
    NewSession,
    RestoreLastSession,
    CustomCommand(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkingDirectoryBehavior {
    Home,
    LastUsed,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalPreferences {
    pub scrollback_lines: usize,
    pub scroll_sensitivity: f32,
    pub mouse_reporting: bool,
    pub copy_on_select: bool,
    pub paste_on_right_click: bool,
    pub confirm_before_closing: bool,
    pub bell_behavior: BellBehavior,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
    pub word_separators: String,
    pub url_detection: bool,
    pub hyperlink_behavior: HyperlinkBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BellBehavior {
    None,
    Visual,
    Audio,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperlinkBehavior {
    Click,
    CtrlClick,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPreferences {
    pub vim_mode: bool,
    pub auto_suggestions: bool,
    pub syntax_highlighting: bool,
    pub auto_completion: bool,
    pub bracket_matching: bool,
    pub indent_size: usize,
    pub tab_width: usize,
    pub insert_spaces: bool,
    pub trim_whitespace: bool,
    pub auto_save: bool,
    pub word_wrap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    pub show_tab_bar: TabBarVisibility,
    pub show_title_bar: bool,
    pub show_menu_bar: bool,
    pub compact_mode: bool,
    pub transparency: f32,
    pub blur_background: bool,
    pub animations_enabled: bool,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub zoom_level: f32,
    // New fields for Appearance settings
    pub sync_with_os_theme: bool,
    pub app_icon: String, // e.g., "Default", "Custom1"
    pub open_new_windows_custom_size: bool,
    pub window_opacity: f32,
    pub window_blur_radius: f32,
    pub input_type: InputType,
    pub input_position: InputPosition,
    pub dim_inactive_panes: bool,
    pub focus_follows_mouse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TabBarVisibility {
    Always,
    WhenMultiple,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePreferences {
    pub gpu_acceleration: bool,
    pub vsync: bool,
    pub max_fps: Option<u32>,
    pub memory_limit: Option<usize>,
    pub background_throttling: bool,
    pub lazy_rendering: bool,
    pub texture_atlas_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyPreferences {
    pub history_enabled: bool,
    pub history_limit: usize,
    pub clear_history_on_exit: bool,
    pub incognito_mode: bool,
    pub log_level: LogLevel,
    pub share_usage_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub bindings: HashMap<String, KeyBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    pub modifiers: Vec<Modifier>,
    pub action: Action,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Super,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    SplitHorizontal,
    SplitVertical,
    CloseSplit,
    Copy,
    Paste,
    Cut,
    SelectAll,
    Find,
    FindNext,
    FindPrevious,
    ScrollUp,
    ScrollDown,
    ScrollToTop,
    ScrollToBottom,
    ToggleFullscreen,
    ToggleSettings,
    Quit,
    Command(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub plugin_settings: HashMap<String, serde_json::Value>,
    pub auto_update_plugins: bool,
    pub allow_unsigned_plugins: bool,
}

// New enums for Appearance settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    Universal,
    Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputPosition {
    PinToBottom,
    // Add other options if needed, e.g., Floating
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            general: GeneralPreferences::default(),
            terminal: TerminalPreferences::default(),
            editor: EditorPreferences::default(),
            ui: UiPreferences::default(),
            performance: PerformancePreferences::default(),
            privacy: PrivacyPreferences::default(),
            font_size: 14,
            shell: if cfg!(windows) { "powershell.exe".to_string() } else { "bash".to_string() },
            enable_ai_suggestions: true,
            enable_telemetry: false,
            terminal_rows: 24,
            terminal_cols: 80,
            enable_ligatures: true,
            scrollback_lines: 10000,
            default_working_directory: None,
            enable_transparency: false,
            transparency_level: 0.9,
            enable_bell: true,
            paste_on_middle_click: false,
            confirm_exit: true,
            auto_update_check: true,
            log_level: LogLevel::Info,
            enable_gpu_rendering: true,
            max_history_size: 1000,
            enable_fuzzy_search: true,
            enable_ai_agent: true,
            enable_cloud_sync: false,
            enable_session_sharing: false,
            enable_plugins: true,
            default_environment_profile: None,
            terminal_font_size: 14,
            terminal_font_family: "Fira Code".to_string(),
            theme_name: "nord".to_string(),
            enable_ai_assistant: true,
            enable_collaboration: false,
            ai_api_key: None,
            ai_model: "gpt-4o".to_string(),
            ai_temperature: 0.7,
            ai_max_tokens: 500,
            ai_provider_type: "openai".to_string(), // Default to OpenAI
            fallback_ai_provider_type: Some("ollama".to_string()), // Default fallback to Ollama
            fallback_ai_model: Some("llama2".to_string()), // Default fallback model
            redact_sensitive_info: true, // Default to true for privacy
            local_only_ai_mode: false, // Default to false, allow cloud AI
            keybindings_file: "keybindings.yaml".to_string(),
            enable_performance_profiling: true,
            enable_wasm_plugins: true,
            enable_lua_plugins: true,
            enable_virtual_fs: false,
            enable_graphql_api: false,
            enable_natural_language_detection: false,
            enable_markdown_preview: true,
            enable_syntax_tree: false,
            enable_fuzzy_match: true,
            enable_lpc_support: false,
            enable_mcq_support: false,
            enable_asset_macro: false,
            enable_workflow_engine: true,
            enable_debugger: false,
            enable_drive_integration: false,
            enable_watcher: true,
            enable_websocket_server: false,
            enable_cli_integration: true,
            enable_distribution_packaging: true, // Default to true for new phase
        }
    }
}

impl Default for GeneralPreferences {
    fn default() -> Self {
        Self {
            startup_behavior: StartupBehavior::NewSession,
            default_shell: None,
            working_directory: WorkingDirectoryBehavior::Home,
            auto_update: true,
            telemetry_enabled: false,
            crash_reporting: true,
        }
    }
}

impl Default for TerminalPreferences {
    fn default() -> Self {
        Self {
            scrollback_lines: 10000,
            scroll_sensitivity: 1.0,
            mouse_reporting: true,
            copy_on_select: false,
            paste_on_right_click: true,
            confirm_before_closing: true,
            bell_behavior: BellBehavior::Visual,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            word_separators: " \t\n\"'`()[]{}".to_string(),
            url_detection: true,
            hyperlink_behavior: HyperlinkBehavior::CtrlClick,
        }
    }
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            vim_mode: false,
            auto_suggestions: true,
            syntax_highlighting: true,
            auto_completion: true,
            bracket_matching: true,
            indent_size: 4,
            tab_width: 4,
            insert_spaces: true,
            trim_whitespace: true,
            auto_save: false,
            word_wrap: false,
        }
    }
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            show_tab_bar: TabBarVisibility::WhenMultiple,
            show_title_bar: true,
            show_menu_bar: false,
            compact_mode: false,
            transparency: 1.0,
            blur_background: false,
            animations_enabled: true,
            reduce_motion: false,
            high_contrast: false,
            zoom_level: 1.0,
            // Default values for new fields
            sync_with_os_theme: true,
            app_icon: "Default".to_string(),
            open_new_windows_custom_size: false,
            window_opacity: 1.0, // Represents 100%
            window_blur_radius: 1.0,
            input_type: InputType::Universal,
            input_position: InputPosition::PinToBottom,
            dim_inactive_panes: false,
            focus_follows_mouse: false,
        }
    }
}

impl Default for PerformancePreferences {
    fn default() -> Self {
        Self {
            gpu_acceleration: true,
            vsync: true,
            max_fps: Some(60),
            memory_limit: Some(1024),
            background_throttling: true,
            lazy_rendering: true,
            texture_atlas_size: 1024,
        }
    }
}

impl Default for PrivacyPreferences {
    fn default() -> Self {
        Self {
            history_enabled: true,
            history_limit: 10000,
            clear_history_on_exit: false,
            incognito_mode: false,
            log_level: LogLevel::Info,
            share_usage_data: false,
        }
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        
        bindings.insert("new_tab".to_string(), KeyBinding {
            key: "t".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::NewTab,
            when: None,
        });
        
        bindings.insert("close_tab".to_string(), KeyBinding {
            key: "w".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::CloseTab,
            when: None,
        });
        
        bindings.insert("next_tab".to_string(), KeyBinding {
            key: "Tab".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::NextTab,
            when: None,
        });
        
        bindings.insert("previous_tab".to_string(), KeyBinding {
            key: "Tab".to_string(),
            modifiers: vec![Modifier::Ctrl, Modifier::Shift],
            action: Action::PreviousTab,
            when: None,
        });
        
        bindings.insert("copy".to_string(), KeyBinding {
            key: "c".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::Copy,
            when: None,
        });
        
        bindings.insert("paste".to_string(), KeyBinding {
            key: "v".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::Paste,
            when: None,
        });
        
        bindings.insert("find".to_string(), KeyBinding {
            key: "f".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::Find,
            when: None,
        });
        
        bindings.insert("fullscreen".to_string(), KeyBinding {
            key: "F11".to_string(),
            modifiers: vec![],
            action: Action::ToggleFullscreen,
            when: None,
        });
        
        bindings.insert("settings".to_string(), KeyBinding {
            key: "comma".to_string(),
            modifiers: vec![Modifier::Ctrl],
            action: Action::ToggleSettings,
            when: None,
        });
        
        Self { bindings }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled_plugins: Vec::new(),
            plugin_settings: HashMap::new(),
            auto_update_plugins: true,
            allow_unsigned_plugins: false,
        }
    }
}

impl UserPreferences {
    pub fn path() -> PathBuf {
        CONFIG_DIR.join("preferences.json")
    }

    pub async fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let contents = fs::read_to_string(&path).await?;
            let prefs: UserPreferences = serde_json::from_str(&contents)?;
            log::info!("Preferences loaded from {:?}", path);
            Ok(prefs)
        } else {
            log::info!("Preferences file not found at {:?}, creating default.", path);
            let default_prefs = Self::default();
            default_prefs.save().await?;
            Ok(default_prefs)
        }
    }

    pub async fn save(&self) -> Result<()> {
        let path = Self::path();
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents).await?;
        log::info!("Preferences saved to {:?}", path);
        Ok(())
    }
}

pub fn init() {
    println!("config/preferences module loaded");
}
