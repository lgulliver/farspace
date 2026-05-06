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
        Screen::Colony => " Colony Help ",
        Screen::Research => " Research Help ",
        Screen::Diplomacy => " Diplomacy Help ",
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
            ("c", "Enter colony (if colonized star selected)"),
            ("C", "Colonize selected system with idle colonizer fleet"),
            ("r", "Open research screen"),
            ("D", "Open diplomacy screen"),
            ("S", "Dispatch scout to selected unexplored system"),
            ("M", "Move idle fleet to selected explored system"),
            ("E / T / Enter", "End turn (AI acts automatically)"),
            (":", "Command palette (:save, :load)"),
            ("/", "Search"),
            ("?", "Toggle this help"),
            ("Q", "Quit"),
        ],
        Screen::Colony => vec![
            ("j / ↓", "Move cursor down in build picker"),
            ("k / ↑", "Move cursor up in build picker"),
            ("Enter", "Queue selected building"),
            ("e / t", "End turn"),
            (":", "Command palette (:save, :load)"),
            ("?", "Toggle this help"),
            ("Esc", "Return to galaxy map"),
        ],
        Screen::Research => vec![
            ("j / ↓", "Move cursor down"),
            ("k / ↑", "Move cursor up"),
            ("Enter", "Select highlighted technology"),
            ("e / t", "End turn"),
            (":", "Command palette (:save, :load)"),
            ("?", "Toggle this help"),
            ("Esc", "Return to galaxy map"),
        ],
        Screen::Diplomacy => vec![
            ("e / t / Enter", "End turn"),
            (":", "Command palette (:save, :load)"),
            ("?", "Toggle this help"),
            ("Esc", "Return to galaxy map"),
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

    #[test]
    fn render_help_colony() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::Colony);
            })
            .unwrap();
    }

    #[test]
    fn render_help_research() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::Research);
            })
            .unwrap();
    }

    #[test]
    fn render_help_diplomacy() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::Diplomacy);
            })
            .unwrap();
    }
}
