use super::{AIProvider, ChatMessage, ToolCall, ToolFunction};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use futures_util::StreamExt;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: Option<String>, model: String) -> Result<Self> {
        let api_key = api_key.ok_or_else(|| anyhow!("Anthropic API key not provided"))?;
        Ok(Self {
            api_key,
            model,
            client: Client::new(),
        })
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage> {
        let system_message = messages.iter().find(|m| m.role == "system").map(|m| m.content.clone().unwrap_or_default());
        let user_messages: Vec<ChatMessage> = messages.into_iter().filter(|m| m.role != "system").collect();

        let mut request_body = json!({
            "model": self.model,
            "messages": user_messages,
            "max_tokens": 4096, // A reasonable default
        });

        if let Some(sys_msg) = system_message {
            request_body["system"] = json!(sys_msg);
        }
        
        // Anthropic's tool support is different (tool_use, tool_results)
        // This simplified example doesn't fully map it, but you'd convert `tools` here.
        if let Some(t) = tools {
            // TODO: Convert generic tools to Anthropic's specific tool format
            log::warn!("Anthropic tool support is not fully implemented in chat_completion.");
        }

        let response = self.client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01") // Required API version
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        log::debug!("Anthropic chat_completion response: {:?}", response);

        let content_blocks = response["content"].as_array().ok_or_else(|| anyhow!("No content blocks in Anthropic response"))?;
        let mut full_content = String::new();
        let mut tool_calls: Option<Vec<ToolCall>> = None;

        for block in content_blocks {
            if block["type"] == "text" {
                if let Some(text) = block["text"].as_str() {
                    full_content.push_str(text);
                }
            } else if block["type"] == "tool_use" {
                // Anthropic tool_use block
                let tool_name = block["name"].as_str().unwrap_or_default().to_string();
                let tool_id = block["id"].as_str().unwrap_or_default().to_string();
                let tool_arguments = block["input"].clone(); // Anthropic uses "input" for arguments

                let tc = ToolCall {
                    id: tool_id,
                    type_: "function".to_string(),
                    function: ToolFunction {
                        name: tool_name,
                        arguments: tool_arguments,
                    },
                };
                tool_calls.get_or_insert_with(Vec::new).push(tc);
            }
        }

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content: Some(full_content),
            tool_calls,
            tool_call_id: None, // Anthropic doesn't provide this on the top-level message
        })
    }

    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let (tx, rx) = mpsc::channel(100);

        let system_message = messages.iter().find(|m| m.role == "system").map(|m| m.content.clone().unwrap_or_default());
        let user_messages: Vec<ChatMessage> = messages.into_iter().filter(|m| m.role != "system").collect();

        let mut request_body = json!({
            "model": self.model,
            "messages": user_messages,
            "max_tokens": 4096,
            "stream": true,
        });

        if let Some(sys_msg) = system_message {
            request_body["system"] = json!(sys_msg);
        }

        if let Some(t) = tools {
            // TODO: Convert generic tools to Anthropic's specific tool format for streaming
            log::warn!("Anthropic tool support is not fully implemented in stream_chat_completion.");
        }

        let request_builder = self.client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body);

        tokio::spawn(async move {
            match request_builder.send().await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    let mut current_tool_calls: HashMap<String, ToolCall> = HashMap::new();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                let chunk_str = String::from_utf8_lossy(&chunk);
                                for line in chunk_str.lines() {
                                    if line.starts_with("event: message_start") || line.starts_with("event: content_block_start") ||
                                       line.starts_with("event: content_block_delta") || line.starts_with("event: content_block_stop") ||
                                       line.starts_with("event: message_delta") || line.starts_with("event: message_stop") {
                                        // Parse the data part of the event
                                        if let Some(data_line) = line.strip_prefix("data: ") {
                                            match serde_json::from_str::<Value>(data_line) {
                                                Ok(event) => {
                                                    match event["type"].as_str() {
                                                        Some("content_block_delta") => {
                                                            if let Some(delta) = event["delta"].as_object() {
                                                                if delta["type"] == "text_delta" {
                                                                    if let Some(text_chunk) = delta["text"].as_str() {
                                                                        if tx.send(ChatMessage {
                                                                            role: "assistant".to_string(),
                                                                            content: Some(text_chunk.to_string()),
                                                                            tool_calls: None,
                                                                            tool_call_id: None,
                                                                        }).await.is_err() {
                                                                            log::warn!("Receiver dropped, stopping Anthropic stream.");
                                                                            return;
                                                                        }
                                                                    }
                                                                } else if delta["type"] == "tool_use" {
                                                                    // Handle tool_use delta (e.g., arguments streaming)
                                                                    let tool_id = event["content_block"]["id"].as_str().unwrap_or_default().to_string();
                                                                    let tool_name = event["content_block"]["name"].as_str().unwrap_or_default().to_string();
                                                                    let input_chunk = delta["input"].as_str().unwrap_or_default().to_string();

                                                                    let entry = current_tool_calls.entry(tool_id.clone()).or_insert_with(|| ToolCall {
                                                                        id: tool_id.clone(),
                                                                        type_: "function".to_string(),
                                                                        function: ToolFunction {
                                                                            name: tool_name.clone(),
                                                                            arguments: Value::String("".to_string()),
                                                                        },
                                                                    });

                                                                    if let Value::String(ref mut args_str) = entry.function.arguments {
                                                                        args_str.push_str(&input_chunk);
                                                                    } else {
                                                                        entry.function.arguments = Value::String(input_chunk);
                                                                    }

                                                                    if tx.send(ChatMessage {
                                                                        role: "tool_calls".to_string(),
                                                                        content: None,
                                                                        tool_calls: Some(vec![entry.clone()]),
                                                                        tool_call_id: None,
                                                                    }).await.is_err() {
                                                                        log::warn!("Receiver dropped, stopping Anthropic stream.");
                                                                        return;
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        Some("message_stop") => {
                                                            // Stream finished
                                                            break;
                                                        },
                                                        _ => {}
                                                    }
                                                },
                                                Err(e) => log::error!("Failed to parse Anthropic stream event data: {:?} - {}", data_line, e),
                                            }
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                log::error!("Error receiving chunk from Anthropic stream: {:?}", e);
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
                    log::error!("Failed to send request to Anthropic: {:?}", e);
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
