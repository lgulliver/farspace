//! Sector governance and automation screen.

use crate::AppState;
use crate::components::{
    derive_header_data, quiet_panel_block, render_footer, render_header, section_heading,
};
use crate::layout::{compose_layout, split_main_detail};
use crate::screens::Screen;
use crate::theme::Theme;
use game_core::{
    ColonyAutomation, ColonyId, EmpireId, GameState, SectorDirective, SectorId, yield_model,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

const MIN_WIDTH_FOR_SIDE_BY_SIDE: u16 = 90;
const MIN_HEIGHT_FOR_SIDE_BY_SIDE: u16 = 14;

/// One colony's automation/queue status within a sector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorColonyRow {
    pub colony_id: ColonyId,
    pub label: String,
    pub automation: ColonyAutomation,
    pub current_production: String,
    pub queue_idle: bool,
}

/// Aggregated governance view for a single sector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorGovernanceRow {
    pub sector_id: SectorId,
    pub name: String,
    pub directive: SectorDirective,
    pub colony_count: usize,
    pub automated_count: usize,
    pub idle_count: usize,
    pub industry_per_turn: i64,
    pub science_per_turn: i64,
    pub food_balance: i64,
    pub colonies: Vec<SectorColonyRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SectorGovernanceData {
    pub rows: Vec<SectorGovernanceRow>,
}

/// Build the governance view for `empire_id`. Sectors with no colonies owned by
/// the empire are omitted. Output is fully deterministic (sectors iterate in
/// `SectorId` order; colonies via `colonies_in_sector`).
///
/// Yield totals are the engine's recorded actuals from the last processed turn
/// (`GameState::last_colony_yields`), so they match real income exactly. Before
/// the first turn is processed they fall back to a live estimate (base yield
/// plus flat per-colony bonuses).
pub fn derive_sector_governance(
    game_state: &GameState,
    empire_id: EmpireId,
) -> SectorGovernanceData {
    // Flat per-colony bonuses (tech + faction trait + strategic resource) used
    // only as a pre-first-turn estimate. Once a turn has been processed we read
    // the engine's recorded actual yields instead, which already include the
    // research percentage and isolation/blockade penalties.
    let estimate_bonuses = game_state.per_colony_yield_bonuses(empire_id);

    let mut rows = Vec::new();
    for (&sector_id, sector) in &game_state.sectors {
        let mut colonies = Vec::new();
        let mut automated_count = 0usize;
        let mut idle_count = 0usize;
        let mut industry_per_turn = 0i64;
        let mut science_per_turn = 0i64;
        let mut food_balance = 0i64;

        for colony_id in game_state.colonies_in_sector(sector_id) {
            let Some(colony) = game_state.colonies.get(&colony_id) else {
                continue;
            };
            if colony.owner != empire_id {
                continue;
            }
            let star = game_state.stars.get(&colony.star);
            let planet = star.and_then(|s| s.planets.get(colony.planet_index));
            if let Some(actual) = game_state.last_colony_yields.get(&colony_id) {
                industry_per_turn += actual.industry;
                science_per_turn += actual.science;
                food_balance += actual.food - actual.food_consumed;
            } else {
                // No turn processed yet: estimate from base yield + flat bonuses.
                let y = yield_model::calculate_yield(colony, planet);
                industry_per_turn += y.industry + estimate_bonuses.industry;
                science_per_turn += y.science + estimate_bonuses.science;
                food_balance += (y.food + estimate_bonuses.food) - y.food_consumed;
            }

            let automation = game_state.colony_automation_mode(colony_id);
            if automation == ColonyAutomation::SectorGuided {
                automated_count += 1;
            }
            let queue_idle = colony.build_queue.is_empty();
            if queue_idle {
                idle_count += 1;
            }
            let current_production = colony
                .build_queue
                .first()
                .map(|item| item.name().to_string())
                .unwrap_or_else(|| "Idle".to_string());
            let label = match (star, planet) {
                (Some(s), Some(p)) => format!("{}/{}", s.name, p.name),
                (Some(s), None) => format!("{}/Orbit {}", s.name, colony.planet_index + 1),
                _ => format!("Colony {}", colony_id.0),
            };
            colonies.push(SectorColonyRow {
                colony_id,
                label,
                automation,
                current_production,
                queue_idle,
            });
        }

        if colonies.is_empty() {
            continue;
        }

        rows.push(SectorGovernanceRow {
            sector_id,
            name: sector.name.clone(),
            directive: game_state.sector_directive(sector_id),
            colony_count: colonies.len(),
            automated_count,
            idle_count,
            industry_per_turn,
            science_per_turn,
            food_balance,
            colonies,
        });
    }
    SectorGovernanceData { rows }
}

