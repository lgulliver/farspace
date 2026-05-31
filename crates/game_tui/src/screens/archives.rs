//! Campaign Archives overlay — a polished save browser.
//!
//! Presents persisted campaigns as "Campaign Archives" rather than a raw file
//! list. Metadata shown here is read from real save data by `game_save`; this
//! module only decides how it looks. No persistence logic lives here.

use crate::components::chrome::{key_hint, page_block};
use crate::theme::Theme;
use game_save::SaveSlotSummary;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};

/// Threshold (inner width) above which a detail side-panel is shown.
const WIDE_DETAIL_WIDTH: u16 = 72;

fn turn_text(turn: Option<u32>) -> String {
    match turn {
        Some(t) => format!("Turn {t}"),
        None => "Unknown turn".to_string(),
    }
}

fn empire_text(name: Option<&str>) -> String {
    name.unwrap_or("Unknown empire").to_string()
}

fn updated_text(updated: Option<&str>) -> String {
    updated.unwrap_or("Unknown date").to_string()
}

/// Render the Campaign Archives overlay. `entries` is most-recent first.
pub fn render_archives(
    frame: &mut Frame,
    area: Rect,
    entries: &[SaveSlotSummary],
    cursor: usize,
    confirm_delete: bool,
    show_help: bool,
    error: Option<&str>,
) {
    if area.width < 8 || area.height < 5 {
        return;
    }

    // Centred panel with a small margin.
    let panel_w = area.width.saturating_sub(4).clamp(40, 96);
    let panel_h = area.height.saturating_sub(2).clamp(8, 28);
    let panel = Rect::new(
        area.x + (area.width.saturating_sub(panel_w)) / 2,
        area.y + (area.height.saturating_sub(panel_h)) / 2,
        panel_w,
        panel_h,
    );

    frame.render_widget(Clear, panel);
    let block = page_block("Campaign Archives");
    let inner = block.inner(panel);
    frame.render_widget(block, panel);
    if inner.width == 0 || inner.height < 2 {
        return;
    }

    // Reserve the bottom row for footer hints.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let body = rows[0];
    let footer = rows[1];

    render_body(frame, body, entries, cursor, error);
    render_footer(frame, footer, entries.is_empty());

    if show_help {
        render_help_box(frame, inner);
    }
    if confirm_delete {
        render_confirm_box(frame, inner, entries.get(cursor));
    }
}

fn render_body(
    frame: &mut Frame,
    area: Rect,
    entries: &[SaveSlotSummary],
    cursor: usize,
    error: Option<&str>,
) {
    if entries.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No saved campaigns found.",
                Theme::text_secondary_style(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press N to begin a new campaign.",
                Theme::muted_style(),
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .style(Theme::default_style()),
            area,
        );
        return;
    }

    let selected = cursor.min(entries.len().saturating_sub(1));

    // An error band spans the full body width above the columns so long,
    // human-readable messages wrap rather than being clipped by the list column.
    let columns_area = if let Some(err) = error {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(area);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                err.to_string(),
                Theme::error_style(),
            )))
            .wrap(Wrap { trim: true })
            .style(Theme::default_style()),
            split[0],
        );
        split[1]
    } else {
        area
    };

    // Split off a detail panel when there is room.
    let (list_area, detail_area) = if columns_area.width >= WIDE_DETAIL_WIDTH {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(28), Constraint::Length(34)])
            .split(columns_area);
        (cols[0], Some(cols[1]))
    } else {
        (columns_area, None)
    };

    render_list(frame, list_area, entries, selected, detail_area.is_some());
    if let Some(detail) = detail_area {
        render_detail(frame, detail, &entries[selected]);
    }
}

