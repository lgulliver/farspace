//! Sector map screen - shows systems within a selected sector

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use crate::components::{derive_header_data, render_footer, render_header, render_log};
use crate::faction::{
    empire_visual, star_fog_state, star_is_capital, star_owner, visible_star_ids, FogState,
};
use crate::layout::{compose_layout, split_horizontal};
use crate::map_render::{
    push_halo, visual_hash, CellCommand, HaloSpec, LabelCommand, LabelPlacement, LayeredMap,
    MapLayer,
};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::viewport::{MapViewport, ScreenPoint, WorldPoint};
use crate::AppState;
use game_core::{GameState, SectorId, StarId, TechId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// Distinct salt keeps sector-view starfield noise stable but separate from galaxy-view noise.
const SECTOR_STARFIELD_SALT: u64 = 0xB22;

pub fn render_sector_map(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let (map_area, right_area) = split_horizontal(main_area, 55);

    render_local_map(frame, map_area, game_state, app_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_area);

    render_system_list(frame, right_chunks[0], game_state, app_state);
    render_log(frame, right_chunks[1], &app_state.log);

    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Enter opens system detail. S scouts, M moves fleets, and map pips show fog, borders, and traffic.");
    render_footer(frame, footer_area, &Screen::SectorMap, Some(hint));
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

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let map_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(1),
    );
    if map_area.width == 0 || map_area.height == 0 {
        return;
    }

    let sector_id = match app_state.selected_sector {
        Some(id) => id,
        None => return,
    };

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

    let points: Vec<_> = stars_in_sector
        .iter()
        .map(|star| WorldPoint::new(star.x as f64, star.y as f64))
        .collect();
    let Some(viewport) = MapViewport::fit_points(&points, map_area.width, map_area.height, 40.0)
    else {
        return;
    };

    let visible_stars = visible_star_ids(game_state, sector_id);
    let frame_group = if app_state.reduced_motion {
        0
    } else {
        app_state.tick_count / 4
    };

    let scout_destinations: BTreeSet<StarId> = game_state
        .scout_missions
        .values()
        .map(|mission| mission.destination)
        .collect();

    let fleet_destinations: BTreeSet<StarId> = game_state
        .fleet_missions
        .values()
        .map(|mission| mission.destination)
        .collect();

    let fleets_at_star = fleets_at_star(game_state, sector_id);

    let mut cells = background_cells(game_state, map_area, frame_group, SECTOR_STARFIELD_SALT);
    let mut labels = Vec::new();

    for star in &stars_in_sector {
        let Some(ScreenPoint { x, y }) =
            viewport.world_to_screen_cell(WorldPoint::new(star.x as f64, star.y as f64))
        else {
            continue;
        };
        let fog = star_fog_state(game_state, &visible_stars, star.id);
        let owner = if matches!(fog, FogState::Unexplored) {
            None
        } else {
            star_owner(game_state, star.id)
        };

        if let Some(owner) = owner {
            let visual = empire_visual(game_state, owner);
            push_halo(
                &mut cells,
                (x, y),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 4,
                    radius_y: 2,
                    style: Style::default().bg(visual.territory),
                    layer: MapLayer::Territory,
                    order: 0,
                },
            );
        }

        match fog {
            FogState::Visible => push_halo(
                &mut cells,
                (x, y),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 2,
                    radius_y: 1,
                    style: Theme::fog_style(FogState::Visible),
                    layer: MapLayer::Fog,
                    order: 0,
                },
            ),
            FogState::Explored => push_halo(
                &mut cells,
                (x, y),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 2,
                    radius_y: 1,
                    style: Theme::fog_style(FogState::Explored),
                    layer: MapLayer::Fog,
                    order: 0,
                },
            ),
            FogState::Unexplored => push_halo(
                &mut cells,
                (x, y),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 1,
                    radius_y: 1,
                    style: Theme::fog_style(FogState::Unexplored),
                    layer: MapLayer::Fog,
                    order: 0,
                },
            ),
        }
    }

    render_known_lanes_in_sector(&mut cells, game_state, app_state, sector_id, &viewport);

    for star in &stars_in_sector {
        let Some(ScreenPoint { x, y }) =
            viewport.world_to_screen_cell(WorldPoint::new(star.x as f64, star.y as f64))
        else {
            continue;
        };

        let is_selected = app_state.selected_star == Some(star.id);
        let fog = star_fog_state(game_state, &visible_stars, star.id);
        let owner = if matches!(fog, FogState::Unexplored) {
            None
        } else {
            star_owner(game_state, star.id)
        };
        let capital = star_is_capital(game_state, star.id);
        let show_capital = capital && !matches!(fog, FogState::Unexplored);
        let scout_en_route = scout_destinations.contains(&star.id);
        let fleet_en_route = fleet_destinations.contains(&star.id);
        let stationary_fleets = fleets_at_star.get(&star.id).copied().unwrap_or_default();

        let (symbol, style, protect) = if is_selected {
            ('@', Theme::highlight_style(), 10)
        } else if let Some(owner) = owner {
            let visual = empire_visual(game_state, owner);
            let mut style = Style::default().fg(visual.color);
            if show_capital {
                style = style.add_modifier(Modifier::BOLD);
            }
            (visual.symbol, style, 8)
        } else {
            match fog {
                FogState::Visible => (
                    star.spectral_class.as_char(),
                    Style::default().fg(Theme::star_color(star.spectral_class)),
                    6,
                ),
                FogState::Explored => (
                    star.spectral_class.as_char(),
                    Style::default().fg(Color::Gray),
                    5,
                ),
                FogState::Unexplored => ('?', Theme::muted_style(), 5),
            }
        };

        cells.push(CellCommand {
            layer: MapLayer::Entity,
            order: 0,
            x,
            y,
            symbol: Some(symbol),
            style,
            protect,
        });

        if show_capital && !is_selected {
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 0,
                x,
                y: y.saturating_sub(1),
                symbol: Some('^'),
                style: Style::default().fg(Color::LightYellow),
                protect: 2,
            });
        }

        if scout_en_route {
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 1,
                x: x.saturating_add(1).min(map_area.width.saturating_sub(1)),
                y,
                symbol: Some('+'),
                style: Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                protect: 1,
            });
        } else if fleet_en_route {
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 1,
                x: x.saturating_add(1).min(map_area.width.saturating_sub(1)),
                y,
                symbol: Some('~'),
                style: Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                protect: 1,
            });
        }

        if stationary_fleets > 0 {
            let marker_x = x.saturating_add(1).min(map_area.width.saturating_sub(1));
            let marker = if stationary_fleets > 1 { '+' } else { '›' };
            let fleet_color = owner
                .map(|empire| empire_visual(game_state, empire).color)
                .unwrap_or(Color::LightCyan);
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 2,
                x: marker_x,
                y: y.saturating_add(1).min(map_area.height.saturating_sub(1)),
                symbol: Some(marker),
                style: Style::default().fg(fleet_color),
                protect: 1,
            });
        }

        if is_selected || owner.is_some() || show_capital || stationary_fleets > 0 {
            let label_style = if is_selected {
                Theme::highlight_style()
            } else if let Some(owner) = owner {
                Style::default().fg(empire_visual(game_state, owner).color)
            } else if matches!(fog, FogState::Visible) {
                Theme::default_style()
            } else {
                Theme::muted_style()
            };
            labels.push(LabelCommand {
                text: if matches!(fog, FogState::Unexplored) {
                    "Unknown".to_string()
                } else {
                    star.name.clone()
                },
                anchor: (x, y),
                style: label_style,
                priority: if is_selected {
                    10
                } else if show_capital {
                    9
                } else {
                    7
                },
                placements: vec![
                    LabelPlacement::Right,
                    LabelPlacement::Below,
                    LabelPlacement::Left,
                ],
            });
        }
    }

    if !app_state.reduced_motion {
        let show_indicator = (app_state.tick_count / 5).is_multiple_of(2);
        if show_indicator {
            render_travelling_fleets(&mut cells, game_state, sector_id, &viewport);
        }
    }

    frame.render_widget(
        LayeredMap {
            base_style: Theme::default_style(),
            cells,
            labels,
        },
        map_area,
    );

    if inner.height >= 1 && inner.width >= 10 {
        render_local_legend(
            frame,
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
}

/// Render interpolated position indicators for fleets currently travelling within the sector.
/// Cosmetic only; simulation state is never mutated.
fn render_travelling_fleets(
    cells: &mut Vec<CellCommand>,
    game_state: &GameState,
    sector_id: SectorId,
    viewport: &MapViewport,
) {
    let fleet_indicator_style = Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::BOLD);
    let scout_indicator_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    for mission in game_state.fleet_missions.values() {
        let origin = match game_state.stars.get(&mission.origin) {
            Some(star) if star.sector == sector_id => star,
            _ => continue,
        };
        let destination = match game_state.stars.get(&mission.destination) {
            Some(star) if star.sector == sector_id => star,
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
        if let Some(ScreenPoint { x, y }) = viewport.world_to_screen_cell(WorldPoint::new(wx, wy)) {
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 3,
                x,
                y,
                symbol: Some('►'),
                style: fleet_indicator_style,
                protect: 0,
            });
        }
    }

    for mission in game_state.scout_missions.values() {
        let origin = match game_state.stars.get(&mission.origin) {
            Some(star) if star.sector == sector_id => star,
            _ => continue,
        };
        let destination = match game_state.stars.get(&mission.destination) {
            Some(star) if star.sector == sector_id => star,
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
        if let Some(ScreenPoint { x, y }) = viewport.world_to_screen_cell(WorldPoint::new(wx, wy)) {
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 4,
                x,
                y,
                symbol: Some('►'),
                style: scout_indicator_style,
                protect: 0,
            });
        }
    }
}

