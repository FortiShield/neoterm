use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_stream::{Stream, StreamExt};
use reqwest::Client;
use futures::stream::BoxStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    OpenAI,
    Claude,
    Groq,
    Local,
    Ollama,
    Gemini,
}

#[derive(Debug, Clone)]
pub struct AiClient {
    pub config: super::AgentConfig,
    client: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<super::tools::ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub tool_calls: Option<Vec<super::tools::ToolCall>>,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct StreamingResponse {
    pub content: String,
    pub is_complete: bool,
}

impl AiClient {
    pub fn new(config: super::AgentConfig) -> Result<Self, AiClientError> {
        // Validate model for provider
        Self::validate_model_for_provider(&config.provider, &config.model)?;
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        Ok(Self { config, client })
    }

    fn validate_model_for_provider(provider: &AiProvider, model: &str) -> Result<(), AiClientError> {
        let valid_models = match provider {
            AiProvider::OpenAI => vec![
                "gpt-4o", "gpt-4", "gpt-4-turbo", "gpt-4-mini", 
                "gpt-3.5-turbo", "gpt-3o", "o3", "o3-mini"
            ],
            AiProvider::Claude => vec![
                "claude-4-sonnet-20250514", "claude-4-opus-20250514",
                "claude-3-7-sonnet-20241022", "claude-3-5-sonnet-20241022", 
                "claude-3-7-haiku-20241022"
            ],
            AiProvider::Gemini => vec![
                "gemini-2.0-flash-exp", "gemini-2.0-pro-exp",
                "gemini-1.5-pro", "gemini-1.5-flash"
            ],
            AiProvider::Ollama => vec![
                "llama3.2", "llama3.1", "codellama", "mistral", 
                "phi3", "qwen2.5", "deepseek-coder"
            ],
            AiProvider::Groq => vec![
                "llama-3.1-70b-versatile", "llama-3.1-8b-instant",
                "mixtral-8x7b-32768", "gemma2-9b-it"
            ],
            AiProvider::Local => return Ok(()), // Local models can be anything
        };

        if !valid_models.contains(&model) {
            return Err(AiClientError::ConfigError(
                format!("Model '{}' is not supported for provider {:?}", model, provider)
            ));
        }

        Ok(())
    }

    pub async fn complete(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        match self.config.provider {
            AiProvider::OpenAI => self.openai_complete(messages, tools).await,
            AiProvider::Claude => self.claude_complete(messages, tools).await,
            AiProvider::Groq => self.groq_complete(messages, tools).await,
            AiProvider::Local => self.local_complete(messages, tools).await,
            AiProvider::Ollama => self.ollama_complete(messages, tools).await,
            AiProvider::Gemini => self.gemini_complete(messages, tools).await,
        }
    }

    pub async fn stream_completion(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        match self.config.provider {
            AiProvider::OpenAI => self.openai_stream(messages, tools).await,
            AiProvider::Claude => self.claude_stream(messages, tools).await,
            AiProvider::Groq => self.groq_stream(messages, tools).await,
            AiProvider::Local => self.local_stream(messages, tools).await,
            AiProvider::Ollama => self.ollama_stream(messages, tools).await,
            AiProvider::Gemini => self.gemini_stream(messages, tools).await,
        }
    }

    async fn openai_complete(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or(AiClientError::MissingApiKey)?;

        let url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "stream": false
        });

        if let Some(max_tokens) = self.config.max_tokens {
            request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        }

