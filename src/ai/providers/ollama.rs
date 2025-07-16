use super::{AIProvider, ChatMessage, ToolCall, ToolFunction};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::Client;
use tokio::sync::mpsc;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    model: String,
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Result<Self> {
        Ok(Self {
            model,
            client: Client::new(),
            base_url: "http://localhost:11434/api".to_string(), // Default Ollama local URL
        })
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat_completion(&self, messages: Vec<ChatMessage>, _tools: Option<Value>) -> Result<ChatMessage> {
        // Ollama's /api/chat endpoint
        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        let response = self.client.post(format!("{}/chat", self.base_url))
            .json(&request_body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        log::debug!("Ollama chat_completion response: {:?}", response);

        let message = response["message"].clone();
        let content = message["content"].as_str().map(|s| s.to_string());
        let role = message["role"].as_str().unwrap_or("assistant").to_string();

        Ok(ChatMessage {
            role,
            content,
            tool_calls: None, // Ollama currently has limited tool call support
            tool_call_id: None,
        })
    }

    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, _tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let (tx, rx) = mpsc::channel(100);

        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        let request_builder = self.client.post(format!("{}/chat", self.base_url))
            .json(&request_body);

        tokio::spawn(async move {
            match request_builder.send().await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                let chunk_str = String::from_utf8_lossy(&chunk);
                                for line in chunk_str.lines() {
                                    match serde_json::from_str::<Value>(line) {
                                        Ok(event) => {
                                            if let Some(message_delta) = event["message"].as_object() {
                                                if let Some(content_chunk) = message_delta["content"].as_str() {
                                                    if tx.send(ChatMessage {
                                                        role: "assistant".to_string(),
                                                        content: Some(content_chunk.to_string()),
                                                        tool_calls: None,
                                                        tool_call_id: None,
                                                    }).await.is_err() {
                                                        log::warn!("Receiver dropped, stopping Ollama stream.");
                                                        return;
                                                    }
                                                }
                                            }
                                            if event["done"].as_bool().unwrap_or(false) {
                                                break; // Stream finished
                                            }
                                        },
                                        Err(e) => log::error!("Failed to parse Ollama stream event: {:?} - {}", line, e),
                                    }
                                }
                            },
                            Err(e) => {
                                log::error!("Error receiving chunk from Ollama stream: {:?}", e);
                                let _ = tx.send(ChatMessage {
                                    role: "error".to_string(),
                                    content: Some(format!("Stream error: {}", e)),
                                    tool_calls: None,
                                    tool_call_id: None,
                                }).await;
                                break;
                            }
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to send request to Ollama: {:?}", e);
                    let _ = tx.send(ChatMessage {
                        role: "error".to_string(),
                        content: Some(format!("Request error: {}", e)),
                        tool_calls: None,
                        tool_call_id: None,
                    }).await;
                }
            }
        });

        Ok(rx)
    }
}
