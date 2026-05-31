//! Sector overview screen - shows all sectors in the galaxy

use std::collections::BTreeMap;

use crate::components::{
    advisor_strip_text, derive_header_data, meter_line, page_block, panel_block, render_footer,
    render_header, render_log, render_turn_brief, section_heading,
};
use crate::faction::{empire_visual, sector_dominant_owner, sector_fog_state, FogState};
use crate::glyphs::glyphs_for_mode;
use crate::layout::{compose_layout, split_horizontal};
use crate::map_render::{
    push_halo, visual_hash, CellCommand, HaloSpec, LabelCommand, LabelPlacement, LayeredMap,
    MapLayer,
};
use crate::screens::Screen;
use crate::theme::{lerp_rgb, Theme};
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
    widgets::Paragraph,
    Frame,
};

// Distinct salt keeps galaxy-view starfield noise stable but separate from sector-view noise.
const GALAXY_STARFIELD_SALT: u64 = 0xA11;
const GALAXY_STARFIELD_TWINKLE_SALT_XOR: u64 = 0x73;
const SELECTION_PULSE_PERIOD: u64 = 3;
const SELECTED_SECTOR_LABEL_PROTECT: u8 = 10;
/// Sector-level threat contribution per hostile fleet in the selected sector.
const THREAT_PER_HOSTILE_FLEET: usize = 25;

pub fn render_sector_overview(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let (map_area, right_area) = split_horizontal(main_area, 60);

    render_sector_map(frame, map_area, game_state, app_state);

    let compact_right = right_area.width < 32 || right_area.height < 14;
    if compact_right {
        render_sector_details(
            frame,
            right_area,
            game_state,
            app_state.navigation.selected_sector,
        );
    } else if right_area.height >= 20 {
        // Tall right column: details, turn brief, then event log.
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(52),
                Constraint::Length(6),
                Constraint::Min(4),
            ])
            .split(right_area);
        render_sector_details(
            frame,
            right_chunks[0],
            game_state,
            app_state.navigation.selected_sector,
        );
        render_turn_brief(
            frame,
            right_chunks[1],
            &app_state.advisor_output,
            app_state.visual_mode,
        );
        render_log(
            frame,
            right_chunks[2],
            &app_state.log,
            app_state.visual_mode,
        );
    } else {
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
            .split(right_area);
        render_sector_details(
            frame,
            right_chunks[0],
            game_state,
            app_state.navigation.selected_sector,
        );
        render_log(
            frame,
            right_chunks[1],
            &app_state.log,
            app_state.visual_mode,
        );
    }

    let advisor_hint = advisor_strip_text(&app_state.advisor_output);
    let hint = app_state
        .status_message
        .as_deref()
        .or(advisor_hint.as_deref())
        .unwrap_or(
            "Enter opens Sector Map. Move to inspect ownership, fleets, threat, and known systems.",
        );
    render_footer(frame, footer_area, &Screen::SectorOverview, Some(hint));
}

fn render_sector_map(frame: &mut Frame, area: Rect, game_state: &GameState, app_state: &AppState) {
    let palette = Theme::splash_palette();
    let glyphs = glyphs_for_mode(app_state.visual_mode);
    let block = page_block("Galaxy — Sector Overview")
        .title_style(
            Style::default()
                .fg(palette.title_primary)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(lerp_rgb(palette.void_bg, palette.nebula_a, 0.10)));

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
            (glyphs.sector_selected, style, SELECTED_SECTOR_LABEL_PROTECT)
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
                FogState::Unexplored => ('◌', Theme::muted_style(), 5),
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
            app_state.visual_mode,
        );
    }
}

