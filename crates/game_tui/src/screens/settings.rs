//! Settings screen — update channel, auto-update toggle, visual mode.

use crate::theme::Theme;
use crate::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const NUM_SETTINGS: usize = 3;

pub fn render_settings(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let box_width = 52u16;
    let box_height = 14u16;

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

    let items = settings_lines(app_state);

    let paragraph = Paragraph::new(items)
        .block(
            Block::default()
                .title(" Settings ")
                .borders(Borders::ALL)
                .border_style(Theme::default_style())
                .style(Theme::default_style()),
        )
        .alignment(Alignment::Left)
        .style(Theme::default_style());

    frame.render_widget(paragraph, panel);
}

fn settings_lines(app_state: &AppState) -> Vec<Line<'static>> {
    let cursor = app_state.settings_cursor;
    let mut lines = vec![Line::from(""), Line::from("  Use j/k to navigate, Enter to cycle, Esc to save."), Line::from("")];

    let entries: &[(&str, String)] = &[
        (
            "Visual Mode",
            app_state.visual_mode.label().to_string(),
        ),
        (
            "Update Channel",
            app_state.update_channel.label().to_string(),
        ),
        (
            "Auto-update",
            if app_state.auto_update { "On".to_string() } else { "Off".to_string() },
        ),
    ];

    for (i, (label, value)) in entries.iter().enumerate() {
        let (row_style, marker) = if i == cursor {
            (
                Style::default()
                    .fg(Theme::accent())
                    .add_modifier(Modifier::BOLD),
                ">",
            )
        } else {
            (Theme::default_style(), " ")
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {marker} {label:<18}"), row_style),
            Span::styled(format!("[{value}]"), row_style),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Esc  Save & return",
        Theme::muted_style(),
    )));

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
}