pub fn render_sector_governance(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);
    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let data = derive_sector_governance(game_state, game_state.player_empire);

    let compact = main_area.width < MIN_WIDTH_FOR_SIDE_BY_SIDE
        || main_area.height < MIN_HEIGHT_FOR_SIDE_BY_SIDE;
    if compact {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_area);
        render_sector_list(frame, chunks[0], app_state, &data.rows);
        render_sector_detail(frame, chunks[1], app_state, &data.rows);
    } else {
        let (list_area, detail_area) = split_main_detail(main_area);
        render_sector_list(frame, list_area, app_state, &data.rows);
        render_sector_detail(frame, detail_area, app_state, &data.rows);
    }

    let hint = app_state.status_message.as_deref().unwrap_or(
        "D cycles the directive · A toggles sector-guided automation for the sector's colonies.",
    );
    render_footer(frame, footer_area, &Screen::SectorGovernance, Some(hint));
}

fn render_sector_list(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    rows: &[SectorGovernanceRow],
) {
    let block = quiet_panel_block("Sector Governance");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No colonized sectors yet.").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let selected = app_state
        .governance
        .cursor
        .min(rows.len().saturating_sub(1));
    let mut lines = vec![Line::from(Span::styled(
        " > Selected Sector · Ind/Sci/Food = last turn",
        Theme::muted_style(),
    ))];

    let max_rows = inner.height.saturating_sub(1) as usize;
    let start = if max_rows == 0 {
        0
    } else {
        selected.saturating_sub(max_rows.saturating_sub(1))
    };
    let end = (start + max_rows).min(rows.len());

    for (idx, row) in rows[start..end].iter().enumerate() {
        let absolute_idx = start + idx;
        let prefix = if absolute_idx == selected { ">" } else { " " };
        let style = if absolute_idx == selected {
            Theme::highlight_style()
        } else {
            Theme::default_style()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", prefix), style),
            Span::styled(format!("{} ", row.name), style),
            Span::styled(
                format!("[{}] ", row.directive.name()),
                Theme::accent_style(),
            ),
            Span::styled(format!("Col {} ", row.colony_count), style),
            Span::styled(
                format!("Auto {}/{} ", row.automated_count, row.colony_count),
                style,
            ),
            Span::styled(
                format!(
                    "Ind {} Sci {} ",
                    row.industry_per_turn, row.science_per_turn
                ),
                style,
            ),
            Span::styled(
                format!("Food {:+} ", row.food_balance),
                if row.food_balance < 0 {
                    Theme::warning_style()
                } else {
                    style
                },
            ),
            Span::styled(
                format!("Idle {}", row.idle_count),
                if row.idle_count > 0 {
                    Theme::warning_style()
                } else {
                    style
                },
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn render_sector_detail(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    rows: &[SectorGovernanceRow],
) {
    let block = quiet_panel_block("Sector Detail");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No sector selected.").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let selected = app_state
        .governance
        .cursor
        .min(rows.len().saturating_sub(1));
    let row = &rows[selected];
    let mut lines = vec![
        section_heading(row.name.clone()),
        Line::from(vec![
            Span::styled("Directive ", Theme::muted_style()),
            Span::styled(row.directive.name(), Theme::accent_style()),
        ]),
        Line::from(vec![Span::styled(
            row.directive.description(),
            Theme::muted_style(),
        )]),
        Line::from(vec![
            Span::styled("Output (last turn) ", Theme::muted_style()),
            Span::raw(format!(
                "Industry {}  Science {}  Food {:+}",
                row.industry_per_turn, row.science_per_turn, row.food_balance
            )),
        ]),
        Line::from(Span::styled(
            "Actuals from the last processed turn; updates on End Turn.",
            Theme::muted_style(),
        )),
        Line::from(vec![
            Span::styled("Automation ", Theme::muted_style()),
            Span::raw(format!(
                "{}/{} sector-guided",
                row.automated_count, row.colony_count
            )),
        ]),
        Line::from(""),
        section_heading("Colonies"),
    ];

    for colony in &row.colonies {
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", colony.label), Theme::default_style()),
            Span::styled(
                format!("[{}] ", colony.automation.name()),
                if colony.automation == ColonyAutomation::SectorGuided {
                    Theme::accent_style()
                } else {
                    Theme::muted_style()
                },
            ),
            Span::styled(
                format!("Q:{}", colony.current_production),
                if colony.queue_idle {
                    Theme::warning_style()
                } else {
                    Theme::default_style()
                },
            ),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(ratatui::widgets::Wrap { trim: false }),
        inner,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{Command, Engine};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn render_buffer(engine: &Engine, app_state: &AppState, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_sector_governance(frame, frame.area(), app_state, &engine.state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn derive_lists_only_colonized_player_sectors() {
        let engine = Engine::new(42);
        let data = derive_sector_governance(&engine.state, engine.state.player_empire);
        assert!(!data.rows.is_empty());
        for row in &data.rows {
            assert!(row.colony_count >= 1);
        }
    }

    #[test]
    fn derive_reflects_directive_and_automation() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let colony = ColonyId(1);
        let sector = engine.state.colony_sector(colony).unwrap();
        engine.apply_turn(vec![
            Command::SetSectorDirective {
                sector,
                directive: SectorDirective::Research,
            },
            Command::SetColonyAutomation {
                colony,
                automation: ColonyAutomation::SectorGuided,
            },
        ]);
        let data = derive_sector_governance(&engine.state, player);
        let row = data
            .rows
            .iter()
            .find(|r| r.sector_id == sector)
            .expect("player sector should be present");
        assert_eq!(row.directive, SectorDirective::Research);
        assert_eq!(row.automated_count, 1);
    }

    #[test]
    fn derive_is_deterministic() {
        let engine = Engine::new(42);
        let a = derive_sector_governance(&engine.state, engine.state.player_empire);
        let b = derive_sector_governance(&engine.state, engine.state.player_empire);
        assert_eq!(a, b);
    }

    #[test]
    fn derive_includes_strategic_resource_per_colony_bonus() {
        // Regression: sector science totals must include the same flat
        // per-colony bonuses the engine applies (here, a strategic resource),
        // not just the base pop/jobs yield. Granting QuantumCrystals adds
        // +1 science per colony, so the summed total must rise by exactly the
        // player's colony count.
        use game_core::StrategicResource;
        use std::collections::BTreeMap;

        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;

        let baseline: i64 = derive_sector_governance(&engine.state, player)
            .rows
            .iter()
            .map(|r| r.science_per_turn)
            .sum();

        let player_colonies = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == player)
            .count() as i64;
        assert!(player_colonies > 0, "fixture should have player colonies");

        let mut by_resource = BTreeMap::new();
        by_resource.insert(StrategicResource::QuantumCrystals, 1);
        engine
            .state
            .empire_resource_access
            .insert(player, by_resource);

        let boosted: i64 = derive_sector_governance(&engine.state, player)
            .rows
            .iter()
            .map(|r| r.science_per_turn)
            .sum();

        assert_eq!(boosted - baseline, player_colonies);
    }

    #[test]
    fn derive_uses_engine_recorded_yields_after_a_turn() {
        // After a processed turn the screen must report the engine's actual
        // per-colony yields (which already fold in penalties), not a re-derived
        // estimate. The summed sector science must equal the sum of the
        // recorded yields for the player's colonies.
        let mut engine = Engine::new(42);
        engine.apply_turn(vec![Command::EndTurn]);
        let player = engine.state.player_empire;

        let expected: i64 = engine
            .state
            .colonies
            .iter()
            .filter(|(_, c)| c.owner == player)
            .filter_map(|(id, _)| engine.state.last_colony_yields.get(id))
            .map(|y| y.science)
            .sum();

        let total: i64 = derive_sector_governance(&engine.state, player)
            .rows
            .iter()
            .map(|r| r.science_per_turn)
            .sum();

        assert_eq!(total, expected);
        assert!(!engine.state.last_colony_yields.is_empty());
    }

    #[test]
    fn renders_at_80x24_with_footer_hint() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("Sector Governance"));
        assert!(text.contains("Directive"));
    }

    #[test]
    fn renders_at_120x36_with_detail_panel() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 120, 36);
        let text = buffer_text(&buffer);
        assert!(text.contains("Sector Detail"));
        assert!(text.contains("Colonies"));
        assert!(text.contains("last turn"));
    }

    #[test]
    fn renders_safely_with_no_player_colonies() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        engine.state.colonies.retain(|_, c| c.owner != player);
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("No colonized sectors"));
    }
}
