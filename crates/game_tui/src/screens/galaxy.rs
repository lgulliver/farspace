//! Galaxy map screen

use crate::components::{render_footer, render_header, render_log};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{GameState, StarId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the galaxy map screen
pub fn render_galaxy(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    // Get empire info
    let empire = game_state.empires.get(&game_state.player_empire);
    let (credits, research, empire_name) = match empire {
        Some(e) => (e.credits, e.research_points, e.name.as_str()),
        None => (0, 0, "Unknown"),
    };

    // Render header
    render_header(
        frame,
        header_area,
        game_state.turn,
        empire_name,
        credits,
        research,
    );

    // Split main area: 60% map, 40% right column
    let (map_area, right_area) = split_horizontal(main_area, 60);

    // Render star map
    render_star_map(frame, map_area, game_state, app_state.selected_star);

    // Split right column: 60% star details, 40% event log
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_area);

    // Render star details
    render_star_details(frame, right_chunks[0], game_state, app_state.selected_star);

    // Render event log
    render_log(frame, right_chunks[1], &app_state.log);

    // Render footer
    render_footer(frame, footer_area, &Screen::Galaxy);
}

/// Render the star map
fn render_star_map(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_star: Option<StarId>,
) {
    let block = Block::default()
        .title(" Galaxy Map ")
        .borders(Borders::ALL)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate map bounds
    let map_width = inner.width.saturating_sub(2) as i32;
    let map_height = inner.height.saturating_sub(1) as i32;

    if map_width <= 0 || map_height <= 0 {
        return;
    }

    // Collect stars that have an active scout en route (by destination)
    let scout_destinations: std::collections::BTreeSet<StarId> = game_state
        .scout_missions
        .values()
        .map(|m| m.destination)
        .collect();

    // Collect stars that have an active fleet mission en route (by destination)
    let fleet_destinations: std::collections::BTreeSet<StarId> = game_state
        .fleet_missions
        .values()
        .map(|m| m.destination)
        .collect();

    // Scale stars to fit the map area
    // Stars are in range -500..500, map to 0..map_width/height
    for star in game_state.stars.values() {
        let screen_x = ((star.x + 500) * map_width / 1000).clamp(0, map_width - 1);
        let screen_y = ((star.y + 500) * map_height / 1000).clamp(0, map_height - 1);

        let x = inner.x + screen_x as u16;
        let y = inner.y + screen_y as u16;

        // Check bounds
        if x >= inner.x + inner.width || y >= inner.y + inner.height {
            continue;
        }

        let is_selected = selected_star == Some(star.id);
        let is_explored = game_state.explored_stars.contains(&star.id);
        let scout_en_route = scout_destinations.contains(&star.id);
        let fleet_en_route = fleet_destinations.contains(&star.id);

        let (render_char, style) = if is_selected {
            ('@', Theme::highlight_style())
        } else if scout_en_route {
            // Scout is heading here — show with a distinct marker
            (
                '+',
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else if fleet_en_route {
            // Fleet is heading here
            (
                '~',
                Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_explored {
            (
                '*',
                Style::default().fg(Theme::star_color(star.spectral_class)),
            )
        } else {
            // Unexplored: dim question mark
            ('?', Theme::muted_style())
        };

        let star_widget = Paragraph::new(render_char.to_string()).style(style);
        frame.render_widget(star_widget, Rect::new(x, y, 1, 1));
    }
}

/// Render star details panel
fn render_star_details(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_star: Option<StarId>,
) {
    let block = Block::default()
        .title(" Star Details ")
        .borders(Borders::ALL)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let star = match selected_star.and_then(|id| game_state.stars.get(&id)) {
        Some(s) => s,
        None => {
            let no_selection = Paragraph::new("No star selected").style(Theme::muted_style());
            frame.render_widget(no_selection, inner);
            return;
        }
    };

    let is_explored = game_state.explored_stars.contains(&star.id);

    if !is_explored {
        // Unknown system — show limited info only
        // Find mission in a single pass (reused below to avoid duplicate search)
        let active_mission = game_state
            .scout_missions
            .values()
            .find(|m| m.destination == star.id);

        let mut lines = vec![
            Line::from(vec![Span::styled(
                "[ Unknown System ]",
                Theme::muted_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "No data available.",
                Theme::muted_style(),
            )]),
        ];

        if let Some(mission) = active_mission {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Scout en route — ", Theme::accent_style()),
                Span::raw(format!("{} turn(s) remaining", mission.turns_remaining)),
            ]));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Press S to dispatch a scout.",
                Theme::muted_style(),
            )]));
        }

        let paragraph = Paragraph::new(lines).style(Theme::default_style());
        frame.render_widget(paragraph, inner);
        return;
    }

    // Explored system — show full details
    let mut lines = vec![
        Line::from(vec![Span::styled(&star.name, Theme::title_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Class: ", Theme::muted_style()),
            Span::styled(
                format!("{}", star.spectral_class.as_char()),
                Style::default().fg(Theme::star_color(star.spectral_class)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Theme::muted_style()),
            Span::raw(format!("({}, {})", star.x, star.y)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Planets:", Theme::title_style())),
    ];

    for planet in &star.planets {
        let colony_info = match &planet.colony {
            Some(colony_id) => {
                if let Some(colony) = game_state.colonies.get(colony_id) {
                    format!(" [Colony - Pop: {}]", colony.population)
                } else {
                    String::new()
                }
            }
            None if planet.habitable => " [Habitable]".to_string(),
            None => " [Uninhabitable]".to_string(),
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(&planet.name, Theme::default_style()),
            Span::styled(format!(" ({:?})", planet.size), Theme::muted_style()),
            Span::styled(colony_info, Theme::accent_style()),
        ]));
    }

    // Fleets present at this star
    let fleets_here: Vec<_> = game_state
        .fleets
        .values()
        .filter(|f| {
            f.location == star.id
                && !game_state.fleet_missions.contains_key(&f.id)
                && !game_state.scout_missions.contains_key(&f.id)
        })
        .collect();

    // Fleets en route to this star
    let fleets_en_route: Vec<_> = game_state
        .fleet_missions
        .values()
        .filter(|m| m.destination == star.id)
        .collect();
    let scouts_en_route: Vec<_> = game_state
        .scout_missions
        .values()
        .filter(|m| m.destination == star.id)
        .collect();

    if !fleets_here.is_empty() || !fleets_en_route.is_empty() || !scouts_en_route.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Fleets:", Theme::title_style())));
        for fleet in &fleets_here {
            let label = if fleet.kind == game_core::FleetKind::Colonizer {
                format!("Colony Ship {} (idle)", fleet.id.0)
            } else {
                format!("Fleet {} (idle)", fleet.id.0)
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(label, Theme::accent_style()),
            ]));
        }
        for mission in &fleets_en_route {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "Fleet {} en route ({} turn(s))",
                        mission.fleet.0, mission.turns_remaining
                    ),
                    Style::default().fg(ratatui::style::Color::Cyan),
                ),
            ]));
        }
        for mission in &scouts_en_route {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "Scout {} en route ({} turn(s))",
                        mission.fleet.0, mission.turns_remaining
                    ),
                    Style::default().fg(ratatui::style::Color::Yellow),
                ),
            ]));
        }
    }

    // Show colonize hint if a colonizer is present and there's a habitable unowned planet
    let colonizer_present = game_state.fleets.values().any(|f| {
        f.location == star.id
            && f.kind == game_core::FleetKind::Colonizer
            && !game_state.fleet_missions.contains_key(&f.id)
            && !game_state.scout_missions.contains_key(&f.id)
    });
    let has_colonizable = star
        .planets
        .iter()
        .any(|p| p.habitable && p.colony.is_none());
    if colonizer_present && has_colonizable {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Press C to colonize.",
            Theme::accent_style(),
        )]));
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn galaxy_screen_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn galaxy_screen_with_selection() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState {
            selected_star: engine.state.stars.keys().next().copied(),
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn galaxy_screen_with_unexplored_selected() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        // Select an unexplored star (Engine::new(42) explores at most 4 of 20 stars)
        let unexplored = engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .copied()
            .expect("Engine::new(42) must have unexplored stars");

        let app_state = AppState {
            selected_star: Some(unexplored),
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn galaxy_screen_with_scout_en_route() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = Engine::new(42);
        use game_core::{Command, FleetId};

        let dest = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        engine.apply_turn(vec![Command::SendScout {
            fleet: FleetId(1),
            destination: dest,
        }]);

        let app_state = AppState {
            selected_star: Some(dest),
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn galaxy_screen_with_fleet_mission_en_route() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = Engine::new(42);
        use game_core::{Command, FleetId};

        // Move to an explored star
        let fleet_id = FleetId(1);
        let initial = engine.state.fleets.get(&fleet_id).unwrap().location;
        let dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != initial)
            .expect("Need explored star other than home");

        engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: dest,
        }]);

        let app_state = AppState {
            selected_star: Some(dest),
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn galaxy_screen_with_idle_fleet_at_system() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        // Select the home star which has the initial fleet
        let fleet_id = game_core::FleetId(1);
        let home_star = engine.state.fleets.get(&fleet_id).unwrap().location;

        let app_state = AppState {
            selected_star: Some(home_star),
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }
}
