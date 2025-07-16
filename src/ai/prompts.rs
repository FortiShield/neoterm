use crate::ai::context::AIContext;
use crate::ai::providers::ChatMessage;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build_suggestion_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are a helpful terminal assistant. Suggest relevant commands or actions based on the user's current context and query. Be concise.

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
        prompt.push_str("Suggest a command or action.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_fix_prompt(&self, context: &AIContext, command_snippet: &str, error_message: &str) -> ChatMessage {
        let mut prompt = String::from("You are an expert terminal assistant. Your task is to analyze a failed shell command and its error message, then provide the corrected shell command.
Provide ONLY the corrected shell command. Do not include any explanations, markdown formatting, or extra text. Just the command.

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
        prompt.push_str(&format!("Failed Command: {}
", command_snippet));
        prompt.push_str(&format!("Error Message: {}
", error_message));
        prompt.push_str("Provide ONLY the corrected shell command.");

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

    pub fn build_command_generation_prompt(&self, context: &AIContext) -> ChatMessage {
        let mut prompt = String::from("You are an expert terminal command generator. Your task is to convert natural language queries into valid shell commands.
Provide ONLY the shell command. Do not include any explanations, markdown formatting, or extra text. Just the command.

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
        prompt.push_str("Generate a shell command for the given natural language query.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}
