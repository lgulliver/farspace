//! Sector overview screen - shows all sectors in the galaxy

use std::{borrow::Cow, collections::BTreeMap};

use crate::components::{derive_header_data, render_footer, render_header, render_log};
use crate::faction::{empire_visual, sector_dominant_owner, sector_fog_state, FogState};
use crate::layout::{compose_layout, split_horizontal};
use crate::map_render::{
    push_halo, visual_hash, CellCommand, HaloSpec, LabelCommand, LabelPlacement, LayeredMap,
    MapLayer,
};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::viewport::{MapViewport, ScreenPoint, ViewportBounds, WorldPoint};
use crate::{
    renderer::{
        sprite::DetailLevel,
        starfield::{
            detail_star_glyph, should_render_star, star_magnitude_color, starfield_detail,
        },
    },
    AppState,
};
use game_core::{GameState, SectorId, StarId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// Distinct salt keeps galaxy-view starfield noise stable but separate from sector-view noise.
const GALAXY_STARFIELD_SALT: u64 = 0xA11;
const GALAXY_STARFIELD_TWINKLE_SALT_XOR: u64 = 0x73;
const SELECTION_PULSE_PERIOD: u64 = 3;

pub fn render_sector_overview(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let (map_area, right_area) = split_horizontal(main_area, 55);

    render_sector_map(frame, map_area, game_state, app_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_area);

    render_sector_details(
        frame,
        right_chunks[0],
        game_state,
        app_state.navigation.selected_sector,
    );
    render_log(frame, right_chunks[1], &app_state.log);

    let hint = app_state.status_message.as_deref().unwrap_or(
        "Enter zooms to sector detail. Strategic map shows borders, capitals, fleets, and fog.",
    );
    render_footer(frame, footer_area, &Screen::SectorOverview, Some(hint));
}

fn render_sector_map(frame: &mut Frame, area: Rect, game_state: &GameState, app_state: &AppState) {
    let block = Block::default()
        .title(" Galaxy — Sector Overview ")
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let map_height = inner.height.saturating_sub(1);
    let map_area = Rect::new(inner.x, inner.y, inner.width, map_height);
    if map_area.width == 0 || map_area.height == 0 {
        return;
    }

    let viewport = MapViewport::fit_bounds(
        ViewportBounds::from_min_max(-560.0, -560.0, 560.0, 560.0),
        map_area.width,
        map_area.height,
    );
    let frame_group = if app_state.reduced_motion {
        0
    } else {
        app_state.tick_count / 6
    };

    let sector_star_counts = sector_star_counts(game_state);
    let sector_fleet_counts = sector_fleet_counts(game_state);
    let capital_sectors = capital_sectors(game_state);

    let mut cells = background_cells(game_state, map_area, frame_group, GALAXY_STARFIELD_SALT);
    let mut labels = Vec::new();

    for sector in game_state.sectors.values() {
        let Some(ScreenPoint { x: sx, y: sy }) =
            viewport.world_to_screen_cell(WorldPoint::new(sector.x as f64, sector.y as f64))
        else {
            continue;
        };
        let fog = sector_fog_state(game_state, sector.id);
        let owner = if matches!(fog, FogState::Unexplored) {
            None
        } else {
            sector_dominant_owner(game_state, sector.id)
        };

        if let Some(owner) = owner {
            let visual = empire_visual(game_state, owner);
            push_halo(
                &mut cells,
                (sx, sy),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 5,
                    radius_y: 2,
                    style: Style::default().bg(visual.territory),
                    layer: MapLayer::Territory,
                    order: 0,
                },
            );
        }

        match fog {
            FogState::Unexplored => push_halo(
                &mut cells,
                (sx, sy),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 4,
                    radius_y: 2,
                    style: Theme::fog_style(FogState::Unexplored),
                    layer: MapLayer::Fog,
                    order: 0,
                },
            ),
            FogState::Explored => push_halo(
                &mut cells,
                (sx, sy),
                (map_area.width, map_area.height),
                &HaloSpec {
                    radius_x: 3,
                    radius_y: 1,
                    style: Theme::fog_style(FogState::Explored),
                    layer: MapLayer::Fog,
                    order: 0,
                },
            ),
            FogState::Visible => {}
        }
    }

    if app_state.sector_overview.show_inter_sector_lanes {
        for lane in &game_state.known_hyperspace_lanes {
            let Some(a_star) = game_state.stars.get(&lane.a()) else {
                continue;
            };
            let Some(b_star) = game_state.stars.get(&lane.b()) else {
                continue;
            };
            if a_star.sector == b_star.sector {
                continue;
            }
            let Some(a_sector) = game_state.sectors.get(&a_star.sector) else {
                continue;
            };
            let Some(b_sector) = game_state.sectors.get(&b_star.sector) else {
                continue;
            };

            let highlighted = app_state
                .navigation
                .selected_sector
                .is_some_and(|selected| selected == a_sector.id || selected == b_sector.id);
            let glyph = if highlighted { '•' } else { '·' };
            let style = if highlighted {
                Theme::border_glow_style()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            for ScreenPoint { x, y } in viewport.world_line_to_cells(
                WorldPoint::new(a_sector.x as f64, a_sector.y as f64),
                WorldPoint::new(b_sector.x as f64, b_sector.y as f64),
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

    for sector in game_state.sectors.values() {
        let Some(ScreenPoint { x, y }) =
            viewport.world_to_screen_cell(WorldPoint::new(sector.x as f64, sector.y as f64))
        else {
            continue;
        };

        let fog = sector_fog_state(game_state, sector.id);
        let owner = if matches!(fog, FogState::Unexplored) {
            None
        } else {
            sector_dominant_owner(game_state, sector.id)
        };
        let is_selected = app_state.navigation.selected_sector == Some(sector.id);
        let has_capital = capital_sectors.contains_key(&sector.id);
        let show_capital = has_capital && !matches!(fog, FogState::Unexplored);
        let fleet_count = sector_fleet_counts
            .get(&sector.id)
            .copied()
            .unwrap_or_default();
        let system_count = sector_star_counts
            .get(&sector.id)
            .copied()
            .unwrap_or_default();

        let (symbol, style, protect) = if is_selected {
            let pulse_bright = !app_state.reduced_motion
                && (app_state.tick_count / SELECTION_PULSE_PERIOD).is_multiple_of(2);
            let style = if pulse_bright {
                Style::default().fg(Theme::accent2()).bg(Theme::accent())
            } else {
                Theme::highlight_style()
            };
            ('@', style, 10)
        } else if let Some(owner) = owner {
            let visual = empire_visual(game_state, owner);
            let mut style = Style::default().fg(visual.color);
            if show_capital {
                style = style.add_modifier(Modifier::BOLD);
            }
            (visual.symbol, style, 8)
        } else {
            match fog {
                FogState::Visible => ('◉', Style::default().fg(Color::White), 6),
                FogState::Explored => ('◌', Style::default().fg(Color::Gray), 5),
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

        if fleet_count > 0 {
            let fleet_style = owner
                .map(|empire| empire_visual(game_state, empire).color)
                .map(|color| Style::default().fg(color).add_modifier(Modifier::BOLD))
                .unwrap_or_else(Theme::accent_style);
            let marker_x = x.saturating_add(1).min(map_area.width.saturating_sub(1));
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 1,
                x: marker_x,
                y,
                symbol: Some(if fleet_count > 1 { '+' } else { '›' }),
                style: fleet_style,
                protect: 3,
            });
        }

        if show_capital && !is_selected {
            let marker_y = y.saturating_sub(1);
            cells.push(CellCommand {
                layer: MapLayer::Overlay,
                order: 0,
                x,
                y: marker_y,
                symbol: Some('^'),
                style: Style::default().fg(Color::LightYellow),
                protect: 2,
            });
        }

        if is_selected || owner.is_some() || show_capital || matches!(fog, FogState::Visible) {
            let title = if matches!(fog, FogState::Unexplored) {
                format!("Unknown · {}", system_count)
            } else {
                format!("{} · {}", sector.name, system_count)
            };
            labels.push(LabelCommand {
                text: title,
                anchor: (x, y),
                style: if is_selected {
                    Theme::highlight_style()
                } else if let Some(owner) = owner {
                    Style::default().fg(empire_visual(game_state, owner).color)
                } else {
                    Theme::muted_style()
                },
                priority: if is_selected {
                    10
                } else if show_capital {
                    8
                } else {
                    6
                },
                placements: vec![
                    LabelPlacement::Right,
                    LabelPlacement::Below,
                    LabelPlacement::Left,
                ],
            });
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
        render_map_legend(
            frame,
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
}

fn render_map_legend(frame: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("@", Theme::highlight_style()),
        Span::styled(" Selected  ", Theme::dim_border_style()),
        Span::styled("^", Style::default().fg(Color::LightYellow)),
        Span::styled(" Capital  ", Theme::dim_border_style()),
        Span::styled("›", Theme::accent_style()),
        Span::styled(" Fleets  ", Theme::dim_border_style()),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::styled(" Route  ", Theme::dim_border_style()),
        Span::styled("?", Theme::muted_style()),
        Span::styled(" Unexplored", Theme::dim_border_style()),
    ];

    let legend = Paragraph::new(Line::from(spans)).style(Theme::muted_style());
    frame.render_widget(legend, area);
}

fn render_sector_details(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_sector: Option<SectorId>,
) {
    let border_style = Theme::dim_border_style();

    let block = Block::default()
        .title(" Sector Details ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sector = match selected_sector.and_then(|id| game_state.sectors.get(&id)) {
        Some(s) => s,
        None => {
            let no_selection = Paragraph::new("No sector selected").style(Theme::muted_style());
            frame.render_widget(no_selection, inner);
            return;
        }
    };

    let owner = sector_dominant_owner(game_state, sector.id);
    let fog = sector_fog_state(game_state, sector.id);

    let mut lines = vec![
        Line::from(vec![Span::styled(&sector.name, Theme::title_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Visibility: ", Theme::muted_style()),
            Span::raw(match fog {
                FogState::Unexplored => "Unexplored",
                FogState::Explored => "Explored",
                FogState::Visible => "Visible",
            }),
        ]),
        Line::from(vec![
            Span::styled("Systems: ", Theme::muted_style()),
            Span::raw(count_systems_in_sector(game_state, sector.id).to_string()),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Theme::muted_style()),
            Span::raw(format!("({}, {})", sector.x, sector.y)),
        ]),
    ];

    if !matches!(fog, FogState::Unexplored) {
        if let Some(owner) = owner {
            let visual = empire_visual(game_state, owner);
            let owner_name = game_state
                .empires
                .get(&owner)
                .map(|empire| empire.name.as_str())
                .unwrap_or("Unknown Empire");
            lines.push(Line::from(vec![
                Span::styled("Dominant Power: ", Theme::muted_style()),
                Span::styled(
                    format!("{} {}", visual.symbol, owner_name),
                    Style::default().fg(visual.color),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Systems in Sector:",
        Theme::title_style(),
    )));

    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector.id)
        .collect();

    for star in stars_in_sector {
        let is_explored = game_state.explored_stars.contains(&star.id);
        let name: Cow<'_, str> = if is_explored {
            Cow::Borrowed(star.name.as_str())
        } else {
            Cow::Owned("???".to_string())
        };
        let mut style = if is_explored {
            Style::default().fg(Theme::star_color(star.spectral_class))
        } else {
            Theme::muted_style()
        };
        let owner_symbol = if is_explored {
            crate::faction::star_owner(game_state, star.id).map(|empire_id| {
                let visual = empire_visual(game_state, empire_id);
                style = style.fg(visual.color).add_modifier(Modifier::BOLD);
                visual.symbol
            })
        } else {
            None
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                owner_symbol
                    .unwrap_or(star.spectral_class.as_char())
                    .to_string(),
                style,
            ),
            Span::raw(" "),
            Span::styled(
                name,
                if is_explored {
                    Theme::default_style()
                } else {
                    Theme::muted_style()
                },
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn count_systems_in_sector(game_state: &GameState, sector_id: SectorId) -> usize {
    game_state
        .stars
        .values()
        .filter(|s| s.sector == sector_id)
        .count()
}

fn sector_star_counts(game_state: &GameState) -> BTreeMap<SectorId, usize> {
    let mut counts = BTreeMap::<SectorId, usize>::new();
    for star in game_state.stars.values() {
        *counts.entry(star.sector).or_default() += 1;
    }
    counts
}

fn sector_fleet_counts(game_state: &GameState) -> BTreeMap<SectorId, usize> {
    let mut counts = BTreeMap::<SectorId, usize>::new();
    for fleet in game_state.fleets.values() {
        if let Some(star) = game_state.stars.get(&fleet.location) {
            *counts.entry(star.sector).or_default() += 1;
        }
    }
    counts
}

fn capital_sectors(game_state: &GameState) -> BTreeMap<SectorId, StarId> {
    let mut capitals = BTreeMap::new();
    for empire in game_state.empires.values() {
        if let Some(star) = game_state.stars.get(&empire.home_star) {
            capitals.insert(star.sector, empire.home_star);
        }
    }
    capitals
}

fn background_cells(
    game_state: &GameState,
    area: Rect,
    frame_group: u64,
    salt: u64,
) -> Vec<CellCommand> {
    let detail = starfield_detail(area);
    let mut cells = Vec::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let static_hash = visual_hash(game_state.seed, x, y, 0, salt);
            let style = Style::default().bg(Theme::space_bg());
            if should_render_star(static_hash, detail) {
                let twinkle_hash = visual_hash(
                    game_state.seed,
                    x,
                    y,
                    frame_group,
                    salt ^ GALAXY_STARFIELD_TWINKLE_SALT_XOR,
                );
                cells.push(CellCommand {
                    layer: MapLayer::Background,
                    order: 0,
                    x,
                    y,
                    symbol: Some(detail_star_glyph(static_hash, detail)),
                    style: style.fg(star_magnitude_color(static_hash, twinkle_hash)),
                    protect: 0,
                });
            } else if matches!(detail, DetailLevel::Cinematic | DetailLevel::Standard)
                && static_hash.is_multiple_of(241)
            {
                cells.push(CellCommand {
                    layer: MapLayer::Background,
                    order: 1,
                    x,
                    y,
                    symbol: Some('✶'),
                    style: style.fg(Color::Rgb(168, 188, 236)),
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
    use crate::faction::{empire_visual, sector_dominant_owner};
    use game_core::Engine;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

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
                render_sector_overview(frame, area, app_state, game_state);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_sector_details_to_buffer(
        game_state: &GameState,
        selected_sector: Option<SectorId>,
        width: u16,
        height: u16,
    ) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_sector_details(frame, frame.area(), game_state, selected_sector);
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

    fn overview_viewport(width: u16, height: u16) -> (Rect, MapViewport) {
        let area = Rect::new(0, 0, width, height);
        let (_, main, _) = compose_layout(area);
        let (map_area, _) = split_horizontal(main, 55);
        let block_inner = Block::default().borders(Borders::ALL).inner(map_area);
        let render_area = Rect::new(
            block_inner.x,
            block_inner.y,
            block_inner.width,
            block_inner.height.saturating_sub(1),
        );
        (
            render_area,
            MapViewport::fit_bounds(
                ViewportBounds::from_min_max(-560.0, -560.0, 560.0, 560.0),
                render_area.width,
                render_area.height,
            ),
        )
    }

    #[test]
    fn sector_overview_renders_without_panic() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        render_to_buffer(&app_state, &engine.state, 120, 40);
    }

    #[test]
    fn sector_overview_with_selection() {
        let engine = Engine::new(42);
        let first_sector = engine.state.sectors.keys().next().copied();
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_sector: first_sector,
                ..Default::default()
            },
            ..Default::default()
        };

        render_to_buffer(&app_state, &engine.state, 120, 40);
    }

    #[test]
    fn sector_overview_small_terminal_does_not_panic() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        render_to_buffer(&app_state, &engine.state, 40, 15);
    }

    #[test]
    fn sector_overview_is_deterministic_for_same_tick() {
        let engine = Engine::new(42);
        let app_state = AppState {
            tick_count: 12,
            sector_overview: crate::app::SectorOverviewState {
                show_inter_sector_lanes: true,
            },
            ..Default::default()
        };
        let a = render_to_buffer(&app_state, &engine.state, 120, 40);
        let b = render_to_buffer(&app_state, &engine.state, 120, 40);
        assert_eq!(a, b);
    }

    #[test]
    fn overview_background_starfield_stays_fixed_while_stars_twinkle() {
        let engine = Engine::new(42);
        let a = background_cells(
            &engine.state,
            Rect::new(0, 0, 80, 24),
            0,
            GALAXY_STARFIELD_SALT,
        );
        let b = background_cells(
            &engine.state,
            Rect::new(0, 0, 80, 24),
            6,
            GALAXY_STARFIELD_SALT,
        );

        let a_layout: Vec<_> = a.iter().map(|cell| (cell.x, cell.y, cell.symbol)).collect();
        let b_layout: Vec<_> = b.iter().map(|cell| (cell.x, cell.y, cell.symbol)).collect();

        assert_eq!(a_layout, b_layout, "background layout should stay fixed");
        assert!(
            a.iter()
                .zip(&b)
                .any(|(left, right)| left.style != right.style),
            "some background stars should still twinkle"
        );
    }

    #[test]
    fn sector_overview_shows_faction_identity_and_fog() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buf = render_to_buffer(&app_state, &engine.state, 120, 40);
        let (render_area, viewport) = overview_viewport(120, 40);

        let owned_sector = engine
            .state
            .sectors
            .values()
            .find(|sector| {
                sector_dominant_owner(&engine.state, sector.id).is_some()
                    && matches!(
                        sector_fog_state(&engine.state, sector.id),
                        FogState::Visible
                    )
            })
            .unwrap();
        let unexplored_sector = engine
            .state
            .sectors
            .values()
            .find(|sector| {
                matches!(
                    sector_fog_state(&engine.state, sector.id),
                    FogState::Unexplored
                ) && sector_dominant_owner(&engine.state, sector.id).is_none()
            })
            .unwrap();

        let owned_pos = viewport
            .world_to_screen_cell(WorldPoint::new(
                owned_sector.x as f64,
                owned_sector.y as f64,
            ))
            .unwrap();
        let unexplored_pos = viewport
            .world_to_screen_cell(WorldPoint::new(
                unexplored_sector.x as f64,
                unexplored_sector.y as f64,
            ))
            .unwrap();

        let owned_cell = buf
            .cell((render_area.x + owned_pos.x, render_area.y + owned_pos.y))
            .unwrap();
        let unexplored_cell = buf
            .cell((
                render_area.x + unexplored_pos.x,
                render_area.y + unexplored_pos.y,
            ))
            .unwrap();

        let owner = sector_dominant_owner(&engine.state, owned_sector.id).unwrap();
        let visual = empire_visual(&engine.state, owner);
        assert_eq!(owned_cell.symbol(), visual.symbol.to_string());
        assert_eq!(owned_cell.fg, visual.color);
        assert_eq!(unexplored_cell.symbol(), "?");
    }

    #[test]
    fn sector_overview_labels_do_not_overwrite_selected_marker() {
        let engine = Engine::new(42);
        let selected_sector = engine.state.sectors.keys().next().copied().unwrap();
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_sector: Some(selected_sector),
                ..Default::default()
            },
            ..Default::default()
        };
        let buf = render_to_buffer(&app_state, &engine.state, 120, 40);
        let (render_area, viewport) = overview_viewport(120, 40);
        let sector = engine.state.sectors.get(&selected_sector).unwrap();
        let pos = viewport
            .world_to_screen_cell(WorldPoint::new(sector.x as f64, sector.y as f64))
            .unwrap();
        let cell = buf
            .cell((render_area.x + pos.x, render_area.y + pos.y))
            .unwrap();
        assert_eq!(cell.symbol(), "@");
    }

    #[test]
    fn unexplored_owned_sector_does_not_show_territory_halo() {
        let engine = Engine::new(42);
        let mut game_state = engine.state.clone();
        let target_sector = game_state
            .sectors
            .values()
            .find(|sector| sector_dominant_owner(&game_state, sector.id).is_some())
            .map(|sector| sector.id)
            .unwrap();
        let sector_star_ids: Vec<_> = game_state
            .stars
            .values()
            .filter(|star| star.sector == target_sector)
            .map(|star| star.id)
            .collect();
        for star_id in &sector_star_ids {
            game_state.explored_stars.remove(star_id);
        }
        assert_eq!(
            sector_fog_state(&game_state, target_sector),
            FogState::Unexplored
        );

        let colony_ids: Vec<_> = game_state
            .colonies
            .iter()
            .filter_map(|(colony_id, colony)| {
                game_state
                    .stars
                    .get(&colony.star)
                    .filter(|star| star.sector == target_sector)
                    .map(|_| *colony_id)
            })
            .collect();
        let mut without_colonies = game_state.clone();
        for colony_id in colony_ids {
            without_colonies.colonies.remove(&colony_id);
        }

        let app_state = AppState::default();
        let with_buffer = render_to_buffer(&app_state, &game_state, 120, 40);
        let without_buffer = render_to_buffer(&app_state, &without_colonies, 120, 40);
        let (render_area, viewport) = overview_viewport(120, 40);
        let sector = game_state.sectors.get(&target_sector).unwrap();
        let pos = viewport
            .world_to_screen_cell(WorldPoint::new(sector.x as f64, sector.y as f64))
            .unwrap();
        let halo_x = if pos.x + 5 < render_area.width {
            pos.x + 5
        } else {
            pos.x.saturating_sub(5)
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

    #[test]
    fn unexplored_owned_sector_details_hide_owner_information() {
        let engine = Engine::new(42);
        let mut game_state = engine.state.clone();
        let target_sector = game_state
            .sectors
            .values()
            .find(|sector| sector_dominant_owner(&game_state, sector.id).is_some())
            .map(|sector| sector.id)
            .unwrap();
        for star_id in game_state
            .stars
            .values()
            .filter(|star| star.sector == target_sector)
            .map(|star| star.id)
            .collect::<Vec<_>>()
        {
            game_state.explored_stars.remove(&star_id);
        }

        let owner = sector_dominant_owner(&game_state, target_sector).unwrap();
        let visual = empire_visual(&game_state, owner);
        let buffer = render_sector_details_to_buffer(&game_state, Some(target_sector), 48, 18);
        let text = buffer_text(&buffer, Rect::new(0, 0, 48, 18));

        assert_eq!(
            sector_fog_state(&game_state, target_sector),
            FogState::Unexplored
        );
        assert!(!text.contains("Dominant Power"));
        assert!(!text.contains(&visual.symbol.to_string()));
    }
}
