use crate::ai::providers::{AIProvider, ChatMessage, OpenAIProvider, OllamaProvider, AnthropicProvider};
use crate::ai::prompts::PromptBuilder;
use crate::ai::context::AIContext;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use serde_json::Value;
use crate::workflows::Workflow; // Import Workflow struct

pub struct Assistant {
    provider: Box<dyn AIProvider + Send + Sync>,
    prompt_builder: PromptBuilder,
    context: AIContext,
    pub conversation_history: Vec<ChatMessage>, // Made public for AgentMode to manage
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
            content: Some(user_query.to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(user_message);

        let response = self.provider.chat_completion(messages, None).await?;
        self.conversation_history.push(response.clone());
        Ok(response.content.unwrap_or_default())
    }

    pub async fn fix(&mut self, code_snippet: &str, error_message: &str) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_fix_prompt(&self.context, code_snippet, error_message);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some(format!("Failed command: `{}`\nError: {}", code_snippet, error_message)),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_prompt, user_message]; // No history for fix to keep it focused

        let response = self.provider.chat_completion(messages, None).await?;
        // Do NOT add to conversation history, as this is a specific command generation/fix, not general chat.
        Ok(response.content.unwrap_or_default().trim().to_string()) // Trim to remove any leading/trailing whitespace
    }

    pub async fn explain(&mut self, text_to_explain: &str) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_explanation_prompt(&self.context);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some(format!("Explain the following:\n{}", text_to_explain)),
            tool_calls: None,
            tool_call_id: None,
        };

        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(user_message);

        let response = self.provider.chat_completion(messages, None).await?;
        self.conversation_history.push(response.clone());
        Ok(response.content.unwrap_or_default())
    }

    pub async fn stream_chat(&mut self, user_message: &str) -> Result<mpsc::Receiver<ChatMessage>> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_general_chat_prompt(&self.context);
        
        let mut messages = vec![system_prompt];
        messages.extend(self.conversation_history.clone());
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        self.provider.stream_chat_completion(messages, None).await
    }

    pub async fn generate_command(&mut self, natural_language_query: &str) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_command_generation_prompt(&self.context);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some(natural_language_query.to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_prompt, user_message]; // No history for command generation to keep it focused

        let response = self.provider.chat_completion(messages, None).await?;
        // Do NOT add to conversation history, as this is a specific command generation, not general chat.
        Ok(response.content.unwrap_or_default().trim().to_string()) // Trim to remove any leading/trailing whitespace
    }

    pub async fn explain_output(&mut self, command_input: &str, output: &str, error_message: Option<&str>) -> Result<String> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_explain_output_prompt(&self.context, command_input, output, error_message);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some("Please explain the above command's output/error.".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_prompt, user_message];
        let response = self.provider.chat_completion(messages, None).await?;
        Ok(response.content.unwrap_or_default())
    }

    /// Infers a multi-step workflow from a natural language request.
    pub async fn infer_workflow(&mut self, user_request: &str) -> Result<Workflow> {
        self.context.update_current_state().await?;
        let system_prompt = self.prompt_builder.build_workflow_inference_prompt(&self.context, user_request);
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some(user_request.to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_prompt, user_message];
        let response = self.provider.chat_completion(messages, None).await?;
        
        let yaml_content = response.content.unwrap_or_default();
        log::debug!("AI inferred workflow YAML:\n{}", yaml_content);

        // Attempt to parse the YAML content into a Workflow struct
        let workflow = Workflow::from_yaml(&yaml_content)
            .map_err(|e| anyhow!("Failed to parse AI-inferred workflow YAML: {}", e))?;
        
        Ok(workflow)
    }

    pub fn get_history(&self) -> &Vec<ChatMessage> {
        &self.conversation_history
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }
}
