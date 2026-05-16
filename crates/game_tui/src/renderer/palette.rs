use ratatui::style::Style;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorToken {
    Default,
    Muted,
    Accent,
    Accent2,
    Warning,
    Error,
    Success,
    SpaceBg,
    StarCold,
    StarWarm,
    PlanetLand,
    PlanetWater,
    PlanetIce,
    PlanetDesert,
    PlanetLava,
    ColonyLight,
    DimOverlay,
}

impl ColorToken {
    pub fn to_style(self, bg: Option<ColorToken>) -> Style {
        let mut style = Style::default().fg(self.fg_color());
        if let Some(bg) = bg {
            style = style.bg(bg.bg_color());
        }
        style
    }

    pub fn fg_color(self) -> ratatui::style::Color {
        match self {
            ColorToken::Default => Theme::fg(),
            ColorToken::Muted => Theme::muted(),
            ColorToken::Accent => Theme::accent(),
            ColorToken::Accent2 => Theme::accent2(),
            ColorToken::Warning => Theme::warning(),
            ColorToken::Error => Theme::error(),
            ColorToken::Success => Theme::success(),
            ColorToken::SpaceBg => Theme::space_bg(),
            ColorToken::StarCold => ratatui::style::Color::Rgb(150, 185, 255),
            ColorToken::StarWarm => ratatui::style::Color::Rgb(255, 219, 132),
            ColorToken::PlanetLand => ratatui::style::Color::Rgb(119, 178, 98),
            ColorToken::PlanetWater => ratatui::style::Color::Rgb(73, 129, 199),
            ColorToken::PlanetIce => ratatui::style::Color::Rgb(179, 220, 255),
            ColorToken::PlanetDesert => ratatui::style::Color::Rgb(210, 170, 98),
            ColorToken::PlanetLava => ratatui::style::Color::Rgb(214, 98, 72),
            ColorToken::ColonyLight => ratatui::style::Color::Rgb(255, 226, 149),
            ColorToken::DimOverlay => ratatui::style::Color::Rgb(42, 54, 74),
        }
    }

    pub fn bg_color(self) -> ratatui::style::Color {
        match self {
            ColorToken::Default
            | ColorToken::Muted
            | ColorToken::Accent
            | ColorToken::Accent2
            | ColorToken::Warning
            | ColorToken::Error
            | ColorToken::Success
            | ColorToken::SpaceBg
            | ColorToken::StarCold
            | ColorToken::StarWarm
            | ColorToken::PlanetLand
            | ColorToken::PlanetWater
            | ColorToken::PlanetIce
            | ColorToken::PlanetDesert
            | ColorToken::PlanetLava
            | ColorToken::ColonyLight
            | ColorToken::DimOverlay => self.fg_color(),
        }
    }
}
