use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod openai;
pub mod ollama;
pub mod anthropic;

pub use openai::OpenAIProvider;
pub use ollama::OllamaProvider;
pub use anthropic::AnthropicProvider;

/// Represents a message in the conversation with the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Represents a tool call made by the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String, // "function"
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: Value, // JSON object as string
}

/// Trait for AI client implementations.
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Sends a list of messages to the AI and gets a single response.
    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage>;

    /// Sends a list of messages to the AI and streams the response.
    /// Returns a receiver for `ChatMessage` chunks.
    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>>;
}
