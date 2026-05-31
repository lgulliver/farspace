//! Advisor guidance rendering: turn brief, contextual strip, alert severity.
//!
//! This is the TUI presentation layer for the deterministic advisor facts
//! produced by `game_core::advisor`. The core decides *what* to say; this module
//! decides *how* it looks. No styling lives in `game_core`.

use crate::components::chrome::quiet_panel_block;
use crate::theme::Theme;
use crate::visual_mode::VisualMode;
use game_core::advisor::{AdvisorAction, AdvisorOutput, AdvisorSeverity, ScreenRef};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// UI-facing alert severity vocabulary. Drives glyph + style so important
/// states are never communicated by colour alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Success,
    Warning,
    Critical,
}

impl AlertSeverity {
    /// Map a core advisor severity onto the UI vocabulary. The core has no
    /// `Success` (that is reserved for positive outcome feedback), so
    /// suggestions read as informational guidance.
    pub fn from_advisor(severity: AdvisorSeverity) -> Self {
        match severity {
            AdvisorSeverity::Info | AdvisorSeverity::Suggestion => AlertSeverity::Info,
            AdvisorSeverity::Warning => AlertSeverity::Warning,
            AdvisorSeverity::Critical => AlertSeverity::Critical,
        }
    }

    /// Terminal-safe leading glyph. ASCII-safe in every visual mode except the
    /// success tick, which degrades to `+` outside Unicode.
    pub fn glyph(self, mode: VisualMode) -> &'static str {
        match self {
            AlertSeverity::Info => "i",
            AlertSeverity::Success => match mode {
                VisualMode::Ascii => "+",
                _ => "✓",
            },
            AlertSeverity::Warning => "!",
            AlertSeverity::Critical => "!!",
        }
    }

    /// Semantic style for this severity. Always non-empty (carries a colour).
    pub fn style(self) -> Style {
        match self {
            AlertSeverity::Info => Style::default().fg(Theme::info()),
            AlertSeverity::Success => Theme::success_style(),
            AlertSeverity::Warning => Theme::warning_style(),
            AlertSeverity::Critical => Theme::error_style(),
        }
    }
}

/// Short, screen-agnostic hint describing the suggested action for a message.
fn action_hint(action: &AdvisorAction) -> &'static str {
    match action {
        AdvisorAction::OpenScreen(ScreenRef::Colony) => "open Colony (C)",
        AdvisorAction::OpenScreen(ScreenRef::Research) => "open Research (R)",
        AdvisorAction::OpenScreen(ScreenRef::Fleets) => "open Sector Map (Enter)",
        AdvisorAction::OpenScreen(ScreenRef::Diplomacy) => "open Diplomacy (D)",
        AdvisorAction::OpenScreen(ScreenRef::Galaxy) => "Galaxy Overview (Esc)",
        AdvisorAction::OpenScreen(ScreenRef::AdvisorHistory) => "Advisor history",
        AdvisorAction::OpenResearch => "open Research (R)",
        AdvisorAction::OpenShipyard(_) | AdvisorAction::FocusColony(_) => "open Colony (C)",
        AdvisorAction::FocusFleet(_) | AdvisorAction::FocusSystem(_) => "open Sector Map (Enter)",
        AdvisorAction::OpenCommandPalette(_) => "command palette (:)",
    }
}

/// One-line contextual advisor sentence for the highest-priority message, e.g.
/// `Advisor: Cygnus Prime has no active build queue — open Colony (C)`.
/// Returns `None` when there is no active guidance.
pub fn advisor_strip_text(output: &AdvisorOutput) -> Option<String> {
    let msg = output.active.first()?;
    let mut text = format!("Advisor: {}", msg.body.trim());
    if let Some(action) = msg.actions.first() {
        text.push_str(" — ");
        text.push_str(action_hint(action));
    }
    Some(text)
}

/// Render the contextual advisor strip on a single muted-but-visible line.
/// Renders nothing when there is no active guidance.
pub fn render_advisor_strip(
    frame: &mut Frame,
    area: Rect,
    output: &AdvisorOutput,
    mode: VisualMode,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let Some(msg) = output.active.first() else {
        return;
    };
    let severity = AlertSeverity::from_advisor(msg.severity);
    let mut spans = vec![
        Span::styled(severity.glyph(mode), severity.style()),
        Span::raw(" "),
        Span::styled("Advisor: ", Theme::muted_style()),
        Span::styled(msg.body.trim().to_string(), Theme::text_secondary_style()),
    ];
    if let Some(action) = msg.actions.first() {
        spans.push(Span::raw(" — "));
        spans.push(Span::styled(action_hint(action), Theme::accent_style()));
    }
    let paragraph = Paragraph::new(Line::from(spans)).style(Theme::default_style());
    frame.render_widget(paragraph, area);
}

