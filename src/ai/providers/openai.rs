use super::{AIProvider, ChatMessage, ToolCall, ToolFunction};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use futures_util::StreamExt;
use bytes::BytesMut;

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>, model: String) -> Result<Self> {
        let api_key = api_key.ok_or_else(|| anyhow!("OpenAI API key not provided"))?;
        Ok(Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
        })
    }

    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.api_key).parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });

        if let Some(t) = tools {
            body["tools"] = t;
            body["tool_choice"] = serde_json::json!("auto");
        }

        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .headers(self.get_headers())
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let json_response: Value = response.json().await?;
        
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
        let tool_call_id = message["tool_call_id"].as_str().map(|s| s.to_string());

        Ok(ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
        })
    }

    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let (tx, rx) = mpsc::channel(100); // Channel for sending chunks

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true, // Enable streaming
        });

        if let Some(t) = tools {
            body["tools"] = t;
            body["tool_choice"] = serde_json::json!("auto");
        }

        let request_builder = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .headers(self.get_headers())
            .json(&body);

        tokio::spawn(async move {
            let response = match request_builder.send().await {
                Ok(res) => res,
                Err(e) => {
                    log::error!("Failed to send stream request: {:?}", e);
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                log::error!("Stream request failed with status {}: {}", status, text);
                return;
            }

            let mut stream = response.bytes_stream();
            let mut buffer = BytesMut::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.extend_from_slice(&chunk);
                        while let Some(line) = extract_line(&mut buffer) {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    break;
                                }
                                match serde_json::from_str::<Value>(data) {
                                    Ok(json_chunk) => {
                                        if let Some(delta) = json_chunk["choices"][0]["delta"].as_object() {
                                            let role = delta["role"].as_str().unwrap_or_default().to_string();
                                            let content = delta["content"].as_str().unwrap_or_default().to_string();
                                            
                                            let tool_calls = delta["tool_calls"].as_array().map(|calls| {
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

                                            let msg = ChatMessage {
                                                role,
                                                content,
                                                tool_calls,
                                                tool_call_id: None, // Tool call ID is for tool_message, not delta
                                            };
                                            if tx.send(msg).await.is_err() {
                                                log::warn!("Receiver dropped, stopping stream.");
                                                return;
                                            }
                                        }
                                    },
                                    Err(e) => log::error!("Failed to parse JSON chunk: {:?} from data: {}", e, data),
                                }
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("Error in stream: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

fn extract_line(buffer: &mut BytesMut) -> Option<String> {
    if let Some(newline_pos) = buffer.windows(2).position(|w| w == b"\n\n") {
        let line = buffer.split_to(newline_pos + 2);
        String::from_utf8(line.to_vec()).ok().map(|s| s.trim().to_string())
    } else if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line = buffer.split_to(newline_pos + 1);
        String::from_utf8(line.to_vec()).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}
