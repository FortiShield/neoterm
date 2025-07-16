use crate::ai::context::AIContext;
use crate::ai::providers::ChatMessage;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build_suggestion_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a helpful terminal assistant. Provide concise and actionable suggestions, often in the form of shell commands. Consider the user's current environment and recent history.\n\n");
        prompt.push_str(&format!("Current Working Directory: {}\n", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}\n", env));
        }
        if !context.recent_commands.is_empty() {
            prompt.push_str(&format!("Recent Commands: {:?}\n", context.recent_commands));
        }
        if let Some(selected_text) = &context.selected_text {
            prompt.push_str(&format!("Selected Text: {}\n", selected_text));
        }
        prompt.push_str("Based on this context, suggest a relevant action or command for the user's query.");

        ChatMessage {
            role: "system".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_fix_prompt(&self, context: &AIContext, code_snippet: &str, error_message: &str) -> ChatMessage {
        let mut prompt = String::from("You are an expert debugger and code fixer. Analyze the provided code snippet and error message, then provide the corrected code. Explain your changes concisely.\n\n");
        prompt.push_str(&format!("Current Working Directory: {}\n", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}\n", env));
        }
        prompt.push_str(&format!("Code to fix:\n```\n{}\n```\n", code_snippet));
        prompt.push_str(&format!("Error Message:\n{}\n", error_message));
        prompt.push_str("Provide only the corrected code block, followed by a brief explanation.");

        ChatMessage {
            role: "system".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_explanation_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a knowledgeable explainer. Provide clear, concise, and easy-to-understand explanations for technical concepts, code, or commands. Tailor your explanation to the context if possible.\n\n");
        prompt.push_str(&format!("Current Working Directory: {}\n", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}\n", env));
        }
        prompt.push_str("Explain the given text or concept.");

        ChatMessage {
            role: "system".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_general_chat_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a general-purpose AI assistant integrated into a terminal. You can answer questions, provide information, and assist with various tasks. Be helpful and informative.\n\n");
        prompt.push_str(&format!("Current Working Directory: {}\n", context.cwd));
        if let Some(env) = &context.env_vars {
            prompt.push_str(&format!("Environment Variables: {:?}\n", env));
        }
        if !context.recent_commands.is_empty() {
            prompt.push_str(&format!("Recent Commands: {:?}\n", context.recent_commands));
        }
        if let Some(selected_text) = &context.selected_text {
            prompt.push_str(&format!("Selected Text: {}\n", selected_text));
        }
        prompt.push_str("Engage in a helpful conversation.");

        ChatMessage {
            role: "system".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}
