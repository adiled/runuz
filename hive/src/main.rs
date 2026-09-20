//! runuz-hive — the runuz formal remote forager hive for hum.
//!
//! Stands up a thrum-attached forager process that advertises the
//! runuz filesystem tool surface (`humfs_read`, `humfs_do_code`,
//! `humfs_do_noncode`) and handles `chi:"tool-call"` tones humd
//! routes here. Pure forager: it translates `chi:"tool-call"` into
//! `runuz` CLI invocations — the actual file ops happen in the
//! installed `runuz` binary, not in-process.
//!
//! This is a *formal remote hive*: it depends on hum's reusable hive
//! kernel (`hum-nest`) via git addressing — the same building blocks
//! hum's own humfs hive uses — rather than vendoring self-contained
//! stand-ins or a daemon tree. It advertises its own kind (`runuz`)
//! and a canonical persisted `fbee_<hex>` hid, so humd dedupes it
//! across reconnects.

use std::sync::Arc;

use anyhow::Result;
use hum_nest::{serve_forager, ForagerAdvert};
use tracing_subscriber::EnvFilter;

mod dispatch;

use dispatch::RunuzDispatcher;

#[tokio::main]
async fn main() -> Result<()> {
    hum_paths::init();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("HUM_LOG_LEVEL")
                .unwrap_or_else(|_| EnvFilter::new("info,runuz_hive=trace")),
        )
        .init();

    let dispatcher = Arc::new(RunuzDispatcher::new());
    let advert = ForagerAdvert {
        hive: "runuz".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        source: Some("https://github.com/adiled/runuz/tree/main/hive".into()),
        // Hive-level capability claim: runuz owns the fs surface for
        // whichever humd it attaches to, same as humfs.
        provides: vec!["fs".into()],
    };
    serve_forager(dispatcher, advert).await
}
