use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;
use futures_util::StreamExt; // For consuming reqwest response stream

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    OpenAI,
    Claude,
    Gemini,
    Ollama,
    Groq,
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
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
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

    pub async fn send_message(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        match self.config.provider {
            AiProvider::OpenAI => self.send_openai_message(messages, tools).await,
            AiProvider::Claude => self.send_claude_message(messages, tools).await,
            AiProvider::Gemini => self.send_gemini_message(messages, tools).await,
            AiProvider::Ollama => self.send_ollama_message(messages, tools).await,
            AiProvider::Groq => self.send_groq_message(messages, tools).await,
        }
    }

    async fn send_openai_message(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        
        let api_key = self.config.api_key.as_ref()
            .ok_or("OpenAI API key required")?;

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

        if let Some(tools) = tools {
            if self.config.tools_enabled && !tools.is_empty() {
                request_body["tools"] = serde_json::json!(
                    tools.iter().map(|tool| tool.to_openai_format()).collect::<Vec<_>>()
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
                    if let Ok(text) = resp.text().await {
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    break;
                                }
                                
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(choices) = json["choices"].as_array() {
                                        if let Some(choice) = choices.first() {
                                            if let Some(delta) = choice["delta"].as_object() {
                                                if let Some(content) = delta["content"].as_str() {
                                                    let _ = tx.send(content.to_string()).await;
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
                    let _ = tx.send(format!("Error: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    async fn send_claude_message(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        
        let api_key = self.config.api_key.as_ref()
            .ok_or("Claude API key required")?;

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
        }

        if let Some(system_prompt) = &self.config.system_prompt {
            request_body["system"] = serde_json::Value::String(system_prompt.clone());
        }

        if let Some(tools) = tools {
            if self.config.tools_enabled && !tools.is_empty() {
                request_body["tools"] = serde_json::json!(
                    tools.iter().map(|tool| tool.to_claude_format()).collect::<Vec<_>>()
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
                    if let Ok(text) = resp.text().await {
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(delta) = json["delta"].as_object() {
                                        if let Some(text) = delta["text"].as_str() {
                                            let _ = tx.send(text.to_string()).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Error: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    async fn send_gemini_message(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        
        let api_key = self.config.api_key.as_ref()
            .ok_or("Gemini API key required")?;

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
                    if let Ok(text) = resp.text().await {
                        for line in text.lines() {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                if let Some(candidates) = json["candidates"].as_array() {
                                    if let Some(candidate) = candidates.first() {
                                        if let Some(content) = candidate["content"]["parts"].as_array() {
                                            if let Some(part) = content.first() {
                                                if let Some(text) = part["text"].as_str() {
                                                    let _ = tx.send(text.to_string()).await;
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
                    let _ = tx.send(format!("Error: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    async fn send_ollama_message(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        
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
                                                    let _ = tx.send(content.to_string()).await;
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(format!("Error reading stream: {}", e)).await;
                                    break;
                                }
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        let _ = tx.send(format!("Ollama API error: Status {} - {}", resp.status(), error_text)).await;
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Error connecting to Ollama: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    async fn send_groq_message(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::agent_mode_eval::tools::Tool>>,
    ) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        
        let api_key = self.config.api_key.as_ref()
            .ok_or("Groq API key required")?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.groq.com/openai/v1");

        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": self.format_messages_for_openai(&messages),
            "temperature": self.config.temperature,
            "stream": true
        });

        if let Some(max_tokens) = self.config.max_tokens {
            request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        }

        if let Some(tools) = tools {
            if self.config.tools_enabled && !tools.is_empty() {
                request_body["tools"] = serde_json::json!(
                    tools.iter().map(|tool| tool.to_openai_format()).collect::<Vec<_>>()
                );
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
                    if let Ok(text) = resp.text().await {
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    break;
                                }
                                
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(choices) = json["choices"].as_array() {
                                        if let Some(choice) = choices.first() {
                                            if let Some(delta) = choice["delta"].as_object() {
                                                if let Some(content) = delta["content"].as_str() {
                                                    let _ = tx.send(content.to_string()).await;
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
                    let _ = tx.send(format!("Error: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    fn format_messages_for_openai(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().map(|msg| {
            serde_json::json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                },
                "content": msg.content
            })
        }).collect()
    }

    fn format_messages_for_claude(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().filter_map(|msg| {
            match msg.role {
                MessageRole::System => None,
                _ => Some(serde_json::json!({
                    "role": match msg.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "user",
                        MessageRole::System => unreachable!(),
                    },
                    "content": msg.content
                }))
            }
        }).collect()
    }

    fn format_messages_for_gemini(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages.iter().map(|msg| {
            serde_json::json!({
                "role": match msg.role {
                    MessageRole::User | MessageRole::System => "user",
                    MessageRole::Assistant => "model",
                    MessageRole::Tool => "user",
                },
                "parts": [{"text": msg.content}]
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
                    MessageRole::Tool => "user",
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
