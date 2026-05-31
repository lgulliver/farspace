//! Reusable empire emblem renderer for premium setup / identity screens.

use game_core::{all_empire_definitions, EmpireDefinition};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Handcrafted emblem motif.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmblemPattern {
    Frontier,
    Citadel,
    Eclipse,
    Nexus,
    Obelisk,
    Helix,
    Dominion,
    Aegis,
}

impl EmblemPattern {
    pub fn from_empire_index(index: usize) -> Self {
        match index % 8 {
            0 => Self::Frontier,
            1 => Self::Citadel,
            2 => Self::Eclipse,
            3 => Self::Nexus,
            4 => Self::Obelisk,
            5 => Self::Helix,
            6 => Self::Dominion,
            _ => Self::Aegis,
        }
    }

    fn art(self) -> &'static [&'static str] {
        match self {
            Self::Frontier => &["  ◇◇◇  ", " ╱███╲ ", "◇██◆██◇", " ╲███╱ ", "  ◇◇◇  "],
            Self::Citadel => &[" █████ ", " █▣▣▣█ ", " █▣█▣█ ", " █▣▣▣█ ", " █████ "],
            Self::Eclipse => &["  ╱╲   ", " ◐██◐  ", "◐█◆█◐ ", " ◑██◑  ", "  ╲╱   "],
            Self::Nexus => &["   ╳   ", "  ╳◆╳  ", " ╳█◆█╳ ", "  ╳◆╳  ", "   ╳   "],
            Self::Obelisk => &["  ▉▉   ", " ▉█▉▉  ", "▉███▉ ", " ▉█▉▉  ", "  ▉▉   "],
            Self::Helix => &[" ╱═╲   ", "╱╳█╳╲  ", " ╲◆╱   ", "╱╳█╳╲  ", " ╲═╱   "],
            Self::Dominion => &[" ▲▲▲▲  ", "▲▓▓▓▲ ", "▲▓◆▓▲ ", "▲▓▓▓▲ ", " ▲▲▲▲  "],
            Self::Aegis => &[" ▢▢▢   ", "▢◈█◈▢ ", "▢◈◆◈▢ ", "▢◈█◈▢ ", " ▢▢▢   "],
        }
    }
}

/// Palette used by an empire emblem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpireEmblemPalette {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
}

impl EmpireEmblemPalette {
    pub fn from_empire_index(index: usize) -> Self {
        match index % 8 {
            0 => Self {
                primary: Color::Rgb(116, 176, 255),
                secondary: Color::Rgb(63, 102, 145),
                accent: Color::Rgb(214, 235, 255),
            },
            1 => Self {
                primary: Color::Rgb(104, 196, 255),
                secondary: Color::Rgb(54, 122, 173),
                accent: Color::Rgb(230, 247, 255),
            },
            2 => Self {
                primary: Color::Rgb(145, 214, 135),
                secondary: Color::Rgb(72, 123, 72),
                accent: Color::Rgb(221, 255, 221),
            },
            3 => Self {
                primary: Color::Rgb(214, 179, 116),
                secondary: Color::Rgb(127, 94, 56),
                accent: Color::Rgb(255, 241, 204),
            },
            4 => Self {
                primary: Color::Rgb(214, 133, 63),
                secondary: Color::Rgb(128, 79, 32),
                accent: Color::Rgb(255, 220, 176),
            },
            5 => Self {
                primary: Color::Rgb(194, 143, 255),
                secondary: Color::Rgb(102, 72, 160),
                accent: Color::Rgb(240, 225, 255),
            },
            6 => Self {
                primary: Color::Rgb(255, 145, 116),
                secondary: Color::Rgb(155, 68, 58),
                accent: Color::Rgb(255, 228, 221),
            },
            _ => Self {
                primary: Color::Rgb(171, 219, 225),
                secondary: Color::Rgb(74, 116, 126),
                accent: Color::Rgb(236, 249, 251),
            },
        }
    }
}

/// Reusable empire emblem definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpireEmblem {
    pub pattern: EmblemPattern,
    pub palette: EmpireEmblemPalette,
    pub symbol: char,
}

impl EmpireEmblem {
    pub fn from_empire_index(index: usize, symbol: char) -> Self {
        Self {
            pattern: EmblemPattern::from_empire_index(index),
            palette: EmpireEmblemPalette::from_empire_index(index),
            symbol,
        }
    }
}

fn center_pad(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        let start = (len - width) / 2;
        return text.chars().skip(start).take(width).collect();
    }
    let left = (width - len) / 2;
    let right = width - len - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

