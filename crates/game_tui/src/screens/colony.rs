//! Colony detail screen

use crate::components::{
    derive_header_data, meter_line, panel_block, quiet_panel_block, render_footer, render_header,
    section_heading,
};
use crate::glyphs::glyphs_for_mode;
use crate::layout::{compose_layout, split_horizontal};
use crate::renderer::{
    palette::ColorToken,
    planet_art::{colony_portrait, portrait_input_from_colony},
    sprite::DetailLevel,
    Canvas, RenderLayer,
};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    yield_model, BuildItem, BuildingType, ColonyRole, ColonySupplyState, EmpireId, GameState,
    OrbitalStructureType, TechId,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Minimum portrait canvas height (rows) needed to reach Compact detail level.
const MIN_PORTRAIT_HEIGHT_FOR_COMPACT: u16 = 10;
/// Height (rows) reserved for the caption block below the portrait canvas.
const PORTRAIT_CAPTION_HEIGHT: u16 = 2;
const MIN_WIDTH_FOR_FULL_COLONY_LAYOUT: u16 = 100;
const MIN_HEIGHT_FOR_FULL_COLONY_LAYOUT: u16 = 24;
/// Minimum panel height needed before rendering extended job details.
const MIN_HEIGHT_FOR_JOB_DETAILS: u16 = 22;

fn ratio_percent_u64(value: u64, total: u64) -> u8 {
    value
        .saturating_mul(100)
        .checked_div(total)
        .map(|pct| pct.min(100) as u8)
        .unwrap_or(0)
}

fn ratio_percent_i64(value: i64, total: i64) -> u8 {
    if total <= 0 {
        0
    } else {
        ((value.saturating_mul(100) / total).clamp(0, 100)) as u8
    }
}

/// Render the colony detail screen
pub fn render_colony(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let compact = main_area.width < MIN_WIDTH_FOR_FULL_COLONY_LAYOUT
        || main_area.height < MIN_HEIGHT_FOR_FULL_COLONY_LAYOUT;
    if compact {
        let (left_area, right_area) = split_horizontal(main_area, 52);
        render_colony_stats(frame, left_area, app_state, game_state);
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(28),
                Constraint::Length(7),
                Constraint::Min(8),
            ])
            .split(right_area);
        render_production_queue(frame, right_chunks[0], app_state, game_state);
        render_role_selector(frame, right_chunks[1], app_state, game_state);
        render_build_picker(frame, right_chunks[2], app_state, game_state);
    } else {
        // Split main area left (portrait+stats+buildings) / right (queue+role+picker)
        let (left_area, right_area) = split_horizontal(main_area, 52);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(44),
                Constraint::Percentage(26),
            ])
            .split(left_area);

        render_colony_portrait(frame, left_chunks[0], app_state, game_state);
        render_colony_stats(frame, left_chunks[1], app_state, game_state);
        render_colony_buildings(frame, left_chunks[2], app_state, game_state);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(28),
                Constraint::Length(8),
                Constraint::Min(10),
            ])
            .split(right_area);

        render_production_queue(frame, right_chunks[0], app_state, game_state);
        render_role_selector(frame, right_chunks[1], app_state, game_state);
        render_build_picker(frame, right_chunks[2], app_state, game_state);
    }

    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Use Tab to switch panels, then Enter to set role or queue production.");
    render_footer(frame, footer_area, &Screen::Colony, Some(hint));
}

