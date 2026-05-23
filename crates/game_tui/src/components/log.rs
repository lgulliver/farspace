//! Event log component

use crate::glyphs::glyphs_for_mode;
use crate::theme::Theme;
use crate::visual_mode::VisualMode;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Event log storage
#[derive(Debug, Clone)]
pub struct EventLog {
    entries: Vec<String>,
    metadata: Vec<LogEntryKind>,
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
            metadata: Vec::new(),
            max_entries: 50,
        }
    }

    /// Add an entry to the log
    pub fn push(&mut self, entry: String) {
        self.push_with_kind(LogEntryKind::from_message(&entry), entry);
    }

    /// Add an entry with explicit display metadata.
    pub fn push_with_kind(&mut self, kind: LogEntryKind, entry: String) {
        self.entries.push(entry);
        self.metadata.push(kind);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
            self.metadata.remove(0);
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
        self.metadata.clear();
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn last_n_with_kind(&self, n: usize) -> impl Iterator<Item = (&str, LogEntryKind)> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..]
            .iter()
            .zip(self.metadata[start..].iter().copied())
            .map(|(entry, kind)| (entry.as_str(), kind))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryKind {
    LowSignal,
    Error,
    Warning,
    TurnReport,
    TurnFlow,
    Research,
    Survey,
    Colony,
    Scout,
    Fleet,
    SaveLoad,
    Diplomacy,
    Other,
}

impl LogEntryKind {
    pub fn from_message(entry: &str) -> Self {
        classify_log_entry(&entry.to_ascii_lowercase())
    }

    pub fn from_core_event(event: &game_core::Event) -> Self {
        Self::from_message(&event.to_log_message())
    }
}

fn classify_log_entry(lower: &str) -> LogEntryKind {
    if (lower.starts_with("colony ") && lower.contains(" produced "))
        || (lower.starts_with("empire ") && lower.contains(" generated "))
        || lower.starts_with("ai empire ")
    {
        LogEntryKind::LowSignal
    } else if lower.starts_with("error:") {
        LogEntryKind::Error
    } else if lower.starts_with("warning:")
        || lower.contains(" shortage")
        || lower.contains(" deficit")
    {
        LogEntryKind::Warning
    } else if lower.starts_with("turn ") && lower.contains(" report:") {
        LogEntryKind::TurnReport
    } else if lower.starts_with("turn ")
        || lower.starts_with("game started")
        || lower.starts_with("game saved")
        || lower.starts_with("game loaded")
    {
        LogEntryKind::TurnFlow
    } else if lower.contains("research") || lower.contains("technology") || lower.contains("tech ")
    {
        LogEntryKind::Research
    } else if lower.contains("survey") || lower.contains("anomaly") || lower.contains("discovery:")
    {
        LogEntryKind::Survey
    } else if lower.contains("colony") || lower.contains("coloniz") {
        LogEntryKind::Colony
    } else if lower.contains("scout") || lower.contains("explored") {
        LogEntryKind::Scout
    } else if lower.contains("fleet")
        || lower.contains("ship")
        || lower.contains("depart")
        || lower.contains("arriv")
    {
        LogEntryKind::Fleet
    } else if lower.contains("save:") || lower.contains("load:") {
        LogEntryKind::SaveLoad
    } else if lower.contains("contact") || lower.contains("diplomacy") || lower.contains("empire") {
        LogEntryKind::Diplomacy
    } else {
        LogEntryKind::Other
    }
}

fn style_for_class(class: LogEntryKind) -> Style {
    match class {
        LogEntryKind::LowSignal => Theme::muted_style(),
        LogEntryKind::Error => Theme::error_style(),
        LogEntryKind::Warning => Theme::warning_style(),
        LogEntryKind::TurnReport
        | LogEntryKind::TurnFlow
        | LogEntryKind::Survey
        | LogEntryKind::Colony => Theme::accent_style(),
        LogEntryKind::Research => Theme::success_style(),
        LogEntryKind::Scout => Style::default().fg(ratatui::style::Color::Yellow),
        LogEntryKind::Fleet => Style::default().fg(ratatui::style::Color::LightBlue),
        LogEntryKind::SaveLoad => Theme::accent_style(),
        LogEntryKind::Diplomacy => Style::default().fg(ratatui::style::Color::LightMagenta),
        LogEntryKind::Other => Theme::muted_style(),
    }
}

fn prefix_for_class(class: LogEntryKind, mode: VisualMode) -> Option<char> {
    let glyphs = glyphs_for_mode(mode);
    let icon = match class {
        LogEntryKind::LowSignal => return None,
        LogEntryKind::Error => glyphs.status_error,
        LogEntryKind::Warning => glyphs.warning,
        LogEntryKind::TurnReport => glyphs.resource,
        LogEntryKind::TurnFlow => glyphs.status_progress,
        LogEntryKind::Research => glyphs.status_done,
        LogEntryKind::Survey => glyphs.star_unexplored,
        LogEntryKind::Colony => glyphs.planet_colonized,
        LogEntryKind::Scout | LogEntryKind::Fleet => glyphs.transit,
        LogEntryKind::SaveLoad => glyphs.status_save,
        LogEntryKind::Diplomacy | LogEntryKind::Other => glyphs.bullet,
    };
    Some(icon)
}

