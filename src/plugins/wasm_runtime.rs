use wasmtime::*;
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use log::{info, error, warn};
use crate::plugins::PluginEvent;
use std::fs;

/// A WebAssembly (WASM) plugin runtime.
/// This allows loading and executing WASM modules as plugins.
pub struct WasmRuntime {
    engine: Engine,
    event_sender: mpsc::Sender<WasmPluginEvent>,
    module_instances: HashMap<String, Instance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmPluginEvent {
    Loaded { name: String },
    Executed { name: String, result: String },
    Error { name: String, error: String },
    // Add events for API calls from WASM to Rust
}

impl WasmRuntime {
    pub fn new(event_sender: mpsc::Sender<WasmPluginEvent>) -> Self {
        let mut config = WasmtimeConfig::new();
        config.debug_info(true); // Enable debug info for better error messages
        let engine = Engine::new(&config).expect("Failed to create Wasmtime engine");
        Self { engine, event_sender, module_instances: HashMap::new() }
    }

    pub async fn init(&self) -> Result<(), String> {
        info!("WASM runtime initialized.");
        Ok(())
    }

    /// Loads a WASM module and runs its main function (if exported).
    pub async fn load_and_run_plugin(&self, name: String, path: PathBuf) -> Result<(), String> {
        info!("Loading and running WASM plugin: {} from {:?}", name, path);
        let sender_clone = self.event_sender.clone();
        let engine_clone = self.engine.clone();

        tokio::spawn(async move {
            match fs::read(&path).await {
                Ok(wasm_bytes) => {
                    match Module::from_binary(&engine_clone, &wasm_bytes) {
                        Ok(module) => {
                            let mut store = Store::new(&engine_clone, ()); // Store can hold host state
                            match Instance::new(&mut store, &module, &[]) {
                                Ok(instance) => {
                                    // Attempt to find and call a "_start" or "main" function
                                    if let Some(func) = instance.get_typed_func::<(), (), _>(&mut store, "_start") {
                                        match func.call(&mut store, ()).await {
                                            Ok(_) => {
                                                info!("WASM plugin '{}' executed successfully.", name);
                                                let _ = sender_clone.send(WasmPluginEvent::Executed { name, result: "Success".to_string() }).await;
                                            },
                                            Err(e) => {
                                                let error_str = format!("WASM plugin '{}' execution error: {}", name, e);
                                                error!("{}", error_str);
                                                let _ = sender_clone.send(WasmPluginEvent::Error { name, error: error_str }).await;
                                            }
                                        }
                                    } else {
                                        warn!("WASM plugin '{}' has no '_start' function. Loaded but not executed.", name);
                                        let _ = sender_clone.send(WasmPluginEvent::Loaded { name }).await;
                                    }
                                },
                                Err(e) => {
                                    let error_str = format!("Failed to instantiate WASM module '{}': {}", name, e);
                                    error!("{}", error_str);
                                    let _ = sender_clone.send(WasmPluginEvent::Error { name, error: error_str }).await;
                                }
                            }
                        },
                        Err(e) => {
                            let error_str = format!("Failed to compile WASM module '{}': {}", name, e);
                            error!("{}", error_str);
                            let _ = sender_clone.send(WasmPluginEvent::Error { name, error: error_str }).await;
                        }
                    }
                },
                Err(e) => {
                    let error_str = format!("Failed to read WASM file {:?}: {}", path, e);
                    error!("{}", error_str);
                    let _ = sender_clone.send(WasmPluginEvent::Error { name, error: error_str }).await;
                }
            }
        });
        Ok(())
    }

    /// Exposes Rust functions to the WASM environment (WASI or custom imports).
    pub async fn expose_host_functions(&self) -> Result<(), String> {
        // This is where you would define `Linker` and add host functions
        // that WASM modules can import and call.
        info!("WASM runtime exposing host functions (stub).");
        Ok(())
    }
}

impl super::Plugin for WasmRuntime {
    fn name(&self) -> &str {
        "WasmRuntime"
    }

    fn initialize(&mut self, event_sender: mpsc::UnboundedSender<PluginEvent>) -> Result<(), String> {
        // Update the event sender in the store's host data
        self.event_sender = event_sender.into();
        info!("WASM runtime module initialized.");
        Ok(())
    }

    async fn execute_function(&self, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        // Placeholder for execute_function implementation
        Err("Not implemented".to_string())
    }

    fn start_background_tasks(&self, _event_sender: mpsc::UnboundedSender<PluginEvent>) {
        // WASM plugins might define their own background tasks,
        // or we could expose a way for them to register Rust-side tasks.
        info!("WASM runtime has no explicit Rust-side background tasks registered.");
    }
}

pub fn init() {
    info!("WASM runtime module loaded");
}
