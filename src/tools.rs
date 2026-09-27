//! The runuz tool surface - `read`, `code`, `text`.
//! (bash intentionally dropped - not part of the standalone CLI.)

pub(crate) mod code;
pub(crate) mod text;
pub(crate) mod read;
pub mod surface;

// Re-export so the CLI can address them uniformly.
pub use code::run as code;
pub use text::run as text;
pub use read::run as read;

// The tool contract lives in the crate root; surface it here too so
// the binary and any hive wrapper can address results uniformly.
pub use crate::ToolDef;
pub use crate::ToolResult;
