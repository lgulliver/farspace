//! TUI Components

pub mod battle_report;
pub mod dispatch;
pub mod footer;
pub mod header;
pub mod help;
pub mod log;
pub mod palette;

pub use battle_report::render_battle_reports;
pub use dispatch::render_dispatch;
pub use footer::render_footer;
pub use header::{derive_header_data, render_header, HeaderData};
pub use help::render_help;
pub use log::{render_log, EventLog, LogEntryKind};
pub use palette::{render_palette, PaletteCommand};