fn render_map_legend(frame: &mut Frame, area: Rect, mode: crate::visual_mode::VisualMode) {
    let glyphs = glyphs_for_mode(mode);
    let spans = vec![
        Span::styled(glyphs.sector_selected.to_string(), Theme::highlight_style()),
        Span::styled(" Selected  ", Theme::dim_border_style()),
        Span::styled("^", Style::default().fg(Color::LightYellow)),
        Span::styled(" Capital  ", Theme::dim_border_style()),
        Span::styled("›", Theme::accent_style()),
        Span::styled(" Fleets  ", Theme::dim_border_style()),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::styled(" Route  ", Theme::dim_border_style()),
        Span::styled("◌", Theme::muted_style()),
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
    let block = panel_block("Sector Command Detail", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

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
    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector.id)
        .collect();
    let known_systems = stars_in_sector
        .iter()
        .filter(|star| game_state.explored_stars.contains(&star.id))
        .count();
    let colony_count = stars_in_sector
        .iter()
        .flat_map(|star| star.planets.iter())
        .filter(|planet| planet.colony.is_some())
        .count();
    let fleets_total = game_state
        .fleets
        .values()
        .filter(|fleet| {
            game_state
                .stars
                .get(&fleet.location)
                .is_some_and(|star| star.sector == sector.id)
        })
        .count();
    let hostile_fleets = if matches!(fog, FogState::Visible) {
        game_state
            .fleets
            .values()
            .filter(|fleet| {
                game_state
                    .stars
                    .get(&fleet.location)
                    .is_some_and(|star| star.sector == sector.id)
                    && fleet.owner != game_state.player_empire
                    && game_state
                        .relationship_status(game_state.player_empire, fleet.owner)
                        .is_hostile_or_war()
            })
            .count()
    } else {
        0
    };
    let threat_percent = (hostile_fleets.saturating_mul(THREAT_PER_HOSTILE_FLEET)).min(100) as u8;
    let strategic_notes = strategic_notes(fog, owner, colony_count, fleets_total, hostile_fleets);
    let owner_text = if matches!(fog, FogState::Unexplored) {
        "Unknown".to_string()
    } else {
        owner
            .and_then(|owner_id| {
                game_state.empires.get(&owner_id).map(|empire| {
                    format!(
                        "{} {}",
                        empire_visual(game_state, owner_id).symbol,
                        empire.name.as_str()
                    )
                })
            })
            .unwrap_or_else(|| "Unclaimed".to_string())
    };
    let visibility = match fog {
        FogState::Unexplored => "Unexplored",
        FogState::Explored => "Explored",
        FogState::Visible => "Visible",
    };

    let mut lines = vec![
        section_heading(format!("Sector Name: {}", sector.name)),
        Line::from(""),
        Line::from(vec![
            Span::styled("Owner: ", Theme::muted_style()),
            Span::styled(owner_text, Theme::default_style()),
        ]),
        Line::from(vec![
            Span::styled("Visibility: ", Theme::muted_style()),
            Span::raw(visibility),
        ]),
        Line::from(vec![
            Span::styled("Known Systems: ", Theme::muted_style()),
            Span::raw(if matches!(fog, FogState::Unexplored) {
                "?".to_string()
            } else {
                format!("{known_systems} / {}", stars_in_sector.len())
            }),
        ]),
        Line::from(vec![
            Span::styled("Colonies: ", Theme::muted_style()),
            Span::raw(if matches!(fog, FogState::Unexplored) {
                "Unknown".to_string()
            } else {
                colony_count.to_string()
            }),
        ]),
        Line::from(vec![
            Span::styled("Fleet Presence: ", Theme::muted_style()),
            Span::raw(if matches!(fog, FogState::Unexplored) {
                "Unknown".to_string()
            } else {
                fleets_total.to_string()
            }),
        ]),
        meter_line("Threat", threat_percent, inner.width.saturating_sub(1)),
        Line::from(vec![
            Span::styled("Strategic Notes: ", Theme::muted_style()),
            Span::styled(strategic_notes[0].as_str(), Theme::text_primary_style()),
        ]),
    ];

    if inner.height > 13 {
        lines.push(Line::from(vec![
            Span::styled("  ", Theme::muted_style()),
            Span::styled(strategic_notes[1].as_str(), Theme::muted_style()),
        ]));
    }

    if inner.height > 16 {
        lines.push(Line::from(vec![
            Span::styled("Position: ", Theme::muted_style()),
            Span::raw(format!("({}, {})", sector.x, sector.y)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn strategic_notes(
    fog: FogState,
    owner: Option<game_core::EmpireId>,
    colonies: usize,
    fleets: usize,
    hostile_fleets: usize,
) -> [String; 2] {
    if matches!(fog, FogState::Unexplored) {
        return [
            "Long-range scans incomplete.".to_string(),
            "Send scouts before committing fleets.".to_string(),
        ];
    }
    if hostile_fleets > 0 {
        return [
            format!("Hostile contacts detected ({hostile_fleets})."),
            "Reinforce supply and keep strike groups nearby.".to_string(),
        ];
    }
    if colonies == 0 && owner.is_none() {
        return [
            "Open frontier sector.".to_string(),
            "Candidate for expansion and survey operations.".to_string(),
        ];
    }
    if fleets > 0 {
        return [
            "Friendly traffic active.".to_string(),
            "Use as staging lane for adjacent sectors.".to_string(),
        ];
    }
    [
        "Stable command space.".to_string(),
        "Monitor lanes and keep scouts rotating.".to_string(),
    ]
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
                    style: style.fg(Theme::bright_star()),
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
        let (map_area, _) = split_horizontal(main, 60);
        let block_inner = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .inner(map_area);
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
    fn sector_overview_renders_at_80x24_with_footer() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buf = render_to_buffer(&app_state, &engine.state, 80, 24);
        let text = buffer_text(&buf, Rect::new(0, 0, 80, 24));
        assert!(text.contains("Enter"));
        assert!(text.contains("Sector Map"));
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
        assert_eq!(unexplored_cell.symbol(), "◌");
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
        assert_eq!(
            cell.symbol(),
            glyphs_for_mode(app_state.visual_mode)
                .sector_selected
                .to_string()
        );
    }

    #[test]
    fn selected_sector_remains_visible_at_80x24() {
        let engine = Engine::new(42);
        let selected_sector = engine.state.sectors.keys().next().copied().unwrap();
        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_sector: Some(selected_sector),
                ..Default::default()
            },
            ..Default::default()
        };
        let buf = render_to_buffer(&app_state, &engine.state, 80, 24);
        let (render_area, viewport) = overview_viewport(80, 24);
        let sector = engine.state.sectors.get(&selected_sector).unwrap();
        let pos = viewport
            .world_to_screen_cell(WorldPoint::new(sector.x as f64, sector.y as f64))
            .unwrap();
        let cell = buf
            .cell((render_area.x + pos.x, render_area.y + pos.y))
            .unwrap();
        assert_eq!(
            cell.symbol(),
            glyphs_for_mode(app_state.visual_mode)
                .sector_selected
                .to_string()
        );
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
            "◌",
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
