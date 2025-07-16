use wasmtime::*;
use tokio::sync::mpsc;
use std::collections::HashMap;
use crate::plugins::PluginEvent;

/// A WebAssembly (WASM) plugin runtime.
/// This allows loading and executing WASM modules as plugins.
pub struct WasmRuntime {
    name: String,
    wasm_path: String,
    engine: Engine,
    module: Option<Module>,
    linker: Linker<PluginHost>,
    store: Store<PluginHost>,
}

/// Host data available to WASM modules.
#[derive(Clone)]
pub struct PluginHost {
    pub event_sender: mpsc::UnboundedSender<PluginEvent>,
    pub plugin_name: String,
    // Other host data like access to file system, network, etc.
}

impl WasmRuntime {
    pub fn new(name: String, wasm_path: String) -> Self {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let store = Store::new(&engine, PluginHost {
            event_sender: mpsc::unbounded_channel().0, // Dummy sender for initial creation
            plugin_name: name.clone(),
        });

        // Define host functions that WASM modules can import
        // Example: `host_log(ptr, len)` for logging from WASM
        linker.func_wrap(
            "host",
            "log",
            |mut caller: Caller<'_, PluginHost>, ptr: i32, len: i32| {
                let (memory, data) = caller.data_and_store_mut();
                let memory = memory.get_export("memory").unwrap().into_memory().unwrap();
                let bytes = memory.data(&data).get(ptr as usize..(ptr + len) as usize).unwrap();
                let message = std::str::from_utf8(bytes).unwrap_or("[invalid utf8]");
                println!("[WASM Plugin: {}] {}", data.plugin_name, message);
            },
        ).unwrap();

        // Example: `host_send_event(event_type_ptr, event_type_len, data_ptr, data_len)`
        linker.func_wrap(
            "host",
            "send_event",
            |mut caller: Caller<'_, PluginHost>, event_type_ptr: i32, event_type_len: i32, data_ptr: i32, data_len: i32| {
                let (memory, data) = caller.data_and_store_mut();
                let memory = memory.get_export("memory").unwrap().into_memory().unwrap();
                
                let event_type_bytes = memory.data(&data).get(event_type_ptr as usize..(event_type_ptr + event_type_len) as usize).unwrap();
                let event_type = std::str::from_utf8(event_type_bytes).unwrap_or("[invalid utf8]").to_string();

                let data_bytes = memory.data(&data).get(data_ptr as usize..(data_ptr + data_len) as usize).unwrap();
                let data_str = std::str::from_utf8(data_bytes).unwrap_or("[invalid utf8]").to_string();

                let event = match event_type.as_str() {
                    "status_update" => PluginEvent::StatusUpdate(data.plugin_name.clone(), data_str),
                    "command_executed" => PluginEvent::CommandExecuted(data.plugin_name.clone(), data_str),
                    "data" => PluginEvent::Data(data.plugin_name.clone(), serde_json::Value::String(data_str)),
                    "error" => PluginEvent::Error(data.plugin_name.clone(), data_str),
                    _ => PluginEvent::Error(data.plugin_name.clone(), format!("Unknown event type from WASM: {}", event_type)),
                };
                let _ = data.event_sender.send(event);
            },
        ).unwrap();


        Self {
            name,
            wasm_path,
            engine,
            module: None,
            linker,
            store,
        }
    }
}

impl super::Plugin for WasmRuntime {
    fn name(&self) -> &str {
        &self.name
    }

    fn initialize(&mut self, event_sender: mpsc::UnboundedSender<PluginEvent>) -> Result<(), String> {
        // Update the event sender in the store's host data
        self.store.data_mut().event_sender = event_sender;

        let module = Module::from_file(&self.engine, &self.wasm_path)
            .map_err(|e| format!("Failed to load WASM module {}: {}", self.wasm_path, e))?;
        self.module = Some(module);
        println!("WASM plugin '{}' loaded module from: {}", self.name, self.wasm_path);
        Ok(())
    }

    async fn execute_function(&self, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let module = self.module.as_ref().ok_or("WASM module not loaded.")?;
        let mut store = self.store.clone(); // Clone store for each execution if it holds mutable state
        let instance = self.linker.instantiate(&mut store, module)
            .map_err(|e| format!("Failed to instantiate WASM module: {}", e))?;

        let func = instance.get_typed_func::<(i32, i32), (i32, i32)>(&mut store, function_name)
            .map_err(|e| format!("Failed to get WASM function '{}': {}", function_name, e))?;

        // Serialize args to JSON string and write to WASM memory
        let args_str = serde_json::to_string(&args).map_err(|e| format!("Failed to serialize args: {}", e))?;
        let args_bytes = args_str.as_bytes();

        let memory = instance.get_memory(&mut store, "memory").ok_or("WASM module must export 'memory'")?;
        let alloc_func = instance.get_typed_func::<i32, i32>(&mut store, "allocate")
            .map_err(|e| format!("WASM module must export 'allocate' function: {}", e))?;
        let dealloc_func = instance.get_typed_func::<(i32, i32), ()>(&mut store, "deallocate")
            .map_err(|e| format!("WASM module must export 'deallocate' function: {}", e))?;

        let args_ptr = alloc_func.call(&mut store, args_bytes.len() as i32)
            .map_err(|e| format!("Failed to allocate memory in WASM: {}", e))?;
        
        memory.write(&mut store, args_ptr as usize, args_bytes)
            .map_err(|e| format!("Failed to write args to WASM memory: {}", e))?;

        // Call the WASM function
        let (result_ptr, result_len) = func.call(&mut store, (args_ptr, args_bytes.len() as i32))
            .map_err(|e| format!("Failed to call WASM function '{}': {}", function_name, e))?;

        // Read result from WASM memory
        let result_bytes = memory.data(&store).get(result_ptr as usize..(result_ptr + result_len) as usize)
            .ok_or("Failed to read result from WASM memory: Invalid pointer or length")?;
        let result_str = std::str::from_utf8(result_bytes)
            .map_err(|e| format!("Failed to decode WASM result as UTF-8: {}", e))?;
        
        // Deallocate memory in WASM
        dealloc_func.call(&mut store, (result_ptr, result_len))
            .map_err(|e| format!("Failed to deallocate memory in WASM: {}", e))?;

        serde_json::from_str(result_str).map_err(|e| format!("Failed to parse WASM result as JSON: {}", e))
    }

    fn start_background_tasks(&self, _event_sender: mpsc::UnboundedSender<PluginEvent>) {
        // WASM plugins might define their own background tasks,
        // or we could expose a way for them to register Rust-side tasks.
        println!("WASM plugin '{}' has no explicit Rust-side background tasks registered.", self.name);
    }
}

pub fn init() {
    println!("plugins/wasm_runtime module loaded");
}
