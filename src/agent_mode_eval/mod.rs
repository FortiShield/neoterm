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

pub mod ai_client;
pub mod conversation;
pub mod tools;

pub use ai_client::{AIClient, AiConfig, AiProvider, Message as AiClientMessage, MessageRole as AiClientMessageRole};
pub use conversation::{Conversation, ConversationManager};
pub use tools::{Tool, ToolManager, ToolResult as ToolExecutionResult, ToolCall as AgentToolCall};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub ai_config: AiConfig,
    pub max_history_length: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ai_config: AiConfig::OpenAI { api_key: None },
            max_history_length: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentMessage {
    Text(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    Error(String),
    Done,
}

pub struct AgentMode {
    config: AiConfig,
    ai_client: AIClient,
    tool_registry: ToolRegistry,
    current_conversation: Conversation,
    command_executor: CommandExecutor,
    // Channel to send messages back to the main application loop
    message_sender: mpsc::UnboundedSender<AgentMessage>,
}

impl AgentMode {
    pub fn new(config: AiConfig, command_executor: CommandExecutor, message_sender: mpsc::UnboundedSender<AgentMessage>) -> Result<Self, Box<dyn std::error::Error>> {
        let ai_client = AIClient::new(config.clone())?;
        let tool_registry = ToolRegistry::new();

        // Register default tools
        tool_registry.register_tool(Tool::new(
            "read_file".to_string(),
            "Reads the content of a file from the file system.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read."
                    }
                },
                "required": ["path"]
            }),
            |args| {
                Box::pin(async move {
                    let path = args["path"].as_str().ok_or("Missing 'path' argument")?.to_string();
                    tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| format!("Failed to read file {}: {}", path, e))
                })
            },
        ))?;

        tool_registry.register_tool(Tool::new(
            "list_dir".to_string(),
            "Lists the contents of a directory.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the directory to list."
                    }
                },
                "required": ["path"]
            }),
            |args| {
                Box::pin(async move {
                    let path = args["path"].as_str().ok_or("Missing 'path' argument")?.to_string();
                    let mut entries = Vec::new();
                    let mut dir = tokio::fs::read_dir(&path).await
                        .map_err(|e| format!("Failed to read directory {}: {}", path, e))?;
                    
                    while let Some(entry) = dir.next_entry().await.map_err(|e| format!("Failed to read directory entry: {}", e))? {
                        entries.push(entry.file_name().to_string_lossy().into_owned());
                    }
                    Ok(entries.join("\n"))
                })
            },
        ))?;

        // Add a tool for executing shell commands
        let cmd_executor_clone = command_executor.clone();
        tool_registry.register_tool(Tool::new(
            "execute_command".to_string(),
            "Executes a shell command and returns its output.".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Arguments for the command."
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Optional working directory for the command."
                    }
                },
                "required": ["command"]
            }),
            move |args| {
                let executor_clone = cmd_executor_clone.clone();
                Box::pin(async move {
                    let command = args["command"].as_str().ok_or("Missing 'command' argument")?.to_string();
                    let args_vec: Vec<String> = args["args"].as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    let working_directory = args["working_directory"].as_str().map(String::from);

                    let (tx, mut rx) = mpsc::unbounded_channel();
                    let _ = executor_clone.execute_command(&command, &args_vec, working_directory, tx);

                    let mut output = String::new();
                    while let Some(msg) = rx.recv().await {
                        output.push_str(&msg);
                    }
                    Ok(output)
                })
            },
        ))?;


        Ok(Self {
            config,
            ai_client,
            tool_registry,
            current_conversation: Conversation::new("New AI Conversation".to_string()),
            command_executor,
            message_sender,
        })
    }

    pub fn update_config(&mut self, config: AiConfig) {
        self.config = config.clone();
        self.ai_client.update_config(config);
    }

    pub fn get_config(&self) -> &AiConfig {
        self.ai_client.get_config()
    }

    pub fn get_conversation(&self) -> &Conversation {
        &self.current_conversation
    }

    pub fn start_new_conversation(&mut self, title: String) {
        self.current_conversation = Conversation::new(title);
    }

    pub async fn send_message(&mut self, user_input: String, context_blocks: Vec<Block>) {
        let mut messages = self.current_conversation.messages.clone();

        // Add system prompt if configured and not already present
        if let Some(system_prompt) = &self.config.system_prompt {
            if !messages.iter().any(|m| m.role == MessageRole::System) {
                messages.insert(0, Message::new(MessageRole::System, system_prompt.clone()));
            }
        }

        // Add contextual blocks to the user message
        let mut full_user_content = user_input;
        if !context_blocks.is_empty() {
            full_user_content.push_str("\n\n--- Contextual Information ---\n");
            for block in context_blocks {
                full_user_content.push_str(&format!("\nBlock ID: {}\nType: {:?}\nContent:\n\`\`\`\n{}\n\`\`\`\n",
                    block.id, block.block_type, block.content.to_string()));
            }
            full_user_content.push_str("----------------------------\n");
        }

        messages.push(Message::new(MessageRole::User, full_user_content));
        self.current_conversation.add_message(messages.last().unwrap().clone()); // Add user message to conversation history

        let tools = if self.config.tools_enabled {
            Some(self.tool_registry.get_all_tools())
        } else {
            None
        };

        let mut assistant_response_text = String::new();
        let mut tool_calls_to_execute: Vec<ToolCall> = Vec::new();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops

        loop {
            iteration_count += 1;
            if iteration_count > MAX_ITERATIONS {
                let _ = self.message_sender.send(AgentMessage::Error("Agent mode reached max iterations without a final response.".to_string()));
                break;
            }

            let ai_stream_result = self.ai_client.stream_response(messages.clone(), tools.clone()).await;

            match ai_stream_result {
                Ok(mut stream) => {
                    assistant_response_text.clear();
                    tool_calls_to_execute.clear();
                    let mut received_done = false;

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                match chunk {
                                    AIStreamChunk::Text(text) => {
                                        assistant_response_text.push_str(&text);
                                        let _ = self.message_sender.send(AgentMessage::Text(text));
                                    },
                                    AIStreamChunk::ToolCall(calls) => {
                                        for call in calls {
                                            tool_calls_to_execute.push(call.clone());
                                            let _ = self.message_sender.send(AgentMessage::ToolCall(call));
                                        }
                                    },
                                    AIStreamChunk::Done => {
                                        received_done = true;
                                        break; // End of AI response for this turn
                                    },
                                }
                            }
                            Err(e) => {
                                let _ = self.message_sender.send(AgentMessage::Error(format!("AI Stream Error: {}", e)));
                                received_done = true; // Treat stream error as end of response
                                break;
                            }
                        }
                    }

                    // After receiving all chunks for this turn
                    if !assistant_response_text.is_empty() {
                        self.current_conversation.add_message(Message::new(MessageRole::Assistant, assistant_response_text.clone()));
                    }

                    if !tool_calls_to_execute.is_empty() {
                        // Add the tool calls to the conversation history
                        self.current_conversation.add_message(Message::new_tool_call(tool_calls_to_execute.clone()));

                        // Execute tools and prepare results
                        let mut tool_results_for_next_turn: Vec<Message> = Vec::new();
                        for tool_call in tool_calls_to_execute.drain(..) {
                            let tool_result = self.tool_registry.execute_tool(&tool_call).await;
                            match tool_result {
                                Ok(output) => {
                                    let result_msg = Message::new_tool_result(tool_call.id.clone(), output, false);
                                    let _ = self.message_sender.send(AgentMessage::ToolResult(result_msg.tool_results.as_ref().unwrap()[0].clone()));
                                    tool_results_for_next_turn.push(result_msg);
                                }
                                Err(e) => {
                                    let result_msg = Message::new_tool_result(tool_call.id.clone(), e.to_string(), true);
                                    let _ = self.message_sender.send(AgentMessage::ToolResult(result_msg.tool_results.as_ref().unwrap()[0].clone()));
                                    tool_results_for_next_turn.push(result_msg);
                                }
                            }
                        }
                        // Add tool results to messages for the next AI turn
                        messages.extend(tool_results_for_next_turn);
                        self.current_conversation.messages.extend(tool_results_for_next_turn); // Add to conversation history
                        // Continue loop to send tool results back to AI
                    } else if received_done {
                        // If AI sent text and then Done, or just Done without tool calls, we are finished.
                        let _ = self.message_sender.send(AgentMessage::Done);
                        break;
                    } else {
                        // This case should ideally not be reached if AI sends Done, but as a safeguard
                        let _ = self.message_sender.send(AgentMessage::Error("Unexpected state: AI response ended without Done or tool calls.".to_string()));
                        break;
                    }
                }
                Err(e) => {
                    let _ = self.message_sender.send(AgentMessage::Error(format!("AI Client Error: {}", e)));
                    break;
                }
            }
        }
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
        let config = AiConfig::default();
        let command_executor = CommandExecutor::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = AgentMode::new(config, command_executor, tx);
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_conversation_lifecycle() {
        let config = AiConfig::default();
        let command_executor = CommandExecutor::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut agent = AgentMode::new(config, command_executor, tx).unwrap();
        
        agent.start_new_conversation("Test Conversation".to_string());
        
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
