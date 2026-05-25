//! Semantic palettes for themed UI surfaces.

use ratatui::style::Color;

use super::capability::ColorMode;

/// Splash screen semantic palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplashPalette {
    pub void_bg: Color,
    pub nebula_a: Color,
    pub nebula_b: Color,
    pub star_core: Color,
    pub title_primary: Color,
    pub title_secondary: Color,
    pub accent: Color,
    pub border_hot: Color,
    pub border_cold: Color,
    pub text_muted: Color,
    pub warning: Color,
}

impl SplashPalette {
    pub fn for_mode(mode: ColorMode) -> Self {
        match mode {
            ColorMode::TrueColor => Self {
                void_bg: Color::Rgb(4, 7, 18),
                nebula_a: Color::Rgb(22, 38, 82),
                nebula_b: Color::Rgb(78, 28, 92),
                star_core: Color::Rgb(246, 244, 220),
                title_primary: Color::Rgb(148, 224, 255),
                title_secondary: Color::Rgb(114, 159, 255),
                accent: Color::Rgb(92, 238, 208),
                border_hot: Color::Rgb(113, 214, 255),
                border_cold: Color::Rgb(62, 94, 176),
                text_muted: Color::Rgb(140, 155, 188),
                warning: Color::Rgb(255, 188, 94),
            },
            ColorMode::Ansi256 => Self {
                void_bg: Color::Black,
                nebula_a: Color::Blue,
                nebula_b: Color::Magenta,
                star_core: Color::White,
                title_primary: Color::Cyan,
                title_secondary: Color::LightBlue,
                accent: Color::Green,
                border_hot: Color::Cyan,
                border_cold: Color::Blue,
                text_muted: Color::Gray,
                warning: Color::Yellow,
            },
            ColorMode::Mono => Self {
                void_bg: Color::Black,
                nebula_a: Color::Black,
                nebula_b: Color::Black,
                star_core: Color::White,
                title_primary: Color::White,
                title_secondary: Color::Gray,
                accent: Color::White,
                border_hot: Color::White,
                border_cold: Color::Gray,
                text_muted: Color::Gray,
                warning: Color::White,
            },
        }
    }
}
