//! The advertised runuz tool surface - the single source of truth the
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
}

/// The complete advertised surface, in CLI order.
pub const SURFACE: &[SurfaceTool] = &[
    SurfaceTool {
        sub: "read",
        description: "Filesystem analysis: discover, study, and search. Works on any file. Code returns a STRUCTURE-ONLY symbol outline (kinds, names, line ranges — no source dumps) with per-symbol callees as a `→ name, name` suffix; configs and docs return an anchor outline; extensionless files return content. Path auto-detection: file | directory | glob. Pick at most one modifier: symbol (any address — NAME or rung 'token|slot|statement|block <text>' — outputs just that range, prefixed by its calls: line for code names), query (fuzzy on symbol NAMES — structure listing only), pattern (regex over CONTENT).",
        props: &["file_path", "symbol", "query", "pattern"],
        required: &["file_path"],
    },
    SurfaceTool {
        sub: "create",
        description: "Create a new file (fails if it exists). Code files are re-parsed after write and a syntax-error result aborts; configs and docs are written as-is.",
        props: &["file_path", "new_source"],
        required: &["file_path"],
    },
    SurfaceTool {
        sub: "replace",
        description: "Replace an addressed unit or the whole file (address omitted = whole file). --symbol takes ANY address: a NAME (code: fn/Class.method/imports, sub-walks, re-parsed on write) or a shape rung 'token <t>' | 'slot <s>' | 'statement <s>' | 'block <s>' (shape edits on code are re-parsed too; on text they validate structure — JSON stays JSON). --symbols A,B = one atomic multi-edit over code names. --new-source '' deletes the resolved range.",
        props: &["file_path", "symbol", "symbols", "new_source"],
        required: &["file_path"],
    },
    SurfaceTool {
        sub: "insert_before",
        description: "Splice new_source immediately before the anchor. The anchor is --symbol: a NAME (code) or a shape rung 'token|slot|statement|block <t>' (any file). Code writes are re-parsed; text writes validate structure.",
        props: &["file_path", "symbol", "new_source"],
        required: &["file_path", "symbol"],
    },
    SurfaceTool {
        sub: "insert_after",
        description: "Splice new_source immediately after the anchor. The anchor is --symbol: a NAME (code) or a shape rung 'token|slot|statement|block <t>' (any file). Code writes are re-parsed; text writes validate structure.",
        props: &["file_path", "symbol", "new_source"],
        required: &["file_path", "symbol"],
    },
    SurfaceTool {
        sub: "delete",
        description: "Delete an addressed unit (--symbol NAME or --symbols A,B for a contiguous run, blank-line hygiene; or --symbol 'rung <t>' — any file).",
        props: &["file_path", "symbol", "symbols"],
        required: &["file_path"],
    },
    SurfaceTool {
        sub: "rename",
        description: "Rename a symbol (function, struct, etc.) across a code file — finds all word-boundary occurrences of the name and replaces them. On text files, renames the word everywhere.",
        props: &["file_path", "symbol", "new_name"],
        required: &["file_path", "symbol", "new_name"],
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