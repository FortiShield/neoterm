use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::ai_client::{ChatMessage, ToolCall, ToolFunction};
use super::tools::{Tool, ToolManager};
use crate::ai::assistant::Assistant;
use crate::block::{Block, BlockContent};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde_json::Value;
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl Conversation {
    pub fn new(id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn get_last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Executes tool calls from an AI message.
    pub async fn execute_tool_calls(&mut self, tool_calls: Vec<ToolCall>, tool_manager: &ToolManager) -> Result<Vec<ChatMessage>> {
        let mut tool_messages = Vec::new();
        for tool_call in tool_calls {
            log::info!("Executing tool call: {:?}", tool_call);
            let tool_name = &tool_call.function.name;
            let tool_args = &tool_call.function.arguments;

            match tool_manager.get_tool(tool_name) {
                Some(tool) => {
                    match tool.execute(tool_args.clone()).await {
                        Ok(output) => {
                            log::info!("Tool '{}' executed successfully. Output: {}", tool_name, output);
                            tool_messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: output,
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            });
                        },
                        Err(e) => {
                            log::error!("Error executing tool '{}': {:?}", tool_name, e);
                            tool_messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: format!("Error: {:?}", e),
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            });
                        }
                    }
                },
                None => {
                    log::warn!("Tool '{}' not found.", tool_name);
                    tool_messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: format!("Error: Tool '{}' not found.", tool_name),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
            }
        }
        Ok(tool_messages)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub model: String,
    pub enable_tool_use: bool,
    pub max_conversation_history: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider_type: "openai".to_string(),
            api_key: None, // Should be loaded from env or config
            model: "gpt-4o".to_string(),
            enable_tool_use: true,
            max_conversation_history: 20,
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
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

pub struct AgentMode {
    assistant: Arc<RwLock<Assistant>>, // Wrap Assistant in Arc<RwLock>
    config: AgentConfig,
    is_active: bool,
    // Channel to send messages from the agent's internal processing to the UI
    message_sender: mpsc::Sender<AgentMessage>,
    message_receiver: mpsc::Receiver<AgentMessage>,
}

impl AgentMode {
    pub fn new(config: AgentConfig) -> Result<Self> {
        let (tx, rx) = mpsc::channel(100); // Channel for agent messages to UI
        let assistant = Arc::new(RwLock::new(Assistant::new(
            &config.provider_type,
            config.api_key.clone(),
            config.model.clone(),
        )?));
        Ok(Self {
            assistant,
            config,
            is_active: false,
            message_sender: tx,
            message_receiver: rx,
        })
    }

    pub fn toggle(&mut self) -> bool {
        self.is_active = !self.is_active;
        self.is_active
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub async fn start_conversation(&mut self) -> Result<()> {
        let mut assistant_lock = self.assistant.write().await;
        assistant_lock.clear_history();
        // Optionally send an initial system message or greeting
        let _ = self.message_sender.send(AgentMessage::SystemMessage("Agent mode activated. How can I help you?".to_string())).await;
        Ok(())
    }

    pub async fn send_message(&mut self, user_message: String, context_blocks: Vec<Block>) -> Result<mpsc::Receiver<AgentMessage>> {
        let sender_clone = self.message_sender.clone();
        let assistant_arc_clone = self.assistant.clone(); // Clone the Arc for the spawned task

        tokio::spawn(async move {
            let mut assistant_lock = assistant_arc_clone.write().await; // Get write lock inside the task

            // Add user message to history
            assistant_lock.conversation_history.push(crate::ai::providers::ChatMessage {
                role: "user".to_string(),
                content: Some(user_message.clone()),
                tool_calls: None,
                tool_call_id: None,
            });

            let stream_result = assistant_lock.stream_chat(&user_message).await;
            match stream_result {
                Ok(mut rx) => {
                    let mut full_response_content = String::new();
                    while let Some(msg) = rx.recv().await {
                        match msg.role.as_str() {
                            "assistant" => {
                                if let Some(content) = msg.content {
                                    full_response_content.push_str(&content);
                                    if sender_clone.send(AgentMessage::AgentResponse(content)).await.is_err() {
                                        log::warn!("Agent message receiver dropped during streaming.");
                                        break;
                                    }
                                }
                            },
                            "tool_calls" => {
                                if let Some(tool_calls) = msg.tool_calls {
                                    for tool_call in tool_calls {
                                        let agent_tool_call = AgentToolCall {
                                            id: tool_call.id,
                                            name: tool_call.function.name,
                                            arguments: tool_call.function.arguments,
                                        };
                                        if sender_clone.send(AgentMessage::ToolCall(agent_tool_call)).await.is_err() {
                                            log::warn!("Agent message receiver dropped during tool call.");
                                            break;
                                        }
                                        // TODO: Execute tool and send result back to assistant
                                    }
                                }
                            },
                            _ => {} // Ignore other roles for now
                        }
                    }
                    // Add the full response to the assistant's history
                    assistant_lock.conversation_history.push(crate::ai::providers::ChatMessage {
                        role: "assistant".to_string(),
                        content: Some(full_response_content),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    let _ = sender_clone.send(AgentMessage::Done).await;
                },
                Err(e) => {
                    let _ = sender_clone.send(AgentMessage::Error(format!("AI stream error: {}", e))).await;
                }
            }
        });

        Ok(self.message_receiver.clone()) // Return a clone of the receiver for the UI to subscribe
    }

    // New method for command generation
    pub async fn generate_command(&mut self, natural_language_query: &str) -> Result<String> {
        let mut assistant_lock = self.assistant.write().await; // Get write lock
        assistant_lock.generate_command(natural_language_query).await
    }
}
