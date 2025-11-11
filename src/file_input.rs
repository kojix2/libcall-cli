use crate::parser::{Argument, CallSpec};
use crate::types::{parse_type_name, Type, Value};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCallSpec {
    pub library: String,
    pub function: String,
    pub args: Vec<FileArg>,
    #[serde(default)]
    pub returns: String,
}

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

pub fn load_yaml_file(path: &Path) -> Result<FileCallSpec> {
    let content = fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(|e| anyhow!("Failed to parse YAML file: {}", e))
}

pub fn load_json_file(path: &Path) -> Result<FileCallSpec> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| anyhow!("Failed to parse JSON file: {}", e))
}
