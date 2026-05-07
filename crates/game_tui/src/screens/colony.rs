//! Colony detail screen

use crate::components::{render_footer, render_header};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    yield_model, BuildItem, BuildingType, ColonyRole, GameState, OrbitalStructureType, TechId,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the colony detail screen
pub fn render_colony(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
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

    // Split main area left (colony info+buildings) / right (queue+role+picker) at 50%
    let (left_area, right_area) = split_horizontal(main_area, 50);

    // Left column: stats (top 60%) and buildings (bottom 40%)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(left_area);

    render_colony_stats(frame, left_chunks[0], app_state, game_state);
    render_colony_buildings(frame, left_chunks[1], app_state, game_state);

    // Right column: queue (top 35%), role selector (middle 30%), build picker (bottom 35%)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ])
        .split(right_area);

    render_production_queue(frame, right_chunks[0], app_state, game_state);
    render_role_selector(frame, right_chunks[1], app_state, game_state);
    render_build_picker(frame, right_chunks[2], app_state, game_state);

    render_footer(frame, footer_area, &Screen::Colony);
}

/// Render colony statistics panel
fn render_colony_stats(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let block = Block::default()
        .title(" Colony ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let colony_id = match app_state.selected_colony {
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

    // Look up star and planet for name and food capacity
    let star = game_state.stars.get(&colony.star);
    let star_name = star.map(|s| s.name.as_str()).unwrap_or("Unknown");
    let planet = star.and_then(|s| s.planets.get(colony.planet_index));
    let food_cap = planet.map(|p| p.size.base_capacity()).unwrap_or(0);
    let planet_name = planet.map(|p| p.name.as_str()).unwrap_or("Unknown");
    let planet_class = planet.map(|p| p.class.name()).unwrap_or("Unknown");

    // Calculate yields via the v2 model
    let colony_yield = yield_model::calculate_yield(colony, planet);
    let industry = colony_yield.industry;
    let research_out = colony_yield.science;
    let food_out = colony_yield.food;
    let total_maint = colony_yield.maintenance;

    // Get planet size for infrastructure slot capacity
    let (surface_used, surface_max, orbital_used, orbital_max) = planet
        .map(|p| {
            (
                colony.surface_installations.len() as u32,
                p.size.surface_slots() as u32,
                colony.orbital_installations.len() as u32,
                p.size.orbital_slots() as u32,
            )
        })
        .unwrap_or((0, 0, 0, 0));

    let lines = vec![
        Line::from(vec![Span::styled(planet_name, Theme::title_style())]),
        Line::from(vec![
            Span::styled("Star: ", Theme::muted_style()),
            Span::raw(star_name),
        ]),
        Line::from(vec![
            Span::styled("Class: ", Theme::muted_style()),
            Span::raw(planet_class),
            Span::styled("  Stability: ", Theme::muted_style()),
            Span::raw(format!("{}", colony.stability)),
        ]),
        Line::from(vec![
            Span::styled("Role:  ", Theme::muted_style()),
            Span::styled(colony.role.name(), Theme::accent_style()),
            Span::styled("  (", Theme::muted_style()),
            Span::styled(colony.role.description(), Theme::muted_style()),
            Span::styled(")", Theme::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Population : ", Theme::muted_style()),
            Span::styled(
                format!("{} / {}", colony.population, food_cap),
                Theme::accent_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Food Cap   : ", Theme::muted_style()),
            Span::raw(format!("{}", food_cap)),
        ]),
        Line::from(vec![
            Span::styled("Food/turn  : ", Theme::muted_style()),
            Span::styled(format!("+{}/turn", food_out), Theme::accent_style()),
        ]),
        Line::from(vec![
            Span::styled("Surface    : ", Theme::muted_style()),
            Span::styled(
                format!("{}/{}", surface_used, surface_max),
                Theme::accent_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Orbital    : ", Theme::muted_style()),
            Span::styled(
                format!("{}/{}", orbital_used, orbital_max),
                Theme::accent_style(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Industry   : ", Theme::muted_style()),
            Span::styled(format!("{}/turn", industry), Theme::accent_style()),
        ]),
        Line::from(vec![
            Span::styled("Research   : ", Theme::muted_style()),
            Span::styled(format!("{}/turn", research_out), Theme::accent_style()),
        ]),
        Line::from(vec![
            Span::styled("Maint cost : ", Theme::muted_style()),
            Span::styled(format!("{} cr/turn", total_maint), Theme::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Prod focus : ", Theme::muted_style()),
            Span::raw(format!("{}%", colony.prod_pct)),
            Span::styled("  Res focus: ", Theme::muted_style()),
            Span::raw(format!("{}%", colony.research_pct)),
        ]),
    ];

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
    let block = Block::default()
        .title(" Buildings ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let colony_id = match app_state.selected_colony {
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
                Span::styled("  • ", Theme::accent_style()),
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
    let is_active = app_state.colony_role_panel_active;
    let border_style = if is_active {
        Theme::focused_border_style()
    } else {
        Theme::dim_border_style()
    };
    let block = Block::default()
        .title(" Colony Role [Tab] ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Determine the current role for the selected colony
    let current_role = app_state
        .selected_colony
        .and_then(|cid| game_state.colonies.get(&cid))
        .map(|c| c.role)
        .unwrap_or(ColonyRole::Balanced);

    let roles = ColonyRole::all();
    let total = roles.len();
    let cursor = if total > 0 {
        app_state.colony_role_cursor % total
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
            let current_mark = if is_current { " ✓" } else { "  " };
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
    let block = Block::default()
        .title(" Production Queue ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let colony_id = match app_state.selected_colony {
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
            // Show progress bar for the active item
            let accumulated = colony.accumulated_production;
            let bar_width = inner.width.saturating_sub(4) as usize;
            let bar_width = bar_width.min(20);
            let filled = (accumulated * bar_width as u64)
                .checked_div(cost)
                .unwrap_or(bar_width as u64)
                .min(bar_width as u64) as usize;
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ({}) ", item.name(), item.category_name()),
                    Theme::accent_style(),
                ),
                Span::styled(
                    format!("[{}{}]", "=".repeat(filled), " ".repeat(bar_width - filled)),
                    Theme::muted_style(),
                ),
                Span::raw(format!(
                    " {}/{} ({}pp left)",
                    accumulated,
                    cost,
                    cost.saturating_sub(accumulated)
                )),
            ]));
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
    let is_active = !app_state.colony_role_panel_active;
    let border_style = if is_active {
        Theme::focused_border_style()
    } else {
        Theme::dim_border_style()
    };
    let block = Block::default()
        .title(" Add to Queue ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());
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

    // Build the combined list: surface buildings, orbital structures, then ships
    let surface_count = BuildingType::all().len();
    let orbital_count = OrbitalStructureType::all().len();
    let ship_count = game_core::all_ship_designs().len();
    let total_count = surface_count + orbital_count + ship_count;
    let cursor = if total_count > 0 {
        app_state.colony_build_cursor % total_count
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

    // Orbital structures section
    lines.push(Line::from(vec![Span::styled(
        " Orbital Structures",
        Theme::muted_style(),
    )]));
    for (i, ot) in OrbitalStructureType::all().iter().enumerate() {
        let idx = surface_count + i;
        let is_selected = idx == cursor;
        let prefix = if is_selected { ">" } else { " " };
        let tech_unlocked = ot
            .required_tech()
            .map(|t| completed_techs.contains(&t))
            .unwrap_or(true);
        let lock_tag = if tech_unlocked { "" } else { " [LOCKED]" };
        let style = if is_selected {
            Theme::highlight_style()
        } else if tech_unlocked {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };
        lines.push(Line::from(vec![Span::styled(
            format!(" {} [{:>3}pp] {}{}", prefix, ot.cost(), ot.name(), lock_tag),
            style,
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("        {}", ot.description()),
            Theme::muted_style(),
        )]));
    }

    // Ships section — iterate over all_ship_designs() directly to avoid per-frame allocation
    lines.push(Line::from(vec![Span::styled(
        " Ships",
        Theme::muted_style(),
    )]));
    for (i, design) in game_core::all_ship_designs().iter().enumerate() {
        let ship_item = BuildItem::Ship(design.id);
        let idx = surface_count + orbital_count + i;
        let is_selected = idx == cursor;
        let prefix = if is_selected { ">" } else { " " };
        let tech_unlocked = design
            .required_tech
            .map(|t| completed_techs.contains(&t))
            .unwrap_or(true);
        let lock_tag = match (has_shipyard, tech_unlocked) {
            (true, true) => "",
            (false, true) => " [NO SHIPYARD]",
            (true, false) => " [LOCKED]",
            (false, false) => " [NO SHIPYARD][LOCKED]",
        };
        let style = if is_selected {
            Theme::highlight_style()
        } else if has_shipyard && tech_unlocked {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };
        lines.push(Line::from(vec![Span::styled(
            format!(
                " {} [{:>3}pp] {}{}",
                prefix,
                ship_item.cost(),
                design.name,
                lock_tag
            ),
            style,
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

    fn make_app_state_with_colony(engine: &Engine) -> AppState {
        // Find the first player colony
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id);
        AppState {
            selected_colony: colony_id,
            ..Default::default()
        }
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
        let app_state = AppState {
            selected_colony: None,
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
            selected_colony: Some(colony_id),
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
            selected_colony: Some(colony_id),
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
            selected_colony: engine
                .state
                .colonies
                .iter()
                .find(|(_, c)| c.owner == engine.state.player_empire)
                .map(|(id, _)| *id),
            // cursor beyond bounds wraps via modulo
            colony_build_cursor: 100,
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
    fn build_picker_shows_shipyard_in_orbital_section() {
        // The build picker renders without panic and includes Shipyard in the output
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = make_app_state_with_colony(&engine);

        // Set cursor to the Shipyard position (after surface buildings)
        let shipyard_cursor = BuildingType::all().len(); // first orbital = after all surface
        let app_state_at_shipyard = AppState {
            selected_colony: app_state.selected_colony,
            colony_build_cursor: shipyard_cursor,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_colony(frame, area, &app_state_at_shipyard, &engine.state);
            })
            .unwrap();

        // Verify "Shipyard" appears in the rendered output
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("Shipyard"),
            "Build picker must render Shipyard; got: <content truncated>"
        );
    }

    #[test]
    fn build_picker_shows_locked_when_tech_missing() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        // New game — no techs researched, so Shipyard should show [LOCKED]
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
            content.contains("LOCKED"),
            "Shipyard must show [LOCKED] when Orbital Engineering is not researched"
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
        // [LOCKED] should not appear anywhere when tech is available
        assert!(
            !content.contains("LOCKED"),
            "LOCKED badge must not appear when Orbital Engineering is researched"
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
            selected_colony: Some(colony_id),
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
}