/// Render the compact turn-start briefing panel. Lists the highest-signal
/// advisor items, one per line, glyph-tagged by severity. Renders a calm
/// "all nominal" line when there is nothing to flag.
pub fn render_turn_brief(frame: &mut Frame, area: Rect, output: &AdvisorOutput, mode: VisualMode) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = quiet_panel_block("Turn Brief");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let capacity = inner.height as usize;
    let lines: Vec<Line> = if output.active.is_empty() {
        vec![Line::from(Span::styled(
            "All systems nominal — no priorities flagged.",
            Theme::muted_style(),
        ))]
    } else {
        output
            .active
            .iter()
            .take(capacity)
            .map(|msg| {
                let severity = AlertSeverity::from_advisor(msg.severity);
                Line::from(vec![
                    Span::styled(severity.glyph(mode), severity.style()),
                    Span::raw(" "),
                    Span::styled(msg.title.trim().to_string(), Theme::text_primary_style()),
                ])
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::advisor::{
        AdvisorCategory, AdvisorMessage, AdvisorMessageId, AdvisorMessageKey, AdvisorPersona,
        AdvisorRuleId, AdvisorTarget,
    };
    use ratatui::{backend::TestBackend, Terminal};

    fn message(severity: AdvisorSeverity, title: &str, body: &str) -> AdvisorMessage {
        AdvisorMessage {
            id: AdvisorMessageId(title.to_string()),
            key: AdvisorMessageKey {
                rule_id: AdvisorRuleId("test"),
                target: Some(AdvisorTarget::Empire),
            },
            category: AdvisorCategory::Colony,
            persona: AdvisorPersona::Guide,
            severity,
            title: title.to_string(),
            body: body.to_string(),
            turn_created: 1,
            expires_on_turn: None,
            actions: vec![AdvisorAction::OpenScreen(ScreenRef::Colony)],
            dismissible: true,
            tutorial_id: None,
            target: Some(AdvisorTarget::Empire),
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn every_severity_maps_to_non_empty_style_and_glyph() {
        for severity in [
            AlertSeverity::Info,
            AlertSeverity::Success,
            AlertSeverity::Warning,
            AlertSeverity::Critical,
        ] {
            assert!(severity.style().fg.is_some(), "{severity:?} needs a colour");
            assert!(
                !severity.glyph(VisualMode::Unicode).is_empty(),
                "{severity:?} needs a glyph"
            );
            assert!(!severity.glyph(VisualMode::Ascii).is_empty());
        }
    }

    #[test]
    fn advisor_severity_maps_onto_ui_vocabulary() {
        assert_eq!(
            AlertSeverity::from_advisor(AdvisorSeverity::Info),
            AlertSeverity::Info
        );
        assert_eq!(
            AlertSeverity::from_advisor(AdvisorSeverity::Suggestion),
            AlertSeverity::Info
        );
        assert_eq!(
            AlertSeverity::from_advisor(AdvisorSeverity::Warning),
            AlertSeverity::Warning
        );
        assert_eq!(
            AlertSeverity::from_advisor(AdvisorSeverity::Critical),
            AlertSeverity::Critical
        );
    }

    #[test]
    fn success_glyph_degrades_outside_unicode() {
        assert_eq!(AlertSeverity::Success.glyph(VisualMode::Ascii), "+");
        assert_eq!(AlertSeverity::Success.glyph(VisualMode::Unicode), "✓");
    }

    #[test]
    fn strip_text_is_none_without_messages() {
        assert!(advisor_strip_text(&AdvisorOutput::default()).is_none());
    }

    #[test]
    fn strip_text_includes_body_and_action_hint() {
        let output = AdvisorOutput {
            active: vec![message(
                AdvisorSeverity::Warning,
                "Idle colony",
                "Cygnus Prime has no active build queue",
            )],
        };
        let text = advisor_strip_text(&output).unwrap();
        assert!(text.contains("Cygnus Prime"));
        assert!(text.contains("open Colony (C)"));
    }

    #[test]
    fn advisor_strip_renders_at_80_columns() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let output = AdvisorOutput {
            active: vec![message(
                AdvisorSeverity::Warning,
                "Idle colony",
                "Cygnus Prime has no active build queue",
            )],
        };
        terminal
            .draw(|frame| {
                render_advisor_strip(frame, frame.area(), &output, VisualMode::Unicode);
            })
            .unwrap();
        assert!(buffer_text(&terminal).contains("Advisor"));
    }

    #[test]
    fn turn_brief_renders_with_zero_items() {
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_turn_brief(
                    frame,
                    frame.area(),
                    &AdvisorOutput::default(),
                    VisualMode::Unicode,
                );
            })
            .unwrap();
        assert!(buffer_text(&terminal).contains("nominal"));
    }

    #[test]
    fn turn_brief_renders_several_items() {
        let backend = TestBackend::new(60, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let output = AdvisorOutput {
            active: vec![
                message(
                    AdvisorSeverity::Critical,
                    "Treasury low",
                    "Credits depleting",
                ),
                message(AdvisorSeverity::Warning, "Idle colony", "Cygnus Prime idle"),
                message(
                    AdvisorSeverity::Info,
                    "Research done",
                    "Fusion Lattice ready",
                ),
            ],
        };
        terminal
            .draw(|frame| {
                render_turn_brief(frame, frame.area(), &output, VisualMode::Unicode);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Treasury low"));
        assert!(text.contains("Idle colony"));
    }

    #[test]
    fn render_helpers_do_not_panic_at_zero_size() {
        let backend = TestBackend::new(1, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let zero = Rect::new(0, 0, 0, 0);
                render_advisor_strip(frame, zero, &AdvisorOutput::default(), VisualMode::Unicode);
                render_turn_brief(frame, zero, &AdvisorOutput::default(), VisualMode::Unicode);
            })
            .unwrap();
    }
}
