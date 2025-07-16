use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use crate::config::CONFIG_DIR;
use super::Workflow; // Import Workflow from parent module

// This module manages workflows: loading, saving, executing, and providing
// a user interface for creating and editing workflows.

pub struct WorkflowManager {
    workflows: HashMap<String, Workflow>,
    workflow_dir: PathBuf,
}

impl WorkflowManager {
    pub fn new() -> Self {
        let workflow_dir = CONFIG_DIR.join("workflows");
        Self {
            workflows: HashMap::new(),
            workflow_dir,
        }
    }

    pub async fn init(&mut self) -> Result<()> { // Changed to mutable self
        log::info!("Workflow manager initialized. Workflow directory: {:?}", self.workflow_dir);
        fs::create_dir_all(&self.workflow_dir).await?;
        self.load_default_workflows().await?;
        self.load_user_workflows().await?; // Load any user-defined workflows
        Ok(())
    }

    async fn load_default_workflows(&mut self) -> Result<()> { // Changed to mutable self
        // Simulate loading some default workflows from YAML files
        let defaults = vec![
            ("git-status.yaml", include_str!("../../workflows/git-status.yaml")),
            ("docker-cleanup.yaml", include_str!("../../workflows/docker-cleanup.yaml")),
            ("find-large-files.yaml", include_str!("../../workflows/find-large-files.yaml")),
        ];

        for (filename, content) in defaults {
            let wf_path = self.workflow_dir.join(filename);
            if !wf_path.exists() {
                fs::write(&wf_path, content).await?;
            }
            let wf_contents = fs::read_to_string(&wf_path).await?;
            let wf: Workflow = serde_yaml::from_str(&wf_contents)?;
            self.workflows.insert(wf.name.clone(), wf);
        }
        log::info!("Loaded {} default workflows.", self.workflows.len());
        Ok(())
    }

    async fn load_user_workflows(&mut self) -> Result<()> {
        let mut entries = fs::read_dir(&self.workflow_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path).await?;
                match Workflow::from_yaml(&content) {
                    Ok(wf) => {
                        self.workflows.insert(wf.name.clone(), wf);
                    },
                    Err(e) => {
                        log::error!("Failed to load workflow from {:?}: {}", path, e);
                    }
                }
            }
        }
        log::info!("Loaded {} user workflows.", self.workflows.len());
        Ok(())
    }

    pub async fn get_workflow(&self, name: &str) -> Result<Workflow> {
        self.workflows.get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Workflow '{}' not found.", name))
    }

    pub async fn list_workflows(&self) -> Vec<Workflow> {
        self.workflows.values().cloned().collect()
    }

    pub async fn save_workflow(&mut self, workflow: Workflow) -> Result<()> {
        let path = self.workflow_dir.join(format!("{}.yaml", workflow.name));
        let contents = serde_yaml::to_string(&workflow)?;
        fs::write(&path, contents).await?;
        log::info!("Workflow '{}' saved to {:?}", workflow.name, path);
        self.workflows.insert(workflow.name.clone(), workflow);
        Ok(())
    }

    pub async fn delete_workflow(&mut self, name: &str) -> Result<()> {
        let path = self.workflow_dir.join(format!("{}.yaml", name));
        if path.exists() {
            fs::remove_file(&path).await?;
            log::info!("Workflow '{}' deleted from {:?}", name, path);
        }
        self.workflows.remove(name);
        Ok(())
    }

    pub async fn import_workflow(&mut self, source: &str) -> Result<String> {
        // Simulate importing from a file or URL
        let contents = match source {
            "default" => {
                // Load a default workflow from a string
                let default_workflow = Workflow {
                    id: Uuid::new_v4().to_string(),
                    name: "Imported Workflow".to_string(),
                    description: Some("A basic imported workflow".to_string()),
                    steps: vec![],
                    environment: HashMap::new(),
                    timeout: None,
                    tags: Vec::new(),
                    source_url: None,
                    author: None,
                    author_url: None,
                    shells: None,
                    arguments: Vec::new(),
                    file_path: None,
                    last_used: None,
                    usage_count: 0,
                };
                serde_yaml::to_string(&default_workflow)?
            }
            _ => {
                // Load from a file (assuming it's a path)
                fs::read_to_string(source).await?
            }
        };

        let workflow: Workflow = serde_yaml::from_str(&contents)?;
        self.save_workflow(workflow.clone()).await?;
        Ok(workflow.name)
    }
}
