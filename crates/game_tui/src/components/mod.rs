//! TUI Components

pub mod advisor;
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

pub use advisor::{AlertSeverity, advisor_strip_text, render_advisor_strip, render_turn_brief};
pub use battle_report::{render_battle_reports, render_battle_reports_v3};
pub use chrome::{key_hint, page_block, panel_block, quiet_panel_block, section_heading};
pub use dispatch::render_dispatch;
pub use emblem::{EmblemPattern, EmpireEmblem, EmpireEmblemPalette, render_empire_emblem};
pub use footer::render_footer;
pub use header::{
    HeaderData, derive_header_data, render_brand_header, render_header, render_screen_title_header,
};
pub use help::render_help;
pub use log::{EventLog, LogEntryKind, render_log};
pub use meter::meter_line;
pub use palette::{PaletteCommand, render_palette};
