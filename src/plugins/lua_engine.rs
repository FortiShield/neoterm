use mlua::{Lua, Result as LuaResult, Value as LuaValue, Table, Function};
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::plugins::PluginEvent;

/// A Lua plugin engine that can execute Lua scripts and interact with the host application.
pub struct LuaEngine {
    name: String,
    lua: Arc<Lua>, // Use Arc for thread-safe sharing if needed
    script_path: String,
}

impl LuaEngine {
    pub fn new(name: String, script_path: String) -> Self {
        let lua = Lua::new();
        Self {
            name,
            lua: Arc::new(lua),
            script_path,
        }
    }

    /// Loads and executes the Lua script.
    pub fn load_script(&self) -> LuaResult<()> {
        let script_content = std::fs::read_to_string(&self.script_path)
            .map_err(|e| mlua::Error::external(format!("Failed to read Lua script: {}", e)))?;
        self.lua.load(&script_content).exec()?;
        Ok(())
    }

    /// Sets up global functions/variables that Lua scripts can call to interact with the host.
    pub fn setup_host_api(&self, event_sender: mpsc::UnboundedSender<PluginEvent>) -> LuaResult<()> {
        let lua = &self.lua;
        let plugin_name = self.name.clone();

        // Example: `host.log(message)` function
        let log_fn = lua.create_function(move |_, message: String| {
            println!("[Lua Plugin: {}] {}", plugin_name, message);
            Ok(())
        })?;
        lua.globals().set("log", log_fn)?;

        // Example: `host.send_event(event_type, data)` function
        let event_sender_clone = event_sender.clone();
        let plugin_name_clone = self.name.clone();
        let send_event_fn = lua.create_function(move |_, (event_type, data): (String, LuaValue)| {
            let event = match event_type.as_str() {
                "status_update" => PluginEvent::StatusUpdate(plugin_name_clone.clone(), data.to_string()),
                "command_executed" => PluginEvent::CommandExecuted(plugin_name_clone.clone(), data.to_string()),
                "data" => PluginEvent::Data(plugin_name_clone.clone(), serde_json::to_value(data.to_string()).unwrap_or_default()),
                "error" => PluginEvent::Error(plugin_name_clone.clone(), data.to_string()),
                _ => PluginEvent::Error(plugin_name_clone.clone(), format!("Unknown event type: {}", event_type)),
            };
            let _ = event_sender_clone.send(event);
            Ok(())
        })?;
        lua.globals().set("send_event", send_event_fn)?;

        Ok(())
    }

    /// Calls a Lua function from Rust.
    pub async fn call_lua_function(&self, function_name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let lua = self.lua.clone();
        let func_name = function_name.to_string();
        let args_lua = serde_json_to_lua(&lua, args)?;

        tokio::task::spawn_blocking(move || {
            let func: Function = lua.globals().get(&func_name)
                .map_err(|e| format!("Lua function '{}' not found: {}", func_name, e))?;
            let result: LuaValue = func.call(args_lua)
                .map_err(|e| format!("Error calling Lua function '{}': {}", func_name, e))?;
            lua_to_serde_json(&result)
        }).await.map_err(|e| format!("Lua function call panicked: {}", e))?
    }
}

// Helper to convert serde_json::Value to mlua::Value
fn serde_json_to_lua(lua: &Lua, value: serde_json::Value) -> LuaResult<LuaValue> {
    match value {
        serde_json::Value::Null => Ok(LuaValue::Nil),
        serde_json::Value::Bool(b) => Ok(LuaValue::Boolean(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Err(mlua::Error::external(format!("Unsupported number format: {}", n)))
            }
        },
        serde_json::Value::String(s) => Ok(LuaValue::String(lua.create_string(&s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.into_iter().enumerate() {
                table.set(i + 1, serde_json_to_lua(lua, item)?)?; // Lua arrays are 1-indexed
            }
            Ok(LuaValue::Table(table))
        },
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (key, val) in obj {
                table.set(key, serde_json_to_lua(lua, val)?)?;
            }
            Ok(LuaValue::Table(table))
        },
    }
}

// Helper to convert mlua::Value to serde_json::Value
fn lua_to_serde_json(value: &LuaValue) -> Result<serde_json::Value, String> {
    match value {
        LuaValue::Nil => Ok(serde_json::Value::Null),
        LuaValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        LuaValue::LightUserData(_) => Err("LightUserData not supported".to_string()),
        LuaValue::Integer(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
        LuaValue::Number(f) => Ok(serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or_default())),
        LuaValue::String(s) => Ok(serde_json::Value::String(s.to_str().map_err(|e| e.to_string())?.to_string())),
        LuaValue::Table(table) => {
            let mut map = serde_json::Map::new();
            let mut is_array = true;
            let mut array_elements = Vec::new();

            for pair in table.clone().pairs::<LuaValue, LuaValue>() {
                let (key, val) = pair.map_err(|e| e.to_string())?;
                if let LuaValue::Integer(idx) = key {
                    if idx as usize != array_elements.len() + 1 {
                        is_array = false;
                    }
                    array_elements.push(lua_to_serde_json(&val)?);
                } else {
                    is_array = false;
                    map.insert(lua_to_serde_json(&key)?.as_str().unwrap_or_default().to_string(), lua_to_serde_json(&val)?);
                }
            }

            if is_array {
                Ok(serde_json::Value::Array(array_elements))
            } else {
                Ok(serde_json::Value::Object(map))
            }
        },
        LuaValue::Function(_) => Err("Function not supported".to_string()),
        LuaValue::Thread(_) => Err("Thread not supported".to_string()),
        LuaValue::UserData(_) => Err("UserData not supported".to_string()),
        LuaValue::Error(e) => Err(format!("Lua error: {}", e)),
    }
}

pub fn init() {
    println!("plugins/lua_engine module loaded");
}
