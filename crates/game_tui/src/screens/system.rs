//! System inspector screen

use std::{borrow::Cow, f32::consts::PI};

use crate::components::{derive_header_data, render_footer, render_header, render_log};
use crate::glyphs::glyphs_for_mode;
use crate::layout::{compose_layout, split_horizontal};
use crate::renderer::{
    palette::ColorToken,
    planet_art::{
        colony_portrait, planet_kind_from_class, planet_sprite, portrait_input_from_colony,
        star_sprite, PlanetVisualKind,
    },
    sprite::{detail_for_area, DetailLevel},
    Canvas, RenderLayer,
};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{empire_definition_by_id, ColonySupplyState, FleetKind, GameState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const ORBIT_SELECTION_PULSE_PERIOD: u64 = 5;
/// Rows above the planet sprite top at which the orbit number label is drawn.
/// Placing it here keeps the label clear of the bottom selection bracket (z=120),
/// which sits one row below the sprite bottom.
const ORBIT_LABEL_OFFSET_ABOVE_SPRITE: u16 = 2;

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
    render_log(
        frame,
        right_chunks[1],
        &app_state.log,
        app_state.visual_mode,
    );

    let hint = app_state.status_message.as_deref().unwrap_or(
        "Survey with S, colonize with C, invade with I, and watch fleet supply in roster.",
    );
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

fn empire_intel_level(
    game_state: &GameState,
    empire_id: game_core::EmpireId,
) -> game_core::IntelLevel {
    game_state.intel_level_for_empire(empire_id)
}

fn can_show_foreign_colony_details(game_state: &GameState, empire_id: game_core::EmpireId) -> bool {
    empire_id == game_state.player_empire
        || empire_intel_level(game_state, empire_id) >= game_core::IntelLevel::Informed
}

fn render_orbital_panel(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let glyphs = glyphs_for_mode(app_state.visual_mode);
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
        Span::styled(format!("{} ", glyphs.star), Theme::title_style()),
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
        let prefix = if selected {
            glyphs.list_selected.to_string()
        } else {
            " ".to_string()
        };
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
            .map(|_| glyphs.blockade.to_string())
            .unwrap_or_default();
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
            glyphs.planet_colonized.to_string()
        } else {
            glyphs.planet_uncolonized.to_string()
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

    let orbit_center_x = area.width.saturating_mul(2) / 5;
    let orbit_center_y = area.height / 2;
    let star_detail = if matches!(detail, DetailLevel::Cinematic) {
        DetailLevel::Standard
    } else {
        detail
    };
    let star_visual_sprite = star_sprite(star.spectral_class, star_detail);
    let star_x = orbit_center_x.saturating_sub(star_visual_sprite.width / 2);
    let star_y = orbit_center_y.saturating_sub(star_visual_sprite.height / 2);
    canvas.draw_sprite(
        &star_visual_sprite,
        star_x,
        star_y,
        0,
        RenderLayer::Bodies.z_base(),
    );

    if !star.planets.is_empty() {
        let selected_planet = app_state
            .navigation
            .selected_planet_index
            .min(star.planets.len().saturating_sub(1));
        let min_rx = star_visual_sprite.width / 2 + 3;
        let max_rx = orbit_center_x
            .min(area.width.saturating_sub(orbit_center_x).saturating_sub(6))
            .max(min_rx.saturating_add(1));
        let min_ry = star_visual_sprite.height / 2 + 1;
        let max_ry = orbit_center_y
            .min(area.height.saturating_sub(orbit_center_y).saturating_sub(3))
            .max(min_ry.saturating_add(1));
        let rx_span = max_rx.saturating_sub(min_rx);
        let ry_span = max_ry.saturating_sub(min_ry);

        for (index, planet) in star.planets.iter().enumerate() {
            let rx = min_rx
                + ((u32::from(rx_span) * (index as u32 + 1)) / star.planets.len() as u32) as u16;
            let ry = min_ry
                + ((u32::from(ry_span) * (index as u32 + 1)) / star.planets.len() as u32) as u16;
            draw_orbit_ring(
                &mut canvas,
                area,
                orbit_center_x,
                orbit_center_y,
                rx,
                ry,
                index == selected_planet,
            );

            let angle = (-0.58 * PI) + ((index % 6) as f32 * 0.37);
            let x = orbit_center_x as f32 + angle.cos() * rx as f32;
            let y = orbit_center_y as f32 + angle.sin() * ry as f32;
            let x = x.round().clamp(0.0, area.width.saturating_sub(1) as f32) as u16;
            let y = y.round().clamp(0.0, area.height.saturating_sub(1) as f32) as u16;
            let kind = if planet.surveyed {
                planet_kind_from_class(Some(planet.class))
            } else {
                PlanetVisualKind::Unknown
            };
            let sprite = if index == selected_planet && !matches!(detail, DetailLevel::Tiny) {
                planet_sprite(kind, DetailLevel::Compact)
            } else {
                planet_sprite(kind, DetailLevel::Tiny)
            };
            canvas.draw_sprite(
                &sprite,
                x.saturating_sub(sprite.width / 2),
                y.saturating_sub(sprite.height / 2),
                0,
                RenderLayer::Bodies.z_base() + 1,
            );
            let label_y = y
                .saturating_sub(sprite.height / 2)
                .saturating_sub(ORBIT_LABEL_OFFSET_ABOVE_SPRITE)
                .min(area.height.saturating_sub(1));
            canvas.draw_text(
                x.saturating_sub(1),
                label_y,
                &format!("{:>2}", index + 1),
                ColorToken::Muted.to_style(None),
                RenderLayer::Labels.z_base(),
            );
            if index == selected_planet {
                let flash = !app_state.reduced_motion
                    && (app_state.tick_count / ORBIT_SELECTION_PULSE_PERIOD).is_multiple_of(2);
                draw_selection_brackets(
                    &mut canvas,
                    area,
                    x,
                    y,
                    sprite.width,
                    sprite.height,
                    flash,
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

fn draw_orbit_ring(
    canvas: &mut Canvas,
    area: Rect,
    cx: u16,
    cy: u16,
    rx: u16,
    ry: u16,
    selected: bool,
) {
    if rx < 2 || ry < 2 {
        return;
    }

    let glyph = if selected { '•' } else { '·' };
    let style = if selected {
        ColorToken::Accent.to_style(None)
    } else {
        ColorToken::DimOverlay.to_style(None)
    };
    let tolerance = if selected { 0.22 } else { 0.14 };
    let x_start = cx.saturating_sub(rx).saturating_sub(1);
    let x_end = cx.saturating_add(rx).saturating_add(1).min(area.width);
    let y_start = cy.saturating_sub(ry).saturating_sub(1);
    let y_end = cy.saturating_add(ry).saturating_add(1).min(area.height);

    for y in y_start..y_end {
        let dy = y as f32 - cy as f32;
        for x in x_start..x_end {
            let dx = x as f32 - cx as f32;
            let distance =
                (dx * dx) / (rx as f32 * rx as f32) + (dy * dy) / (ry as f32 * ry as f32);
            if (distance - 1.0).abs() <= tolerance {
                canvas.set_cell(x, y, glyph, style, RenderLayer::Lanes.z_base());
            }
        }
    }
}

fn draw_selection_brackets(
    canvas: &mut Canvas,
    area: Rect,
    x: u16,
    y: u16,
    sprite_width: u16,
    sprite_height: u16,
    flash: bool,
) {
    let style = if flash {
        ColorToken::Accent2.to_style(None)
    } else {
        ColorToken::Accent.to_style(None)
    };
    let left_x = x.saturating_sub(sprite_width / 2).saturating_sub(2);
    let right_x = x
        .saturating_add(sprite_width / 2)
        .saturating_add(2)
        .min(area.width.saturating_sub(1));
    let top_y = y.saturating_sub(sprite_height / 2).saturating_sub(1);
    let bottom_y = y
        .saturating_add(sprite_height / 2)
        .saturating_add(1)
        .min(area.height.saturating_sub(1));

    canvas.set_cell(left_x, y, '⟨', style, RenderLayer::Selection.z_base());
    canvas.set_cell(right_x, y, '⟩', style, RenderLayer::Selection.z_base());
    canvas.set_cell(x, top_y, '⌃', style, RenderLayer::Selection.z_base());
    canvas.set_cell(x, bottom_y, '⌄', style, RenderLayer::Selection.z_base());
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
    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Min(0)])
        .split(inner);

    render_selected_planet_hero(
        frame,
        detail_chunks[0],
        game_state,
        star,
        planet,
        selected_planet,
        survey_state,
    );
    render_system_detail_facts(
        frame,
        detail_chunks[1],
        app_state,
        game_state,
        planet,
        survey_state,
        &fleets_here,
    );
}

fn render_selected_planet_hero(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    star: &game_core::Star,
    planet: &game_core::Planet,
    selected_planet: usize,
    survey_state: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(18)])
        .split(area);

    let mut canvas = Canvas::new(chunks[0].width, chunks[0].height);
    canvas.fill(ColorToken::SpaceBg, RenderLayer::Background.z_base());
    let render_detail = selected_world_detail(chunks[0]);
    let needs_identity_overlay = needs_planet_identity_overlay(chunks[0], render_detail);
    let sprite = if let Some(colony_id) = planet.colony {
        colony_portrait(
            portrait_input_from_colony(
                if survey_state == "Surveyed" {
                    Some(planet.class)
                } else {
                    None
                },
                game_state.colonies.get(&colony_id),
            ),
            render_detail,
        )
    } else {
        planet_sprite(
            if survey_state == "Surveyed" {
                planet_kind_from_class(Some(planet.class))
            } else {
                PlanetVisualKind::Unknown
            },
            render_detail,
        )
    };
    let sprite_x = chunks[0].width.saturating_sub(sprite.width) / 2;
    let sprite_y = chunks[0].height.saturating_sub(sprite.height) / 2;
    canvas.draw_sprite(&sprite, sprite_x, sprite_y, 0, RenderLayer::Bodies.z_base());
    if needs_identity_overlay {
        draw_planet_identity_overlay(&mut canvas, chunks[0]);
    }
    draw_selection_brackets(
        &mut canvas,
        chunks[0],
        sprite_x.saturating_add(sprite.width / 2),
        sprite_y.saturating_add(sprite.height / 2),
        sprite.width,
        sprite.height,
        true,
    );
    canvas.render_to_buffer(chunks[0], frame.buffer_mut());

    let mut lines = vec![
        Line::from(vec![Span::styled("Selected World", Theme::title_style())]),
        Line::from(vec![
            Span::styled(
                if survey_state == "Surveyed" {
                    planet.name.as_str()
                } else {
                    "Unidentified World"
                },
                Theme::accent_style(),
            ),
            Span::styled(
                format!("  Orbit {}", selected_planet + 1),
                Theme::muted_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled(star.name.as_str(), Theme::muted_style()),
            Span::styled(
                format!(" [{}]", star.spectral_class.as_char()),
                Style::default().fg(Theme::star_color(star.spectral_class)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Survey ", Theme::muted_style()),
            Span::styled(survey_state, survey_style(survey_state)),
            Span::styled("  Signature ", Theme::muted_style()),
            Span::styled(
                planet_signature(planet, survey_state),
                Theme::accent_style(),
            ),
        ]),
    ];
    if let Some(colony_id) = planet.colony {
        if let Some(colony) = game_state.colonies.get(&colony_id) {
            if can_show_foreign_colony_details(game_state, colony.owner) {
                lines.push(Line::from(vec![
                    Span::styled("Colony ", Theme::muted_style()),
                    Span::styled(format!("Pop {}", colony.population), Theme::accent_style()),
                    Span::styled("  Order ", Theme::muted_style()),
                    Span::styled(
                        game_state.colony_unrest_label(colony.id),
                        Theme::default_style(),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Colony ", Theme::muted_style()),
                    Span::styled("Foreign settlement — details hidden", Theme::muted_style()),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("Colony ", Theme::muted_style()),
            Span::styled("Absent", Theme::muted_style()),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Theme::default_style()),
        chunks[1],
    );
}

fn selected_world_detail(area: Rect) -> DetailLevel {
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

fn draw_planet_identity_overlay(canvas: &mut Canvas, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let center_x = area.width / 2;
    let center_y = area.height / 2;
    canvas.set_cell(
        center_x,
        center_y,
        '◉',
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

fn render_system_detail_facts(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
    planet: &game_core::Planet,
    survey_state: &str,
    fleets_here: &[&game_core::Fleet],
) {
    let glyphs = glyphs_for_mode(app_state.visual_mode);
    let selected_fleet_index = app_state.navigation.selected_fleet_index;
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Survey: ", Theme::muted_style()),
        Span::styled(survey_state, survey_style(survey_state)),
    ]));

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
        let completed_techs = game_state
            .empires
            .get(&game_state.player_empire)
            .map(|e| e.research.completed.as_slice())
            .unwrap_or(&[]);
        let visible_specials = game_core::visible_specials_for_empire(planet, completed_techs);
        let visible_anomalies = game_core::visible_anomalies_for_empire(planet, completed_techs);
        if visible_specials.is_empty()
            && visible_anomalies.is_empty()
            && planet.resources.is_empty()
        {
            lines.push(Line::from(vec![
                Span::styled("Specials: ", Theme::muted_style()),
                Span::styled("None", Theme::muted_style()),
            ]));
        } else {
            if !visible_specials.is_empty() {
                let specials_text: Vec<String> = visible_specials
                    .iter()
                    .map(|s| {
                        format!(
                            "{}{} ({}, {}, {})",
                            glyphs.special,
                            s.name(),
                            s.rarity().label(),
                            s.category().label(),
                            s.effect_summary()
                        )
                    })
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("Specials: ", Theme::muted_style()),
                    Span::styled(specials_text.join(", "), Theme::accent_style()),
                ]));
            }
            if !visible_anomalies.is_empty() {
                let anomalies_text: Vec<String> = visible_anomalies
                    .iter()
                    .map(|a| {
                        format!(
                            "{}{} ({}, {}, {})",
                            glyphs.anomaly,
                            a.name(),
                            a.rarity().label(),
                            a.category().label(),
                            a.formatted_risk()
                        )
                    })
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("Anomalies: ", Theme::muted_style()),
                    Span::styled(anomalies_text.join(", "), Theme::warning_style()),
                ]));
            }
            let visible_resources =
                game_core::visible_resources_for_empire(planet, completed_techs);
            if !visible_resources.is_empty() {
                let resources_text: Vec<String> = visible_resources
                    .iter()
                    .map(|r| {
                        format!(
                            "{} ({}, {}, tv {})",
                            r.name(),
                            r.rarity().label(),
                            r.category().label(),
                            r.trade_value()
                        )
                    })
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("Resources: ", Theme::muted_style()),
                    Span::styled(resources_text.join(", "), Theme::accent_style()),
                ]));
                if let Some(colony_id) = planet.colony {
                    let extracted: Vec<String> = visible_resources
                        .iter()
                        .map(|resource| {
                            if game_state.colony_can_extract_resource(colony_id, *resource) {
                                format!("{}: active", resource.name())
                            } else {
                                format!("{}: offline", resource.name())
                            }
                        })
                        .collect();
                    lines.push(Line::from(vec![
                        Span::styled("Extraction: ", Theme::muted_style()),
                        Span::styled(extracted.join(", "), Theme::default_style()),
                    ]));
                }
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
            if can_show_foreign_colony_details(game_state, colony.owner) {
                let infra = colony.surface_installations.len() + colony.orbital_installations.len();
                let supply = game_state.colony_supply_state(colony.id);
                let planet_ref = game_state
                    .stars
                    .get(&colony.star)
                    .and_then(|s| s.planets.get(colony.planet_index));
                let y = game_core::yield_model::calculate_yield(colony, planet_ref);
                format!(
                    "Colony {} (Empire {}, Pop {}, Emp {}/{}, Infra {}, {}, {})",
                    colony_id.0,
                    colony.owner.0,
                    colony.population,
                    y.workforce.employed,
                    y.workforce.population,
                    infra,
                    supply.label(),
                    game_state.colony_unrest_label(colony.id)
                )
            } else {
                format!(
                    "Foreign colony ({})",
                    game_state
                        .empires
                        .get(&colony.owner)
                        .map(|empire| empire.name.as_str())
                        .unwrap_or("unknown owner")
                )
            }
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
            if can_show_foreign_colony_details(game_state, colony.owner) {
                let supply = game_state.colony_supply_state(colony.id);
                let planet_ref = game_state
                    .stars
                    .get(&colony.star)
                    .and_then(|s| s.planets.get(colony.planet_index));
                let y = game_core::yield_model::calculate_yield(colony, planet_ref);
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
                lines.push(Line::from(vec![
                    Span::styled("Economy:", Theme::muted_style()),
                    Span::styled(
                        format!(
                            " F {:+}  I {}  S {}  C {}",
                            y.food - y.food_consumed,
                            y.industry,
                            y.science,
                            y.credits
                        ),
                        Theme::accent_style(),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Housing:", Theme::muted_style()),
                    Span::styled(
                        format!(
                            " {} / {}  unemployed {}",
                            y.workforce.population, y.workforce.housing, y.workforce.unemployed
                        ),
                        if y.workforce.unemployed > 0 || y.workforce.housing_deficit > 0 {
                            Theme::warning_style()
                        } else {
                            Theme::accent_style()
                        },
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Trade:  ", Theme::muted_style()),
                    Span::styled("Hidden by limited intel", Theme::muted_style()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Economy:", Theme::muted_style()),
                    Span::styled("Hidden by limited intel", Theme::muted_style()),
                ]));
            }
        }
    }

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
        for (idx, fleet) in fleets_here.iter().enumerate() {
            let intel = empire_intel_level(game_state, fleet.owner);
            let owns_fleet = fleet.owner == game_state.player_empire;
            let order_label = match game_state.fleet_orders.get(&fleet.id) {
                Some(game_core::FleetOrder::Hold) => " [Hold]".to_string(),
                Some(game_core::FleetOrder::MoveToSystem(star_id)) => {
                    format!(" [→ {}]", star_id.0)
                }
                None => String::new(),
            };
            let role = game_state.fleet_role_for(fleet.id);
            let formation = game_state.fleet_formation_for(fleet.id);
            let doctrine = game_state
                .empires
                .get(&fleet.owner)
                .and_then(|empire| empire.empire_def)
                .and_then(empire_definition_by_id)
                .map(|def| def.doctrine_short_summary())
                .unwrap_or_else(|| "N/A".to_string());
            let composition = format!("{}x {}", fleet.ships, fleet.kind.label());
            let summary = game_state
                .fleet_evaluation(fleet.id)
                .map(|eval| {
                    format!(
                        "off {} def {} inv {} mob {}",
                        eval.offensive, eval.defensive, eval.invasion_capability, eval.mobility
                    )
                })
                .unwrap_or_else(|| "off ? def ? inv ? mob ?".to_string());
            let supply = game_state.fleet_supply_state(fleet.id);
            let prefix = if idx == selected_fleet_index {
                glyphs.list_selected.to_string()
            } else {
                " ".to_string()
            };
            let mut name = if owns_fleet || intel >= game_core::IntelLevel::Basic {
                game_state.fleet_name_for(fleet.id)
            } else {
                "Foreign Fleet".to_string()
            };
            if owns_fleet && matches!(fleet.kind, FleetKind::Science | FleetKind::SurveyCutter) {
                if let Some(mission) = game_state.survey_missions.get(&fleet.id) {
                    name.push_str(&format!(" (Surveying orbit {})", mission.planet_index + 1));
                }
            }
            lines.push(Line::from(vec![
                Span::raw(format!("{} {} ", prefix, name)),
                Span::styled(
                    if owns_fleet || intel >= game_core::IntelLevel::Informed {
                        format!(
                            "[{} | {} | DOC {}]",
                            role.label(),
                            formation.label(),
                            doctrine
                        )
                    } else {
                        "[details hidden]".to_string()
                    },
                    Theme::muted_style(),
                ),
                Span::raw(" "),
                Span::raw(if owns_fleet || intel >= game_core::IntelLevel::Informed {
                    composition
                } else if intel.reveals_fleet_strength() {
                    game_state
                        .empire_fleet_strength_band(fleet.owner)
                        .to_string()
                } else {
                    "strength hidden".to_string()
                }),
                Span::styled(
                    if owns_fleet || intel >= game_core::IntelLevel::Informed {
                        format!(" {} {}", glyphs.separator_dot, supply.label())
                    } else {
                        format!(" {} supply hidden", glyphs.separator_dot)
                    },
                    if owns_fleet || intel >= game_core::IntelLevel::Informed {
                        Theme::fleet_supply_style(supply)
                    } else {
                        Theme::muted_style()
                    },
                ),
                Span::styled(
                    if owns_fleet || intel >= game_core::IntelLevel::Informed {
                        format!(" {} {}", glyphs.separator_dot, summary)
                    } else {
                        format!(" {} intel {}", glyphs.separator_dot, intel.label())
                    },
                    if owns_fleet || intel >= game_core::IntelLevel::Informed {
                        Theme::default_style()
                    } else {
                        Theme::muted_style()
                    },
                ),
                Span::styled(
                    if owns_fleet {
                        order_label
                    } else {
                        String::new()
                    },
                    Theme::muted_style(),
                ),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "  [f] focus fleet  [R] next role  [F] next formation  [B] battle reports  supply: Supplied / Extended / Out of Supply",
            Theme::muted_style(),
        )));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), area);
}

fn survey_style(survey_state: &str) -> Style {
    match survey_state {
        "Surveyed" => Theme::accent_style(),
        "Surveying" => Theme::warning_style(),
        _ => Theme::muted_style(),
    }
}

fn planet_signature(planet: &game_core::Planet, survey_state: &str) -> &'static str {
    if survey_state != "Surveyed" {
        return "Unknown";
    }

    match planet.class {
        game_core::PlanetClass::Terran => "Blue-green world",
        game_core::PlanetClass::Oceanic => "Deep-water world",
        game_core::PlanetClass::Desert => "Dry dust world",
        game_core::PlanetClass::Volcanic => "Molten fault world",
        game_core::PlanetClass::Frozen => "Icebound world",
        game_core::PlanetClass::Barren => "Airless stone world",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{
        EmpireIntel, Engine, Fleet, FleetId, FleetKind, IntelLevel, RelationshipStatus,
    };
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};

    fn render_to_string(engine: &Engine, app_state: &AppState) -> String {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_system(frame, frame.area(), app_state, &engine.state))
            .unwrap();
        let buf = terminal.backend().buffer();
        (0..40u16)
            .flat_map(|y| {
                (0..120u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect()
    }

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
        assert!(rendered.contains("Selected World"));
        assert!(rendered.contains("Signature"));
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
    fn foreign_colony_and_fleet_details_stay_hidden_until_intel_improves() {
        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Contacted);
        engine.state.empire_intel.insert(
            ai_id,
            EmpireIntel {
                level: IntelLevel::Contacted,
                points: 0,
                last_gather_turn: None,
            },
        );

        let enemy_colony_id = *engine
            .state
            .colonies
            .iter()
            .find_map(|(colony_id, colony)| (colony.owner == ai_id).then_some(colony_id))
            .expect("enemy colony should exist");
        let enemy_colony = engine.state.colonies[&enemy_colony_id].clone();
        engine.state.fleets.insert(
            FleetId(777),
            Fleet {
                id: FleetId(777),
                owner: ai_id,
                location: enemy_colony.star,
                ships: 4,
                kind: FleetKind::Destroyer,
                strength: 6,
                integrity: 100,
            },
        );

        let app_state = AppState {
            navigation: crate::app::NavigationState {
                selected_star: Some(enemy_colony.star),
                selected_planet_index: enemy_colony.planet_index,
                ..Default::default()
            },
            ..Default::default()
        };

        let rendered = render_to_string(&engine, &app_state);
        assert!(rendered.contains("Foreign settlement"));
        assert!(rendered.contains("Foreign Fleet"));
        assert!(!rendered.contains("4x Destroyer"));
        assert!(rendered.contains("details hidden"));
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

    #[test]
    fn selected_world_detail_prefers_standard_when_it_fits() {
        assert_eq!(
            selected_world_detail(Rect::new(0, 0, 20, 9)),
            DetailLevel::Standard
        );
        assert_eq!(
            selected_world_detail(Rect::new(0, 0, 10, 7)),
            DetailLevel::Standard
        );
    }

    #[test]
    fn selected_world_detail_downgrades_when_canvas_is_too_small() {
        assert_eq!(
            selected_world_detail(Rect::new(0, 0, 8, 6)),
            DetailLevel::Compact
        );
        assert_eq!(
            selected_world_detail(Rect::new(0, 0, 4, 2)),
            DetailLevel::Tiny
        );
    }

    #[test]
    fn selected_world_identity_overlay_applies_for_small_or_low_detail() {
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
