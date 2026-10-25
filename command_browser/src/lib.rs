//! command_browser — offline fuzzy-search command index backed by SQLite FTS5.

pub mod index;
pub mod ingest;
pub mod overlay;

pub use index::{CommandEntry, Index};
pub use overlay::OverlayState;
