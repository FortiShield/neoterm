use super::{AIProvider, ChatMessage, ToolCall, ToolFunction};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use futures_util::StreamExt;
use bytes::BytesMut;

pub struct OllamaProvider {
    client: Client,
    model: String,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            model,
            base_url: "http://localhost:11434".to_string(), // Default Ollama URL
        })
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    async fn chat_completion(&self, messages: Vec<ChatMessage>, _tools: Option<Value>) -> Result<ChatMessage> {
        // Ollama's /api/chat endpoint supports messages directly
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        let response = self.client
            .post(&format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let json_response: Value = response.json().await?;

        let message = json_response["message"].clone();
        let role = message["role"].as_str().unwrap_or("assistant").to_string();
        let content = message["content"].as_str().unwrap_or("").to_string();

        Ok(ChatMessage {
            role,
            content,
            tool_calls: None, // Ollama's native tool calling is not as standardized yet
            tool_call_id: None,
        })
    }

    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, _tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let (tx, rx) = mpsc::channel(100);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        let request_builder = self.client
            .post(&format!("{}/api/chat", self.base_url))
            .json(&body);

        tokio::spawn(async move {
            let response = match request_builder.send().await {
                Ok(res) => res,
                Err(e) => {
                    log::error!("Failed to send stream request to Ollama: {:?}", e);
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                log::error!("Ollama stream request failed with status {}: {}", status, text);
                return;
            }

            let mut stream = response.bytes_stream();
            let mut buffer = BytesMut::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.extend_from_slice(&chunk);
                        while let Some(line) = extract_line(&mut buffer) {
                            match serde_json::from_str::<Value>(&line) {
                                Ok(json_chunk) => {
                                    if let Some(message_obj) = json_chunk["message"].as_object() {
                                        let role = message_obj["role"].as_str().unwrap_or_default().to_string();
                                        let content = message_obj["content"].as_str().unwrap_or_default().to_string();
                                        
                                        let msg = ChatMessage {
                                            role,
                                            content,
                                            tool_calls: None,
                                            tool_call_id: None,
                                        };
                                        if tx.send(msg).await.is_err() {
                                            log::warn!("Receiver dropped, stopping Ollama stream.");
                                            return;
                                        }
                                    }
                                    if json_chunk["done"].as_bool().unwrap_or(false) {
                                        break; // Stream is done
                                    }
                                },
                                Err(e) => log::error!("Failed to parse JSON chunk from Ollama: {:?} from data: {}", e, line),
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("Error in Ollama stream: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

fn extract_line(buffer: &mut BytesMut) -> Option<String> {
    if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.split_to(newline_pos + 1);
        String::from_utf8(line.to_vec()).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}
