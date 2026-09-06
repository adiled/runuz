//! runuz-hive — the runuz remote forager hive.
//!
//! Stands up a thrum-attached forager process that advertises hum's
//! filesystem tool surface (`humfs_read`, `humfs_do_code`,
//! `humfs_do_noncode`) and handles `chi:"tool-call"` tones humd
//! routes here. Pure forager: it translates `chi:"tool-call"` into
//! `runuz` CLI invocations — the actual file ops happen in the
//! installed `runuz` binary, not in-process.
//!
//! Self-contained: no hum internals. The wire machinery (Hid, bee
//! identity, serve_forager, XDG paths) is vendored under `wire/`.

use std::sync::Arc;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

mod dispatch;
mod wire;

use dispatch::RunuzDispatcher;
use wire::forager::serve_forager;

#[tokio::main]
async fn main() -> Result<()> {
    wire::paths::init();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("HUM_LOG_LEVEL")
                .unwrap_or_else(|_| EnvFilter::new("info,runuz_hive=trace")),
        )
        .init();

    let dispatcher = Arc::new(RunuzDispatcher::new());
    let advert = dispatch::advert();
    serve_forager(dispatcher, advert).await
}
