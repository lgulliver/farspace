//! Command palette component

use crate::layout::centered_fixed;
use crate::theme::Theme;
use crate::{glyphs::glyphs_for_mode, visual_mode::VisualMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
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
    // Height 8 leaves room for the command list to wrap one extra row on narrow
    // terminals without pushing the input or help lines out of view.
    let popup_area = centered_fixed(70, 8, area);
    let glyphs = glyphs_for_mode(mode);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Reserve fixed rows for the input and help lines so they stay visible even
    // when the command list wraps. The command region absorbs any extra height.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // commands (wraps)
            Constraint::Length(1), // input
            Constraint::Length(1), // help
        ])
        .split(inner);

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

    let commands = Paragraph::new(Line::from(command_spans))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .style(Theme::default_style());
    frame.render_widget(commands, chunks[0]);

    let input_line = Paragraph::new(Line::from(vec![
        Span::styled(": ", Theme::accent_style()),
        Span::raw(input),
        Span::styled(glyphs.palette_cursor.to_string(), Theme::accent_style()),
    ]))
    .style(Theme::default_style());
    frame.render_widget(input_line, chunks[1]);

    let help_line = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Theme::title_style()),
        Span::styled(" to execute  ", Theme::muted_style()),
        Span::styled("Esc", Theme::title_style()),
        Span::styled(" to close", Theme::muted_style()),
    ]))
    .style(Theme::default_style());
    frame.render_widget(help_line, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

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
    fn render_palette_keeps_input_and_help_visible_when_commands_wrap() {
        // Regression: on a narrow terminal the wrapped "Commands: …" line could
        // consume the whole popup and clip the input/cursor and help lines,
        // making the palette unusable. Those rows are now reserved, so they must
        // remain visible regardless of how far the command list wraps.
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_palette(frame, area, "save", VisualMode::Ascii);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let rendered: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            rendered.contains("save"),
            "input text should stay visible, got:\n{rendered}"
        );
        assert!(
            rendered.contains("Enter") && rendered.contains("Esc"),
            "help line should stay visible, got:\n{rendered}"
        );
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
