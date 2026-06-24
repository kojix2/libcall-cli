use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct FileCallSpec {
    pub library: String,
    pub function: String,
    pub args: Vec<FileArg>,
    #[serde(default)]
    pub returns: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct FileArg {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub arg_type: String,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub count: Option<usize>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[allow(dead_code)]
pub fn load_yaml_file(path: &Path) -> Result<FileCallSpec> {
    let content = fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(|e| anyhow!("Failed to parse YAML file: {}", e))
}

#[allow(dead_code)]
pub fn load_json_file(path: &Path) -> Result<FileCallSpec> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| anyhow!("Failed to parse JSON file: {}", e))
}

pub fn load_spec_file(path: &Path) -> Result<FileCallSpec> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("yaml") | Some("yml") => load_yaml_file(path),
        Some("json") => load_json_file(path),
        Some(ext) => Err(anyhow!(
            "Unsupported spec file extension: .{} (expected .json, .yaml, or .yml)",
            ext
        )),
        None => Err(anyhow!(
            "Spec file must have an extension: .json, .yaml, or .yml"
        )),
    }
}

pub fn spec_arg_tokens(spec: &FileCallSpec) -> Result<Vec<String>> {
    let mut tokens = spec
        .args
        .iter()
        .map(file_arg_to_token)
        .collect::<Result<Vec<_>>>()?;

    if !spec.returns.trim().is_empty() {
        if spec.returns.starts_with(':') {
            tokens.push(spec.returns.clone());
        } else {
            tokens.push(format!(":{}", spec.returns));
        }
    }

    Ok(tokens)
}

fn file_arg_to_token(arg: &FileArg) -> Result<String> {
    let arg_type = arg.arg_type.trim();
    let mode = arg.mode.as_deref().unwrap_or("input");

    if arg_type == "callback" || mode == "callback" {
        let signature = arg
            .signature
            .as_deref()
            .ok_or_else(|| anyhow!("Callback argument requires signature"))?;
        let body = arg
            .body
            .as_deref()
            .ok_or_else(|| anyhow!("Callback argument requires body"))?;
        return Ok(format!("'{}{{ {} }}'", signature.trim(), body.trim()));
    }

    match mode {
        "output" => Ok(match arg.count {
            Some(count) => format!("@{}{}", count, arg_type),
            None => format!("@{}", arg_type),
        }),
        "inout" => {
            let value = arg_value(arg)?;
            Ok(match arg.count {
                Some(count) => format!("@{}{}:{}", count, arg_type, value),
                None => {
                    return Err(anyhow!(
                        "Scalar inout arguments are not supported; use output or an array count"
                    ));
                }
            })
        }
        "input" => {
            let value = arg_value(arg)?;
            Ok(match arg.count {
                Some(count) => format!("{}{}:{}", count, arg_type, value),
                None => format!("{}:{}", arg_type, value),
            })
        }
        other => Err(anyhow!("Unknown argument mode: {}", other)),
    }
}

fn arg_value(arg: &FileArg) -> Result<String> {
    let value = arg
        .value
        .as_ref()
        .ok_or_else(|| anyhow!("Input argument requires value"))?;

    if let Some(values) = value.as_array() {
        return Ok(values
            .iter()
            .map(json_value_to_token)
            .collect::<Result<Vec<_>>>()?
            .join(","));
    }

    json_value_to_token(value)
}

fn json_value_to_token(value: &serde_json::Value) -> Result<String> {
    Ok(match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::String(v) => v.clone(),
        serde_json::Value::Array(_) => {
            return Err(anyhow!("Nested arrays are not supported in spec arguments"));
        }
        serde_json::Value::Object(_) => {
            return Err(anyhow!("Objects are not supported in spec argument values"));
        }
    })
}
