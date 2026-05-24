//! Diplomacy screen — shows known empires and their relationship status

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::centered_rect;
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    empire_definition_by_id, DiplomaticCommunicationType, DiplomaticResponse, GameState,
    IntelLevel, RelationshipStatus,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
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
    let foreign_ids: Vec<_> = game_state
        .empires
        .keys()
        .copied()
        .filter(|empire_id| *empire_id != game_state.player_empire)
        .collect();

    let selected_empire = selected_foreign_empire(app_state, &foreign_ids);
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);
    let list_block = Block::default()
        .title(" Diplomacy ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let list_inner = list_block.inner(panels[0]);
    frame.render_widget(list_block, panels[0]);

    let detail_block = Block::default()
        .title(" Empire Detail ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let detail_inner = detail_block.inner(panels[1]);
    frame.render_widget(detail_block, panels[1]);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled("Known Empires", Theme::title_style())]),
        Line::from(""),
    ];

    for empire_id in &foreign_ids {
        let Some(empire) = game_state.empires.get(empire_id) else {
            continue;
        };

        let status = game_state
            .diplomacy
            .get(empire_id)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown);
        let intel = game_state.intel_level_for_empire(*empire_id);
        let selected_marker = if Some(*empire_id) == selected_empire {
            "▶ "
        } else {
            "  "
        };
        let (icon, icon_style, name, status_label, status_style) = match status {
            RelationshipStatus::Contacted => (
                "● ",
                Theme::accent_style(),
                empire.name.as_str(),
                "Contacted",
                Theme::accent_style(),
            ),
            RelationshipStatus::Neutral => (
                "◎ ",
                Theme::accent_style(),
                empire.name.as_str(),
                "Neutral",
                Theme::accent_style(),
            ),
            RelationshipStatus::Cooperative => (
                "✶ ",
                Theme::success_style(),
                empire.name.as_str(),
                "Cooperative",
                Theme::success_style(),
            ),
            RelationshipStatus::Tense => (
                "◈ ",
                Theme::warning_style(),
                empire.name.as_str(),
                "Tense",
                Theme::warning_style(),
            ),
            RelationshipStatus::Hostile => (
                "⚠ ",
                Theme::error_style(),
                empire.name.as_str(),
                "Hostile",
                Theme::error_style(),
            ),
            RelationshipStatus::War => (
                "⚔ ",
                Theme::error_style(),
                empire.name.as_str(),
                "At War",
                Theme::error_style(),
            ),
            RelationshipStatus::Unknown => (
                "○ ",
                Theme::muted_style(),
                "[ Unknown Empire ]",
                "No contact",
                Theme::muted_style(),
            ),
        };

        lines.push(Line::from(vec![
            Span::styled(selected_marker, Theme::accent_style()),
            Span::styled(icon, icon_style),
            Span::styled(
                name,
                if intel == IntelLevel::Unknown {
                    Theme::muted_style()
                } else {
                    Theme::title_style()
                },
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(status_label, status_style),
            Span::raw("  "),
            Span::styled(format!("Intel {}", intel.label()), Theme::muted_style()),
        ]));
        lines.push(Line::from(""));
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
    let actions_hint = if app_state.diplomacy.show_communication_modal {
        "Tab/j/k select empire · w war · p peace · n NAP · i gather intel · Enter respond (modal)"
    } else {
        "Tab/j/k select empire · w war · p peace · n NAP · i gather intel · c communications · Enter/e/t end turn"
    };
    lines.push(Line::from(vec![Span::styled(
        actions_hint,
        Theme::muted_style(),
    )]));

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, list_inner);
    render_empire_detail(frame, detail_inner, selected_empire, game_state);
}

fn selected_foreign_empire(
    app_state: &AppState,
    foreign_ids: &[game_core::EmpireId],
) -> Option<game_core::EmpireId> {
    if foreign_ids.is_empty() {
        None
    } else {
        let selected_idx = app_state.diplomacy.selected_empire_index % foreign_ids.len();
        Some(foreign_ids[selected_idx])
    }
}

