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

/// A help binding entry: either a key→description pair or a section separator.
enum HelpEntry {
    /// A section header label
    Section(&'static str),
    /// A key binding (key text, description)
    Binding(&'static str, &'static str),
}

/// Render the help overlay
pub fn render_help(frame: &mut Frame, area: Rect, screen: &Screen) {
    let popup_area = centered_rect(65, 75, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = match screen {
        Screen::Menu => " Main Menu Help ",
        Screen::Galaxy => " Galaxy Map Help ",
        Screen::Colony => " Colony Help ",
        Screen::Research => " Research Help ",
        Screen::Diplomacy => " Diplomacy Help ",
    };

    let entries: Vec<HelpEntry> = match screen {
        Screen::Menu => vec![
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("N", "Start a new game"),
            HelpEntry::Binding("L", "Load a saved game"),
            HelpEntry::Binding("Q", "Quit the game"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("?", "Toggle this help"),
        ],
        Screen::Galaxy => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("h / ←", "Move selection left"),
            HelpEntry::Binding("j / ↓", "Move selection down"),
            HelpEntry::Binding("k / ↑", "Move selection up"),
            HelpEntry::Binding("l / →", "Move selection right"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("c", "Enter colony (if colonized star selected)"),
            HelpEntry::Binding("C", "Colonize selected system with idle colonizer"),
            HelpEntry::Binding("r", "Open research screen"),
            HelpEntry::Binding("D", "Open diplomacy screen"),
            HelpEntry::Binding("S", "Dispatch scout to selected unexplored system"),
            HelpEntry::Binding("M", "Move idle fleet to selected explored system"),
            HelpEntry::Binding("E / T / Enter", "End turn (AI acts automatically)"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("/", "Search"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Q", "Quit"),
        ],
        Screen::Colony => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("Tab", "Switch between role selector / build picker"),
            HelpEntry::Binding("j / ↓", "Move cursor down in active panel"),
            HelpEntry::Binding("k / ↑", "Move cursor up in active panel"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Assign role or queue building (active panel)"),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to galaxy map"),
        ],
        Screen::Research => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move cursor down"),
            HelpEntry::Binding("k / ↑", "Move cursor up"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Select highlighted technology"),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to galaxy map"),
        ],
        Screen::Diplomacy => vec![
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("e / t / Enter", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to galaxy map"),
        ],
    };

    let lines: Vec<Line> = entries
        .iter()
        .map(|entry| match entry {
            HelpEntry::Section(label) => Line::from(vec![
                Span::raw(" "),
                Span::styled(*label, Theme::accent_style()),
                Span::styled(
                    " ─────────────────────────────────────",
                    Theme::dim_border_style(),
                ),
            ]),
            HelpEntry::Binding(key, desc) => Line::from(vec![
                Span::styled(format!("{:>14}", key), Theme::title_style()),
                Span::raw("  "),
                Span::styled(*desc, Theme::default_style()),
            ]),
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

    #[test]
    fn render_help_tiny_terminal_does_not_panic() {
        // Ensure centered_rect clamps gracefully on tiny areas
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::Galaxy);
            })
            .unwrap();
    }
}
