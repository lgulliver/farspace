//! TUI Components

pub mod battle_report;
pub mod chrome;
pub mod dispatch;
pub mod emblem;
pub mod footer;
pub mod header;
pub mod help;
pub mod log;
pub mod meter;
pub mod palette;

pub use battle_report::render_battle_reports;
pub use chrome::{key_hint, page_block, panel_block, quiet_panel_block, section_heading};
pub use dispatch::render_dispatch;
pub use emblem::{render_empire_emblem, EmblemPattern, EmpireEmblem, EmpireEmblemPalette};
pub use footer::render_footer;
pub use header::{
    derive_header_data, render_brand_header, render_header, render_screen_title_header, HeaderData,
};
pub use help::render_help;
pub use log::{render_log, EventLog, LogEntryKind};
pub use meter::meter_line;
pub use palette::{render_palette, PaletteCommand};
