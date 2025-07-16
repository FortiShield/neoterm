use iced::{widget::{column, row, text, button, text_input, scrollable}, Element, Command, Length};
use iced::keyboard::{KeyCode, Modifiers};
use std::collections::HashMap;
use crate::input::Keybinding; // Assuming Keybinding is defined here
use crate::settings::Settings; // Assuming Settings struct is defined

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
        let mut action_keybindings = HashMap::new();
        // Populate with default/current keybindings from settings or a predefined list
        // For demonstration, let's assume some actions
        action_keybindings.insert("Toggle Command Palette".to_string(), Some(Keybinding::Single(KeyCode::P, Modifiers::COMMAND)));
        action_keybindings.insert("Toggle AI Sidebar".to_string(), Some(Keybinding::Single(KeyCode::A, Modifiers::COMMAND)));
        action_keybindings.insert("Run Benchmarks".to_string(), Some(Keybinding::Single(KeyCode::F1, Modifiers::NONE)));
        action_keybindings.insert("Save File".to_string(), Some(Keybinding::Single(KeyCode::S, Modifiers::COMMAND)));
        action_keybindings.insert("Open File".to_string(), Some(Keybinding::Single(KeyCode::O, Modifiers::COMMAND)));


        KeybindingEditor {
            settings,
            action_keybindings,
            selected_action: None,
            new_key_input: String::new(),
            capturing_key: false,
            captured_key: None,
        }
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
