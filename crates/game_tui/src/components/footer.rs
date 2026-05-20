//! Footer component

use crate::screens::Screen;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

fn line_width(line: &Line<'_>) -> usize {
    line.width()
}

fn push_wrapped_hint_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    hints: &'a [(&'a str, &'a str)],
    max_width: usize,
) {
    if hints.is_empty() || max_width == 0 {
        return;
    }

    let separator = Span::styled("  │  ", Theme::dim_border_style());
    let separator_width = separator.width();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_width = 0usize;

    for (index, (key, desc)) in hints.iter().enumerate() {
        let entry = vec![
            Span::styled((*key).to_string(), Theme::title_style()),
            Span::raw(" ".to_string()),
            Span::styled((*desc).to_string(), Theme::muted_style()),
        ];
        let entry_line = Line::from(entry.clone());
        let entry_width = line_width(&entry_line);
        let needs_separator = !current_spans.is_empty();
        let projected_width =
            current_width + if needs_separator { separator_width } else { 0 } + entry_width;

        if needs_separator && projected_width > max_width {
            lines.push(Line::from(std::mem::take(&mut current_spans)));
            current_width = 0;
        }

        if !current_spans.is_empty() {
            current_spans.push(separator.clone());
            current_width += separator_width;
        }
        current_spans.extend(entry);
        current_width += entry_width;

        if index == hints.len() - 1 && !current_spans.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current_spans)));
        }
    }
}

/// Render the footer with contextual hints
pub fn render_footer(frame: &mut Frame, area: Rect, screen: &Screen, context: Option<&str>) {
    let hints = match screen {
        Screen::Menu => vec![
            ("[N]", "New Game"),
            ("[L]", "Load"),
            ("[V]", "Visual Mode"),
            ("[Q]", "Quit"),
        ],
        Screen::EmpireSelect => vec![
            ("[j/k ↑↓]", "Browse"),
            ("[Enter]", "Confirm"),
            ("[Esc]", "Back"),
            ("[?]", "Help"),
        ],
        Screen::NewGameSetup => vec![
            ("[j/k]", "Select Field"),
            ("[h/l]", "Change Value"),
            ("[Enter]", "Use Field"),
            ("[S]", "Start"),
            ("[Esc]", "Back"),
            ("[?]", "Help"),
        ],
        Screen::SectorOverview => vec![
            ("[hjkl/←↓↑→]", "Move"),
            ("[Enter]", "Sector Map"),
            ("[L]", "Toggle Lanes"),
            ("[r]", "Research"),
            ("[D]", "Diplomacy"),
            ("[O/V]", "Overview/Victory"),
            ("[E/T]", "End Turn"),
            ("[?]", "Help"),
            ("[:]", "Command"),
            ("[Q]", "Quit"),
        ],
        Screen::SectorMap => vec![
            ("[hjkl/←↓↑→]", "Move"),
            ("[Enter]", "System View"),
            ("[c]", "Colony"),
            ("[r]", "Research"),
            ("[D]", "Diplomacy"),
            ("[O/V]", "Overview/Victory"),
            ("[S]", "Scout"),
            ("[M]", "Move Fleet"),
            ("[E/T]", "End Turn"),
            ("[?]", "Help"),
            ("[:]", "Command"),
            ("[Esc]", "Galaxy"),
            ("[Q]", "Quit"),
        ],
        Screen::System => vec![
            ("[j/k]", "Select Planet"),
            ("[Enter]", "Open Selected/First Player Colony"),
            ("[S]", "Survey"),
            ("[C]", "Colonize Selected"),
            ("[I]", "Invade Selected"),
            ("[f]", "Cycle Fleet Focus"),
            ("[R]", "Next Fleet Role"),
            ("[F]", "Next Fleet Formation"),
            ("[O/V]", "Overview/Victory"),
            ("[e/t]", "End Turn"),
            ("[?]", "Help"),
            ("[Esc]", "Back"),
        ],
        Screen::Colony => vec![
            ("[j/k]", "Select"),
            ("[Enter]", "Queue"),
            ("[R]", "Set Rally"),
            ("[X]", "Clear Rally"),
            ("[O/V]", "Overview/Victory"),
            ("[e/t]", "End Turn"),
            ("[?]", "Help"),
            ("[Esc]", "Back"),
        ],
        Screen::EmpireOverview => vec![
            ("[j/k]", "Select"),
            ("[s]", "Sort"),
            ("[/]", "Filter"),
            ("[Enter]", "Colony"),
            ("[S]", "System"),
            ("[e/t]", "End Turn"),
            ("[Esc]", "Back"),
        ],
        Screen::Research => vec![
            ("[j/k]", "Tree Cursor"),
            ("[Tab]", "Domain"),
            ("[[]", "Era"),
            ("[]]", "Status"),
            ("[/]", "Search"),
            ("[Enter]", "Set Active Tech"),
            ("[a]", "Queue"),
            ("[x]", "Remove"),
            ("[u/d]", "Reorder"),
            ("[c]", "Clear Queue"),
            ("[O/V]", "Overview/Victory"),
            ("[e/t]", "End Turn"),
            ("[?]", "Help"),
            ("[Esc]", "Back"),
        ],
        Screen::Diplomacy => vec![
            ("[e/t/Enter]", "End Turn"),
            ("[O/V]", "Overview/Victory"),
            ("[?]", "Help"),
            ("[Esc]", "Back"),
        ],
        Screen::ShipDesigner => vec![
            ("[n]", "New Design"),
            ("[j/k]", "Navigate"),
            ("[h/l]", "Component"),
            ("[Enter]", "Confirm"),
            ("[Tab]", "Panel"),
            ("[s]", "Save"),
            ("[d]", "Delete"),
            ("[Esc]", "Back"),
            ("[?]", "Help"),
        ],
        Screen::Settings => vec![
            ("[j/k]", "Navigate"),
            ("[Enter]", "Cycle"),
            ("[Esc]", "Save & Return"),
        ],
    };

    let inner_width = usize::from(area.width.saturating_sub(2)).max(1);
    let mut lines = Vec::new();
    push_wrapped_hint_lines(&mut lines, &hints, inner_width);
    if let Some(context_line) = context {
        lines.push(Line::from(vec![
            Span::styled("Hint: ", Theme::title_style()),
            Span::styled(context_line, Theme::muted_style()),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::TOP))
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_footer_menu() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Menu, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_new_game_setup() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::NewGameSetup, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_sector_overview() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::SectorOverview, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_sector_map() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::SectorMap, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_system() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::System, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_colony() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Colony, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_research() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Research, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_empire_overview() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::EmpireOverview, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_diplomacy() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Diplomacy, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_ship_designer() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_footer(frame, frame.area(), &Screen::ShipDesigner, None);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_sector_map_wide_terminal() {
        // Wider terminal to verify hint separators don't panic
        let backend = TestBackend::new(200, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::SectorMap, Some("Test hint"));
            })
            .unwrap();
    }
}
