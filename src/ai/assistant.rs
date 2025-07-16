use crate::ai::providers::{AIProvider, ChatMessage, OpenAIProvider, OllamaProvider, AnthropicProvider};
use crate::ai::prompts::PromptBuilder;
use crate::ai::context::AIContext;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use serde_json::Value;
use crate::workflows::Workflow; // Import Workflow struct
use regex::Regex; // For redaction

pub struct Assistant {
    primary_provider: Box<dyn AIProvider + Send + Sync>,
    fallback_provider: Option<Box<dyn AIProvider + Send + Sync>>,
    prompt_builder: PromptBuilder,
    context: AIContext,
    pub conversation_history: Vec<ChatMessage>, // Made public for AgentMode to manage
    redact_sensitive_info: bool, // New: Control redaction
    local_only_ai_mode: bool, // New: Force local AI models
}

impl Assistant {
    pub fn new(
        command_manager: Arc<CommandManager>, // Added for AIContext
        virtual_file_system: Arc<VirtualFileSystem>, // Added for AIContext
        watcher: Arc<Watcher>, // Added for AIContext
        primary_provider_type: &str,
        primary_api_key: Option<String>,
        primary_model: String,
        fallback_provider_type: Option<String>,
        fallback_api_key: Option<String>, // Fallback API key might be different
        fallback_model: Option<String>,
        redact_sensitive_info: bool, // New
        local_only_ai_mode: bool, // New
    ) -> Result<Self> {
        let primary_provider: Box<dyn AIProvider + Send + Sync> = match primary_provider_type {
            "openai" => Box::new(OpenAIProvider::new(primary_api_key, primary_model)?),
            "ollama" => Box::new(OllamaProvider::new(primary_model)?),
            "anthropic" => Box::new(AnthropicProvider::new(primary_api_key, primary_model)?),
            _ => return Err(anyhow!("Unsupported primary AI provider: {}", primary_provider_type)),
        };

        let fallback_provider = if let Some(fb_type) = fallback_provider_type {
            if let Some(fb_model) = fallback_model {
                match fb_type.as_str() {
                    "openai" => Some(Box::new(OpenAIProvider::new(fallback_api_key, fb_model)?)),
                    "ollama" => Some(Box::new(OllamaProvider::new(fb_model)?)),
                    "anthropic" => Some(Box::new(AnthropicProvider::new(fallback_api_key, fb_model)?)),
                    _ => {
                        log::warn!("Unsupported fallback AI provider: {}. Fallback will not be used.", fb_type);
                        None
                    }
                }
            } else {
                log::warn!("Fallback AI provider type specified but no model. Fallback will not be used.");
                None
            }
        } else {
            None
        };

        Ok(Self {
            primary_provider,
            fallback_provider,
            prompt_builder: PromptBuilder::new(),
            context: AIContext::new(command_manager, virtual_file_system, watcher, redact_sensitive_info), // Pass redact_sensitive_info
            conversation_history: Vec::new(),
            redact_sensitive_info, // Store preference
            local_only_ai_mode, // Store preference
        })
    }

    // Helper to redact sensitive information from a ChatMessage
    fn redact_chat_message(&self, mut message: ChatMessage) -> ChatMessage {
        if !self.redact_sensitive_info {
            return message;
        }

        if let Some(content) = message.content.as_mut() {
            // Regex for common API keys/tokens (simplified, can be expanded)
            let api_key_regex = Regex::new(r"(sk-[a-zA-Z0-9]{32,}|[A-Za-z0-9]{32,}_[A-Za-z0-9]{32,})").unwrap();
            *content = api_key_regex.replace_all(content, "[REDACTED_API_KEY]").to_string();

            // Regex for common environment variables (e.g., VAR=value)
            let env_var_regex = Regex::new(r"([A-Z_]+)=([a-zA-Z0-9_.-]+)").unwrap();
            *content = env_var_regex.replace_all(content, "$1=[REDACTED_ENV_VAR]").to_string();

            // Redact common sensitive file paths if they appear in content
            let sensitive_path_regex = Regex::new(r"(/home/[^/]+/\.ssh|/root/\.ssh|/etc/secrets|/var/lib/jenkins/secrets|~/\.aws/credentials)").unwrap();
            *content = sensitive_path_regex.replace_all(content, "[REDACTED_PATH]").to_string();
        }
        message
    }

