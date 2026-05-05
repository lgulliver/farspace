//! Main menu screen

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the main menu
pub fn render_menu(frame: &mut Frame, area: Rect) {
    // Center the menu vertically
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(12),
            Constraint::Percentage(30),
        ])
        .split(area);

    let center = chunks[1];

    // Center horizontally
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(center);

    let menu_area = h_chunks[1];

    let title = r#"
  ███████╗ █████╗ ██████╗ ███████╗██████╗  █████╗  ██████╗███████╗
  ██╔════╝██╔══██╗██╔══██╗██╔════╝██╔══██╗██╔══██╗██╔════╝██╔════╝
  █████╗  ███████║██████╔╝███████╗██████╔╝███████║██║     █████╗  
  ██╔══╝  ██╔══██║██╔══██╗╚════██║██╔═══╝ ██╔══██║██║     ██╔══╝  
  ██║     ██║  ██║██║  ██║███████║██║     ██║  ██║╚██████╗███████╗
  ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚══════╝
"#;

    let menu_items = vec![
        Line::from(""),
        Line::from(Span::styled(title, Theme::title_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled("[N]", Theme::title_style()),
            Span::raw(" New Game"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[L]", Theme::title_style()),
            Span::raw(" Load Game"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Q]", Theme::title_style()),
            Span::raw(" Quit"),
        ]),
    ];

    let paragraph = Paragraph::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Theme::default_style()),
        )
        .alignment(Alignment::Center)
        .style(Theme::default_style());

    frame.render_widget(paragraph, menu_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn menu_screen_renders_without_panic() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_menu(frame, area);
            })
            .unwrap();
    }
}
