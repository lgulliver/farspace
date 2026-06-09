//! Terminal color capability layer.
//!
//! Colors throughout the TUI are authored as truecolor RGB. [`adapt_color`]
//! downconverts them for less capable terminals; `App::render` applies it to
//! the whole frame buffer, so every widget, sprite, and gradient degrades
//! without per-call-site plumbing.

use ratatui::style::Color;
use std::sync::OnceLock;

/// Terminal color capability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    Mono,
}

/// Detect the terminal's color capability once and cache it for the process.
///
/// Precedence: `FARSPACE_COLOR` override → `TERM=dumb` / `NO_COLOR` →
/// `COLORTERM` truecolor advertisement → `TERM` 256-color suffix → a
/// conservative default (truecolor on Windows consoles, 256-color elsewhere).
pub fn detect_color_mode() -> ColorMode {
    static DETECTED: OnceLock<ColorMode> = OnceLock::new();
    *DETECTED.get_or_init(|| {
        color_mode_from_env(
            std::env::var("FARSPACE_COLOR").ok().as_deref(),
            std::env::var("NO_COLOR").ok().as_deref(),
            std::env::var("COLORTERM").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
        )
    })
}

/// Testable inner implementation of [`detect_color_mode`].
fn color_mode_from_env(
    override_var: Option<&str>,
    no_color: Option<&str>,
    colorterm: Option<&str>,
    term: Option<&str>,
) -> ColorMode {
    if let Some(value) = override_var {
        match value.trim().to_ascii_lowercase().as_str() {
            "truecolor" | "24bit" | "rgb" => return ColorMode::TrueColor,
            "256" | "ansi256" => return ColorMode::Ansi256,
            "mono" | "none" => return ColorMode::Mono,
            _ => {} // unrecognised override: fall through to detection
        }
    }

    let term = term.unwrap_or("");
    if term == "dumb" {
        return ColorMode::Mono;
    }
    // https://no-color.org/ — any non-empty value disables color.
    if no_color.is_some_and(|v| !v.is_empty()) {
        return ColorMode::Mono;
    }
    if colorterm.is_some_and(|v| v.contains("truecolor") || v.contains("24bit")) {
        return ColorMode::TrueColor;
    }
    if term.contains("256color") {
        return ColorMode::Ansi256;
    }
    if term.is_empty() && cfg!(windows) {
        // Modern Windows consoles support truecolor and rarely set TERM.
        return ColorMode::TrueColor;
    }
    ColorMode::Ansi256
}

/// Downconvert a color for the detected terminal capability.
pub fn adapt_color(color: Color) -> Color {
    adapt_color_for(detect_color_mode(), color)
}

/// Downconvert a color for an explicit capability mode.
///
/// - `TrueColor`: passthrough.
/// - `Ansi256`: RGB values map onto the xterm 6×6×6 cube / grayscale ramp;
///   named and indexed colors pass through.
/// - `Mono`: everything collapses to Black/DarkGray/Gray/White by luminance.
pub fn adapt_color_for(mode: ColorMode, color: Color) -> Color {
    match mode {
        ColorMode::TrueColor => color,
        ColorMode::Ansi256 => match color {
            Color::Rgb(r, g, b) => Color::Indexed(rgb_to_ansi256(r, g, b)),
            other => other,
        },
        ColorMode::Mono => mono_color(color),
    }
}

/// Map an RGB value to the nearest xterm-256 palette index.
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // Near-gray values use the dedicated 24-step grayscale ramp (232-255).
    if r == g && g == b {
        if r < 8 {
            return 16; // cube black
        }
        if r > 248 {
            return 231; // cube white
        }
        return 232 + ((u16::from(r) - 8) / 10) as u8;
    }
    let to_cube = |v: u8| -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((u16::from(v) - 35) / 40) as u8
        }
    };
    16 + 36 * to_cube(r) + 6 * to_cube(g) + to_cube(b)
}

