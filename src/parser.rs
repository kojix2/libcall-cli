use crate::types::{infer_type, parse_array_values, parse_type_name, parse_value, Type, Value};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref OUTPUT_SCALAR_RE: Regex =
        Regex::new(r"^@(i8|u8|i16|u16|i32|u32|i64|u64|isize|usize|f32|f64|cstr|ptr)$").unwrap();
    static ref OUTPUT_ARRAY_RE: Regex = Regex::new(
        r"^@(\d+)(i8|u8|i16|u16|i32|u32|i64|u64|isize|usize|f32|f64|cstr|ptr)(?::(.+))?$"
    )
    .unwrap();
    static ref INPUT_ARRAY_RE: Regex =
        Regex::new(r"^(\d+)(i8|u8|i16|u16|i32|u32|i64|u64|isize|usize|f32|f64|cstr|ptr):(.+)$")
            .unwrap();
    static ref TYPED_VALUE_RE: Regex =
        Regex::new(r"^(i8|u8|i16|u16|i32|u32|i64|u64|isize|usize|f32|f64|cstr|ptr|callback):(.+)$")
            .unwrap();
    static ref RETURN_TYPE_RE: Regex =
        Regex::new(r"^:(i8|u8|i16|u16|i32|u32|i64|u64|isize|usize|f32|f64|cstr|ptr|void)$")
            .unwrap();
    static ref CALLBACK_RE: Regex = Regex::new(r"^'(.+)\((.*)\)\{(.*)\}'$").unwrap();
    static ref CALLBACK_LIKE_RE: Regex = Regex::new(r"^'.*\(.*\).*\{.*\}.*'$").unwrap();
}

#[derive(Debug)]
pub struct Argument {
    pub value: Value,
    pub is_output: bool,
}

#[derive(Debug)]
pub struct CallSpec {
    pub function: String,
    pub args: Vec<Argument>,
    pub return_type: Type,
}

pub fn parse_type_token(token: &str) -> Result<Argument> {
    if let Some(captures) = OUTPUT_SCALAR_RE.captures(token) {
        let elem_type = parse_type_name(&captures[1])?;

        return Ok(Argument {
            value: Value::Array {
                elem_type,
                values: vec![Value::default_for_type(elem_type)],
                is_output: true,
            },
            is_output: true,
        });
    }

    if let Some(captures) = OUTPUT_ARRAY_RE.captures(token) {
        let count: usize = captures[1].parse()?;
        let elem_type = parse_type_name(&captures[2])?;

        let values = if let Some(init) = captures.get(3) {
            parse_array_values(init.as_str(), elem_type, count)?
        } else {
            vec![Value::default_for_type(elem_type); count]
        };

        return Ok(Argument {
            value: Value::Array {
                elem_type,
                values,
                is_output: true,
            },
            is_output: true,
        });
    }

    if let Some(captures) = INPUT_ARRAY_RE.captures(token) {
        let count: usize = captures[1].parse()?;
        let elem_type = parse_type_name(&captures[2])?;
        let values = parse_array_values(&captures[3], elem_type, count)?;

        return Ok(Argument {
            value: Value::Array {
                elem_type,
                values,
                is_output: false,
            },
            is_output: false,
        });
    }

    if let Some(captures) = CALLBACK_RE.captures(token) {
        let ret_type_str = captures[1].trim();
        let args_str = captures[2].trim();
        let body = captures[3].trim();

        let signature = format!("{}({})", ret_type_str, args_str);

        return Ok(Argument {
            value: Value::Callback {
                signature: signature.clone(),
                body: body.to_string(),
            },
            is_output: false,
        });
    }

    if CALLBACK_LIKE_RE.is_match(token) {
        return Err(anyhow!("Invalid callback specification: {}", token));
    }

    if let Some(captures) = TYPED_VALUE_RE.captures(token) {
        let type_name = &captures[1];
        let value_str = &captures[2];
        let ty = parse_type_name(type_name)?;
        let value = parse_value(value_str, ty)?;

        return Ok(Argument {
            value,
            is_output: false,
        });
    }

    let ty = infer_type(token)?;
    let value = parse_value(token, ty)?;

    Ok(Argument {
        value,
        is_output: false,
    })
}

pub fn parse_return_type(token: &str) -> Result<Type> {
    if let Some(captures) = RETURN_TYPE_RE.captures(token) {
        let type_name = &captures[1];
        parse_type_name(type_name)
    } else {
        Err(anyhow!("Invalid return type specification: {}", token))
    }
}

pub fn parse_call_spec(function: String, arg_tokens: Vec<String>) -> Result<CallSpec> {
    let mut args = Vec::new();
    let mut return_type = Type::Void;

    for token in arg_tokens {
        if token.starts_with(':') {
            return_type = parse_return_type(&token)?;
        } else {
            args.push(parse_type_token(&token)?);
        }
    }

    Ok(CallSpec {
        function,
        args,
        return_type,
    })
}
