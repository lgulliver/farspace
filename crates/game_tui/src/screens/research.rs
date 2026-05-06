//! Research screen

use crate::components::{render_footer, render_header};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{all_techs, GameState};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the research screen
pub fn render_research(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let empire = game_state.empires.get(&game_state.player_empire);
    let (credits, food, research_pts, empire_name) = match empire {
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
        research_pts,
    );

    // Split: 60% tech list, 40% right panel (current research + completed)
    let (list_area, right_area) = split_horizontal(main_area, 60);

    render_tech_list(frame, list_area, app_state, game_state);
    render_research_status(frame, right_area, game_state);

    render_footer(frame, footer_area, &Screen::Research);
}

/// Render the list of available (not yet completed) technologies
fn render_tech_list(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let block = Block::default()
        .title(" Available Technologies ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let empire = match game_state.empires.get(&game_state.player_empire) {
        Some(e) => e,
        None => return,
    };

    let all = all_techs();
    let available: Vec<_> = all
        .iter()
        .filter(|t| !empire.research.completed.contains(&t.id))
        .collect();

    if available.is_empty() {
        let msg = Paragraph::new("All technologies researched!").style(Theme::accent_style());
        frame.render_widget(msg, inner);
        return;
    }

    let cursor = app_state.research_cursor % available.len();
    let mut lines = Vec::new();

    for (i, tech) in available.iter().enumerate() {
        let is_selected = i == cursor;
        let is_current = empire.research.current_tech == Some(tech.id);

        let prefix = if is_selected {
            ">"
        } else if is_current {
            "·"
        } else {
            " "
        };

        let style = if is_selected {
            Theme::highlight_style()
        } else if is_current {
            Theme::accent_style()
        } else {
            Theme::default_style()
        };

        lines.push(Line::from(vec![Span::styled(
            format!(" {} [{:>3}rp] {} ", prefix, tech.cost, tech.name),
            style,
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("        {}", tech.description),
            Theme::muted_style(),
        )]));
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render the current research progress and completed techs panel
fn render_research_status(frame: &mut Frame, area: Rect, game_state: &GameState) {
    let block = Block::default()
        .title(" Research Status ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let empire = match game_state.empires.get(&game_state.player_empire) {
        Some(e) => e,
        None => return,
    };

    let all = all_techs();
    let mut lines: Vec<Line> = Vec::new();

    // Current research section
    lines.push(Line::from(Span::styled(
        "Current Research",
        Theme::title_style(),
    )));
    lines.push(Line::from(""));

    if let Some(tech_id) = empire.research.current_tech {
        if let Some(tech) = all.iter().find(|t| t.id == tech_id) {
            let progress = empire.research.progress;
            let cost = tech.cost;

            // Calculate research per turn from colonies
            let rp_per_turn: i64 = game_state
                .colonies
                .values()
                .filter(|c| c.owner == game_state.player_empire)
                .map(|c| (c.production as i64 * c.research_pct as i64) / 100)
                .sum();

            let eta = if rp_per_turn > 0 {
                let remaining = cost - progress;
                let turns = (remaining + rp_per_turn - 1) / rp_per_turn;
                format!("~{} turns", turns)
            } else {
                "∞".to_string()
            };

            // Progress bar (capped at 20 chars wide)
            let bar_width = (inner.width.saturating_sub(4) as usize).min(20);
            let filled = if cost > 0 {
                ((progress * bar_width as i64) / cost).min(bar_width as i64) as usize
            } else {
                0
            };

            lines.push(Line::from(vec![Span::styled(
                tech.name,
                Theme::accent_style(),
            )]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}{}]", "=".repeat(filled), " ".repeat(bar_width - filled)),
                    Theme::muted_style(),
                ),
                Span::raw(format!(" {}/{} rp", progress, cost)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("ETA: ", Theme::muted_style()),
                Span::raw(eta),
            ]));
            if rp_per_turn > 0 {
                lines.push(Line::from(vec![
                    Span::styled("Per turn: ", Theme::muted_style()),
                    Span::raw(format!("{} rp", rp_per_turn)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "None — select a technology",
            Theme::muted_style(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Completed", Theme::title_style())));
    lines.push(Line::from(""));

    if empire.research.completed.is_empty() {
        lines.push(Line::from(Span::styled("None yet", Theme::muted_style())));
    } else {
        for tech_id in &empire.research.completed {
            if let Some(tech) = all.iter().find(|t| t.id == *tech_id) {
                lines.push(Line::from(vec![
                    Span::styled("  ✓ ", Theme::accent_style()),
                    Span::styled(tech.name, Theme::default_style()),
                ]));
            }
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
    use ratatui::Terminal;

    #[test]
    fn research_screen_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_research(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn research_screen_with_current_tech() {
        use game_core::{Command, TechId};

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);

        engine.apply_turn(vec![Command::SelectResearch { tech: TechId(1) }]);

        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_research(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn research_screen_cursor_beyond_bounds() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let engine = Engine::new(42);
        let app_state = AppState {
            research_cursor: 999,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_research(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn research_screen_with_completed_tech() {
        use game_core::TechId;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);

        // Manually complete a tech
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .push(TechId(1));

        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_research(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn research_screen_all_techs_completed() {
        use game_core::{all_techs, TechId};

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);

        // Complete all techs
        let all: Vec<TechId> = all_techs().iter().map(|t| t.id).collect();
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .extend(all);

        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_research(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn research_screen_no_empire() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);

        // Remove the player empire to test graceful fallback
        let player_id = engine.state.player_empire;
        engine.state.empires.remove(&player_id);

        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_research(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }
}
