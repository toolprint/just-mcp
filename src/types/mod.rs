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
    // Internal name used for execution (includes full path)
    #[serde(skip)]
    pub internal_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JustTask {
    pub name: String,
    pub body: String,
    pub parameters: Vec<Parameter>,
    pub dependencies: Vec<String>,
    pub comments: Vec<String>,
    pub line_number: usize,
    /// Recipe group for organization (from [group('name')] attribute)
    pub group: Option<String>,
    /// Whether recipe is private (from [private] attribute)
    pub is_private: bool,
    /// Confirmation message if required (from [confirm] or [confirm("msg")] attribute)
    pub confirm_message: Option<String>,
    /// Recipe documentation (from [doc("text")] attribute)
    pub doc: Option<String>,
    /// Raw attribute information for advanced use cases
    #[cfg(feature = "ast-parser")]
    pub attributes: Vec<crate::parser::ast::queries::AttributeInfo>,
    #[cfg(not(feature = "ast-parser"))]
    pub attributes: Vec<String>, // Simplified representation when AST parser is not available
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
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
