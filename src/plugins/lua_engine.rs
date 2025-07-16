use mlua::{Lua, Table, Function, Value as LuaValue, Result as LuaResult};
use std::collections::HashMap;
use serde_json::Value;
use crate::plugins::{Plugin, PluginRuntime, Permission};

pub struct LuaEngine {
    lua: Lua,
    loaded_plugins: HashMap<String, String>, // plugin_id -> script_content
    plugin_permissions: HashMap<String, Vec<Permission>>,
}

impl LuaEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();
        
        // Set up sandbox environment
        let globals = lua.globals();
        
        // Remove dangerous functions
        globals.set("os", lua.create_table()?)?;
        globals.set("io", lua.create_table()?)?;
        globals.set("debug", lua.create_table()?)?;
        
        // Add safe terminal API
        let terminal_api = lua.create_table()?;
        
        terminal_api.set("log", lua.create_function(|_, message: String| {
            println!("[Lua Plugin]: {}", message);
            Ok(())
        })?)?;
        
        terminal_api.set("get_current_directory", lua.create_function(|_, ()| {
            Ok(std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string())
        })?)?;
        
        terminal_api.set("execute_command", lua.create_function(|_, command: String| {
            // This would integrate with the actual command execution system
            println!("[Lua Plugin] Executing: {}", command);
            Ok("Command executed")
        })?)?;
        
        globals.set("terminal", terminal_api)?;
        
        // Add JSON utilities
        let json_api = lua.create_table()?;
        
        json_api.set("encode", lua.create_function(|_, value: LuaValue| {
            let json_value = lua_value_to_json(value)?;
            Ok(serde_json::to_string(&json_value).unwrap_or_default())
        })?)?;
        
        json_api.set("decode", lua.create_function(|lua, json_str: String| {
            match serde_json::from_str::<Value>(&json_str) {
                Ok(value) => json_to_lua_value(lua, value),
                Err(e) => Err(mlua::Error::RuntimeError(e.to_string())),
            }
        })?)?;
        
        globals.set("json", json_api)?;
        
        Ok(Self {
            lua,
            loaded_plugins: HashMap::new(),
            plugin_permissions: HashMap::new(),
        })
    }

    fn check_permissions(&self, plugin_id: &str, required_permission: &Permission) -> bool {
        if let Some(permissions) = self.plugin_permissions.get(plugin_id) {
            permissions.iter().any(|p| std::mem::discriminant(p) == std::mem::discriminant(required_permission))
        } else {
            false
        }
    }
}

impl PluginRuntime for LuaEngine {
    fn load_plugin(&mut self, plugin: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        let script_content = std::fs::read_to_string(&plugin.install_path)?;
        
        // Store plugin permissions
        self.plugin_permissions.insert(plugin.id.clone(), plugin.permissions.clone());
        
        // Execute the plugin script to load it
        self.lua.load(&script_content).exec()?;
        
        self.loaded_plugins.insert(plugin.id.clone(), script_content);
        Ok(())
    }

    fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.loaded_plugins.remove(plugin_id);
        self.plugin_permissions.remove(plugin_id);
        Ok(())
    }

    fn execute_function(&mut self, plugin_id: &str, function: &str, args: &[Value]) -> Result<Value, Box<dyn std::error::Error>> {
        if !self.loaded_plugins.contains_key(plugin_id) {
            return Err("Plugin not loaded".into());
        }

        let globals = self.lua.globals();
        let func: Function = globals.get(function)?;
        
        // Convert JSON args to Lua values
        let lua_args: LuaResult<Vec<LuaValue>> = args.iter()
            .map(|arg| json_to_lua_value(&self.lua, arg.clone()))
            .collect();
        
        let result = func.call::<_, LuaValue>(lua_args?)?;
        Ok(lua_value_to_json(result)?)
    }

    fn list_functions(&self, plugin_id: &str) -> Vec<String> {
        if !self.loaded_plugins.contains_key(plugin_id) {
            return Vec::new();
        }

        let globals = self.lua.globals();
        let mut functions = Vec::new();
        
        // This is a simplified implementation - in practice, you'd want to
        // track which functions belong to which plugin
        for pair in globals.pairs::<String, LuaValue>() {
            if let Ok((name, value)) = pair {
                if matches!(value, LuaValue::Function(_)) {
                    functions.push(name);
                }
            }
        }
        
        functions
    }
}

fn lua_value_to_json(value: LuaValue) -> LuaResult<Value> {
    match value {
        LuaValue::Nil => Ok(Value::Null),
        LuaValue::Boolean(b) => Ok(Value::Bool(b)),
        LuaValue::Integer(i) => Ok(Value::Number(i.into())),
        LuaValue::Number(n) => Ok(Value::Number(serde_json::Number::from_f64(n).unwrap_or_default())),
        LuaValue::String(s) => Ok(Value::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => {
            let mut map = serde_json::Map::new();
            for pair in t.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;
                if let LuaValue::String(key) = k {
                    map.insert(key.to_str()?.to_string(), lua_value_to_json(v)?);
                }
            }
            Ok(Value::Object(map))
        }
        _ => Ok(Value::Null),
    }
}

fn json_to_lua_value(lua: &Lua, value: Value) -> LuaResult<LuaValue> {
    match value {
        Value::Null => Ok(LuaValue::Nil),
        Value::Bool(b) => Ok(LuaValue::Boolean(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Ok(LuaValue::Nil)
            }
        }
        Value::String(s) => Ok(LuaValue::String(lua.create_string(&s)?)),
        Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.into_iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, item)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        Value::Object(obj) => {
            let table = lua.create_table()?;
            for (key, value) in obj {
                table.set(key, json_to_lua_value(lua, value)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_engine_creation() {
        let engine = LuaEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_json_conversion() {
        let lua = Lua::new();
        let json_val = Value::String("test".to_string());
        let lua_val = json_to_lua_value(&lua, json_val.clone()).unwrap();
        let converted_back = lua_value_to_json(lua_val).unwrap();
        assert_eq!(json_val, converted_back);
    }
}
