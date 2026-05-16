//! System inspector screen

use std::borrow::Cow;

use crate::components::{derive_header_data, render_footer, render_header, render_log};
use crate::layout::{compose_layout, split_horizontal};
use crate::renderer::{
    palette::ColorToken,
    planet_art::{planet_kind_from_class, planet_sprite, star_sprite, PlanetVisualKind},
    sprite::{detail_for_area, DetailLevel},
    Canvas, RenderLayer,
};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{ColonySupplyState, FleetKind, GameState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const ORBIT_SELECTION_PULSE_PERIOD: u64 = 5;

pub fn render_system(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let (header_area, main_area, footer_area) = compose_layout(area);
    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let (left, right) = split_horizontal(main_area, 55);
    render_orbital_panel(frame, left, app_state, game_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(right);
    render_system_details(frame, right_chunks[0], app_state, game_state);
    render_log(frame, right_chunks[1], &app_state.log);

    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Survey with S, colonize with C, and invade hostile colonies with I.");
    render_footer(frame, footer_area, &Screen::System, Some(hint));
}

fn selected_star<'a>(
    app_state: &AppState,
    game_state: &'a GameState,
) -> Option<&'a game_core::Star> {
    app_state
        .navigation
        .selected_star
        .and_then(|id| game_state.stars.get(&id))
}

fn planet_survey_state(
    game_state: &GameState,
    star_id: game_core::StarId,
    planet_index: usize,
    planet_surveyed: bool,
) -> &'static str {
    if !game_state.explored_stars.contains(&star_id) {
        return "Unknown";
    }

    if game_state
        .survey_missions
        .values()
        .any(|mission| mission.star == star_id && mission.planet_index == planet_index)
    {
        "Surveying"
    } else if planet_surveyed {
        "Surveyed"
    } else {
        "Unsurveyed"
    }
}

fn render_orbital_panel(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let block = Block::default()
        .title(" System Orbits ")
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let star = match selected_star(app_state, game_state) {
        Some(star) => star,
        None => {
            frame.render_widget(
                Paragraph::new("No system selected").style(Theme::muted_style()),
                inner,
            );
            return;
        }
    };

    let mut lines = vec![Line::from(vec![
        Span::styled("☉ ", Theme::title_style()),
        Span::styled(star.name.as_str(), Theme::title_style()),
        Span::styled(
            format!(" [{}]", star.spectral_class.as_char()),
            Style::default().fg(Theme::star_color(star.spectral_class)),
        ),
    ])];
    lines.push(Line::from(""));

    let split_height = (inner.height / 2)
        .max(4)
        .min(inner.height.saturating_sub(2));
    let panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(split_height), Constraint::Min(1)])
        .split(inner);
    render_system_visual(frame, panel_chunks[0], app_state, star);

    let selected_planet = app_state
        .navigation
        .selected_planet_index
        .min(star.planets.len().saturating_sub(1));
    for (index, planet) in star.planets.iter().enumerate() {
        let selected = index == selected_planet;
        let prefix = if selected { "▶" } else { " " };
        let survey_state = planet_survey_state(game_state, star.id, index, planet.surveyed);
        let surveyed_mark = match survey_state {
            "Surveyed" => "S",
            "Surveying" => "~",
            "Unsurveyed" => "?",
            _ => "!",
        };
        // Determine if this planet's colony is currently blockaded
        let blockade_mark = planet
            .colony
            .and_then(|cid| game_state.colony_blockade_state(cid))
            .map(|_| "⚔")
            .unwrap_or("");
        let invasion_mark = planet
            .colony
            .and_then(|cid| game_state.colonies.get(&cid))
            .and_then(|colony| {
                (colony.owner != game_state.player_empire
                    && game_state
                        .relationship_status(game_state.player_empire, colony.owner)
                        .is_hostile_or_war())
                .then_some(" [I]")
            })
            .unwrap_or("");
        let colony_mark = if planet.colony.is_some() {
            "◉"
        } else {
            "○"
        };
        let label: Cow<'_, str> = if survey_state == "Surveyed" {
            Cow::Borrowed(planet.name.as_str())
        } else {
            Cow::Owned(format!("Orbit {}", index + 1))
        };
        let is_blockaded = !blockade_mark.is_empty();
        let style = if selected {
            Theme::highlight_style()
        } else if is_blockaded {
            Theme::error_style()
        } else if survey_state == "Surveyed" {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };
        let mut spans = vec![
            Span::raw(format!("{} ", prefix)),
            Span::styled(format!("{} ", colony_mark), style),
            Span::styled(format!("{} ", surveyed_mark), style),
            Span::styled(format!("{}{}", label, invasion_mark), style),
            Span::styled(format!(" — {}", survey_state), Theme::muted_style()),
        ];
        if is_blockaded {
            spans.push(Span::styled(
                format!(" {}", blockade_mark),
                Theme::error_style(),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Theme::default_style()),
        panel_chunks[1],
    );
}

