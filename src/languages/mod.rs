use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;
use tree_sitter::{Language, Parser};
// use syntect::{highlighting::Highlighter, parsing::SyntaxSet};

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
    parsers: Arc<Mutex<HashMap<String, Parser>>>,
    languages: HashMap<String, Language>, // Keyed by language name
    // Add Syntect theme set and syntax set for highlighting
    // syntax_set: Arc<Mutex<syntect::parsing::SyntaxSet>>,
    // theme_set: Arc<Mutex<syntect::highlighting::ThemeSet>>,
}

impl LanguageManager {
    pub fn new() -> Self {
        Self {
            parsers: Arc::new(Mutex::new(HashMap::new())),
            languages: HashMap::new(),
            // syntax_set: Arc::new(Mutex::new(syntect::parsing::SyntaxSet::load_defaults_newlines())),
            // theme_set: Arc::new(Mutex::new(syntect::highlighting::ThemeSet::load_defaults())),
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Language manager initialized.");
        self.load_default_languages().await?;
        Ok(())
    }

    async fn load_default_languages(&self) -> Result<()> {
        let mut parsers = self.parsers.lock().await;
        let mut languages = self.languages.lock().await;

        // Load Tree-sitter languages
        // Ensure these grammars are built and available (e.g., via build.rs)
        // For example:
        // parsers.insert("rust".to_string(), {
        //     let mut parser = Parser::new();
        //     parser.set_language(tree_sitter_rust::language())?;
        //     parser
        // });
        parsers.insert("bash".to_string(), {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_bash::language())?;
            parser
        });

        // Register languages
        languages.insert("Rust".to_string(), Language {
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

        languages.insert("Python".to_string(), Language {
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

        languages.insert("JavaScript".to_string(), Language {
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

        log::info!("Loaded {} Tree-sitter languages.", parsers.len());
        Ok(())
    }

    pub async fn get_parser(&self, language_name: &str) -> Option<Parser> {
        let parsers = self.parsers.lock().await;
        parsers.get(language_name).cloned()
    }

    /// Example: Parse code and return a syntax tree.
    pub async fn parse_code(&self, language_name: &str, code: &str) -> Result<String> {
        let mut parsers = self.parsers.lock().await;
        if let Some(parser) = parsers.get_mut(language_name) {
            if let Some(tree) = parser.parse(code, None) {
                Ok(tree.root_node().to_sexp())
            } else {
                Err(anyhow::anyhow!("Failed to parse code for language: {}", language_name))
            }
        } else {
            Err(anyhow::anyhow!("Language parser not found for: {}", language_name))
        }
    }

    /// Example: Get syntax highlighting for a line of code.
    pub async fn highlight_line(&self, _language_name: &str, _line: &str) -> Result<Vec<(syntect::highlighting::Style, &str)>> {
        // This would use Syntect
        // let syntax_set = self.syntax_set.lock().await;
        // let theme_set = self.theme_set.lock().await;
        // let syntax = syntax_set.find_syntax_by_name(language_name)
        //     .ok_or_else(|| anyhow::anyhow!("Syntax not found for: {}", language_name))?;
        // let theme = &theme_set.themes["base16-ocean.dark"]; // Or load from config
        // let mut highlighter = syntect::highlighting::Highlighter::new(theme);
        // let regions = highlighter.highlight_line(line, syntax, &syntax_set)?;
        // Ok(regions)
        Ok(vec![]) // Placeholder
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
    log::info!("Languages module initialized.");
}