fn render_colony_portrait(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let glyphs = glyphs_for_mode(app_state.visual_mode);
    let block = Block::default()
        .title(" Colony Portrait ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Reserve caption only when there is enough height for the portrait canvas to reach at least
    // Compact detail (MIN_PORTRAIT_HEIGHT_FOR_COMPACT = 10 rows). Caption needs
    // PORTRAIT_CAPTION_HEIGHT rows, so the total threshold is their sum.
    let show_caption = inner.height >= MIN_PORTRAIT_HEIGHT_FOR_COMPACT + PORTRAIT_CAPTION_HEIGHT;

    let (portrait_area, caption_area_opt) = if show_caption {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),
                Constraint::Length(PORTRAIT_CAPTION_HEIGHT),
            ])
            .split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };

    let mut canvas = Canvas::new(portrait_area.width, portrait_area.height);
    canvas.fill(ColorToken::SpaceBg, RenderLayer::Background.z_base());
    let detail = surface_portrait_detail(portrait_area);
    let Some(colony_id) = app_state.colony.selected_colony else {
        canvas.draw_text(
            1,
            portrait_area.height / 2,
            "No colony selected",
            ColorToken::Muted.to_style(None),
            RenderLayer::Labels.z_base(),
        );
        canvas.render_to_buffer(portrait_area, frame.buffer_mut());
        return;
    };
    let Some(colony) = game_state.colonies.get(&colony_id) else {
        return;
    };
    let star = game_state.stars.get(&colony.star);
    let planet = star.and_then(|system| system.planets.get(colony.planet_index));
    let planet_class = planet.map(|planet| planet.class);
    let portrait = colony_portrait(
        portrait_input_from_colony(planet_class, Some(colony)),
        detail,
    );
    let x = portrait_area.width.saturating_sub(portrait.width) / 2;
    let y = portrait_area.height.saturating_sub(portrait.height) / 2;
    let needs_identity_overlay = needs_planet_identity_overlay(portrait_area, detail);
    canvas.draw_sprite(&portrait, x, y, 0, RenderLayer::Bodies.z_base());
    if needs_identity_overlay {
        draw_planet_identity_overlay(&mut canvas, portrait_area, glyphs.planet_colonized);
    }
    canvas.draw_text(
        1,
        0,
        "Surface View",
        ColorToken::Accent.to_style(None),
        RenderLayer::Labels.z_base(),
    );
    canvas.render_to_buffer(portrait_area, frame.buffer_mut());

    if let Some(caption_area) = caption_area_opt {
        let planet_name = planet.map(|p| p.name.as_str()).unwrap_or("Unknown");
        let class_name = planet.map(|p| p.class.name()).unwrap_or("Unknown");
        let star_name = star.map(|s| s.name.as_str()).unwrap_or("Unknown");
        let status_line = format!(
            "{}  |  {}  |  pop {}",
            class_name, star_name, colony.population
        );
        let caption = Paragraph::new(vec![
            Line::from(vec![Span::styled(planet_name, Theme::title_style())]),
            Line::from(vec![Span::styled(status_line, Theme::muted_style())]),
        ])
        .style(Theme::default_style());
        frame.render_widget(caption, caption_area);
    }
}

/// Surface view should prioritize a readable planet silhouette over coarse map-level detail
/// thresholds. Pick the largest detail that physically fits in the portrait canvas.
fn surface_portrait_detail(area: Rect) -> DetailLevel {
    if area.width >= 17 && area.height >= 11 {
        DetailLevel::Cinematic
    } else if area.width >= 9 && area.height >= 7 {
        DetailLevel::Standard
    } else if area.width >= 5 && area.height >= 3 {
        DetailLevel::Compact
    } else {
        DetailLevel::Tiny
    }
}

fn needs_planet_identity_overlay(area: Rect, detail: DetailLevel) -> bool {
    matches!(detail, DetailLevel::Tiny | DetailLevel::Compact) || area.width < 9 || area.height < 7
}

fn draw_planet_identity_overlay(canvas: &mut Canvas, area: Rect, icon: char) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let center_x = area.width / 2;
    let center_y = area.height / 2;
    canvas.set_cell(
        center_x,
        center_y,
        icon,
        ColorToken::Accent.to_style(None),
        RenderLayer::Labels.z_base() + 2,
    );
    if area.width >= 8 && area.height >= 4 {
        let label = "PLANET";
        let label_x = center_x.saturating_sub((label.len() as u16) / 2);
        let label_y = center_y
            .saturating_add(1)
            .min(area.height.saturating_sub(1));
        canvas.draw_text(
            label_x,
            label_y,
            label,
            ColorToken::Muted.to_style(None),
            RenderLayer::Labels.z_base() + 2,
        );
    }
}

