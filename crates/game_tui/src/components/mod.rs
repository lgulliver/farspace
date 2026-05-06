//! TUI Components

pub mod footer;
pub mod header;
pub mod help;
pub mod log;
pub mod palette;

pub use footer::render_footer;
pub use header::render_header;
pub use help::render_help;
pub use log::EventLog;
pub use palette::render_palette;
