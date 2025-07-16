use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use async_trait::async_trait;
use chrono::Utc;
use crate::agent_mode_eval::ai_client::{AIClient, OpenAIClient, AIClientError, AIStreamChunk};
use crate::block::Block as UIBlock; // Alias to avoid conflict with Message

pub mod ai_client;
pub mod conversation;
pub mod tools;

pub use ai_client::{AIClient, AiConfig, AiProvider, Message, MessageRole};
pub use conversation::{Conversation, ConversationManager};
pub use tools::{Tool, ToolManager, ToolResult as ToolExecutionResult, ToolCall as AgentToolCall};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub ai_config: ai_client::AIClientConfig,
    pub max_history_length: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ai_config: ai_client::AIClientConfig::OpenAI { api_key: None },
            max_history_length: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentMessage {
    UserMessage(String),
    AgentResponse(String),
    ToolCall(AgentToolCall),
    ToolResult(String),
    SystemMessage(String),
    Error(String),
    Done, // Indicates the AI conversation turn is complete
}

#[derive(Debug, Clone)]
pub struct AgentMode {
    is_enabled: bool,
    client: Box<dyn AIClient + Send + Sync>,
    conversation_history: VecDeque<Message>, // Stores ai_client::Message for full context
    config: AgentConfig,
    tool_registry: tools::ToolRegistry,
}

impl AgentMode {
    pub fn new(config: AgentConfig) -> Result<Self, AIClientError> {
        let client: Box<dyn AIClient + Send + Sync> = match &config.ai_config {
            ai_client::AIClientConfig::OpenAI { api_key } => {
                let key = api_key.clone().ok_or(AIClientError::ConfigurationError("OpenAI API key not provided".to_string()))?;
                Box::new(OpenAIClient::new(key, "gpt-4o".to_string())) // Default model for OpenAIClient
            }
            // Add other AI clients here
        };

        Ok(Self {
            is_enabled: false,
            client,
            conversation_history: VecDeque::new(),
            config,
            tool_registry: tools::ToolRegistry::new(),
        })
    }

