use crate::ai::providers::{AIProvider, ChatMessage, OpenAIProvider, OllamaProvider, AnthropicProvider};
use crate::ai::prompts::PromptBuilder;
use crate::ai::context::AIContext;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use serde_json::Value;

pub struct Assistant {
    provider: Box<dyn AIProvider + Send + Sync>,
    prompt_builder: PromptBuilder,
    context: AIContext,
    conversation_history: Vec<ChatMessage>,
}

impl Assistant {
    pub fn new(provider_type: &str, api_key: Option<String>, model: String) -> Result<Self> {
        let provider: Box<dyn AIProvider + Send + Sync> = match provider_type {
            "openai" => Box::new(OpenAIProvider::new(api_key, model)?),
            "ollama" => Box::new(OllamaProvider::new(model)?),
            "anthropic" => Box::new(AnthropicProvider::new(api_key, model)?),
            _ => return Err(anyhow!("Unsupported AI provider: {}", provider_type)),
        };

        Ok(Self {
            provider,
            prompt_builder: PromptBuilder::new(),
            context: AIContext::new(),
            conversation_history: Vec::new(),
        })
    }

    pub async fn suggest(&mut self, user_query: &str) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_suggestion_prompt(&self.context);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: user_query.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(user_message);

        let response = self.provider.chat_completion(messages, None).await?;
        self.conversation_history.push(response.clone());
        Ok(response.content)
    }

    pub async fn fix(&mut self, code_snippet: &str, error_message: &str) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_fix_prompt(&self.context, code_snippet, error_message);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: format!("Please fix the following code:\n```\n{}\n```\nError: {}", code_snippet, error_message),
            tool_calls: None,
            tool_call_id: None,
        };

        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(user_message);

        let response = self.provider.chat_completion(messages, None).await?;
        self.conversation_history.push(response.clone());
        Ok(response.content)
    }

    pub async fn explain(&mut self, text_to_explain: &str) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_explanation_prompt(&self.context);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: format!("Explain the following:\n{}", text_to_explain),
            tool_calls: None,
            tool_call_id: None,
        };

        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(user_message);

        let response = self.provider.chat_completion(messages, None).await?;
        self.conversation_history.push(response.clone());
        Ok(response.content)
    }

    pub async fn stream_chat(&mut self, user_message: &str) -> Result<mpsc::Receiver<ChatMessage>> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_general_chat_prompt(&self.context);
        
        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });

        self.provider.stream_chat_completion(messages, None).await
    }

    pub fn get_history(&self) -> &Vec<ChatMessage> {
        &self.conversation_history
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }
}
