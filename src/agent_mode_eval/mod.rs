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
    assistant: Assistant,
    is_active: bool,
    conversation: Conversation,
    // Add other state relevant to agent mode, e.g., tool definitions
}

impl AgentMode {
    pub fn new(config: AgentConfig) -> Result<Self> {
        let assistant = Assistant::new(
            &config.provider_type,
            config.api_key,
            config.model,
        )?;
        Ok(Self {
            assistant,
            is_active: false,
            conversation: Conversation::new(),
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
        self.assistant.clear_history();
        self.conversation.clear();
        // Optionally send an initial system message to the assistant
        // self.assistant.stream_chat("Hello! How can I help you today?").await?;
        Ok(())
    }

    pub async fn send_message(&mut self, user_input: String, context_blocks: Vec<Block>) -> Result<mpsc::Receiver<AgentMessage>> {
        if !self.is_active {
            return Err(anyhow!("Agent mode is not active."));
        }

        // Add user message to internal conversation history
        self.conversation.add_message(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: user_input.clone(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });

        // Prepare context for the AI assistant
        let mut full_user_message = user_input;
        if !context_blocks.is_empty() {
            full_user_message.push_str("\n\n--- Context from Blocks ---\n");
            for block in context_blocks {
                full_user_message.push_str(&format!("Block Type: {:?}\n", block.block_type));
                full_user_message.push_str(&format!("Content:\n{}\n", match block.content {
                    BlockContent::Command { input, output, .. } => format!("Command: {}\nOutput:\n{}", input, output.iter().map(|(s, _)| s.clone()).collect::<Vec<String>>().join("\n")),
                    BlockContent::AgentMessage { content, .. } => content,
                    BlockContent::Info { message, .. } => message,
                    BlockContent::Error { message, .. } => message,
                    BlockContent::Welcome => "Welcome message".to_string(),
                    BlockContent::Terminal => "Terminal block".to_string(),
                    BlockContent::BenchmarkResults => "Benchmark results".to_string(),
                    BlockContent::Output { output, .. } => output.iter().map(|(s, _)| s.clone()).collect::<Vec<String>>().join("\n"),
                }));
                full_user_message.push_str("---\n");
            }
        }

        // Stream response from the AI assistant
        let mut stream_rx = self.assistant.stream_chat(&full_user_message).await?;
        let (tx, rx) = mpsc::channel(100);

        let conversation_arc = Arc::new(RwLock::new(self.conversation.clone())); // Clone for async task

        tokio::spawn(async move {
            let mut assistant_response_content = String::new();
            let mut tool_calls_buffer: Vec<ProviderChatMessage> = Vec::new(); // Use the new ToolCall type

            while let Some(chat_msg) = stream_rx.recv().await {
                if !chat_msg.content.is_empty() {
                    assistant_response_content.push_str(&chat_msg.content);
                    if tx.send(AgentMessage::AgentResponse(chat_msg.content)).await.is_err() {
                        break;
                    }
                }
                if let Some(tcs) = chat_msg.tool_calls {
                    tool_calls_buffer.extend(tcs);
                    // For simplicity, send tool calls as soon as they appear.
                    // A more robust implementation might wait for full tool call arguments.
                    for tc in tool_calls_buffer.drain(..) {
                        if tx.send(AgentMessage::ToolCall(tc.into())).await.is_err() { // Convert to AgentToolCall
                            break;
                        }
                    }
                }
            }

            // Add the full assistant response to conversation history
            if !assistant_response_content.is_empty() {
                let mut conv = conversation_arc.write().await;
                conv.add_message(Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::Assistant,
                    content: assistant_response_content,
                    timestamp: chrono::Utc::now(),
                    tool_calls: None, // Tool calls are handled separately
                    tool_results: None,
                });
            }
            
            let _ = tx.send(AgentMessage::Done).await;
        });

        Ok(rx)
    }

    pub async fn get_conversation_history(&self) -> Vec<Message> {
        self.conversation.get_messages().await
    }

    pub async fn reset_conversation(&mut self) {
        self.assistant.clear_history();
        self.conversation.clear();
    }
}

// Helper to convert ai::providers::ToolCall to agent_mode_eval::tools::ToolCall
impl From<ProviderChatMessage> for AgentToolCall {
    fn from(val: ProviderChatMessage) -> Self {
        AgentToolCall {
            id: val.id,
            name: val.function.name,
            arguments: val.function.arguments,
        }
    }
}

pub struct AgentModeEvaluator {
    ai_client: Arc<dyn AIClient>,
    tool_manager: Arc<Mutex<ToolManager>>,
    current_conversation: Arc<Mutex<Conversation>>,
    system_prompt: String,
}

impl AgentModeEvaluator {
    pub fn new(ai_config: AiConfig, initial_system_prompt: String) -> Self {
        let ai_client = Arc::new(OpenAIClient::new(ai_config));
        let tool_manager = Arc::new(Mutex::new(ToolManager::new()));
        let conversation_id = uuid::Uuid::new_v4().to_string();
        let current_conversation = Arc::new(Mutex::new(Conversation::new(conversation_id)));

        // Add initial system message to conversation
        let mut conv = current_conversation.blocking_lock();
        conv.add_message(Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: initial_system_prompt.clone(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
        drop(conv); // Release the lock

        AgentModeEvaluator {
            ai_client,
            tool_manager,
            current_conversation,
            system_prompt: initial_system_prompt,
        }
    }

    pub async fn init(&self) -> Result<()> {
        // Initialize tools if necessary
        let mut tool_manager = self.tool_manager.lock().await;
        tool_manager.register_default_tools().await?;
        Ok(())
    }

    pub async fn handle_user_input(&self, input: String) -> Result<Vec<Message>> {
        let mut conversation = self.current_conversation.lock().await;
        conversation.add_message(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: input.clone(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });

        let mut response_messages = Vec::new();
        let mut current_messages = conversation.get_messages().to_vec();
        let tool_manager = self.tool_manager.lock().await;
        let available_tools = tool_manager.get_all_tools_schema();
        drop(tool_manager); // Release lock on tool_manager

        loop {
            log::info!("Sending messages to AI: {:?}", current_messages);
            let ai_response = self.ai_client.chat_completion(current_messages.clone(), Some(available_tools.clone())).await?;
            log::info!("Received AI response: {:?}", ai_response);

            conversation.add_message(ai_response.clone());
            response_messages.push(ai_response.clone());

            if let Some(tool_calls) = ai_response.tool_calls {
                let tool_manager = self.tool_manager.lock().await;
                let tool_outputs = conversation.execute_tool_calls(tool_calls, &tool_manager).await?;
                drop(tool_manager); // Release lock on tool_manager

                for output_msg in tool_outputs {
                    log::info!("Adding tool output to conversation: {:?}", output_msg);
                    conversation.add_message(output_msg.clone());
                    current_messages.push(output_msg); // Add tool output to messages for next AI call
                }
                current_messages.push(ai_response); // Add AI's tool_call message to messages for next AI call
            } else {
                // If no tool calls, and content is present, the conversation can end.
                // If content is empty, it might be waiting for tool outputs.
                if !ai_response.content.is_empty() {
                    break;
                }
            }
        }

        Ok(response_messages)
    }

    pub async fn get_conversation_history(&self) -> Vec<Message> {
        self.current_conversation.lock().await.get_messages().to_vec()
    }

    pub async fn reset_conversation(&self) {
        let mut conversation = self.current_conversation.lock().await;
        let conversation_id = uuid::Uuid::new_v4().to_string();
        *conversation = Conversation::new(conversation_id);
        // Re-add the system prompt after reset
        conversation.add_message(Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: self.system_prompt.clone(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
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

    #[tokio::test]
    async fn test_agent_mode_creation() {
        let config = AgentConfig::default();
        let agent = AgentMode::new(config).unwrap();
        assert_eq!(agent.is_active(), false);
    }

    #[tokio::test]
    async fn test_conversation_lifecycle() {
        let config = AgentConfig::default();
        let mut agent = AgentMode::new(config).unwrap();
        
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
