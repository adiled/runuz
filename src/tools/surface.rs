//! The advertised runuz tool surface — the single source of truth the
//! runuz-hive mirrors on-the-fly.
//!
//! `runuz tools --json` emits one `ToolDef` per CLI subcommand, named
//! `runuz_<subcommand>`. The hive advertises these verbatim and
//! dispatches generically (`runuz <sub> --<key> <val>`), so growing
//! the CLI never requires a hive change: add a subcommand + a line
//! here, and the hive picks it up automatically.

use serde_json::{json, Value};

use crate::ToolDef;

/// One advertised CLI subcommand: its `runuz_<name>`, a description,
/// and the input-schema property keys (each maps to `--<kebab-key>`
/// on the CLI).
pub struct SurfaceTool {
    /// CLI subcommand (without the `runuz_` prefix).
    pub sub: &'static str,
    pub description: &'static str,
    /// Schema property keys in order. Hyphenated on the CLI: `_` -> `-`.
    pub props: &'static [&'static str],
    /// Properties that are required.
    pub required: &'static [&'static str],
    /// True when the subcommand takes a positional scope value
    /// (`word|phrase|sentence|paragraph`).
    pub positional: bool,
}

/// The complete advertised surface, in CLI order.
pub const SURFACE: &[SurfaceTool] = &[
    SurfaceTool {
        sub: "read",
        description: "Filesystem analysis: discover, study, and search. Works on any file — code returns a tree-sitter symbol outline; configs and docs return an anchor outline; extensionless files return content. Path auto-detection: file | directory | glob. Pick at most one modifier: symbol (exact, dot-nested), query (fuzzy on symbol NAMES), pattern (regex over CONTENT).",
        props: &["file_path", "symbol", "query", "pattern"],
        required: &["file_path"],
        positional: false,
    },
    SurfaceTool {
        sub: "create",
        description: "Create a new code file (fails if it exists). AST-grounded; the write is re-parsed and a syntax-error result aborts.",
        props: &["file_path", "new_source"],
        required: &["file_path"],
        positional: false,
    },
    SurfaceTool {
        sub: "replace",
        description: "Replace code — symbol-scoped, or whole-file when symbol is omitted. AST-grounded; every write is re-parsed and a syntax-error result aborts.",
        props: &["file_path", "symbol", "symbols", "new_source"],
        required: &["file_path"],
        positional: false,
    },
    SurfaceTool {
        sub: "insert_before",
        description: "Splice new_source immediately before the anchor symbol. AST-grounded; re-parsed on write.",
        props: &["file_path", "symbol", "new_source"],
        required: &["file_path", "symbol"],
        positional: false,
    },
    SurfaceTool {
        sub: "insert_after",
        description: "Splice new_source immediately after the anchor symbol. AST-grounded; re-parsed on write.",
        props: &["file_path", "symbol", "new_source"],
        required: &["file_path", "symbol"],
        positional: false,
    },
    SurfaceTool {
        sub: "delete",
        description: "Delete the anchor symbol's byte range (or a --symbols contiguous run). AST-grounded; re-parsed on write.",
        props: &["file_path", "symbol", "symbols"],
        required: &["file_path"],
        positional: false,
    },
    SurfaceTool {
        sub: "write",
        description: "Whole-file write, auto-routed by extension: code files go through the AST-grounded code author, non-code through linguistic-scope author.",
        props: &["file_path", "content"],
        required: &["file_path", "content"],
        positional: false,
    },
    SurfaceTool {
        sub: "word",
        description: "Linguistic-scope edit: single token swap, format-agnostic. Omit replace to delete the resolved scope.",
        props: &["file_path", "word", "replace"],
        required: &["file_path"],
        positional: true,
    },
    SurfaceTool {
        sub: "phrase",
        description: "Linguistic-scope edit: structural name or exact text. Omit replace to delete the resolved scope.",
        props: &["file_path", "phrase", "replace"],
        required: &["file_path"],
        positional: true,
    },
    SurfaceTool {
        sub: "sentence",
        description: "Linguistic-scope edit: the single line containing the scope text. Omit replace to delete the resolved scope.",
        props: &["file_path", "sentence", "replace"],
        required: &["file_path"],
        positional: true,
    },
    SurfaceTool {
        sub: "paragraph",
        description: "Linguistic-scope edit: the full block containing the scope text. Omit replace to delete the resolved scope.",
        props: &["file_path", "paragraph", "replace"],
        required: &["file_path"],
        positional: true,
    },
];

/// Build the advertised `ToolDef` list, one `runuz_<sub>` per CLI subcommand.
pub fn advertised_defs() -> Vec<ToolDef> {
    SURFACE.iter().map(|t| {
        let mut props = serde_json::Map::new();
        for p in t.props {
            props.insert(p.to_string(), json!({"type": "string"}));
        }
        ToolDef {
            name: format!("runuz_{}", t.sub),
            description: t.description.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": props,
                "required": t.required,
            }),
        }
    }).collect()
}

/// Emit the surface as a JSON array (for `runuz tools --json`).
pub fn surface_json() -> Value {
    json!(advertised_defs().iter().map(|d| json!({
        "name": d.name,
        "description": d.description,
        "inputSchema": d.input_schema,
    })).collect::<Vec<_>>())
}