fn render_empire_detail(
    frame: &mut Frame,
    area: Rect,
    selected_empire: Option<game_core::EmpireId>,
    game_state: &GameState,
) {
    let mut lines = Vec::new();
    let Some(empire_id) = selected_empire else {
        lines.push(Line::from(Span::styled(
            "No foreign empires present.",
            Theme::muted_style(),
        )));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    };

    let intel = game_state.intel_level_for_empire(empire_id);
    let status = game_state
        .diplomacy
        .get(&empire_id)
        .copied()
        .unwrap_or(RelationshipStatus::Unknown);
    let detail_name = if intel == IntelLevel::Unknown {
        "[ Unknown Empire ]"
    } else {
        game_state
            .empires
            .get(&empire_id)
            .map(|empire| empire.name.as_str())
            .unwrap_or("[ Unknown Empire ]")
    };

    lines.push(Line::from(vec![
        Span::styled(
            detail_name,
            if intel == IntelLevel::Unknown {
                Theme::muted_style()
            } else {
                Theme::title_style()
            },
        ),
        Span::raw("  "),
        Span::styled(format!("Intel {}", intel.label()), Theme::accent_style()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Diplomatic stance ", Theme::muted_style()),
        Span::styled(
            if intel.reveals_diplomatic_stance() {
                status.label()
            } else {
                "Hidden"
            },
            if intel.reveals_diplomatic_stance() {
                Theme::default_style()
            } else {
                Theme::muted_style()
            },
        ),
    ]));
    lines.push(Line::from(""));

    if intel == IntelLevel::Unknown {
        lines.push(Line::from(Span::styled(
            "No verified contact. Empire details remain concealed.",
            Theme::muted_style(),
        )));
    } else {
        let Some(empire) = game_state.empires.get(&empire_id) else {
            frame.render_widget(Paragraph::new(lines), area);
            return;
        };
        push_identity_lines(&mut lines, empire);
        lines.push(Line::from(""));

        if let Some(relationship) = game_state.diplomacy_relationships.get(&empire_id) {
            let active_treaties: Vec<_> = relationship
                .active_treaties
                .iter()
                .filter(|t| t.is_active(game_state.turn))
                .map(|t| format!("{} (until T{})", t.treaty_type.label(), t.expires_turn()))
                .collect();
            lines.push(Line::from(vec![
                Span::styled("Treaties ", Theme::muted_style()),
                Span::styled(
                    if active_treaties.is_empty() {
                        "None".to_string()
                    } else {
                        active_treaties.join(", ")
                    },
                    if active_treaties.is_empty() {
                        Theme::muted_style()
                    } else {
                        Theme::success_style()
                    },
                ),
            ]));
            if intel >= IntelLevel::Informed {
                lines.push(Line::from(vec![
                    Span::styled("Relations ", Theme::muted_style()),
                    Span::styled(
                        format!(
                            "{}  tension {}  trust {}",
                            relationship.relationship_score,
                            relationship.tension_score,
                            relationship
                                .trust_score
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "-".to_string())
                        ),
                        Theme::default_style(),
                    ),
                ]));
            }
        }

        lines.push(Line::from(vec![
            Span::styled("Colonies ", Theme::muted_style()),
            Span::styled(
                if intel.reveals_colony_count() {
                    game_state.empire_colony_count(empire_id).to_string()
                } else {
                    "Hidden".to_string()
                },
                if intel.reveals_colony_count() {
                    Theme::default_style()
                } else {
                    Theme::muted_style()
                },
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Fleet strength ", Theme::muted_style()),
            Span::styled(
                if intel.reveals_fleet_strength() {
                    if intel >= IntelLevel::Informed {
                        format!(
                            "{} ({})",
                            game_state.empire_total_fleet_strength(empire_id),
                            game_state.empire_fleet_strength_band(empire_id)
                        )
                    } else {
                        game_state.empire_fleet_strength_band(empire_id).to_string()
                    }
                } else {
                    "Hidden".to_string()
                },
                if intel.reveals_fleet_strength() {
                    Theme::default_style()
                } else {
                    Theme::muted_style()
                },
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Tech level ", Theme::muted_style()),
            Span::styled(
                if intel.reveals_tech_level() {
                    game_state
                        .empire_highest_tech_tier(empire_id)
                        .map(|tier| tier.short_label().to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                } else {
                    "Hidden".to_string()
                },
                if intel.reveals_tech_level() {
                    Theme::default_style()
                } else {
                    Theme::muted_style()
                },
            ),
        ]));
        if intel.reveals_economy_summary() {
            if let Some(summary) = game_state.empire_economy_summary(empire_id) {
                lines.push(Line::from(vec![
                    Span::styled("Economy ", Theme::muted_style()),
                    Span::styled(
                        format!(
                            "F {:+} · I {} · S {} · C {}",
                            summary.food_balance,
                            summary.industry,
                            summary.science,
                            summary.credits
                        ),
                        Theme::default_style(),
                    ),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("Economy ", Theme::muted_style()),
                Span::styled("Hidden", Theme::muted_style()),
            ]));
        }

        if intel.reveals_strategic_resources() {
            let resources = game_state.visible_empire_resources_for_player(empire_id);
            lines.push(Line::from(vec![
                Span::styled("Strategic assets ", Theme::muted_style()),
                Span::styled(
                    if resources.is_empty() {
                        "None".to_string()
                    } else {
                        resources
                            .iter()
                            .map(|(resource, count)| format!("{} x{}", resource.name(), count))
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                    Theme::default_style(),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Strategic assets ", Theme::muted_style()),
                Span::styled("Hidden", Theme::muted_style()),
            ]));
        }
    }

    if selected_empire.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Active intel: [i] gather  [z] sabotage placeholder  [y] research theft placeholder",
            Theme::muted_style(),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: true }),
        area,
    );
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

    let selected_response_index = app_state
        .diplomacy
        .selected_response_index
        .min(msg.available_responses.len().saturating_sub(1));
    for (idx, response) in msg.available_responses.iter().copied().enumerate() {
        let selected = idx == selected_response_index;
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

    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: true }),
        inner,
    );
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

        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        if let Some(ai_empire) = engine.state.empires.get_mut(&ai_id) {
            ai_empire.name = "Leaked Empire".to_string();
        }
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
        assert!(!rendered.contains("Leaked Empire"));
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

    #[test]
    fn basic_intel_reveals_limited_summary() {
        use game_core::{EmpireIntel, IntelLevel, RelationshipStatus, StrategicResource};

        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Neutral);
        engine.state.empire_intel.insert(
            ai_id,
            EmpireIntel {
                level: IntelLevel::Basic,
                points: 4,
                last_gather_turn: None,
            },
        );
        engine
            .state
            .empire_resource_access
            .entry(ai_id)
            .or_default()
            .insert(StrategicResource::QuantumCrystals, 2);

        let rendered = render_to_string(&engine);
        assert!(rendered.contains("Intel Basic"));
        assert!(rendered.contains("Colonies"));
        assert!(rendered.contains("Fleet strength"));
        assert!(!rendered.contains("Quantum Crystals"));
        assert!(rendered.contains("Strategic assets Hidden"));
    }

    #[test]
    fn deep_intel_reveals_fuller_summary() {
        use game_core::{EmpireIntel, IntelLevel, RelationshipStatus, StrategicResource, TechId};

        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Neutral);
        engine.state.diplomacy_relationships.insert(
            ai_id,
            game_core::DiplomaticRelationship::from_status(RelationshipStatus::Neutral),
        );
        engine.state.empire_intel.insert(
            ai_id,
            EmpireIntel {
                level: IntelLevel::Deep,
                points: 16,
                last_gather_turn: None,
            },
        );
        engine
            .state
            .empire_resource_access
            .entry(ai_id)
            .or_default()
            .insert(StrategicResource::QuantumCrystals, 2);
        if let Some(empire) = engine.state.empires.get_mut(&ai_id) {
            empire.research.completed.push(TechId::SECTOR_CARTOGRAPHY);
        }

        let rendered = render_to_string(&engine);
        assert!(rendered.contains("Intel Deep"));
        assert!(rendered.contains("Tech level"));
        assert!(rendered.contains("Economy"));
        assert!(rendered.contains("Quantum Crystals x2"));
    }
}
