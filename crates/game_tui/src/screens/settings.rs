//! Settings modal overlay — update channel, auto-update toggle, visual mode.

use crate::components::{key_hint, panel_block, section_heading};
use crate::theme::Theme;
use crate::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

const NUM_SETTINGS: usize = 3;

/// A single configurable setting row.
struct SettingEntry {
    category: &'static str,
    label: &'static str,
    value: String,
    description: &'static str,
}

fn setting_entries(app_state: &AppState) -> [SettingEntry; NUM_SETTINGS] {
    [
        SettingEntry {
            category: "Display",
            label: "Visual Mode",
            value: app_state.visual_mode.label().to_string(),
            description: "Glyph set for icons and art (ASCII · Unicode · NerdFont).",
        },
        SettingEntry {
            category: "Updates",
            label: "Update Channel",
            value: app_state.update_channel.label().to_string(),
            description: "Release stream new versions are pulled from.",
        },
        SettingEntry {
            category: "Updates",
            label: "Auto-update",
            value: if app_state.auto_update {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            description: "Check for and apply updates on launch.",
        },
    ]
}

pub fn render_settings(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let box_width = 60u16.min(area.width);
    let box_height = 18u16.min(area.height);

    // Center vertically and horizontally
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(box_height),
            Constraint::Fill(1),
        ])
        .split(area);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(box_width),
            Constraint::Fill(1),
        ])
        .split(v_chunks[1]);

    let panel = h_chunks[1];

    // Clear the area beneath the modal so the background doesn't bleed through.
    frame.render_widget(Clear, panel);

    let block = panel_block("⚙ Settings", true);
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let paragraph = Paragraph::new(settings_lines(app_state)).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

fn settings_lines(app_state: &AppState) -> Vec<Line<'static>> {
    let cursor = app_state.settings_cursor.min(NUM_SETTINGS - 1);
    let entries = setting_entries(app_state);

    let mut lines = vec![
        Line::from(Span::styled(
            "  Tune how FARSPACE looks and stays current.",
            Theme::muted_style(),
        )),
        Line::from(""),
    ];

    let mut last_category: Option<&str> = None;
    for (i, entry) in entries.iter().enumerate() {
        if last_category != Some(entry.category) {
            lines.push(section_heading(format!("  {}", entry.category)));
            last_category = Some(entry.category);
        }

        let selected = i == cursor;
        let marker = if selected { "▶" } else { " " };
        let row_style = if selected {
            Theme::highlight_style()
        } else {
            Theme::text_primary_style()
        };
        let value_style = if selected {
            Theme::highlight_style()
        } else {
            Theme::accent_style()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {marker} {:<16}", entry.label), row_style),
            Span::styled(format!(" [{}]", entry.value), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::raw("      "),
            Span::styled(entry.description, Theme::muted_style()),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from({
        let mut spans = vec![Span::raw("  ")];
        spans.extend(key_hint("j / k", "Navigate"));
        spans.push(Span::styled("   ", Theme::muted_style()));
        spans.extend(key_hint("Enter", "Cycle value"));
        spans.push(Span::styled("   ", Theme::muted_style()));
        spans.extend(key_hint("Esc", "Save & return"));
        spans
    }));

    lines
}

pub fn settings_cursor_count() -> usize {
    NUM_SETTINGS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn settings_renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app_state = AppState::default();
        terminal
            .draw(|frame| render_settings(frame, frame.area(), &app_state))
            .unwrap();
    }

    #[test]
    fn settings_renders_cursor_row() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app_state = AppState {
            settings_cursor: 1,
            ..AppState::default()
        };
        terminal
            .draw(|frame| render_settings(frame, frame.area(), &app_state))
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(rendered.contains("Update Channel"));
    }

    #[test]
    fn settings_shows_categories_descriptions_and_controls() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app_state = AppState::default();
        terminal
            .draw(|frame| render_settings(frame, frame.area(), &app_state))
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(rendered.contains("Display"));
        assert!(rendered.contains("Updates"));
        assert!(rendered.contains("Glyph set"));
        assert!(rendered.contains("Save & return"));
        // Selected row (cursor 0 = Visual Mode) marker is visible.
        assert!(rendered.contains('▶'));
    }
}
