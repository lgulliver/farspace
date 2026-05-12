//! Empire overview screen

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{all_techs, yield_model, Colony, ColonyId, EmpireId, GameState, StarId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverviewSort {
    #[default]
    Name,
    StabilityWarnings,
    ProductionCompletion,
    Population,
}

impl OverviewSort {
    pub fn next(self) -> Self {
        match self {
            OverviewSort::Name => OverviewSort::StabilityWarnings,
            OverviewSort::StabilityWarnings => OverviewSort::ProductionCompletion,
            OverviewSort::ProductionCompletion => OverviewSort::Population,
            OverviewSort::Population => OverviewSort::Name,
        }
    }

    fn label(self) -> &'static str {
        match self {
            OverviewSort::Name => "Name",
            OverviewSort::StabilityWarnings => "Stability warnings",
            OverviewSort::ProductionCompletion => "Production ETA",
            OverviewSort::Population => "Population",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpireOverviewSummary {
    pub credits: i64,
    pub food: i64,
    pub science_per_turn: i64,
    pub maintenance_per_turn: i64,
    pub active_research: String,
    pub fleet_count: usize,
    pub colony_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColonyOverviewRow {
    pub colony_id: ColonyId,
    pub star_id: StarId,
    pub planet_index: usize,
    pub system: String,
    pub planet: String,
    pub role: String,
    pub population: u64,
    pub housing: u64,
    pub stability: u8,
    pub food_balance: i64,
    pub economic_industry_output: i64,
    pub build_output_per_turn: u64,
    pub current_production: String,
    pub turns_remaining: Option<u64>,
    pub warnings: Vec<&'static str>,
}

impl ColonyOverviewRow {
    fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

fn colony_build_output_per_turn(colony: &Colony) -> u64 {
    colony
        .build_queue
        .first()
        .map(|item| {
            colony.production
                + if item.is_ship() {
                    colony.role.ship_production_bonus()
                } else {
                    0
                }
        })
        .unwrap_or(colony.production)
}

fn stability_has_yield_penalty(stability: u8) -> bool {
    (stability as i64 - 100) / 10 < 0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpireOverviewData {
    pub summary: EmpireOverviewSummary,
    pub rows: Vec<ColonyOverviewRow>,
}

pub fn derive_empire_overview(
    game_state: &GameState,
    empire_id: EmpireId,
    sort: OverviewSort,
    filter: &str,
) -> EmpireOverviewData {
    let mut science_per_turn = 0i64;
    let mut colony_maintenance = 0i64;
    let mut rows = Vec::new();

    for colony in game_state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id)
    {
        let star = game_state.stars.get(&colony.star);
        let planet = star.and_then(|s| s.planets.get(colony.planet_index));
        let y = yield_model::calculate_yield(colony, planet);
        science_per_turn += y.science;
        colony_maintenance += y.maintenance;

        let housing = planet.map(|p| p.size.base_capacity()).unwrap_or(0);
        let current_production = colony
            .build_queue
            .first()
            .map(|item| item.name().to_string())
            .unwrap_or_else(|| "Idle".to_string());

        let build_output_per_turn = colony_build_output_per_turn(colony);

        let turns_remaining = colony.build_queue.first().and_then(|item| {
            let remaining = item.cost().saturating_sub(colony.accumulated_production);
            let per_turn = build_output_per_turn;
            if per_turn == 0 {
                None
            } else {
                Some(remaining.div_ceil(per_turn))
            }
        });

        let food_balance = y.food - y.food_consumed;
        let mut warnings = Vec::new();
        if stability_has_yield_penalty(colony.stability) {
            warnings.push("Low stability");
        }
        if food_balance < 0 {
            warnings.push("Food deficit");
        }
        if housing > 0 && colony.population >= housing {
            warnings.push("Housing full");
        }
        if colony.build_queue.is_empty() {
            warnings.push("Queue idle");
        }

        rows.push(ColonyOverviewRow {
            colony_id: colony.id,
            star_id: colony.star,
            planet_index: colony.planet_index,
            system: star
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("Star {}", colony.star.0)),
            planet: planet
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Orbit {}", colony.planet_index + 1)),
            role: colony.role.name().to_string(),
            population: colony.population,
            housing,
            stability: colony.stability,
            food_balance,
            economic_industry_output: y.industry,
            build_output_per_turn,
            current_production,
            turns_remaining,
            warnings,
        });
    }

    let colony_count = rows.len();
    sort_rows(&mut rows, sort);

    let filter = filter.trim().to_ascii_lowercase();
    if !filter.is_empty() {
        rows.retain(|row| {
            row.system.to_ascii_lowercase().contains(&filter)
                || row.planet.to_ascii_lowercase().contains(&filter)
                || row.role.to_ascii_lowercase().contains(&filter)
        });
    }

    let empire = game_state.empires.get(&empire_id);
    let active_research = empire
        .and_then(|e| e.research.current_tech)
        .and_then(|tid| {
            all_techs()
                .iter()
                .find(|t| t.id == tid)
                .map(|t| t.name.to_string())
        })
        .unwrap_or_else(|| "None".to_string());

    let fleet_count = game_state
        .fleets
        .values()
        .filter(|f| f.owner == empire_id)
        .count();
    let maintenance_per_turn = colony_maintenance + fleet_count as i64;

    EmpireOverviewData {
        summary: EmpireOverviewSummary {
            credits: empire.map(|e| e.credits).unwrap_or(0),
            food: empire.map(|e| e.food).unwrap_or(0),
            science_per_turn,
            maintenance_per_turn,
            active_research,
            fleet_count,
            colony_count,
        },
        rows,
    }
}

fn sort_rows(rows: &mut [ColonyOverviewRow], sort: OverviewSort) {
    match sort {
        OverviewSort::Name => rows.sort_by(|a, b| {
            a.system
                .cmp(&b.system)
                .then(a.planet.cmp(&b.planet))
                .then(a.colony_id.0.cmp(&b.colony_id.0))
        }),
        OverviewSort::StabilityWarnings => rows.sort_by(|a, b| {
            let a_warn = usize::from(a.stability < 100);
            let b_warn = usize::from(b.stability < 100);
            b_warn
                .cmp(&a_warn)
                .then(a.stability.cmp(&b.stability))
                .then(b.warning_count().cmp(&a.warning_count()))
                .then(a.system.cmp(&b.system))
                .then(a.planet.cmp(&b.planet))
                .then(a.colony_id.0.cmp(&b.colony_id.0))
        }),
        OverviewSort::ProductionCompletion => rows.sort_by(|a, b| {
            a.turns_remaining
                .unwrap_or(u64::MAX)
                .cmp(&b.turns_remaining.unwrap_or(u64::MAX))
                .then(a.system.cmp(&b.system))
                .then(a.planet.cmp(&b.planet))
                .then(a.colony_id.0.cmp(&b.colony_id.0))
        }),
        OverviewSort::Population => rows.sort_by(|a, b| {
            b.population
                .cmp(&a.population)
                .then(a.system.cmp(&b.system))
                .then(a.planet.cmp(&b.planet))
                .then(a.colony_id.0.cmp(&b.colony_id.0))
        }),
    }
}

pub fn render_empire_overview(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);
    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let data = derive_empire_overview(
        game_state,
        game_state.player_empire,
        app_state.overview_sort,
        &app_state.overview_filter,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(8)])
        .split(main_area);

    render_summary(frame, chunks[0], &data.summary);
    render_colony_table(frame, chunks[1], app_state, &data.rows);
    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Use Enter to open colony and S to jump to its system for direct actions.");
    render_footer(frame, footer_area, &Screen::EmpireOverview, Some(hint));
}

fn render_summary(frame: &mut Frame, area: Rect, summary: &EmpireOverviewSummary) {
    let block = Block::default()
        .title(" Empire Summary ")
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let line = Line::from(vec![
        Span::styled("Credits ", Theme::muted_style()),
        Span::styled(format!("{}", summary.credits), Theme::default_style()),
        Span::raw("  "),
        Span::styled("Food ", Theme::muted_style()),
        Span::styled(format!("{}", summary.food), Theme::default_style()),
        Span::raw("  "),
        Span::styled("Science ", Theme::muted_style()),
        Span::styled(
            format!("{}/t", summary.science_per_turn),
            Theme::default_style(),
        ),
        Span::raw("  "),
        Span::styled("Maint ", Theme::muted_style()),
        Span::styled(
            format!("{}/t", summary.maintenance_per_turn),
            Theme::default_style(),
        ),
        Span::raw("  "),
        Span::styled("Research ", Theme::muted_style()),
        Span::styled(summary.active_research.as_str(), Theme::accent_style()),
        Span::raw("  "),
        Span::styled("Fleets ", Theme::muted_style()),
        Span::styled(format!("{}", summary.fleet_count), Theme::default_style()),
        Span::raw("  "),
        Span::styled("Colonies ", Theme::muted_style()),
        Span::styled(format!("{}", summary.colony_count), Theme::default_style()),
    ]);
    frame.render_widget(Paragraph::new(line).style(Theme::default_style()), inner);
}

fn render_colony_table(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    rows: &[ColonyOverviewRow],
) {
    let title = if app_state.overview_filter_input {
        format!(
            " Colonies [Sort:{}]  /{} (typing) ",
            app_state.overview_sort.label(),
            app_state.overview_filter
        )
    } else if app_state.overview_filter.is_empty() {
        format!(" Colonies [Sort:{}] ", app_state.overview_sort.label())
    } else {
        format!(
            " Colonies [Sort:{}] [Filter:{}] ",
            app_state.overview_sort.label(),
            app_state.overview_filter
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No colonies match the current filter.").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" ", Theme::default_style()),
        Span::styled("System/Planet", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Role", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Pop/Housing", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Stab", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Food", Theme::title_style()),
        Span::raw("  "),
        Span::styled("EcoInd", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Build", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Production", Theme::title_style()),
        Span::raw("  "),
        Span::styled("ETA", Theme::title_style()),
        Span::raw("  "),
        Span::styled("Warnings", Theme::title_style()),
    ]));

    let selected = app_state.overview_cursor.min(rows.len().saturating_sub(1));
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
        let eta = if row.current_production == "Idle" {
            "-".to_string()
        } else {
            row.turns_remaining
                .map(|v| v.to_string())
                .unwrap_or_else(|| "∞".to_string())
        };
        let warnings = if row.warnings.is_empty() {
            "-".to_string()
        } else {
            row.warnings.join(",")
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", prefix), style),
            Span::styled(format!("{}/{}", row.system, row.planet), style),
            Span::raw("  "),
            Span::styled(row.role.as_str(), style),
            Span::raw("  "),
            Span::styled(format!("{}/{}", row.population, row.housing), style),
            Span::raw("  "),
            Span::styled(format!("{}", row.stability), style),
            Span::raw("  "),
            Span::styled(format!("{:+}", row.food_balance), style),
            Span::raw("  "),
            Span::styled(format!("{}", row.economic_industry_output), style),
            Span::raw("  "),
            Span::styled(format!("{}", row.build_output_per_turn), style),
            Span::raw("  "),
            Span::styled(row.current_production.as_str(), style),
            Span::raw("  "),
            Span::styled(eta, style),
            Span::raw("  "),
            Span::styled(
                warnings,
                if row.warnings.is_empty() {
                    style
                } else {
                    Theme::warning_style()
                },
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{Command, Engine};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn overview_derives_correct_colony_count() {
        let engine = Engine::new(42);
        let data = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::Name,
            "",
        );
        assert_eq!(data.summary.colony_count, 1);
    }

    #[test]
    fn overview_derives_correct_fleet_count() {
        let engine = Engine::new(42);
        let data = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::Name,
            "",
        );
        let expected = engine
            .state
            .fleets
            .values()
            .filter(|f| f.owner == engine.state.player_empire)
            .count();
        assert_eq!(data.summary.fleet_count, expected);
    }

    #[test]
    fn overview_derives_empire_resource_summary() {
        let engine = Engine::new(42);
        let data = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::Name,
            "",
        );
        assert_eq!(data.summary.credits, 100);
        assert_eq!(data.summary.food, 0);
        assert_eq!(data.summary.science_per_turn, 5);
        let expected_maintenance = engine
            .state
            .fleets
            .values()
            .filter(|f| f.owner == engine.state.player_empire)
            .count() as i64;
        assert_eq!(data.summary.maintenance_per_turn, expected_maintenance);
    }

    #[test]
    fn colonies_with_warnings_are_flagged() {
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
        colony.stability = 89;
        colony.build_queue.clear();

        let data = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::Name,
            "",
        );
        let row = data
            .rows
            .iter()
            .find(|r| r.colony_id == colony_id)
            .expect("colony row should exist");
        assert!(row.warnings.contains(&"Low stability"));
        assert!(row.warnings.contains(&"Queue idle"));
    }

    #[test]
    fn minor_stability_drop_without_penalty_is_not_flagged() {
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
        colony.stability = 95;

        let data = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::Name,
            "",
        );
        let row = data
            .rows
            .iter()
            .find(|r| r.colony_id == colony_id)
            .expect("colony row should exist");
        assert!(!row.warnings.contains(&"Low stability"));
    }

    #[test]
    fn sorting_is_deterministic() {
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        let mut extra = engine.state.colonies[&colony_id].clone();
        extra.id = game_core::ColonyId(engine.state.next_colony_id);
        engine.state.next_colony_id += 1;
        extra.population += 2;
        engine.state.colonies.insert(extra.id, extra);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: game_core::BuildItem::Structure(game_core::BuildingType::AquacultureBay),
        }]);

        let a = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::ProductionCompletion,
            "",
        );
        let b = derive_empire_overview(
            &engine.state,
            engine.state.player_empire,
            OverviewSort::ProductionCompletion,
            "",
        );
        assert_eq!(a.rows, b.rows);
    }

    #[test]
    fn empty_no_colony_state_renders_safely() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        engine.state.colonies.retain(|_, c| c.owner != player);
        for star in engine.state.stars.values_mut() {
            for planet in &mut star.planets {
                planet.colony = None;
            }
        }
        let app_state = AppState::default();
        terminal
            .draw(|frame| render_empire_overview(frame, frame.area(), &app_state, &engine.state))
            .unwrap();
    }
}
