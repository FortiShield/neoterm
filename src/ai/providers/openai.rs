use super::{AIProvider, ChatMessage, ToolCall, ToolFunction};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::{Client, Error as ReqwestError};
use tokio::sync::mpsc;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap; // For current_tool_calls

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct UsageResponse {
    pub total_granted: f64,
    pub total_used: f64,
    pub total_available: f64,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>, model: String) -> Result<Self> {
        let api_key = api_key.ok_or_else(|| anyhow!("OpenAI API key not provided"))?;
        Ok(Self {
            api_key,
            model,
            client: Client::new(),
        })
    }

    pub async fn get_usage_quota(&self) -> Result<UsageResponse> {
        let response = self.client.get("https://api.openai.com/v1/dashboard/billing/credit_grants")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .json::<UsageResponse>()
            .await?;
        Ok(response)
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage> {
        let mut request_body = json!({
            "model": self.model,
            "messages": messages,
        });

        if let Some(t) = tools {
            request_body["tools"] = t;
        }

        let response = self.client.post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        log::debug!("OpenAI chat_completion response: {:?}", response);

        let choice = response["choices"][0].clone();
        let message = choice["message"].clone();

        let content = message["content"].as_str().map(|s| s.to_string());
        let role = message["role"].as_str().unwrap_or("assistant").to_string();

        let tool_calls = if message["tool_calls"].is_array() {
            let mut calls = Vec::new();
            for tc_val in message["tool_calls"].as_array().unwrap() {
                if let Ok(tool_call) = serde_json::from_value::<ToolCall>(tc_val.clone()) {
                    calls.push(tool_call);
                }
            }
            Some(calls)
        } else {
            None
        };

        Ok(ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id: None, // OpenAI doesn't provide this on the top-level message
        })
    }

    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let (tx, rx) = mpsc::channel(100);

        let mut request_body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(t) = tools {
            request_body["tools"] = t;
        }

        let request_builder = self.client.post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body);

        tokio::spawn(async move {
            match request_builder.send().await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    let mut current_content = String::new();
                    let mut current_tool_calls: HashMap<String, ToolCall> = HashMap::new();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                let chunk_str = String::from_utf8_lossy(&chunk);
                                for line in chunk_str.lines() {
                                    if line.starts_with("data: ") {
                                        let data = &line[6..];
                                        if data == "[DONE]" {
                                            break;
                                        }
                                        match serde_json::from_str::<Value>(data) {
                                            Ok(event) => {
                                                if let Some(delta) = event["choices"][0]["delta"].as_object() {
                                                    if let Some(content_chunk) = delta["content"].as_str() {
                                                        current_content.push_str(content_chunk);
                                                        if tx.send(ChatMessage {
                                                            role: "assistant".to_string(),
                                                            content: Some(content_chunk.to_string()),
                                                            tool_calls: None,
                                                            tool_call_id: None,
                                                        }).await.is_err() {
                                                            log::warn!("Receiver dropped, stopping OpenAI stream.");
                                                            return;
                                                        }
                                                    }
                                                    
                                                    if let Some(tool_calls_array) = delta["tool_calls"].as_array() {
                                                        for tool_call_delta in tool_calls_array {
                                                            let index = tool_call_delta["index"].as_u64().unwrap_or(0) as usize;
                                                            let id = tool_call_delta["id"].as_str().unwrap_or_default().to_string();
                                                            let name = tool_call_delta["function"]["name"].as_str().unwrap_or_default().to_string();
                                                            let arguments_chunk = tool_call_delta["function"]["arguments"].as_str().unwrap_or_default().to_string();

                                                            let entry = current_tool_calls.entry(id.clone()).or_insert_with(|| ToolCall {
                                                                id: id.clone(),
                                                                type_: "function".to_string(),
                                                                function: ToolFunction {
                                                                    name: name.clone(),
                                                                    arguments: Value::String("".to_string()),
                                                                },
                                                            });
                                                            
                                                            // Append argument chunks
                                                            if let Value::String(ref mut args_str) = entry.function.arguments {
                                                                args_str.push_str(&arguments_chunk);
                                                            } else {
                                                                entry.function.arguments = Value::String(arguments_chunk);
                                                            }

                                                            // Send tool call delta
                                                            if tx.send(ChatMessage {
                                                                role: "tool_calls".to_string(),
                                                                content: None,
                                                                tool_calls: Some(vec![entry.clone()]), // Send current state of tool call
                                                                tool_call_id: None,
                                                            }).await.is_err() {
                                                                log::warn!("Receiver dropped, stopping OpenAI stream.");
                                                                return;
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                            Err(e) => log::error!("Failed to parse OpenAI stream event: {:?} - {}", data, e),
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                log::error!("Error receiving chunk from OpenAI stream: {:?}", e);
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
                    log::error!("Failed to send request to OpenAI: {:?}", e);
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
