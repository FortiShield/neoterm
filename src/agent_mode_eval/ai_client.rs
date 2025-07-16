use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;
use futures_util::StreamExt; // For consuming reqwest response stream
use async_trait::async_trait;
use tokio_stream::{Stream, StreamExt};
use openai_api::{
    chat::{ChatCompletion, ChatCompletionChunk, ChatCompletionRequest, Message as OpenAIMessage, Role},
    Client,
};
use std::error::Error;
use std::fmt;
use crate::agent_mode_eval::conversation::{Message, MessageRole};
use crate::agent_mode_eval::tools::{ToolCall as AgentToolCall, ToolResult as AgentToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    OpenAI,
    Claude,
    Gemini,
    Ollama,
    Groq,
}

#[derive(Debug, Clone)]
pub enum AIClientConfig {
    OpenAI { api_key: Option<String> },
    // Add other AI providers here (e.g., Google, Anthropic)
}

#[derive(Debug)]
pub enum AIClientError {
    ConfigurationError(String),
    APIError(String),
    StreamError(String),
    SerializationError(String),
    UnknownError(String),
}

impl fmt::Display for AIClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AIClientError::ConfigurationError(msg) => write!(f, "Configuration Error: {}", msg),
            AIClientError::APIError(msg) => write!(f, "API Error: {}", msg),
            AIClientError::StreamError(msg) => write!(f, "Stream Error: {}", msg),
            AIClientError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            AIClientError::UnknownError(msg) => write!(f, "Unknown Error: {}", msg),
        }
    }
}

impl Error for AIClientError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIStreamChunk {
    Text(String),
    ToolCall(Vec<AgentToolCall>),
    Done,
}

