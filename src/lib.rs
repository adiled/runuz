//! runuz — a standalone filesystem tool: `do_code`, `do_nocode`,
//! `do_read` for any project on Earth. AST-grounded via tree-sitter;
//! zero hum dependencies. The CLI binary (`runuz`) wraps these ops.

pub mod ast;
pub mod tools;

// ── tool contract ────────────────────────────────────────────────────────
// Replicates humfs's `nest_common::ToolDef`/`ToolResult` so the tool
// bodies stay drop-in identical. A hive re-integration can reuse this
// same contract over the thrum protocol.

use serde_json::Value;

/// One advertised tool. Description + schema land in a registry /
/// MCP client's tool pickers.
#[derive(Debug, Clone, Default)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Outcome of one tool dispatch.
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