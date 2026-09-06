//! `serve_forager` — wraps a tool-call dispatcher into a standalone
//! process that handshakes with humd via thrum. Ported from hum's
//! `nest-common::forager`, minus the ensemble/thrum-core deps.
//!
//! Wire contract:
//! - **Hello**: announce as `bee:["forager"]`, advertise `tools` +
//!   `toolNames` + `hive` (kind) + `provides`.
//! - **Tool-call in**: humd routes `chi:"tool-call"` tones whose
//!   `toolName` matches an advertised tool. The forager runs the
//!   tool and emits `chi:"tool-result"` keyed by the same `callId`.
//! - **Cancel**: `chi:"cancel"` (with `callId`) signals abort — the
//!   hive currently runs tools via a subprocess; cancellation is
//!   best-effort (dropping the child).

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use super::identity::load_or_mint_bee_key;
use super::hid::HidPrefix;
use super::paths;

pub const THRUM_VERSION: &str = "0.7.0";

/// One advertised tool.
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

/// Forager-side tool dispatcher — the seam humd's tool-call router calls.
#[async_trait]
pub trait ToolDispatcher: Send + Sync + 'static {
    fn tool_defs(&self) -> Vec<ToolDef>;
    async fn dispatch(&self, tone: Value) -> ToolResult;
}

/// What the forager advertises on hello.
#[derive(Debug, Clone)]
pub struct ForagerAdvert {
    pub hive: String,
    pub version: String,
    pub source: Option<String>,
    pub provides: Vec<String>,
}

fn default_socket_path() -> std::path::PathBuf {
    paths::thrum_sock_resolved()
}

/// Run the forager service loop. Blocks until shutdown; reconnects on drop.
pub async fn serve_forager<D: ToolDispatcher + 'static>(
    dispatcher: Arc<D>,
    advert: ForagerAdvert,
) -> Result<()> {
    let path = default_socket_path();
    loop {
        match dial_and_serve(&path, dispatcher.clone(), &advert).await {
            Ok(()) => tracing::trace!("serve_forager: clean exit, reconnecting"),
            Err(e) => tracing::warn!(err = %e, "serve_forager: connection failed, retrying"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn dial_and_serve<D: ToolDispatcher + 'static>(
    path: &Path,
    dispatcher: Arc<D>,
    advert: &ForagerAdvert,
) -> Result<()> {
    tracing::info!(socket = %path.display(), hive = %advert.hive, "forager.connecting");
    let stream = UnixStream::connect(path).await
        .with_context(|| format!("connect to thrum at {}", path.display()))?;
    let (read_half, write_half) = stream.into_split();
    let write_half = Arc::new(Mutex::new(write_half));

    let bee_key = load_or_mint_bee_key(&advert.hive, HidPrefix::Fbee)
        .with_context(|| format!("load/mint fbee key for hive {}", advert.hive))?;

    let defs = dispatcher.tool_defs();
    let tool_names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    let tools_value: Vec<Value> = defs.iter().map(|d| json!({
        "name": d.name,
        "description": d.description,
        "inputSchema": d.input_schema,
    })).collect();

    let hello = json!({
        "chi": "hello",
        "bee": ["forager"],
        "hid": bee_key.hid.to_hex(),
        "from": bee_key.hid.to_hex(),
        "hive": &advert.hive,
        "version": &advert.version,
        "protoVersion": THRUM_VERSION,
        "tools": tools_value,
        "toolNames": tool_names,
        "provides": &advert.provides,
        "chis": ["hello", "tool-call", "tool-result", "cancel", "breath", "echo"],
        "source": advert.source.clone().unwrap_or_default(),
    });
    write_half.lock().await.write_all(format!("{}\n", hello).as_bytes()).await?;
    tracing::info!(
        hive = %advert.hive,
        hid = %bee_key.hid.short(),
        tools = ?tool_names,
        provides = ?advert.provides,
        "forager.hello.sent"
    );

    let mut reader = BufReader::new(read_half).lines();
    while let Some(line) = reader.next_line().await? {
        if line.is_empty() { continue; }
        let tone: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => { tracing::trace!(err = %e, "forager.parse.skip"); continue; }
        };
        let chi = tone.get("chi").and_then(Value::as_str).unwrap_or("");
        match chi {
            "tool-call" => {
                let dispatcher = dispatcher.clone();
                let write_half = write_half.clone();
                tokio::spawn(async move {
                    let sid = tone.get("sid").and_then(Value::as_str).unwrap_or("").to_string();
                    let call_id = tone.get("callId").and_then(Value::as_str).unwrap_or("").to_string();
                    let tool_name = tone.get("toolName").and_then(Value::as_str).unwrap_or("").to_string();
                    let result = dispatcher.dispatch(tone).await;
                    let body = json!({
                        "chi": "tool-result",
                        "sid": sid,
                        "callId": call_id,
                        "toolName": tool_name,
                        "output": result.output,
                        "isError": result.is_error,
                        "title": result.title,
                        "metadata": result.metadata,
                    });
                    let line = format!("{}\n", body);
                    let _ = write_half.lock().await.write_all(line.as_bytes()).await;
                });
            }
            "breath" | "echo" | "" => {}
            other => tracing::trace!(chi = other, "forager.unknown.chi"),
        }
    }
    Ok(())
}