#[async_trait]
pub trait AIClient: Send + Sync {
    fn clone_box(&self) -> Box<dyn AIClient + Send + Sync>;
    async fn stream_response(&self, messages: Vec<Message>, tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub model: String,
    pub api_key: Option<String>, // Not strictly needed for local Ollama, but kept for consistency
    pub base_url: Option<String>, // Base URL for the AI provider, e.g., Ollama server address
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub tools_enabled: bool,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::OpenAI,
            model: "gpt-4o".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: Some(4096),
            system_prompt: Some("You are a helpful AI assistant integrated into a terminal. You can execute commands, read files, and help with development tasks.".to_string()),
            tools_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_calls: Option<Vec<AgentToolCall>>,
    pub tool_results: Option<Vec<AgentToolResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

pub struct AiClient {
    config: AiConfig,
    http_client: reqwest::Client,
    available_models: HashMap<AiProvider, Vec<String>>,
}

impl AiClient {
    pub fn new(config: AiConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let mut available_models = HashMap::new();
        
        available_models.insert(AiProvider::OpenAI, vec![
            "gpt-4o".to_string(),
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-4-mini".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3o".to_string(),
            "o3".to_string(),
            "o3-mini".to_string(),
        ]);

        available_models.insert(AiProvider::Claude, vec![
            "claude-4-sonnet-20250514".to_string(),
            "claude-4-opus-20250514".to_string(),
            "claude-3-7-sonnet-20241022".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-7-haiku-20241022".to_string(),
        ]);

        available_models.insert(AiProvider::Gemini, vec![
            "gemini-2.0-flash-exp".to_string(),
            "gemini-2.0-pro-exp".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ]);

        available_models.insert(AiProvider::Ollama, vec![
            "llama3.2".to_string(),
            "llama3.1".to_string(),
            "codellama".to_string(),
            "mistral".to_string(),
            "phi3".to_string(),
            "qwen2.5".to_string(),
            "deepseek-coder".to_string(),
            "llava".to_string(),
            "nomic-embed-text".to_string(),
        ]);

        available_models.insert(AiProvider::Groq, vec![
            "llama-3.1-70b-versatile".to_string(),
            "llama-3.1-8b-instant".to_string(),
            "mixtral-8x7b-32768".to_string(),
            "gemma2-9b-it".to_string(),
        ]);

        Ok(Self {
            config,
            http_client,
            available_models,
        })
    }

    pub fn get_available_models(&self, provider: &AiProvider) -> Vec<String> {
        self.available_models.get(provider).cloned().unwrap_or_default()
    }

    pub fn is_model_supported(&self, provider: &AiProvider, model: &str) -> bool {
        self.available_models
            .get(provider)
            .map(|models| models.contains(&model.to_string()))
            .unwrap_or(false)
    }

    pub async fn stream_response(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        match self.config.provider {
            AiProvider::OpenAI => self.stream_openai_response(messages, tools).await,
            AiProvider::Claude => self.stream_claude_response(messages, tools).await,
            AiProvider::Gemini => self.stream_gemini_response(messages, tools).await,
            AiProvider::Ollama => self.stream_ollama_response(messages, tools).await,
            AiProvider::Groq => self.stream_groq_response(messages, tools).await,
        }
    }

    async fn stream_openai_response(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let api_key = self.config.api_key.as_ref()
            .ok_or(AIClientError::ConfigurationError("OpenAI API key required".to_string()))?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1");

        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": self.format_messages_for_openai(&messages),
            "temperature": self.config.temperature,
            "stream": true
        });

        if let Some(max_tokens) = self.config.max_tokens {
            request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        }

        if let Some(tools_def) = tools {
            if self.config.tools_enabled && !tools_def.is_empty() {
                request_body["tools"] = serde_json::json!(
                    tools_def.iter().map(|tool| tool.to_openai_format()).collect::<Vec<_>>()
                );
                request_body["tool_choice"] = serde_json::Value::String("auto".to_string());
            }
        }

        let client = self.http_client.clone();
        let url = format!("{}/chat/completions", base_url);
        let auth_header = format!("Bearer {}", api_key);

        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("Authorization", auth_header)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let mut stream = resp.bytes_stream();
                        let mut current_tool_calls: HashMap<String, AgentToolCall> = HashMap::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    for line in text.lines() {
                                        if line.starts_with("data: ") {
                                            let data = &line[6..];
                                            if data == "[DONE]" {
                                                let _ = tx.send(Ok(AIStreamChunk::Done));
                                                return;
                                            }
                                            
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                if let Some(choices) = json["choices"].as_array() {
                                                    if let Some(choice) = choices.first() {
                                                        if let Some(delta) = choice["delta"].as_object() {
                                                            if let Some(content) = delta["content"].as_str() {
                                                                let _ = tx.send(Ok(AIStreamChunk::Text(content.to_string())));
                                                            }
                                                            if let Some(tool_calls_array) = delta["tool_calls"].as_array() {
                                                                for tool_call_delta in tool_calls_array {
                                                                    if let Some(id) = tool_call_delta["id"].as_str() {
                                                                        let name = tool_call_delta["function"]["name"].as_str().unwrap_or_default().to_string();
                                                                        let arguments_delta = tool_call_delta["function"]["arguments"].as_str().unwrap_or_default().to_string();

                                                                        let entry = current_tool_calls.entry(id.to_string()).or_insert_with(|| AgentToolCall {
                                                                            id: id.to_string(),
                                                                            name: name.clone(),
                                                                            arguments: serde_json::Value::String("".to_string()),
                                                                        });
                                                                        
                                                                        if let serde_json::Value::String(ref mut args_str) = entry.arguments {
                                                                            args_str.push_str(&arguments_delta);
                                                                        }
                                                                    }
                                                                }
                                                                // Send accumulated tool calls if they are complete or if it's the last chunk
                                                                // For simplicity, we'll send them when the stream ends or when a new text chunk appears.
                                                                // A more robust solution would check for 'finish_reason' or 'tool_calls' completion.
                                                                if !current_tool_calls.is_empty() {
                                                                    let completed_tool_calls: Vec<AgentToolCall> = current_tool_calls.values().cloned().collect();
                                                                    if !completed_tool_calls.is_empty() {
                                                                        let _ = tx.send(Ok(AIStreamChunk::ToolCall(completed_tool_calls)));
                                                                        current_tool_calls.clear(); // Clear after sending
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(AIClientError::StreamError(format!("Error reading stream: {}", e))));
                                    return;
                                }
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let _ = tx.send(Err(AIClientError::APIError(format!("OpenAI API error: Status {} - {}", resp.status(), error_text))));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(AIClientError::APIError(format!("Error connecting to OpenAI: {}", e))));
                }
            }
        });

        Ok(Box::new(mpsc::unbounded_channel_to_stream(rx)))
    }

    async fn stream_claude_response(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let api_key = self.config.api_key.as_ref()
            .ok_or(AIClientError::ConfigurationError("Claude API key required".to_string()))?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.anthropic.com/v1");

        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": self.format_messages_for_claude(&messages),
            "temperature": self.config.temperature,
            "stream": true
        });

