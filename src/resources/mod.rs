use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

// This module manages application resources such as icons, images, sounds,
// and other static assets. It can handle loading them from disk or embedded
// sources (e.g., using `asset_macro`).

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Image,
    Icon,
    Sound,
    Font,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub resource_type: ResourceType,
    pub path: PathBuf, // Path to the resource file
    pub metadata: HashMap<String, String>, // e.g., "format": "png", "size": "1024x768"
}

pub struct ResourceManager {
    resources: HashMap<String, Resource>,
    resource_dir: PathBuf,
}

impl ResourceManager {
    pub fn new() -> Self {
        let resource_dir = PathBuf::from("./assets"); // Or use a config-defined path
        Self {
            resources: HashMap::new(),
            resource_dir,
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Resource manager initialized. Resource directory: {:?}", self.resource_dir);
        fs::create_dir_all(&self.resource_dir).await?;
        self.load_default_resources().await?;
        Ok(())
    }

    async fn load_default_resources(&self) -> Result<()> {
        // Simulate loading some default resources
        // In a real app, you'd scan the resource_dir or use embedded assets.
        let mut resources = self.resources.clone(); // Clone to modify

        let icon_path = self.resource_dir.join("default_icon.png");
        if !icon_path.exists() {
            // Simulate creating a dummy file
            fs::write(&icon_path, b"// Dummy PNG content").await?;
        }
        resources.insert("default_icon".to_string(), Resource {
            id: "default_icon".to_string(),
            name: "Default App Icon".to_string(),
            resource_type: ResourceType::Icon,
            path: icon_path,
            metadata: [("format".to_string(), "png".to_string())].iter().cloned().collect(),
        });

        log::info!("Loaded {} default resources.", resources.len());
        Ok(())
    }

    pub async fn get_resource(&self, id: &str) -> Option<Resource> {
        self.resources.get(id).cloned()
    }

    /// Loads the content of a resource as bytes.
    pub async fn load_resource_bytes(&self, id: &str) -> Result<Vec<u8>> {
        if let Some(resource) = self.resources.get(id) {
            Ok(fs::read(&resource.path).await?)
        } else {
            Err(anyhow::anyhow!("Resource '{}' not found.", id))
        }
    }

    /// Loads the content of a resource as a string (for text-based resources).
    pub async fn load_resource_string(&self, id: &str) -> Result<String> {
        if let Some(resource) = self.resources.get(id) {
            Ok(fs::read_to_string(&resource.path).await?)
        } else {
            Err(anyhow::anyhow!("Resource '{}' not found.", id))
        }
    }
}

pub fn init() {
    log::info!("Resources module initialized.");
}