/// Render the event log
pub fn render_log(frame: &mut Frame, area: Rect, log: &EventLog, mode: VisualMode) {
    let visible_lines = (area.height.saturating_sub(2)) as usize;
    let formatted: Vec<(String, Style)> = log
        .last_n_with_kind(log.len())
        .filter_map(|(entry, class)| {
            if class == LogEntryKind::LowSignal {
                None
            } else {
                let prefix = prefix_for_class(class, mode)
                    .map(|icon| format!("{icon} "))
                    .unwrap_or_default();
                Some((format!("{prefix}{entry}"), style_for_class(class)))
            }
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
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn test_style(entry: &str) -> Style {
        let lower = entry.to_ascii_lowercase();
        style_for_class(classify_log_entry(&lower))
    }

    fn test_is_low_signal(entry: &str) -> bool {
        let lower = entry.to_ascii_lowercase();
        classify_log_entry(&lower) == LogEntryKind::LowSignal
    }

    fn test_format_entry(entry: &str) -> Option<String> {
        let lower = entry.to_ascii_lowercase();
        let class = classify_log_entry(&lower);
        if class == LogEntryKind::LowSignal {
            None
        } else {
            let prefix = prefix_for_class(class, VisualMode::Unicode)
                .map(|icon| format!("{icon} "))
                .unwrap_or_default();
            Some(format!("{prefix}{entry}"))
        }
    }

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
            metadata: Vec::new(),
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
                render_log(frame, area, &log, VisualMode::Unicode);
            })
            .unwrap();
    }

    // --- log_entry_style categorisation ---

    #[test]
    fn error_prefix_uses_error_style() {
        let style = test_style("Error: bad thing happened");
        assert_eq!(style.fg, Theme::error_style().fg);
    }

    #[test]
    fn turn_prefix_uses_accent_style() {
        let style = test_style("Turn 5 begins.");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn research_keyword_uses_success_style() {
        let style = test_style("Research complete: Propulsion I");
        assert_eq!(style.fg, Theme::success_style().fg);
    }

    #[test]
    fn colony_keyword_uses_accent_style() {
        let style = test_style("Colony founded on Kerenthis III");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn scout_keyword_uses_yellow() {
        let style = test_style("Scout arrived at Velara");
        assert_eq!(style.fg, Some(ratatui::style::Color::Yellow));
    }

    #[test]
    fn survey_keyword_uses_accent_style() {
        let style = test_style("Survey started for orbit 2");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn fleet_keyword_uses_light_blue() {
        let style = test_style("Fleet 2 departed home system");
        assert_eq!(style.fg, Some(ratatui::style::Color::LightBlue));
    }

    #[test]
    fn unknown_entry_uses_muted_style() {
        let style = test_style("Some unrecognised event happened");
        assert_eq!(style.fg, Theme::muted_style().fg);
    }

    #[test]
    fn game_saved_uses_accent_style() {
        let style = test_style("Game saved.");
        assert_eq!(style.fg, Theme::accent_style().fg);
    }

    #[test]
    fn warning_entry_uses_warning_style() {
        let style = test_style("WARNING: Empire 1 food shortage — deficit 2");
        assert_eq!(style.fg, Theme::warning_style().fg);
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
                render_log(frame, frame.area(), &log, VisualMode::Unicode);
            })
            .unwrap();
    }

    #[test]
    fn low_signal_entries_are_filtered() {
        assert!(test_is_low_signal(
            "Colony 1 produced 3 credits, 2 research, 1 food"
        ));
        assert!(test_is_low_signal("Empire 1 generated 6 science this turn"));
        assert!(test_is_low_signal(
            "AI Empire 2: queued Shipyard at colony 4"
        ));
        assert!(!test_is_low_signal("Turn 3 report: explored 1, surveyed 0"));
    }

    #[test]
    fn formatted_entries_get_visual_prefixes() {
        assert_eq!(
            test_format_entry("Turn 4 report: explored 1, surveyed 1").unwrap(),
            "◆ Turn 4 report: explored 1, surveyed 1"
        );
        assert_eq!(
            test_format_entry("Error: bad command").unwrap(),
            "✖ Error: bad command"
        );
        assert_eq!(
            test_format_entry("Research complete: tech 2").unwrap(),
            "✓ Research complete: tech 2"
        );
    }

    #[test]
    fn render_log_filters_low_signal_entries() {
        let backend = TestBackend::new(100, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut log = EventLog::new();
        log.push("Colony 1 produced 3 credits, 2 research, 1 food".to_string());
        log.push("Empire 1 generated 6 science this turn".to_string());
        log.push("AI Empire 2: queued Shipyard at colony 4".to_string());
        log.push("Turn 3 report: explored 1, surveyed 0".to_string());

        terminal
            .draw(|frame| {
                render_log(frame, frame.area(), &log, VisualMode::Unicode);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..8u16)
            .flat_map(|y| {
                (0..100u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();

        assert!(!rendered.contains("produced"));
        assert!(!rendered.contains("generated"));
        assert!(!rendered.contains("AI Empire"));
        assert!(rendered.contains("Turn 3 report"));
    }

    #[test]
    fn render_log_wraps_long_lines() {
        let backend = TestBackend::new(30, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut log = EventLog::new();
        log.push(
            "Turn 1 report: this is a long event log line that should wrap to show tail"
                .to_string(),
        );

        terminal
            .draw(|frame| {
                render_log(frame, frame.area(), &log, VisualMode::Unicode);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..8u16)
            .flat_map(|y| {
                (0..30u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();

        assert!(rendered.contains("show tail"));
    }
}
