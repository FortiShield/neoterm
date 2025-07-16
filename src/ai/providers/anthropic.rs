use super::{AIProvider, ChatMessage, ToolCall, ToolFunction};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc;
use futures_util::StreamExt;
use bytes::BytesMut;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: Option<String>, model: String) -> Result<Self> {
        let api_key = api_key.ok_or_else(|| anyhow!("Anthropic API key not provided"))?;
        Ok(Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.anthropic.com/v1".to_string(),
        })
    }

    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::HeaderName::from_static("x-api-key"),
            self.api_key.parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("anthropic-version"),
            "2023-06-01".parse().unwrap(), // Required for Anthropic API
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers
    }

    // Helper to format messages for Claude's API
    fn format_messages_for_claude(&self, messages: Vec<ChatMessage>) -> Vec<Value> {
        messages.into_iter().filter_map(|msg| {
            match msg.role.as_str() {
                "system" => None, // System prompt is handled separately for Claude
                "user" => Some(serde_json::json!({
                    "role": "user",
                    "content": if let Some(tool_call_id) = msg.tool_call_id {
                        // Claude tool results are sent as 'user' role with 'tool_result' type
                        serde_json::json!([
                            {
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": msg.content,
                            }
                        ])
                    } else {
                        serde_json::json!([{"type": "text", "text": msg.content}])
                    }
                })),
                "assistant" => Some(serde_json::json!({
                    "role": "assistant",
                    "content": if let Some(tool_calls) = msg.tool_calls {
                        // Claude tool calls are sent as 'assistant' role with 'tool_use' type
                        serde_json::json!(tool_calls.into_iter().map(|tc| {
                            serde_json::json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.function.name,
                                "input": tc.function.arguments, // Arguments are direct JSON
                            })
                        }).collect::<Vec<_>>())
                    } else {
                        serde_json::json!([{"type": "text", "text": msg.content}])
                    }
                })),
                _ => None, // Tool role is converted to user with tool_result content for Claude
            }
        }).collect()
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    async fn chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage> {
        let formatted_messages = self.format_messages_for_claude(messages);
        let system_prompt = if let Some(msg) = formatted_messages.iter().find(|m| m["role"] == "system") {
            msg["content"].as_str().map(|s| s.to_string())
        } else {
            None
        };

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": formatted_messages,
            "max_tokens": 4096, // Claude requires max_tokens
        });

        if let Some(sys_p) = system_prompt {
            body["system"] = Value::String(sys_p);
        }

        if let Some(t) = tools {
            body["tools"] = t;
        }

        let response = self.client
            .post(&format!("{}/messages", self.base_url))
            .headers(self.get_headers())
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let json_response: Value = response.json().await?;
        
        let content_blocks = json_response["content"].as_array().ok_or_else(|| anyhow!("No content blocks in Claude response"))?;
        let mut full_content = String::new();
        let mut tool_calls: Option<Vec<ToolCall>> = None;

        for block in content_blocks {
            match block["type"].as_str() {
                Some("text") => {
                    if let Some(text) = block["text"].as_str() {
                        full_content.push_str(text);
                    }
                },
                Some("tool_use") => {
                    let id = block["id"].as_str().unwrap_or_default().to_string();
                    let name = block["name"].as_str().unwrap_or_default().to_string();
                    let input = block["input"].clone();
                    let tc = ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: ToolFunction {
                            name,
                            arguments: input,
                        },
                    };
                    tool_calls.get_or_insert_with(Vec::new).push(tc);
                },
                _ => {}
            }
        }

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content: full_content,
            tool_calls,
            tool_call_id: None,
        })
    }

    async fn stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let (tx, rx) = mpsc::channel(100);

        let formatted_messages = self.format_messages_for_claude(messages);
        let system_prompt = if let Some(msg) = formatted_messages.iter().find(|m| m["role"] == "system") {
            msg["content"].as_str().map(|s| s.to_string())
        } else {
            None
        };

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": formatted_messages,
            "max_tokens": 4096, // Claude requires max_tokens
            "stream": true,
        });

        if let Some(sys_p) = system_prompt {
            body["system"] = Value::String(sys_p);
        }

        if let Some(t) = tools {
            body["tools"] = t;
        }

        let request_builder = self.client
            .post(&format!("{}/messages", self.base_url))
            .headers(self.get_headers())
            .json(&body);

        tokio::spawn(async move {
            let response = match request_builder.send().await {
                Ok(res) => res,
                Err(e) => {
                    log::error!("Failed to send stream request to Anthropic: {:?}", e);
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                log::error!("Anthropic stream request failed with status {}: {}", status, text);
                return;
            }

            let mut stream = response.bytes_stream();
            let mut buffer = BytesMut::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.extend_from_slice(&chunk);
                        while let Some(line) = extract_line(&mut buffer) {
                            if line.starts_with("event: ") {
                                let event_type = &line[7..];
                                if event_type == "message_stop" {
                                    break; // Stream is done
                                }
                            } else if line.starts_with("data: ") {
                                let data = &line[6..];
                                match serde_json::from_str::<Value>(data) {
                                    Ok(json_chunk) => {
                                        match json_chunk["type"].as_str() {
                                            Some("content_block_delta") => {
                                                if let Some(text) = json_chunk["delta"]["text"].as_str() {
                                                    let msg = ChatMessage {
                                                        role: "assistant".to_string(),
                                                        content: text.to_string(),
                                                        tool_calls: None,
                                                        tool_call_id: None,
                                                    };
                                                    if tx.send(msg).await.is_err() {
                                                        log::warn!("Receiver dropped, stopping Anthropic stream.");
                                                        return;
                                                    }
                                                }
                                            },
                                            Some("content_block_start") => {
                                                if let Some(block_type) = json_chunk["content_block"]["type"].as_str() {
                                                    if block_type == "tool_use" {
                                                        let id = json_chunk["content_block"]["id"].as_str().unwrap_or_default().to_string();
                                                        let name = json_chunk["content_block"]["name"].as_str().unwrap_or_default().to_string();
                                                        let input = json_chunk["content_block"]["input"].clone();
                                                        let tc = ToolCall {
                                                            id,
                                                            call_type: "function".to_string(),
                                                            function: ToolFunction {
                                                                name,
                                                                arguments: input,
                                                            },
                                                        };
                                                        let msg = ChatMessage {
                                                            role: "assistant".to_string(),
                                                            content: "".to_string(), // Tool calls don't have content in delta
                                                            tool_calls: Some(vec![tc]),
                                                            tool_call_id: None,
                                                        };
                                                        if tx.send(msg).await.is_err() {
                                                            log::warn!("Receiver dropped, stopping Anthropic stream.");
                                                            return;
                                                        }
                                                    }
                                                }
                                            },
                                            _ => {}
                                        }
                                    },
                                    Err(e) => log::error!("Failed to parse JSON chunk from Anthropic: {:?} from data: {}", e, data),
                                }
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("Error in Anthropic stream: {:?}", e);
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
