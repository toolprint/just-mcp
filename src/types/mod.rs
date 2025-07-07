use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(skip)]
    pub source_hash: String,
    #[serde(skip, default = "std::time::SystemTime::now")]
    pub last_modified: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JustTask {
    pub name: String,
    pub body: String,
    pub parameters: Vec<Parameter>,
    pub dependencies: Vec<String>,
    pub comments: Vec<String>,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub default: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub context: ExecutionContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub environment: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub change_type: ChangeType,
    pub tool_name: String,
    pub timestamp: std::time::SystemTime,
}