fn render_list(
    frame: &mut Frame,
    area: Rect,
    entries: &[SaveSlotSummary],
    selected: usize,
    wide: bool,
) {
    let mut lines: Vec<Line> = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        let is_selected = index == selected;
        let marker = if is_selected { "▶ " } else { "  " };
        let name_style = if is_selected {
            Theme::highlight_style().add_modifier(Modifier::BOLD)
        } else {
            Theme::text_primary_style()
        };

        let mut spans = vec![
            Span::styled(marker, Theme::accent_style()),
            Span::styled(entry.display_name.clone(), name_style),
        ];

        // In the narrow layout the row carries abbreviated metadata inline,
        // since there is no detail panel to show it.
        if !wide {
            if entry.readable {
                spans.push(Span::styled(
                    format!("  ·  {}", turn_text(entry.turn)),
                    Theme::text_secondary_style(),
                ));
            } else {
                spans.push(Span::styled("  ·  unreadable", Theme::warning_style()));
            }
            spans.push(Span::styled(
                format!("  ·  {}", updated_text(entry.updated_at.as_deref())),
                Theme::muted_style(),
            ));
        } else if !entry.readable {
            spans.push(Span::styled("  ·  unreadable", Theme::warning_style()));
        }

        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_detail(frame: &mut Frame, area: Rect, entry: &SaveSlotSummary) {
    let mut lines = vec![
        Line::from(Span::styled(
            entry.display_name.clone(),
            Theme::title_style().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if entry.readable {
        lines.push(detail_row("Turn", turn_text(entry.turn)));
        lines.push(detail_row(
            "Empire",
            empire_text(entry.empire_name.as_deref()),
        ));
        lines.push(detail_row(
            "Galaxy",
            entry
                .galaxy_size
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
        ));
        if let Some(ai) = entry.ai_empires {
            lines.push(detail_row("Empires", format!("{ai} AI")));
        }
        lines.push(detail_row(
            "Difficulty",
            entry.difficulty.clone().unwrap_or_else(|| "Unknown".into()),
        ));
    } else {
        lines.push(Line::from(Span::styled(
            "This archive could not be read.",
            Theme::warning_style(),
        )));
        lines.push(Line::from(Span::styled(
            "It may be corrupted or from a newer build.",
            Theme::muted_style(),
        )));
    }

    lines.push(detail_row(
        "Last played",
        updated_text(entry.updated_at.as_deref()),
    ));

    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn detail_row(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<11} "), Theme::muted_style()),
        Span::styled(value, Theme::text_primary_style()),
    ])
}

fn render_footer(frame: &mut Frame, area: Rect, empty: bool) {
    let mut spans: Vec<Span> = Vec::new();
    let mut push = |pair: Vec<Span<'static>>| {
        if !spans.is_empty() {
            spans.push(Span::raw("   "));
        }
        spans.extend(pair);
    };

    if !empty {
        push(key_hint("↑↓", "Navigate"));
        push(key_hint("Enter", "Load"));
    }
    push(key_hint("N", "New"));
    if !empty {
        push(key_hint("D", "Delete"));
    }
    push(key_hint("Esc", "Back"));
    push(key_hint("?", "Help"));

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Theme::default_style()),
        area,
    );
}

fn render_confirm_box(frame: &mut Frame, area: Rect, entry: Option<&SaveSlotSummary>) {
    let name = entry
        .map(|e| e.display_name.as_str())
        .unwrap_or("this campaign");
    let box_w = area.width.clamp(20, 56);
    let box_h = 5.min(area.height);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(box_w)) / 2,
        area.y + (area.height.saturating_sub(box_h)) / 2,
        box_w,
        box_h,
    );
    frame.render_widget(Clear, rect);
    let block = page_block("Confirm Delete");
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let lines = vec![
        Line::from(Span::styled(
            format!("Delete archive \"{name}\"?"),
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "This cannot be undone.",
            Theme::warning_style(),
        )),
        Line::from({
            let mut spans = key_hint("Enter", "Confirm");
            spans.push(Span::raw("   "));
            spans.extend(key_hint("Esc", "Cancel"));
            spans
        }),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Theme::default_style()),
        inner,
    );
}