        if let Some(max_tokens) = self.config.max_tokens {
            request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        } else {
            request_body["max_tokens"] = serde_json::Value::Number(4096.into()); // Claude requires max_tokens
        }

        if let Some(system_prompt) = &self.config.system_prompt {
            request_body["system"] = serde_json::Value::String(system_prompt.clone());
        }

        if let Some(tools_def) = tools {
            if self.config.tools_enabled && !tools_def.is_empty() {
                request_body["tools"] = serde_json::json!(
                    tools_def.iter().map(|tool| tool.to_claude_format()).collect::<Vec<_>>()
                );
            }
        }

        let client = self.http_client.clone();
        let url = format!("{}/messages", base_url);

        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let mut stream = resp.bytes_stream();
                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    for line in text.lines() {
                                        if line.starts_with("data: ") {
                                            let data = &line[6..];
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                if let Some(event_type) = json["type"].as_str() {
                                                    match event_type {
                                                        "content_block_delta" => {
                                                            if let Some(delta) = json["delta"].as_object() {
                                                                if let Some(text) = delta["text"].as_str() {
                                                                    let _ = tx.send(Ok(AIStreamChunk::Text(text.to_string())));
                                                                }
                                                            }
                                                        },
                                                        "content_block_start" => {
                                                            if let Some(content_block) = json["content_block"].as_object() {
                                                                if let Some(block_type) = content_block["type"].as_str() {
                                                                    if block_type == "tool_use" {
                                                                        let id = content_block["id"].as_str().unwrap_or_default().to_string();
                                                                        let name = content_block["name"].as_str().unwrap_or_default().to_string();
                                                                        let input = content_block["input"].clone(); // Claude sends full input on start
                                                                        let _ = tx.send(Ok(AIStreamChunk::ToolCall(vec![AgentToolCall {
                                                                            id,
                                                                            name,
                                                                            arguments: input,
                                                                        }])));
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "content_block_stop" => {
                                                            // Tool arguments are complete here, but we sent the tool_call on start.
                                                        },
                                                        "message_stop" => {
                                                            let _ = tx.send(Ok(AIStreamChunk::Done));
                                                            return;
                                                        },
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(AIClientError::StreamError(format!("Error reading stream: {}", e))));
                                    return;
                                }
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let _ = tx.send(Err(AIClientError::APIError(format!("Claude API error: Status {} - {}", resp.status(), error_text))));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(AIClientError::APIError(format!("Error connecting to Claude: {}", e))));
                }
            }
        });

        Ok(Box::new(mpsc::unbounded_channel_to_stream(rx)))
    }

    async fn stream_gemini_response(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>, // Tools not supported in this iteration
    ) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let api_key = self.config.api_key.as_ref()
            .ok_or(AIClientError::ConfigurationError("Gemini API key required".to_string()))?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta");

