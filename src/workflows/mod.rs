use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod parser;
pub mod manager;
pub mod executor;
pub mod ui;

pub use parser::*;
pub use manager::*;
pub use executor::*;
pub use ui::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// The name of the Workflow. Required.
    pub name: String,
    
    /// The command that is executed when the Workflow is selected. Required.
    pub command: String,
    
    /// An array of tags that are useful to categorize the Workflow. Optional.
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// The description of the Workflow and what it does. Optional.
    pub description: Option<String>,
    
    /// The URL from where the Workflow was originally generated from. Optional.
    pub source_url: Option<String>,
    
    /// The original author of the Workflow. Optional.
    pub author: Option<String>,
    
    /// The URL of original author of the Workflow. Optional.
    pub author_url: Option<String>,
    
    /// The list of shells where this Workflow is valid. Optional.
    /// Must be one of: zsh, bash, fish
    pub shells: Option<Vec<Shell>>,
    
    /// Parameterized arguments for the workflow. Optional.
    #[serde(default)]
    pub arguments: Vec<WorkflowArgument>,
    
    // Internal metadata
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
    
    #[serde(skip)]
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    
    #[serde(skip)]
    pub usage_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowArgument {
    /// The name of the argument. Required.
    pub name: String,
    
    /// The description of the argument. Optional.
    pub description: Option<String>,
    
    /// The default value for the argument. Optional.
    pub default_value: Option<String>,
    
    /// The type of argument for validation. Optional.
    #[serde(default)]
    pub arg_type: ArgumentType,
    
    /// Whether this argument is required. Optional.
    #[serde(default)]
    pub required: bool,
    
    /// Possible values for this argument (for enum-like arguments). Optional.
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArgumentType {
    #[default]
    String,
    Number,
    Boolean,
    Path,
    Url,
    Email,
    Enum,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub workflow: Workflow,
    pub arguments: HashMap<String, String>,
    pub resolved_command: String,
    pub shell: Shell,
}

#[derive(Debug, Clone)]
pub struct WorkflowSearchResult {
    pub workflow: Workflow,
    pub score: f32,
    pub matched_fields: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Argument error: {0}")]
    ArgumentError(String),
    #[error("Shell not supported: {0:?}")]
    UnsupportedShell(Shell),
    #[error("Missing required argument: {0}")]
    MissingArgument(String),
    #[error("Invalid argument value: {0}")]
    InvalidArgumentValue(String),
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
}

impl Workflow {
    /// Parse workflow from YAML string
    pub fn from_yaml(yaml_str: &str) -> Result<Self, WorkflowError> {
        let mut workflow: Workflow = serde_yaml::from_str(yaml_str)
            .map_err(|e| WorkflowError::ParseError(e.to_string()))?;
        
        workflow.validate()?;
        Ok(workflow)
    }

    /// Convert workflow to YAML string
    pub fn to_yaml(&self) -> Result<String, WorkflowError> {
        serde_yaml::to_string(self)
            .map_err(|e| WorkflowError::ParseError(e.to_string()))
    }

    /// Load workflow from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, WorkflowError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;
        
        let mut workflow = Self::from_yaml(&content)?;
        workflow.file_path = Some(path.as_ref().to_path_buf());
        Ok(workflow)
    }

    /// Save workflow to file
    pub fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), WorkflowError> {
        let yaml_str = self.to_yaml()?;
        std::fs::write(&path, yaml_str)
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Validate workflow structure and content
    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.name.trim().is_empty() {
            return Err(WorkflowError::ValidationError("Name is required".to_string()));
        }

        if self.command.trim().is_empty() {
            return Err(WorkflowError::ValidationError("Command is required".to_string()));
        }

        // Validate shell compatibility
        if let Some(shells) = &self.shells {
            if shells.is_empty() {
                return Err(WorkflowError::ValidationError("Shells array cannot be empty".to_string()));
            }
        }

        // Validate arguments
        for arg in &self.arguments {
            if arg.name.trim().is_empty() {
                return Err(WorkflowError::ValidationError("Argument name is required".to_string()));
            }

            // Check if argument is used in command
            let placeholder = format!("{{{{{}}}}}", arg.name);
            if !self.command.contains(&placeholder) {
                return Err(WorkflowError::ValidationError(
                    format!("Argument '{}' is not used in command", arg.name)
                ));
            }

            // Validate enum options
            if arg.arg_type == ArgumentType::Enum && arg.options.is_none() {
                return Err(WorkflowError::ValidationError(
                    format!("Enum argument '{}' must have options", arg.name)
                ));
            }
        }

        // Check for unused placeholders in command
        let placeholders = self.extract_placeholders();
        for placeholder in placeholders {
            if !self.arguments.iter().any(|arg| arg.name == placeholder) {
                return Err(WorkflowError::ValidationError(
                    format!("Placeholder '{}' has no corresponding argument", placeholder)
                ));
            }
        }

        Ok(())
    }

    /// Extract all placeholders from the command
    pub fn extract_placeholders(&self) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut chars = self.command.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                let mut placeholder = String::new();
                
                while let Some(ch) = chars.next() {
                    if ch == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        if !placeholder.is_empty() {
                            placeholders.push(placeholder);
                        }
                        break;
                    } else {
                        placeholder.push(ch);
                    }
                }
            }
        }
        
        placeholders
    }

    /// Check if workflow is compatible with given shell
    pub fn is_compatible_with_shell(&self, shell: &Shell) -> bool {
        self.shells.as_ref().map_or(true, |shells| shells.contains(shell))
    }

    /// Get workflow category based on tags
    pub fn get_category(&self) -> WorkflowCategory {
        for tag in &self.tags {
            match tag.to_lowercase().as_str() {
                "git" => return WorkflowCategory::Git,
                "docker" => return WorkflowCategory::Docker,
                "kubernetes" | "k8s" => return WorkflowCategory::Kubernetes,
                "aws" => return WorkflowCategory::Aws,
                "database" | "db" => return WorkflowCategory::Database,
                "network" => return WorkflowCategory::Network,
                "file" | "filesystem" => return WorkflowCategory::FileSystem,
                "system" => return WorkflowCategory::System,
                _ => continue,
            }
        }
        WorkflowCategory::Other
    }

    /// Calculate search relevance score
    pub fn calculate_search_score(&self, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let mut score = 0.0;

        // Name match (highest weight)
        if self.name.to_lowercase().contains(&query_lower) {
            score += 10.0;
            if self.name.to_lowercase() == query_lower {
                score += 20.0; // Exact match bonus
            }
        }

        // Tag match (high weight)
        for tag in &self.tags {
            if tag.to_lowercase().contains(&query_lower) {
                score += 8.0;
                if tag.to_lowercase() == query_lower {
                    score += 12.0; // Exact match bonus
                }
            }
        }

        // Description match (medium weight)
        if let Some(description) = &self.description {
            if description.to_lowercase().contains(&query_lower) {
                score += 5.0;
            }
        }

        // Command match (lower weight)
        if self.command.to_lowercase().contains(&query_lower) {
            score += 3.0;
        }

        // Author match (low weight)
        if let Some(author) = &self.author {
            if author.to_lowercase().contains(&query_lower) {
                score += 2.0;
            }
        }

        // Usage frequency bonus
        score += (self.usage_count as f32).log10();

        score
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowCategory {
    Git,
    Docker,
    Kubernetes,
    Aws,
    Database,
    Network,
    FileSystem,
    System,
    Other,
}

impl std::fmt::Display for WorkflowCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowCategory::Git => write!(f, "Git"),
            WorkflowCategory::Docker => write!(f, "Docker"),
            WorkflowCategory::Kubernetes => write!(f, "Kubernetes"),
            WorkflowCategory::Aws => write!(f, "AWS"),
            WorkflowCategory::Database => write!(f, "Database"),
            WorkflowCategory::Network => write!(f, "Network"),
            WorkflowCategory::FileSystem => write!(f, "File System"),
            WorkflowCategory::System => write!(f, "System"),
            WorkflowCategory::Other => write!(f, "Other"),
        }
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Zsh => write!(f, "zsh"),
            Shell::Bash => write!(f, "bash"),
            Shell::Fish => write!(f, "fish"),
        }
    }
}

impl std::str::FromStr for Shell {
    type Err = WorkflowError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zsh" => Ok(Shell::Zsh),
            "bash" => Ok(Shell::Bash),
            "fish" => Ok(Shell::Fish),
            _ => Err(WorkflowError::UnsupportedShell(Shell::Bash)), // Default error
        }
    }
}
