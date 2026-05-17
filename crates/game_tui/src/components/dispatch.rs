//! Galactic Dispatch modal component

use crate::layout::centered_rect;
use crate::theme::Theme;
use game_core::{DispatchCategory, DispatchSeverity, GalacticDispatch};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render the Galactic Dispatch modal overlay.
///
/// `dispatch` is the dispatch to display.
/// `dispatch_index` is the index into history (0 = oldest, len-1 = newest).
/// `total_count` is the total number of dispatches in history.
pub fn render_dispatch(
    frame: &mut Frame,
    area: Rect,
    dispatch: &GalacticDispatch,
    dispatch_index: usize,
    total_count: usize,
) {
    let popup_area = centered_rect(90, 88, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let separator_len = (popup_area.width as usize).saturating_sub(4).clamp(8, 76);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // Header: turn number and dispatch index
    let display_turn = dispatch.turn + 1;
    let header_text = format!(
        " Turn {}  ·  Dispatch {}/{} ",
        display_turn,
        dispatch_index + 1,
        total_count
    );
    lines.push(Line::from(Span::styled(header_text, Theme::title_style())));

    // Separator
    lines.push(Line::from(Span::styled(
        "─".repeat(separator_len),
        Theme::dim_border_style(),
    )));

    if dispatch.items.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No significant events to report this cycle.",
            Theme::muted_style(),
        )));
    } else {
        for item in &dispatch.items {
            lines.push(Line::from(""));

            // Severity marker + category label + headline on one line
            let (severity_prefix, severity_style) = match item.severity {
                DispatchSeverity::Historic => ("[★★] ", Theme::accent_style()),
                DispatchSeverity::Urgent => ("[!!] ", Theme::error_style()),
                DispatchSeverity::Notable => ("[»] ", Theme::title_style()),
                DispatchSeverity::Notice => ("[·] ", Theme::muted_style()),
            };

            let category_label = match item.category {
                DispatchCategory::Exploration => "[EXPLORATION]",
                DispatchCategory::Colonization => "[COLONIZATION]",
                DispatchCategory::Research => "[RESEARCH]",
                DispatchCategory::Economy => "[ECONOMY]",
                DispatchCategory::Diplomacy => "[DIPLOMACY]",
                DispatchCategory::War => "[WAR]",
                DispatchCategory::Blockades => "[BLOCKADES]",
                DispatchCategory::Invasions => "[INVASIONS]",
                DispatchCategory::Trade => "[TRADE]",
                DispatchCategory::VictoryRace => "[VICTORY RACE]",
                DispatchCategory::MinorFactions => "[MINOR FACTIONS]",
            };

            lines.push(Line::from(vec![
                Span::styled(severity_prefix, severity_style),
                Span::styled(category_label, Theme::dim_border_style()),
                Span::raw(" "),
                Span::styled(item.headline.as_str(), Theme::title_style()),
            ]));

            // Body, indented
            if !item.body.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", item.body),
                    Theme::muted_style(),
                )));
            }
        }
    }

    // Separator before footer
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─".repeat(separator_len),
        Theme::dim_border_style(),
    )));

    // Footer: navigation hint + close hint
    let mut footer_spans: Vec<Span> = Vec::new();
    if total_count > 1 {
        footer_spans.push(Span::styled("← prev / → next", Theme::muted_style()));
        footer_spans.push(Span::styled("  ·  ", Theme::dim_border_style()));
    }
    footer_spans.push(Span::styled("Esc", Theme::title_style()));
    footer_spans.push(Span::styled(" to close", Theme::muted_style()));
    lines.push(Line::from(footer_spans));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" ◈ GALACTIC DISPATCH ◈ ")
                .borders(Borders::ALL)
                .border_style(Theme::focused_border_style())
                .style(Theme::default_style()),
        )
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{DispatchCategory, DispatchItem, DispatchSeverity, GalacticDispatch};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn make_dispatch_with_items() -> GalacticDispatch {
        GalacticDispatch {
            turn: 4,
            title: "Galactic Dispatch — Turn 5".to_string(),
            items: vec![
                DispatchItem {
                    category: DispatchCategory::Exploration,
                    severity: DispatchSeverity::Notice,
                    headline: "Scout reached the outer rim".to_string(),
                    body: "Your scout fleet has completed its survey run.".to_string(),
                    related_empire_id: None,
                    related_star_id: None,
                    related_planet_index: None,
                },
                DispatchItem {
                    category: DispatchCategory::War,
                    severity: DispatchSeverity::Urgent,
                    headline: "Hostile fleet detected".to_string(),
                    body: "An enemy fleet has entered your territory.".to_string(),
                    related_empire_id: None,
                    related_star_id: None,
                    related_planet_index: None,
                },
                DispatchItem {
                    category: DispatchCategory::Research,
                    severity: DispatchSeverity::Historic,
                    headline: "Breakthrough in quantum mechanics".to_string(),
                    body: "Your scientists have made a historic discovery.".to_string(),
                    related_empire_id: None,
                    related_star_id: None,
                    related_planet_index: None,
                },
                DispatchItem {
                    category: DispatchCategory::Colonization,
                    severity: DispatchSeverity::Notable,
                    headline: "New colony established".to_string(),
                    body: "A colony has been founded on a distant world.".to_string(),
                    related_empire_id: None,
                    related_star_id: None,
                    related_planet_index: None,
                },
            ],
        }
    }

    fn make_dispatch_empty() -> GalacticDispatch {
        GalacticDispatch {
            turn: 9,
            title: "Galactic Dispatch — Turn 10".to_string(),
            items: vec![],
        }
    }

    #[test]
    fn render_dispatch_no_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let dispatch = make_dispatch_with_items();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dispatch(frame, area, &dispatch, 2, 3);
            })
            .unwrap();
    }

    #[test]
    fn render_dispatch_empty_items_no_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let dispatch = make_dispatch_empty();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dispatch(frame, area, &dispatch, 0, 1);
            })
            .unwrap();
    }

    #[test]
    fn render_dispatch_tiny_terminal_no_panic() {
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let dispatch = make_dispatch_with_items();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dispatch(frame, area, &dispatch, 0, 1);
            })
            .unwrap();
    }

    #[test]
    fn render_dispatch_single_history_entry_no_nav_hint() {
        // total_count=1 means no nav hint shown — just verify no panic
        let backend = TestBackend::new(100, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let dispatch = make_dispatch_with_items();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dispatch(frame, area, &dispatch, 0, 1);
            })
            .unwrap();
    }
}
