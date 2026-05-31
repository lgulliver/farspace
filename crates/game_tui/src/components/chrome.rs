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
    panel_block(title, false)
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
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    #[test]
    fn panel_block_uses_focused_style_when_focused() {
        let focused_backend = TestBackend::new(20, 3);
        let mut focused_terminal = Terminal::new(focused_backend).unwrap();
        focused_terminal
            .draw(|frame| {
                frame.render_widget(panel_block("Panel", true), Rect::new(0, 0, 20, 3));
            })
            .unwrap();
        let focused_cell = focused_terminal
            .backend()
            .buffer()
            .cell((0, 0))
            .expect("focused border cell")
            .style();

        let dim_backend = TestBackend::new(20, 3);
        let mut dim_terminal = Terminal::new(dim_backend).unwrap();
        dim_terminal
            .draw(|frame| {
                frame.render_widget(panel_block("Panel", false), Rect::new(0, 0, 20, 3));
            })
            .unwrap();
        let dim_cell = dim_terminal
            .backend()
            .buffer()
            .cell((0, 0))
            .expect("dim border cell")
            .style();

        assert_eq!(focused_cell.fg, Theme::focused_border_style().fg);
        assert_eq!(dim_cell.fg, Theme::dim_border_style().fg);
        assert_ne!(focused_cell.fg, dim_cell.fg);
    }
}
