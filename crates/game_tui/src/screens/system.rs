//! System inspector screen

use std::borrow::Cow;

use crate::components::{render_footer, render_header, render_log};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{FleetKind, GameState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_system(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let (header_area, main_area, footer_area) = compose_layout(area);
    let empire = game_state.empires.get(&game_state.player_empire);
    let (credits, food, research, empire_name) = match empire {
        Some(e) => (e.credits, e.food, e.research_points, e.name.as_str()),
        None => (0, 0, 0, "Unknown"),
    };

    render_header(
        frame,
        header_area,
        game_state.turn,
        empire_name,
        credits,
        food,
        research,
    );

    let (left, right) = split_horizontal(main_area, 55);
    render_orbital_panel(frame, left, app_state, game_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(right);
    render_system_details(frame, right_chunks[0], app_state, game_state);
    render_log(frame, right_chunks[1], &app_state.log);

    render_footer(frame, footer_area, &Screen::System);
}

fn selected_star<'a>(
    app_state: &AppState,
    game_state: &'a GameState,
) -> Option<&'a game_core::Star> {
    app_state
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

    let selected_planet = app_state
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
        let style = if selected {
            Theme::highlight_style()
        } else if survey_state == "Surveyed" {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{} ", prefix)),
            Span::styled(format!("{} ", colony_mark), style),
            Span::styled(format!("{} ", surveyed_mark), style),
            Span::styled(label, style),
            Span::styled(format!(" — {}", survey_state), Theme::muted_style()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
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
            format!(
                "Colony {} (Pop {}, Infra {})",
                colony_id.0, colony.population, infra
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

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Fleets:", Theme::title_style())));
    if fleets_here.is_empty() {
        lines.push(Line::from(Span::styled("  None", Theme::muted_style())));
    } else {
        for fleet in fleets_here {
            let label = match fleet.kind {
                FleetKind::Colonizer => format!("  Colony Ship {}", fleet.id.0),
                FleetKind::Scout => format!("  Scout {}", fleet.id.0),
                FleetKind::Science => {
                    let mission = game_state.survey_missions.get(&fleet.id);
                    match mission {
                        Some(mission) => format!(
                            "  Science Ship {} (Surveying orbit {})",
                            fleet.id.0,
                            mission.planet_index + 1
                        ),
                        None => format!("  Science Ship {}", fleet.id.0),
                    }
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
            selected_star: engine.state.stars.keys().next().copied(),
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
            selected_star: Some(star_id),
            selected_planet_index: 0,
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
            selected_star: Some(star_id),
            selected_planet_index: 0,
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
            selected_star: Some(star_id),
            selected_planet_index: 0,
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
