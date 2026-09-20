//! RunuzDispatcher — the runuz-hive tool registry + dispatch.
//!
//! The hive is a **zero-maintenance mirror** of the runuz CLI: it does
//! no file ops in-process, and it hardcodes no tool list. Instead it
//! shells out to the installed `runuz` binary (`~/.cargo/bin/runuz`, or
//! `RUNUZ_BIN`) and:
//!
//! - **Discovers** the surface on-the-fly via `runuz tools --json`,
//!   which emits one `ToolDef` per CLI subcommand named `runuz_<sub>`.
//!   Those defs are advertised verbatim.
//! - **Dispatches** generically: `toolName` is `runuz_<sub>`, so it
//!   maps each schema key present in `args` to `--<kebab-key>` and
//!   shells out. Scope tools (`runuz_word` etc.) pass the scope value
//!   as the positional.
//!
//! Because the CLI is the single source of truth, growing the CLI never
//! requires a hive change — the hive mirrors whatever `runuz tools`
//! reports.

use std::path::PathBuf;
use std::sync::OnceLock;

use async_trait::async_trait;
use hum_nest::{ToolDef, ToolDispatcher, ToolResult};
use serde_json::{json, Value};
use tokio::process::Command;

/// Cached advertised surface, loaded once from `runuz tools --json`.
static DEFS: OnceLock<Vec<ToolDef>> = OnceLock::new();

pub struct RunuzDispatcher;

impl RunuzDispatcher {
    pub fn new() -> Self {
        Self
    }
}

fn runuz_bin_path() -> PathBuf {
    std::env::var_os("RUNUZ_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".cargo/bin/runuz")
        })
}

/// Fetch the advertised surface from the CLI. On any failure, fall back
/// to an empty list (humd simply sees no tools until the CLI is fixed).
fn load_defs() -> Vec<ToolDef> {
    let bin = runuz_bin_path();
    let out = match std::process::Command::new(&bin)
        .arg("tools").arg("--json")
        .output() {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(bin = %bin.display(), "failed to spawn runuz tools: {e}");
            return vec![];
        }
    };
    if !out.status.success() {
        tracing::warn!(bin = %bin.display(), "runuz tools exited {}", out.status);
        return vec![];
    }
    match serde_json::from_slice::<Value>(&out.stdout) {
        Ok(v) => {
            let arr = v.as_array().cloned().unwrap_or_default();
            let defs: Vec<ToolDef> = arr.iter().filter_map(|t| {
                let name = t.get("name").and_then(Value::as_str)?.to_string();
                let description = t.get("description").and_then(Value::as_str).unwrap_or("").to_string();
                let schema = t.get("inputSchema").cloned().unwrap_or(json!({"type":"object"}));
                Some(ToolDef { name, description, input_schema: schema })
            }).collect();
            tracing::info!(count = defs.len(), "advertised runuz surface");
            defs
        }
        Err(e) => {
            tracing::warn!("runuz tools --json parse failed: {e}");
            vec![]
        }
    }
}

#[async_trait]
impl ToolDispatcher for RunuzDispatcher {
    fn tool_defs(&self) -> Vec<ToolDef> {
        DEFS.get_or_init(load_defs).clone()
    }

    async fn dispatch(&self, tone: Value) -> ToolResult {
        let tool_name = tone.get("toolName").and_then(Value::as_str).unwrap_or("").to_string();
        let args = tone.get("args").cloned().unwrap_or(Value::Null);
        runuz_dispatch(&tool_name, args).await
    }
}

/// Generic dispatch: `runuz_<sub>` -> `runuz <sub> --<kebab-key> <val>`.
async fn runuz_dispatch(tool_name: &str, args: Value) -> ToolResult {
    let Some(sub) = tool_name.strip_prefix("runuz_") else {
        return ToolResult::error(format!("runuz: unknown toolName {tool_name:?}"));
    };
    let Some(args_obj) = args.as_object() else {
        return ToolResult::error(format!("runuz: args must be an object"));
    };

    let mut cli: Vec<String> = vec![sub.to_string()];
    let scope_keys = ["word", "phrase", "sentence", "paragraph"];

    // Scope tools pass their scope value as the first positional.
    if scope_keys.contains(&sub) {
        if let Some(text) = args_obj.get(sub).and_then(Value::as_str) {
            cli.push(text.to_string());
        }
    }

    // Every other schema key maps to --<kebab-key> <val>.
    for (key, val) in args_obj {
        if key == sub {
            continue; // scope value already handled as positional
        }
        if let Some(s) = val.as_str() {
            cli.push(format!("--{}", key.replace('_', "-")));
            cli.push(s.to_string());
        }
    }
    cli.push("--json".to_string());

    runuz(&cli).await
}

async fn runuz(cli: &[String]) -> ToolResult {
    let bin = runuz_bin_path();
    let mut cmd = Command::new(&bin);
    cmd.args(cli)
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
