//! RunuzDispatcher — the remote-hive tool registry + dispatch.
//!
//! Translates `chi:"tool-call"` tones into `runuz` CLI invocations.
//! The hive does NO file ops in-process: it shells out to the
//! installed `runuz` binary (`~/.local/bin/runuz`, or `RUNUZ_BIN`)
//! and parses the `--json` result back into a [`ToolResult`]. This is
//! the "remote hive that uses the cli tool to do file ops" design:
//! runuz is the executable, the hive is the thrum bridge.

use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use crate::wire::{ForagerAdvert, ToolDef, ToolDispatcher, ToolResult};

pub struct RunuzDispatcher {
    runuz_bin: PathBuf,
}

impl RunuzDispatcher {
    pub fn new() -> Self {
        Self { runuz_bin: runuz_bin_path() }
    }
}

fn runuz_bin_path() -> PathBuf {
    std::env::var_os("RUNUZ_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/bin/runuz")
        })
}

#[async_trait]
impl ToolDispatcher for RunuzDispatcher {
    fn tool_defs(&self) -> Vec<ToolDef> {
        vec![read_def(), do_code_def(), do_noncode_def()]
    }

    async fn dispatch(&self, tone: Value) -> ToolResult {
        let tool_name = tone.get("toolName").and_then(Value::as_str).unwrap_or("");
        let args = tone.get("args").cloned().unwrap_or(Value::Null);
        match tool_name {
            "humfs_read"       => run_read(args).await,
            "humfs_do_code"    => run_do_code(args).await,
            "humfs_do_noncode" => run_do_noncode(args).await,
            other => ToolResult::error(format!("runuz: unknown toolName {other:?}")),
        }
    }
}

pub fn advert() -> ForagerAdvert {
    ForagerAdvert {
        hive: "humfs".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        source: Some("https://github.com/adiled/runuz".into()),
        provides: vec!["fs".into()],
    }
}

// ── tool defs (same contract as humfs, advertised verbatim) ──────────

fn read_def() -> ToolDef {
    ToolDef {
        name: "humfs_read".into(),
        description: "Filesystem analysis: discover, study, and search. Works on any file — code returns a tree-sitter symbol outline; configs and docs return an anchor outline; extensionless files return content. Path auto-detection: file | directory | glob. Pick at most one modifier: symbol (exact, dot-nested), query (fuzzy on symbol NAMES), pattern (regex over CONTENT). The tool decides framing.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" },
                "symbol":    { "type": "string" },
                "query":     { "type": "string" },
                "pattern":   { "type": "string" },
            },
            "required": ["file_path"],
        }),
    }
}

fn do_code_def() -> ToolDef {
    ToolDef {
        name: "humfs_do_code".into(),
        description: "Author code — AST-grounded, symbol-scoped. Operations: create | replace | insert_before | insert_after | delete. The top-of-file import block is addressable as the synthetic 'imports' symbol. Sub-symbol walks compose with dots. Every write is re-parsed; a syntax-error result aborts the write. Non-code files route to humfs_do_noncode.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path":  { "type": "string" },
                "operation":  { "type": "string" },
                "symbol":     { "type": "string" },
                "new_source": { "type": "string" },
            },
            "required": ["file_path"],
        }),
    }
}

fn do_noncode_def() -> ToolDef {
    ToolDef {
        name: "humfs_do_noncode".into(),
        description: "Author non-code files using linguistic scope. Four scopes (pass exactly one): word (token swap), phrase (structural name or exact text), sentence (whole line), paragraph (full block). Omit 'replace' to delete the scope; no scope param creates/overwrites the whole file. Code files route to humfs_do_code.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" },
                "word":      { "type": "string" },
                "phrase":    { "type": "string" },
                "sentence":  { "type": "string" },
                "paragraph": { "type": "string" },
                "replace":   { "type": "string" },
            },
            "required": ["file_path"],
        }),
    }
}

// ── shelling out to runuz ─────────────────────────────────────────────

async fn runuz(cli: &[String]) -> ToolResult {
    let bin = runuz_bin_path();
    let mut cmd = Command::new(&bin);
    cmd.args(cli).arg("--json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let out = match cmd.output().await {
        Ok(o) => o,
        Err(e) => return ToolResult::error(format!("failed to spawn runuz {}: {e}", bin.display())),
    };
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return ToolResult::error(format!("runuz exited {}: {}", out.status, err.trim()));
    }
    let parsed: Value = match serde_json::from_slice(&out.stdout) {
        Ok(v) => v,
        Err(e) => return ToolResult::error(format!("runuz --json parse failed: {e}")),
    };
    ToolResult {
        output: parsed.get("output").and_then(Value::as_str).unwrap_or("").to_string(),
        title: parsed.get("title").and_then(Value::as_str).map(str::to_string),
        metadata: parsed.get("metadata").cloned(),
        is_error: parsed.get("is_error").and_then(Value::as_bool).unwrap_or(false),
    }
}

fn req_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

async fn run_read(args: Value) -> ToolResult {
    let fp = match req_str(&args, "file_path") {
        Some(s) => s,
        None => return ToolResult::error("file_path required"),
    };
    let mut cli = vec!["read".to_string(), "--file-path".to_string(), fp];
    for (k, flag) in [("symbol", "--symbol"), ("query", "--query"), ("pattern", "--pattern")] {
        if let Some(v) = req_str(&args, k) {
            cli.push(flag.to_string());
            cli.push(v);
        }
    }
    runuz(&cli).await
}

async fn run_do_code(args: Value) -> ToolResult {
    let fp = match req_str(&args, "file_path") {
        Some(s) => s,
        None => return ToolResult::error("file_path required"),
    };
    let op = req_str(&args, "operation").unwrap_or_else(|| "replace".to_string());
    let mut cli = vec![op, "--file-path".to_string(), fp];
    if let Some(s) = req_str(&args, "symbol") {
        cli.push("--symbol".to_string());
        cli.push(s);
    }
    if let Some(ns) = req_str(&args, "new_source") {
        cli.push("--new-source".to_string());
        cli.push(ns);
    }
    runuz(&cli).await
}

async fn run_do_noncode(args: Value) -> ToolResult {
    let fp = match req_str(&args, "file_path") {
        Some(s) => s,
        None => return ToolResult::error("file_path required"),
    };
    let replace = req_str(&args, "replace");

    // Exactly one of word/phrase/sentence/paragraph is the scope.
    let scope = ["word", "phrase", "sentence", "paragraph"]
        .iter()
        .find_map(|s| req_str(&args, s).map(|v| (s.to_string(), v)));

    let mut cli: Vec<String> = vec![];
    match scope {
        Some((kind, text)) => {
            cli.push(kind);
            cli.push(text);
        }
        None => {
            // No scope param => create/overwrite the whole file.
            // runuz has no bare whole-file op; map to `create` (new)
            // or `replace` whole-file (existing) by probing existence.
            let exists = std::path::Path::new(&fp).exists();
            cli.push(if exists { "replace".to_string() } else { "create".to_string() });
        }
    }
    cli.push("--file-path".to_string());
    cli.push(fp);
    if let Some(r) = replace {
        cli.push("--replace".to_string());
        cli.push(r);
    }
    runuz(&cli).await
}
