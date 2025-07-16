use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use async_trait::async_trait;
use chrono::Utc;
use crate::agent_mode_eval::ai_client::{AIClient, OpenAIClient, AIClientError, AIStreamChunk};
use crate::block::{Block as UIBlock, Block, BlockContent, BlockId, BlockType}; // Alias to avoid conflict with Message
use crate::command::CommandExecutor;
use crate::config::Config;
use crate::agent_mode_eval::conversation::{Conversation, Message, MessageRole};
use crate::agent_mode_eval::tools::{Tool, ToolRegistry, ToolCall, ToolResult};
use ai_client::{AIClient, ChatMessage, AiConfig, OpenAIClient};
use conversation::Conversation;
use tools::ToolManager;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::workflows::Workflow; // Import Workflow

pub mod ai_client;
pub mod conversation;
pub mod tools;

use crate::ai::assistant::Assistant;
use crate::ai::providers::ChatMessage as ProviderChatMessage;
use crate::block::{Block, BlockContent};
use crate::agent_mode_eval::tools::{ToolCall as AgentToolCall, ToolResult};

pub use conversation::{Conversation, Message, MessageRole};
pub use tools::{Tool, ToolCall, ToolResult};

#[derive(Debug, Clone)]
pub enum AgentMessage {
    UserMessage(String),
    AgentResponse(String),
    ToolCall(AgentToolCall),
    ToolResult(String), // Simplified for now, could be more structured
    SystemMessage(String),
    Error(String),
    Done,
    WorkflowSuggested(Workflow), // New: AI suggests a multi-step workflow
    AgentPromptRequest { // New: Agent requests user input during a workflow
        prompt_id: String,
        message: String,
    },
    AgentPromptResponse { // New: User's response to an agent prompt
        prompt_id: String,
        response: String,
    },
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub model: String,
    pub system_prompt: String,
    pub tools_enabled: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider_type: "openai".to_string(),
            api_key: None, // Should be loaded from env or config
            model: "gpt-4o".to_string(),
            system_prompt: "You are a helpful AI assistant integrated into a terminal. You can execute commands, read files, and help with development tasks.".to_string(),
            tools_enabled: true,
        }
    }
}

pub struct AgentMode {
    assistant: Arc<RwLock<Assistant>>, // Wrap Assistant in Arc<RwLock>
    config: AgentConfig,
    is_active: bool,
    // Channel to send messages from the agent's internal processing to the UI
    message_sender: mpsc::Sender<AgentMessage>,
    message_receiver: mpsc::Receiver<AgentMessage>,
    // State for interactive workflows
    active_workflow_prompt_tx: HashMap<String, mpsc::Sender<String>>, // prompt_id -> sender for user response
}

