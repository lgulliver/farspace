//! Diplomacy screen — shows known empires and their relationship status

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::centered_rect;
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    empire_definition_by_id, DiplomaticCommunicationType, DiplomaticResponse, GameState,
    RelationshipStatus,
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
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

    render_empire_list(frame, main_area, app_state, game_state);
    render_communication_modal(frame, main_area, app_state, game_state);

    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Use this screen to monitor first contact, then return with Esc.");
    render_footer(frame, footer_area, &Screen::Diplomacy, Some(hint));
}

/// Render the list of known (contacted) empires and hidden placeholders for unknown ones.
fn render_empire_list(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
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

    let foreign_ids: Vec<_> = game_state
        .empires
        .keys()
        .copied()
        .filter(|empire_id| *empire_id != game_state.player_empire)
        .collect();
    let selected_idx = app_state
        .diplomacy
        .selected_empire_index
        .min(foreign_ids.len().saturating_sub(1));
    let selected_empire = foreign_ids.get(selected_idx).copied();

    // Iterate all empires except the player empire, in BTreeMap order (deterministic).
    for empire_id in foreign_ids {
        let Some(empire) = game_state.empires.get(&empire_id) else {
            continue;
        };

        let status = game_state
            .diplomacy
            .get(&empire_id)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown);
        let selected_marker = if Some(empire_id) == selected_empire {
            "▶ "
        } else {
            "  "
        };

        match status {
            RelationshipStatus::Contacted => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("● ", Theme::accent_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("Contacted", Theme::accent_style()),
                ]));
                push_identity_lines(&mut lines, empire);
                lines.push(Line::from(""));
            }
            RelationshipStatus::Neutral => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("◎ ", Theme::accent_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("Neutral", Theme::accent_style()),
                ]));
                push_identity_lines(&mut lines, empire);
                lines.push(Line::from(""));
            }
            RelationshipStatus::Cooperative => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("✶ ", Theme::success_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("Cooperative", Theme::success_style()),
                ]));
                push_identity_lines(&mut lines, empire);
                lines.push(Line::from(""));
            }
            RelationshipStatus::Tense => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("◈ ", Theme::warning_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("Tense", Theme::warning_style()),
                ]));
                push_identity_lines(&mut lines, empire);
                lines.push(Line::from(""));
            }
            RelationshipStatus::Hostile => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("⚠ ", Theme::error_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("Hostile", Theme::error_style()),
                ]));
                push_identity_lines(&mut lines, empire);
                lines.push(Line::from(""));
            }
            RelationshipStatus::War => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("⚔ ", Theme::error_style()),
                    Span::styled(empire.name.as_str(), Theme::title_style()),
                    Span::raw("  "),
                    Span::styled("At War", Theme::error_style()),
                ]));
                push_identity_lines(&mut lines, empire);
                lines.push(Line::from(""));
            }
            RelationshipStatus::Unknown => {
                lines.push(Line::from(vec![
                    Span::styled(selected_marker, Theme::accent_style()),
                    Span::styled("○ ", Theme::muted_style()),
                    Span::styled("[ Unknown Empire ]", Theme::muted_style()),
                    Span::raw("  "),
                    Span::styled("No contact", Theme::muted_style()),
                ]));
            }
        }

        if let Some(relationship) = game_state.diplomacy_relationships.get(&empire_id) {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "Relationship {} · Tension {} · Trust {}",
                        relationship.relationship_score,
                        relationship.tension_score,
                        relationship
                            .trust_score
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "-".to_string())
                    ),
                    Theme::muted_style(),
                ),
            ]));
            let active_treaties: Vec<_> = relationship
                .active_treaties
                .iter()
                .filter(|t| t.is_active(game_state.turn))
                .map(|t| format!("{} (until T{})", t.treaty_type.label(), t.expires_turn()))
                .collect();
            if !active_treaties.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("Treaties: {}", active_treaties.join(", ")),
                        Theme::success_style(),
                    ),
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
        "Tab/j/k select empire · w war · p peace · n NAP · x cancel NAP · c communications · Enter respond",
        Theme::muted_style(),
    )]));

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

