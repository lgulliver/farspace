//! Shared UI chrome primitives.

use crate::theme::Theme;
use ratatui::{
    style::Modifier,
    symbols::border::ROUNDED,
    text::{Line, Span},
    widgets::{Block, Borders},
};

pub fn page_block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title.into()))
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style())
}

pub fn panel_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title.into()))
        .borders(Borders::ALL)
        .border_set(ROUNDED)
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            Theme::dim_border_style()
        })
        .style(Theme::default_style())
}

pub fn quiet_panel_block(title: impl Into<String>) -> Block<'static> {
    panel_block(title, false).style(Theme::default_style().bg(Theme::panel_bg()))
}

pub fn section_heading(text: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Theme::title_style().add_modifier(Modifier::BOLD),
    ))
}

pub fn key_hint(key: &'static str, label: &'static str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key, Theme::title_style()),
        Span::raw(" "),
        Span::styled(label, Theme::muted_style()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_block_uses_focused_style_when_focused() {
        assert_eq!(
            panel_block("Panel", true).border_style,
            Some(Theme::focused_border_style())
        );
        assert_eq!(
            panel_block("Panel", false).border_style,
            Some(Theme::dim_border_style())
        );
    }
}
