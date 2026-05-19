//! Command palette component

use crate::layout::centered_fixed;
use crate::theme::Theme;
use crate::{glyphs::glyphs_for_mode, visual_mode::VisualMode};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Commands accepted by the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    Save,
    Load,
    ClearRally,
    VisualMode,
    /// Show the Galactic Dispatch modal
    Dispatch,
    /// Show the Galactic Dispatch modal (alias)
    News,
}

impl PaletteCommand {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Save,
            Self::Load,
            Self::ClearRally,
            Self::VisualMode,
            Self::Dispatch,
            Self::News,
        ]
    }

    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Load => "load",
            Self::ClearRally => "clear-rally",
            Self::VisualMode => "visual-mode",
            Self::Dispatch => "dispatch",
            Self::News => "news",
        }
    }

    /// Parse a palette input string. Empty input and a bare ':' are no-ops.
    pub fn parse(input: &str) -> Result<Option<Self>, PaletteCommandParseError> {
        let normalized = input.trim_start_matches(':').trim();
        if normalized.is_empty() {
            return Ok(None);
        }

        let parsed = match normalized {
            "mode" | "visual" => Some(Self::VisualMode),
            _ => Self::all()
                .iter()
                .copied()
                .find(|command| command.keyword() == normalized),
        };
        parsed.map(Some).ok_or_else(|| PaletteCommandParseError {
            command: normalized.to_string(),
        })
    }
}

/// Error returned when palette input does not match a known command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteCommandParseError {
    command: String,
}

impl PaletteCommandParseError {
    pub fn command(&self) -> &str {
        &self.command
    }
}

/// Render the command palette
pub fn render_palette(frame: &mut Frame, area: Rect, input: &str, mode: VisualMode) {
    // Height 7 = border top + hint line + blank + input line + blank + help line + border bottom
    let popup_area = centered_fixed(70, 7, area);
    let glyphs = glyphs_for_mode(mode);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let command_spans = PaletteCommand::all()
        .iter()
        .enumerate()
        .flat_map(|(index, command)| {
            let mut spans = Vec::new();
            if index == 0 {
                spans.push(Span::styled("Commands: ", Theme::muted_style()));
            } else {
                spans.push(Span::styled(
                    format!("  {}  ", glyphs.separator_dot),
                    Theme::dim_border_style(),
                ));
            }
            spans.push(Span::styled(command.keyword(), Theme::title_style()));
            spans
        })
        .collect::<Vec<_>>();

    let lines = vec![
        Line::from(command_spans),
        Line::from(""),
        Line::from(vec![
            Span::styled(": ", Theme::accent_style()),
            Span::raw(input),
            Span::styled(glyphs.palette_cursor.to_string(), Theme::accent_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Theme::title_style()),
            Span::styled(" to execute  ", Theme::muted_style()),
            Span::styled("Esc", Theme::title_style()),
            Span::styled(" to close", Theme::muted_style()),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Command Palette ")
                .borders(Borders::ALL)
                .border_style(Theme::focused_border_style())
                .style(Theme::default_style()),
        )
        .style(Theme::default_style());

    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_palette_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "test input", VisualMode::Unicode);
            })
            .unwrap();
    }

    #[test]
    fn render_palette_empty_input() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "", VisualMode::Ascii);
            })
            .unwrap();
    }

    #[test]
    fn render_palette_long_input_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(
                    frame,
                    area,
                    "save_a_very_long_filename_that_overflows",
                    VisualMode::NerdFont,
                );
            })
            .unwrap();
    }

    #[test]
    fn render_palette_tiny_terminal_no_panic() {
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "save", VisualMode::Unicode);
            })
            .unwrap();
    }

    #[test]
    fn palette_command_parse_accepts_known_commands() {
        assert_eq!(
            PaletteCommand::parse(":save").unwrap(),
            Some(PaletteCommand::Save)
        );
        assert_eq!(
            PaletteCommand::parse(" load ").unwrap(),
            Some(PaletteCommand::Load)
        );
        assert_eq!(
            PaletteCommand::parse("clear-rally").unwrap(),
            Some(PaletteCommand::ClearRally)
        );
        assert_eq!(
            PaletteCommand::parse("visual-mode").unwrap(),
            Some(PaletteCommand::VisualMode)
        );
        assert_eq!(
            PaletteCommand::parse("mode").unwrap(),
            Some(PaletteCommand::VisualMode)
        );
        assert_eq!(
            PaletteCommand::parse("visual").unwrap(),
            Some(PaletteCommand::VisualMode)
        );
        assert_eq!(
            PaletteCommand::parse("dispatch").unwrap(),
            Some(PaletteCommand::Dispatch)
        );
        assert_eq!(
            PaletteCommand::parse("news").unwrap(),
            Some(PaletteCommand::News)
        );
    }

    #[test]
    fn palette_command_parse_treats_empty_input_as_noop() {
        assert_eq!(PaletteCommand::parse("").unwrap(), None);
        assert_eq!(PaletteCommand::parse(":").unwrap(), None);
        assert_eq!(PaletteCommand::parse(":  ").unwrap(), None);
    }

    #[test]
    fn palette_command_parse_reports_unknown_command() {
        let err = PaletteCommand::parse(":bogus").unwrap_err();
        assert_eq!(err.command(), "bogus");
    }
}
