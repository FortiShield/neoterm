use crate::ai::context::AIContext;
use crate::ai::providers::ChatMessage;
use std::collections::HashMap; // Import HashMap

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build_suggestion_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are an intelligent terminal assistant providing helpful suggestions. Based on the user's current context, suggest relevant commands, files, or actions.

");
        prompt.push_str(&format!("Current Working Directory: {}
", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}
", env));
        }
        if !context.recent_commands.is_empty() {
            prompt.push_str(&format!("Recent Commands: {:?}
", context.recent_commands));
        }
        if let Some(selected_text) = &context.selected_text {
            prompt.push_str(&format!("Selected Text: {}
", selected_text));
        }
        prompt.push_str("Provide concise and actionable suggestions.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_fix_prompt(&self, context: &AIContext, code_snippet: &str, error_message: &str) -> ChatMessage {
        let mut prompt = String::from("You are an expert debugger and code fixer. Analyze the provided code snippet and error message, then provide the corrected code.

");
        prompt.push_str(&format!("Current Working Directory: {}
", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}
", env));
        }
        prompt.push_str(&format!("Code Snippet:
{}
", code_snippet));
        prompt.push_str(&format!("Error Message:
{}
", error_message));
        prompt.push_str("Provide only the corrected code block, followed by a brief explanation.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_explanation_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a knowledgeable explainer. Provide clear, concise, and easy-to-understand explanations for technical concepts, code, or commands. Tailor your explanation to the context if possible.

");
        prompt.push_str(&format!("Current Working Directory: {}
", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}
", env));
        }
        prompt.push_str("Explain the given text or concept.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_general_chat_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a general-purpose AI assistant integrated into a terminal. You can answer questions, provide information, and assist with various tasks. Be helpful and informative.

");
        prompt.push_str(&format!("Current Working Directory: {}
", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}
", env));
        }
        if !context.recent_commands.is_empty() {
            prompt.push_str(&format!("Recent Commands: {:?}
", context.recent_commands));
        }
        if let Some(selected_text) = &context.selected_text {
            prompt.push_str(&format!("Selected Text: {}
", selected_text));
        }
        prompt.push_str("Engage in a helpful conversation.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    // New function for command generation prompt
    pub fn build_command_generation_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a highly specialized AI assistant whose sole purpose is to generate a single, valid shell command based on a natural language request. You must only output the command, with no additional text, explanations, or formatting (e.g., no markdown code blocks). If you cannot generate a suitable command, output 'ERROR: Could not generate command.'.

Current Terminal Context:
");
        prompt.push_str(&format!("- Current Working Directory: {}
", context.cwd));
        if let Some(env) = &context.env_vars {
            // Filter environment variables to only include common ones for context
            let filtered_env: HashMap<String, String> = env.iter()
                .filter(|(k, _)| k.starts_with("PATH") || k.starts_with("HOME") || k.starts_with("USER") || k.starts_with("LANG") || k.starts_with("SHELL"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            prompt.push_str(&format!("- Environment Variables (filtered for common ones): {:?}
", filtered_env));
        }
        if !context.recent_commands.is_empty() {
            prompt.push_str(&format!("- Recent Commands: {:?}
", context.recent_commands));
        }
        if let Some(selected_text) = &context.selected_text {
            prompt.push_str(&format!("- Selected Text: {}
", selected_text));
        }
        prompt.push_str("\nBased on the user's request and the provided context, generate the most appropriate shell command. Output ONLY the command.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}
