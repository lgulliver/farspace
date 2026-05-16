//! Research screen

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    all_techs, is_tech_available, tech_by_id, tech_yield_bonus_per_colony, Empire, GameState,
    TechDomain, TechRecord, YieldType,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TechStatus {
    Available,
    Locked,
    Active,
    Completed,
}

impl TechStatus {
    fn label(self) -> &'static str {
        match self {
            TechStatus::Available => "Available",
            TechStatus::Locked => "Locked",
            TechStatus::Active => "Active",
            TechStatus::Completed => "Completed",
        }
    }
}

const TECH_DOMAIN_ORDER: [TechDomain; 5] = [
    TechDomain::Exploration,
    TechDomain::Engineering,
    TechDomain::Military,
    TechDomain::Economy,
    TechDomain::Biology,
];

pub(crate) fn ordered_research_techs() -> Vec<&'static TechRecord> {
    let all = all_techs();
    TECH_DOMAIN_ORDER
        .into_iter()
        .flat_map(|domain| all.iter().filter(move |t| t.domain == domain))
        .collect()
}

/// Render the research screen
pub fn render_research(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    // Split: 60% list, 40% detail/progress
    let (list_area, right_area) = split_horizontal(main_area, 60);

    render_tech_list(frame, list_area, app_state, game_state);
    render_research_detail_and_status(frame, right_area, app_state, game_state);

    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Pick an available tech with Enter so science has an active target.");
    render_footer(frame, footer_area, &Screen::Research, Some(hint));
}

fn tech_status(game_state: &GameState, tech: &TechRecord) -> TechStatus {
    let Some(empire) = game_state.empires.get(&game_state.player_empire) else {
        return TechStatus::Locked;
    };
    if empire.research.completed.contains(&tech.id) {
        return TechStatus::Completed;
    }
    if empire.research.current_tech == Some(tech.id) {
        return TechStatus::Active;
    }
    if is_tech_available(&empire.research.completed, tech.id) {
        TechStatus::Available
    } else {
        TechStatus::Locked
    }
}

/// Render grouped technologies with status tags.
fn render_tech_list(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let block = Block::default()
        .title(" Technology Tree ")
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ordered = ordered_research_techs();
    if ordered.is_empty() {
        frame.render_widget(Paragraph::new("No technologies defined."), inner);
        return;
    }
    let cursor = app_state.research.cursor % ordered.len();
    let selected_id = ordered[cursor].id;
    let mut lines = Vec::new();

    for domain in TECH_DOMAIN_ORDER {
        let domain_techs: Vec<_> = ordered
            .iter()
            .copied()
            .filter(|t| t.domain == domain)
            .collect();
        if domain_techs.is_empty() {
            continue;
        }
        lines.push(Line::from(Span::styled(
            format!(" {} ", domain.name()),
            Theme::title_style(),
        )));
        for tech in domain_techs {
            let status = tech_status(game_state, tech);
            let is_selected = tech.id == selected_id;
            let prefix = if is_selected { ">" } else { " " };
            let status_tag = match status {
                TechStatus::Available => "[Available]",
                TechStatus::Locked => "[Locked]",
                TechStatus::Active => "[Active]",
                TechStatus::Completed => "[Completed]",
            };
            let style = if is_selected {
                Theme::highlight_style()
            } else if matches!(status, TechStatus::Active | TechStatus::Completed) {
                Theme::accent_style()
            } else if status == TechStatus::Locked {
                Theme::muted_style()
            } else {
                Theme::default_style()
            };
            lines.push(Line::from(Span::styled(
                format!(
                    " {} [{:>3}rp] {} ({}) {}",
                    prefix,
                    tech.cost,
                    tech.name,
                    tech.tier.label(),
                    status_tag
                ),
                style,
            )));
        }
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

/// Render selected-tech details and active/completed summary.
fn render_research_detail_and_status(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let chunks =
        Layout::vertical([Constraint::Percentage(62), Constraint::Percentage(38)]).split(area);
    render_selected_tech_detail(frame, chunks[0], app_state, game_state);
    render_research_status(frame, chunks[1], game_state);
}

fn render_selected_tech_detail(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let block = Block::default()
        .title(" Technology Detail ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ordered = ordered_research_techs();
    if ordered.is_empty() {
        frame.render_widget(Paragraph::new("No technology selected."), inner);
        return;
    }
    let tech = ordered[app_state.research.cursor % ordered.len()];
    let status = tech_status(game_state, tech);
    let empire = game_state.empires.get(&game_state.player_empire);
    let completed = empire
        .map(|e| e.research.completed.as_slice())
        .unwrap_or(&[]);
    let mut lines = vec![
        Line::from(Span::styled(tech.name, Theme::accent_style())),
        Line::from(Span::styled(
            format!(
                "{} • {} • {} rp",
                tech.domain.name(),
                tech.tier.label(),
                tech.cost
            ),
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            format!("Status: {}", status.label()),
            Theme::default_style(),
        )),
        Line::from(""),
        Line::from(tech.description),
        Line::from(""),
        Line::from(Span::styled("Prerequisites", Theme::title_style())),
    ];
    if tech.prerequisites.is_empty() {
        lines.push(Line::from(Span::styled("  None", Theme::muted_style())));
    } else {
        for req in tech.prerequisites {
            let req_name = tech_by_id(*req).map(|t| t.name).unwrap_or("Unknown Tech");
            let done = completed.contains(req);
            let marker = if done { "✓" } else { "×" };
            let style = if done {
                Theme::accent_style()
            } else {
                Theme::muted_style()
            };
            lines.push(Line::from(Span::styled(
                format!("  {} {}", marker, req_name),
                style,
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Unlocks", Theme::title_style())));
    if tech.unlocks.is_empty() {
        lines.push(Line::from(Span::styled("  None", Theme::muted_style())));
    } else {
        for unlock in tech.unlocks {
            lines.push(Line::from(format!("  • {}", unlock.description())));
        }
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

/// Render active progress and completed-tech summary.
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

            // Calculate research per turn using the same model path as the engine.
            let rp_per_turn = player_research_per_turn(game_state, empire);

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
    lines.push(Line::from(Span::styled(
        "Completed Techs",
        Theme::title_style(),
    )));
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

fn player_research_per_turn(game_state: &GameState, empire: &Empire) -> i64 {
    let science_bonus = tech_yield_bonus_per_colony(&empire.research.completed, YieldType::Science);
    game_state
        .colonies
        .values()
        .filter(|c| c.owner == game_state.player_empire)
        .map(|colony| {
            let planet = game_state
                .stars
                .get(&colony.star)
                .and_then(|s| s.planets.get(colony.planet_index));
            game_core::yield_model::calculate_yield(colony, planet).science + science_bonus
        })
        .sum()
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
            research: crate::app::ResearchScreenState { cursor: 999 },
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
