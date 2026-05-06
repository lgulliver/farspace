//! Colony detail screen

use crate::components::{render_footer, render_header};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{BuildingType, GameState};
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
    let (credits, research, empire_name) = match empire {
        Some(e) => (e.credits, e.research_points, e.name.as_str()),
        None => (0, 0, "Unknown"),
    };

    render_header(
        frame,
        header_area,
        game_state.turn,
        empire_name,
        credits,
        research,
    );

    // Split main area 50/50: left=colony info+buildings, right=queue+picker
    let (left_area, right_area) = split_horizontal(main_area, 50);

    // Left column: stats (top 60%) and buildings (bottom 40%)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(left_area);

    render_colony_stats(frame, left_chunks[0], app_state, game_state);
    render_colony_buildings(frame, left_chunks[1], app_state, game_state);

    // Right column: queue (top 50%) and build picker (bottom 50%)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_area);

    render_production_queue(frame, right_chunks[0], app_state, game_state);
    render_build_picker(frame, right_chunks[1], app_state);

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

    let industry = (colony.production as i64 * colony.prod_pct as i64) / 100;
    let research_out = (colony.production as i64 * colony.research_pct as i64) / 100;

    let lines = vec![
        Line::from(vec![Span::styled(planet_name, Theme::title_style())]),
        Line::from(vec![
            Span::styled("Star: ", Theme::muted_style()),
            Span::raw(star_name),
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
            Span::styled("Industry   : ", Theme::muted_style()),
            Span::styled(format!("{}/turn", industry), Theme::accent_style()),
        ]),
        Line::from(vec![
            Span::styled("Research   : ", Theme::muted_style()),
            Span::styled(format!("{}/turn", research_out), Theme::accent_style()),
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
                Span::styled(format!(" {} ", item.name()), Theme::accent_style()),
                Span::styled(
                    format!("[{}{}]", "=".repeat(filled), " ".repeat(bar_width - filled)),
                    Theme::muted_style(),
                ),
                Span::raw(format!(" {}/{}", accumulated, cost)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::raw(format!("{}. {} ({}pp)", i + 1, item.name(), cost)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render the available buildings picker
fn render_build_picker(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let block = Block::default()
        .title(" Add to Queue ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let buildings = BuildingType::all();
    let cursor = app_state.colony_build_cursor % buildings.len();

    let mut lines = Vec::new();
    for (i, bt) in buildings.iter().enumerate() {
        let is_selected = i == cursor;
        let prefix = if is_selected { ">" } else { " " };
        let style = if is_selected {
            Theme::highlight_style()
        } else {
            Theme::default_style()
        };
        lines.push(Line::from(vec![Span::styled(
            format!(" {} [{:>3}pp] {} ", prefix, bt.cost(), bt.name()),
            style,
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("        {}", bt.description()),
            Theme::muted_style(),
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
}
