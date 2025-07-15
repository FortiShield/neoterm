use super::{Workflow, WorkflowError, WorkflowCategory, Shell, WorkflowSearchResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

pub struct WorkflowManager {
    workflows: HashMap<String, Workflow>,
    workflows_dir: PathBuf,
    categories: HashMap<WorkflowCategory, Vec<String>>,
    matcher: SkimMatcherV2,
    usage_stats: HashMap<String, WorkflowUsageStats>,
}

#[derive(Debug, Clone)]
pub struct WorkflowUsageStats {
    pub usage_count: u32,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub average_execution_time: Option<std::time::Duration>,
    pub success_rate: f32,
}

impl WorkflowManager {
    pub fn new() -> Result<Self, WorkflowError> {
        let workflows_dir = Self::get_workflows_dir()?;
        
        // Ensure workflows directory exists
        if !workflows_dir.exists() {
            std::fs::create_dir_all(&workflows_dir)
                .map_err(|e| WorkflowError::IoError(e.to_string()))?;
            
            // Create example workflows
            Self::create_example_workflows(&workflows_dir)?;
        }

        let mut manager = Self {
            workflows: HashMap::new(),
            workflows_dir,
            categories: HashMap::new(),
            matcher: SkimMatcherV2::default(),
            usage_stats: HashMap::new(),
        };

        manager.load_workflows()?;
        manager.load_usage_stats()?;
        Ok(manager)
    }

    /// Get the workflows directory path
    pub fn get_workflows_dir() -> Result<PathBuf, WorkflowError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| WorkflowError::IoError("Config directory not found".to_string()))?;
        
        Ok(config_dir.join("neoterm").join("workflows"))
    }

    /// Load all workflows from the workflows directory
    pub fn load_workflows(&mut self) -> Result<(), WorkflowError> {
        self.workflows.clear();
        self.categories.clear();

        if !self.workflows_dir.exists() {
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&self.workflows_dir) {
            let entry = entry.map_err(|e| WorkflowError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "yml" || extension == "yaml" {
                        match Workflow::from_file(path) {
                            Ok(mut workflow) => {
                                // Apply usage stats
                                if let Some(stats) = self.usage_stats.get(&workflow.name) {
                                    workflow.usage_count = stats.usage_count;
                                    workflow.last_used = Some(stats.last_used);
                                }

                                let category = workflow.get_category();
                                self.categories
                                    .entry(category)
                                    .or_insert_with(Vec::new)
                                    .push(workflow.name.clone());

                                self.workflows.insert(workflow.name.clone(), workflow);
                            }
                            Err(e) => {
                                eprintln!("Failed to load workflow from {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Search workflows by query
    pub fn search_workflows(&self, query: &str, shell: Option<&Shell>) -> Vec<WorkflowSearchResult> {
        if query.is_empty() {
            return self.get_all_workflows(shell);
        }

        let mut results: Vec<WorkflowSearchResult> = self.workflows
            .values()
            .filter(|workflow| {
                shell.map_or(true, |s| workflow.is_compatible_with_shell(s))
            })
            .filter_map(|workflow| {
                let score = workflow.calculate_search_score(query);
                if score > 0.0 {
                    Some(WorkflowSearchResult {
                        workflow: workflow.clone(),
                        score,
                        matched_fields: self.get_matched_fields(workflow, query),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        results
    }

    /// Get all workflows, optionally filtered by shell
    pub fn get_all_workflows(&self, shell: Option<&Shell>) -> Vec<WorkflowSearchResult> {
        let mut workflows: Vec<WorkflowSearchResult> = self.workflows
            .values()
            .filter(|workflow| {
                shell.map_or(true, |s| workflow.is_compatible_with_shell(s))
            })
            .map(|workflow| WorkflowSearchResult {
                workflow: workflow.clone(),
                score: workflow.usage_count as f32,
                matched_fields: vec![],
            })
            .collect();

        // Sort by usage count and last used
        workflows.sort_by(|a, b| {
            let usage_cmp = b.workflow.usage_count.cmp(&a.workflow.usage_count);
            if usage_cmp == std::cmp::Ordering::Equal {
                b.workflow.last_used.cmp(&a.workflow.last_used)
            } else {
                usage_cmp
            }
        });

        workflows
    }

    /// Get workflows by category
    pub fn get_workflows_by_category(&self, category: &WorkflowCategory, shell: Option<&Shell>) -> Vec<Workflow> {
        self.categories
            .get(category)
            .map(|workflow_names| {
                workflow_names
                    .iter()
                    .filter_map(|name| self.workflows.get(name))
                    .filter(|workflow| {
                        shell.map_or(true, |s| workflow.is_compatible_with_shell(s))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get workflow by name
    pub fn get_workflow(&self, name: &str) -> Option<&Workflow> {
        self.workflows.get(name)
    }

    /// Add or update a workflow
    pub fn add_workflow(&mut self, workflow: Workflow) -> Result<(), WorkflowError> {
        workflow.validate()?;

        let file_path = self.workflows_dir.join(format!("{}.yaml", sanitize_filename(&workflow.name)));
        workflow.to_file(&file_path)?;

        let category = workflow.get_category();
        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(workflow.name.clone());

        self.workflows.insert(workflow.name.clone(), workflow);
        Ok(())
    }

    /// Remove a workflow
    pub fn remove_workflow(&mut self, name: &str) -> Result<(), WorkflowError> {
        if let Some(workflow) = self.workflows.remove(name) {
            // Remove from file system
            if let Some(file_path) = &workflow.file_path {
                if file_path.exists() {
                    std::fs::remove_file(file_path)
                        .map_err(|e| WorkflowError::IoError(e.to_string()))?;
                }
            }

            // Remove from categories
            let category = workflow.get_category();
            if let Some(category_workflows) = self.categories.get_mut(&category) {
                category_workflows.retain(|n| n != name);
            }

            // Remove usage stats
            self.usage_stats.remove(name);
        }

        Ok(())
    }

    /// Record workflow usage
    pub fn record_usage(&mut self, workflow_name: &str, execution_time: Option<std::time::Duration>, success: bool) {
        let stats = self.usage_stats
            .entry(workflow_name.to_string())
            .or_insert_with(|| WorkflowUsageStats {
                usage_count: 0,
                last_used: chrono::Utc::now(),
                average_execution_time: None,
                success_rate: 1.0,
            });

        stats.usage_count += 1;
        stats.last_used = chrono::Utc::now();

        // Update average execution time
        if let Some(exec_time) = execution_time {
            stats.average_execution_time = Some(
                stats.average_execution_time
                    .map(|avg| (avg + exec_time) / 2)
                    .unwrap_or(exec_time)
            );
        }

        // Update success rate (simple moving average)
        let success_value = if success { 1.0 } else { 0.0 };
        stats.success_rate = (stats.success_rate * 0.9) + (success_value * 0.1);

        // Update workflow usage count
        if let Some(workflow) = self.workflows.get_mut(workflow_name) {
            workflow.usage_count = stats.usage_count;
            workflow.last_used = Some(stats.last_used);
        }

        // Save usage stats
        let _ = self.save_usage_stats();
    }

    /// Get popular workflows
    pub fn get_popular_workflows(&self, limit: usize, shell: Option<&Shell>) -> Vec<Workflow> {
        let mut workflows: Vec<_> = self.workflows
            .values()
            .filter(|workflow| {
                shell.map_or(true, |s| workflow.is_compatible_with_shell(s))
            })
            .collect();

        workflows.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        workflows.into_iter().take(limit).cloned().collect()
    }

    /// Get recently used workflows
    pub fn get_recent_workflows(&self, limit: usize, shell: Option<&Shell>) -> Vec<Workflow> {
        let mut workflows: Vec<_> = self.workflows
            .values()
            .filter(|workflow| {
                workflow.last_used.is_some() && 
                shell.map_or(true, |s| workflow.is_compatible_with_shell(s))
            })
            .collect();

        workflows.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        workflows.into_iter().take(limit).cloned().collect()
    }

    /// Get all available categories
    pub fn get_categories(&self) -> Vec<WorkflowCategory> {
        self.categories.keys().cloned().collect()
    }

    /// Import workflow from URL
    pub async fn import_workflow_from_url(&mut self, url: &str) -> Result<String, WorkflowError> {
        let response = reqwest::get(url).await
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;
        
        let content = response.text().await
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;

        let workflow = Workflow::from_yaml(&content)?;
        let name = workflow.name.clone();
        self.add_workflow(workflow)?;
        
        Ok(name)
    }

    /// Export workflow to string
    pub fn export_workflow(&self, name: &str) -> Result<String, WorkflowError> {
        let workflow = self.workflows.get(name)
            .ok_or_else(|| WorkflowError::WorkflowNotFound(name.to_string()))?;
        
        workflow.to_yaml()
    }

    fn get_matched_fields(&self, workflow: &Workflow, query: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let query_lower = query.to_lowercase();

        if workflow.name.to_lowercase().contains(&query_lower) {
            fields.push("name".to_string());
        }

        for tag in &workflow.tags {
            if tag.to_lowercase().contains(&query_lower) {
                fields.push("tags".to_string());
                break;
            }
        }

        if let Some(description) = &workflow.description {
            if description.to_lowercase().contains(&query_lower) {
                fields.push("description".to_string());
            }
        }

        if workflow.command.to_lowercase().contains(&query_lower) {
            fields.push("command".to_string());
        }

        fields
    }

    fn load_usage_stats(&mut self) -> Result<(), WorkflowError> {
        let stats_file = self.workflows_dir.join("usage_stats.json");
        if stats_file.exists() {
            let content = std::fs::read_to_string(&stats_file)
                .map_err(|e| WorkflowError::IoError(e.to_string()))?;
            
            self.usage_stats = serde_json::from_str(&content)
                .map_err(|e| WorkflowError::ParseError(e.to_string()))?;
        }
        Ok(())
    }

    fn save_usage_stats(&self) -> Result<(), WorkflowError> {
        let stats_file = self.workflows_dir.join("usage_stats.json");
        let content = serde_json::to_string_pretty(&self.usage_stats)
            .map_err(|e| WorkflowError::ParseError(e.to_string()))?;
        
        std::fs::write(&stats_file, content)
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;
        
        Ok(())
    }

    fn create_example_workflows(workflows_dir: &Path) -> Result<(), WorkflowError> {
        let examples = vec![
            ("git-status.yaml", include_str!("../../workflows/git-status.yaml")),
            ("docker-cleanup.yaml", include_str!("../../workflows/docker-cleanup.yaml")),
            ("find-large-files.yaml", include_str!("../../workflows/find-large-files.yaml")),
            ("port-kill.yaml", include_str!("../../workflows/port-kill.yaml")),
            ("git-branch-cleanup.yaml", include_str!("../../workflows/git-branch-cleanup.yaml")),
        ];

        for (filename, content) in examples {
            let file_path = workflows_dir.join(filename);
            if !file_path.exists() {
                std::fs::write(&file_path, content)
                    .map_err(|e| WorkflowError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}