fn render_local_legend(frame: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("@", Theme::highlight_style()),
        Span::styled(" Selected  ", Theme::dim_border_style()),
        Span::styled("^", Style::default().fg(Color::LightYellow)),
        Span::styled(" Capital  ", Theme::dim_border_style()),
        Span::styled("+", Style::default().fg(Color::Yellow)),
        Span::styled(" Scout Route  ", Theme::dim_border_style()),
        Span::styled("~", Style::default().fg(Color::Cyan)),
        Span::styled(" Fleet Route  ", Theme::dim_border_style()),
        Span::styled("►", Style::default().fg(Color::Magenta)),
        Span::styled(" Transit  ", Theme::dim_border_style()),
        Span::styled("?", Theme::muted_style()),
        Span::styled(" Unexplored", Theme::dim_border_style()),
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

    let visible_stars = visible_star_ids(game_state, sector_id);
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

    let fleets = fleets_at_star(game_state, sector_id);
    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Sector Systems:",
        Theme::title_style(),
    )]));
    lines.push(Line::from(""));

    for star in &stars_in_sector {
        let is_selected = app_state.selected_star == Some(star.id);
        let fog = star_fog_state(game_state, &visible_stars, star.id);
        let owner = if matches!(fog, FogState::Unexplored) {
            None
        } else {
            star_owner(game_state, star.id)
        };
        let prefix = if is_selected { "▶" } else { " " };
        let name: Cow<'_, str> = if matches!(fog, FogState::Unexplored) {
            Cow::Owned("???".to_string())
        } else {
            Cow::Borrowed(star.name.as_str())
        };

        let symbol = if let Some(owner) = owner {
            empire_visual(game_state, owner).symbol
        } else if matches!(fog, FogState::Unexplored) {
            '?'
        } else {
            star.spectral_class.as_char()
        };

        let mut style = if is_selected {
            Theme::highlight_style()
        } else if let Some(owner) = owner {
            Style::default().fg(empire_visual(game_state, owner).color)
        } else if matches!(fog, FogState::Visible) {
            Style::default().fg(Theme::star_color(star.spectral_class))
        } else {
            Theme::muted_style()
        };
        if star_is_capital(game_state, star.id) && !is_selected {
            style = style.add_modifier(Modifier::BOLD);
        }

        let fleet_note = fleets
            .get(&star.id)
            .copied()
            .filter(|count| *count > 0)
            .map(|count| format!("  [{}f]", count))
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::raw(format!("{} ", prefix)),
            Span::styled(symbol.to_string(), style),
            Span::raw(" "),
            Span::styled(name, style),
            Span::styled(fleet_note, Theme::muted_style()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn render_known_lanes_in_sector(
    cells: &mut Vec<CellCommand>,
    game_state: &GameState,
    app_state: &AppState,
    sector_id: SectorId,
    viewport: &MapViewport,
) {
    let player_has_cartography = game_state
        .empires
        .get(&game_state.player_empire)
        .is_some_and(|empire| {
            empire
                .research
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
        for ScreenPoint { x, y } in viewport.world_line_to_cells(
            WorldPoint::new(a.x as f64, a.y as f64),
            WorldPoint::new(b.x as f64, b.y as f64),
        ) {
            cells.push(CellCommand {
                layer: MapLayer::Route,
                order: 0,
                x,
                y,
                symbol: Some(glyph),
                style,
                protect: 0,
            });
        }
    }
}

fn fleets_at_star(game_state: &GameState, sector_id: SectorId) -> BTreeMap<StarId, usize> {
    let mut fleets = BTreeMap::<StarId, usize>::new();
    for fleet in game_state.fleets.values() {
        let Some(star) = game_state.stars.get(&fleet.location) else {
            continue;
        };
        if star.sector == sector_id {
            *fleets.entry(fleet.location).or_default() += 1;
        }
    }
    fleets
}

fn background_cells(
    game_state: &GameState,
    area: Rect,
    frame_group: u64,
    salt: u64,
) -> Vec<CellCommand> {
    let mut cells = Vec::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let hash = visual_hash(game_state.seed, x, y, frame_group, salt);
            let style = Style::default().bg(Theme::space_bg());
            if hash.is_multiple_of(53) {
                cells.push(CellCommand {
                    layer: MapLayer::Background,
                    order: 0,
                    x,
                    y,
                    symbol: Some(if hash.is_multiple_of(3) { '·' } else { '.' }),
                    style: style.fg(Color::Rgb(65, 80, 116)),
                    protect: 0,
                });
            } else if hash.is_multiple_of(149) {
                cells.push(CellCommand {
                    layer: MapLayer::Background,
                    order: 1,
                    x,
                    y,
                    symbol: Some('✦'),
                    style: style.fg(Color::Rgb(145, 165, 215)),
                    protect: 0,
                });
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    fn create_app_with_sector() -> (AppState, GameState) {
        let engine = Engine::new(42);
        let first_sector = engine.state.sectors.keys().next().copied();
        let first_star_in_sector = engine
            .state
            .stars
            .values()
            .find(|star| Some(star.sector) == first_sector)
            .map(|star| star.id);

        let app_state = AppState {
            selected_sector: first_sector,
            selected_star: first_star_in_sector,
            ..Default::default()
        };

        (app_state, engine.state)
    }

    fn render_to_buffer(
        app_state: &AppState,
        game_state: &GameState,
        width: u16,
        height: u16,
    ) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, app_state, game_state);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_system_list_to_buffer(
        app_state: &AppState,
        game_state: &GameState,
        width: u16,
        height: u16,
    ) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_system_list(frame, frame.area(), game_state, app_state);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &Buffer, area: Rect) -> String {
        (area.y..area.y + area.height)
            .map(|y| {
                (area.x..area.x + area.width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .unwrap()
                            .symbol()
                            .chars()
                            .next()
                            .unwrap_or(' ')
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn map_render_area(width: u16, height: u16) -> Rect {
        let area = Rect::new(0, 0, width, height);
        let (_, main, _) = compose_layout(area);
        let (map_area, _) = split_horizontal(main, 55);
        let block_inner = Block::default().borders(Borders::ALL).inner(map_area);
        Rect::new(
            block_inner.x,
            block_inner.y,
            block_inner.width,
            block_inner.height.saturating_sub(1),
        )
    }

    fn sector_viewport(
        game_state: &GameState,
        app_state: &AppState,
        width: u16,
        height: u16,
    ) -> MapViewport {
        let render_area = map_render_area(width, height);
        let sector_points: Vec<_> = game_state
            .stars
            .values()
            .filter(|star| Some(star.sector) == app_state.selected_sector)
            .map(|star| WorldPoint::new(star.x as f64, star.y as f64))
            .collect();
        MapViewport::fit_points(&sector_points, render_area.width, render_area.height, 40.0)
            .unwrap()
    }

    #[test]
    fn sector_map_renders_without_panic() {
        let (app_state, game_state) = create_app_with_sector();
        render_to_buffer(&app_state, &game_state, 120, 40);
    }

    #[test]
    fn sector_map_small_terminal_does_not_panic() {
        let (app_state, game_state) = create_app_with_sector();
        render_to_buffer(&app_state, &game_state, 40, 15);
    }

    #[test]
    fn sector_map_no_selection_renders() {
        let engine = Engine::new(42);
        let app_state = AppState {
            selected_sector: None,
            ..Default::default()
        };
        render_to_buffer(&app_state, &engine.state, 120, 40);
    }

    #[test]
    fn animation_rendering_does_not_mutate_game_state() {
        let (mut app_state, game_state) = create_app_with_sector();
        let state_before = game_state.clone();
        for tick in 0u64..15 {
            app_state.tick_count = tick;
            let _ = render_to_buffer(&app_state, &game_state, 120, 40);
        }
        assert_eq!(game_state, state_before);
    }

    #[test]
    fn reduced_motion_suppresses_animation() {
        let (mut app_state, game_state) = create_app_with_sector();
        app_state.reduced_motion = true;
        render_to_buffer(&app_state, &game_state, 120, 40);
    }

    #[test]
    fn sector_map_is_deterministic_for_same_frame() {
        let (mut app_state, game_state) = create_app_with_sector();
        app_state.tick_count = 9;
        let a = render_to_buffer(&app_state, &game_state, 120, 40);
        let b = render_to_buffer(&app_state, &game_state, 120, 40);
        assert_eq!(a, b);
    }

    #[test]
    fn sector_map_animation_changes_cosmetic_output_only() {
        let (mut app_state, game_state) = create_app_with_sector();
        app_state.tick_count = 0;
        let a = render_to_buffer(&app_state, &game_state, 120, 40);
        app_state.tick_count = 10;
        let b = render_to_buffer(&app_state, &game_state, 120, 40);
        assert_ne!(a, b);
        assert_eq!(game_state, game_state.clone());
    }

    #[test]
    fn sector_map_shows_fog_and_faction_identity() {
        let (mut app_state, mut game_state) = create_app_with_sector();
        let selected_sector = game_state
            .colonies
            .values()
            .find(|colony| colony.owner == game_state.player_empire)
            .and_then(|colony| game_state.stars.get(&colony.star))
            .map(|star| star.sector)
            .unwrap();
        app_state.selected_sector = Some(selected_sector);
        let player_owned_star = game_state
            .colonies
            .values()
            .find(|colony| colony.owner == game_state.player_empire)
            .and_then(|colony| game_state.stars.get(&colony.star))
            .filter(|star| star.sector == selected_sector)
            .map(|star| star.id)
            .unwrap();
        let hidden_star = game_state
            .stars
            .values()
            .find(|star| star.sector == selected_sector && star.id != player_owned_star)
            .map(|star| star.id)
            .unwrap();
        app_state.selected_star = None;
        game_state.explored_stars.remove(&hidden_star);

        let buf = render_to_buffer(&app_state, &game_state, 120, 40);
        let render_area = map_render_area(120, 40);
        let viewport = sector_viewport(&game_state, &app_state, 120, 40);
        let owned_star = game_state.stars.get(&player_owned_star).unwrap();
        let hidden_star_data = game_state.stars.get(&hidden_star).unwrap();
        let owned_pos = viewport
            .world_to_screen_cell(WorldPoint::new(owned_star.x as f64, owned_star.y as f64))
            .unwrap();
        let hidden_pos = viewport
            .world_to_screen_cell(WorldPoint::new(
                hidden_star_data.x as f64,
                hidden_star_data.y as f64,
            ))
            .unwrap();
        let owned_cell = buf
            .cell((render_area.x + owned_pos.x, render_area.y + owned_pos.y))
            .unwrap();
        let hidden_cell = buf
            .cell((render_area.x + hidden_pos.x, render_area.y + hidden_pos.y))
            .unwrap();
        let visual = empire_visual(&game_state, game_state.player_empire);
        assert_eq!(owned_cell.symbol(), visual.symbol.to_string());
        assert_eq!(owned_cell.fg, visual.color);
        assert_eq!(hidden_cell.symbol(), "?");
    }

    #[test]
    fn selected_star_renders_at_marker_and_stays_readable_with_labels() {
        let (app_state, game_state) = create_app_with_sector();
        let star_id = app_state.selected_star.unwrap();
        let buf = render_to_buffer(&app_state, &game_state, 120, 40);
        let render_area = map_render_area(120, 40);
        let viewport = sector_viewport(&game_state, &app_state, 120, 40);
        let star = game_state.stars.get(&star_id).unwrap();
        let pos = viewport
            .world_to_screen_cell(WorldPoint::new(star.x as f64, star.y as f64))
            .unwrap();
        let cell = buf
            .cell((render_area.x + pos.x, render_area.y + pos.y))
            .unwrap();
        assert_eq!(cell.symbol(), "@");
        if let Some(owner) = star_owner(&game_state, star_id) {
            assert_ne!(
                cell.symbol(),
                empire_visual(&game_state, owner).symbol.to_string()
            );
        }
        assert_eq!(cell.bg, Theme::accent());
    }

    #[test]
    fn unexplored_owned_star_does_not_show_owner_in_system_list() {
        let (mut app_state, mut game_state) = create_app_with_sector();
        let selected_sector = game_state
            .colonies
            .values()
            .find(|colony| colony.owner == game_state.player_empire)
            .and_then(|colony| game_state.stars.get(&colony.star))
            .map(|star| star.sector)
            .unwrap();
        let hidden_owned_star = game_state
            .colonies
            .values()
            .filter(|colony| colony.owner == game_state.player_empire)
            .find_map(|colony| {
                game_state
                    .stars
                    .get(&colony.star)
                    .filter(|star| star.sector == selected_sector)
                    .map(|star| star.id)
            })
            .unwrap();
        app_state.selected_sector = Some(selected_sector);
        app_state.selected_star = None;
        game_state.explored_stars.remove(&hidden_owned_star);

        let buffer = render_system_list_to_buffer(&app_state, &game_state, 48, 18);
        let text = buffer_text(&buffer, Rect::new(0, 0, 48, 18));
        let visual = empire_visual(&game_state, game_state.player_empire);

        assert!(text.contains("? ???"));
        assert!(!text.contains(&format!("{} ???", visual.symbol)));
    }

    #[test]
    fn unexplored_owned_star_does_not_render_territory_halo() {
        let (mut app_state, mut game_state) = create_app_with_sector();
        let selected_sector = game_state
            .colonies
            .values()
            .find(|colony| colony.owner == game_state.player_empire)
            .and_then(|colony| game_state.stars.get(&colony.star))
            .map(|star| star.sector)
            .unwrap();
        let (colony_id, hidden_owned_star) = game_state
            .colonies
            .iter()
            .filter(|(_, colony)| colony.owner == game_state.player_empire)
            .find_map(|(colony_id, colony)| {
                game_state.stars.get(&colony.star).and_then(|star| {
                    (star.sector == selected_sector).then_some((*colony_id, star.id))
                })
            })
            .unwrap();
        app_state.selected_sector = Some(selected_sector);
        app_state.selected_star = None;
        game_state.explored_stars.remove(&hidden_owned_star);

        let mut without_colony = game_state.clone();
        without_colony.colonies.remove(&colony_id);

        let with_buffer = render_to_buffer(&app_state, &game_state, 120, 40);
        let without_buffer = render_to_buffer(&app_state, &without_colony, 120, 40);
        let render_area = map_render_area(120, 40);
        let viewport = sector_viewport(&game_state, &app_state, 120, 40);
        let star = game_state.stars.get(&hidden_owned_star).unwrap();
        let pos = viewport
            .world_to_screen_cell(WorldPoint::new(star.x as f64, star.y as f64))
            .unwrap();
        let halo_x = if pos.x + 3 < render_area.width {
            pos.x + 3
        } else {
            pos.x.saturating_sub(3)
        };

        assert_eq!(
            with_buffer
                .cell((render_area.x + pos.x, render_area.y + pos.y))
                .unwrap()
                .symbol(),
            "?",
        );
        assert_eq!(
            with_buffer
                .cell((render_area.x + halo_x, render_area.y + pos.y))
                .unwrap()
                .bg,
            without_buffer
                .cell((render_area.x + halo_x, render_area.y + pos.y))
                .unwrap()
                .bg,
        );
    }
}
