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
    /// Informational note (not a key binding)
    Note(&'static str),
}

/// Render the help overlay
pub fn render_help(frame: &mut Frame, area: Rect, screen: &Screen) {
    let popup_area = centered_rect(65, 75, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = match screen {
        Screen::Menu => " Main Menu Help ",
        Screen::NewGameSetup => " New Game Setup Help ",
        Screen::SectorOverview => " Galaxy Overview Help ",
        Screen::SectorMap => " Sector Map Help ",
        Screen::System => " System View Help ",
        Screen::Colony => " Colony Help ",
        Screen::EmpireOverview => " Empire Overview Help ",
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
        Screen::NewGameSetup => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move to next field"),
            HelpEntry::Binding("k / ↑", "Move to previous field"),
            HelpEntry::Section("Empire Selection"),
            HelpEntry::Binding("h / ←", "Previous empire"),
            HelpEntry::Binding("l / →", "Next empire"),
            HelpEntry::Section("Other Fields"),
            HelpEntry::Binding("h / ←", "Decrease / cycle left"),
            HelpEntry::Binding("l / →", "Increase / cycle right"),
            HelpEntry::Binding("Enter", "Use selected field action"),
            HelpEntry::Binding("S", "Start game immediately"),
            HelpEntry::Binding("Esc", "Cancel seed edit / Go back to menu"),
            HelpEntry::Section("Seed editing"),
            HelpEntry::Binding("0-9", "Type seed digits"),
            HelpEntry::Binding("Backspace", "Delete last digit"),
            HelpEntry::Binding("Enter", "Confirm seed"),
            HelpEntry::Binding("Esc", "Discard changes"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("?", "Toggle this help"),
        ],
        Screen::SectorOverview => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("h / ←", "Move selection left"),
            HelpEntry::Binding("j / ↓", "Move selection down"),
            HelpEntry::Binding("k / ↑", "Move selection up"),
            HelpEntry::Binding("l / →", "Move selection right"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Enter selected sector (Sector Map)"),
            HelpEntry::Binding("L", "Toggle inter-sector hyperspace lane overlay"),
            HelpEntry::Binding("r", "Open research screen"),
            HelpEntry::Binding("D", "Open diplomacy screen"),
            HelpEntry::Binding("E / T", "End turn (AI acts automatically)"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Q", "Quit"),
        ],
        Screen::SectorMap => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("h / ←", "Move selection left"),
            HelpEntry::Binding("j / ↓", "Move selection down"),
            HelpEntry::Binding("k / ↑", "Move selection up"),
            HelpEntry::Binding("l / →", "Move selection right"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Open selected system in System View"),
            HelpEntry::Binding("c", "Enter colony (if colonized star selected)"),
            HelpEntry::Binding("r", "Open research screen"),
            HelpEntry::Binding("D", "Open diplomacy screen"),
            HelpEntry::Binding("S", "Dispatch scout to selected unexplored system"),
            HelpEntry::Binding("M", "Move idle fleet to selected explored system"),
            HelpEntry::Binding("R", "Confirm rally point for selected colony (pick mode)"),
            HelpEntry::Binding("E / T", "End turn (AI acts automatically)"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load · clear-rally)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Galaxy Overview (or cancel pick mode)"),
            HelpEntry::Binding("Q", "Quit"),
        ],
        Screen::System => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Select next planet"),
            HelpEntry::Binding("k / ↑", "Select previous planet"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Open selected/first player-owned colony"),
            HelpEntry::Binding("S", "Survey selected planet with science ship"),
            HelpEntry::Binding(
                "C",
                "Colonize selected planet with idle colony ship in system",
            ),
            HelpEntry::Binding(
                "I",
                "Invade selected hostile colony with idle troop transport in system",
            ),
            HelpEntry::Note("Planet detail shows colony trade/supply status"),
            HelpEntry::Note("⚔ marker indicates a blockaded colony (hostile fleet present)"),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::Colony => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("Tab", "Switch between role selector / build picker"),
            HelpEntry::Binding("j / ↓", "Move cursor down in active panel"),
            HelpEntry::Binding("k / ↑", "Move cursor up in active panel"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding(
                "Enter",
                "Assign role or queue production item (active panel)",
            ),
            HelpEntry::Binding("R", "Set rally point — navigate to a star and press R"),
            HelpEntry::Binding("X", "Clear rally point for this colony"),
            HelpEntry::Note("Colony panel shows Connected/Isolated supply state"),
            HelpEntry::Note(
                "Blockade: ⚔ means hostile fleet holds orbit — no food, -50% yield, -stability",
            ),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load · clear-rally)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::EmpireOverview => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Select next colony"),
            HelpEntry::Binding("k / ↑", "Select previous colony"),
            HelpEntry::Binding("s", "Cycle sort mode"),
            HelpEntry::Binding("/", "Filter colonies"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Open selected colony"),
            HelpEntry::Binding("S", "Open selected system"),
            HelpEntry::Note("Rows show supply connectivity and blockade warnings"),
            HelpEntry::Note(
                "'Blockaded' warning = hostile fleet in orbit (no food, -50% yield, -stability)",
            ),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::Research => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move cursor through technology tree"),
            HelpEntry::Binding("k / ↑", "Move cursor through technology tree"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Set highlighted tech as active research"),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::Diplomacy => vec![
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("e / t / Enter", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O", "Open empire overview"),
            HelpEntry::Binding(":", "Command palette (save · load)"),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
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
            HelpEntry::Note(desc) => Line::from(vec![
                Span::styled(format!("{:>14}", "·"), Theme::dim_border_style()),
                Span::raw("  "),
                Span::styled(*desc, Theme::muted_style()),
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
    fn render_help_sector_overview() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::SectorOverview);
            })
            .unwrap();
    }

    #[test]
    fn render_help_sector_map() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::SectorMap);
            })
            .unwrap();
    }

    #[test]
    fn render_help_system() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::System);
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
    fn render_help_empire_overview() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_help(frame, area, &Screen::EmpireOverview);
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
                render_help(frame, area, &Screen::SectorOverview);
            })
            .unwrap();
    }
}
