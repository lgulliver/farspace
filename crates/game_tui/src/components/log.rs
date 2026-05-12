//! Event log component

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
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

/// Categorise a log entry and return an appropriate style.
///
/// Priority order (first match wins):
/// 1. `Error:` prefix → error (red)
/// 2. Turn / game-start keywords → accent (cyan)
/// 3. Research keywords → success (green)
/// 4. Colony / colonize keywords → accent (cyan)  (`"colon"` catches colony/colonize/colonization)
/// 5. Scout keywords → yellow
/// 6. Fleet / ship keywords → light-blue
/// 7. Diplomacy / contact keywords → magenta
/// 8. Everything else → muted (dark-gray)
fn log_entry_style(entry: &str) -> Style {
    let lower = entry.to_ascii_lowercase();
    if lower.starts_with("error:") {
        Theme::error_style()
    } else if lower.starts_with("turn ")
        || lower.starts_with("game started")
        || lower.starts_with("game saved")
        || lower.starts_with("game loaded")
    {
        Theme::accent_style()
    } else if lower.contains("research") || lower.contains("technology") || lower.contains("tech ")
    {
        Theme::success_style()
    } else if lower.contains("survey") {
        Theme::accent_style()
    } else if lower.contains("colon") {
        // "colon" is a deliberate substring that matches colony/colonize/colonization
        Theme::accent_style()
    } else if lower.contains("scout") || lower.contains("explored") {
        Style::default().fg(ratatui::style::Color::Yellow)
    } else if lower.contains("fleet")
        || lower.contains("ship")
        || lower.contains("depart")
        || lower.contains("arriv")
    {
        Style::default().fg(ratatui::style::Color::LightBlue)
    } else if lower.contains("contact") || lower.contains("diplomacy") || lower.contains("empire") {
        Style::default().fg(ratatui::style::Color::LightMagenta)
    } else {
        Theme::muted_style()
    }
}

fn is_low_signal_entry(entry: &str) -> bool {
    let lower = entry.to_ascii_lowercase();
    (lower.starts_with("colony ") && lower.contains(" produced "))
        || (lower.starts_with("empire ") && lower.contains(" generated "))
        || lower.starts_with("ai empire ")
}

fn format_log_entry(entry: &str) -> Option<String> {
    if is_low_signal_entry(entry) {
        return None;
    }
    let lower = entry.to_ascii_lowercase();
    let prefix = if lower.starts_with("error:") {
        "✖ "
    } else if lower.starts_with("warning:") || lower.contains(" shortage") || lower.contains(" deficit")
    {
        "⚠ "
    } else if lower.starts_with("turn ") && lower.contains(" report:") {
        "📊 "
    } else if lower.starts_with("turn ") {
        "⏵ "
    } else if lower.contains("research complete") {
        "✓ "
    } else if lower.contains("colon") {
        "◎ "
    } else if lower.contains("survey") {
        "◌ "
    } else if lower.contains("scout") || lower.contains("fleet") {
        "➤ "
    } else if lower.contains("save:") || lower.contains("load:") {
        "💾 "
    } else {
        "• "
    };
    Some(format!("{}{}", prefix, entry))
}

/// Render the event log
pub fn render_log(frame: &mut Frame, area: Rect, log: &EventLog) {
    let visible_lines = (area.height.saturating_sub(2)) as usize;
    let formatted: Vec<(String, Style)> = log
        .last_n(log.len())
        .iter()
        .filter_map(|entry| {
            format_log_entry(entry).map(|rendered| (rendered, log_entry_style(entry)))
        })
        .collect();
    let start = formatted.len().saturating_sub(visible_lines);
    let entries = &formatted[start..];

    let lines: Vec<Line> = entries
        .iter()
        .map(|(entry, style)| Line::from(Span::styled(entry.clone(), *style)))
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

    // --- log_entry_style categorisation ---

    #[test]
    fn error_prefix_uses_error_style() {
        let style = log_entry_style("Error: bad thing happened");
        assert_eq!(style.fg, Theme::error_style().fg);
    }

    #[test]
    fn turn_prefix_uses_accent_style() {
        let style = log_entry_style("Turn 5 begins.");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn research_keyword_uses_success_style() {
        let style = log_entry_style("Research complete: Propulsion I");
        assert_eq!(style.fg, Theme::success_style().fg);
    }

    #[test]
    fn colony_keyword_uses_accent_style() {
        let style = log_entry_style("Colony founded on Kerenthis III");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn scout_keyword_uses_yellow() {
        let style = log_entry_style("Scout arrived at Velara");
        assert_eq!(style.fg, Some(ratatui::style::Color::Yellow));
    }

    #[test]
    fn survey_keyword_uses_accent_style() {
        let style = log_entry_style("Survey started for orbit 2");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn fleet_keyword_uses_light_blue() {
        let style = log_entry_style("Fleet 2 departed home system");
        assert_eq!(style.fg, Some(ratatui::style::Color::LightBlue));
    }

    #[test]
    fn unknown_entry_uses_muted_style() {
        let style = log_entry_style("Some unrecognised event happened");
        assert_eq!(style.fg, Theme::muted_style().fg);
    }

    #[test]
    fn game_saved_uses_accent_style() {
        let style = log_entry_style("Game saved.");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn render_log_all_categories_no_panic() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut log = EventLog::new();
        log.push("Error: something".to_string());
        log.push("Turn 3 begins".to_string());
        log.push("Research complete".to_string());
        log.push("Colony founded".to_string());
        log.push("Scout mission launched".to_string());
        log.push("Fleet 1 arrived".to_string());
        log.push("First contact with empire".to_string());
        log.push("Some other message".to_string());

        terminal
            .draw(|frame| {
                render_log(frame, frame.area(), &log);
            })
            .unwrap();
    }

    #[test]
    fn low_signal_entries_are_filtered() {
        assert!(is_low_signal_entry("Colony 1 produced 3 credits, 2 research, 1 food"));
        assert!(is_low_signal_entry("Empire 1 generated 6 science this turn"));
        assert!(is_low_signal_entry("AI Empire 2: queued Shipyard at colony 4"));
        assert!(!is_low_signal_entry("Turn 3 report: explored 1, surveyed 0"));
    }

    #[test]
    fn formatted_entries_get_visual_prefixes() {
        assert_eq!(
            format_log_entry("Turn 4 report: explored 1, surveyed 1").unwrap(),
            "📊 Turn 4 report: explored 1, surveyed 1"
        );
        assert_eq!(
            format_log_entry("Error: bad command").unwrap(),
            "✖ Error: bad command"
        );
        assert_eq!(
            format_log_entry("Research complete: tech 2").unwrap(),
            "✓ Research complete: tech 2"
        );
    }
}
