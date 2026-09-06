//! Self-contained thrum wire machinery for the runuz hive.
//!
//! Vendored from hum (nest-common + ensemble + hum-paths) but
//! stripped of every hum-internal dependency: no nest, config, drone,
//! ids, codegen, mcp, or full ensemble. The hive only needs:
//!   - paths.rs     — XDG socket + bee-key resolution (hum-paths stand-in)
//!   - hid.rs       — content-addressable Hid (ensemble stand-in)
//!   - identity.rs  — persistent fbee ed25519 key (nest-common stand-in)
//!   - forager.rs   — serve_forager loop + ToolDef/ToolResult contract

pub mod forager;
pub mod hid;
pub mod identity;
pub mod paths;

pub use forager::{ForagerAdvert, ToolDef, ToolDispatcher, ToolResult};
