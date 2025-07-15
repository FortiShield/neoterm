use iced::{Element, widget::{column, row, text, button, container, scrollable, pick_list, slider, checkbox, text_input}};
use crate::{Message, config::*};

pub mod theme_editor;
pub mod keybinding_editor;

use theme_editor::ThemeEditor;
use keybinding_editor::KeyBindingEditor;

#[derive(Debug, Clone)]
pub struct SettingsView {
    pub active_tab: SettingsTab,
    pub config: AppConfig,
    pub theme_editor: ThemeEditor,
    pub keybinding_editor: KeyBindingEditor,
    pub unsaved_changes: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    General,
    Appearance,
    Terminal,
    Editor,
    KeyBindings,
    Performance,
    Privacy,
    Plugins,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    TabChanged(SettingsTab),
    ConfigChanged(ConfigChange),
    ThemeChanged(String),
    CustomThemeCreated(String),
    KeyBindingChanged(String, KeyBinding),
    ResetToDefaults,
    ImportConfig,
    ExportConfig,
    Save,
    Cancel,
    ThemeEditor(theme_editor::Message),
    KeyBindingEditor(keybinding_editor::Message),
}

#[derive(Debug, Clone)]
pub enum ConfigChange {
    // General
    StartupBehavior(StartupBehavior),
    DefaultShell(String),
    WorkingDirectory(WorkingDirectoryBehavior),
    AutoUpdate(bool),
    TelemetryEnabled(bool),
    
    // Terminal
    ScrollbackLines(usize),
    ScrollSensitivity(f32),
    MouseReporting(bool),
    CopyOnSelect(bool),
    PasteOnRightClick(bool),
    ConfirmBeforeClosing(bool),
    BellBehavior(BellBehavior),
    CursorStyle(CursorStyle),
    CursorBlink(bool),
    
    // Editor
    VimMode(bool),
    AutoSuggestions(bool),
    SyntaxHighlighting(bool),
    AutoCompletion(bool),
    IndentSize(usize),
    TabWidth(usize),
    InsertSpaces(bool),
    
    // UI
    ShowTabBar(TabBarVisibility),
    ShowTitleBar(bool),
    CompactMode(bool),
    Transparency(f32),
    BlurBackground(bool),
    AnimationsEnabled(bool),
    ZoomLevel(f32),
    
    // Performance
    GpuAcceleration(bool),
    Vsync(bool),
    MaxFps(Option<u32>),
    MemoryLimit(Option<usize>),
    
    // Privacy
    HistoryEnabled(bool),
    HistoryLimit(usize),
    ClearHistoryOnExit(bool),
    IncognitoMode(bool),
    LogLevel(LogLevel),
}

impl SettingsView {
    pub fn new(config: AppConfig) -> Self {
        Self {
            active_tab: SettingsTab::General,
            theme_editor: ThemeEditor::new(config.theme.clone()),
            keybinding_editor: KeyBindingEditor::new(config.keybindings.clone()),
            config,
            unsaved_changes: false,
        }
    }