fn render_communication_modal(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    if !app_state.diplomacy.show_communication_modal {
        return;
    }
    let player = game_state.player_empire;
    let mut messages: Vec<_> = game_state
        .diplomacy_pending_communications
        .iter()
        .filter(|msg| msg.receiving_empire == player)
        .collect();
    messages.sort_by_key(|msg| msg.communication_id);
    if messages.is_empty() {
        return;
    }
    let msg_idx = app_state
        .diplomacy
        .selected_communication_index
        .min(messages.len().saturating_sub(1));
    let msg = messages[msg_idx];
    let popup = centered_rect(72, 58, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Diplomatic Communication ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let from_name = game_state
        .empires
        .get(&msg.sending_empire)
        .map(|empire| empire.name.as_str())
        .unwrap_or("Unknown Empire");
    let comm_type = match msg.communication_type {
        DiplomaticCommunicationType::FirstContact => "First Contact",
        DiplomaticCommunicationType::TreatyProposal => "Treaty Proposal",
        DiplomaticCommunicationType::TreatyAccepted => "Treaty Accepted",
        DiplomaticCommunicationType::TreatyRejected => "Treaty Rejected",
        DiplomaticCommunicationType::Warning => "Warning",
        DiplomaticCommunicationType::TributeDemand => "Tribute Demand",
        DiplomaticCommunicationType::PeaceOffer => "Peace Offer",
        DiplomaticCommunicationType::WarDeclaration => "War Declaration",
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(from_name, Theme::title_style()),
            Span::raw("  "),
            Span::styled(comm_type, Theme::accent_style()),
            Span::raw("  "),
            Span::styled(format!("Tone: {}", msg.tone.label()), Theme::muted_style()),
        ]),
        Line::from(""),
        Line::from(Span::styled(msg.title.as_str(), Theme::title_style())),
        Line::from(""),
        Line::from(Span::raw(msg.body.as_str())),
        Line::from(""),
        Line::from(Span::styled("Responses", Theme::title_style())),
    ];

    for (idx, response) in msg.available_responses.iter().copied().enumerate() {
        let selected = idx == (app_state.diplomacy.selected_response_index % msg.available_responses.len());
        let marker = if selected { "▶ " } else { "  " };
        let label = match response {
            DiplomaticResponse::Acknowledge => "Acknowledge",
            DiplomaticResponse::Accept => "Accept",
            DiplomaticResponse::Reject => "Reject",
            DiplomaticResponse::Comply => "Comply",
            DiplomaticResponse::Refuse => "Refuse",
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Theme::accent_style()),
            Span::styled(
                label,
                if selected {
                    Theme::accent_style()
                } else {
                    Theme::default_style()
                },
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Enter: send response · Tab: next message · Esc: close",
        Theme::muted_style(),
    )));

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn push_identity_lines(lines: &mut Vec<Line>, empire: &game_core::Empire) {
    if let Some(def) = empire.empire_def.and_then(empire_definition_by_id) {
        let tag_labels: Vec<&str> = def.playstyle.iter().map(|t| t.label()).collect();
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} {}", def.symbol, def.short_description),
                Theme::muted_style(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(def.tone, Theme::success_style()),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(tag_labels.join(" · "), Theme::accent_style()),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(def.playstyle_summary, Theme::muted_style()),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Doctrine ", Theme::muted_style()),
            Span::styled(def.doctrine_short_summary(), Theme::accent_style()),
        ]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use game_core::{EmpireDefinitionId, Engine};
    use ratatui::{backend::TestBackend, Terminal};

    fn render_to_string(engine: &Engine) -> String {
        let app_state = AppState::default();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_diplomacy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut rendered = String::new();
        for y in 0..40u16 {
            for x in 0..120u16 {
                let ch = buffer
                    .cell((x, y))
                    .and_then(|cell| cell.symbol().chars().next())
                    .unwrap_or(' ');
                rendered.push(ch);
            }
        }
        rendered
    }

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
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        let doctrine = engine
            .state
            .empires
            .get(&ai_id)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
            .map(|def| def.doctrine_short_summary())
            .expect("AI doctrine should exist");
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
        let rendered = render_to_string(&engine);
        assert!(rendered.contains("[ Unknown Empire ]"));
        assert!(!rendered.contains(&doctrine));
    }

    #[test]
    fn diplomacy_screen_shows_faction_identity_and_tone() {
        use game_core::RelationshipStatus;

        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        let ai_empire = engine.state.empires.get_mut(&ai_id).unwrap();
        ai_empire.empire_def = Some(EmpireDefinitionId(6));
        ai_empire.name = "Terran Concord".to_string();
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Neutral);

        let doctrine = engine
            .state
            .empires
            .get(&ai_id)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
            .map(|def| def.doctrine_short_summary())
            .expect("AI doctrine should exist");
        let rendered = render_to_string(&engine);
        assert!(rendered.contains("Terran Concord"));
        assert!(rendered.contains("science-forward federation"));
        assert!(rendered.contains(&doctrine));
    }
}
