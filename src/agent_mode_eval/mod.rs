use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use async_trait::async_trait;
use crate::agent_mode_eval::ai_client::{AIClient, OpenAIClient, AIClientError};

pub mod ai_client;
pub mod conversation;
pub mod tools;

pub use ai_client::{AIClient, AiConfig, AiProvider, Message, MessageRole};
pub use conversation::{Conversation, ConversationManager};
pub use tools::{Tool, ToolManager, ToolResult as ToolExecutionResult};

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
    ToolCall(tools::ToolCall),
    ToolResult(String),
    SystemMessage(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct AgentMode {
    is_enabled: bool,
    client: Box<dyn AIClient + Send + Sync>,
    conversation_history: VecDeque<AgentMessage>,
    config: AgentConfig,
}

impl AgentMode {
    pub fn new(config: AgentConfig) -> Result<Self, AIClientError> {
        let client: Box<dyn AIClient + Send + Sync> = match &config.ai_config {
            ai_client::AIClientConfig::OpenAI { api_key } => {
                let key = api_key.clone().ok_or(AIClientError::ConfigurationError("OpenAI API key not provided".to_string()))?;
                Box::new(OpenAIClient::new(key))
            }
            // Add other AI clients here
        };

        Ok(Self {
            is_enabled: false,
            client,
            conversation_history: VecDeque::new(),
            config,
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
        // Optionally add a system message to start the conversation
        self.conversation_history.push_back(AgentMessage::SystemMessage("You are a helpful terminal assistant. Provide concise answers and command suggestions.".to_string()));
        Ok(())
    }

    pub async fn send_message(&mut self, message: String) -> Result<mpsc::Receiver<String>, AIClientError> {
        self.conversation_history.push_back(AgentMessage::UserMessage(message.clone()));
        self.trim_history();

        let (tx, rx) = mpsc::channel(100);
        let client_clone = self.client.clone_box();
        let history_clone = self.conversation_history.clone();

        tokio::spawn(async move {
            let messages = history_clone.iter().map(|msg| {
                match msg {
                    AgentMessage::UserMessage(s) => conversation::Message {
                        role: conversation::MessageRole::User,
                        content: s.clone(),
                    },
                    AgentMessage::AgentResponse(s) => conversation::Message {
                        role: conversation::MessageRole::Assistant,
                        content: s.clone(),
                    },
                    AgentMessage::SystemMessage(s) => conversation::Message {
                        role: conversation::MessageRole::System,
                        content: s.clone(),
                    },
                    _ => conversation::Message { // Handle other types as needed, or filter them out
                        role: conversation::MessageRole::System,
                        content: format!("Unhandled message type: {:?}", msg),
                    },
                }
            }).collect();

            match client_clone.stream_text(messages).await {
                Ok(mut stream) => {
                    let mut full_response = String::new();
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                full_response.push_str(&chunk);
                                if let Err(_) = tx.send(chunk).await {
                                    eprintln!("Failed to send chunk to UI");
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error streaming AI response: {}", e);
                                if let Err(_) = tx.send(format!("Error: {}", e)).await {
                                    eprintln!("Failed to send error to UI");
                                }
                                break;
                            }
                        }
                    }
                    // Add the full response to history after streaming is complete
                    // This requires mutable access to self, which is tricky in this async block.
                    // A better pattern would be to send a final message back to the main loop
                    // to update the history. For now, we'll assume the main loop handles history.
                }
                Err(e) => {
                    eprintln!("Failed to get AI stream: {}", e);
                    if let Err(_) = tx.send(format!("Error: {}", e)).await {
                        eprintln!("Failed to send error to UI");
                    }
                }
            }
        });

        Ok(rx)
    }

    fn trim_history(&mut self) {
        while self.conversation_history.len() > self.config.max_history_length {
            self.conversation_history.pop_front();
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
        
        // Assuming send_message and other methods are implemented similarly
        // let response = agent.send_message("Hello".to_string()).await.unwrap();
        // assert!(response.recv().await.is_some());
        
        // let stats = agent.get_conversation_stats().unwrap();
        // assert!(stats.message_count >= 0);
        
        // agent.clear_conversation().unwrap();
    }

    #[test]
    fn test_model_switching() {
        let config = AiConfig {
            provider: AiProvider::OpenAI,
            model: "gpt-4o".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: None,
            temperature: 0.7,
            max_tokens: None,
            system_prompt: None,
            tools_enabled: true,
        };
        let mut agent = AgentMode::new(AgentConfig { ai_config: config, ..Default::default() }).unwrap();
        
        let result = agent.switch_model("gpt-4".to_string());
        assert!(result.is_ok());
        
        let result = agent.switch_model("invalid-model".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_switching() {
        let mut config = AgentConfig::default();
        config.ai_config.api_key = Some("test-key".to_string());
        
        let mut agent = AgentMode::new(config).unwrap();
        
        let result = agent.switch_provider(AiProvider::Claude, "claude-4-sonnet-20250514".to_string());
        assert!(result.is_ok());
        
        assert_eq!(agent.config.ai_config.provider, AiProvider::Claude);
        assert_eq!(agent.config.ai_config.model, "claude-4-sonnet-20250514");
    }
}
