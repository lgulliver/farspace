//! Terminal color capability layer.

/// Terminal color capability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    Mono,
}

/// Placeholder detection hook. Future work can inspect terminal capabilities.
pub fn detect_color_mode() -> ColorMode {
    ColorMode::TrueColor
}
