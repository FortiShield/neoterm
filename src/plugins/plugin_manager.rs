use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::plugins::{Plugin, PluginConfig, PluginEvent};
use crate::plugins::lua_engine::LuaEngine;
use crate::plugins::wasm_runtime::WasmRuntime;

pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
    event_sender: mpsc::UnboundedSender<PluginEvent>,
}

impl PluginManager {
    pub fn new(event_sender: mpsc::UnboundedSender<PluginEvent>) -> Self {
        Self {
            plugins: HashMap::new(),
            event_sender,
        }
    }

    /// Registers a new plugin.
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<(), String> {
        let name = plugin.name().to_string();
        if self.plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' already registered.", name));
        }
        self.plugins.insert(name, plugin);
        Ok(())
    }

    /// Loads plugins based on configuration.
    pub fn load_plugins(&mut self, configs: Vec<PluginConfig>) {
        for config in configs {
            match config {
                PluginConfig::Lua { script_path, enabled } => {
                    if enabled {
                        let name = format!("LuaPlugin_{}", uuid::Uuid::new_v4());
                        let mut lua_engine = LuaEngine::new(name.clone(), script_path.clone());
                        match lua_engine.load_script() {
                            Ok(_) => {
                                match lua_engine.setup_host_api(self.event_sender.clone()) {
                                    Ok(_) => {
                                        println!("Loaded Lua plugin: {}", name);
                                        self.register_plugin(Box::new(LuaPluginWrapper { engine: lua_engine })).unwrap();
                                    },
                                    Err(e) => eprintln!("Failed to setup Lua host API for {}: {}", name, e),
                                }
                            },
                            Err(e) => eprintln!("Failed to load Lua script {}: {}", script_path, e),
                        }
                    }
                },
                PluginConfig::Wasm { wasm_path, enabled } => {
                    if enabled {
                        let name = format!("WasmPlugin_{}", uuid::Uuid::new_v4());
                        let mut wasm_runtime = WasmRuntime::new(name.clone(), wasm_path.clone());
                        match wasm_runtime.initialize(self.event_sender.clone()) {
                            Ok(_) => {
                                println!("Loaded WASM plugin: {}", name);
                                self.register_plugin(Box::new(wasm_runtime)).unwrap();
                            },
                            Err(e) => eprintln!("Failed to load WASM module {}: {}", wasm_path, e),
                        }
                    }
                },
            }
        }
    }

    /// Initializes all registered plugins.
    pub fn initialize_all(&mut self) {
        for (name, plugin) in self.plugins.iter_mut() {
            match plugin.initialize(self.event_sender.clone()) {
                Ok(_) => println!("Plugin '{}' initialized successfully.", name),
                Err(e) => eprintln!("Failed to initialize plugin '{}': {}", name, e),
            }
        }
    }

    /// Starts background tasks for all registered plugins.
    pub fn start_all_background_tasks(&self) {
        for plugin in self.plugins.values() {
            plugin.start_background_tasks(self.event_sender.clone());
        }
    }

    /// Executes a function in a specific plugin.
    pub async fn execute_plugin_function(&self, plugin_name: &str, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        if let Some(plugin) = self.plugins.get(plugin_name) {
            plugin.execute_function(function_name, args).await
        } else {
            Err(format!("Plugin '{}' not found.", plugin_name))
        }
    }
}

// A wrapper to make LuaEngine conform to the Plugin trait
struct LuaPluginWrapper {
    engine: LuaEngine,
}

impl Plugin for LuaPluginWrapper {
    fn name(&self) -> &str {
        &self.engine.name
    }

    fn initialize(&mut self, event_sender: mpsc::UnboundedSender<PluginEvent>) -> Result<(), String> {
        // LuaEngine's initialize is done in load_plugins, but this is for trait consistency
        self.engine.setup_host_api(event_sender)
            .map_err(|e| format!("Lua setup error: {}", e))
    }

    async fn execute_function(&self, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        self.engine.call_lua_function(function_name, args).await
    }

    fn start_background_tasks(&self, _event_sender: mpsc::UnboundedSender<PluginEvent>) {
        // Lua plugins might define their own background tasks within Lua,
        // or we could expose a way for them to register Rust-side tasks.
        println!("Lua plugin '{}' has no explicit Rust-side background tasks registered.", self.name());
    }
}

pub fn init() {
    println!("plugins/plugin_manager module loaded");
}
