use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a programming language supported by NeoTerm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub extensions: Vec<String>,
    pub syntax_highlight_scope: String, // e.g., "source.rust", "source.js"
    pub comment_syntax: CommentSyntax,
    pub linter_command: Option<String>,
    pub formatter_command: Option<String>,
    pub build_command: Option<String>,
    pub run_command: Option<String>,
}

/// Defines the comment syntax for a language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentSyntax {
    pub single_line: Option<String>,
    pub multi_line_start: Option<String>,
    pub multi_line_end: Option<String>,
}

impl Default for CommentSyntax {
    fn default() -> Self {
        Self {
            single_line: None,
            multi_line_start: None,
            multi_line_end: None,
        }
    }
}

/// Manages supported programming languages and their configurations.
pub struct LanguageManager {
    languages: HashMap<String, Language>, // Keyed by language name
}

impl LanguageManager {
    pub fn new() -> Self {
        let mut manager = Self {
            languages: HashMap::new(),
        };
        manager.load_default_languages();
        manager
    }

    fn load_default_languages(&mut self) {
        self.register_language(Language {
            name: "Rust".to_string(),
            extensions: vec!["rs".to_string()],
            syntax_highlight_scope: "source.rust".to_string(),
            comment_syntax: CommentSyntax {
                single_line: Some("//".to_string()),
                multi_line_start: Some("/*".to_string()),
                multi_line_end: Some("*/".to_string()),
            },
            linter_command: Some("cargo clippy".to_string()),
            formatter_command: Some("cargo fmt".to_string()),
            build_command: Some("cargo build".to_string()),
            run_command: Some("cargo run".to_string()),
        });

        self.register_language(Language {
            name: "Python".to_string(),
            extensions: vec!["py".to_string()],
            syntax_highlight_scope: "source.python".to_string(),
            comment_syntax: CommentSyntax {
                single_line: Some("#".to_string()),
                multi_line_start: Some("\"\"\"".to_string()),
                multi_line_end: Some("\"\"\"".to_string()),
            },
            linter_command: Some("flake8".to_string()),
            formatter_command: Some("black".to_string()),
            build_command: None,
            run_command: Some("python".to_string()),
        });

        self.register_language(Language {
            name: "JavaScript".to_string(),
            extensions: vec!["js".to_string(), "jsx".to_string(), "mjs".to_string(), "cjs".to_string()],
            syntax_highlight_scope: "source.js".to_string(),
            comment_syntax: CommentSyntax {
                single_line: Some("//".to_string()),
                multi_line_start: Some("/*".to_string()),
                multi_line_end: Some("*/".to_string()),
            },
            linter_command: Some("eslint".to_string()),
            formatter_command: Some("prettier".to_string()),
            build_command: Some("npm run build".to_string()),
            run_command: Some("node".to_string()),
        });
    }

    /// Registers a new language.
    pub fn register_language(&mut self, language: Language) {
        self.languages.insert(language.name.clone(), language);
    }

    /// Retrieves a language by its name.
    pub fn get_language_by_name(&self, name: &str) -> Option<&Language> {
        self.languages.get(name)
    }

    /// Retrieves a language by its file extension.
    pub fn get_language_by_extension(&self, extension: &str) -> Option<&Language> {
        self.languages.values().find(|lang| lang.extensions.contains(&extension.to_string()))
    }

    /// Returns a list of all registered language names.
    pub fn get_all_language_names(&self) -> Vec<String> {
        self.languages.keys().cloned().collect()
    }
}

pub fn init() {
    println!("languages module loaded");
}
