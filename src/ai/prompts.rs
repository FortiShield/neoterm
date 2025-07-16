use crate::ai::context::AIContext;
use crate::ai::providers::ChatMessage;
use std::collections::HashMap; // Import HashMap

pub struct PromptBuilder {
    // Add any internal state if needed for prompt building
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build_suggestion_prompt(&self, context: &AIContext) -> ChatMessage {
        let context_str = context.to_string();
        ChatMessage {
            role: "system".to_string(),
            content: Some(format!(
                "You are a helpful terminal assistant. Based on the current context, suggest the most relevant next command or action.
Current context:
{}
Your response should be a single, concise command or a short suggestion.",
                context_str
            )),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn build_fix_prompt(&self, context: &AIContext, code_snippet: &str, error_message: &str) -> ChatMessage {
        let context_str = context.to_string();
        ChatMessage {
            role: "system".to_string(),
            content: Some(format!(
                "You are an expert debugger and terminal command fixer. Analyze the provided failed command and error message, and suggest a corrected command.
Current context:
{}
Failed command:
{}
Error message:
{}
Provide ONLY the corrected command.",
                context_str, code_snippet, error_message
            )),
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

    pub fn build_explain_output_prompt(&self, context: &AIContext, command_input: &str, output: &str, error_message: Option<&str>) -> ChatMessage {
        let mut prompt = String::from("You are an expert terminal output and error explainer. Your task is to provide a clear, concise, and easy-to-understand explanation of a given shell command's output or error message. Focus on what happened, why it happened (if an error), and what it means.

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
        prompt.push_str(&format!("Command: `{}`
", command_input));
        prompt.push_str(&format!("Output:\n\`\`\`\n{}\n\`\`\`\n", output));
        if let Some(error) = error_message {
            prompt.push_str(&format!("Error Message:\n\`\`\`\n{}\n\`\`\`\n", error));
        }
        prompt.push_str("Explain the output and/or error message in natural language.");

        ChatMessage {
            role: "system".to_string(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}
