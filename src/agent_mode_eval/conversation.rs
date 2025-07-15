use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub token_count: u32,
    pub cost_estimate: Option<f64>,
    pub model_used: String,
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
                token_count: 0,
                cost_estimate: None,
                model_used: "unknown".to_string(),
            },
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
        
        // Auto-generate title from first user message
        if self.metadata.title.is_none() && matches!(message.role, MessageRole::User) {
            self.metadata.title = Some(self.generate_title(&message.content));
        }
    }

    pub fn get_recent_messages(&self, limit: usize) -> &[Message] {
        let start = if self.messages.len() > limit {
            self.messages.len() - limit
        } else {
            0
        };
        &self.messages[start..]
    }

    pub fn update_metadata(&mut self, token_count: u32, cost: Option<f64>, model: String) {
        self.metadata.token_count += token_count;
        if let Some(cost) = cost {
            self.metadata.cost_estimate = Some(
                self.metadata.cost_estimate.unwrap_or(0.0) + cost
            );
        }
        self.metadata.model_used = model;
        self.updated_at = Utc::now();
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
    }

    pub fn set_title(&mut self, title: String) {
        self.metadata.title = Some(title);
    }

    fn generate_title(&self, content: &str) -> String {
        // Generate a title from the first user message
        let words: Vec<&str> = content.split_whitespace().take(6).collect();
        let title = words.join(" ");
        
        if title.len() > 50 {
            format!("{}...", &title[..47])
        } else {
            title
        }
    }

    pub fn export_markdown(&self) -> String {
        let mut markdown = String::new();
        
        // Header
        markdown.push_str(&format!("# {}\n\n", 
            self.metadata.title.as_deref().unwrap_or("Conversation")));
        
        markdown.push_str(&format!("**Created:** {}\n", 
            self.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
        
        markdown.push_str(&format!("**Model:** {}\n", self.metadata.model_used));
        
        if let Some(cost) = self.metadata.cost_estimate {
            markdown.push_str(&format!("**Estimated Cost:** ${:.4}\n", cost));
        }
        
        markdown.push_str(&format!("**Tokens:** {}\n\n", self.metadata.token_count));
        
        // System prompt
        if !self.system_prompt.is_empty() {
            markdown.push_str("## System Prompt\n\n");
            markdown.push_str(&format!("```\n{}\n```\n\n", self.system_prompt));
        }
        
        // Messages
        markdown.push_str("## Conversation\n\n");
        
        for message in &self.messages {
            let role_emoji = match message.role {
                MessageRole::User => "👤",
                MessageRole::Assistant => "🤖",
                MessageRole::System => "⚙️",
            };
            
            markdown.push_str(&format!("### {} {:?}\n\n", role_emoji, message.role));
            markdown.push_str(&format!("{}\n\n", message.content));
            
            if let Some(tool_calls) = &message.tool_calls {
                if !tool_calls.is_empty() {
                    markdown.push_str("**Tool Calls:**\n");
                    for tool_call in tool_calls {
                        markdown.push_str(&format!("- `{}`: {}\n", 
                            tool_call.name, 
                            serde_json::to_string_pretty(&tool_call.arguments).unwrap_or_default()));
                    }
                    markdown.push_str("\n");
                }
            }
            
            markdown.push_str("---\n\n");
        }
        
        markdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let conv = Conversation::new("Test system prompt".to_string());
        assert_eq!(conv.system_prompt, "Test system prompt");
        assert!(conv.messages.is_empty());
        assert!(conv.metadata.title.is_none());
    }

    #[test]
    fn test_add_message() {
        let mut conv = Conversation::new("Test".to_string());
        let message = Message {
            role: MessageRole::User,
            content: "Hello, world!".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        };
        
        conv.add_message(message);
        assert_eq!(conv.messages.len(), 1);
        assert!(conv.metadata.title.is_some());
    }

    #[test]
    fn test_title_generation() {
        let mut conv = Conversation::new("Test".to_string());
        let message = Message {
            role: MessageRole::User,
            content: "How do I list files in a directory?".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        };
        
        conv.add_message(message);
        assert_eq!(conv.metadata.title.as_deref(), Some("How do I list files"));
    }
}