/// Render colony statistics panel
fn render_colony_stats(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let glyphs = glyphs_for_mode(app_state.visual_mode);
    let block = quiet_panel_block("Colony Command");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let colony_id = match app_state.colony.selected_colony {
        Some(id) => id,
        None => {
            frame.render_widget(
                Paragraph::new("No colony selected").style(Theme::muted_style()),
                inner,
            );
            return;
        }
    };

    let colony = match game_state.colonies.get(&colony_id) {
        Some(c) => c,
        None => {
            frame.render_widget(
                Paragraph::new("Colony not found").style(Theme::error_style()),
                inner,
            );
            return;
        }
    };

    // Look up star and planet for name and housing capacity
    let star = game_state.stars.get(&colony.star);
    let star_name = star.map(|s| s.name.as_str()).unwrap_or("Unknown");
    let planet = star.and_then(|s| s.planets.get(colony.planet_index));
    let planet_name = planet.map(|p| p.name.as_str()).unwrap_or("Unknown");
    let planet_class = planet.map(|p| p.class.name()).unwrap_or("Unknown");

    // Calculate yields via the pop/jobs model
    let colony_yield = yield_model::calculate_yield(colony, planet);
    let workforce = &colony_yield.workforce;
    let housing_cap = workforce.housing;
    let industry = colony_yield.industry;
    let research_out = colony_yield.science;
    let total_maint = colony_yield.maintenance;
    let supply = game_state.colony_supply_state(colony.id);
    let blockade_empire: Option<EmpireId> = game_state.colony_blockade_state(colony.id);
    let unrest_state = game_state.colony_unrest_state(colony.id);
    let unrest_causes = game_state.colony_unrest_causes(colony.id);
    let unrest_risk_bp = game_state.colony_rebellion_risk_bp(colony.id);
    let specials_summary = if let Some(p) = planet {
        if p.surveyed
            && (!p.specials.is_empty() || !p.resources.is_empty() || !p.anomalies.is_empty())
        {
            let completed_techs = game_state
                .empires
                .get(&colony.owner)
                .map(|e| e.research.completed.as_slice())
                .unwrap_or(&[]);
            let visible_specials = game_core::visible_specials_for_empire(p, completed_techs).len();
            let visible_anomalies =
                game_core::visible_anomalies_for_empire(p, completed_techs).len();
            let visible_resources =
                game_core::visible_resources_for_empire(p, completed_techs).len();
            format!(
                "{} special  {} resource  {} anomaly",
                visible_specials, visible_resources, visible_anomalies
            )
        } else {
            "None detected".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    let rally_text = match colony.rally_point {
        Some(star_id) => {
            let star_name = game_state
                .stars
                .get(&star_id)
                .map(|s| s.name.as_str())
                .unwrap_or("Unknown");
            format!("{} ({})", star_name, star_id.0)
        }
        None => "None".to_string(),
    };

    let mut lines = vec![
        section_heading(format!("{} · {}", planet_name, star_name)),
        Line::from(vec![
            Span::styled("Class ", Theme::muted_style()),
            Span::raw(planet_class),
            Span::styled("  Role ", Theme::muted_style()),
            Span::styled(colony.role.name(), Theme::accent_style()),
        ]),
        meter_line(
            format!("Population {}/{}", colony.population, housing_cap),
            ratio_percent_u64(colony.population, housing_cap),
            inner.width.saturating_sub(1),
        ),
        meter_line(
            format!("Morale {}", colony.stability),
            colony.stability.min(100),
            inner.width.saturating_sub(1),
        ),
        {
            let (food_label, food_pct) = if colony_yield.food_consumed <= 0 {
                (format!("Food {}/0", colony_yield.food), if colony_yield.food > 0 { 100 } else { 0 })
            } else {
                (
                    format!("Food {}/{}", colony_yield.food, colony_yield.food_consumed),
                    ratio_percent_i64(colony_yield.food, colony_yield.food_consumed),
                )
            };
            meter_line(food_label, food_pct, inner.width.saturating_sub(1))
        },
        meter_line(
            format!("Industry {}/t", industry),
            colony.prod_pct.min(100),
            inner.width.saturating_sub(1),
        ),
        meter_line(
            format!("Research {}/t", research_out),
            colony.research_pct.min(100),
            inner.width.saturating_sub(1),
        ),
        Line::from(vec![
            Span::styled("Supply ", Theme::muted_style()),
            Span::styled(
                supply.label(),
                if supply == ColonySupplyState::Isolated {
                    Theme::warning_style()
                } else {
                    Theme::accent_style()
                },
            ),
            Span::styled("  Order ", Theme::muted_style()),
            Span::styled(
                unrest_state.label(),
                if unrest_state.is_unrest() {
                    Theme::warning_style()
                } else {
                    Theme::default_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Rebellion risk ", Theme::muted_style()),
            Span::styled(
                format!("{:.1}%", f64::from(unrest_risk_bp) / 100.0),
                if unrest_risk_bp >= 500 {
                    Theme::error_style()
                } else if unrest_risk_bp > 0 {
                    Theme::warning_style()
                } else {
                    Theme::success_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Planet specials ", Theme::muted_style()),
            Span::styled(specials_summary, Theme::default_style()),
        ]),
        Line::from(vec![
            Span::styled("Rally Point ", Theme::muted_style()),
            Span::styled(rally_text, Theme::accent_style()),
            Span::styled("  [R]set [X]clear", Theme::dim_border_style()),
        ]),
    ];

    if let Some(by_empire) = blockade_empire {
        let empire_name = game_state
            .empires
            .get(&by_empire)
            .map(|e| e.name.as_str())
            .unwrap_or("Unknown");
        lines.push(Line::from(vec![
            Span::styled("Blockade ", Theme::muted_style()),
            Span::styled(
                format!("{} by {}", glyphs.blockade, empire_name),
                Theme::error_style(),
            ),
        ]));
    }
    if !unrest_causes.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Unrest causes ", Theme::muted_style()),
            Span::styled(
                unrest_causes
                    .iter()
                    .map(|cause| cause.label())
                    .collect::<Vec<_>>()
                    .join(", "),
                Theme::warning_style(),
            ),
        ]));
    }

    // Show active specials/resources/anomalies for this colonized, surveyed planet.
    if let Some(p) = planet {
        if p.surveyed
            && (!p.specials.is_empty() || !p.resources.is_empty() || !p.anomalies.is_empty())
        {
            let completed_techs = game_state
                .empires
                .get(&colony.owner)
                .map(|e| e.research.completed.as_slice())
                .unwrap_or(&[]);
            let visible_specials = game_core::visible_specials_for_empire(p, completed_techs);
            let visible_anomalies = game_core::visible_anomalies_for_empire(p, completed_techs);
            let visible_resources = game_core::visible_resources_for_empire(p, completed_techs);
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Active Effects:",
                Theme::title_style(),
            )));
            for special in &visible_specials {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", glyphs.special), Theme::accent_style()),
                    Span::styled(special.name(), Theme::default_style()),
                    Span::styled(
                        format!(
                            " ({}, {}, {})",
                            special.rarity().label(),
                            special.category().label(),
                            special.effect_summary()
                        ),
                        Theme::muted_style(),
                    ),
                ]));
            }
            for anomaly in &visible_anomalies {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", glyphs.anomaly), Theme::warning_style()),
                    Span::styled(anomaly.name(), Theme::default_style()),
                    Span::styled(
                        format!(
                            " ({}, {}, {})",
                            anomaly.rarity().label(),
                            anomaly.category().label(),
                            anomaly.formatted_risk()
                        ),
                        Theme::muted_style(),
                    ),
                ]));
            }
            for resource in &visible_resources {
                let extracted = if game_state.colony_can_extract_resource(colony.id, *resource) {
                    "active"
                } else {
                    "offline"
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", glyphs.resource), Theme::accent_style()),
                    Span::styled(resource.name(), Theme::default_style()),
                    Span::styled(
                        format!(
                            " ({}, {}, {})",
                            resource.rarity().label(),
                            resource.category().label(),
                            extracted
                        ),
                        Theme::muted_style(),
                    ),
                ]));
            }
        }
    }

    if inner.height > MIN_HEIGHT_FOR_JOB_DETAILS {
        let mut job_lines = Vec::new();
        for assignment in &workforce.assignments {
            if assignment.total_slots == 0 && assignment.filled == 0 {
                continue;
            }
            job_lines.push(Line::from(vec![
                Span::styled("  ", Theme::muted_style()),
                Span::styled(
                    format!("{:<13}", assignment.job.label()),
                    Theme::muted_style(),
                ),
                Span::styled(
                    format!("{}/{}", assignment.filled, assignment.total_slots),
                    if assignment.job == yield_model::JobType::Unemployed && assignment.filled > 0 {
                        Theme::warning_style()
                    } else {
                        Theme::accent_style()
                    },
                ),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Maintenance ", Theme::muted_style()),
            Span::styled(format!("{} cr/t", total_maint), Theme::default_style()),
            Span::styled("  Employed ", Theme::muted_style()),
            Span::styled(
                format!("{}/{}", workforce.employed, workforce.population),
                Theme::accent_style(),
            ),
            Span::styled("  Unemployed ", Theme::muted_style()),
            Span::styled(
                workforce.unemployed.to_string(),
                if workforce.unemployed > 0 {
                    Theme::warning_style()
                } else {
                    Theme::success_style()
                },
            ),
        ]));
        if !job_lines.is_empty() {
            lines.push(Line::from(Span::styled("Jobs:", Theme::title_style())));
            lines.extend(job_lines);
        }
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render completed buildings panel
fn render_colony_buildings(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let glyphs = glyphs_for_mode(app_state.visual_mode);
    let block = quiet_panel_block("Structures");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let colony_id = match app_state.colony.selected_colony {
        Some(id) => id,
        None => return,
    };

    let colony = match game_state.colonies.get(&colony_id) {
        Some(c) => c,
        None => return,
    };

    if colony.buildings.is_empty() {
        let msg = Paragraph::new("No buildings yet").style(Theme::muted_style());
        frame.render_widget(msg, inner);
        return;
    }

    let lines: Vec<Line> = colony
        .buildings
        .iter()
        .map(|bt| {
            Line::from(vec![
                Span::styled(format!("  {} ", glyphs.bullet), Theme::accent_style()),
                Span::styled(bt.name(), Theme::default_style()),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render the colony role selector panel
fn render_role_selector(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let glyphs = glyphs_for_mode(app_state.visual_mode);
    let is_active = app_state.colony.role_panel_active;
    let block = panel_block("Colony Role [Tab]", is_active);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Determine the current role for the selected colony
    let current_role = app_state
        .colony
        .selected_colony
        .and_then(|cid| game_state.colonies.get(&cid))
        .map(|c| c.role)
        .unwrap_or(ColonyRole::Balanced);

    let roles = ColonyRole::all();
    let total = roles.len();
    let cursor = if total > 0 {
        app_state.colony.role_cursor % total
    } else {
        0
    };

    let lines: Vec<Line> = roles
        .iter()
        .enumerate()
        .map(|(i, role)| {
            let is_selected = i == cursor && is_active;
            let is_current = *role == current_role;
            let prefix = if is_selected { ">" } else { " " };
            let current_mark = if is_current {
                format!(" {}", glyphs.status_done)
            } else {
                "  ".to_string()
            };
            let style = if is_selected {
                Theme::highlight_style()
            } else if is_current {
                Theme::accent_style()
            } else {
                Theme::default_style()
            };
            Line::from(vec![Span::styled(
                format!(
                    " {} {}{} — {}",
                    prefix,
                    role.name(),
                    current_mark,
                    role.description()
                ),
                style,
            )])
        })
        .collect();

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render the production queue panel
fn render_production_queue(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let block = quiet_panel_block("Build Queue");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let colony_id = match app_state.colony.selected_colony {
        Some(id) => id,
        None => return,
    };

    let colony = match game_state.colonies.get(&colony_id) {
        Some(c) => c,
        None => return,
    };

    if colony.build_queue.is_empty() {
        let msg = Paragraph::new("Queue is empty").style(Theme::muted_style());
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines = Vec::new();
    for (i, item) in colony.build_queue.iter().enumerate() {
        let cost = item.cost();
        if i == 0 {
            let accumulated = colony.accumulated_production;
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ({})", item.name(), item.category_name()),
                    Theme::accent_style(),
                ),
                Span::raw(format!("  {}/{}", accumulated, cost)),
            ]));
            lines.push(meter_line(
                "Queue progress",
                if cost == 0 {
                    100
                } else {
                    ratio_percent_u64(accumulated, cost)
                },
                inner.width.saturating_sub(1),
            ));
            lines.push(Line::from(vec![Span::styled(
                format!("{} pp left", cost.saturating_sub(accumulated)),
                Theme::muted_style(),
            )]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::raw(format!(
                    "{}. [{}] {} ({}pp)",
                    i + 1,
                    item.category_name(),
                    item.name(),
                    cost
                )),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render the available buildings picker
fn render_build_picker(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let is_active = !app_state.colony.role_panel_active;
    let block = panel_block("Add to Queue", is_active);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Determine completed techs and colony state for lock/availability badges
    let completed_techs: Vec<TechId> = game_state
        .empires
        .get(&game_state.player_empire)
        .map(|e| e.research.completed.clone())
        .unwrap_or_default();

    // Look up the selected colony and its planet size for slot checks
    let (has_shipyard, has_surface_slot) = app_state
        .colony
        .selected_colony
        .and_then(|cid| game_state.colonies.get(&cid))
        .map(|colony| {
            let planet_size = game_state
                .stars
                .get(&colony.star)
                .and_then(|s| s.planets.get(colony.planet_index))
                .map(|p| p.size);
            let has_surface = planet_size
                .map(|sz| colony.can_place_surface_building(sz))
                .unwrap_or(true);
            (colony.has_shipyard(), has_surface)
        })
        .unwrap_or((false, true));

    // Filter to only unlocked orbital structures and ships
    let visible_orbitals: Vec<_> = OrbitalStructureType::all()
        .iter()
        .filter(|ot| {
            ot.required_tech()
                .map(|t| completed_techs.contains(&t))
                .unwrap_or(true)
        })
        .collect();
    let visible_ships: Vec<_> = game_core::all_ship_designs()
        .iter()
        .filter(|d| {
            d.required_tech
                .map(|t| completed_techs.contains(&t))
                .unwrap_or(true)
        })
        .collect();

    // Build the combined list: surface buildings, unlocked orbitals, unlocked ships
    let surface_count = BuildingType::all().len();
    let orbital_count = visible_orbitals.len();
    let total_count = surface_count + orbital_count + visible_ships.len();
    let cursor = if total_count > 0 {
        app_state.colony.build_cursor % total_count
    } else {
        0
    };

    let mut lines = Vec::new();

    // Surface buildings section
    lines.push(Line::from(vec![Span::styled(
        " Surface Structures",
        Theme::muted_style(),
    )]));
    for (i, bt) in BuildingType::all().iter().enumerate() {
        let is_selected = i == cursor;
        let prefix = if is_selected { ">" } else { " " };
        let slots_full = !has_surface_slot;
        let lock_tag = if slots_full { " [FULL]" } else { "" };
        let style = if is_selected {
            Theme::highlight_style()
        } else if slots_full {
            Theme::muted_style()
        } else {
            Theme::default_style()
        };
        lines.push(Line::from(vec![Span::styled(
            format!(
                " {} [{:>3}pp] {}{} ",
                prefix,
                bt.cost(),
                bt.name(),
                lock_tag
            ),
            style,
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("        {}", bt.description()),
            Theme::muted_style(),
        )]));
    }

    // Orbital structures section — only unlocked items shown
    if !visible_orbitals.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " Orbital Structures",
            Theme::muted_style(),
        )]));
        for (i, ot) in visible_orbitals.iter().enumerate() {
            let idx = surface_count + i;
            let is_selected = idx == cursor;
            let prefix = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Theme::highlight_style()
            } else {
                Theme::default_style()
            };
            lines.push(Line::from(vec![Span::styled(
                format!(" {} [{:>3}pp] {}", prefix, ot.cost(), ot.name()),
                style,
            )]));
            lines.push(Line::from(vec![Span::styled(
                format!("        {}", ot.description()),
                Theme::muted_style(),
            )]));
        }
    }

    // Ships section — only tech-unlocked ships shown
    if !visible_ships.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " Ships",
            Theme::muted_style(),
        )]));
        for (i, design) in visible_ships.iter().enumerate() {
            let ship_item = BuildItem::Ship(design.id);
            let idx = surface_count + orbital_count + i;
            let is_selected = idx == cursor;
            let prefix = if is_selected { ">" } else { " " };
            let no_shipyard_tag = if has_shipyard { "" } else { " [NO SHIPYARD]" };
            let style = if is_selected {
                Theme::highlight_style()
            } else if has_shipyard {
                Theme::default_style()
            } else {
                Theme::muted_style()
            };
            lines.push(Line::from(vec![Span::styled(
                format!(
                    " {} [{:>3}pp] {} [{}]{}",
                    prefix,
                    ship_item.cost(),
                    design.name,
                    design.role,
                    no_shipyard_tag
                ),
                style,
            )]));
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "        str:{} maint:{}/turn",
                    design.strength, design.maintenance
                ),
                Theme::muted_style(),
            )]));
        }
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    fn make_app_state_with_colony(engine: &Engine) -> AppState {
        // Find the first player colony
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id);
        AppState {
            colony: crate::app::ColonyScreenState {
                selected_colony: colony_id,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn render_buffer(
        width: u16,
        height: u16,
        app_state: &AppState,
        game_state: &GameState,
    ) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_colony(frame, frame.area(), app_state, game_state);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        buffer.content().iter().map(|c| c.symbol()).collect()
    }

    #[test]
    fn colony_screen_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn colony_screen_with_no_colony_selected() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn colony_screen_shows_supply_state() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("Surface View"));
        assert!(content.contains("Supply "));
        assert!(content.contains("Connected"));
    }

    #[test]
    fn colony_screen_with_buildings_in_queue() {
        use game_core::{BuildItem, Command};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();

        // Queue two buildings
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::AquacultureBay),
        }]);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::FabricationYard),
        }]);

        let app_state = AppState {
            colony: crate::app::ColonyScreenState {
                selected_colony: Some(colony_id),
                ..Default::default()
            },
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn colony_screen_with_completed_buildings() {
        use game_core::{BuildItem, Command};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();

        // Queue and complete a building
        engine.apply_turn(vec![
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Structure(BuildingType::ScienceNexus),
            },
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
        ]);
        for _ in 0..12 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let app_state = AppState {
            colony: crate::app::ColonyScreenState {
                selected_colony: Some(colony_id),
                ..Default::default()
            },
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn colony_screen_cursor_wraps_around() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = AppState {
            colony: crate::app::ColonyScreenState {
                selected_colony: engine
                    .state
                    .colonies
                    .iter()
                    .find(|(_, c)| c.owner == engine.state.player_empire)
                    .map(|(id, _)| *id),
                // cursor beyond bounds wraps via modulo
                build_cursor: 100,
                ..Default::default()
            },
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn build_picker_shows_shipyard_when_tech_researched() {
        use game_core::TechId;
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        // Grant Orbital Engineering so Shipyard is visible
        let empire_id = engine.state.player_empire;
        engine
            .state
            .empires
            .get_mut(&empire_id)
            .unwrap()
            .research
            .completed
            .push(TechId::ORBITAL_ENGINEERING);

        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("Shipyard"),
            "Build picker must render Shipyard when Orbital Engineering is researched"
        );
    }

    #[test]
    fn colony_screen_shows_unrest_state_and_causes() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        engine
            .state
            .colony_unrest
            .insert(colony_id, game_core::ColonyUnrestState::RevoltRisk);
        engine.state.colony_unrest_causes.insert(
            colony_id,
            vec![
                game_core::UnrestCause::Blockade,
                game_core::UnrestCause::RecentConquest,
            ],
        );
        engine.state.colony_rebellion_risk_bp.insert(colony_id, 900);
        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("Revolt Risk"));
        assert!(content.contains("Unrest causes"));
        assert!(content.contains("Recent conquest"));
    }

    #[test]
    fn build_picker_shows_scout_ship_without_research() {
        let backend = TestBackend::new(120, 80);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("Scout"),
            "Build picker must render Scout (no tech required); got: <content truncated>"
        );
    }

    #[test]
    fn build_picker_hides_locked_items_when_tech_missing() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        // New game — no techs researched, so Shipyard must not appear
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            !content.contains("Shipyard"),
            "Shipyard must be hidden when Orbital Engineering is not researched"
        );
    }

    #[test]
    fn build_picker_does_not_show_locked_when_tech_present() {
        use game_core::TechId;
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = Engine::new(42);
        // Grant Orbital Engineering to the player empire
        let empire_id = engine.state.player_empire;
        engine
            .state
            .empires
            .get_mut(&empire_id)
            .unwrap()
            .research
            .completed
            .push(TechId(7));

        let app_state = make_app_state_with_colony(&engine);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        // Shipyard is present but not marked locked
        assert!(
            content.contains("Shipyard"),
            "Shipyard must be visible when Orbital Engineering is researched"
        );
        // Shipyard specifically should not be marked locked when tech is available.
        assert!(
            !content.contains("Shipyard [LOCKED]"),
            "Shipyard must not show [LOCKED] when Orbital Engineering is researched"
        );
    }

    #[test]
    fn production_queue_shows_item_type_and_progress() {
        use game_core::{BuildItem, Command, ShipDesignId};

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .orbital_installations
            .push(game_core::OrbitalStructureType::Shipyard);

        engine.apply_turn(vec![
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::SurfaceStructure(BuildingType::AquacultureBay),
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Ship(ShipDesignId::SCOUT),
            },
        ]);
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .accumulated_production = 17;

        let app_state = AppState {
            colony: crate::app::ColonyScreenState {
                selected_colony: Some(colony_id),
                ..Default::default()
            },
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state, &engine.state);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("Surface"));
        assert!(content.contains("[Ship]"));
        assert!(content.contains("17/60"));
    }

    #[test]
    fn colony_screen_renders_at_80x24_with_footer() {
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);
        let buffer = render_buffer(80, 24, &app_state, &engine.state);
        let text = buffer_text(&buffer);
        assert!(text.contains("Colony Command"));
        assert!(text.contains("Enter"));
    }

    #[test]
    fn colony_screen_renders_at_120x36_with_footer() {
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);
        let buffer = render_buffer(120, 36, &app_state, &engine.state);
        let text = buffer_text(&buffer);
        assert!(text.contains("Build Queue"));
        assert!(text.contains("Esc"));
    }

    #[test]
    fn colony_screen_keeps_selected_role_visible() {
        let engine = Engine::new(42);
        let mut app_state = make_app_state_with_colony(&engine);
        app_state.colony.role_panel_active = true;
        let buffer = render_buffer(80, 24, &app_state, &engine.state);
        let text = buffer_text(&buffer);
        assert!(text.contains("Colony Role [Tab]"));
        assert!(text.contains(">"));
    }

    #[test]
    fn colony_screen_handles_minimal_state_at_80x24() {
        let mut engine = Engine::new(42);
        engine.state.colonies.clear();
        let app_state = AppState::default();
        let _ = render_buffer(80, 24, &app_state, &engine.state);
    }

    #[test]
    fn surface_portrait_detail_prefers_standard_when_it_fits() {
        assert_eq!(
            surface_portrait_detail(Rect::new(0, 0, 20, 9)),
            DetailLevel::Standard
        );
        assert_eq!(
            surface_portrait_detail(Rect::new(0, 0, 10, 7)),
            DetailLevel::Standard
        );
    }

    #[test]
    fn surface_portrait_detail_downgrades_when_canvas_is_too_small() {
        assert_eq!(
            surface_portrait_detail(Rect::new(0, 0, 8, 6)),
            DetailLevel::Compact
        );
        assert_eq!(
            surface_portrait_detail(Rect::new(0, 0, 4, 2)),
            DetailLevel::Tiny
        );
    }

    #[test]
    fn portrait_identity_overlay_applies_for_small_or_low_detail() {
        assert!(needs_planet_identity_overlay(
            Rect::new(0, 0, 8, 6),
            DetailLevel::Compact
        ));
        assert!(needs_planet_identity_overlay(
            Rect::new(0, 0, 12, 6),
            DetailLevel::Standard
        ));
        assert!(!needs_planet_identity_overlay(
            Rect::new(0, 0, 20, 12),
            DetailLevel::Cinematic
        ));
    }
}
