use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use semver::Version;
use super::lua_engine::{LuaEngine, LuaPluginEvent};
use super::wasm_runtime::{WasmRuntime, WasmPluginEvent};
use crate::config::DATA_DIR;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    Wasm,
    Lua,
    Native, // For Rust-based plugins compiled directly into the app
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub plugin_type: PluginType,
    pub entrypoint: String, // Path to .wasm file or .lua script
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub capabilities: Vec<String>, // e.g., "access_fs", "network_access", "ui_extension"
}

#[derive(Debug, Clone)]
pub enum PluginEvent {
    Installed { name: String, version: Version },
    Uninstalled { name: String },
    Updated { name: String, old_version: Version, new_version: Version },
    Activated { name: String },
    Deactivated { name: String },
    Error { name: String, message: String },
    Lua(LuaPluginEvent),
    Wasm(WasmPluginEvent),
}

pub struct PluginManager {
    plugins: HashMap<String, PluginManifest>,
    plugin_dir: PathBuf,
    event_sender: mpsc::Sender<PluginEvent>,
    lua_engine: LuaEngine,
    wasm_runtime: WasmRuntime,
}

impl PluginManager {
    pub fn new() -> Self {
        let plugin_dir = DATA_DIR.join("plugins");
        let (tx, rx) = mpsc::channel(100); // Channel for plugin events

        // Create sub-senders for Lua and WASM engines
        let lua_tx = tx.clone();
        let wasm_tx = tx.clone();

        Self {
            plugins: HashMap::new(),
            plugin_dir,
            event_sender: tx,
            lua_engine: LuaEngine::new(lua_tx.map_msg(PluginEvent::Lua)),
            wasm_runtime: WasmRuntime::new(wasm_tx.map_msg(PluginEvent::Wasm)),
        }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Plugin manager initialized. Plugin directory: {:?}", self.plugin_dir);
        fs::create_dir_all(&self.plugin_dir).await?;
        self.load_installed_plugins().await?;
        self.lua_engine.init().await?;
        self.wasm_runtime.init().await?;
        Ok(())
    }

    async fn load_installed_plugins(&mut self) -> Result<()> {
        self.plugins.clear();
        let mut entries = fs::read_dir(&self.plugin_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    match fs::read_to_string(&manifest_path).await {
                        Ok(contents) => {
                            match serde_json::from_str::<PluginManifest>(&contents) {
                                Ok(manifest) => {
                                    log::info!("Loaded plugin: {} (Type: {:?})", manifest.name, manifest.plugin_type);
                                    self.plugins.insert(manifest.name.clone(), manifest);
                                },
                                Err(e) => log::error!("Failed to parse plugin manifest {:?}: {}", manifest_path, e),
                            }
                        },
                        Err(e) => log::error!("Failed to read plugin manifest {:?}: {}", manifest_path, e),
                    }
                }
            }
        }
        log::info!("Finished loading installed plugins. Total plugins loaded: {}", self.plugins.len());
        Ok(())
    }

    pub async fn install_plugin(&mut self, source: &str) -> Result<String> {
        log::info!("Installing plugin from source: {}", source);
        // This is a simplified installation. In a real scenario, you'd:
        // 1. Download from URL or copy from local path.
        // 2. Validate the plugin (e.g., checksum, signature).
        // 3. Extract to a unique directory under `plugin_dir`.
        // 4. Read and validate `plugin.json`.
        // 5. Copy plugin assets/binaries.

        let temp_plugin_name = format!("plugin_{}", uuid::Uuid::new_v4().simple());
        let plugin_path = self.plugin_dir.join(&temp_plugin_name);
        fs::create_dir_all(&plugin_path).await?;

        // Simulate creating a manifest and a dummy entrypoint file
        let manifest = PluginManifest {
            name: temp_plugin_name.clone(),
            version: Version::new(0, 1, 0),
            description: format!("A simulated plugin from {}", source),
            plugin_type: PluginType::Wasm, // Default to WASM for simulation
            entrypoint: "plugin.wasm".to_string(),
            author: Some("v0".to_string()),
            homepage: None,
            capabilities: vec!["simulated_capability".to_string()],
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(plugin_path.join("plugin.json"), manifest_json).await?;
        fs::write(plugin_path.join("plugin.wasm"), b"// Dummy WASM content").await?; // Dummy WASM

        self.load_installed_plugins().await?; // Reload to pick up new plugin
        self.event_sender.send(PluginEvent::Installed {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
        }).await?;
        Ok(manifest.name)
    }

    pub async fn uninstall_plugin(&mut self, name: &str) -> Result<()> {
        if let Some(manifest) = self.plugins.remove(name) {
            let plugin_path = self.plugin_dir.join(&manifest.name);
            if plugin_path.exists() {
                fs::remove_dir_all(&plugin_path).await?;
                log::info!("Uninstalled plugin: {}", name);
                self.event_sender.send(PluginEvent::Uninstalled { name: name.to_string() }).await?;
                Ok(())
            } else {
                Err(anyhow!("Plugin directory not found for '{}'", name))
            }
        } else {
            Err(anyhow!("Plugin '{}' not found.", name))
        }
    }

    pub async fn list_plugins(&self) -> Vec<PluginManifest> {
        self.plugins.values().cloned().collect()
    }

    pub async fn get_plugin_manifest(&self, name: &str) -> Option<PluginManifest> {
        self.plugins.get(name).cloned()
    }

    pub async fn activate_plugin(&self, name: &str) -> Result<()> {
        if let Some(manifest) = self.plugins.get(name) {
            log::info!("Activating plugin: {}", name);
            let plugin_path = self.plugin_dir.join(&manifest.name).join(&manifest.entrypoint);
            match manifest.plugin_type {
                PluginType::Wasm => {
                    self.wasm_runtime.load_and_run_plugin(name.to_string(), plugin_path).await?;
                },
                PluginType::Lua => {
                    let code = fs::read_to_string(&plugin_path).await?;
                    self.lua_engine.execute_script(name.to_string(), code).await?;
                },
                PluginType::Native => {
                    log::warn!("Native plugin activation not directly managed by manager (compiled in): {}", name);
                }
            }
            self.event_sender.send(PluginEvent::Activated { name: name.to_string() }).await?;
            Ok(())
        } else {
            Err(anyhow!("Plugin '{}' not found.", name))
        }
    }

    pub async fn deactivate_plugin(&self, name: &str) -> Result<()> {
        log::info!("Deactivating plugin: {}", name);
        // For WASM/Lua, this might involve stopping the instance or cleaning up resources.
        // For this stub, it's mostly a log and event.
        self.event_sender.send(PluginEvent::Deactivated { name: name.to_string() }).await?;
        Ok(())
    }

    pub fn get_event_receiver(&mut self) -> mpsc::Receiver<PluginEvent> {
        self.event_sender.subscribe() // Assuming event_sender is a broadcast channel
    }
}
