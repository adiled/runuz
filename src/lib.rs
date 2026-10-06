pub mod ast;
mod io;
pub mod tools;

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub title: Option<String>,
    pub metadata: Option<Value>,
    pub is_error: bool,
}

impl ToolResult {
    pub fn text(s: impl Into<String>) -> Self {
        Self { output: s.into(), title: None, metadata: None, is_error: false }
    }
    pub fn error(s: impl Into<String>) -> Self {
        Self { output: s.into(), title: None, metadata: None, is_error: true }
    }
}