        let request_body = serde_json::json!({
            "contents": self.format_messages_for_gemini(&messages),
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": self.config.max_tokens.unwrap_or(4096)
            }
        });

        let client = self.http_client.clone();
        let url = format!("{}/models/{}:streamGenerateContent?key={}", base_url, self.config.model, api_key);

        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let mut stream = resp.bytes_stream();
                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    for line in text.lines() {
                                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                            if let Some(candidates) = json["candidates"].as_array() {
                                                if let Some(candidate) = candidates.first() {
                                                    if let Some(content_parts) = candidate["content"]["parts"].as_array() {
                                                        for part in content_parts {
                                                            if let Some(text) = part["text"].as_str() {
                                                                let _ = tx.send(Ok(AIStreamChunk::Text(text.to_string())));
                                                            }
                                                            // No tool call parsing for Gemini in this iteration
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(AIClientError::StreamError(format!("Error reading stream: {}", e))));
                                    return;
                                }
                            }
                        }
                        let _ = tx.send(Ok(AIStreamChunk::Done));
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let _ = tx.send(Err(AIClientError::APIError(format!("Gemini API error: Status {} - {}", resp.status(), error_text))));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(AIClientError::APIError(format!("Error connecting to Gemini: {}", e))));
                }
            }
        });

        Ok(Box::new(mpsc::unbounded_channel_to_stream(rx)))
    }

    async fn stream_ollama_response(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>, // Tools not supported in this iteration
    ) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("http://localhost:11434");

        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": self.format_messages_for_ollama(&messages),
            "stream": true,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens.unwrap_or(4096)
            }
        });

        let client = self.http_client.clone();
        let url = format!("{}/api/chat", base_url);

        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(mut resp) => {
                    if resp.status().is_success() {
                        while let Some(chunk) = resp.chunk().await.transpose() {
                            match chunk {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    for line in text.lines() {
                                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                            if let Some(message) = json["message"].as_object() {
                                                if let Some(content) = message["content"].as_str() {
                                                    let _ = tx.send(Ok(AIStreamChunk::Text(content.to_string())));
                                                }
                                            }
                                            if json["done"].as_bool().unwrap_or(false) {
                                                let _ = tx.send(Ok(AIStreamChunk::Done));
                                                return;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(AIClientError::StreamError(format!("Error reading stream: {}", e))));
                                    break;
                                }
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let _ = tx.send(Err(AIClientError::APIError(format!("Ollama API error: Status {} - {}", resp.status(), error_text))));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(AIClientError::APIError(format!("Error connecting to Ollama: {}", e))));
                }
            }
        });

        Ok(Box::new(mpsc::unbounded_channel_to_stream(rx)))
    }

    async fn stream_groq_response(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let api_key = self.config.api_key.as_ref()
            .ok_or(AIClientError::ConfigurationError("Groq API key required".to_string()))?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.groq.com/openai/v1");

        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": self.format_messages_for_openai(&messages), // Groq uses OpenAI message format
            "temperature": self.config.temperature,
            "stream": true
        });

        if let Some(max_tokens) = self.config.max_tokens {
            request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        }

        if let Some(tools_def) = tools {
            if self.config.tools_enabled && !tools_def.is_empty() {
                request_body["tools"] = serde_json::json!(
                    tools_def.iter().map(|tool| tool.to_openai_format()).collect::<Vec<_>>()
                );
                request_body["tool_choice"] = serde_json::Value::String("auto".to_string());
            }
        }

        let client = self.http_client.clone();
        let url = format!("{}/chat/completions", base_url);
        let auth_header = format!("Bearer {}", api_key);

        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("Authorization", auth_header)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let mut stream = resp.bytes_stream();
                        let mut current_tool_calls: HashMap<String, AgentToolCall> = HashMap::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    for line in text.lines() {
                                        if line.starts_with("data: ") {
                                            let data = &line[6..];
                                            if data == "[DONE]" {
                                                let _ = tx.send(Ok(AIStreamChunk::Done));
                                                return;
                                            }
                                            
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                if let Some(choices) = json["choices"].as_array() {
                                                    if let Some(choice) = choices.first() {
                                                        if let Some(delta) = choice["delta"].as_object() {
                                                            if let Some(content) = delta["content"].as_str() {
                                                                let _ = tx.send(Ok(AIStreamChunk::Text(content.to_string())));
                                                            }
                                                            if let Some(tool_calls_array) = delta["tool_calls"].as_array() {
                                                                for tool_call_delta in tool_calls_array {
                                                                    if let Some(id) = tool_call_delta["id"].as_str() {
                                                                        let name = tool_call_delta["function"]["name"].as_str().unwrap_or_default().to_string();
                                                                        let arguments_delta = tool_call_delta["function"]["arguments"].as_str().unwrap_or_default().to_string();

                                                                        let entry = current_tool_calls.entry(id.to_string()).or_insert_with(|| AgentToolCall {
                                                                            id: id.to_string(),
                                                                            name: name.clone(),
                                                                            arguments: serde_json::Value::String("".to_string()),
                                                                        });
                                                                        
                                                                        if let serde_json::Value::String(ref mut args_str) = entry.arguments {
                                                                            args_str.push_str(&arguments_delta);
                                                                        }
                                                                    }
                                                                }
                                                                if !current_tool_calls.is_empty() {
                                                                    let completed_tool_calls: Vec<AgentToolCall> = current_tool_calls.values().cloned().collect();
                                                                    if !completed_tool_calls.is_empty() {
                                                                        let _ = tx.send(Ok(AIStreamChunk::ToolCall(completed_tool_calls)));
                                                                        current_tool_calls.clear();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(AIClientError::StreamError(format!("Error reading stream: {}", e))));
                                    return;
                                }
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let _ = tx.send(Err(AIClientError::APIError(format!("Groq API error: Status {} - {}", resp.status(), error_text))));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(AIClientError::APIError(format!("Error connecting to Groq: {}", e))));
                }
            }
        });

        Ok(Box::new(mpsc::unbounded_channel_to_stream(rx)))
    }

    fn format_messages_for_openai(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().map(|msg| {
            let mut json_msg = serde_json::json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                },
                "content": msg.content
            });

            if let Some(tool_calls) = &msg.tool_calls {
                json_msg["tool_calls"] = serde_json::json!(tool_calls.iter().map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string(), // Arguments are stringified JSON
                        }
                    })
                }).collect::<Vec<_>>());
                json_msg["content"] = serde_json::Value::Null; // Content is null for tool_calls
            }

            if let Some(tool_results) = &msg.tool_results {
                // OpenAI tool results are sent as 'tool' role messages
                // The content is the tool's output, and tool_call_id links to the original tool_call
                json_msg["tool_call_id"] = serde_json::Value::String(tool_results[0].tool_call_id.clone());
                json_msg["content"] = serde_json::Value::String(tool_results[0].content.clone());
                // Assuming one tool result per message for simplicity, or combine content
            }

            json_msg
        }).collect()
    }

    fn format_messages_for_claude(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().filter_map(|msg| {
            match msg.role {
                MessageRole::System => None, // System prompt is handled separately for Claude
                MessageRole::User => Some(serde_json::json!({
                    "role": "user",
                    "content": if let Some(tool_results) = &msg.tool_results {
                        // Claude tool results are sent as 'user' role with 'tool_result' type
                        serde_json::json!([
                            {
                                "type": "tool_result",
                                "tool_use_id": tool_results[0].tool_call_id,
                                "content": tool_results[0].content,
                            }
                        ])
                    } else {
                        serde_json::json!([{"type": "text", "text": msg.content}])
                    }
                })),
                MessageRole::Assistant => Some(serde_json::json!({
                    "role": "assistant",
                    "content": if let Some(tool_calls) = &msg.tool_calls {
                        // Claude tool calls are sent as 'assistant' role with 'tool_use' type
                        serde_json::json!(tool_calls.iter().map(|tc| {
                            serde_json::json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.name,
                                "input": tc.arguments, // Arguments are direct JSON
                            })
                        }).collect::<Vec<_>>())
                    } else {
                        serde_json::json!([{"type": "text", "text": msg.content}])
                    }
                })),
                MessageRole::Tool => None, // Tool role is converted to user with tool_result content for Claude
            }
        }).collect()
    }

    fn format_messages_for_gemini(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().map(|msg| {
            serde_json::json!({
                "role": match msg.role {
                    MessageRole::User | MessageRole::System => "user",
                    MessageRole::Assistant => "model",
                    MessageRole::Tool => "function", // Gemini uses 'function' role for tool results
                },
                "parts": if let Some(tool_results) = &msg.tool_results {
                    // Gemini tool results are sent as 'function' role with 'functionResponse' type
                    serde_json::json!([
                        {
                            "functionResponse": {
                                "name": tool_results[0].tool_call_id, // Assuming tool_call_id can be used as name
                                "response": serde_json::json!({"output": tool_results[0].content}),
                            }
                        }
                    ])
                } else if let Some(tool_calls) = &msg.tool_calls {
                    // Gemini tool calls are sent as 'model' role with 'functionCall' type
                    serde_json::json!(tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "functionCall": {
                                "name": tc.name,
                                "args": tc.arguments, // Arguments are direct JSON
                            }
                        })
                    }).collect::<Vec<_>>())
                } else {
                    serde_json::json!([{"text": msg.content}])
                }
            })
        }).collect()
    }

    fn format_messages_for_ollama(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().map(|msg| {
            serde_json::json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "user", // Ollama typically treats tool results as user messages
                },
                "content": msg.content
            })
        }).collect()
    }

    pub fn update_config(&mut self, config: AiConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &AiConfig {
        &self.config
    }
}

