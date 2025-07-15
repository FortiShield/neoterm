use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

pub mod ai_client;
pub mod conversation;
pub mod tools;

pub use ai_client::{AiClient, AiConfig, AiProvider, Message, MessageRole};
pub use conversation::{Conversation, ConversationManager};
pub use tools::{Tool, ToolManager, ToolResult as ToolExecutionResult};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub ai_config: AiConfig,
    pub tools_enabled: bool,
    pub context_window: usize,
    pub auto_execute_commands: bool,
    pub working_directory: std::path::PathBuf,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ai_config: AiConfig::default(),
            tools_enabled: true,
            context_window: 8192,
            auto_execute_commands: false,
            working_directory: std::env::current_dir().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentMessage {
    UserInput(String),
    AssistantResponse(String),
    ToolCall(String, serde_json::Value),
    ToolResult(String, String),
    Error(String),
    SystemMessage(String),
}

pub struct AgentMode {
    config: AgentConfig,
    ai_client: AiClient,
    conversation_manager: ConversationManager,
    tool_manager: ToolManager,
    current_conversation: Option<Uuid>,
    enabled: bool,
}

impl Clone for AgentMode {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            ai_client: AiClient::new(self.config.ai_config.clone()).unwrap(),
            conversation_manager: self.conversation_manager.clone(),
            tool_manager: self.tool_manager.clone(),
            current_conversation: self.current_conversation,
            enabled: self.enabled,
        }
    }
}

impl AgentMode {
    pub fn new(config: AgentConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let ai_client = AiClient::new(config.ai_config.clone())?;
        let conversation_manager = ConversationManager::new();
        let tool_manager = ToolManager::new(config.working_directory.clone());

        Ok(Self {
            config,
            ai_client,
            conversation_manager,
            tool_manager,
            current_conversation: None,
            enabled: false,
        })
    }

    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.enabled
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn start_conversation(&mut self) -> Result<Uuid, Box<dyn std::error::Error>> {
        let conversation_id = self.conversation_manager.create_conversation(
            "Terminal Session".to_string(),
            Some("AI-assisted terminal session".to_string()),
        )?;
        
        self.current_conversation = Some(conversation_id);
        
        if let Some(system_prompt) = &self.config.ai_config.system_prompt {
            self.conversation_manager.add_message(
                conversation_id,
                Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::System,
                    content: system_prompt.clone(),
                    timestamp: chrono::Utc::now(),
                    tool_calls: None,
                    tool_results: None,
                },
            )?;
        }

        Ok(conversation_id)
    }

    pub async fn send_message(&self, content: String) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>> {
        let conversation_id = self.current_conversation
            .ok_or("No active conversation")?;

        let user_message = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: content.clone(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        };

        self.conversation_manager.add_message(conversation_id, user_message)?;

        let messages = self.conversation_manager.get_messages(conversation_id)?;
        
        let tools = if self.config.tools_enabled {
            Some(self.tool_manager.get_available_tools())
        } else {
            None
        };

        let response_stream = self.ai_client.send_message(messages, tools).await?;

        Ok(response_stream)
    }

    pub async fn execute_tool_call(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolExecutionResult, Box<dyn std::error::Error>> {
        self.tool_manager.execute_tool(tool_name, arguments).await
    }

    pub fn get_conversation_history(&self) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        if let Some(conversation_id) = self.current_conversation {
            self.conversation_manager.get_messages(conversation_id)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_available_models(&self) -> Vec<String> {
        self.ai_client.get_available_models(&self.config.ai_config.provider)
    }

    pub fn switch_model(&mut self, model: String) -> Result<(), Box<dyn std::error::Error>> {
        if !self.ai_client.is_model_supported(&self.config.ai_config.provider, &model) {
            return Err(format!("Model {} not supported for provider {:?}", model, self.config.ai_config.provider).into());
        }

        self.config.ai_config.model = model;
        self.ai_client = AiClient::new(self.config.ai_config.clone())?;
        Ok(())
    }

    pub fn switch_provider(&mut self, provider: AiProvider, model: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut new_config = self.config.ai_config.clone();
        new_config.provider = provider;
        new_config.model = model;

        let new_client = AiClient::new(new_config.clone())?;
        
        if !new_client.is_model_supported(&provider, &new_config.model) {
            return Err(format!("Model {} not supported for provider {:?}", new_config.model, provider).into());
        }

        self.config.ai_config = new_config;
        self.ai_client = new_client;
        Ok(())
    }

    pub fn update_config(&mut self, config: AgentConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.ai_client = AiClient::new(config.ai_config.clone())?;
        self.config = config;
        Ok(())
    }

    pub fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn export_conversation(&self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(conversation_id) = self.current_conversation {
            self.conversation_manager.export_conversation(conversation_id)
        } else {
            Err("No active conversation to export".into())
        }
    }

    pub fn import_conversation(&mut self, data: &str) -> Result<Uuid, Box<dyn std::error::Error>> {
        let conversation_id = self.conversation_manager.import_conversation(data)?;
        self.current_conversation = Some(conversation_id);
        Ok(conversation_id)
    }

    pub fn clear_conversation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(conversation_id) = self.current_conversation {
            self.conversation_manager.clear_conversation(conversation_id)?;
        }
        Ok(())
    }

    pub fn get_conversation_stats(&self) -> Result<ConversationStats, Box<dyn std::error::Error>> {
        if let Some(conversation_id) = self.current_conversation {
            let messages = self.conversation_manager.get_messages(conversation_id)?;
            let conversation = self.conversation_manager.get_conversation(conversation_id)?;
            
            Ok(ConversationStats {
                message_count: messages.len(),
                user_messages: messages.iter().filter(|m| matches!(m.role, MessageRole::User)).count(),
                assistant_messages: messages.iter().filter(|m| matches!(m.role, MessageRole::Assistant)).count(),
                tool_calls: messages.iter().filter_map(|m| m.tool_calls.as_ref()).map(|tc| tc.len()).sum(),
                created_at: conversation.created_at,
                last_updated: conversation.updated_at,
            })
        } else {
            Err("No active conversation".into())
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
        
        let conversation_id = agent.start_conversation().unwrap();
        assert_eq!(agent.current_conversation, Some(conversation_id));
        
        let stats = agent.get_conversation_stats().unwrap();
        assert!(stats.message_count >= 0);
        
        agent.clear_conversation().unwrap();
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
