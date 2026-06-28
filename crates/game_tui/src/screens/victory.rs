//! Victory screen — campaign progress and outcome.

use crate::AppState;
use crate::components::{
    derive_header_data, quiet_panel_block, render_footer, render_header, section_heading,
};
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use game_core::{
    EmpireId, EmpireVictoryProgress, FinalVictory, FleetKind, GameState, LegacyScoreBreakdown,
    VictoryCondition, VictoryPath, VictoryPathStatus, VictorySettings,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

const MIN_WIDTH_FOR_SIDE_BY_SIDE: u16 = 100;
const MIN_HEIGHT_FOR_SIDE_BY_SIDE: u16 = 18;

/// Full snapshot of everything the Victory screen renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VictoryData {
    pub paths: Vec<PathView>,
    pub final_victory: Option<FinalVictory>,
    pub turn_limit_enabled: bool,
    pub turn_limit: u32,
    pub current_turn: u32,
    pub empire_id: EmpireId,
    pub empire_name: String,
    pub empire_progress: EmpireVictoryProgress,
    pub rivals: Vec<(EmpireId, String, EmpireVictoryProgress)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathView {
    pub path: VictoryPath,
    pub status: VictoryPathStatus,
    pub progress_percent: u8,
    pub leading_empire: Option<EmpireId>,
    pub leading_empire_name: Option<String>,
    pub player_progress: Option<PlayerPathView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerPathView {
    pub value_line: String,
    pub rivals_line: String,
}

/// Build a deterministic snapshot of the campaign's victory posture.
pub fn derive_victory_data(game_state: &GameState) -> VictoryData {
    let settings: VictorySettings = game_state
        .scenario
        .as_ref()
        .map(|s| s.victory_settings.clone())
        .unwrap_or_default();
    let player = game_state.player_empire;
    let player_name = game_state
        .empires
        .get(&player)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "Player".to_string());
    let empire_progress = game_state
        .victory_status
        .per_empire
        .get(&player)
        .cloned()
        .unwrap_or_default();

    let mut rivals: Vec<(EmpireId, String, EmpireVictoryProgress)> = game_state
        .empires
        .iter()
        .filter(|(id, _)| **id != player)
        .map(|(id, empire)| {
            (
                *id,
                empire.name.clone(),
                game_state
                    .victory_status
                    .per_empire
                    .get(id)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect();
    rivals.sort_by_key(|r| r.0);

    let mut paths = Vec::new();
    for path in VictoryPath::tie_break_order() {
        let progress = game_state
            .victory_status
            .progress
            .iter()
            .find(|p| p.path == *path)
            .copied()
            .unwrap_or(game_core::VictoryProgress {
                path: *path,
                status: VictoryPathStatus::Disabled,
                progress_percent: 0,
                leading_empire: None,
            });
        let leading_empire_name = progress
            .leading_empire
            .and_then(|id| game_state.empires.get(&id).map(|e| e.name.clone()));
        let player_progress = if settings.is_enabled(*path) {
            Some(player_path_view(
                game_state,
                &settings,
                *path,
                &empire_progress,
                &rivals,
            ))
        } else {
            None
        };
        paths.push(PathView {
            path: *path,
            status: progress.status,
            progress_percent: progress.progress_percent,
            leading_empire: progress.leading_empire,
            leading_empire_name,
            player_progress,
        });
    }

    VictoryData {
        paths,
        final_victory: game_state.victory_status.final_victory.clone(),
        turn_limit_enabled: settings.turn_limit_enabled,
        turn_limit: settings.turn_limit,
        current_turn: game_state.turn,
        empire_id: player,
        empire_name: player_name,
        empire_progress,
        rivals,
    }
}

fn player_path_view(
    game_state: &GameState,
    settings: &VictorySettings,
    path: VictoryPath,
    player_progress: &EmpireVictoryProgress,
    rivals: &[(EmpireId, String, EmpireVictoryProgress)],
) -> PlayerPathView {
    match path {
        VictoryPath::Supremacy => {
            let player_colonies = game_state
                .colonies
                .values()
                .filter(|c| c.owner == game_state.player_empire)
                .count();
            let alive_empires = game_state
                .empires
                .keys()
                .filter(|id| empire_is_alive(game_state, **id))
                .count();
            PlayerPathView {
                value_line: format!("{player_colonies} colonies owned"),
                rivals_line: format!("{alive_empires} major empires still alive"),
            }
        }
        VictoryPath::Ascendancy => {
            let req = settings
                .condition_for(VictoryPath::Ascendancy)
                .and_then(|c| match c {
                    VictoryCondition::Ascendancy {
                        control_percent, ..
                    } => Some(*control_percent),
                    _ => None,
                })
                .unwrap_or(50);
            let turns_required = settings
                .condition_for(VictoryPath::Ascendancy)
                .and_then(|c| match c {
                    VictoryCondition::Ascendancy {
                        consecutive_turns_required,
                        ..
                    } => Some(*consecutive_turns_required),
                    _ => None,
                })
                .unwrap_or(10);
            let leading = game_state
                .victory_status
                .progress
                .iter()
                .find(|p| p.path == VictoryPath::Ascendancy)
                .and_then(|p| p.leading_empire)
                .and_then(|id| game_state.empires.get(&id))
                .map(|e| e.name.clone());
            let leading_line = match leading {
                Some(name) => format!("Leader: {name}"),
                None => "Leader: —".to_string(),
            };
            PlayerPathView {
                value_line: format!(
                    "Held {}/{} turns at ≥{req}% systems",
                    player_progress.ascendancy_hold_turns, turns_required
                ),
                rivals_line: leading_line,
            }
        }
        VictoryPath::Scientific => {
            let req = settings
                .condition_for(VictoryPath::Scientific)
                .and_then(|c| match c {
                    VictoryCondition::Scientific {
                        project_points_required,
                        ..
                    } => Some(*project_points_required),
                    _ => None,
                })
                .unwrap_or(1_500);
            let pct = if req > 0 {
                ((player_progress.scientific_project_points.max(0) as u64 * 100) / req as u64)
                    .min(100) as u8
            } else {
                100
            };
            let leader_name = game_state
                .victory_status
                .progress
                .iter()
                .find(|p| p.path == VictoryPath::Scientific)
                .and_then(|p| p.leading_empire)
                .and_then(|id| game_state.empires.get(&id))
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "—".to_string());
            let next_warning = [25u8, 50, 75, 90]
                .into_iter()
                .find(|b| *b > pct && pct < 100)
                .map(|b| format!("{b}%"))
                .unwrap_or_else(|| "—".to_string());
            PlayerPathView {
                value_line: format!(
                    "Project {}/{} ({}%{})",
                    player_progress.scientific_project_points,
                    req,
                    pct,
                    if player_progress.scientific_eligible {
                        ""
                    } else {
                        " — ineligible"
                    }
                ),
                rivals_line: format!("Leader: {leader_name} · next warning {next_warning}"),
            }
        }
        VictoryPath::Legacy => {
            let b = &player_progress.legacy_breakdown;
            PlayerPathView {
                value_line: format!("Score {} (Legacy)", b.total),
                rivals_line: legacy_rivals_line(game_state, rivals),
            }
        }
    }
}

fn legacy_rivals_line(
    game_state: &GameState,
    rivals: &[(EmpireId, String, EmpireVictoryProgress)],
) -> String {
    let player = game_state.player_empire;
    let mut entries: Vec<(i64, &str, EmpireId)> = Vec::new();
    let player_score = game_state
        .victory_status
        .per_empire
        .get(&player)
        .map(|p| p.legacy_breakdown.total)
        .unwrap_or(0);
    entries.push((player_score, "Player", player));
    for (id, name, progress) in rivals {
        entries.push((progress.legacy_breakdown.total, name.as_str(), *id));
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
    let top: Vec<String> = entries
        .iter()
        .take(3)
        .map(|(score, name, _)| format!("{name} {score}"))
        .collect();
    format!("Top: {}", top.join(" · "))
}

fn empire_is_alive(state: &GameState, empire: EmpireId) -> bool {
    let has_colony = state.colonies.values().any(|c| c.owner == empire);
    if has_colony {
        return true;
    }
    state.fleets.values().any(|f| {
        f.owner == empire
            && !matches!(
                f.kind,
                FleetKind::Scout
                    | FleetKind::FastScout
                    | FleetKind::Science
                    | FleetKind::SurveyCutter
                    | FleetKind::Colonizer
                    | FleetKind::ColonyArk
            )
    })
}

/// Render the Victory screen into `area`.  The screen is read-only:
/// it always shows the current state and never emits commands.  The
/// layout is resize-safe: on small terminals the Legacy breakdown is
/// rendered inside the status column rather than being dropped.
pub fn render_victory(
    frame: &mut Frame,
    area: Rect,
    _app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);
    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let data = derive_victory_data(game_state);

    let compact = main_area.width < MIN_WIDTH_FOR_SIDE_BY_SIDE
        || main_area.height < MIN_HEIGHT_FOR_SIDE_BY_SIDE;
    // Three real top-level vertical regions in both modes.  A nested
    // split from a Length(7) parent collapses an inner Min(0) to
    // height 0, so the Legacy breakdown must always own a dedicated
    // top-level slot.  Compact mode only tightens the paths-region
    // minimum.
    let paths_min = if compact { 4 } else { 6 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(paths_min),
            Constraint::Length(8),
        ])
        .split(main_area);
    render_status_block(frame, chunks[0], &data);
    render_paths_block(frame, chunks[1], &data);
    render_legacy_block(frame, chunks[2], &data);

    let hint = if data.final_victory.is_some() {
        "Esc returns to the previous screen · campaign complete."
    } else {
        "Esc returns to the previous screen · V opens Victory from any game screen."
    };
    render_footer(frame, footer_area, &Screen::Victory, Some(hint));
}

fn render_status_block(frame: &mut Frame, area: Rect, data: &VictoryData) {
    let block = quiet_panel_block("Campaign Status");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    if let Some(final_v) = &data.final_victory {
        lines.push(section_heading("VICTORY ACHIEVED"));
        let winner_name = data
            .rivals
            .iter()
            .find(|(id, _, _)| *id == final_v.winner)
            .map(|(_, name, _)| name.clone())
            .unwrap_or_else(|| {
                if final_v.winner == data.empire_id {
                    data.empire_name.clone()
                } else {
                    format!("Empire {}", final_v.winner.0)
                }
            });
        lines.push(Line::from(vec![
            Span::styled("Winner: ", Theme::title_style()),
            Span::styled(winner_name, Theme::success_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Path: ", Theme::title_style()),
            Span::styled(final_v.path.label(), Theme::accent_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Turn: ", Theme::title_style()),
            Span::raw(final_v.turn.to_string()),
        ]));
        lines.push(Line::from(Span::styled(
            final_v.reason.clone(),
            Theme::muted_style(),
        )));
    } else {
        lines.push(section_heading("No victory achieved yet."));
        if data.turn_limit_enabled {
            lines.push(Line::from(vec![
                Span::styled("Turn limit: ", Theme::title_style()),
                Span::raw(format!(
                    "{} / {} (Legacy winner at limit)",
                    data.current_turn, data.turn_limit
                )),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                "Turn limit disabled — play indefinitely until a path fires.",
                Theme::muted_style(),
            )));
        }
        lines.push(Line::from(Span::styled(
            "Four paths: Supremacy · Ascendancy · Scientific · Legacy.",
            Theme::muted_style(),
        )));
    }
    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn render_paths_block(frame: &mut Frame, area: Rect, data: &VictoryData) {
    let block = quiet_panel_block("Victory Paths");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    for path in &data.paths {
        let (status_text, status_style) = match path.status {
            VictoryPathStatus::Achieved => ("DONE", Theme::success_style()),
            VictoryPathStatus::Disabled => ("OFF", Theme::muted_style()),
            VictoryPathStatus::InProgress => ("ON", Theme::accent_style()),
        };
        let bar = make_bar(path.progress_percent as usize);
        let leader = path
            .leading_empire_name
            .clone()
            .unwrap_or_else(|| "—".to_string());
        let mut header = vec![
            Span::styled(format!("{:<10}", path.path.label()), Theme::title_style()),
            Span::styled(format!("[{status_text}] "), status_style),
            Span::styled(format!("{bar} "), Theme::default_style()),
            Span::styled(
                format!("{:>3}% ", path.progress_percent),
                Theme::accent_style(),
            ),
            Span::styled(format!("leader {leader}"), Theme::default_style()),
        ];
        lines.push(Line::from(std::mem::take(&mut header)));
        if let Some(player) = &path.player_progress {
            lines.push(Line::from(vec![
                Span::styled("  you ", Theme::muted_style()),
                Span::styled(player.value_line.clone(), Theme::default_style()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  rivals ", Theme::muted_style()),
                Span::styled(player.rivals_line.clone(), Theme::muted_style()),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                "  (path disabled)",
                Theme::muted_style(),
            )));
        }
    }
    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn render_legacy_block(frame: &mut Frame, area: Rect, data: &VictoryData) {
    let block = quiet_panel_block("Legacy Score Breakdown");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let breakdown: LegacyScoreBreakdown = data.empire_progress.legacy_breakdown.clone();
    let entries: Vec<(&str, i64)> = vec![
        ("Colonies", breakdown.colonies),
        ("Population", breakdown.population),
        ("Completed techs", breakdown.completed_technologies),
        ("Explored systems", breakdown.explored_systems),
        ("Surveyed planets", breakdown.surveyed_planets),
        (
            "Discoveries & resources",
            breakdown.discoveries_and_resources,
        ),
        ("Battle victories", breakdown.battle_victories),
        ("Credits (÷10)", breakdown.credits),
    ];
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("Player Legacy total: {}", breakdown.total),
        Theme::accent_style(),
    ))];
    for (label, value) in entries {
        lines.push(Line::from(vec![
            Span::styled(format!("  {label:<22} "), Theme::muted_style()),
            Span::raw(value.to_string()),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn make_bar(percent: usize) -> String {
    let p = percent.min(100);
    let filled = p * 10 / 100;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(10 - filled))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn render_buffer(engine: &Engine, app_state: &AppState, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_victory(frame, frame.area(), app_state, &engine.state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn derive_includes_all_four_paths() {
        let engine = Engine::new(42);
        let data = derive_victory_data(&engine.state);
        assert_eq!(data.paths.len(), 4);
        for path in VictoryPath::tie_break_order() {
            assert!(data.paths.iter().any(|p| p.path == *path));
        }
    }

    #[test]
    fn default_turn_limit_matches_default_v1() {
        let engine = Engine::new(42);
        let data = derive_victory_data(&engine.state);
        assert!(data.turn_limit_enabled);
        assert_eq!(data.turn_limit, 300);
    }

    #[test]
    fn derive_reflects_disabled_paths() {
        let mut engine = Engine::new(42);
        if let Some(scenario) = engine.state.scenario.as_mut() {
            scenario
                .victory_settings
                .enabled_paths
                .remove(&VictoryPath::Legacy);
        }
        // Trigger a re-evaluation so the new status lands in
        // `victory_status.progress`.  Without a turn, the snapshot still
        // reflects the default-on settings.
        let _ = engine.apply_turn(vec![game_core::Command::EndTurn]);
        let data = derive_victory_data(&engine.state);
        let legacy = data
            .paths
            .iter()
            .find(|p| p.path == VictoryPath::Legacy)
            .expect("legacy path present");
        assert_eq!(legacy.status, VictoryPathStatus::Disabled);
        assert!(legacy.player_progress.is_none());
    }

    #[test]
    fn derive_is_deterministic() {
        let engine = Engine::new(42);
        let a = derive_victory_data(&engine.state);
        let b = derive_victory_data(&engine.state);
        assert_eq!(a, b);
    }

    #[test]
    fn final_victory_surfaces_in_data() {
        let mut engine = Engine::new(42);
        engine.state.victory_status.final_victory = Some(FinalVictory {
            winner: engine.state.player_empire,
            path: VictoryPath::Supremacy,
            turn: 12,
            reason: "test".to_string(),
        });
        let data = derive_victory_data(&engine.state);
        let final_v = data.final_victory.expect("final victory present");
        assert_eq!(final_v.path, VictoryPath::Supremacy);
        assert_eq!(final_v.turn, 12);
    }

    #[test]
    fn renders_at_80x24_with_status_and_paths() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("Campaign Status"));
        assert!(text.contains("Supremacy"));
        assert!(text.contains("Ascendancy"));
        assert!(text.contains("Scientific"));
        assert!(text.contains("Legacy"));
    }

    #[test]
    fn renders_at_120x36_with_legacy_breakdown() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 120, 36);
        let text = buffer_text(&buffer);
        assert!(text.contains("Legacy Score Breakdown"));
        assert!(text.contains("Player Legacy total"));
    }

    #[test]
    fn renders_final_victory_banner() {
        let mut engine = Engine::new(42);
        engine.state.victory_status.final_victory = Some(FinalVictory {
            winner: engine.state.player_empire,
            path: VictoryPath::Supremacy,
            turn: 5,
            reason: "test winner".to_string(),
        });
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("VICTORY ACHIEVED"));
        assert!(text.contains("Supremacy"));
    }

    /// Compact 80×24 must still reach the Legacy breakdown.  An
    /// earlier layout nested a Length(7) + Min(0) split under a
    /// Length(7) parent, which collapsed the inner Min(0) to zero
    /// height and made the Legacy block invisible on small terminals.
    /// This test pins the regression.
    #[test]
    fn compact_80x24_renders_legacy_breakdown() {
        let engine = Engine::new(42);
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("Legacy Score Breakdown"));
        assert!(text.contains("Player Legacy total"));
    }

    #[test]
    fn renders_safely_when_rivals_list_is_empty() {
        let mut engine = Engine::new(42);
        // Strip every non-player empire from `state.empires` so
        // `derive_victory_data` has no rivals to list.  Clearing just
        // `state.ai_empires` would leave the empire records in place.
        let player = engine.state.player_empire;
        let non_player_empires: Vec<_> = engine
            .state
            .empires
            .keys()
            .copied()
            .filter(|id| *id != player)
            .collect();
        for id in non_player_empires {
            engine.state.empires.remove(&id);
        }
        engine.state.ai_empires.clear();
        engine.state.ai_explored_stars.clear();
        engine.state.ai_relations.clear();
        engine.state.diplomacy.clear();
        engine.state.diplomacy_relationships.clear();
        engine.state.diplomacy_pending_communications.clear();
        engine.state.colonies.retain(|_, c| c.owner == player);
        engine.state.fleets.retain(|_, f| f.owner == player);
        engine.state.empire_explored_stars.clear();
        engine.state.empire_resource_access.clear();
        engine.state.empire_intel.clear();
        engine.state.empire_trade_routes.clear();
        engine.state.empire_trade_income.clear();
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 80, 24);
        let text = buffer_text(&buffer);
        assert!(text.contains("Campaign Status"));
        // Sanity: data builder should report zero rivals.
        let data = derive_victory_data(&engine.state);
        assert!(data.rivals.is_empty());
    }

    #[test]
    fn renders_with_rivals_populated() {
        let engine = Engine::new(42);
        let data = derive_victory_data(&engine.state);
        assert!(!data.rivals.is_empty());
        let app_state = AppState::default();
        let buffer = render_buffer(&engine, &app_state, 120, 36);
        let text = buffer_text(&buffer);
        assert!(text.contains("Campaign Status"));
        assert!(text.contains("Legacy Score Breakdown"));
    }
}
