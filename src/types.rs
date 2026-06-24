use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Isize,
    Usize,
    F32,
    F64,
    Ptr,
    CStr,
    Void,
    Callback,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Type::I8 => "i8",
            Type::U8 => "u8",
            Type::I16 => "i16",
            Type::U16 => "u16",
            Type::I32 => "i32",
            Type::U32 => "u32",
            Type::I64 => "i64",
            Type::U64 => "u64",
            Type::Isize => "isize",
            Type::Usize => "usize",
            Type::F32 => "f32",
            Type::F64 => "f64",
            Type::Ptr => "ptr",
            Type::CStr => "cstr",
            Type::Void => "void",
            Type::Callback => "callback",
        };
        write!(f, "{}", s)
    }
}

impl Type {
    pub fn size(&self) -> usize {
        match self {
            Type::I8 | Type::U8 => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 => 4,
            Type::I64 | Type::U64 | Type::F64 => 8,
            Type::Isize | Type::Usize | Type::Ptr | Type::CStr | Type::Callback => {
                std::mem::size_of::<usize>()
            }
            Type::Void => 0,
        }
    }
}

pub fn parse_type_name(name: &str) -> Result<Type> {
    match name.to_lowercase().as_str() {
        "i8" | "char" | "int8" | "int8_t" => Ok(Type::I8),
        "u8" | "uchar" | "uint8" | "uint8_t" | "byte" => Ok(Type::U8),
        "i16" | "short" | "int16" | "int16_t" => Ok(Type::I16),
        "u16" | "ushort" | "uint16" | "uint16_t" => Ok(Type::U16),
        "i32" | "int" | "int32" | "int32_t" => Ok(Type::I32),
        "u32" | "uint" | "uint32" | "uint32_t" => Ok(Type::U32),
        "i64" | "long_long" | "int64" | "int64_t" => Ok(Type::I64),
        "u64" | "ulong_long" | "uint64" | "uint64_t" => Ok(Type::U64),
        "isize" | "ssize" | "ssize_t" | "long" => Ok(Type::Isize),
        "usize" | "size" | "size_t" | "ulong" => Ok(Type::Usize),
        "f32" | "float" => Ok(Type::F32),
        "f64" | "double" => Ok(Type::F64),
        "ptr" | "pointer" | "voidp" | "void*" => Ok(Type::Ptr),
        "cstr" | "string" | "str" | "char*" => Ok(Type::CStr),
        "void" => Ok(Type::Void),
        "callback" | "func" | "function" => Ok(Type::Callback),
        _ => Err(anyhow!("Unknown type: {}", name)),
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Isize(isize),
    Usize(usize),
    F32(f32),
    F64(f64),
    Ptr(*mut std::ffi::c_void),
    CStr(String),
    Array { elem_type: Type, values: Vec<Value> },
    Callback { signature: String, body: String },
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::I8(_) => Type::I8,
            Value::U8(_) => Type::U8,
            Value::I16(_) => Type::I16,
            Value::U16(_) => Type::U16,
            Value::I32(_) => Type::I32,
            Value::U32(_) => Type::U32,
            Value::I64(_) => Type::I64,
            Value::U64(_) => Type::U64,
            Value::Isize(_) => Type::Isize,
            Value::Usize(_) => Type::Usize,
            Value::F32(_) => Type::F32,
            Value::F64(_) => Type::F64,
            Value::Ptr(_) => Type::Ptr,
            Value::CStr(_) => Type::CStr,
            Value::Array { .. } => Type::Ptr,
            Value::Callback { .. } => Type::Callback,
        }
    }

    pub fn default_for_type(ty: Type) -> Self {
        match ty {
            Type::I8 => Value::I8(0),
            Type::U8 => Value::U8(0),
            Type::I16 => Value::I16(0),
            Type::U16 => Value::U16(0),
            Type::I32 => Value::I32(0),
            Type::U32 => Value::U32(0),
            Type::I64 => Value::I64(0),
            Type::U64 => Value::U64(0),
            Type::Isize => Value::Isize(0),
            Type::Usize => Value::Usize(0),
            Type::F32 => Value::F32(0.0),
            Type::F64 => Value::F64(0.0),
            Type::Ptr => Value::Ptr(std::ptr::null_mut()),
            Type::CStr => Value::CStr(String::new()),
            Type::Void => panic!("Cannot create default value for void type"),
            Type::Callback => panic!("Cannot create default value for callback type"),
        }
    }
}

