use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod openai;
pub mod ollama;
pub mod anthropic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String, // "function"
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: Value, // JSON object as a string
}

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage>;
    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>>;
}