    pub fn update(&mut self, message: SettingsMessage) -> Option<AppConfig> {
        match message {
            SettingsMessage::TabChanged(tab) => {
                self.active_tab = tab;
                None
            }
            SettingsMessage::ConfigChanged(change) => {
                self.apply_config_change(change);
                self.unsaved_changes = true;
                None
            }
            SettingsMessage::ThemeChanged(theme_name) => {
                if let Some(theme) = ThemeConfig::builtin_themes()
                    .into_iter()
                    .find(|t| t.name == theme_name)
                {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
                None
            }
            SettingsMessage::Save => {
                if let Err(e) = self.config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
                self.unsaved_changes = false;
                Some(self.config.clone())
            }
            SettingsMessage::Cancel => {
                // Reload config from disk
                if let Ok(config) = AppConfig::load() {
                    self.config = config.clone();
                    self.unsaved_changes = false;
                    Some(config)
                } else {
                    None
                }
            }
            SettingsMessage::ResetToDefaults => {
                self.config = AppConfig::default();
                self.unsaved_changes = true;
                None
            }
            SettingsMessage::ThemeEditor(msg) => {
                if let Some(theme) = self.theme_editor.update(msg) {
                    self.config.theme = theme;
                    self.unsaved_changes = true;
                }
                None
            }
            SettingsMessage::KeyBindingEditor(msg) => {
                if let Some(keybindings) = self.keybinding_editor.update(msg) {
                    self.config.keybindings = keybindings;
                    self.unsaved_changes = true;
                }
                None
            }
            _ => None,
        }
    }

    fn apply_config_change(&mut self, change: ConfigChange) {
        match change {
            ConfigChange::StartupBehavior(behavior) => {
                self.config.preferences.general.startup_behavior = behavior;
            }
            ConfigChange::DefaultShell(shell) => {
                self.config.preferences.general.default_shell = Some(shell);
            }
            ConfigChange::AutoUpdate(enabled) => {
                self.config.preferences.general.auto_update = enabled;
            }
            ConfigChange::ScrollbackLines(lines) => {
                self.config.preferences.terminal.scrollback_lines = lines;
            }
            ConfigChange::ScrollSensitivity(sensitivity) => {
                self.config.preferences.terminal.scroll_sensitivity = sensitivity;
            }
            ConfigChange::CopyOnSelect(enabled) => {
                self.config.preferences.terminal.copy_on_select = enabled;
            }
            ConfigChange::VimMode(enabled) => {
                self.config.preferences.editor.vim_mode = enabled;
            }
            ConfigChange::AutoSuggestions(enabled) => {
                self.config.preferences.editor.auto_suggestions = enabled;
            }
            ConfigChange::Transparency(value) => {
                self.config.preferences.ui.transparency = value;
            }
            ConfigChange::GpuAcceleration(enabled) => {
                self.config.preferences.performance.gpu_acceleration = enabled;
            }
            // Add other config changes...
            _ => {}
        }
    }

    pub fn view(&self) -> Element<SettingsMessage> {
        let tabs = self.create_tabs();
        let content = self.create_content();
        let actions = self.create_actions();

        container(
            column![
                tabs,
                scrollable(content).height(iced::Length::Fill),
                actions
            ]
            .spacing(16)
        )
        .padding(24)
        .into()
    }

    fn create_tabs(&self) -> Element<SettingsMessage> {
        let tabs = vec![
            ("General", SettingsTab::General),
            ("Appearance", SettingsTab::Appearance),
            ("Terminal", SettingsTab::Terminal),
            ("Editor", SettingsTab::Editor),
            ("Key Bindings", SettingsTab::KeyBindings),
            ("Performance", SettingsTab::Performance),
            ("Privacy", SettingsTab::Privacy),
            ("Plugins", SettingsTab::Plugins),
        ];

        row(
            tabs.into_iter()
                .map(|(label, tab)| {
                    button(text(label))
                        .on_press(SettingsMessage::TabChanged(tab.clone()))
                        .style(if self.active_tab == tab {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .into()
                })
                .collect::<Vec<_>>()
        )
        .spacing(8)
        .into()
    }

    fn create_content(&self) -> Element<SettingsMessage> {
        match self.active_tab {
            SettingsTab::General => self.create_general_settings(),
            SettingsTab::Appearance => self.create_appearance_settings(),
            SettingsTab::Terminal => self.create_terminal_settings(),
            SettingsTab::Editor => self.create_editor_settings(),
            SettingsTab::KeyBindings => self.create_keybinding_settings(),
            SettingsTab::Performance => self.create_performance_settings(),
            SettingsTab::Privacy => self.create_privacy_settings(),
            SettingsTab::Plugins => self.create_plugin_settings(),
        }
    }

    fn create_general_settings(&self) -> Element<SettingsMessage> {
        column![
            text("General Settings").size(20),
            
            row![
                text("Startup Behavior:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    vec![
                        StartupBehavior::NewSession,
                        StartupBehavior::RestoreLastSession,
                    ],
                    Some(self.config.preferences.general.startup_behavior.clone()),
                    |behavior| SettingsMessage::ConfigChanged(ConfigChange::StartupBehavior(behavior))
                )
            ].spacing(8),
            
            row![
                text("Default Shell:").width(iced::Length::Fixed(150.0)),
                text_input(
                    "Shell path...",
                    self.config.preferences.general.default_shell.as_deref().unwrap_or("")
                )
                .on_input(|shell| SettingsMessage::ConfigChanged(ConfigChange::DefaultShell(shell)))
            ].spacing(8),
            
            row![
                checkbox(
                    "Auto Update",
                    self.config.preferences.general.auto_update,
                    |enabled| SettingsMessage::ConfigChanged(ConfigChange::AutoUpdate(enabled))
                ),
                text("Automatically check for and install updates")
            ].spacing(8),
            
            row![
                checkbox(
                    "Telemetry",
                    self.config.preferences.general.telemetry_enabled,
                    |enabled| SettingsMessage::ConfigChanged(ConfigChange::TelemetryEnabled(enabled))
                ),
                text("Help improve NeoTerm by sharing anonymous usage data")
            ].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn create_appearance_settings(&self) -> Element<SettingsMessage> {
        let theme_names: Vec<String> = ThemeConfig::builtin_themes()
            .into_iter()
            .map(|t| t.name)
            .collect();

        column![
            text("Appearance Settings").size(20),
            
            row![
                text("Theme:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    theme_names,
                    Some(self.config.theme.name.clone()),
                    SettingsMessage::ThemeChanged
                )
            ].spacing(8),
            
            row![
                text("Font Family:").width(iced::Length::Fixed(150.0)),
                text_input(
                    "Font name...",
                    &self.config.theme.typography.font_family
                )
            ].spacing(8),
            
            row![
                text("Font Size:").width(iced::Length::Fixed(150.0)),
                slider(8.0..=24.0, self.config.theme.typography.font_size, |size| {
                    // This would need to be handled differently in a real implementation
                    SettingsMessage::ConfigChanged(ConfigChange::AutoUpdate(true))
                })
            ].spacing(8),
            
            row![
                text("Transparency:").width(iced::Length::Fixed(150.0)),
                slider(0.0..=1.0, self.config.preferences.ui.transparency, |value| {
                    SettingsMessage::ConfigChanged(ConfigChange::Transparency(value))
                })
            ].spacing(8),
            
            checkbox(
                "Blur Background",
                self.config.preferences.ui.blur_background,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::BlurBackground(enabled))
            ),
            
            checkbox(
                "Enable Animations",
                self.config.preferences.ui.animations_enabled,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::AnimationsEnabled(enabled))
            ),
            
            // Theme editor section
            text("Custom Theme Editor").size(16),
            self.theme_editor.view().map(SettingsMessage::ThemeEditor),
        ]
        .spacing(16)
        .into()
    }

    fn create_terminal_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Terminal Settings").size(20),
            
            row![
                text("Scrollback Lines:").width(iced::Length::Fixed(150.0)),
                slider(1000.0..=50000.0, self.config.preferences.terminal.scrollback_lines as f32, |lines| {
                    SettingsMessage::ConfigChanged(ConfigChange::ScrollbackLines(lines as usize))
                })
            ].spacing(8),
            
            row![
                text("Scroll Sensitivity:").width(iced::Length::Fixed(150.0)),
                slider(0.1..=5.0, self.config.preferences.terminal.scroll_sensitivity, |sensitivity| {
                    SettingsMessage::ConfigChanged(ConfigChange::ScrollSensitivity(sensitivity))
                })
            ].spacing(8),
            
            checkbox(
                "Copy on Select",
                self.config.preferences.terminal.copy_on_select,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::CopyOnSelect(enabled))
            ),
            
            checkbox(
                "Paste on Right Click",
                self.config.preferences.terminal.paste_on_right_click,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::PasteOnRightClick(enabled))
            ),
            
