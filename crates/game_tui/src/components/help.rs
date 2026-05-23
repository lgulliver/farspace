//! Help overlay component

use crate::layout::centered_rect;
use crate::screens::Screen;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
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
    let popup_area = centered_rect(80, 80, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = match screen {
        Screen::Menu => " Main Menu Help ",
        Screen::EmpireSelect => " Empire Selection Help ",
        Screen::NewGameSetup => " New Game Setup Help ",
        Screen::SectorOverview => " Galaxy Overview Help ",
        Screen::SectorMap => " Sector Map Help ",
        Screen::System => " System View Help ",
        Screen::Colony => " Colony Help ",
        Screen::EmpireOverview => " Empire Overview Help ",
        Screen::Research => " Research Help ",
        Screen::Diplomacy => " Diplomacy Help ",
        Screen::ShipDesigner => " Ship Designer Help ",
        Screen::Settings => " Settings Help ",
    };

    let entries: Vec<HelpEntry> = match screen {
        Screen::Menu => vec![
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("N", "Start a new game"),
            HelpEntry::Binding("L", "Load a saved game"),
            HelpEntry::Binding("V", "Cycle visual mode (ASCII → Unicode → NerdFont)"),
            HelpEntry::Binding(":", "Command palette (save · load · visual-mode)"),
            HelpEntry::Binding("Q", "Quit the game"),
            HelpEntry::Note("NerdFont mode needs terminal fonts with Nerd Font glyphs"),
            HelpEntry::Note(
                "Recommended: JetBrainsMono NF, FiraCode NF, MesloLGS NF, Hack Nerd Font",
            ),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("?", "Toggle this help"),
        ],
        Screen::EmpireSelect => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move to next faction"),
            HelpEntry::Binding("k / ↑", "Move to previous faction"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Confirm empire and open galaxy setup"),
            HelpEntry::Binding("Esc", "Return to main menu"),
            HelpEntry::Note("Left panel selects faction; right panel shows emblem and dossier"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("?", "Toggle this help"),
        ],
        Screen::NewGameSetup => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move to next field"),
            HelpEntry::Binding("k / ↑", "Move to previous field"),
            HelpEntry::Section("Fields"),
            HelpEntry::Binding("h / ←", "Decrease / cycle left"),
            HelpEntry::Binding("l / →", "Increase / cycle right"),
            HelpEntry::Binding("Enter", "Use selected field action"),
            HelpEntry::Binding("S", "Start game immediately"),
            HelpEntry::Binding("Esc", "Cancel seed edit / Return to empire selection"),
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
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · dispatch · news)",
            ),
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
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · clear-rally · dispatch · news)",
            ),
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
            HelpEntry::Binding("f", "Cycle focused player fleet in this system"),
            HelpEntry::Binding("R", "Assign next strategic role to focused fleet"),
            HelpEntry::Binding("F", "Assign next formation to focused fleet"),
            HelpEntry::Note("Planet detail shows colony trade/supply status"),
            HelpEntry::Note("⚔ marker indicates a blockaded colony (hostile fleet present)"),
            HelpEntry::Note("Resources show rarity/category once discovered by survey + tech"),
            HelpEntry::Note(
                "Extraction status depends on buildings/orbitals, tech, supply, and blockade",
            ),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · dispatch · news)",
            ),
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
            HelpEntry::Note("Pops auto-fill jobs by colony role and shortage priority"),
            HelpEntry::Note("Housing shortage and unemployment reduce stability"),
            HelpEntry::Note(
                "Blockade: ⚔ means hostile fleet holds orbit — no food, -50% yield, -stability",
            ),
            HelpEntry::Note("Resource lines show rarity/category and extraction status"),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · clear-rally · dispatch · news)",
            ),
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
            HelpEntry::Note("Rows include employment and unemployment pressure"),
            HelpEntry::Note(
                "'Blockaded' warning = hostile fleet in orbit (no food, -50% yield, -stability)",
            ),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · dispatch · news)",
            ),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::Research => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move cursor through technology tree"),
            HelpEntry::Binding("k / ↑", "Move cursor through technology tree"),
            HelpEntry::Binding("Tab", "Cycle domain filter"),
            HelpEntry::Binding("[", "Cycle era filter"),
            HelpEntry::Binding("]", "Cycle status filter"),
            HelpEntry::Binding("/", "Edit search filter"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("Enter", "Set highlighted tech as active research"),
            HelpEntry::Binding("a", "Queue highlighted tech"),
            HelpEntry::Binding("x", "Remove highlighted tech from queue"),
            HelpEntry::Binding("u / d", "Move highlighted queued tech up/down"),
            HelpEntry::Binding("c", "Clear research queue"),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · dispatch · news)",
            ),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::Diplomacy => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("Tab / j / k", "Cycle selected empire"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("w", "Declare war on selected empire"),
            HelpEntry::Binding("p", "Offer peace to selected empire"),
            HelpEntry::Binding("n", "Propose non-aggression pact"),
            HelpEntry::Binding("x", "Cancel non-aggression pact"),
            HelpEntry::Binding("g", "Send diplomatic greeting"),
            HelpEntry::Binding("u", "Issue warning"),
            HelpEntry::Binding("m", "Demand tribute"),
            HelpEntry::Binding("c", "Open diplomatic communication modal"),
            HelpEntry::Binding(
                "Enter",
                "End turn (or respond when communication modal is open)",
            ),
            HelpEntry::Binding("e / t", "End turn"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("O / V", "Open empire overview / victory"),
            HelpEntry::Binding("N", "Open Galactic Dispatch (latest bulletin)"),
            HelpEntry::Binding("B", "Open Battle Reports"),
            HelpEntry::Binding(
                ":",
                "Command palette (save · load · visual-mode · dispatch · news)",
            ),
            HelpEntry::Binding("?", "Toggle this help"),
            HelpEntry::Binding("Esc", "Return to Sector Map"),
        ],
        Screen::ShipDesigner => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Move cursor down (designs / hulls / slots)"),
            HelpEntry::Binding("k / ↑", "Move cursor up"),
            HelpEntry::Binding("h / ←", "Previous component in slot"),
            HelpEntry::Binding("l / →", "Next component in slot"),
            HelpEntry::Binding("Tab", "Cycle panel focus (List → Slots → Stats)"),
            HelpEntry::Section("Actions"),
            HelpEntry::Binding("n", "Start a new design (hull selection mode)"),
            HelpEntry::Binding("Enter", "Confirm selection / enter slot editing"),
            HelpEntry::Binding("s", "Save current design (emits CreateShipDesign)"),
            HelpEntry::Binding("d", "Delete selected design (emits DeleteShipDesign)"),
            HelpEntry::Binding("Esc", "Cancel / return to previous screen"),
            HelpEntry::Note("Locked hulls shown greyed out with (locked) suffix"),
            HelpEntry::Note("Locked components shown muted — unlock via research"),
            HelpEntry::Section("Global"),
            HelpEntry::Binding("W", "Open ship designer from any game screen"),
            HelpEntry::Binding("?", "Toggle this help"),
        ],
        Screen::Settings => vec![
            HelpEntry::Section("Navigation"),
            HelpEntry::Binding("j / ↓", "Next setting"),
            HelpEntry::Binding("k / ↑", "Previous setting"),
            HelpEntry::Binding("Enter / Space", "Cycle value"),
            HelpEntry::Binding("Esc", "Save and return to menu"),
        ],
    };

    let key_width = usize::from(popup_area.width.saturating_sub(8)).clamp(6, 14);
    let divider_len = usize::from(popup_area.width.saturating_sub(4)).clamp(8, 36);

    let lines: Vec<Line> = entries
        .iter()
        .map(|entry| match entry {
            HelpEntry::Section(label) => Line::from(vec![
                Span::raw(" "),
                Span::styled(*label, Theme::accent_style()),
                Span::styled(
                    format!(" {}", "─".repeat(divider_len)),
                    Theme::dim_border_style(),
                ),
            ]),
            HelpEntry::Binding(key, desc) => Line::from(vec![
                Span::styled(
                    format!("{key:>width$}", width = key_width),
                    Theme::title_style(),
                ),
                Span::raw("  "),
                Span::styled(*desc, Theme::default_style()),
            ]),
            HelpEntry::Note(desc) => Line::from(vec![
                Span::styled(
                    format!("{:>width$}", "·", width = key_width),
                    Theme::dim_border_style(),
                ),
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
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });

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
    fn render_help_ship_designer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_help(frame, frame.area(), &Screen::ShipDesigner);
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