    pub fn toggle(&mut self) -> bool {
        self.is_enabled = !self.is_enabled;
        self.is_enabled
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn start_conversation(&mut self) -> Result<(), AIClientError> {
        self.conversation_history.clear();
        self.conversation_history.push_back(Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: self.config.ai_config.get_system_prompt().unwrap_or_else(|| "You are a helpful terminal assistant. Provide concise answers and command suggestions.".to_string()),
            timestamp: Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
        Ok(())
    }

    pub async fn send_message(&mut self, user_message: String, context_blocks: Vec<UIBlock>) -> Result<mpsc::Receiver<AgentMessage>, AIClientError> {
        let (tx, rx) = mpsc::channel(100);
        let client_clone = self.client.clone_box();
        let tool_registry_clone = self.tool_registry.clone();
        let mut conversation_history_for_task = self.conversation_history.clone(); // Clone for the async task

        // Add user message to history
        conversation_history_for_task.push_back(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: user_message.clone(),
            timestamp: Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
        self.trim_history(&mut conversation_history_for_task);

        // Add contextual blocks to the conversation for the AI
        for block in context_blocks {
            let context_content = match block.content {
                crate::block::BlockContent::Command { input, output, status, .. } => {
                    let output_str = output.iter().map(|(s, _)| s.clone()).collect::<Vec<String>>().join("\n");
                    format!("Previous command: `{}`\nOutput:\n\`\`\`\n{}\n\`\`\`\nStatus: {}", input, output_str, status)
                },
                crate::block::BlockContent::AgentMessage { content, is_user, .. } => {
                    format!("Previous {}: {}", if is_user { "User" } else { "Agent" }, content)
                },
                crate::block::BlockContent::Info { title, message, .. } => {
                    format!("Info ({}): {}", title, message)
                },
                crate::block::BlockContent::Error { message, .. } => {
                    format!("Error: {}", message)
                },
            };
            conversation_history_for_task.push_back(Message {
                id: Uuid::new_v4(),
                role: MessageRole::System, // Using System role for context
                content: format!("Context from previous block:\n{}", context_content),
                timestamp: Utc::now(),
                tool_calls: None,
                tool_results: None,
            });
        }

        let tools_for_ai = tool_registry_clone.get_available_tools();

        tokio::spawn(async move {
            let mut current_agent_response_text = String::new();
            let mut tool_calls_to_execute: Vec<AgentToolCall> = Vec::new();

            // Loop for multi-turn interactions (AI -> Tool -> AI)
            loop {
                let stream_result = client_clone.stream_response(conversation_history_for_task.clone().into(), Some(tools_for_ai.clone())).await;

                match stream_result {
                    Ok(mut stream) => {
                        let mut stream_finished = false;
                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    match chunk {
                                        AIStreamChunk::Text(text_chunk) => {
                                            current_agent_response_text.push_str(&text_chunk);
                                            if let Err(_) = tx.send(AgentMessage::AgentResponse(text_chunk)).await {
                                                eprintln!("Failed to send text chunk to UI");
                                                return;
                                            }
                                        },
                                        AIStreamChunk::ToolCall(tool_calls) => {
                                            tool_calls_to_execute.extend(tool_calls);
                                            // Send tool calls to UI immediately
                                            for tc in &tool_calls_to_execute {
                                                if let Err(_) = tx.send(AgentMessage::ToolCall(tc.clone())).await {
                                                    eprintln!("Failed to send tool call to UI");
                                                    return;
                                                }
                                            }
                                        },
                                        AIStreamChunk::Done => {
                                            stream_finished = true;
                                            break; // Stream finished
                                        },
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Error streaming AI response: {}", e);
                                    if let Err(_) = tx.send(AgentMessage::Error(format!("AI Stream Error: {}", e))).await {
                                        eprintln!("Failed to send error to UI");
                                    }
                                    return;
                                }
                            }
                        }

                        // After stream finishes, add the AI's response (if any) to history
                        if !current_agent_response_text.is_empty() {
                            conversation_history_for_task.push_back(Message {
                                id: Uuid::new_v4(),
                                role: MessageRole::Assistant,
                                content: current_agent_response_text.clone(),
                                timestamp: Utc::now(),
                                tool_calls: None,
                                tool_results: None,
                            });
                            current_agent_response_text.clear();
                        }

                        // Check for tool calls to execute
                        if !tool_calls_to_execute.is_empty() {
                            let mut tool_results_for_ai: Vec<ai_client::ToolResult> = Vec::new();
                            for tc in tool_calls_to_execute.drain(..) {
                                let tool_result = tool_registry_clone.execute_tool(tc.clone()).await;
                                match tool_result {
                                    Ok(tr) => {
                                        tool_results_for_ai.push(ai_client::ToolResult {
                                            tool_call_id: tc.id.clone(),
                                            content: tr.output.clone(),
                                            is_error: !tr.success,
                                        });
                                        if let Err(_) = tx.send(AgentMessage::ToolResult(tr.output)).await {
                                            eprintln!("Failed to send tool result to UI");
                                            return;
                                        }
                                    },
                                    Err(e) => {
                                        let error_msg = format!("Tool execution error: {}", e);
                                        tool_results_for_ai.push(ai_client::ToolResult {
                                            tool_call_id: tc.id.clone(),
                                            content: error_msg.clone(),
                                            is_error: true,
                                        });
                                        if let Err(_) = tx.send(AgentMessage::Error(error_msg)).await {
                                            eprintln!("Failed to send tool error to UI");
                                            return;
                                        }
                                    }
                                }
                            }

                            // Add tool results to the conversation for the next AI turn
                            conversation_history_for_task.push_back(Message {
                                id: Uuid::new_v4(),
                                role: MessageRole::Tool,
                                content: "".to_string(), // Content is empty for tool results, actual content is in tool_results field
                                timestamp: Utc::now(),
                                tool_calls: None,
                                tool_results: Some(tool_results_for_ai),
                            });

                            // Continue the loop for another AI turn
                            continue; // Go to the next iteration of the loop
                        } else {
                            // No tool calls, AI response is complete
                            if stream_finished {
                                let _ = tx.send(AgentMessage::Done);
                                break; // Exit the loop
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to get AI stream: {}", e);
                        if let Err(_) = tx.send(AgentMessage::Error(format!("AI Client Error: {}", e))).await {
                            eprintln!("Failed to send error to UI");
                        }
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    fn trim_history(&self, history: &mut VecDeque<Message>) {
        while history.len() > self.config.max_history_length {
            history.pop_front();
        }
    }
}

impl ai_client::AIClientConfig {
    pub fn get_system_prompt(&self) -> Option<String> {
        match self {
            ai_client::AIClientConfig::OpenAI { .. } => Some("You are a helpful terminal assistant. Provide concise answers and command suggestions. When suggesting commands, use the `execute_command` tool. When asked to read or write files, use `read_file` or `write_file` tools. When asked about system information, use `get_system_info`.".to_string()),
            // Add system prompts for other providers if needed
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStats {
    pub message_count: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub tool_calls: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

pub fn init() {
    tracing::info!("Agent mode system initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_mode_creation() {
        let config = AgentConfig::default();
        let agent = AgentMode::new(config);
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_conversation_lifecycle() {
        let config = AgentConfig::default();
        let mut agent = AgentMode::new(config).unwrap();
        
        let result = agent.start_conversation();
        assert!(result.is_ok());
        
        // Test sending a message and receiving a stream
        // This test would require mocking the AIClient for a true unit test
        // For integration testing, you'd need a real API key and network access.
        // let rx = agent.send_message("Hello".to_string(), vec![]).await.unwrap();
        // while let Some(msg) = rx.recv().await {
        //     println!("{:?}", msg);
        //     if let AgentMessage::Done = msg {
        //         break;
        //     }
        // }
    }
}