fn render_system_visual(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    star: &game_core::Star,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let detail = detail_for_area(area);
    let mut canvas = Canvas::new(area.width, area.height);
    canvas.fill(ColorToken::SpaceBg, RenderLayer::Background.z_base());

    let star_detail = if matches!(detail, DetailLevel::Cinematic) {
        DetailLevel::Standard
    } else {
        detail
    };
    let star_visual_sprite = star_sprite(star.spectral_class, star_detail);
    let center_x = area.width / 2;
    let center_y = area.height / 2;
    let star_x = center_x.saturating_sub(star_visual_sprite.width / 2);
    let star_y = center_y.saturating_sub(star_visual_sprite.height / 2);
    canvas.draw_sprite(
        &star_visual_sprite,
        star_x,
        star_y,
        0,
        RenderLayer::Bodies.z_base(),
    );

    if !star.planets.is_empty() {
        let spacing = (area.width / (star.planets.len() as u16 + 1)).max(2);
        let selected_planet = app_state
            .navigation
            .selected_planet_index
            .min(star.planets.len().saturating_sub(1));
        for (index, planet) in star.planets.iter().enumerate() {
            let x = spacing.saturating_mul(index as u16 + 1);
            let y = if index % 2 == 0 {
                center_y.saturating_add(1)
            } else {
                center_y.saturating_sub(1)
            };
            let kind = if planet.surveyed {
                planet_kind_from_class(Some(planet.class))
            } else {
                PlanetVisualKind::Barren
            };
            let sprite = planet_sprite(kind, DetailLevel::Tiny);
            canvas.draw_sprite(
                &sprite,
                x.saturating_sub(sprite.width / 2),
                y.saturating_sub(sprite.height / 2),
                0,
                RenderLayer::Bodies.z_base() + 1,
            );
            if index == selected_planet {
                let pulse = if app_state.reduced_motion {
                    '◌'
                } else if (app_state.tick_count / ORBIT_SELECTION_PULSE_PERIOD).is_multiple_of(2) {
                    '◉'
                } else {
                    '◌'
                };
                canvas.set_cell(
                    x.min(area.width.saturating_sub(1)),
                    y.saturating_add(1).min(area.height.saturating_sub(1)),
                    pulse,
                    ColorToken::Accent.to_style(None),
                    RenderLayer::Selection.z_base(),
                );
            }
        }
    }

    canvas.draw_text(
        1,
        0,
        &format!("{} [{}]", star.name, star.spectral_class.as_char()),
        ColorToken::Accent.to_style(None),
        RenderLayer::Labels.z_base(),
    );
    canvas.render_to_buffer(area, frame.buffer_mut());
}