    async fn try_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<ChatMessage> {
        let redacted_messages: Vec<ChatMessage> = messages.into_iter().map(|msg| self.redact_chat_message(msg)).collect();

        // Check local_only_ai_mode for primary provider
        if self.local_only_ai_mode && self.primary_provider.name() != "Ollama" { // Assuming Ollama is the primary local provider
            log::warn!("Local-only AI mode is enabled. Skipping primary cloud provider ({}).", self.primary_provider.name());
            if let Some(fb_provider) = &self.fallback_provider {
                if fb_provider.name() == "Ollama" {
                    log::info!("Attempting chat completion with local fallback provider ({}).", fb_provider.name());
                    return fb_provider.chat_completion(redacted_messages, tools).await;
                } else {
                    return Err(anyhow!("Local-only AI mode is enabled, and neither primary nor fallback is a local provider."));
                }
            } else {
                return Err(anyhow!("Local-only AI mode is enabled, but no local fallback provider is configured."));
            }
        }

        // Try primary provider first
        match self.primary_provider.chat_completion(redacted_messages.clone(), tools.clone()).await {
            Ok(response) => {
                log::debug!("Chat completion successful with primary provider: {}", self.primary_provider.name());
                Ok(response)
            },
            Err(e) => {
                log::warn!("Primary AI provider ({}) failed: {}. Attempting fallback...", self.primary_provider.name(), e);
                if let Some(fb_provider) = &self.fallback_provider {
                    // Check local_only_ai_mode for fallback provider
                    if self.local_only_ai_mode && fb_provider.name() != "Ollama" {
                        return Err(anyhow!("Local-only AI mode is enabled, and fallback provider ({}) is not local.", fb_provider.name()));
                    }
                    match fb_provider.chat_completion(redacted_messages, tools).await {
                        Ok(response) => {
                            log::info!("Chat completion successful with fallback provider: {}", fb_provider.name());
                            Ok(response)
                        },
                        Err(fb_e) => {
                            log::error!("Fallback AI provider ({}) also failed: {}", fb_provider.name(), fb_e);
                            Err(anyhow!("Both primary and fallback AI providers failed. Primary error: {}, Fallback error: {}", e, fb_e))
                        }
                    }
                } else {
                    Err(anyhow!("Primary AI provider ({}) failed and no fallback configured: {}", self.primary_provider.name(), e))
                }
            }
        }
    }

    async fn try_stream_chat_completion(&self, messages: Vec<ChatMessage>, tools: Option<Value>) -> Result<mpsc::Receiver<ChatMessage>> {
        let redacted_messages: Vec<ChatMessage> = messages.into_iter().map(|msg| self.redact_chat_message(msg)).collect();

        // Check local_only_ai_mode for primary provider
        if self.local_only_ai_mode && self.primary_provider.name() != "Ollama" { // Assuming Ollama is the primary local provider
            log::warn!("Local-only AI mode is enabled. Skipping primary cloud provider ({}).", self.primary_provider.name());
            if let Some(fb_provider) = &self.fallback_provider {
                if fb_provider.name() == "Ollama" {
                    log::info!("Attempting stream chat completion with local fallback provider ({}).", fb_provider.name());
                    return fb_provider.stream_chat_completion(redacted_messages, tools).await;
                } else {
                    return Err(anyhow!("Local-only AI mode is enabled, and neither primary nor fallback is a local provider."));
                }
            } else {
                return Err(anyhow!("Local-only AI mode is enabled, but no local fallback provider is configured."));
            }
        }

        // Try primary provider first
        match self.primary_provider.stream_chat_completion(redacted_messages.clone(), tools.clone()).await {
            Ok(receiver) => {
                log::debug!("Stream chat completion successful with primary provider: {}", self.primary_provider.name());
                Ok(receiver)
            },
            Err(e) => {
                log::warn!("Primary AI provider ({}) stream failed: {}. Attempting fallback...", self.primary_provider.name(), e);
                if let Some(fb_provider) = &self.fallback_provider {
                    // Check local_only_ai_mode for fallback provider
                    if self.local_only_ai_mode && fb_provider.name() != "Ollama" {
                        return Err(anyhow!("Local-only AI mode is enabled, and fallback provider ({}) is not local.", fb_provider.name()));
                    }
                    match fb_provider.stream_chat_completion(redacted_messages, tools).await {
                        Ok(receiver) => {
                            log::info!("Stream chat completion successful with fallback provider: {}", fb_provider.name());
                            Ok(receiver)
                        },
                        Err(fb_e) => {
                            log::error!("Fallback AI provider ({}) stream also failed: {}", fb_provider.name(), fb_e);
                            Err(anyhow!("Both primary and fallback AI providers stream failed. Primary error: {}, Fallback error: {}", e, fb_e))
                        }
                    }
                } else {
                    Err(anyhow!("Primary AI provider ({}) stream failed and no fallback configured: {}", self.primary_provider.name(), e))
                }
            }
        }
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

        let response = self.try_chat_completion(messages, None).await?;
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

        let response = self.try_chat_completion(messages, None).await?;
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

        let response = self.try_chat_completion(messages, None).await?;
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

        self.try_stream_chat_completion(messages, None).await
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

        let response = self.try_chat_completion(messages, None).await?;
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
        let response = self.try_chat_completion(messages, None).await?;
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
        let response = self.try_chat_completion(messages, None).await?;
        
        let yaml_content = response.content.unwrap_or_default();
        log::debug!("AI inferred workflow YAML:\n{}", yaml_content);

        // Attempt to parse the YAML content into a Workflow struct
        let workflow = Workflow::from_yaml(&yaml_content)
            .map_err(|e| anyhow!("Failed to parse AI-inferred workflow YAML: {}", e))?;
        
        Ok(workflow)
    }

    pub async fn get_usage_quota(&self) -> Result<String> {
        if self.primary_provider.name() == "OpenAI" {
            let openai_provider = self.primary_provider.as_any().downcast_ref::<OpenAIProvider>()
                .ok_or_else(|| anyhow!("Primary provider is not OpenAI"))?;
            let usage = openai_provider.get_usage_quota().await?;
            Ok(format!("OpenAI Usage: Total: ${:.2}, Used: ${:.2}, Remaining: ${:.2}", usage.total_granted, usage.total_used, usage.total_available))
        } else {
            Err(anyhow!("Usage quota is only available for OpenAI provider."))
        }
    }

    pub fn get_history(&self) -> &Vec<ChatMessage> {
        &self.conversation_history
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }
}

// Helper trait to downcast AIProvider
trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: AIProvider + 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