            checkbox(
                "Confirm Before Closing",
                self.config.preferences.terminal.confirm_before_closing,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::ConfirmBeforeClosing(enabled))
            ),
            
            row![
                text("Cursor Style:").width(iced::Length::Fixed(150.0)),
                pick_list(
                    vec![CursorStyle::Block, CursorStyle::Underline, CursorStyle::Bar],
                    Some(self.config.preferences.terminal.cursor_style.clone()),
                    |style| SettingsMessage::ConfigChanged(ConfigChange::CursorStyle(style))
                )
            ].spacing(8),
            
            checkbox(
                "Cursor Blink",
                self.config.preferences.terminal.cursor_blink,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::CursorBlink(enabled))
            ),
        ]
        .spacing(16)
        .into()
    }

    fn create_editor_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Editor Settings").size(20),
            
            checkbox(
                "Vim Mode",
                self.config.preferences.editor.vim_mode,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::VimMode(enabled))
            ),
            
            checkbox(
                "Auto Suggestions",
                self.config.preferences.editor.auto_suggestions,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::AutoSuggestions(enabled))
            ),
            
            checkbox(
                "Syntax Highlighting",
                self.config.preferences.editor.syntax_highlighting,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::SyntaxHighlighting(enabled))
            ),
            
            checkbox(
                "Auto Completion",
                self.config.preferences.editor.auto_completion,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::AutoCompletion(enabled))
            ),
            
            row![
                text("Indent Size:").width(iced::Length::Fixed(150.0)),
                slider(1.0..=8.0, self.config.preferences.editor.indent_size as f32, |size| {
                    SettingsMessage::ConfigChanged(ConfigChange::IndentSize(size as usize))
                })
            ].spacing(8),
            
            row![
                text("Tab Width:").width(iced::Length::Fixed(150.0)),
                slider(1.0..=8.0, self.config.preferences.editor.tab_width as f32, |width| {
                    SettingsMessage::ConfigChanged(ConfigChange::TabWidth(width as usize))
                })
            ].spacing(8),
            
            checkbox(
                "Insert Spaces",
                self.config.preferences.editor.insert_spaces,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::InsertSpaces(enabled))
            ),
        ]
        .spacing(16)
        .into()
    }

    fn create_keybinding_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Key Bindings").size(20),
            self.keybinding_editor.view().map(SettingsMessage::KeyBindingEditor),
        ]
        .spacing(16)
        .into()
    }

    fn create_performance_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Performance Settings").size(20),
            
            checkbox(
                "GPU Acceleration",
                self.config.preferences.performance.gpu_acceleration,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::GpuAcceleration(enabled))
            ),
            
            checkbox(
                "VSync",
                self.config.preferences.performance.vsync,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::Vsync(enabled))
            ),
            
            row![
                text("Max FPS:").width(iced::Length::Fixed(150.0)),
                slider(30.0..=144.0, self.config.preferences.performance.max_fps.unwrap_or(60) as f32, |fps| {
                    SettingsMessage::ConfigChanged(ConfigChange::MaxFps(Some(fps as u32)))
                })
            ].spacing(8),
            
            row![
                text("Memory Limit (MB):").width(iced::Length::Fixed(150.0)),
                slider(256.0..=4096.0, self.config.preferences.performance.memory_limit.unwrap_or(1024) as f32, |mb| {
                    SettingsMessage::ConfigChanged(ConfigChange::MemoryLimit(Some(mb as usize)))
                })
            ].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn create_privacy_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Privacy Settings").size(20),
            
            checkbox(
                "Enable History",
                self.config.preferences.privacy.history_enabled,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::HistoryEnabled(enabled))
            ),
            
            row![
                text("History Limit:").width(iced::Length::Fixed(150.0)),
                slider(100.0..=50000.0, self.config.preferences.privacy.history_limit as f32, |limit| {
                    SettingsMessage::ConfigChanged(ConfigChange::HistoryLimit(limit as usize))
                })
            ].spacing(8),
            
            checkbox(
                "Clear History on Exit",
                self.config.preferences.privacy.clear_history_on_exit,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::ClearHistoryOnExit(enabled))
            ),
            
            checkbox(
                "Incognito Mode",
                self.config.preferences.privacy.incognito_mode,
                |enabled| SettingsMessage::ConfigChanged(ConfigChange::IncognitoMode(enabled))
            ),
        ]
        .spacing(16)
        .into()
    }

    fn create_plugin_settings(&self) -> Element<SettingsMessage> {
        column![
            text("Plugin Settings").size(20),
            text("Plugin management coming soon..."),
        ]
        .spacing(16)
        .into()
    }

    fn create_actions(&self) -> Element<SettingsMessage> {
        row![
            button("Reset to Defaults")
                .on_press(SettingsMessage::ResetToDefaults),
            button("Import Config")
                .on_press(SettingsMessage::ImportConfig),
            button("Export Config")
                .on_press(SettingsMessage::ExportConfig),
            // Spacer
            iced::widget::horizontal_space(iced::Length::Fill),
            button("Cancel")
                .on_press(SettingsMessage::Cancel),
            button("Save")
                .on_press(SettingsMessage::Save)
                .style(if self.unsaved_changes {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(8)
        .into()
    }
}