#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    model: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(api_key),
            model,
        }
    }
}

#[async_trait]
impl AIClient for OpenAIClient {
    fn clone_box(&self) -> Box<dyn AIClient + Send + Sync> {
        Box::new(self.clone())
    }

    async fn stream_response(&self, messages: Vec<Message>, tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>) -> Result<Box<dyn Stream<Item = Result<AIStreamChunk, AIClientError>> + Send + Unpin>, AIClientError> {
        let openai_messages: Vec<OpenAIMessage> = messages
            .into_iter()
            .map(|msg| {
                let mut openai_msg = OpenAIMessage {
                    role: match msg.role {
                        MessageRole::User => Role::User,
                        MessageRole::Assistant => Role::Assistant,
                        MessageRole::System => Role::System,
                        MessageRole::Tool => Role::Tool,
                    },
                    content: msg.content,
                    name: None,
                };

                if let Some(tool_calls) = msg.tool_calls {
                    openai_msg.tool_calls = Some(tool_calls.into_iter().map(|tc| openai_api::chat::ToolCall {
                        id: tc.id,
                        r#type: "function".to_string(),
                        function: openai_api::chat::Function {
                            name: tc.name,
                            arguments: tc.arguments.to_string(),
                        },
                    }).collect());
                    openai_msg.content = "".to_string(); // Content is null for tool_calls
                }

                if let Some(tool_results) = msg.tool_results {
                    openai_msg.tool_call_id = Some(tool_results[0].tool_call_id.clone());
                    openai_msg.content = tool_results[0].content.clone();
                }
                openai_msg
            })
            .collect();

        let mut request = ChatCompletionRequest::new(
            self.model.clone(),
            openai_messages,
        )
        .stream(true);

        if let Some(tools_def) = tools {
            if !tools_def.is_empty() {
                request = request.tools(tools_def.iter().map(|tool| tool.to_openai_format()).collect());
                request = request.tool_choice(openai_api::chat::ToolChoice::Auto);
            }
        }

        let response = self.client.chat_completion_create(request).await
            .map_err(|e| AIClientError::APIError(format!("Failed to create chat completion: {}", e)))?;

        let stream = response
            .map(|chunk_result| {
                chunk_result
                    .map_err(|e| AIClientError::StreamError(format!("Error receiving chunk: {}", e)))
                    .and_then(|chunk: ChatCompletionChunk| {
                        let mut text_content = String::new();
                        let mut tool_calls_list: Vec<AgentToolCall> = Vec::new();

                        if let Some(choice) = chunk.choices.into_iter().next() {
                            if let Some(content) = choice.delta.content {
                                text_content = content;
                            }
                            if let Some(tool_calls) = choice.delta.tool_calls {
                                for tc_delta in tool_calls {
                                    tool_calls_list.push(AgentToolCall {
                                        id: tc_delta.id.unwrap_or_default(),
                                        name: tc_delta.function.name.unwrap_or_default(),
                                        arguments: serde_json::Value::String(tc_delta.function.arguments.unwrap_or_default()),
                                    });
                                }
                            }
                        }

                        if !tool_calls_list.is_empty() {
                            Ok(AIStreamChunk::ToolCall(tool_calls_list))
                        } else if !text_content.is_empty() {
                            Ok(AIStreamChunk::Text(text_content))
                        } else {
                            Ok(AIStreamChunk::Done) // If no content or tool calls, consider it done for this chunk
                        }
                    })
            })
            .filter_map(|res| async move {
                match res {
                    Ok(AIStreamChunk::Done) => None, // Filter out intermediate Done chunks
                    other => Some(other),
                }
            });

        Ok(Box::new(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_support() {
        let config = AiConfig::default();
        let client = AiClient::new(config).unwrap();
        
        assert!(client.is_model_supported(&AiProvider::OpenAI, "gpt-4o"));
        assert!(client.is_model_supported(&AiProvider::Claude, "claude-4-sonnet-20250514"));
        assert!(client.is_model_supported(&AiProvider::Ollama, "llama3.1"));
        assert!(!client.is_model_supported(&AiProvider::OpenAI, "invalid-model"));
    }

    #[test]
    fn test_message_formatting() {
        let config = AiConfig::default();
        let client = AiClient::new(config).unwrap();
        
        let messages = vec![
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: "Hello".to_string(),
                timestamp: chrono::Utc::now(),
                tool_calls: None,
                tool_results: None,
            }
        ];

        let formatted = client.format_messages_for_openai(&messages);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0]["role"], "user");
        assert_eq!(formatted[0]["content"], "Hello");

        let formatted_ollama = client.format_messages_for_ollama(&messages);
        assert_eq!(formatted_ollama.len(), 1);
        assert_eq!(formatted_ollama[0]["role"], "user");
        assert_eq!(formatted_ollama[0]["content"], "Hello");
    }
}
