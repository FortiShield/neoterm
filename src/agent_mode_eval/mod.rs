use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

pub mod ai_client;
pub mod conversation;
pub mod tools;

use ai_client::{AiClient, AiProvider, AiResponse, StreamingResponse};
use conversation::{Conversation, Message, MessageRole};
use tools::{ToolRegistry, ToolCall, ToolResult};

#[derive(Debug, Clone)]
pub struct AgentMode {
    pub enabled: bool,
    pub current_conversation: Option<Conversation>,
    pub ai_client: AiClient,
    pub tool_registry: ToolRegistry,
    pub auto_execute: bool,
    pub context_window: usize,
}

#[derive(Debug, Clone)]
pub enum AgentMessage {
    StartConversation,
    SendMessage(String),
    ExecuteCommand(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    StreamingResponse(String),
    ConversationEnded,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub provider: AiProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub system_prompt: String,
    pub tools_enabled: bool,
    pub auto_execute_commands: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::OpenAI,
            model: "gpt-4o".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: Some(4096),
            system_prompt: "You are a helpful AI assistant integrated into a terminal. You can help users with command-line tasks, explain commands, and execute shell commands when requested. Always be concise and practical in your responses.".to_string(),
            tools_enabled: true,
            auto_execute_commands: false,
        }
    }
}

impl AgentConfig {
    pub fn get_available_models(provider: &AiProvider) -> Vec<&'static str> {
        match provider {
            AiProvider::OpenAI => vec![
                "gpt-4o", "gpt-4", "gpt-4-turbo", "gpt-4-mini", 
                "gpt-3.5-turbo", "gpt-3o", "o3", "o3-mini"
            ],
            AiProvider::Claude => vec![
                "claude-4-sonnet-20250514", "claude-4-opus-20250514",
                "claude-3-7-sonnet-20241022", "claude-3-5-sonnet-20241022", 
                "claude-3-7-haiku-20241022"
            ],
            AiProvider::Gemini => vec![
                "gemini-2.0-flash-exp", "gemini-2.0-pro-exp",
                "gemini-1.5-pro", "gemini-1.5-flash"
            ],
            AiProvider::Ollama => vec![
                "llama3.2", "llama3.1", "codellama", "mistral", 
                "phi3", "qwen2.5", "deepseek-coder"
            ],
            AiProvider::Groq => vec![
                "llama-3.1-70b-versatile", "llama-3.1-8b-instant",
                "mixtral-8x7b-32768", "gemma2-9b-it"
            ],
            AiProvider::Local => vec!["custom-model"],
        }
    }

    pub fn get_default_model(provider: &AiProvider) -> &'static str {
        match provider {
            AiProvider::OpenAI => "gpt-4o",
            AiProvider::Claude => "claude-4-sonnet-20250514",
            AiProvider::Gemini => "gemini-2.0-flash-exp",
            AiProvider::Ollama => "llama3.2",
            AiProvider::Groq => "llama-3.1-70b-versatile",
            AiProvider::Local => "custom-model",
        }
    }

    pub fn get_default_base_url(provider: &AiProvider) -> Option<&'static str> {
        match provider {
            AiProvider::OpenAI => Some("https://api.openai.com/v1/chat/completions"),
            AiProvider::Claude => Some("https://api.anthropic.com/v1/messages"),
            AiProvider::Gemini => Some("https://generativelanguage.googleapis.com/"),
            AiProvider::Ollama => Some("http://localhost:11434"),
            AiProvider::Groq => Some("https://api.groq.com/openai/v1/chat/completions"),
            AiProvider::Local => Some("http://localhost:8080"),
        }
    }
}

impl AgentMode {
    pub fn new(config: AgentConfig) -> Result<Self, AgentError> {
        let ai_client = AiClient::new(config.clone())?;
        let tool_registry = ToolRegistry::new();
        
        Ok(Self {
            enabled: false,
            current_conversation: None,
            ai_client,
            tool_registry,
            auto_execute: config.auto_execute_commands,
            context_window: 8192,
        })
    }

    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        if !self.enabled {
            self.current_conversation = None;
        }
        self.enabled
    }

    pub fn start_conversation(&mut self) -> Result<Uuid, AgentError> {
        let conversation = Conversation::new(self.ai_client.config.system_prompt.clone());
        let id = conversation.id;
        self.current_conversation = Some(conversation);
        Ok(id)
    }

    pub async fn send_message(&mut self, content: String) -> Result<mpsc::Receiver<String>, AgentError> {
        let conversation = self.current_conversation
            .as_mut()
            .ok_or(AgentError::NoActiveConversation)?;

        // Add user message to conversation
        conversation.add_message(Message {
            role: MessageRole::User,
            content: content.clone(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
        });

        // Prepare messages for AI
        let messages = self.prepare_messages_for_ai(conversation)?;
        
        // Get streaming response
        let (tx, rx) = mpsc::channel(100);
        let ai_client = self.ai_client.clone();
        let tools = if self.ai_client.config.tools_enabled {
            Some(self.tool_registry.get_available_tools())
        } else {
            None
        };

        tokio::spawn(async move {
            match ai_client.stream_completion(messages, tools).await {
                Ok(mut stream) => {
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(response) => {
                                if let Err(_) = tx.send(response.content).await {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!("Error: {}", e)).await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to get AI response: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    pub async fn execute_tool_call(&mut self, tool_call: ToolCall) -> Result<ToolResult, AgentError> {
        self.tool_registry.execute_tool(tool_call).await
            .map_err(AgentError::ToolError)
    }

    fn prepare_messages_for_ai(&self, conversation: &Conversation) -> Result<Vec<ai_client::AiMessage>, AgentError> {
        let mut messages = Vec::new();
        
        // Add system message
        messages.push(ai_client::AiMessage {
            role: "system".to_string(),
            content: conversation.system_prompt.clone(),
            tool_calls: None,
        });

        // Add conversation messages (with context window limit)
        let recent_messages = if conversation.messages.len() > self.context_window {
            &conversation.messages[conversation.messages.len() - self.context_window..]
        } else {
            &conversation.messages
        };

        for msg in recent_messages {
            messages.push(ai_client::AiMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                },
                content: msg.content.clone(),
                tool_calls: msg.tool_calls.clone(),
            });
        }

        Ok(messages)
    }

    pub fn get_conversation_history(&self) -> Option<&Conversation> {
        self.current_conversation.as_ref()
    }

    pub fn clear_conversation(&mut self) {
        self.current_conversation = None;
    }

    pub fn update_config(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        self.ai_client = AiClient::new(config)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("No active conversation")]
    NoActiveConversation,
    #[error("AI client error: {0}")]
    AiClientError(#[from] ai_client::AiClientError),
    #[error("Tool error: {0}")]
    ToolError(#[from] tools::ToolError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub fn init() {
    println!("Agent mode evaluation system initialized");
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
        
        // Start conversation
        let conv_id = agent.start_conversation().unwrap();
        assert!(agent.current_conversation.is_some());
        
        // Clear conversation
        agent.clear_conversation();
        assert!(agent.current_conversation.is_none());
    }
}