fn style_for_row(row: usize, total: usize, emblem: &EmpireEmblem) -> Style {
    if row == total / 2 {
        Style::default()
            .fg(emblem.palette.accent)
            .add_modifier(Modifier::BOLD)
    } else if row == 0 || row + 1 == total {
        Style::default().fg(emblem.palette.secondary)
    } else {
        Style::default()
            .fg(emblem.palette.primary)
            .add_modifier(Modifier::BOLD)
    }
}

fn fit_art(pattern: EmblemPattern, width: usize, height: usize) -> Vec<String> {
    let art = pattern.art();
    let base_height = art.len();
    let base_width = art.iter().map(|row| row.chars().count()).max().unwrap_or(0);
    let target_width = width.max(base_width);
    let target_height = height.max(base_height);

    let vertical_pad = target_height.saturating_sub(base_height) / 2;
    let mut lines = Vec::new();
    lines.extend(std::iter::repeat_with(String::new).take(vertical_pad));
    for row in art {
        lines.push(center_pad(row, target_width));
    }
    while lines.len() < target_height {
        lines.push(String::new());
    }
    lines
}

/// Render an empire emblem into the target area.
pub fn render_empire_emblem(frame: &mut Frame, area: Rect, emblem: &EmpireEmblem) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let show_border = area.width >= 10 && area.height >= 7;
    let render_area = if show_border {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(emblem.palette.primary))
            .style(Style::default().bg(Theme::space_bg()));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    } else {
        area
    };

    if render_area.width == 0 || render_area.height == 0 {
        return;
    }

    let target_height = if render_area.width >= 16 && render_area.height >= 9 {
        9
    } else if render_area.width >= 12 && render_area.height >= 7 {
        7
    } else {
        5
    } as usize;
    let target_width = if render_area.width >= 16 {
        9
    } else if render_area.width >= 12 {
        7
    } else {
        5
    } as usize;

    let art = fit_art(emblem.pattern, target_width, target_height);
    let top_pad = render_area.height.saturating_sub(art.len() as u16) / 2;
    let art_height = art.len() as u16;
    let used_rows = top_pad + art_height;
    let symbol_row = if render_area.height > used_rows {
        Some(used_rows)
    } else {
        None
    };

    let mut lines = Vec::new();
    lines.extend(std::iter::repeat_with(|| Line::from("")).take(top_pad as usize));
    for (row_idx, row) in art.iter().enumerate() {
        let style = style_for_row(row_idx, art.len(), emblem);
        lines.push(Line::from(Span::styled(row.clone(), style)));
    }
    if symbol_row.is_some() {
        lines.push(Line::from(Span::styled(
            format!("  {}", emblem.symbol),
            Style::default()
                .fg(emblem.palette.accent)
                .add_modifier(Modifier::BOLD),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Theme::default_style()),
        render_area,
    );
}

/// Resolve an emblem for the given empire index.
pub fn resolve_empire_emblem(index: usize, symbol: char) -> EmpireEmblem {
    EmpireEmblem::from_empire_index(index, symbol)
}

/// Resolve an emblem directly from an empire definition.
pub fn resolve_empire_emblem_for_definition(def: &EmpireDefinition) -> EmpireEmblem {
    let all_defs = all_empire_definitions();
    let index = all_defs
        .iter()
        .position(|candidate| candidate.id == def.id)
        .unwrap_or(def.id.0 as usize);
    resolve_empire_emblem(index, def.symbol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_at(width: u16, height: u16) {
        let backend = TestBackend::new(width.max(1), height.max(1));
        let mut terminal = Terminal::new(backend).unwrap();
        let emblem = EmpireEmblem::from_empire_index(0, '◇');
        terminal
            .draw(|frame| {
                let area = ratatui::layout::Rect::new(0, 0, width, height);
                render_empire_emblem(frame, area, &emblem);
            })
            .unwrap();
    }

    #[test]
    fn emblem_renders_at_narrow_and_zero_sizes() {
        render_at(0, 0);
        render_at(4, 3);
        render_at(6, 5);
        render_at(12, 7);
        render_at(18, 10);
    }

    #[test]
    fn emblem_resolves_distinct_patterns_per_index() {
        let a = EmpireEmblem::from_empire_index(0, '◇');
        let b = EmpireEmblem::from_empire_index(1, '◇');
        assert_ne!(a.pattern, b.pattern);
        assert_ne!(a.palette, b.palette);
    }
}
