//! Main menu screen

use crate::components::render_footer;
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Each line of the FARSPACE ASCII art title — kept as separate entries so
/// ratatui renders them as individual rows (spans with embedded `\n` are
/// stripped by ratatui and would collapse the art into a single garbled line).
const TITLE_LINES: &[&str] = &[
    "  ███████╗ █████╗ ██████╗ ███████╗██████╗  █████╗  ██████╗███████╗",
    "  ██╔════╝██╔══██╗██╔══██╗██╔════╝██╔══██╗██╔══██╗██╔════╝██╔════╝",
    "  █████╗  ███████║██████╔╝███████╗██████╔╝███████║██║     █████╗  ",
    "  ██╔══╝  ██╔══██║██╔══██╗╚════██║██╔═══╝ ██╔══██║██║     ██╔══╝  ",
    "  ██║     ██║  ██║██║  ██║███████║██║     ██║  ██║╚██████╗███████╗",
    "  ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚══════╝",
];

/// Height of the menu box with title, actions, and quickstart guidance.
const MENU_BOX_HEIGHT: u16 = 24;

/// Width of the menu box: wide enough for the 66-char title plus borders/padding
const MENU_BOX_WIDTH: u16 = 70;

/// Render the main menu
pub fn render_menu(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let (_header_area, main_area, footer_area) = compose_layout(area);

    // Footer shows keyboard hints for the menu screen
    let menu_hint = app_state.status_message.as_deref().or(Some(
        "Quickstart: N, Enter, r, Enter, then S/C in system view.",
    ));
    render_footer(frame, footer_area, &Screen::Menu, menu_hint);

    // Center the menu box vertically within the main area
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(MENU_BOX_HEIGHT),
            Constraint::Fill(1),
        ])
        .split(main_area);

    // Center the menu box horizontally
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(MENU_BOX_WIDTH),
            Constraint::Fill(1),
        ])
        .split(v_chunks[1]);

    let menu_area = h_chunks[1];

    // Build menu items — each title row is its own Line so ratatui renders
    // them on separate rows (a single Span with '\n' chars would be stripped).
    let mut menu_items: Vec<Line> = vec![Line::from("")];
    for line in TITLE_LINES {
        menu_items.push(Line::from(Span::styled(*line, Theme::title_style())));
    }
    menu_items.extend([
        Line::from(""),
        Line::from(vec![
            Span::styled("[N]", Theme::title_style()),
            Span::raw(" New Game"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[L]", Theme::title_style()),
            Span::raw(" Load Game"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Q]", Theme::title_style()),
            Span::raw(" Quit"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "First Turn Quickstart",
            Theme::title_style(),
        )]),
        Line::from("  1) [N] New Game"),
        Line::from("  2) [Enter] to open Sector Map and pick a star"),
        Line::from("  3) [Enter] in a system, then [S] survey and [C] colonize"),
        Line::from("  4) [r] research and [Enter] to choose active tech"),
        Line::from("  5) [:] then save / load"),
    ]);

    let paragraph = Paragraph::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Theme::default_style()),
        )
        .alignment(Alignment::Center)
        .style(Theme::default_style());

    frame.render_widget(paragraph, menu_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    /// Extract the first char of a cell's symbol, falling back to a space.
    fn cell_char(buf: &Buffer, x: u16, y: u16) -> char {
        buf.cell((x, y))
            .and_then(|c| c.symbol().chars().next())
            .unwrap_or(' ')
    }

    /// Render the menu into a 100×30 test buffer.
    fn render_to_buffer() -> Buffer {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_menu(frame, frame.area(), &AppState::default()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn menu_screen_renders_without_panic() {
        render_to_buffer();
    }

    /// Title must appear as separate rows — if all lines collapse to one the
    /// first row of the title art will NOT appear on line 1 of the buffer.
    #[test]
    fn title_lines_are_rendered_on_separate_rows() {
        let buf = render_to_buffer();
        let rows: Vec<String> = (0..30u16)
            .map(|y| (0..100u16).map(|x| cell_char(&buf, x, y)).collect())
            .collect();

        // The first and last title lines start with distinct box-drawing sequences.
        let first_title_fragment = "███████"; // top of F
        let last_title_fragment = "╚═╝"; // bottom row of the art

        let has_first = rows.iter().any(|r| r.contains(first_title_fragment));
        let has_last = rows.iter().any(|r| r.contains(last_title_fragment));

        assert!(
            has_first,
            "First title art row not found — title lines may have collapsed"
        );
        assert!(
            has_last,
            "Last title art row not found — title lines may have collapsed"
        );

        // They must appear on *different* rows.
        let first_row = rows.iter().position(|r| r.contains(first_title_fragment));
        let last_row = rows.iter().position(|r| r.contains(last_title_fragment));
        assert_ne!(
            first_row, last_row,
            "First and last title rows appear on the same line — art collapsed"
        );
    }

    /// Footer keyboard hints must be visible on the menu screen.
    #[test]
    fn menu_footer_hints_are_rendered() {
        let buf = render_to_buffer();
        let full: String = (0..30u16)
            .flat_map(|y| (0..100u16).map(move |x| (x, y)))
            .map(|(x, y)| cell_char(&buf, x, y))
            .collect();

        assert!(
            full.contains("[N]"),
            "Footer hint [N] missing from menu screen"
        );
        assert!(
            full.contains("[L]"),
            "Footer hint [L] missing from menu screen"
        );
        assert!(
            full.contains("[Q]"),
            "Footer hint [Q] missing from menu screen"
        );
    }
}