impl AgentMode {
    pub fn new(config: AgentConfig, assistant: Arc<RwLock<Assistant>>) -> Result<Self> {
        let (tx, rx) = mpsc::channel(100); // Channel for agent messages to UI
        Ok(Self {
            assistant,
            config,
            is_active: false,
            message_sender: tx,
            message_receiver: rx,
            active_workflow_prompt_tx: HashMap::new(),
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

    pub async fn send_message(&mut self, user_input: String, context_blocks: Vec<Block>) -> Result<mpsc::Receiver<AgentMessage>> {
        let sender_clone = self.message_sender.clone();
        let assistant_arc_clone = self.assistant.clone(); // Clone the Arc for the spawned task

        // Add user message to history
        let mut assistant_lock = assistant_arc_clone.write().await;
        assistant_lock.conversation_history.push(crate::ai::providers::ChatMessage {
            role: "user".to_string(),
            content: Some(user_input.clone()),
            tool_calls: None,
            tool_call_id: None,
        });
        drop(assistant_lock); // Release lock before async operations

        tokio::spawn(async move {
            let mut assistant_lock = assistant_arc_clone.write().await; // Get write lock inside the task

            // Check for workflow inference intent
            let lower_input = user_input.to_lowercase();
            let is_workflow_request = lower_input.contains("workflow") ||
                                      lower_input.contains("automate") ||
                                      lower_input.contains("sequence of steps") ||
                                      lower_input.contains("multi-step task");

            if is_workflow_request {
                match assistant_lock.infer_workflow(&user_input).await {
                    Ok(workflow) => {
                        log::info!("AI inferred workflow: {}", workflow.name);
                        let _ = sender_clone.send(AgentMessage::WorkflowSuggested(workflow)).await;
                    },
                    Err(e) => {
                        log::error!("Failed to infer workflow: {}", e);
                        let _ = sender_clone.send(AgentMessage::Error(format!("Failed to infer workflow: {}", e))).await;
                    }
                }
            } else {
                // Existing general chat logic
                let stream_result = assistant_lock.stream_chat(&user_input).await;
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
                    },
                    Err(e) => {
                        let _ = sender_clone.send(AgentMessage::Error(format!("AI stream error: {}", e))).await;
                    }
                }
            }
            let _ = sender_clone.send(AgentMessage::Done).await;
        });

        Ok(self.message_receiver.clone()) // Return a clone of the receiver for the UI to subscribe
    }

    // New method for command generation
    pub async fn generate_command(&mut self, natural_language_query: &str) -> Result<String> {
        let mut assistant_lock = self.assistant.write().await; // Get write lock
        assistant_lock.generate_command(natural_language_query).await
    }

    // New method for explaining output
    pub async fn explain_output(&mut self, command_input: &str, output: &str, error_message: Option<&str>) -> Result<String> {
        let mut assistant_lock = self.assistant.write().await;
        assistant_lock.explain_output(command_input, output, error_message).await
    }

    /// Handles a user's response to an agent prompt during workflow execution.
    pub async fn handle_agent_prompt_response(&mut self, prompt_id: String, response: String) -> Result<()> {
        if let Some(tx) = self.active_workflow_prompt_tx.remove(&prompt_id) {
            tx.send(response).await
                .map_err(|e| anyhow!("Failed to send response to workflow executor: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("No active prompt found for ID: {}", prompt_id))
        }
    }

    /// Requests user input for an agent prompt step in a workflow.
    /// Returns a receiver that will get the user's response.
    pub async fn request_agent_prompt_input(&mut self, prompt_id: String, message: String) -> Result<mpsc::Receiver<String>> {
        let (tx, rx) = mpsc::channel(1); // Channel for this specific prompt response
        self.active_workflow_prompt_tx.insert(prompt_id.clone(), tx);
        
        // Send the prompt request to the UI
        self.message_sender.send(AgentMessage::AgentPromptRequest {
            prompt_id,
            message,
        }).await?;

        Ok(rx)
    }
}

impl AiConfig {
    pub fn get_system_prompt(&self) -> Option<String> {
        match self {
            AiConfig::OpenAI { .. } => Some("You are a helpful terminal assistant. Provide concise answers and command suggestions. When suggesting commands, use the `execute_command` tool. When asked to read or write files, use `read_file` or `write_file` tools. When asked about system information, use `get_system_info`.".to_string()),
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
    use crate::ai::providers::AiConfig;
    use crate::ai::assistant::Assistant;

    #[tokio::test]
    async fn test_agent_mode_creation() {
        let ai_config = AiConfig::OpenAI {
            api_key: Some("dummy_key".to_string()),
            model: "gpt-4o".to_string(),
        };
        let assistant = Arc::new(RwLock::new(Assistant::new("openai", ai_config.api_key().cloned(), ai_config.model().to_string()).unwrap()));
        let config = AgentConfig::default();
        let agent = AgentMode::new(config, assistant).unwrap();
        assert_eq!(agent.is_active(), false);
    }

    #[tokio::test]
    async fn test_conversation_lifecycle() {
        let ai_config = AiConfig::OpenAI {
            api_key: Some("dummy_key".to_string()),
            model: "gpt-4o".to_string(),
        };
        let assistant = Arc::new(RwLock::new(Assistant::new("openai", ai_config.api_key().cloned(), ai_config.model().to_string()).unwrap()));
        let config = AgentConfig::default();
        let mut agent = AgentMode::new(config, assistant).unwrap();
        
        agent.toggle();
        assert_eq!(agent.is_active(), true);
        
        agent.start_conversation().await.unwrap();
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
