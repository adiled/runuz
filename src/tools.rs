//! The runuz tool surface — `read`, `do_code`, `do_noncode`.
//! (bash intentionally dropped — not part of the standalone CLI.)

pub(crate) mod do_code;
pub(crate) mod do_noncode;
pub(crate) mod read;

// Re-export so the CLI can address them uniformly.
pub use do_code::run as do_code;
pub use do_noncode::run as do_noncode;
pub use read::run as read;

// The tool contract lives in the crate root; surface it here too so
// the binary and any hive wrapper can address results uniformly.
pub use crate::ToolDef;
pub use crate::ToolResult;
