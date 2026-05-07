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
    let credits_style = if credits < 0 {
        Theme::warning_style()
    } else {
        Theme::default_style()
    };
    let food_style = if food < 0 {
        Theme::warning_style()
    } else {
        Theme::default_style()
    };

    let spans = vec![
        Span::styled(format!(" Turn {} ", turn), Theme::header_style()),
        Span::raw(" │ "),
        Span::styled(empire_name, Theme::title_style()),
        Span::raw(" │ "),
        Span::styled("Credits: ", Theme::muted_style()),
        Span::styled(format!("{}", credits), credits_style),
        Span::raw(" │ "),
        Span::styled("Food: ", Theme::muted_style()),
        Span::styled(format!("{}", food), food_style),
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

    #[test]
    fn render_header_negative_credits_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header(frame, area, 7, "Test Empire", -100, 3, 200);
            })
            .unwrap();
    }

    #[test]
    fn render_header_both_deficits_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header(frame, area, 10, "Test Empire", -50, -3, 0);
            })
            .unwrap();
    }
}
