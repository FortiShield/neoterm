use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: ConversationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Option<Vec<super::tools::ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub token_count: Option<u32>,
    pub model_used: Option<String>,
    pub provider_used: Option<String>,
}

impl Conversation {
    pub fn new(system_prompt: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            system_prompt,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: ConversationMetadata {
                title: None,
                tags: Vec::new(),
                token_count: None,
                model_used: None,
                provider_used: None,
            },
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

    pub fn get_user_messages(&self) -> Vec<&Message> {
        self.messages
            .iter()
            .filter(|msg| matches!(msg.role, MessageRole::User))
            .collect()
    }

    pub fn get_assistant_messages(&self) -> Vec<&Message> {
        self.messages
            .iter()
            .filter(|msg| matches!(msg.role, MessageRole::Assistant))
            .collect()
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    pub fn update_metadata(&mut self, metadata: ConversationMetadata) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    pub fn set_title(&mut self, title: String) {
        self.metadata.title = Some(title);
        self.updated_at = Utc::now();
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.metadata.tags.retain(|t| t != tag);
        self.updated_at = Utc::now();
    }

    pub fn get_message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn get_token_estimate(&self) -> u32 {
        // Simple token estimation (roughly 4 characters per token)
        let total_chars: usize = self.messages
            .iter()
            .map(|msg| msg.content.len())
            .sum();
        
        (total_chars / 4) as u32
    }

    pub fn truncate_to_limit(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            let start_index = self.messages.len() - max_messages;
            self.messages = self.messages[start_index..].to_vec();
            self.updated_at = Utc::now();
        }
    }

    pub fn export_to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn import_from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Serialize for Conversation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Conversation", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("system_prompt", &self.system_prompt)?;
        state.serialize_field("messages", &self.messages)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("metadata", &self.metadata)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Conversation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConversationData {
            id: Uuid,
            system_prompt: String,
            messages: Vec<Message>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            metadata: ConversationMetadata,
        }

        let data = ConversationData::deserialize(deserializer)?;
        Ok(Conversation {
            id: data.id,
            system_prompt: data.system_prompt,
            messages: data.messages,
            created_at: data.created_at,
            updated_at: data.updated_at,
            metadata: data.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let system_prompt = "You are a helpful assistant".to_string();
        let conv = Conversation::new(system_prompt.clone());
        
        assert_eq!(conv.system_prompt, system_prompt);
        assert_eq!(conv.messages.len(), 0);
        assert!(conv.metadata.title.is_none());
    }

    #[test]
    fn test_add_message() {
        let mut conv = Conversation::new("Test".to_string());
        let message = Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        };

        conv.add_message(message);
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].content, "Hello");
    }

    #[test]
    fn test_message_filtering() {
        let mut conv = Conversation::new("Test".to_string());
        
        conv.add_message(Message {
            role: MessageRole::User,
            content: "User message".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        });

        conv.add_message(Message {
            role: MessageRole::Assistant,
            content: "Assistant message".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        });

        let user_messages = conv.get_user_messages();
        let assistant_messages = conv.get_assistant_messages();

        assert_eq!(user_messages.len(), 1);
        assert_eq!(assistant_messages.len(), 1);
        assert_eq!(user_messages[0].content, "User message");
        assert_eq!(assistant_messages[0].content, "Assistant message");
    }

    #[test]
    fn test_token_estimation() {
        let mut conv = Conversation::new("Test".to_string());
        
        conv.add_message(Message {
            role: MessageRole::User,
            content: "This is a test message with some content".to_string(), // ~40 chars = ~10 tokens
            timestamp: Utc::now(),
            tool_calls: None,
        });

        let estimated_tokens = conv.get_token_estimate();
        assert!(estimated_tokens > 0);
        assert!(estimated_tokens < 20); // Should be around 10 tokens
    }

    #[test]
    fn test_conversation_serialization() {
        let conv = Conversation::new("Test system prompt".to_string());
        let json = conv.export_to_json().unwrap();
        let deserialized = Conversation::import_from_json(&json).unwrap();
        
        assert_eq!(conv.id, deserialized.id);
        assert_eq!(conv.system_prompt, deserialized.system_prompt);
        assert_eq!(conv.messages.len(), deserialized.messages.len());
    }
}
