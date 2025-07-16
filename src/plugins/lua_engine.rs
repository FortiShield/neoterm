use anyhow::Result;
use mlua::{Lua, StdLib, Value, Table};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

/// A Lua plugin engine that can execute Lua scripts and interact with the host application.
pub struct LuaEngine {
    lua: Lua,
    event_sender: mpsc::Sender<LuaPluginEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LuaPluginEvent {
    Loaded { name: String },
    Executed { name: String, result: String },
    Error { name: String, error: String },
    // Add events for API calls from Lua to Rust
}

impl LuaEngine {
    pub fn new(event_sender: mpsc::Sender<LuaPluginEvent>) -> Self {
        let lua = Lua::new();
        Self { lua, event_sender }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("Lua engine initialized.");
        self.lua.load_from_std_lib(StdLib::ALL)?;
        self.expose_rust_api().await?;
        Ok(())
    }

    /// Exposes Rust functions and objects to the Lua environment.
    async fn expose_rust_api(&self) -> Result<()> {
        let globals = self.lua.globals();

        // Example: Expose a `term` table with a `print` function
        let term_table = self.lua.create_table()?;
        term_table.set("print", self.lua.create_function(|_, text: String| {
            log::info!("[Lua] {}", text);
            Ok(())
        })?)?;
        globals.set("term", term_table)?;

        // Example: Expose a `fs` table with a `read_file` function
        let fs_table = self.lua.create_table()?;
        fs_table.set("read_file", self.lua.create_async_function(|_, path: String| async move {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => Ok(content),
                Err(e) => Err(mlua::Error::external(format!("Failed to read file: {}", e))),
            }
        })?)?;
        globals.set("fs", fs_table)?;

        log::info!("Rust API exposed to Lua.");
        Ok(())
    }

    /// Loads and executes a Lua script.
    pub async fn execute_script(&self, name: String, code: String) -> Result<()> {
        log::info!("Executing Lua script: {}", name);
        let sender_clone = self.event_sender.clone();
        let lua_clone = self.lua.clone(); // Clone Lua for async execution

        tokio::spawn(async move {
            match lua_clone.load(&code).eval::<Value>() {
                Ok(result) => {
                    let result_str = format!("{:?}", result);
                    log::info!("Lua script '{}' executed successfully. Result: {}", name, result_str);
                    let _ = sender_clone.send(LuaPluginEvent::Executed { name, result: result_str }).await;
                },
                Err(e) => {
                    let error_str = format!("Lua script '{}' error: {}", name, e);
                    log::error!("{}", error_str);
                    let _ = sender_clone.send(LuaPluginEvent::Error { name, error: error_str }).await;
                }
            }
        });
        Ok(())
    }

    /// Calls a Lua function from Rust.
    pub async fn call_lua_function(&self, function_name: &str, args: Value) -> Result<Value> {
        log::info!("Calling Lua function '{}' with args: {:?}", function_name, args);
        let globals = self.lua.globals();
        let func: mlua::Function = globals.get(function_name)?;
        let result = func.call_async(args).await?;
        Ok(result)
    }
}

pub fn init() {
    log::info!("Lua engine module initialized.");
}
