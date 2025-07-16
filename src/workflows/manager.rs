use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use crate::config::CONFIG_DIR;

// This module manages workflows: loading, saving, executing, and providing
// a user interface for creating and editing workflows.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub environment: HashMap<String, String>,
    pub timeout: Option<u64>, // Timeout for the entire workflow
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
    pub environment: HashMap<String, String>,
    pub timeout: Option<u64>, // Timeout for the step
    pub retry_count: u32,
    pub condition: Option<String>, // Conditional execution (e.g., "status == 0")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowOutputFormat {
    PlainText,
    Json,
    Regex { pattern: String },
}

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

    pub async fn init(&self) -> Result<()> {
        log::info!("Workflow manager initialized. Workflow directory: {:?}", self.workflow_dir);
        fs::create_dir_all(&self.workflow_dir).await?;
        self.load_default_workflows().await?;
        Ok(())
    }

    async fn load_default_workflows(&self) -> Result<()> {
        let mut workflows = self.workflows.clone(); // Clone to modify

        // Simulate loading some default workflows from YAML files
        let wf1_path = self.workflow_dir.join("git-status.yaml");
        if !wf1_path.exists() {
            fs::write(&wf1_path, include_str!("../../workflows/git-status.yaml")).await?;
        }
        let wf1_contents = fs::read_to_string(&wf1_path).await?;
        let wf1: Workflow = serde_yaml::from_str(&wf1_contents)?;
        workflows.insert(wf1.name.clone(), wf1);

        let wf2_path = self.workflow_dir.join("docker-cleanup.yaml");
        if !wf2_path.exists() {
            fs::write(&wf2_path, include_str!("../../workflows/docker-cleanup.yaml")).await?;
        }
        let wf2_contents = fs::read_to_string(&wf2_path).await?;
        let wf2: Workflow = serde_yaml::from_str(&wf2_contents)?;
        workflows.insert(wf2.name.clone(), wf2);

        let wf3_path = self.workflow_dir.join("find-large-files.yaml");
        if !wf3_path.exists() {
            fs::write(&wf3_path, include_str!("../../workflows/find-large-files.yaml")).await?;
        }
        let wf3_contents = fs::read_to_string(&wf3_path).await?;
        let wf3: Workflow = serde_yaml::from_str(&wf3_contents)?;
        workflows.insert(wf3.name.clone(), wf3);

        log::info!("Loaded {} default workflows.", workflows.len());
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
                    description: "A basic imported workflow".to_string(),
                    steps: vec![],
                    environment: HashMap::new(),
                    timeout: None,
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
