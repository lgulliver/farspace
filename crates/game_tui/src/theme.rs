//! Color theme for the TUI

use ratatui::style::{Color, Modifier, Style};

/// Theme colors for the UI
pub struct Theme;

impl Theme {
    /// Background color
    pub fn bg() -> Color {
        Color::Black
    }

    /// Primary foreground color
    pub fn fg() -> Color {
        Color::White
    }

    /// Accent color (for highlights)
    pub fn accent() -> Color {
        Color::Cyan
    }

    /// Secondary accent
    pub fn accent2() -> Color {
        Color::Yellow
    }

    /// Error color
    pub fn error() -> Color {
        Color::Red
    }

    /// Success color
    pub fn success() -> Color {
        Color::Green
    }

    /// Muted/dim color
    pub fn muted() -> Color {
        Color::DarkGray
    }

    /// Star colors by spectral class
    pub fn star_color(class: game_core::SpectralClass) -> Color {
        match class {
            game_core::SpectralClass::O => Color::Blue,
            game_core::SpectralClass::B => Color::LightBlue,
            game_core::SpectralClass::A => Color::White,
            game_core::SpectralClass::F => Color::LightYellow,
            game_core::SpectralClass::G => Color::Yellow,
            game_core::SpectralClass::K => Color::Rgb(255, 165, 0), // Orange
            game_core::SpectralClass::M => Color::Red,
        }
    }

    /// Default text style
    pub fn default_style() -> Style {
        Style::default().fg(Self::fg()).bg(Self::bg())
    }

    /// Highlighted/selected style
    pub fn highlight_style() -> Style {
        Style::default()
            .fg(Self::bg())
            .bg(Self::accent())
            .add_modifier(Modifier::BOLD)
    }

    /// Title style
    pub fn title_style() -> Style {
        Style::default()
            .fg(Self::accent())
            .add_modifier(Modifier::BOLD)
    }

    /// Error style
    pub fn error_style() -> Style {
        Style::default().fg(Self::error())
    }

    /// Muted style
    pub fn muted_style() -> Style {
        Style::default().fg(Self::muted())
    }

    /// Accent style
    pub fn accent_style() -> Style {
        Style::default().fg(Self::accent())
    }

    /// Header style
    pub fn header_style() -> Style {
        Style::default()
            .fg(Self::fg())
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colors_are_defined() {
        assert_eq!(Theme::bg(), Color::Black);
        assert_eq!(Theme::fg(), Color::White);
    }

    #[test]
    fn star_colors_vary_by_class() {
        assert_ne!(
            Theme::star_color(game_core::SpectralClass::O),
            Theme::star_color(game_core::SpectralClass::M)
        );
    }
}
