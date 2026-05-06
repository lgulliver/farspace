//! Header component

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Render the header bar showing turn, empire name, and economy summary.
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    turn: u32,
    empire_name: &str,
    credits: i64,
    food: i64,
    research: i64,
) {
    let spans = vec![
        Span::styled(format!(" Turn {} ", turn), Theme::header_style()),
        Span::raw(" │ "),
        Span::styled(empire_name, Theme::title_style()),
        Span::raw(" │ "),
        Span::styled("Credits: ", Theme::muted_style()),
        Span::raw(format!("{}", credits)),
        Span::raw(" │ "),
        Span::styled("Food: ", Theme::muted_style()),
        Span::raw(format!("{}", food)),
        Span::raw(" │ "),
        Span::styled("Research: ", Theme::muted_style()),
        Span::raw(format!("{}", research)),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(Theme::default_style());

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_header_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header(frame, area, 5, "Test Empire", 1000, 50, 500);
            })
            .unwrap();
    }

    #[test]
    fn render_header_negative_food_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header(frame, area, 3, "Test Empire", 20, -5, 10);
            })
            .unwrap();
    }
}
