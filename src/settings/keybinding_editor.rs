use iced::{widget::{column, row, text, button, text_input, scrollable}, Element, Command, Length};
use iced::keyboard::{KeyCode, Modifiers};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use crate::config::CONFIG_DIR;
use crate::input::Keybinding; // Assuming Keybinding is defined here
use crate::settings::Settings; // Assuming Settings struct is defined

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    pub id: String,
    pub keys: Vec<String>, // e.g., ["ctrl", "shift", "s"]
    pub command: String,   // Command ID to execute
    pub description: String,
    pub when: Option<String>, // Contextual condition (e.g., "editor_focused")
}

#[derive(Debug, Clone)]
pub enum KeybindingEditorMessage {
    KeybindingSelected(String), // Name of the action
    NewKeyInputChanged(String),
    SetKeybinding,
    DeleteKeybinding(String),
    CancelEdit,
    // Internal messages for key press detection
    KeyPressed(KeyCode, Modifiers),
    KeyReleased,
}

#[derive(Debug, Clone)]
pub struct KeybindingEditor {
    keybindings_file: PathBuf,
    settings: Settings,
    // Map from action name to its current keybinding
    action_keybindings: HashMap<String, Option<Keybinding>>,
    selected_action: Option<String>,
    new_key_input: String,
    // State for capturing new key presses
    capturing_key: bool,
    captured_key: Option<(KeyCode, Modifiers)>,
}