fn render_help_box(frame: &mut Frame, area: Rect) {
    let box_w = area.width.clamp(20, 52);
    let box_h = 9.min(area.height);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(box_w)) / 2,
        area.y + (area.height.saturating_sub(box_h)) / 2,
        box_w,
        box_h,
    );
    frame.render_widget(Clear, rect);
    let block = page_block("Archives Help");
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let help_line = |key: &'static str, label: &'static str| Line::from(key_hint(key, label));
    let lines = vec![
        help_line("↑↓ / jk", "Move between campaigns"),
        help_line("Enter", "Load the selected campaign"),
        help_line("N", "Start a new campaign"),
        help_line("D", "Delete (asks to confirm)"),
        help_line("Esc", "Back to the main menu"),
        help_line("?", "Toggle this help"),
    ];
    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use std::path::PathBuf;

    fn summary(name: &str, readable: bool) -> SaveSlotSummary {
        SaveSlotSummary {
            path: PathBuf::from(format!("{name}.sav")),
            display_name: name.to_string(),
            turn: if readable { Some(12) } else { None },
            empire_name: if readable {
                Some("Solar Concord".to_string())
            } else {
                None
            },
            galaxy_size: if readable {
                Some("Medium".to_string())
            } else {
                None
            },
            ai_empires: if readable { Some(3) } else { None },
            difficulty: if readable {
                Some("Standard".to_string())
            } else {
                None
            },
            updated_at: Some("2026-05-31 12:00".to_string()),
            readable,
        }
    }

    fn render(width: u16, height: u16, f: impl FnOnce(&mut Frame)) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| f(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_empty_state() {
        let text = render(80, 24, |frame| {
            render_archives(frame, frame.area(), &[], 0, false, false, None);
        });
        assert!(text.contains("Campaign Archives"));
        assert!(text.contains("No saved campaigns found"));
        assert!(text.contains("New"));
    }

    #[test]
    fn renders_single_save_with_metadata() {
        let entries = vec![summary("Vanguard", true)];
        let text = render(100, 28, |frame| {
            render_archives(frame, frame.area(), &entries, 0, false, false, None);
        });
        assert!(text.contains("Vanguard"));
        assert!(text.contains("Turn 12"));
        assert!(text.contains("Solar Concord"));
        assert!(text.contains("Load"));
        assert!(text.contains("Delete"));
    }

    #[test]
    fn renders_many_saves() {
        let entries = vec![
            summary("Alpha", true),
            summary("Beta", true),
            summary("Gamma", true),
        ];
        let text = render(100, 28, |frame| {
            render_archives(frame, frame.area(), &entries, 1, false, false, None);
        });
        assert!(text.contains("Alpha"));
        assert!(text.contains("Beta"));
        assert!(text.contains("Gamma"));
        // The selected (index 1) row carries the marker.
        assert!(text.contains("▶ Beta"));
    }

    #[test]
    fn unreadable_save_is_marked() {
        let entries = vec![summary("Corrupt", false)];
        let text = render(80, 24, |frame| {
            render_archives(frame, frame.area(), &entries, 0, false, false, None);
        });
        assert!(text.contains("Corrupt"));
        assert!(text.contains("unreadable"));
    }

    #[test]
    fn renders_error_without_panic() {
        let entries = vec![summary("Vanguard", true)];
        let text = render(90, 26, |frame| {
            render_archives(
                frame,
                frame.area(),
                &entries,
                0,
                false,
                false,
                Some("Could not load campaign: save file is corrupted or incomplete."),
            );
        });
        assert!(text.contains("corrupted or incomplete"));
    }

    #[test]
    fn renders_delete_confirmation() {
        let entries = vec![summary("Vanguard", true)];
        let text = render(90, 26, |frame| {
            render_archives(frame, frame.area(), &entries, 0, true, false, None);
        });
        assert!(text.contains("This cannot be undone"));
        assert!(text.contains("Confirm"));
        assert!(text.contains("Cancel"));
    }

    #[test]
    fn renders_help_box() {
        let entries = vec![summary("Vanguard", true)];
        let text = render(90, 26, |frame| {
            render_archives(frame, frame.area(), &entries, 0, false, true, None);
        });
        assert!(text.contains("Archives Help"));
    }

    #[test]
    fn does_not_panic_at_tiny_size() {
        render(6, 4, |frame| {
            render_archives(frame, frame.area(), &[], 0, false, false, None);
        });
    }
}
