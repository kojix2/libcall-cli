use anyhow::{anyhow, Result};
use mlua::prelude::*;
use std::cell::Cell;
use std::ffi::c_void;

pub struct LuaCallback {
    lua: Lua,
    // original signature retained for potential future reflective output
    signature: String,
    return_type: String,
    arg_types: Vec<String>,
}

fn lua_error(error: mlua::Error) -> anyhow::Error {
    anyhow!(error.to_string())
}

fn set_lua_function<F, A, R>(lua: &Lua, name: &str, function: F) -> Result<()>
where
    F: Fn(&Lua, A) -> mlua::Result<R> + mlua::MaybeSend + 'static,
    A: mlua::FromLuaMulti,
    R: mlua::IntoLuaMulti,
{
    let function = lua.create_function(function).map_err(lua_error)?;
    lua.globals().set(name, function).map_err(lua_error)
}

impl LuaCallback {
    pub fn new(signature: String, body: String) -> Result<Self> {
        let (return_type, args_part) = signature
            .split_once('(')
            .ok_or_else(|| anyhow!("Invalid callback signature"))?;

        let args_str = args_part.trim_end_matches(')').trim();

        let arg_types: Vec<String> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str
                .split(',')
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                .collect()
        };

        let lua = Lua::new();

        set_lua_function(&lua, "i8", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const i8) };
            Ok(value as i64)
        })?;

        set_lua_function(&lua, "u8", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const u8) };
            Ok(value as i64)
        })?;

        set_lua_function(&lua, "i16", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const i16) };
            Ok(value as i64)
        })?;

        set_lua_function(&lua, "u16", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const u16) };
            Ok(value as i64)
        })?;

        set_lua_function(&lua, "i32", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const i32) };
            Ok(value)
        })?;

        set_lua_function(&lua, "u32", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const u32) };
            Ok(value as i64)
        })?;

        set_lua_function(&lua, "i64", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const i64) };
            Ok(value)
        })?;

        set_lua_function(&lua, "f32", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const f32) };
            Ok(value as f64)
        })?;

        set_lua_function(&lua, "f64", |_, ptr: usize| {
            let value = unsafe { *(ptr as *const f64) };
            Ok(value)
        })?;

        set_lua_function(&lua, "cstr", |_, ptr: usize| {
            let s = unsafe {
                std::ffi::CStr::from_ptr(ptr as *const i8)
                    .to_string_lossy()
                    .into_owned()
            };
            Ok(s)
        })?;

        set_lua_function(&lua, "write_i32", |_, (ptr, val): (usize, i32)| {
            unsafe { *(ptr as *mut i32) = val };
            Ok(())
        })?;

        set_lua_function(&lua, "write_f64", |_, (ptr, val): (usize, f64)| {
            unsafe { *(ptr as *mut f64) = val };
            Ok(())
        })?;

        let arg_names: Vec<String> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str
                .split(',')
                .map(|s| {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    if parts.len() >= 2 {
                        parts[1].to_string()
                    } else {
                        // fallback name based on position will be substituted below
                        "".to_string()
                    }
                })
                .collect()
        };

        // Replace empty fallback names with deterministic argN identifiers
        let mut arg_names_final = Vec::new();
        for (i, name) in arg_names.iter().enumerate() {
            if name.is_empty() {
                arg_names_final.push(format!("arg{}", i));
            } else {
                arg_names_final.push(name.clone());
            }
        }

        let function_def = if arg_names_final.is_empty() {
            format!("function callback() {} end", body)
        } else {
            format!(
                "function callback({}) {} end",
                arg_names_final.join(", "),
                body
            )
        };

        lua.load(&function_def).exec().map_err(lua_error)?;

        Ok(LuaCallback {
            lua,
            signature: signature.to_string(),
            return_type: return_type.trim().to_string(),
            arg_types,
        })
    }

    pub fn call_i32(&self, args: &[*mut c_void]) -> Result<i32> {
        let func: LuaFunction = self.lua.globals().get("callback").map_err(lua_error)?;

        let lua_args: Vec<LuaValue> = args
            .iter()
            .map(|ptr| LuaValue::Integer(*ptr as i64))
            .collect();

        let result: i32 = func
            .call(LuaMultiValue::from_vec(lua_args))
            .map_err(lua_error)?;
        Ok(result)
    }

    pub fn get_c_wrapper(&self) -> Result<*mut c_void> {
        if self.has_i32_2ptr_signature() {
            Ok(callback_wrapper_i32_2ptr as *mut c_void)
        } else {
            Err(anyhow!(
                "Unsupported callback signature: {}. Only i32(ptr, ptr) callbacks are currently supported",
                self.signature
            ))
        }
    }

    fn has_i32_2ptr_signature(&self) -> bool {
        matches!(
            self.return_type.as_str(),
            "i32" | "int" | "int32" | "int32_t"
        ) && self.arg_types.len() == 2
            && self
                .arg_types
                .iter()
                .all(|arg_type| matches!(arg_type.as_str(), "ptr" | "pointer" | "voidp" | "void*"))
    }
}

thread_local! {
    static CURRENT_CALLBACK: Cell<Option<*const LuaCallback>> = const { Cell::new(None) };
}

pub unsafe extern "C" fn callback_wrapper_i32_2ptr(a: *mut c_void, b: *mut c_void) -> i32 {
    CURRENT_CALLBACK.with(|current| {
        if let Some(callback_ptr) = current.get() {
            let callback = unsafe { &*callback_ptr };
            callback.call_i32(&[a, b]).unwrap_or(0)
        } else {
            0
        }
    })
}

pub fn set_global_callback(callback: *const LuaCallback) {
    CURRENT_CALLBACK.with(|current| current.set(Some(callback)));
}

pub fn clear_global_callback() {
    CURRENT_CALLBACK.with(|current| current.set(None));
}
