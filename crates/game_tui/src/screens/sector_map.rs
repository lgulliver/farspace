//! Sector map screen - shows systems within a selected sector

use std::borrow::Cow;

use crate::components::{render_footer, render_header, render_log};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{GameState, SectorId, StarId, TechId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_sector_map(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
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

    let (map_area, right_area) = split_horizontal(main_area, 55);

    render_local_map(frame, map_area, game_state, app_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_area);

    render_system_list(frame, right_chunks[0], game_state, app_state);
    render_log(frame, right_chunks[1], &app_state.log);

    render_footer(frame, footer_area, &Screen::SectorMap);
}

fn render_local_map(frame: &mut Frame, area: Rect, game_state: &GameState, app_state: &AppState) {
    let sector_name = app_state
        .selected_sector
        .and_then(|id| game_state.sectors.get(&id))
        .map(|s| s.name.as_str())
        .unwrap_or("Unknown");

    let title = format!(" {} — Systems ", sector_name);

    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let map_height = inner.height.saturating_sub(2) as i32;
    let map_width = inner.width.saturating_sub(2) as i32;

    if map_width <= 0 || map_height <= 0 {
        return;
    }

    let sector_id = match app_state.selected_sector {
        Some(id) => id,
        None => return,
    };

    // Verify sector exists
    if !game_state.sectors.contains_key(&sector_id) {
        return;
    }

    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector_id)
        .collect();

    if stars_in_sector.is_empty() {
        return;
    }

    let min_x = stars_in_sector.iter().map(|s| s.x).min().unwrap_or(-100);
    let max_x = stars_in_sector.iter().map(|s| s.x).max().unwrap_or(100);
    let min_y = stars_in_sector.iter().map(|s| s.y).min().unwrap_or(-100);
    let max_y = stars_in_sector.iter().map(|s| s.y).max().unwrap_or(100);

    let bounds = MapBounds {
        min_x,
        min_y,
        range_x: (max_x - min_x).max(1),
        range_y: (max_y - min_y).max(1),
        map_width,
        map_height,
        inner,
    };

    let scout_destinations: std::collections::BTreeSet<StarId> = game_state
        .scout_missions
        .values()
        .map(|m| m.destination)
        .collect();

    let fleet_destinations: std::collections::BTreeSet<StarId> = game_state
        .fleet_missions
        .values()
        .map(|m| m.destination)
        .collect();

    render_known_lanes_in_sector(frame, game_state, app_state, sector_id, &bounds);

    // Render star cells
    for star in &stars_in_sector {
        let Some((x, y)) = world_to_screen_f(star.x as f64, star.y as f64, &bounds) else {
            continue;
        };

        let is_selected = app_state.selected_star == Some(star.id);
        let is_explored = game_state.explored_stars.contains(&star.id);
        let scout_en_route = scout_destinations.contains(&star.id);
        let fleet_en_route = fleet_destinations.contains(&star.id);

        let (render_char, style) = if is_selected {
            ('@', Theme::highlight_style())
        } else if scout_en_route {
            (
                '+',
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else if fleet_en_route {
            (
                '~',
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_explored {
            (
                '*',
                Style::default().fg(Theme::star_color(star.spectral_class)),
            )
        } else {
            ('?', Theme::muted_style())
        };

        let star_widget = Paragraph::new(render_char.to_string()).style(style);
        frame.render_widget(star_widget, Rect::new(x, y, 1, 1));
    }

    // Render in-transit fleet indicators (cosmetic only, no game state effect).
    // Visible when reduced_motion is false and the mission is within this sector.
    if !app_state.reduced_motion {
        // Low-frequency blink: show indicator every other ~5-tick window (≈500 ms at 100 ms/tick)
        let show_indicator = (app_state.tick_count / 5).is_multiple_of(2);
        if show_indicator {
            render_travelling_fleets(frame, game_state, sector_id, &bounds);
        }
    }

    if inner.height >= 2 && inner.width >= 10 {
        render_local_legend(
            frame,
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
}

/// Render interpolated position indicators for fleets currently travelling within the sector.
///
/// This is purely cosmetic: positions are linearly interpolated between origin and destination
/// using `elapsed / total_duration`, where `elapsed = total_duration - turns_remaining`.
/// The game state is never modified.
fn render_travelling_fleets(
    frame: &mut Frame,
    game_state: &GameState,
    sector_id: SectorId,
    bounds: &MapBounds,
) {
    let fleet_indicator_style = Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::BOLD);
    let scout_indicator_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Fleet missions (explored-star movement)
    for mission in game_state.fleet_missions.values() {
        let origin = match game_state.stars.get(&mission.origin) {
            Some(s) if s.sector == sector_id => s,
            _ => continue,
        };
        let destination = match game_state.stars.get(&mission.destination) {
            Some(s) if s.sector == sector_id => s,
            _ => continue,
        };
        if mission.total_duration == 0 {
            continue;
        }
        // progress: 0.0 (just departed) → 1.0 (arrived)
        let elapsed = mission
            .total_duration
            .saturating_sub(mission.turns_remaining) as f64;
        let progress = (elapsed / mission.total_duration as f64).clamp(0.0, 1.0);
        let wx = origin.x as f64 + (destination.x as f64 - origin.x as f64) * progress;
        let wy = origin.y as f64 + (destination.y as f64 - origin.y as f64) * progress;

        if let Some((x, y)) = world_to_screen_f(wx, wy, bounds) {
            frame.render_widget(
                Paragraph::new("►").style(fleet_indicator_style),
                Rect::new(x, y, 1, 1),
            );
        }
    }

    // Scout missions (unexplored-star scouting)
    for mission in game_state.scout_missions.values() {
        let origin = match game_state.stars.get(&mission.origin) {
            Some(s) if s.sector == sector_id => s,
            _ => continue,
        };
        let destination = match game_state.stars.get(&mission.destination) {
            Some(s) if s.sector == sector_id => s,
            _ => continue,
        };
        if mission.total_duration == 0 {
            continue;
        }
        let elapsed = mission
            .total_duration
            .saturating_sub(mission.turns_remaining) as f64;
        let progress = (elapsed / mission.total_duration as f64).clamp(0.0, 1.0);
        let wx = origin.x as f64 + (destination.x as f64 - origin.x as f64) * progress;
        let wy = origin.y as f64 + (destination.y as f64 - origin.y as f64) * progress;

        if let Some((x, y)) = world_to_screen_f(wx, wy, bounds) {
            frame.render_widget(
                Paragraph::new("►").style(scout_indicator_style),
                Rect::new(x, y, 1, 1),
            );
        }
    }
}

/// Parameters describing the bounds of the local sector map viewport.
struct MapBounds {
    min_x: i32,
    min_y: i32,
    range_x: i32,
    range_y: i32,
    map_width: i32,
    map_height: i32,
    inner: Rect,
}

/// Map floating-point world coordinates to a terminal cell within the viewport.
fn world_to_screen_f(wx: f64, wy: f64, b: &MapBounds) -> Option<(u16, u16)> {
    let rel_x = wx - b.min_x as f64;
    let rel_y = wy - b.min_y as f64;
    let screen_x = ((rel_x * b.map_width as f64) / b.range_x as f64).round() as i32;
    let screen_y = ((rel_y * b.map_height as f64) / b.range_y as f64).round() as i32;
    let screen_x = screen_x.clamp(0, b.map_width - 1);
    let screen_y = screen_y.clamp(0, b.map_height - 1);
    let x = b.inner.x + screen_x as u16;
    let y = b.inner.y + screen_y as u16;
    if x >= b.inner.x + b.inner.width || y >= b.inner.y + b.inner.height {
        None
    } else {
        Some((x, y))
    }
}

fn render_local_legend(frame: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("@", Theme::highlight_style()),
        Span::styled(" Sel  ", Theme::dim_border_style()),
        Span::styled("*", Theme::default_style()),
        Span::styled(" Explored  ", Theme::dim_border_style()),
        Span::styled("?", Theme::muted_style()),
        Span::styled(" Unknown  ", Theme::dim_border_style()),
        Span::styled("+", Style::default().fg(Color::Yellow)),
        Span::styled(" Scout  ", Theme::dim_border_style()),
        Span::styled("~", Style::default().fg(Color::Cyan)),
        Span::styled(" Fleet  ", Theme::dim_border_style()),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::styled(" Lane  ", Theme::dim_border_style()),
        Span::styled("•", Style::default().fg(Color::LightCyan)),
        Span::styled(" Usable Lane  ", Theme::dim_border_style()),
        Span::styled("►", Style::default().fg(Color::Magenta)),
        Span::styled(" Moving", Theme::dim_border_style()),
    ];

    let legend = Paragraph::new(Line::from(spans)).style(Theme::muted_style());
    frame.render_widget(legend, area);
}

fn render_system_list(frame: &mut Frame, area: Rect, game_state: &GameState, app_state: &AppState) {
    let border_style = Theme::dim_border_style();

    let block = Block::default()
        .title(" Systems ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sector_id = match app_state.selected_sector {
        Some(id) => id,
        None => {
            let no_selection = Paragraph::new("No sector selected").style(Theme::muted_style());
            frame.render_widget(no_selection, inner);
            return;
        }
    };

    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector_id)
        .collect();

    if stars_in_sector.is_empty() {
        frame.render_widget(
            Paragraph::new("No systems in this sector").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Sector Systems:",
        Theme::title_style(),
    )]));
    lines.push(Line::from(""));

    for star in &stars_in_sector {
        let is_selected = app_state.selected_star == Some(star.id);
        let is_explored = game_state.explored_stars.contains(&star.id);

        let prefix = if is_selected { "▶" } else { " " };

        let name: Cow<'_, str> = if is_explored {
            Cow::Borrowed(star.name.as_str())
        } else {
            Cow::Owned("???".to_string())
        };

        let style = if is_selected {
            Theme::highlight_style()
        } else if is_explored {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };

        let spectral_char = if is_explored {
            format!("{}", star.spectral_class.as_char())
        } else {
            "?".to_string()
        };

        lines.push(Line::from(vec![
            Span::raw(format!("{} ", prefix)),
            Span::styled(spectral_char, style),
            Span::raw(" "),
            Span::styled(name, style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn render_known_lanes_in_sector(
    frame: &mut Frame,
    game_state: &GameState,
    app_state: &AppState,
    sector_id: SectorId,
    bounds: &MapBounds,
) {
    let player_has_cartography = game_state
        .empires
        .get(&game_state.player_empire)
        .is_some_and(|e| {
            e.research
                .completed
                .contains(&TechId::HYPERSPACE_CARTOGRAPHY)
        });

    let highlighted_lane = app_state.selected_star.and_then(|destination| {
        let origin = game_state
            .fleets
            .values()
            .find(|fleet| {
                fleet.owner == game_state.player_empire
                    && !game_state.scout_missions.contains_key(&fleet.id)
                    && !game_state.survey_missions.contains_key(&fleet.id)
                    && !game_state.fleet_missions.contains_key(&fleet.id)
            })
            .map(|fleet| fleet.location)?;
        if origin == destination {
            return None;
        }
        game_core::HyperspaceLane::new(origin, destination)
            .filter(|lane| game_state.known_hyperspace_lanes.contains(lane))
            .filter(|_| player_has_cartography)
    });

    for lane in &game_state.known_hyperspace_lanes {
        let Some(a) = game_state.stars.get(&lane.a()) else {
            continue;
        };
        let Some(b) = game_state.stars.get(&lane.b()) else {
            continue;
        };
        if a.sector != sector_id || b.sector != sector_id {
            continue;
        }

        let is_highlight = highlighted_lane == Some(*lane);
        let style = if is_highlight {
            Style::default().fg(Color::LightCyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let glyph = if is_highlight { '•' } else { '·' };
        draw_world_line(
            frame,
            (a.x as f64, a.y as f64),
            (b.x as f64, b.y as f64),
            bounds,
            glyph,
            style,
        );
    }
}

fn draw_world_line(
    frame: &mut Frame,
    start: (f64, f64),
    end: (f64, f64),
    bounds: &MapBounds,
    glyph: char,
    style: Style,
) {
    let (x0, y0) = start;
    let (x1, y1) = end;
    // One sample roughly every 30 world units yields visually continuous but subtle
    // lane lines at typical terminal sizes without overdraw.
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()) / 30.0)
        .ceil()
        .max(1.0) as i32;
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let wx = x0 + (x1 - x0) * t;
        let wy = y0 + (y1 - y0) * t;
        if let Some((x, y)) = world_to_screen_f(wx, wy, bounds) {
            frame.render_widget(
                Paragraph::new(glyph.to_string()).style(style),
                Rect::new(x, y, 1, 1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_app_with_sector() -> (AppState, GameState) {
        let engine = Engine::new(42);
        let first_sector = engine.state.sectors.keys().next().copied();
        let first_star_in_sector = engine
            .state
            .stars
            .values()
            .find(|s| Some(s.sector) == first_sector)
            .map(|s| s.id);

        let app_state = AppState {
            selected_sector: first_sector,
            selected_star: first_star_in_sector,
            ..Default::default()
        };

        (app_state, engine.state)
    }

    #[test]
    fn sector_map_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let (app_state, game_state) = create_app_with_sector();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &game_state);
            })
            .unwrap();
    }

    #[test]
    fn sector_map_small_terminal_does_not_panic() {
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let (app_state, game_state) = create_app_with_sector();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &game_state);
            })
            .unwrap();
    }

    #[test]
    fn sector_map_no_selection_renders() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState {
            selected_sector: None,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn animation_rendering_does_not_mutate_game_state() {
        // Cosmetic animation must never change game state.
        let (mut app_state, game_state) = create_app_with_sector();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        // Capture state before rendering at different tick counts
        let state_before = game_state.clone();

        for tick in 0u64..15 {
            app_state.tick_count = tick;
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_sector_map(frame, area, &app_state, &game_state);
                })
                .unwrap();
        }

        // Game state must be completely unchanged
        assert_eq!(
            game_state, state_before,
            "Rendering with animation ticks must not change game state"
        );
    }

    #[test]
    fn reduced_motion_suppresses_animation() {
        // When reduced_motion = true the render must still succeed without panic.
        let (mut app_state, game_state) = create_app_with_sector();
        app_state.reduced_motion = true;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &game_state);
            })
            .unwrap();
    }
}
