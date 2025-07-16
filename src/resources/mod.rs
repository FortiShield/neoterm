use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a generic resource that can be loaded from the file system.
#[derive(Debug, Clone)]
pub enum Resource {
    Text(String),
    Binary(Vec<u8>),
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
    // Add other resource types as needed (e.g., Image, Audio)
}

/// Manages loading and accessing application resources (e.g., themes, workflows, icons).
pub struct ResourceManager {
    base_path: PathBuf,
    loaded_resources: HashMap<String, Resource>, // Keyed by relative path or ID
}

impl ResourceManager {
    /// Creates a new `ResourceManager` instance with a specified base path.
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            loaded_resources: HashMap::new(),
        }
    }

    /// Loads a resource from a given relative path.
    /// The resource type is inferred from the file extension.
    pub fn load_resource(&mut self, relative_path: &Path) -> Result<&Resource, String> {
        let full_path = self.base_path.join(relative_path);
        let key = relative_path.to_string_lossy().into_owned();

        if self.loaded_resources.contains_key(&key) {
            return Ok(self.loaded_resources.get(&key).unwrap());
        }

        if !full_path.exists() {
            return Err(format!("Resource not found: {}", full_path.display()));
        }

        let resource = match full_path.extension().and_then(|s| s.to_str()) {
            Some("json") => {
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| format!("Failed to read JSON file {}: {}", full_path.display(), e))?;
                let value: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse JSON file {}: {}", full_path.display(), e))?;
                Resource::Json(value)
            },
            Some("yaml") | Some("yml") => {
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| format!("Failed to read YAML file {}: {}", full_path.display(), e))?;
                let value: serde_yaml::Value = serde_yaml::from_str(&content)
                    .map_err(|e| format!("Failed to parse YAML file {}: {}", full_path.display(), e))?;
                Resource::Yaml(value)
            },
            Some("txt") | Some("md") | Some("log") | Some("sh") | Some("ps1") | Some("rs") | Some("py") | Some("js") => {
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| format!("Failed to read text file {}: {}", full_path.display(), e))?;
                Resource::Text(content)
            },
            _ => {
                // Default to binary for unknown extensions
                let content = fs::read(&full_path)
                    .map_err(|e| format!("Failed to read binary file {}: {}", full_path.display(), e))?;
                Resource::Binary(content)
            },
        };

        self.loaded_resources.insert(key.clone(), resource);
        Ok(self.loaded_resources.get(&key).unwrap())
    }

    /// Retrieves a loaded resource by its relative path or ID.
    pub fn get_resource(&self, key: &str) -> Option<&Resource> {
        self.loaded_resources.get(key)
    }

    /// Lists all resources found in a given subdirectory relative to the base path.
    pub fn list_resources_in_subdir(&self, subdir_path: &Path) -> Result<Vec<PathBuf>, String> {
        let full_subdir_path = self.base_path.join(subdir_path);
        if !full_subdir_path.exists() || !full_subdir_path.is_dir() {
            return Err(format!("Subdirectory not found: {}", full_subdir_path.display()));
        }

        let mut resource_paths = Vec::new();
        for entry in fs::read_dir(&full_subdir_path)
            .map_err(|e| format!("Failed to read directory {}: {}", full_subdir_path.display(), e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_file() {
                resource_paths.push(path);
            }
        }
        Ok(resource_paths)
    }
}

pub fn init() {
    println!("resources module loaded");
}
