//! Footer component

use crate::screens::Screen;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the footer with contextual hints
pub fn render_footer(frame: &mut Frame, area: Rect, screen: &Screen) {
    let hints = match screen {
        Screen::Menu => vec![("[N]", "New Game"), ("[L]", "Load"), ("[Q]", "Quit")],
        Screen::Galaxy => vec![
            ("[hjkl/←↓↑→]", "Move"),
            ("[c]", "Colony"),
            ("[r]", "Research"),
            ("[E/T/Enter]", "End Turn"),
            ("[?]", "Help"),
            ("[:]", "Command"),
            ("[Q]", "Quit"),
        ],
        Screen::Colony => vec![
            ("[j/k]", "Select"),
            ("[Enter]", "Queue"),
            ("[e/t]", "End Turn"),
            ("[?]", "Help"),
            ("[Esc]", "Back"),
        ],
        Screen::Research => vec![
            ("[j/k]", "Navigate"),
            ("[Enter]", "Select Tech"),
            ("[e/t]", "End Turn"),
            ("[?]", "Help"),
            ("[Esc]", "Back"),
        ],
        Screen::Diplomacy => vec![("[e/t]", "End Turn"), ("[?]", "Help"), ("[Esc]", "Back")],
    };

    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut v = vec![
                Span::styled(*key, Theme::title_style()),
                Span::raw(" "),
                Span::styled(*desc, Theme::muted_style()),
            ];
            if i < hints.len() - 1 {
                v.push(Span::raw("  "));
            }
            v
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::TOP))
        .style(Theme::default_style());

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_footer_menu() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Menu);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_galaxy() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Galaxy);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_colony() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Colony);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_research() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Research);
            })
            .unwrap();
    }

    #[test]
    fn render_footer_diplomacy() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(frame, area, &Screen::Diplomacy);
            })
            .unwrap();
    }
}