fn render_system_details(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let block = Block::default()
        .title(" Planet Detail ")
        .borders(Borders::ALL)
        .border_style(Theme::dim_border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let star = match selected_star(app_state, game_state) {
        Some(star) => star,
        None => {
            frame.render_widget(
                Paragraph::new("No system selected").style(Theme::muted_style()),
                inner,
            );
            return;
        }
    };

    if star.planets.is_empty() {
        frame.render_widget(
            Paragraph::new("System has no planets").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let selected_planet = app_state
        .navigation
        .selected_planet_index
        .min(star.planets.len().saturating_sub(1));
    let planet = &star.planets[selected_planet];

    let fleets_here: Vec<_> = game_state
        .fleets
        .values()
        .filter(|fleet| {
            fleet.location == star.id
                && !game_state.fleet_missions.contains_key(&fleet.id)
                && !game_state.scout_missions.contains_key(&fleet.id)
        })
        .collect();

    let survey_state = planet_survey_state(game_state, star.id, selected_planet, planet.surveyed);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(star.name.as_str(), Theme::title_style()),
            Span::styled(
                format!(" [{}]", star.spectral_class.as_char()),
                Style::default().fg(Theme::star_color(star.spectral_class)),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Orbit: ", Theme::muted_style()),
            Span::raw((selected_planet + 1).to_string()),
        ]),
        Line::from(vec![
            Span::styled("Survey: ", Theme::muted_style()),
            Span::raw(survey_state),
        ]),
    ];

    if survey_state == "Surveyed" {
        lines.push(Line::from(vec![
            Span::styled("Class: ", Theme::muted_style()),
            Span::raw(planet.class.name()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Size: ", Theme::muted_style()),
            Span::raw(format!("{:?}", planet.size)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Habitability: ", Theme::muted_style()),
            Span::raw(if planet.habitable {
                "Habitable"
            } else {
                "Uninhabitable"
            }),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Modifiers: ", Theme::muted_style()),
            Span::raw(format!(
                "{:+} food, {:+} science",
                planet.class.food_bonus(),
                planet.class.science_bonus()
            )),
        ]));
        // Show planet specials (revealed after survey)
        if planet.specials.is_empty() && planet.resources.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Specials: ", Theme::muted_style()),
                Span::styled("None", Theme::muted_style()),
            ]));
        } else {
            if !planet.specials.is_empty() {
                let specials_text: Vec<String> = planet
                    .specials
                    .iter()
                    .map(|s| format!("{} ({})", s.name(), s.description()))
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("Specials: ", Theme::muted_style()),
                    Span::styled(specials_text.join(", "), Theme::accent_style()),
                ]));
            }
            if !planet.resources.is_empty() {
                let resources_text: Vec<String> = planet
                    .resources
                    .iter()
                    .map(|r| format!("{} ({})", r.name(), r.description()))
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("Resources: ", Theme::muted_style()),
                    Span::styled(resources_text.join(", "), Theme::accent_style()),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("Class: ", Theme::muted_style()),
            Span::styled("Unknown", Theme::muted_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Size: ", Theme::muted_style()),
            Span::styled("Unknown", Theme::muted_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Habitability: ", Theme::muted_style()),
            Span::styled("Unknown", Theme::muted_style()),
        ]));
    }

    let colony_line = if let Some(colony_id) = planet.colony {
        if let Some(colony) = game_state.colonies.get(&colony_id) {
            let infra = colony.surface_installations.len() + colony.orbital_installations.len();
            let supply = game_state.colony_supply_state(colony.id);
            format!(
                "Colony {} (Empire {}, Pop {}, Infra {}, {}, {})",
                colony_id.0,
                colony.owner.0,
                colony.population,
                infra,
                supply.label(),
                colony.unrest_label()
            )
        } else {
            format!("Colony {}", colony_id.0)
        }
    } else {
        "Uncolonized".to_string()
    };
    lines.push(Line::from(vec![
        Span::styled("Status: ", Theme::muted_style()),
        Span::raw(colony_line),
    ]));

    if let Some(colony_id) = planet.colony {
        if let Some(colony) = game_state.colonies.get(&colony_id) {
            let supply = game_state.colony_supply_state(colony.id);
            lines.push(Line::from(vec![
                Span::styled("Trade:  ", Theme::muted_style()),
                Span::styled(
                    supply.label(),
                    if supply == ColonySupplyState::Isolated {
                        Theme::warning_style()
                    } else {
                        Theme::accent_style()
                    },
                ),
            ]));
        }
    }

    // Show rally point for player-owned colony at this planet
    if let Some(colony_id) = planet.colony {
        if let Some(colony) = game_state.colonies.get(&colony_id) {
            if colony.owner == game_state.player_empire {
                let rally_text = match colony.rally_point {
                    Some(star_id) => {
                        let sname = game_state
                            .stars
                            .get(&star_id)
                            .map(|s| s.name.as_str())
                            .unwrap_or("Unknown");
                        format!("{} ({})", sname, star_id.0)
                    }
                    None => "None".to_string(),
                };
                lines.push(Line::from(vec![
                    Span::styled("Rally:  ", Theme::muted_style()),
                    Span::styled(rally_text, Theme::accent_style()),
                ]));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Fleets:", Theme::title_style())));
    if fleets_here.is_empty() {
        lines.push(Line::from(Span::styled("  None", Theme::muted_style())));
    } else {
        for fleet in fleets_here {
            let order_label = match game_state.fleet_orders.get(&fleet.id) {
                Some(game_core::FleetOrder::Hold) => " [Hold]".to_string(),
                Some(game_core::FleetOrder::MoveToSystem(star_id)) => {
                    format!(" [→ {}]", star_id.0)
                }
                None => String::new(),
            };
            let label = match fleet.kind {
                FleetKind::Colonizer => {
                    format!("  Colony Ship {}{}", fleet.id.0, order_label)
                }
                FleetKind::ColonyArk => {
                    format!("  Colony Ark {}{}", fleet.id.0, order_label)
                }
                FleetKind::Scout => format!("  Scout {}{}", fleet.id.0, order_label),
                FleetKind::FastScout => {
                    format!("  Fast Scout {}{}", fleet.id.0, order_label)
                }
                FleetKind::Science => {
                    let mission = game_state.survey_missions.get(&fleet.id);
                    match mission {
                        Some(mission) => format!(
                            "  Science Ship {} (Surveying orbit {}){}",
                            fleet.id.0,
                            mission.planet_index + 1,
                            order_label
                        ),
                        None => format!("  Science Ship {}{}", fleet.id.0, order_label),
                    }
                }
                FleetKind::SurveyCutter => {
                    let mission = game_state.survey_missions.get(&fleet.id);
                    match mission {
                        Some(mission) => format!(
                            "  Survey Cutter {} (Surveying orbit {}){}",
                            fleet.id.0,
                            mission.planet_index + 1,
                            order_label
                        ),
                        None => format!("  Survey Cutter {}{}", fleet.id.0, order_label),
                    }
                }
                FleetKind::TroopTransport => {
                    format!("  Troop Transport {}{}", fleet.id.0, order_label)
                }
                FleetKind::EscortFrigate => {
                    format!(
                        "  Escort Frigate {} [str:{}]{}",
                        fleet.id.0, fleet.strength, order_label
                    )
                }
                FleetKind::MissileFrigate => {
                    format!(
                        "  Missile Frigate {} [str:{}]{}",
                        fleet.id.0, fleet.strength, order_label
                    )
                }
                FleetKind::Destroyer => {
                    format!(
                        "  Destroyer {} [str:{}]{}",
                        fleet.id.0, fleet.strength, order_label
                    )
                }
                FleetKind::PatrolCorvette => {
                    format!(
                        "  Patrol Corvette {} [str:{}]{}",
                        fleet.id.0, fleet.strength, order_label
                    )
                }
            };
            lines.push(Line::from(Span::raw(label)));
        }
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn system_screen_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_star: engine.state.stars.keys().next().copied(),
                ..Default::default()
            },
            ..Default::default()
        };
        terminal
            .draw(|frame| render_system(frame, frame.area(), &app_state, &engine.state))
            .unwrap();
    }

    #[test]
    fn unsurveyed_planet_hides_details() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        let star_id = engine.state.stars.keys().next().copied().unwrap();
        if let Some(star) = engine.state.stars.get_mut(&star_id) {
            if let Some(planet) = star.planets.get_mut(0) {
                planet.surveyed = false;
            }
        }
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_star: Some(star_id),
                selected_planet_index: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        terminal
            .draw(|frame| render_system(frame, frame.area(), &app_state, &engine.state))
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..40u16)
            .flat_map(|y| {
                (0..120u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();
        assert!(
            rendered.contains("Class: Unknown"),
            "unsurveyed details should hide class"
        );
    }

    #[test]
    fn surveyed_planet_shows_details() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        let star_id = engine.state.stars.keys().next().copied().unwrap();
        engine.state.explored_stars.insert(star_id);
        if let Some(star) = engine.state.stars.get_mut(&star_id) {
            if let Some(planet) = star.planets.get_mut(0) {
                planet.surveyed = true;
                planet.class = game_core::PlanetClass::Terran;
            }
        }
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_star: Some(star_id),
                selected_planet_index: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        terminal
            .draw(|frame| render_system(frame, frame.area(), &app_state, &engine.state))
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..40u16)
            .flat_map(|y| {
                (0..120u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();
        assert!(
            rendered.contains("Class: Terran"),
            "surveyed details should show class"
        );
    }

    #[test]
    fn system_screen_shows_trade_status_for_colony() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        let star_id = engine.state.colonies[&colony_id].star;
        engine
            .state
            .colony_supply
            .insert(colony_id, game_core::ColonySupplyState::Isolated);
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_star: Some(star_id),
                selected_planet_index: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        terminal
            .draw(|frame| render_system(frame, frame.area(), &app_state, &engine.state))
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..40u16)
            .flat_map(|y| {
                (0..120u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();
        assert!(rendered.contains("Trade:"));
        assert!(rendered.contains("Isolated"));
    }

    #[test]
    fn surveying_planet_shows_progress_state() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        let star_id = engine.state.stars.keys().next().copied().unwrap();
        engine.state.explored_stars.insert(star_id);
        if let Some(star) = engine.state.stars.get_mut(&star_id) {
            if let Some(planet) = star.planets.get_mut(0) {
                planet.surveyed = false;
            }
        }
        let science_fleet = game_core::FleetId(999);
        engine.state.fleets.insert(
            science_fleet,
            game_core::Fleet {
                id: science_fleet,
                owner: engine.state.player_empire,
                location: star_id,
                ships: 1,
                kind: game_core::FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );
        engine.state.survey_missions.insert(
            science_fleet,
            game_core::SurveyMission {
                fleet: science_fleet,
                star: star_id,
                planet_index: 0,
                turns_remaining: 2,
            },
        );
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_star: Some(star_id),
                selected_planet_index: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        terminal
            .draw(|frame| render_system(frame, frame.area(), &app_state, &engine.state))
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..40u16)
            .flat_map(|y| {
                (0..120u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();
        assert!(
            rendered.contains("Survey: Surveying"),
            "surveying details should show surveying state"
        );
    }
}
