//! Empire overview screen

use crate::components::{
    derive_header_data, meter_line, panel_block, quiet_panel_block, render_footer, render_header,
    section_heading,
};
use crate::layout::{compose_layout, split_main_detail};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    all_techs, yield_model, Colony, ColonyId, ColonySupplyState, ColonyUnrestState, EmpireId,
    GameState, StarId, VictoryProgressValue,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverviewSort {
    #[default]
    Name,
    OrderWarnings,
    ProductionCompletion,
    Population,
}

impl OverviewSort {
    pub fn next(self) -> Self {
        match self {
            OverviewSort::Name => OverviewSort::OrderWarnings,
            OverviewSort::OrderWarnings => OverviewSort::ProductionCompletion,
            OverviewSort::ProductionCompletion => OverviewSort::Population,
            OverviewSort::Population => OverviewSort::Name,
        }
    }

    fn label(self) -> &'static str {
        match self {
            OverviewSort::Name => "Name",
            OverviewSort::OrderWarnings => "Order warnings",
            OverviewSort::ProductionCompletion => "Production ETA",
            OverviewSort::Population => "Population",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpireOverviewSummary {
    pub faction_name: String,
    pub faction_tone: String,
    pub doctrine_summary: String,
    pub credits: i64,
    pub food: i64,
    pub science_per_turn: i64,
    pub maintenance_per_turn: i64,
    pub active_research: String,
    pub fleet_count: usize,
    pub colony_count: usize,
    pub connected_colonies: usize,
    pub isolated_colonies: usize,
    pub unrest_colonies: usize,
    pub victory_lines: Vec<(String, Style)>,
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
    pub employed: u64,
    pub unemployed: u64,
    pub stability: u8,
    pub food_balance: i64,
    pub economic_industry_output: i64,
    pub build_output_per_turn: u64,
    pub current_production: String,
    pub turns_remaining: Option<u64>,
    pub supply: ColonySupplyState,
    pub blockaded: bool,
    pub unrest_state: ColonyUnrestState,
    pub unrest_risk_bp: u16,
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
    let mut connected_colonies = 0usize;
    let mut isolated_colonies = 0usize;
    let mut unrest_colonies = 0usize;

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

        let housing = y.workforce.housing;
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
        let employed = y.workforce.employed;
        let unemployed = y.workforce.unemployed;
        let supply = game_state.colony_supply_state(colony.id);
        let blockaded = game_state.colony_blockade_state(colony.id).is_some();
        let unrest_state = game_state.colony_unrest_state(colony.id);
        let unrest_risk_bp = game_state.colony_rebellion_risk_bp(colony.id);
        let mut warnings = Vec::new();
        if stability_has_yield_penalty(colony.stability) {
            warnings.push("Low stability");
        }
        if unrest_state != ColonyUnrestState::Calm {
            warnings.push("Unrest");
        }
        if food_balance < 0 {
            warnings.push("Food deficit");
        }
        if supply == ColonySupplyState::Isolated {
            warnings.push("Isolated");
        }
        if blockaded {
            warnings.push("Blockaded");
        }
        if housing > 0 && colony.population >= housing {
            warnings.push("Housing full");
        }
        if unemployed > 0 {
            warnings.push("Unemployment");
        }
        if colony.build_queue.is_empty() {
            warnings.push("Queue idle");
        }

        if supply == ColonySupplyState::Connected {
            connected_colonies += 1;
        } else {
            isolated_colonies += 1;
        }
        if unrest_state != ColonyUnrestState::Calm {
            unrest_colonies += 1;
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
            employed,
            unemployed,
            stability: colony.stability,
            food_balance,
            economic_industry_output: y.industry,
            build_output_per_turn,
            current_production,
            turns_remaining,
            supply,
            blockaded,
            unrest_state,
            unrest_risk_bp,
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
    let fleet_maintenance_modifier = empire
        .and_then(|e| e.empire_def)
        .and_then(game_core::empire_definition_by_id)
        .map(|def| def.military_modifiers.fleet_maintenance_modifier_per_fleet)
        .unwrap_or(0);
    let fleet_maintenance =
        ((fleet_count as i64) + (fleet_count as i64 * fleet_maintenance_modifier)).max(0);
    let maintenance_per_turn = colony_maintenance + fleet_maintenance;
    let victory_lines = build_victory_lines(game_state, empire_id);

    EmpireOverviewData {
        summary: EmpireOverviewSummary {
            faction_name: empire
                .and_then(|e| e.empire_def)
                .and_then(game_core::empire_definition_by_id)
                .map(|def| def.name.to_string())
                .unwrap_or_else(|| "Unaligned".to_string()),
            faction_tone: empire
                .and_then(|e| e.empire_def)
                .and_then(game_core::empire_definition_by_id)
                .map(|def| def.tone.to_string())
                .unwrap_or_else(|| "No faction identity".to_string()),
            doctrine_summary: empire
                .and_then(|e| e.empire_def)
                .and_then(game_core::empire_definition_by_id)
                .map(|def| def.doctrine_short_summary())
                .unwrap_or_else(|| "N/A".to_string()),
            credits: empire.map(|e| e.credits).unwrap_or(0),
            food: empire.map(|e| e.food).unwrap_or(0),
            science_per_turn,
            maintenance_per_turn,
            active_research,
            fleet_count,
            colony_count,
            connected_colonies,
            isolated_colonies,
            unrest_colonies,
            victory_lines,
        },
        rows,
    }
}

fn build_victory_lines(game_state: &GameState, empire_id: EmpireId) -> Vec<(String, Style)> {
    let mut lines: Vec<(String, Style)> = Vec::new();
    let settings = game_state
        .scenario
        .as_ref()
        .map(|scenario| scenario.victory_settings.clone())
        .unwrap_or_default();
    for path in game_core::VictoryPath::tie_break_order() {
        let enabled = settings.is_enabled(*path);
        let progress = game_state
            .victory_status
            .progress
            .iter()
            .find(|progress| progress.path == *path);
        let Some(progress) = progress else {
            lines.push((
                format!(
                    "{} [{}] unavailable",
                    path.label(),
                    if enabled { "ON" } else { "OFF" }
                ),
                Theme::muted_style(),
            ));
            continue;
        };
        let mut detail = match &progress.value {
            VictoryProgressValue::Dominion {
                controlled_systems,
                total_colonized_systems,
                control_percent,
                active_major_empires,
            } => format!(
                "{}% systems ({}/{}) · majors {}",
                control_percent, controlled_systems, total_colonized_systems, active_major_empires
            ),
            VictoryProgressValue::Ascendancy {
                completed_victory_techs,
                required_victory_techs,
            } => format!(
                "techs {}/{}",
                completed_victory_techs, required_victory_techs
            ),
            VictoryProgressValue::Prosperity {
                population,
                population_required,
                credits,
                credits_required,
                connected_colonies,
                connected_colonies_required,
                avg_stability,
                avg_stability_required,
                ..
            } => format!(
                "pop {}/{} · cr {}/{} · conn {}/{} · stab {}/{}",
                population,
                population_required,
                credits,
                credits_required,
                connected_colonies,
                connected_colonies_required,
                avg_stability,
                avg_stability_required
            ),
            VictoryProgressValue::Discovery {
                explored_systems_percent,
                required_explored_systems_percent,
                surveyed_planets_percent,
                required_surveyed_planets_percent,
                required_techs_completed,
                required_techs_total,
            } => format!(
                "sys {}/{}% · planets {}/{}% · tech {}/{}",
                explored_systems_percent,
                required_explored_systems_percent,
                surveyed_planets_percent,
                required_surveyed_planets_percent,
                required_techs_completed,
                required_techs_total
            ),
            VictoryProgressValue::Unity {
                contacted_empires,
                contacted_empires_required,
                non_war_relations,
                non_war_relations_required,
                connected_colonies,
                connected_colonies_required,
            } => format!(
                "contact {}/{} · peace {}/{} · connected {}/{}",
                contacted_empires,
                contacted_empires_required,
                non_war_relations,
                non_war_relations_required,
                connected_colonies,
                connected_colonies_required
            ),
        };
        if let Some(leader) = progress.leading_empire {
            detail.push_str(&format!(" · lead E{}", leader.0));
        }
        let status = if !enabled {
            "OFF"
        } else if progress.achieved {
            "DONE"
        } else {
            "ON"
        };
        let percent = progress.progress_percent as usize;
        let filled = percent * 10 / 100;
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(10 - filled));
        let style = if !enabled {
            Theme::muted_style()
        } else if percent >= 75 {
            Theme::success_style()
        } else if percent >= 50 {
            Style::default().fg(Theme::accent2())
        } else {
            Theme::default_style()
        };
        lines.push((
            format!(
                "{} [{}] {} {}% {}",
                path.label(),
                status,
                bar,
                progress.progress_percent,
                detail
            ),
            style,
        ));
    }
    if let (Some(winner), Some(path), Some(turn)) = (
        game_state.victory_status.winner,
        game_state.victory_status.winning_path,
        game_state.victory_status.turn_achieved,
    ) {
        let winner_name = game_state
            .empires
            .get(&winner)
            .map(|empire| empire.name.as_str())
            .unwrap_or("Unknown");
        lines.push((
            format!(
                "Winner: {} (E{}) via {} on turn {}",
                winner_name,
                winner.0,
                path.label(),
                turn
            ),
            Theme::success_style(),
        ));
    } else {
        let player_marker = game_state
            .victory_status
            .progress
            .iter()
            .find(|progress| progress.leading_empire == Some(empire_id))
            .map(|progress| progress.path.label())
            .unwrap_or("none");
        lines.push((
            format!("Player leading path: {}", player_marker),
            Theme::default_style(),
        ));
    }
    lines
}

fn sort_rows(rows: &mut [ColonyOverviewRow], sort: OverviewSort) {
    match sort {
        OverviewSort::Name => rows.sort_by(|a, b| {
            a.system
                .cmp(&b.system)
                .then(a.planet.cmp(&b.planet))
                .then(a.colony_id.0.cmp(&b.colony_id.0))
        }),
        OverviewSort::OrderWarnings => rows.sort_by(|a, b| {
            let a_warn = usize::from(a.unrest_state != ColonyUnrestState::Calm);
            let b_warn = usize::from(b.unrest_state != ColonyUnrestState::Calm);
            b_warn
                .cmp(&a_warn)
                .then(b.unrest_state.cmp(&a.unrest_state))
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
        app_state.overview.sort,
        &app_state.overview.filter,
    );

    let summary_height = if main_area.height < 22 { 8 } else { 11 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(summary_height), Constraint::Min(6)])
        .split(main_area);

    render_summary(frame, chunks[0], &data.summary);
    let compact_content = chunks[1].width < 96 || chunks[1].height < 14;
    if compact_content {
        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(chunks[1]);
        render_colony_table(frame, content[0], app_state, &data.rows);
        render_colony_detail(frame, content[1], app_state, &data.rows, &data.summary);
    } else {
        let (list_area, detail_area) = split_main_detail(chunks[1]);
        render_colony_table(frame, list_area, app_state, &data.rows);
        render_colony_detail(frame, detail_area, app_state, &data.rows, &data.summary);
    }
    let hint = app_state
        .status_message
        .as_deref()
        .unwrap_or("Use Enter to open colony and S to jump to its system for direct actions.");
    render_footer(frame, footer_area, &Screen::EmpireOverview, Some(hint));
}

fn render_summary(frame: &mut Frame, area: Rect, summary: &EmpireOverviewSummary) {
    let block = quiet_panel_block(format!("Strategic Dashboard · {}", summary.faction_name));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let threats = summary.isolated_colonies + summary.unrest_colonies;
    let warning_percent = if summary.colony_count == 0 {
        0
    } else {
        ((threats.saturating_mul(100) / summary.colony_count).min(100)) as u8
    };

    let mut lines = vec![
        section_heading(format!(
            "{} · {}",
            summary.faction_tone, summary.doctrine_summary
        )),
        Line::from(vec![
            Span::styled("Colonies ", Theme::muted_style()),
            Span::styled(summary.colony_count.to_string(), Theme::accent_style()),
            Span::styled("  Fleets ", Theme::muted_style()),
            Span::styled(summary.fleet_count.to_string(), Theme::accent_style()),
            Span::styled("  Research ", Theme::muted_style()),
            Span::styled(summary.active_research.as_str(), Theme::default_style()),
        ]),
        Line::from(vec![
            Span::styled("Economy ", Theme::muted_style()),
            Span::styled(
                format!(
                    "Cr {}  Food {}  Sci {}/t  Maint {}/t",
                    summary.credits, summary.food, summary.science_per_turn, summary.maintenance_per_turn
                ),
                Theme::default_style(),
            ),
        ]),
        meter_line(
            format!(
                "Supply Connected {}/{}",
                summary.connected_colonies, summary.colony_count
            ),
            if summary.colony_count == 0 {
                0
            } else {
                ((summary.connected_colonies.saturating_mul(100) / summary.colony_count).min(100)) as u8
            },
            inner.width.saturating_sub(1),
        ),
        meter_line(
            format!("Threats/Warnings {}", threats),
            warning_percent,
            inner.width.saturating_sub(1),
        ),
    ];
    for (text, style) in summary.victory_lines.iter().take(2) {
        lines.push(Line::from(vec![
            Span::styled("Victory ", Theme::muted_style()),
            Span::styled(text, *style),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn render_colony_table(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    rows: &[ColonyOverviewRow],
) {
    let title = if app_state.overview.filter_input {
        format!(
            " Colonies [Sort:{}]  /{} (typing) ",
            app_state.overview.sort.label(),
            app_state.overview.filter
        )
    } else if app_state.overview.filter.is_empty() {
        format!(" Colonies [Sort:{}] ", app_state.overview.sort.label())
    } else {
        format!(
            " Colonies [Sort:{}] [Filter:{}] ",
            app_state.overview.sort.label(),
            app_state.overview.filter
        )
    };

    let block = panel_block(title, true);
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
    lines.push(Line::from(vec![Span::styled(
        " > Selected colony command target",
        Theme::muted_style(),
    )]));

    let selected = app_state.overview.cursor.min(rows.len().saturating_sub(1));
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

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", prefix), style),
            Span::styled(format!("{}/{} ", row.system, row.planet), style),
            Span::styled(format!("[{}] ", row.role), Theme::muted_style()),
            Span::styled(
                format!("P{}/{} ", row.population, row.housing),
                style,
            ),
            Span::styled(format!("Food {:+} ", row.food_balance), style),
            Span::styled(format!("Ind {} ", row.economic_industry_output), style),
            Span::styled(format!("Q:{} ", row.current_production), style),
            Span::styled(format!("ETA {} ", eta), Theme::muted_style()),
            Span::styled(
                format!("{} ", row.supply.label()),
                if row.supply == ColonySupplyState::Isolated {
                    Theme::warning_style()
                } else {
                    style
                },
            ),
            Span::styled(
                format!("{} ", row.unrest_state.label()),
                if row.unrest_state.is_unrest() {
                    Theme::error_style()
                } else if row.unrest_state == ColonyUnrestState::Strained {
                    Theme::warning_style()
                } else {
                    style
                },
            ),
            Span::styled(
                format!("Warn {}", row.warnings.len()),
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

fn render_colony_detail(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    rows: &[ColonyOverviewRow],
    summary: &EmpireOverviewSummary,
) {
    let block = quiet_panel_block("Colony Detail");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No colony selected.").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let selected = app_state.overview.cursor.min(rows.len().saturating_sub(1));
    let row = &rows[selected];
    let mut lines = vec![
        section_heading(format!("{}/{}", row.system, row.planet)),
        Line::from(vec![
            Span::styled("Role ", Theme::muted_style()),
            Span::styled(row.role.as_str(), Theme::accent_style()),
            Span::styled("  Pop ", Theme::muted_style()),
            Span::raw(format!("{}/{}", row.population, row.housing)),
            Span::styled("  Employed ", Theme::muted_style()),
            Span::raw(format!("{}/{}", row.employed, row.population)),
        ]),
        Line::from(vec![
            Span::styled("Economy ", Theme::muted_style()),
            Span::raw(format!(
                "Food {:+}  Industry {}  Build {}/t",
                row.food_balance, row.economic_industry_output, row.build_output_per_turn
            )),
        ]),
        Line::from(vec![
            Span::styled("Queue ", Theme::muted_style()),
            Span::styled(row.current_production.as_str(), Theme::default_style()),
            Span::styled("  ETA ", Theme::muted_style()),
            Span::raw(
                row.turns_remaining
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "∞".to_string()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Threats ", Theme::muted_style()),
            Span::styled(
                format!(
                    "{} · {} ({:.1}%)",
                    row.supply.label(),
                    row.unrest_state.label(),
                    f64::from(row.unrest_risk_bp) / 100.0
                ),
                if row.unrest_state.is_unrest() || row.supply == ColonySupplyState::Isolated {
                    Theme::warning_style()
                } else {
                    Theme::default_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Warnings ", Theme::muted_style()),
            Span::styled(
                if row.warnings.is_empty() {
                    "None".to_string()
                } else {
                    row.warnings.join(", ")
                },
                if row.warnings.is_empty() {
                    Theme::success_style()
                } else {
                    Theme::warning_style()
                },
            ),
        ]),
    ];
    if let Some((victory, style)) = summary.victory_lines.first() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Victory ", Theme::muted_style()),
            Span::styled(victory, *style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{Command, EmpireDefinitionId, Engine, VictoryPath};
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    fn render_to_string(engine: &Engine) -> String {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let app_state = AppState::default();
        terminal
            .draw(|frame| render_empire_overview(frame, frame.area(), &app_state, &engine.state))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut rendered = String::new();
        for y in 0..30u16 {
            for x in 0..140u16 {
                let ch = buffer
                    .cell((x, y))
                    .and_then(|cell| cell.symbol().chars().next())
                    .unwrap_or(' ');
                rendered.push(ch);
            }
        }
        rendered
    }

    fn render_overview_buffer(
        engine: &Engine,
        app_state: &AppState,
        width: u16,
        height: u16,
    ) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_empire_overview(frame, frame.area(), app_state, &engine.state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

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
    fn overview_applies_faction_fleet_maintenance_modifier() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(7));
        let data = derive_empire_overview(&engine.state, player, OverviewSort::Name, "");
        assert_eq!(data.summary.maintenance_per_turn, 0);
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
    fn unrest_colony_is_highlighted_in_overview() {
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        engine
            .state
            .colony_unrest
            .insert(colony_id, ColonyUnrestState::Unrest);
        engine
            .state
            .colony_unrest_causes
            .insert(colony_id, vec![game_core::UnrestCause::FoodShortage]);
        engine.state.colony_rebellion_risk_bp.insert(colony_id, 320);

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
        assert!(row.warnings.contains(&"Unrest"));
        assert_eq!(data.summary.unrest_colonies, 1);
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
    fn isolated_colony_is_flagged_in_overview() {
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().unwrap();
        engine
            .state
            .colony_supply
            .insert(colony_id, ColonySupplyState::Isolated);

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
        assert!(row.warnings.contains(&"Isolated"));
        assert_eq!(data.summary.isolated_colonies, 1);
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

    #[test]
    fn empire_overview_shows_player_faction_identity() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(6));
        engine.state.empires.get_mut(&player).unwrap().name = "Terran Concord".to_string();
        let doctrine = engine
            .state
            .empires
            .get(&player)
            .and_then(|empire| empire.empire_def)
            .and_then(game_core::empire_definition_by_id)
            .map(|def| def.doctrine_short_summary())
            .expect("player doctrine should exist");
        let rendered = render_to_string(&engine);
        assert!(rendered.contains("Terran Concord"));
        assert!(rendered.contains("science-forward federation"));
        assert!(rendered.contains(&doctrine));
    }

    #[test]
    fn overview_derives_doctrine_summary() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(7));
        let expected = game_core::empire_definition_by_id(EmpireDefinitionId(7))
            .map(|def| def.doctrine_short_summary())
            .expect("definition must exist");

        let data = derive_empire_overview(&engine.state, player, OverviewSort::Name, "");
        assert_eq!(data.summary.doctrine_summary, expected);
    }

    #[test]
    fn overview_shows_enabled_unity_as_on_not_future() {
        let mut engine = Engine::new(42);
        let scenario = engine
            .state
            .scenario
            .as_mut()
            .expect("scenario should exist");
        scenario
            .victory_settings
            .enabled_paths
            .insert(VictoryPath::Unity);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        let lines = build_victory_lines(&engine.state, engine.state.player_empire);
        let unity_line = lines
            .iter()
            .find(|(text, _)| text.starts_with("Unity "))
            .map(|(text, _)| text.as_str())
            .expect("unity line should be present");
        assert!(unity_line.contains("[ON]"));
        assert!(!unity_line.contains("[FUTURE]"));
    }

    #[test]
    fn empire_overview_renders_at_80x24_with_footer() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_overview_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("Strategic Dashboard"));
        assert!(text.contains("Enter"));
    }

    #[test]
    fn empire_overview_renders_at_120x36_with_detail_panel() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_overview_buffer(&engine, &app_state, 120, 36);
        let text = buffer_text(&buffer);
        assert!(text.contains("Colony Detail"));
        assert!(text.contains("Esc"));
    }

    #[test]
    fn empire_overview_selected_row_is_visible() {
        let engine = Engine::new(42);
        let app_state = AppState {
            overview: crate::app::OverviewScreenState {
                cursor: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let buffer = render_overview_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("Selected colony command target"));
        assert!(text.contains(">"));
    }

    #[test]
    fn empire_overview_minimal_state_no_panic_at_80x24() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        engine.state.colonies.retain(|_, c| c.owner != player);
        let app_state = AppState::default();
        let _ = render_overview_buffer(&engine, &app_state, 80, 24);
    }
}
