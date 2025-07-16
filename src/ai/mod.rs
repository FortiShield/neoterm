pub mod assistant;
pub mod prompts;
pub mod context;
pub mod providers;

pub use assistant::Assistant;
pub use prompts::PromptBuilder;
pub use context::AIContext;
pub use providers::{AIProvider, ChatMessage, OpenAIProvider, OllamaProvider, AnthropicProvider};

pub fn init() {
    log::debug!("AI module initialized.");
}
