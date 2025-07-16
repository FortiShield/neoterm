use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::ai_client::{ChatMessage, ToolCall, ToolFunction};
use super::tools::{Tool, ToolManager};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl Conversation {
    pub fn new(id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn get_last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Executes tool calls from an AI message.
    pub async fn execute_tool_calls(&mut self, tool_calls: Vec<ToolCall>, tool_manager: &ToolManager) -> Result<Vec<ChatMessage>> {
        let mut tool_messages = Vec::new();
        for tool_call in tool_calls {
            log::info!("Executing tool call: {:?}", tool_call);
            let tool_name = &tool_call.function.name;
            let tool_args = &tool_call.function.arguments;

            match tool_manager.get_tool(tool_name) {
                Some(tool) => {
                    match tool.execute(tool_args.clone()).await {
                        Ok(output) => {
                            log::info!("Tool '{}' executed successfully. Output: {}", tool_name, output);
                            tool_messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: output,
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            });
                        },
                        Err(e) => {
                            log::error!("Error executing tool '{}': {:?}", tool_name, e);
                            tool_messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: format!("Error: {:?}", e),
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            });
                        }
                    }
                },
                None => {
                    log::warn!("Tool '{}' not found.", tool_name);
                    tool_messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: format!("Error: Tool '{}' not found.", tool_name),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
            }
        }
        Ok(tool_messages)
    }
}
