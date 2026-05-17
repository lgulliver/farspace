//! Research screen

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::state::{TechRarity, TechTag};
use game_core::{
    all_techs, is_tech_available, tech_by_id, tech_yield_bonus_per_colony, Empire, GameState,
    TechDomain, TechRecord, TechTier, YieldType,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TechStatus {
    Available,
    Locked,
    Queued,
    Active,
    Completed,
}

impl TechStatus {
    fn label(self) -> &'static str {
        match self {
            TechStatus::Available => "Available",
            TechStatus::Locked => "Locked",
            TechStatus::Queued => "Queued",
            TechStatus::Active => "Active",
            TechStatus::Completed => "Completed",
        }
    }
}

const TECH_DOMAIN_ORDER: [TechDomain; 6] = [
    TechDomain::Exploration,
    TechDomain::Engineering,
    TechDomain::Military,
    TechDomain::Society,
    TechDomain::Economy,
    TechDomain::Biology,
];
pub(crate) const RESEARCH_DOMAIN_FILTER_COUNT: usize = TECH_DOMAIN_ORDER.len() + 1;
pub(crate) const RESEARCH_STATUS_FILTER_COUNT: usize = 6; // all + 5 statuses

fn tech_domain_sort_index(domain: TechDomain) -> usize {
    TECH_DOMAIN_ORDER
        .iter()
        .position(|candidate| *candidate == domain)
        .unwrap_or(TECH_DOMAIN_ORDER.len())
}

fn tech_rarity_style(rarity: TechRarity) -> Style {
    match rarity {
        TechRarity::Common => Theme::muted_style(),
        TechRarity::Uncommon => Theme::success_style(),
        TechRarity::Rare => Style::default()
            .fg(Theme::accent2())
            .add_modifier(Modifier::BOLD),
        TechRarity::Breakthrough => Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD),
        TechRarity::Dangerous => Style::default()
            .fg(Theme::warning())
            .add_modifier(Modifier::BOLD),
    }
}

fn tech_tag_style(tag: TechTag) -> Style {
    match tag {
        TechTag::Survey | TechTag::Sensors | TechTag::Hyperspace | TechTag::SectorMapping => {
            Theme::accent_style()
        }
        TechTag::Colonization
        | TechTag::Growth
        | TechTag::Housing
        | TechTag::Food
        | TechTag::Terraforming => Theme::success_style(),
        TechTag::Shipyard
        | TechTag::Orbital
        | TechTag::Production
        | TechTag::Supply
        | TechTag::Trade
        | TechTag::Logistics => Style::default().fg(Theme::accent2()),
        TechTag::Weapon
        | TechTag::Defense
        | TechTag::Invasion
        | TechTag::Blockade
        | TechTag::Command
        | TechTag::ShipClass => Theme::warning_style(),
        TechTag::Diplomacy | TechTag::Precursor | TechTag::Gateway => {
            Style::default().fg(Color::LightMagenta)
        }
        TechTag::Megastructure | TechTag::Crisis => Style::default()
            .fg(Theme::warning())
            .add_modifier(Modifier::BOLD),
        TechTag::EspionageFuture | TechTag::PopulationJobsFuture | TechTag::Stability => {
            Theme::muted_style()
        }
    }
}

fn bracket_tag_span(label: impl Into<String>, style: Style) -> Span<'static> {
    Span::styled(format!("[{}]", label.into()), style)
}

fn join_tag_spans(tags: &[TechTag]) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled("Tags: ", Theme::muted_style())];
    for (idx, tag) in tags.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(bracket_tag_span(tag.label(), tech_tag_style(*tag)));
    }
    spans
}

pub(crate) fn ordered_research_techs() -> Vec<&'static TechRecord> {
    let all = all_techs();
    let mut ordered: Vec<_> = TECH_DOMAIN_ORDER
        .into_iter()
        .flat_map(|domain| all.iter().filter(move |t| t.domain == domain))
        .collect();
    ordered.sort_by_key(|tech| {
        (
            tech_domain_sort_index(tech.domain),
            tech.display_order,
            tech.id,
        )
    });
    ordered
}

