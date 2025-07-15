use wasmtime::{Engine, Module, Store, Instance, Func, Caller, Linker};
use std::collections::HashMap;
use serde_json::Value;
use crate::plugins::{Plugin, PluginRuntime, Permission};

pub struct WasmRuntime {
    engine: Engine,
    store: Store<WasmState>,
    linker: Linker<WasmState>,
    instances: HashMap<String, Instance>,
}

#[derive(Default)]
struct WasmState {
    plugin_permissions: HashMap<String, Vec<Permission>>,
    shared_data: HashMap<String, Value>,
}

impl WasmRuntime {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, WasmState::default());
        let mut linker = Linker::new(&engine);

        // Define host functions
        linker.func_wrap("env", "log", |caller: Caller<'_, WasmState>, ptr: i32, len: i32| {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let message = String::from_utf8_lossy(&data[ptr as usize..(ptr + len) as usize]);
            println!("[WASM Plugin]: {}", message);
        })?;

        linker.func_wrap("env", "get_terminal_output", |mut caller: Caller<'_, WasmState>| -> i32 {
            // Return pointer to terminal output data
            // This is a simplified implementation
            0
        })?;

        linker.func_wrap("env", "execute_command", |caller: Caller<'_, WasmState>, cmd_ptr: i32, cmd_len: i32| -> i32 {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let data = memory.data(&caller);
            let command = String::from_utf8_lossy(&data[cmd_ptr as usize..(cmd_ptr + cmd_len) as usize]);
            
            // Check permissions before executing
            // This would integrate with the actual command execution system
            println!("[WASM Plugin] Executing: {}", command);
            0 // Success
        })?;

        Ok(Self {
            engine,
            store,
            linker,
            instances: HashMap::new(),
        })
    }

    fn check_permissions(&self, plugin_id: &str, required_permission: &Permission) -> bool {
        if let Some(permissions) = self.store.data().plugin_permissions.get(plugin_id) {
            permissions.iter().any(|p| std::mem::discriminant(p) == std::mem::discriminant(required_permission))
        } else {
            false
        }
    }
}

impl PluginRuntime for WasmRuntime {
    fn load_plugin(&mut self, plugin: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        let wasm_bytes = std::fs::read(&plugin.install_path)?;
        let module = Module::new(&self.engine, wasm_bytes)?;
        let instance = self.linker.instantiate(&mut self.store, &module)?;
        
        // Store plugin permissions
        self.store.data_mut().plugin_permissions.insert(
            plugin.id.clone(), 
            plugin.permissions.clone()
        );
        
        self.instances.insert(plugin.id.clone(), instance);
        Ok(())
    }

    fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.instances.remove(plugin_id);
        self.store.data_mut().plugin_permissions.remove(plugin_id);
        Ok(())
    }

    fn execute_function(&mut self, plugin_id: &str, function: &str, args: &[Value]) -> Result<Value, Box<dyn std::error::Error>> {
        if let Some(instance) = self.instances.get(plugin_id) {
            let func = instance.get_typed_func::<(), i32>(&mut self.store, function)?;
            let result = func.call(&mut self.store, ())?;
            Ok(Value::Number(result.into()))
        } else {
            Err("Plugin not found".into())
        }
    }

    fn list_functions(&self, plugin_id: &str) -> Vec<String> {
        if let Some(instance) = self.instances.get(plugin_id) {
            instance.exports(&self.store)
                .filter_map(|(name, export)| {
                    if export.into_func().is_some() {
                        Some(name.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
