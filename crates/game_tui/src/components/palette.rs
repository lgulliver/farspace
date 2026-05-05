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
    let popup_area = centered_fixed(50, 5, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            "Command palette — coming soon",
            Theme::muted_style(),
        )),
        Line::from(vec![
            Span::styled("> ", Theme::accent_style()),
            Span::raw(input),
            Span::styled("_", Theme::accent_style()),
        ]),
        Line::from(Span::styled("Press Esc to close", Theme::muted_style())),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Command ")
                .borders(Borders::ALL)
                .style(Theme::default_style()),
        )
        .style(Theme::default_style());

    frame.render_widget(paragraph, popup_area);
}

/// Helper for accent style
trait ThemeExt {
    fn accent_style() -> ratatui::style::Style;
}

impl ThemeExt for Theme {
    fn accent_style() -> ratatui::style::Style {
        ratatui::style::Style::default().fg(Theme::accent())
    }
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
}