pub(crate) fn filtered_research_techs<'a>(
    app_state: &'a AppState,
    game_state: &'a GameState,
) -> Vec<&'static TechRecord> {
    let domain_filter = app_state.research.domain_filter;
    let era_filter = app_state.research.era_filter;
    let status_filter = app_state.research.status_filter;
    let query = app_state.research.query.trim().to_ascii_lowercase();

    ordered_research_techs()
        .into_iter()
        .filter(|tech| {
            if domain_filter == 0 {
                true
            } else {
                TECH_DOMAIN_ORDER
                    .get(domain_filter.saturating_sub(1))
                    .is_some_and(|domain| tech.domain == *domain)
            }
        })
        .filter(|tech| {
            if era_filter == 0 {
                true
            } else {
                let tier = match era_filter {
                    1 => TechTier::I,
                    2 => TechTier::II,
                    3 => TechTier::III,
                    4 => TechTier::IV,
                    5 => TechTier::V,
                    6 => TechTier::VI,
                    _ => unreachable!("era_filter is clamped to 0..=6"),
                };
                tech.tier == tier
            }
        })
        .filter(|tech| {
            if status_filter == 0 {
                true
            } else {
                let status = tech_status(game_state, tech);
                matches!(
                    (status_filter, status),
                    (1, TechStatus::Available)
                        | (2, TechStatus::Locked)
                        | (3, TechStatus::Queued)
                        | (4, TechStatus::Active)
                        | (5, TechStatus::Completed)
                )
            }
        })
        .filter(|tech| tech_status(game_state, tech) != TechStatus::Locked)
        .filter(|tech| {
            if query.is_empty() {
                return true;
            }
            let tags = tech
                .tags
                .iter()
                .map(TechTag::label)
                .collect::<Vec<_>>()
                .join(" ");
            let text = format!(
                "{} {} {} {}",
                tech.name,
                tech.description,
                tech.domain.name(),
                tags
            )
            .to_ascii_lowercase();
            text.contains(&query)
        })
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
    if empire.research.queue.contains(&tech.id) {
        return TechStatus::Queued;
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

    let ordered = filtered_research_techs(app_state, game_state);
    if ordered.is_empty() {
        frame.render_widget(
            Paragraph::new("No technologies match current filters."),
            inner,
        );
        return;
    }
    let cursor = app_state.research.cursor % ordered.len();
    let selected_id = ordered[cursor].id;
    let mut lines = Vec::new();
    let mut selected_line_idx = 0usize;

    let domain_filter_label = if app_state.research.domain_filter == 0 {
        "All".to_string()
    } else {
        TECH_DOMAIN_ORDER[app_state.research.domain_filter.saturating_sub(1)]
            .name()
            .to_string()
    };
    let status_filter_label = match app_state.research.status_filter {
        0 => "All".to_string(),
        1 => "Available".to_string(),
        2 => "Locked".to_string(),
        3 => "Queued".to_string(),
        4 => "Active".to_string(),
        _ => "Completed".to_string(),
    };
    let query_label = if app_state.research.query.is_empty() {
        "none".to_string()
    } else {
        app_state.research.query.clone()
    };
    lines.push(Line::from(Span::styled(
        format!(
            "Filters: Domain={} · Status={} · Search={}",
            domain_filter_label, status_filter_label, query_label
        ),
        Theme::muted_style(),
    )));
    lines.push(Line::from(""));

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
                TechStatus::Queued => "[Queued]",
                TechStatus::Active => "[Active]",
                TechStatus::Completed => "[Completed]",
            };
            let rarity_tag = match tech.rarity.label() {
                "Rare" => "Rare",
                "Breakthrough" => "Breakthrough",
                "Dangerous" => "Dangerous",
                _ => "",
            };
            let hook_tag = if tech.future_hook { "Planned" } else { "" };
            let style = if is_selected {
                Theme::highlight_style()
            } else if matches!(status, TechStatus::Active | TechStatus::Completed) {
                Theme::accent_style()
            } else if status == TechStatus::Queued {
                Style::default().fg(Theme::accent2())
            } else if status == TechStatus::Locked {
                Theme::muted_style()
            } else {
                Theme::default_style()
            };
            lines.push(Line::from(Span::styled(
                format!(
                    " {} [{:>3}rp] {} {}",
                    prefix, tech.cost, tech.name, status_tag
                ),
                style,
            )));
            if is_selected {
                selected_line_idx = lines.len().saturating_sub(1);
            }
            if !rarity_tag.is_empty() || !hook_tag.is_empty() {
                let mut spans = vec![Span::raw("   ")];
                if !rarity_tag.is_empty() {
                    spans.push(bracket_tag_span(rarity_tag, tech_rarity_style(tech.rarity)));
                }
                if !hook_tag.is_empty() {
                    if !rarity_tag.is_empty() {
                        spans.push(Span::raw(" "));
                    }
                    spans.push(bracket_tag_span(hook_tag, Theme::warning_style()));
                }
                lines.push(Line::from(spans));
            }
        }
        lines.push(Line::from(""));
    }

    let height = inner.height.saturating_sub(1) as usize;
    let scroll = selected_line_idx.saturating_sub(height / 2) as u16;
    let paragraph = Paragraph::new(lines)
        .style(Theme::default_style())
        .scroll((scroll, 0));
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

    let ordered = filtered_research_techs(app_state, game_state);
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
            format!("{} • {} rp", tech.domain.name(), tech.cost),
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            format!("Status: {}", status.label()),
            Theme::default_style(),
        )),
        Line::from(vec![
            Span::styled("Rarity: ", Theme::muted_style()),
            Span::styled(tech.rarity.label(), tech_rarity_style(tech.rarity)),
            if tech.future_hook {
                Span::styled(" [Planned/Future Hook]", Theme::warning_style())
            } else {
                Span::raw("")
            },
        ]),
        Line::from(join_tag_spans(tech.tags)),
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

    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: false }),
        inner,
    );
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
        "Queued Techs",
        Theme::title_style(),
    )));
    lines.push(Line::from(""));

    if empire.research.queue.is_empty() {
        lines.push(Line::from(Span::styled(
            "None queued",
            Theme::muted_style(),
        )));
    } else {
        for (idx, tech_id) in empire.research.queue.iter().enumerate() {
            let label = tech_by_id(*tech_id)
                .map(|t| t.name)
                .unwrap_or("Unknown Tech");
            lines.push(Line::from(vec![
                Span::styled(format!("  {}. ", idx + 1), Theme::muted_style()),
                Span::styled(label, Theme::default_style()),
            ]));
        }
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
    use game_core::{all_techs, TechId};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    fn render_detail_buffer(
        width: u16,
        height: u16,
        app_state: &AppState,
        game_state: &GameState,
    ) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_selected_tech_detail(frame, area, app_state, game_state);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_rows(buffer: &Buffer) -> Vec<String> {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .and_then(|cell| cell.symbol().chars().next())
                            .unwrap_or(' ')
                    })
                    .collect()
            })
            .collect()
    }

    fn find_text_style(buffer: &Buffer, needle: &str) -> Option<Style> {
        for y in 0..buffer.area.height {
            let row: String = (0..buffer.area.width)
                .map(|x| {
                    buffer
                        .cell((x, y))
                        .and_then(|cell| cell.symbol().chars().next())
                        .unwrap_or(' ')
                })
                .collect();
            if let Some(x) = row.find(needle) {
                return buffer.cell((x as u16, y)).map(|cell| cell.style());
            }
        }
        None
    }

    #[test]
    fn ordered_research_techs_are_grouped_by_domain_then_order() {
        let ordered = ordered_research_techs();

        for pair in ordered.windows(2) {
            let left = pair[0];
            let right = pair[1];
            let left_key = (
                tech_domain_sort_index(left.domain),
                left.display_order,
                left.id,
            );
            let right_key = (
                tech_domain_sort_index(right.domain),
                right.display_order,
                right.id,
            );
            assert!(
                left_key <= right_key,
                "Research ordering must stay linear for cursor navigation"
            );
        }
    }

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
        use game_core::Command;

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
            research: crate::app::ResearchScreenState {
                cursor: 999,
                ..Default::default()
            },
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

    #[test]
    fn research_detail_wraps_long_description() {
        let engine = Engine::new(42);
        let app_state = AppState {
            research: crate::app::ResearchScreenState {
                query: "Neutrino".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let buffer = render_detail_buffer(36, 18, &app_state, &engine.state);
        let rows = buffer_rows(&buffer).join("\n");

        assert!(rows.contains("Deep-penetrating"));
        assert!(rows.contains("sensor arrays"));
        assert!(rows.contains("interference patterns."));
    }

    #[test]
    fn research_detail_colors_rare_rarity_and_tags() {
        let mut engine = Engine::new(42);
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .extend([TechId(3), TechId(6)]);
        let app_state = AppState {
            research: crate::app::ResearchScreenState {
                query: "Cartography".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let buffer = render_detail_buffer(60, 18, &app_state, &engine.state);
        let rarity_style = find_text_style(&buffer, "Rare").expect("Rare label should render");
        let tag_style =
            find_text_style(&buffer, "[Hyperspace]").expect("Hyperspace tag should render");

        assert_eq!(rarity_style.fg, Some(Theme::accent2()));
        assert_eq!(tag_style.fg, Some(Theme::accent()));
    }

    #[test]
    fn queued_tech_detail_shows_queued_status() {
        use game_core::Command;

        let mut engine = Engine::new(42);
        let queued_tech = TechId(3);
        engine.apply_turn(vec![Command::QueueResearch { tech: queued_tech }]);

        let cursor = ordered_research_techs()
            .iter()
            .position(|tech| tech.id == queued_tech)
            .expect("queued tech should be in ordered research list");
        let app_state = AppState {
            research: crate::app::ResearchScreenState {
                cursor,
                ..Default::default()
            },
            ..Default::default()
        };

        let buffer = render_detail_buffer(60, 18, &app_state, &engine.state);
        let rows = buffer_rows(&buffer).join("\n");

        assert!(rows.contains("Status: Queued"));
    }

    #[test]
    fn queued_status_filter_includes_only_queued_techs() {
        use game_core::Command;

        let mut engine = Engine::new(42);
        let active_tech = TechId(1);
        let queued_tech = TechId(3);
        engine.apply_turn(vec![
            Command::SelectResearch { tech: active_tech },
            Command::QueueResearch { tech: queued_tech },
        ]);

        let app_state = AppState {
            research: crate::app::ResearchScreenState {
                status_filter: 3,
                ..Default::default()
            },
            ..Default::default()
        };

        let filtered = filtered_research_techs(&app_state, &engine.state);
        assert_eq!(filtered.iter().map(|tech| tech.id).collect::<Vec<_>>(), vec![queued_tech]);
    }
}