        if let Some(tools) = tools {
            request_body["tools"] = serde_json::to_value(tools)?;
        }

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiClientError::ApiError(format!("OpenAI API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiClientError::ParseError(e.to_string()))?;

        self.parse_openai_response(response_json)
    }

    async fn claude_complete(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or(AiClientError::MissingApiKey)?;

        let url = self.config.base_url.as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");

        // Convert messages for Claude format
        let (system_message, claude_messages) = self.convert_messages_for_claude(messages);

        let mut request_body = serde_json::json!({
            "model": self.config.model,
            "messages": claude_messages,
            "temperature": self.config.temperature,
            "stream": false
        });

        if let Some(system) = system_message {
            request_body["system"] = serde_json::Value::String(system);
        }

        if let Some(max_tokens) = self.config.max_tokens {
            request_body["max_tokens"] = serde_json::Value::Number(max_tokens.into());
        }

        if let Some(tools) = tools {
            request_body["tools"] = serde_json::to_value(tools)?;
        }

        let response = self.client
            .post(url)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiClientError::ApiError(format!("Claude API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiClientError::ParseError(e.to_string()))?;

        self.parse_claude_response(response_json)
    }

    async fn groq_complete(&self, messages: Vec<AiMessage>, _tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or(AiClientError::MissingApiKey)?;

        let url = self.config.base_url.as_deref()
            .unwrap_or("https://api.groq.com/openai/v1/chat/completions");

        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "stream": false
        });

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiClientError::ApiError(format!("Groq API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiClientError::ParseError(e.to_string()))?;

        self.parse_openai_response(response_json) // Groq uses OpenAI-compatible format
    }

    async fn local_complete(&self, messages: Vec<AiMessage>, _tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        // Placeholder for local model integration (e.g., Ollama)
        let url = self.config.base_url.as_deref()
            .unwrap_or("http://localhost:11434/api/chat");

        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false
        });

        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiClientError::ApiError(format!("Local API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiClientError::ParseError(e.to_string()))?;

        // Parse local model response (format may vary)
        Ok(AiResponse {
            content: response_json["message"]["content"].as_str().unwrap_or("").to_string(),
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            usage: None,
        })
    }

    async fn gemini_complete(&self, messages: Vec<AiMessage>, _tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or(AiClientError::MissingApiKey)?;

        let url = format!(
            "{}v1beta/models/{}:generateContent?key={}",
            self.config.base_url.as_deref().unwrap_or("https://generativelanguage.googleapis.com/"),
            self.config.model,
            api_key
        );

        // Convert messages to Gemini format
        let gemini_messages = self.convert_messages_for_gemini(messages);

        let request_body = serde_json::json!({
            "contents": gemini_messages,
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": self.config.max_tokens.unwrap_or(4096)
            }
        });

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiClientError::ApiError(format!("Gemini API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiClientError::ParseError(e.to_string()))?;

        self.parse_gemini_response(response_json)
    }

    async fn ollama_complete(&self, messages: Vec<AiMessage>, _tools: Option<Vec<super::tools::Tool>>) -> Result<AiResponse, AiClientError> {
        let url = format!(
            "{}/api/chat",
            self.config.base_url.as_deref().unwrap_or("http://localhost:11434")
        );

        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens.unwrap_or(4096)
            }
        });

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiClientError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiClientError::ApiError(format!("Ollama API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiClientError::ParseError(e.to_string()))?;

        Ok(AiResponse {
            content: response_json["message"]["content"].as_str().unwrap_or("").to_string(),
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            usage: None,
        })
    }

    async fn openai_stream(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        // Implementation for OpenAI streaming
        // This is a simplified version - real implementation would handle SSE parsing
        let response = self.openai_complete(messages, tools).await?;
        let stream = tokio_stream::once(Ok(StreamingResponse {
            content: response.content,
            is_complete: true,
        }));
        Ok(Box::pin(stream))
    }

    async fn claude_stream(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        // Implementation for Claude streaming
        let response = self.claude_complete(messages, tools).await?;
        let stream = tokio_stream::once(Ok(StreamingResponse {
            content: response.content,
            is_complete: true,
        }));
        Ok(Box::pin(stream))
    }

    async fn groq_stream(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        // Implementation for Groq streaming
        let response = self.groq_complete(messages, tools).await?;
        let stream = tokio_stream::once(Ok(StreamingResponse {
            content: response.content,
            is_complete: true,
        }));
        Ok(Box::pin(stream))
    }

    async fn local_stream(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        // Implementation for local model streaming
        let response = self.local_complete(messages, tools).await?;
        let stream = tokio_stream::once(Ok(StreamingResponse {
            content: response.content,
            is_complete: true,
        }));
        Ok(Box::pin(stream))
    }

    async fn ollama_stream(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        let response = self.ollama_complete(messages, tools).await?;
        let stream = tokio_stream::once(Ok(StreamingResponse {
            content: response.content,
            is_complete: true,
        }));
        Ok(Box::pin(stream))
    }

    async fn gemini_stream(&self, messages: Vec<AiMessage>, tools: Option<Vec<super::tools::Tool>>) -> Result<BoxStream<'_, Result<StreamingResponse, AiClientError>>, AiClientError> {
        let response = self.gemini_complete(messages, tools).await?;
        let stream = tokio_stream::once(Ok(StreamingResponse {
            content: response.content,
            is_complete: true,
        }));
        Ok(Box::pin(stream))
    }

    fn parse_openai_response(&self, response: serde_json::Value) -> Result<AiResponse, AiClientError> {
        let choices = response["choices"].as_array()
            .ok_or(AiClientError::ParseError("No choices in response".to_string()))?;

        let first_choice = choices.first()
            .ok_or(AiClientError::ParseError("Empty choices array".to_string()))?;

        let message = &first_choice["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();
        let finish_reason = first_choice["finish_reason"].as_str().map(|s| s.to_string());

        let usage = response["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(AiResponse {
            content,
            tool_calls: None, // TODO: Parse tool calls
            finish_reason,
            usage,
        })
    }

    fn parse_claude_response(&self, response: serde_json::Value) -> Result<AiResponse, AiClientError> {
        let content = response["content"].as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["text"].as_str())
            .unwrap_or("")
            .to_string();

        let usage = response["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: (u["input_tokens"].as_u64().unwrap_or(0) + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
        });

        Ok(AiResponse {
            content,
            tool_calls: None, // TODO: Parse tool calls
            finish_reason: Some("stop".to_string()),
            usage,
        })
    }

    fn convert_messages_for_claude(&self, messages: Vec<AiMessage>) -> (Option<String>, Vec<AiMessage>) {
        let mut system_message = None;
        let mut claude_messages = Vec::new();

        for message in messages {
            if message.role == "system" {
                system_message = Some(message.content);
            } else {
                claude_messages.push(message);
            }
        }

        (system_message, claude_messages)
    }

    fn convert_messages_for_gemini(&self, messages: Vec<AiMessage>) -> Vec<serde_json::Value> {
        messages.into_iter()
            .filter(|msg| msg.role != "system") // Gemini handles system messages differently
            .map(|msg| {
                serde_json::json!({
                    "role": if msg.role == "assistant" { "model" } else { "user" },
                    "parts": [{"text": msg.content}]
                })
            })
            .collect()
    }

    fn parse_gemini_response(&self, response: serde_json::Value) -> Result<AiResponse, AiClientError> {
        let content = response["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|candidate| candidate["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(AiResponse {
            content,
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            usage: None,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AiClientError {
    #[error("Missing API key")]
    MissingApiKey,
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