pub fn infer_type(value: &str) -> Result<Type> {
    let value = value.trim();

    if value == "null" {
        return Ok(Type::Ptr);
    }

    if value == "true" || value == "false" {
        return Ok(Type::I32);
    }

    if let Ok(integer) = value.parse::<i64>() {
        if i32::try_from(integer).is_ok() {
            return Ok(Type::I32);
        }
        return Ok(Type::I64);
    }

    if value.parse::<u64>().is_ok() {
        return Ok(Type::U64);
    }

    if value.parse::<f64>().is_ok() {
        return Ok(Type::F64);
    }

    if value.starts_with('"') && value.ends_with('"') {
        return Ok(Type::CStr);
    }

    Ok(Type::CStr)
}

pub fn parse_value(value_str: &str, ty: Type) -> Result<Value> {
    let value_str = value_str.trim();

    match ty {
        Type::I8 => Ok(Value::I8(value_str.parse()?)),
        Type::U8 => Ok(Value::U8(value_str.parse()?)),
        Type::I16 => Ok(Value::I16(value_str.parse()?)),
        Type::U16 => Ok(Value::U16(value_str.parse()?)),
        Type::I32 => {
            if value_str == "true" {
                Ok(Value::I32(1))
            } else if value_str == "false" {
                Ok(Value::I32(0))
            } else {
                Ok(Value::I32(value_str.parse()?))
            }
        }
        Type::U32 => Ok(Value::U32(value_str.parse()?)),
        Type::I64 => Ok(Value::I64(value_str.parse()?)),
        Type::U64 => Ok(Value::U64(value_str.parse()?)),
        Type::Isize => Ok(Value::Isize(value_str.parse()?)),
        Type::Usize => Ok(Value::Usize(value_str.parse()?)),
        Type::F32 => Ok(Value::F32(value_str.parse()?)),
        Type::F64 => Ok(Value::F64(value_str.parse()?)),
        Type::Ptr => {
            if value_str == "null" {
                Ok(Value::Ptr(std::ptr::null_mut()))
            } else if let Some(hex) = value_str.strip_prefix("0x") {
                Ok(Value::Ptr(usize::from_str_radix(hex, 16)? as *mut _))
            } else {
                Ok(Value::Ptr(value_str.parse::<usize>()? as *mut _))
            }
        }
        Type::CStr => {
            let s = if value_str.starts_with('"') && value_str.ends_with('"') {
                value_str[1..value_str.len() - 1].to_string()
            } else {
                value_str.to_string()
            };
            Ok(Value::CStr(s))
        }
        Type::Void => Err(anyhow!("Cannot parse value for void type")),
        Type::Callback => Err(anyhow!("Cannot parse simple value for callback type")),
    }
}

pub fn parse_array_values(
    values_str: &str,
    elem_type: Type,
    expected_count: usize,
) -> Result<Vec<Value>> {
    let parts: Vec<&str> = values_str.split(',').collect();

    if parts.len() != expected_count {
        return Err(anyhow!(
            "Array length mismatch: expected {} elements, got {}",
            expected_count,
            parts.len()
        ));
    }

    parts
        .iter()
        .map(|part| parse_value(part.trim(), elem_type))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_type_uses_i32_for_small_integers() {
        assert_eq!(infer_type("42").unwrap(), Type::I32);
        assert_eq!(infer_type("-42").unwrap(), Type::I32);
    }

    #[test]
    fn infer_type_uses_i64_for_large_integers() {
        assert_eq!(infer_type("3000000000").unwrap(), Type::I64);
        assert_eq!(infer_type("-3000000000").unwrap(), Type::I64);
    }

    #[test]
    fn infer_type_uses_u64_for_unsigned_large_integers() {
        assert_eq!(infer_type("18446744073709551615").unwrap(), Type::U64);
    }

    #[test]
    fn infer_type_uses_f64_for_values_beyond_u64() {
        assert_eq!(infer_type("18446744073709551616").unwrap(), Type::F64);
    }

    #[test]
    fn parse_array_values_rejects_wrong_length() {
        let err = parse_array_values("1,2,3", Type::I32, 4).unwrap_err();
        assert!(err.to_string().contains("Array length mismatch"));
    }

    #[test]
    fn parse_array_values_parses_typed_values() {
        let values = parse_array_values("1,2", Type::I32, 2).unwrap();

        assert!(matches!(values.as_slice(), [Value::I32(1), Value::I32(2)]));
    }
}
