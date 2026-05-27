//! Footer component

use crate::screens::Screen;
use crate::theme::Theme;
use crate::components::key_hint;
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
        let entry = key_hint(key, desc);
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
            ("Enter", "Select"),
            ("↑↓", "Navigate"),
            ("?", "Help"),
            ("Esc", "Quit"),
        ],
        Screen::EmpireSelect => vec![
            ("↑↓ / j k", "Browse"),
            ("Enter", "Confirm"),
            ("Esc", "Back"),
            ("?", "Help"),
        ],
        Screen::NewGameSetup => vec![
            ("↑↓ / j k", "Navigate"),
            ("←→ / h l", "Adjust"),
            ("Enter", "Edit / Start"),
            ("Esc", "Back"),
            ("?", "Help"),
        ],
        Screen::SectorOverview => vec![
            ("←↓↑→ / h j k l", "Move"),
            ("Enter", "Sector Map"),
            ("L", "Lanes"),
            ("R", "Research"),
            ("D", "Diplomacy"),
            ("O / V", "Overview"),
            ("E / T", "End Turn"),
            ("?", "Help"),
            (":", "Command"),
            ("Q", "Quit"),
        ],
        Screen::SectorMap => vec![
            ("←↓↑→ / h j k l", "Move"),
            ("Enter", "System"),
            ("C", "Colony"),
            ("R", "Research"),
            ("D", "Diplomacy"),
            ("O / V", "Overview"),
            ("S", "Scout"),
            ("M", "Move Fleet"),
            ("E / T", "End Turn"),
            ("?", "Help"),
            (":", "Command"),
            ("Esc", "Galaxy"),
            ("Q", "Quit"),
        ],
        Screen::System => vec![
            ("J / K", "Select Planet"),
            ("Enter", "Open Colony"),
            ("S", "Survey"),
            ("C", "Colonize"),
            ("I", "Invade"),
            ("F", "Fleet"),
            ("O / V", "Overview"),
            ("E / T", "End Turn"),
            ("?", "Help"),
            ("Esc", "Back"),
        ],
        Screen::Colony => vec![
            ("J / K", "Select"),
            ("Enter", "Queue"),
            ("R / X", "Rally"),
            ("O / V", "Overview"),
            ("E / T", "End Turn"),
            ("?", "Help"),
            ("Esc", "Back"),
        ],
        Screen::EmpireOverview => vec![
            ("J / K", "Select"),
            ("S", "Sort"),
            ("/", "Filter"),
            ("Enter", "Colony"),
            ("Shift+S", "System"),
            ("E / T", "End Turn"),
            ("Esc", "Back"),
        ],
        Screen::Research => vec![
            ("J / K", "Tree"),
            ("Tab", "Domain"),
            ("[ / ]", "Era / Status"),
            ("/", "Search"),
            ("Enter", "Set Active"),
            ("A / X", "Queue"),
            ("U / D", "Reorder"),
            ("C", "Clear Queue"),
            ("O / V", "Overview"),
            ("E / T", "End Turn"),
            ("?", "Help"),
            ("Esc", "Back"),
        ],
        Screen::Diplomacy => vec![
            ("Enter / E / T", "End Turn"),
            ("O / V", "Overview"),
            ("?", "Help"),
            ("Esc", "Back"),
        ],
        Screen::ShipDesigner => vec![
            ("N", "New"),
            ("J / K", "Navigate"),
            ("H / L", "Component"),
            ("Enter", "Confirm"),
            ("Tab", "Panel"),
            ("S / D", "Save / Delete"),
            ("Esc", "Back"),
            ("?", "Help"),
        ],
        Screen::Settings => vec![
            ("J / K", "Navigate"),
            ("Enter", "Cycle"),
            ("Esc", "Save & Back"),
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
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Theme::dim_border_style()),
        )
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

    #[test]
    fn footer_renders_at_80x3_with_command_hints() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_footer(frame, frame.area(), &Screen::Menu, None);
            })
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Enter"));
        assert!(rendered.contains("Navigate"));
    }

    #[test]
    fn footer_wraps_on_narrow_width() {
        let backend = TestBackend::new(32, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_footer(frame, frame.area(), &Screen::SectorMap, None);
            })
            .unwrap();
    }
}
