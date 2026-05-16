//! TUI Components

pub mod footer;
pub mod header;
pub mod help;
pub mod log;
pub mod palette;

pub use footer::render_footer;
pub use header::{derive_header_data, render_header, HeaderData};
pub use help::render_help;
pub use log::{render_log, EventLog, LogEntryKind};
pub use palette::{render_palette, PaletteCommand};
