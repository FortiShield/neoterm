use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use reqwest::Client;
use anyhow::{Result, anyhow};
use tokio::sync::mpsc;
use bytes::BytesMut;
use futures_util::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AiProvider {
    OpenAI,
    // Add other providers here (e.g., Google, Anthropic)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::OpenAI,
            api_key: None,
            model: "gpt-4o".to_string(),
            temperature: 0.7,
            max_tokens: 500,
        }
    }
}

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
    pub arguments: serde_json::Value, // JSON object as string
}

/// Trait for AI client implementations.
#[async_trait]
pub trait AIClient: Send + Sync {
    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<serde_json::Value>) -> Result<ChatMessage>;
    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<serde_json::Value>) -> Result<mpsc::Receiver<ChatMessage>>;
}

pub struct OpenAIClient {
    client: Client,
    config: AiConfig,
}

impl OpenAIClient {
    pub fn new(config: AiConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(api_key) = &self.config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers
    }
}

#[async_trait]
impl AIClient for OpenAIClient {
    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<serde_json::Value>) -> Result<ChatMessage> {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        if let Some(t) = tools {
            body["tools"] = t;
            body["tool_choice"] = serde_json::json!("auto");
        }

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .headers(self.get_headers())
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let json_response: serde_json::Value = response.json().await?;
        
        let choice = json_response["choices"][0].clone();
        let message = choice["message"].clone();

        let role = message["role"].as_str().unwrap_or("assistant").to_string();
        let content = message["content"].as_str().unwrap_or("").to_string();
        let tool_calls = message["tool_calls"].as_array().map(|calls| {
            calls.iter().map(|call| {
                ToolCall {
                    id: call["id"].as_str().unwrap_or_default().to_string(),
                    call_type: call["type"].as_str().unwrap_or_default().to_string(),
                    function: ToolFunction {
                        name: call["function"]["name"].as_str().unwrap_or_default().to_string(),
                        arguments: call["function"]["arguments"].clone(),
                    },
                }
            }).collect()
        });
        let tool_call_id = message["