fn mono_color(color: Color) -> Color {
    match color {
        Color::Reset => Color::Reset,
        Color::Rgb(r, g, b) => {
            // Integer Rec. 709 luma approximation.
            let luma = (u32::from(r) * 2126 + u32::from(g) * 7152 + u32::from(b) * 722) / 10_000;
            match luma {
                0..=47 => Color::Black,
                48..=119 => Color::DarkGray,
                120..=191 => Color::Gray,
                _ => Color::White,
            }
        }
        Color::Black => Color::Black,
        Color::White | Color::LightYellow | Color::LightCyan | Color::LightGreen => Color::White,
        Color::Gray
        | Color::Cyan
        | Color::Yellow
        | Color::Green
        | Color::LightBlue
        | Color::LightRed
        | Color::LightMagenta => Color::Gray,
        Color::DarkGray | Color::Blue | Color::Red | Color::Magenta => Color::DarkGray,
        Color::Indexed(i) => match i {
            0 | 16 => Color::Black,
            15 | 231 => Color::White,
            i if i >= 244 => Color::White,
            i if i >= 232 => Color::Gray,
            _ => Color::Gray,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_variable_takes_precedence() {
        assert_eq!(
            color_mode_from_env(
                Some("mono"),
                None,
                Some("truecolor"),
                Some("xterm-256color")
            ),
            ColorMode::Mono
        );
        assert_eq!(
            color_mode_from_env(Some("truecolor"), Some("1"), None, Some("dumb")),
            ColorMode::TrueColor
        );
        assert_eq!(
            color_mode_from_env(Some("256"), None, Some("truecolor"), None),
            ColorMode::Ansi256
        );
        // Unrecognised override falls back to detection.
        assert_eq!(
            color_mode_from_env(Some("sparkly"), None, Some("truecolor"), None),
            ColorMode::TrueColor
        );
    }

    #[test]
    fn detection_honours_standard_environment_signals() {
        assert_eq!(
            color_mode_from_env(None, None, None, Some("dumb")),
            ColorMode::Mono
        );
        assert_eq!(
            color_mode_from_env(None, Some("1"), Some("truecolor"), Some("xterm")),
            ColorMode::Mono,
            "NO_COLOR wins over COLORTERM"
        );
        assert_eq!(
            color_mode_from_env(None, Some(""), Some("truecolor"), Some("xterm")),
            ColorMode::TrueColor,
            "empty NO_COLOR is ignored per no-color.org"
        );
        assert_eq!(
            color_mode_from_env(None, None, Some("24bit"), Some("xterm")),
            ColorMode::TrueColor
        );
        assert_eq!(
            color_mode_from_env(None, None, None, Some("xterm-256color")),
            ColorMode::Ansi256
        );
        assert_eq!(
            color_mode_from_env(None, None, None, Some("xterm")),
            ColorMode::Ansi256,
            "plain terminals default to the safe 256-color mode"
        );
    }

    #[test]
    fn truecolor_mode_passes_colors_through() {
        let c = Color::Rgb(150, 185, 255);
        assert_eq!(adapt_color_for(ColorMode::TrueColor, c), c);
    }

    #[test]
    fn ansi256_mode_indexes_rgb_and_keeps_named_colors() {
        match adapt_color_for(ColorMode::Ansi256, Color::Rgb(150, 185, 255)) {
            Color::Indexed(i) => assert!(i >= 16, "cube/grayscale index expected, got {i}"),
            other => panic!("expected indexed color, got {other:?}"),
        }
        assert_eq!(
            adapt_color_for(ColorMode::Ansi256, Color::Cyan),
            Color::Cyan
        );
        // Grays land on the grayscale ramp.
        assert_eq!(
            adapt_color_for(ColorMode::Ansi256, Color::Rgb(128, 128, 128)),
            Color::Indexed(244)
        );
        // Extremes land on the cube corners.
        assert_eq!(
            adapt_color_for(ColorMode::Ansi256, Color::Rgb(0, 0, 0)),
            Color::Indexed(16)
        );
        assert_eq!(
            adapt_color_for(ColorMode::Ansi256, Color::Rgb(255, 255, 255)),
            Color::Indexed(231)
        );
    }

    #[test]
    fn mono_mode_collapses_to_grayscale_named_colors() {
        for color in [
            Color::Rgb(150, 185, 255),
            Color::Rgb(4, 7, 18),
            Color::Cyan,
            Color::Red,
            Color::Indexed(196),
        ] {
            let adapted = adapt_color_for(ColorMode::Mono, color);
            assert!(
                matches!(
                    adapted,
                    Color::Black | Color::DarkGray | Color::Gray | Color::White
                ),
                "mono must collapse {color:?} to grayscale, got {adapted:?}"
            );
        }
        assert_eq!(adapt_color_for(ColorMode::Mono, Color::Reset), Color::Reset);
        // Bright vs dark RGB keep contrast.
        assert_eq!(
            adapt_color_for(ColorMode::Mono, Color::Rgb(250, 250, 240)),
            Color::White
        );
        assert_eq!(
            adapt_color_for(ColorMode::Mono, Color::Rgb(5, 8, 16)),
            Color::Black
        );
    }
}
