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
        provides: vec!["fs".into()],
    };
    serve_forager(dispatcher, advert).await
}
