//! Event log component

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Event log storage
#[derive(Debug, Clone)]
pub struct EventLog {
    entries: Vec<String>,
    max_entries: usize,
}

impl Default for EventLog {
    fn default() -> Self {
        EventLog::new()
    }
}

impl EventLog {
    /// Create a new event log with default capacity
    pub fn new() -> Self {
        EventLog {
            entries: Vec::new(),
            max_entries: 50,
        }
    }

    /// Add an entry to the log
    pub fn push(&mut self, entry: String) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// Get the last N entries
    pub fn last_n(&self, n: usize) -> &[String] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Render the event log
pub fn render_log(frame: &mut Frame, area: Rect, log: &EventLog) {
    let visible_lines = (area.height.saturating_sub(2)) as usize;
    let entries = log.last_n(visible_lines);

    let lines: Vec<Line> = entries
        .iter()
        .map(|entry| {
            let style = if entry.starts_with("Error:") {
                Theme::error_style()
            } else {
                Theme::muted_style()
            };
            Line::from(Span::styled(entry.clone(), style))
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Event Log ")
                .borders(Borders::ALL)
                .style(Theme::default_style()),
        )
        .style(Theme::default_style());

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn event_log_push_and_retrieve() {
        let mut log = EventLog::new();
        log.push("Event 1".to_string());
        log.push("Event 2".to_string());

        assert_eq!(log.len(), 2);
        assert_eq!(log.last_n(2), &["Event 1", "Event 2"]);
    }

    #[test]
    fn event_log_trims_old_entries() {
        let mut log = EventLog {
            entries: Vec::new(),
            max_entries: 3,
        };

        log.push("A".to_string());
        log.push("B".to_string());
        log.push("C".to_string());
        log.push("D".to_string());

        assert_eq!(log.len(), 3);
        assert_eq!(log.last_n(3), &["B", "C", "D"]);
    }

    #[test]
    fn render_log_no_panic() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut log = EventLog::new();
        log.push("Turn 2 begins".to_string());
        log.push("Error: Something went wrong".to_string());

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_log(frame, area, &log);
            })
            .unwrap();
    }
}
