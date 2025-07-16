use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::agent_mode_eval::tools::{ToolCall, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub title: String,
}

impl Conversation {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            title,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn get_last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

impl Message {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: Utc::now(),
            tool_calls: None,
            tool_results: None,
        }
    }

    pub fn new_tool_call(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant, // Tool calls are from assistant
            content: String::new(),
            timestamp: Utc::now(),
            tool_calls: Some(tool_calls),
            tool_results: None,
        }
    }

    pub fn new_tool_result(tool_call_id: String, content: String, is_error: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: MessageRole::Tool, // Tool results are from tool
            content: content.clone(), // Content is the tool's output
            timestamp: Utc::now(),
            tool_calls: None,
            tool_results: Some(vec![ToolResult { tool_call_id, content, is_error }]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}
