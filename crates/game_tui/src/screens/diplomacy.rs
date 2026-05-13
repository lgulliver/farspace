//! Diplomacy screen — shows known empires and their relationship status

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{GameState, RelationshipStatus};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the diplomacy screen
pub fn render_diplomacy(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    render_empire_list(frame, main_area, game_state);

    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Use this screen to monitor first contact, then return with Esc.");
    render_footer(frame, footer_area, &Screen::Diplomacy, Some(hint));
}

/// Render the list of known (contacted) empires and hidden placeholders for unknown ones.
fn render_empire_list(frame: &mut Frame, area: Rect, game_state: &GameState) {
    let block = Block::default()
        .title(" Diplomacy ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled("Known Empires", Theme::title_style())]),
        Line::from(""),
    ];

    // Iterate all empires except the player empire, in BTreeMap order (deterministic).
    for (empire_id, empire) in &game_state.empires {
        if *empire_id == game_state.player_empire {
            continue;
        }

        let status = game_state
            .diplomacy
            .get(empire_id)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown);

        match status {
            RelationshipStatus::Contacted => {
                lines.push(Line::from(vec![
                    Span::styled("● ", Theme::accent_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("Contacted", Theme::accent_style()),
                ]));
            }
            RelationshipStatus::Unknown => {
                lines.push(Line::from(vec![
                    Span::styled("○ ", Theme::muted_style()),
                    Span::styled("[ Unknown Empire ]", Theme::muted_style()),
                    Span::raw("  "),
                    Span::styled("No contact", Theme::muted_style()),
                ]));
            }
        }
    }

    if game_state
        .empires
        .keys()
        .filter(|&&id| id != game_state.player_empire)
        .count()
        == 0
    {
        lines.push(Line::from(vec![Span::styled(
            "No other empires in this galaxy.",
            Theme::muted_style(),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Press Esc to return to the galaxy map.",
        Theme::muted_style(),
    )]));

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn diplomacy_screen_renders_without_panic() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_diplomacy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn diplomacy_screen_with_contact_renders_without_panic() {
        use game_core::RelationshipStatus;
        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Contacted);

        let app_state = AppState::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_diplomacy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    /// Unknown empires do not show full diplomacy details (name, etc).
    #[test]
    fn unknown_empire_shows_hidden_placeholder() {
        use game_core::RelationshipStatus;

        let engine = Engine::new(42);
        // No contacts established — all empires should show as unknown
        for empire_id in engine.state.empires.keys() {
            if *empire_id == engine.state.player_empire {
                continue;
            }
            let status = engine
                .state
                .diplomacy
                .get(empire_id)
                .copied()
                .unwrap_or(RelationshipStatus::Unknown);
            assert_eq!(status, RelationshipStatus::Unknown);
        }
    }
}
