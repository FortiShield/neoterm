use iced::{Element, widget::{column, row, text, button, text_input, scrollable}};
use std::collections::HashMap;
use crate::config::{KeyBindings, KeyBinding, Action, Modifier};

#[derive(Debug, Clone)]
pub struct KeyBindingEditor {
    keybindings: KeyBindings,
    editing_binding: Option<String>,
    new_binding_name: String,
    search_query: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    EditBinding(String),
    UpdateBinding(String, KeyBinding),
    DeleteBinding(String),
    AddBinding,
    NewBindingNameChanged(String),
    SearchChanged(String),
    ResetToDefaults,
    CancelEdit,
}

impl KeyBindingEditor {
    pub fn new(keybindings: KeyBindings) -> Self {
        Self {
            keybindings,
            editing_binding: None,
            new_binding_name: String::new(),
            search_query: String::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Option<KeyBindings> {
        match message {
            Message::EditBinding(name) => {
                self.editing_binding = Some(name);
                None
            }
            Message::UpdateBinding(name, binding) => {
                self.keybindings.bindings.insert(name, binding);
                self.editing_binding = None;
                Some(self.keybindings.clone())
            }
            Message::DeleteBinding(name) => {
                self.keybindings.bindings.remove(&name);
                Some(self.keybindings.clone())
            }
            Message::AddBinding => {
                if !self.new_binding_name.is_empty() {
                    let binding = KeyBinding {
                        key: "".to_string(),
                        modifiers: vec![],
                        action: Action::Command("".to_string()),
                        when: None,
                    };
                    self.keybindings.bindings.insert(self.new_binding_name.clone(), binding);
                    self.new_binding_name.clear();
                    Some(self.keybindings.clone())
                } else {
                    None
                }
            }
            Message::NewBindingNameChanged(name) => {
                self.new_binding_name = name;
                None
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                None
            }
            Message::ResetToDefaults => {
                self.keybindings = KeyBindings::default();
                Some(self.keybindings.clone())
            }
            Message::CancelEdit => {
                self.editing_binding = None;
                None
            }
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        column![
            text("Keybinding Editor").size(20),
            text("Customize your keybindings here."),
            // Search and add new binding
            row![
                text_input("Search bindings...", &self.search_query)
                    .on_input(Message::SearchChanged)
                    .width(iced::Length::Fill),
                text_input("New binding name...", &self.new_binding_name)
                    .on_input(Message::NewBindingNameChanged),
                button("Add")
                    .on_press(Message::AddBinding)
            ].spacing(8),
            
            // Key bindings list
            scrollable(
                column(
                    self.filtered_bindings()
                        .into_iter()
                        .map(|(name, binding)| self.create_binding_row(name, binding))
                        .collect::<Vec<_>>()
                )
                .spacing(8)
            ).height(iced::Length::Fixed(400.0)),
            
            // Actions
            row![
                button("Reset to Defaults")
                    .on_press(Message::ResetToDefaults),
            ].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn filtered_bindings(&self) -> Vec<(String, KeyBinding)> {
        self.keybindings
            .bindings
            .iter()
            .filter(|(name, _)| {
                if self.search_query.is_empty() {
                    true
                } else {
                    name.to_lowercase().contains(&self.search_query.to_lowercase())
                }
            })
            .map(|(name, binding)| (name.clone(), binding.clone()))
            .collect()
    }

    fn create_binding_row(&self, name: String, binding: KeyBinding) -> Element<Message> {
        let is_editing = self.editing_binding.as_ref() == Some(&name);
        
        if is_editing {
            self.create_editing_row(name, binding)
        } else {
            self.create_display_row(name, binding)
        }
    }

    fn create_display_row(&self, name: String, binding: KeyBinding) -> Element<Message> {
        let key_combo = self.format_key_combination(&binding);
        let action_desc = self.format_action(&binding.action);
        
        iced::widget::container(
            row![
                text(&name).width(iced::Length::Fixed(150.0)),
                text(key_combo).width(iced::Length::Fixed(150.0)),
                text(action_desc).width(iced::Length::Fill),
                button("Edit")
                    .on_press(Message::EditBinding(name.clone())),
                button("Delete")
                    .on_press(Message::DeleteBinding(name.clone()))
                    .style(button::danger),
            ]
            .spacing(8)
            .align_items(iced::Alignment::Center)
        )
        .padding(8)
        .style(|theme| iced::widget::container::Appearance {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                color: theme.palette().text.scale_alpha(0.1),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn create_editing_row(&self, name: String, binding: KeyBinding) -> Element<Message> {
        iced::widget::container(
            column![
                row![
                    text("Name:").width(iced::Length::Fixed(80.0)),
                    text(&name)
                ].spacing(8),
                
                row![
                    text("Key:").width(iced::Length::Fixed(80.0)),
                    text_input("Key...", &binding.key)
                ].spacing(8),
                
                row![
                    text("Action:").width(iced::Length::Fixed(80.0)),
                    text(self.format_action(&binding.action))
                ].spacing(8),
                
                row![
                    button("Save")
                        .on_press(Message::UpdateBinding(name.clone(), binding)),
                    button("Cancel")
                        .on_press(Message::CancelEdit),
                ].spacing(8),
            ]
            .spacing(8)
        )
        .padding(12)
        .style(|theme| iced::widget::container::Appearance {
            background: Some(theme.palette().primary.scale_alpha(0.1).into()),
            border: iced::Border {
                color: theme.palette().primary,
                width: 2.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn format_key_combination(&self, binding: &KeyBinding) -> String {
        let mut parts = Vec::new();
        
        for modifier in &binding.modifiers {
            match modifier {
                Modifier::Ctrl => parts.push("Ctrl"),
                Modifier::Alt => parts.push("Alt"),
                Modifier::Shift => parts.push("Shift"),
                Modifier::Super => parts.push("Super"),
            }
        }
        
        if !binding.key.is_empty() {
            parts.push(&binding.key);
        }
        
        parts.join(" + ")
    }

    fn format_action(&self, action: &Action) -> String {
        match action {
            Action::NewTab => "New Tab".to_string(),
            Action::CloseTab => "Close Tab".to_string(),
            Action::NextTab => "Next Tab".to_string(),
            Action::PreviousTab => "Previous Tab".to_string(),
            Action::Copy => "Copy".to_string(),
            Action::Paste => "Paste".to_string(),
            Action::Find => "Find".to_string(),
            Action::ToggleFullscreen => "Toggle Fullscreen".to_string(),
            Action::ToggleSettings => "Toggle Settings".to_string(),
            Action::Quit => "Quit".to_string(),
            Action::Command(cmd) => format!("Command: {}", cmd),
            _ => "Unknown".to_string(),
        }
    }
}
