//! Help overlay component

use crate::layout::centered_rect;
use crate::screens::Screen;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render the help overlay
pub fn render_help(frame: &mut Frame, area: Rect, screen: &Screen) {
    let popup_area = centered_rect(60, 70, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = match screen {
        Screen::Menu => " Main Menu Help ",
        Screen::Galaxy => " Galaxy Map Help ",
    };

    let bindings = match screen {
        Screen::Menu => vec![
            ("N", "Start a new game"),
            ("L", "Load a saved game"),
            ("Q", "Quit the game"),
            ("?", "Toggle this help"),
        ],
        Screen::Galaxy => vec![
            ("h / ←", "Move selection left"),
            ("j / ↓", "Move selection down"),
            ("k / ↑", "Move selection up"),
            ("l / →", "Move selection right"),
            ("E / Enter / T", "End turn"),
            (":", "Command palette (:save, :load)"),
            ("/", "Search"),
            ("?", "Toggle this help"),
            ("Q", "Quit"),
        ],
    };

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!("{:>12}", key), Theme::title_style()),
                Span::raw("  "),
                Span::styled(*desc, Theme::default_style()),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
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
    fn render_help_menu() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::Menu);
            })
            .unwrap();
    }

    #[test]
    fn render_help_galaxy() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::Galaxy);
            })
            .unwrap();
    }
}