impl KeybindingEditor {
    pub fn new(settings: Settings) -> Self {
        let keybindings_file = CONFIG_DIR.join("keybindings.yaml");
        Self {
            keybindings_file,
            settings,
            action_keybindings: HashMap::new(),
            selected_action: None,
            new_key_input: String::new(),
            capturing_key: false,
            captured_key: None,
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Keybinding editor initialized. Keybindings file: {:?}", self.keybindings_file);
        if !self.keybindings_file.exists() {
            self.save_default_keybindings().await?;
        }
        Ok(())
    }

    async fn save_default_keybindings(&self) -> Result<()> {
        let default_keybindings = vec![
            Keybinding {
                id: "quit_app".to_string(),
                keys: vec!["ctrl".to_string(), "c".to_string()],
                command: "app.quit".to_string(),
                description: "Quit the application".to_string(),
                when: None,
            },
            Keybinding {
                id: "next_block".to_string(),
                keys: vec!["tab".to_string()],
                command: "ui.next_block".to_string(),
                description: "Switch to next UI block".to_string(),
                when: None,
            },
            Keybinding {
                id: "run_benchmarks".to_string(),
                keys: vec!["f1".to_string()],
                command: "app.run_benchmarks".to_string(),
                description: "Run performance benchmarks".to_string(),
                when: None,
            },
        ];
        let contents = serde_yaml::to_string(&default_keybindings)?;
        fs::write(&self.keybindings_file, contents).await?;
        log::info!("Default keybindings saved to {:?}", self.keybindings_file);
        Ok(())
    }

    pub async fn load_keybindings(&self) -> Result<Vec<Keybinding>> {
        let contents = fs::read_to_string(&self.keybindings_file).await?;
        let keybindings: Vec<Keybinding> = serde_yaml::from_str(&contents)?;
        log::info!("Loaded {} keybindings from {:?}", keybindings.len(), self.keybindings_file);
        Ok(keybindings)
    }

    pub async fn save_keybindings(&self, keybindings: &[Keybinding]) -> Result<()> {
        let contents = serde_yaml::to_string_pretty(keybindings)?;
        fs::write(&self.keybindings_file, contents).await?;
        log::info!("Saved {} keybindings to {:?}", keybindings.len(), self.keybindings_file);
        Ok(())
    }

    /// Example: Get command for a given key event.
    pub async fn get_command_for_key_event(&self, key_event: &crossterm::event::KeyEvent) -> Option<String> {
        let loaded_keybindings = self.load_keybindings().await.ok()?;
        for binding in loaded_keybindings {
            let mut matches = true;
            if binding.keys.len() != 1 + key_event.modifiers.bits().count_ones() as usize {
                matches = false;
            } else {
                for key_part in &binding.keys {
                    match key_part.as_str() {
                        "ctrl" => if !key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) { matches = false; break; },
                        "alt" => if !key_event.modifiers.contains(crossterm::event::KeyModifiers::ALT) { matches = false; break; },
                        "shift" => if !key_event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) { matches = false; break; break; },
                        _ => {
                            if let crossterm::event::KeyCode::Char(c) = key_event.code {
                                if key_part != &c.to_string() { matches = false; break; }
                            } else {
                                // Handle other KeyCode variants if needed
                                matches = false; break;
                            }
                        }
                    }
                }
            }
            if matches {
                log::debug!("Matched keybinding: {} -> {}", binding.keys.join("+"), binding.command);
                return Some(binding.command);
            }
        }
        None
    }

    pub fn update(&mut self, message: KeybindingEditorMessage) -> Command<KeybindingEditorMessage> {
        match message {
            KeybindingEditorMessage::KeybindingSelected(action_name) => {
                self.selected_action = Some(action_name);
                self.new_key_input.clear();
                self.capturing_key = true; // Start capturing key for this action
                self.captured_key = None;
                Command::none()
            }
            KeybindingEditorMessage::NewKeyInputChanged(value) => {
                self.new_key_input = value;
                Command::none()
            }
            KeybindingEditorMessage::SetKeybinding => {
                if let Some(action_name) = self.selected_action.take() {
                    if let Some((key_code, modifiers)) = self.captured_key.take() {
                        let new_binding = Keybinding::Single(key_code, modifiers);
                        self.action_keybindings.insert(action_name.clone(), Some(new_binding));
                        println!("Set keybinding for {}: {:?}", action_name, new_binding);
                        // In a real app, save to settings file
                    } else {
                        println!("No key captured to set for action: {}", action_name);
                    }
                }
                self.capturing_key = false;
                self.new_key_input.clear();
                Command::none()
            }
            KeybindingEditorMessage::DeleteKeybinding(action_name) => {
                self.action_keybindings.insert(action_name.clone(), None);
                println!("Deleted keybinding for {}", action_name);
                // In a real app, save to settings file
                Command::none()
            }
            KeybindingEditorMessage::CancelEdit => {
                self.selected_action = None;
                self.new_key_input.clear();
                self.capturing_key = false;
                self.captured_key = None;
                Command::none()
            }
            KeybindingEditorMessage::KeyPressed(key_code, modifiers) => {
                if self.capturing_key {
                    self.captured_key = Some((key_code, modifiers));
                    self.new_key_input = format!("{:?} + {:?}", modifiers, key_code);
                    // Automatically set the keybinding after capture (optional, could require explicit click)
                    // return Command::perform(async {}, |_| KeybindingEditorMessage::SetKeybinding);
                }
                Command::none()
            }
            KeybindingEditorMessage::KeyReleased => {
                // Can be used to finalize key capture if not done on KeyPressed
                Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<KeybindingEditorMessage> {
        let mut rows = column![
            text("Keybinding Editor").size(24).width(Length::Fill),
            text("Click an action to set/change its keybinding.").size(16).width(Length::Fill),
            text("Press ESC to cancel key capture.").size(14).width(Length::Fill).style(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            iced::widget::horizontal_rule(1),
        ].spacing(10);

        let mut keybinding_list = column![].spacing(5);

        for (action, binding_opt) in self.action_keybindings.iter() {
            let binding_text = match binding_opt {
                Some(Keybinding::Single(key, mods)) => format!("{:?} + {:?}", mods, key),
                Some(Keybinding::Chord(k1, m1, k2, m2)) => format!("{:?} + {:?}, {:?} + {:?}", m1, k1, m2, k2),
                None => "None".to_string(),
            };

            let action_button = button(text(action.clone()))
                .on_press(KeybindingEditorMessage::KeybindingSelected(action.clone()))
                .width(Length::FillPortion(2));

            let current_binding_text = text(binding_text).width(Length::FillPortion(2));

            let delete_button = button(text("Delete"))
                .on_press(KeybindingEditorMessage::DeleteKeybinding(action.clone()))
                .style(iced::theme::Button::Destructive)
                .width(Length::Shrink);

            keybinding_list = keybinding_list.push(
                row![
                    action_button,
                    current_binding_text,
                    delete_button,
                ].spacing(10).align_items(iced::Alignment::Center)
            );
        }

        rows = rows.push(scrollable(keybinding_list).height(Length::FillPortion(1)));

        if let Some(selected_action) = &self.selected_action {
            let capture_ui = column![
                text(format!("Setting keybinding for: {}", selected_action)).size(20),
                text_input("Press new key combination...", &self.new_key_input)
                    .on_input(KeybindingEditorMessage::NewKeyInputChanged)
                    .width(Length::Fill)
                    .padding(10)
                    .size(18),
                row![
                    button(text("Set")).on_press(KeybindingEditorMessage::SetKeybinding),
                    button(text("Cancel")).on_press(KeybindingEditorMessage::CancelEdit),
                ].spacing(10)
            ].spacing(10).padding(20).align_items(iced::Alignment::Center);
            rows = rows.push(capture_ui);
        }

        rows.into()
    }

    pub fn is_capturing_key(&self) -> bool {
        self.capturing_key
    }
}

pub fn init() {
    println!("settings/keybinding_editor module loaded");
}
