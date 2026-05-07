//! Command palette component

use crate::layout::centered_fixed;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render the command palette
pub fn render_palette(frame: &mut Frame, area: Rect, input: &str) {
    // Height 7 = border top + hint line + blank + input line + blank + help line + border bottom
    let popup_area = centered_fixed(54, 7, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(vec![
            Span::styled("Commands: ", Theme::muted_style()),
            Span::styled("save", Theme::title_style()),
            Span::styled("  ·  ", Theme::dim_border_style()),
            Span::styled("load", Theme::title_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(": ", Theme::accent_style()),
            Span::raw(input),
            Span::styled("▌", Theme::accent_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Theme::title_style()),
            Span::styled(" to execute  ", Theme::muted_style()),
            Span::styled("Esc", Theme::title_style()),
            Span::styled(" to close", Theme::muted_style()),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Command Palette ")
                .borders(Borders::ALL)
                .border_style(Theme::focused_border_style())
                .style(Theme::default_style()),
        )
        .style(Theme::default_style());

    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_palette_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "test input");
            })
            .unwrap();
    }

    #[test]
    fn render_palette_empty_input() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "");
            })
            .unwrap();
    }

    #[test]
    fn render_palette_long_input_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "save_a_very_long_filename_that_overflows");
            })
            .unwrap();
    }

    #[test]
    fn render_palette_tiny_terminal_no_panic() {
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "save");
            })
            .unwrap();
    }
}
