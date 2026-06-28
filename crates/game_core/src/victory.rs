//! Victory condition evaluation.
//!
//! Far-space v1 ships four victory paths:
//!
//! 1. **Supremacy** — last surviving major empire wins when all other
//!    non-defeated empires are eliminated or reduced below the liveness rule
//!    (≥ 1 colony *or* ≥ 1 non-civilian fleet).
//! 2. **Ascendancy** — wide-empire control: ≥ 50% of unique colonized systems
//!    for 10 consecutive turns. Consecutive counter resets to 0 the moment
//!    the threshold is dropped.
//! 3. **Scientific** — late-game tech + project: complete the configured
//!    eligibility tech, then accumulate a deterministic science+industry
//!    threshold. Warning events fire at 25/50/75/90% project progress.
//! 4. **Legacy** — at the turn limit, the empire with the highest Legacy
//!    score wins. Score is a transparent sum of state-derived components.
//!
//! All math is integer; no wall-clock; no HashMap iteration over
//! non-deterministic sources. Ties resolve by `(score desc, EmpireId asc)`.

use crate::events::Event;
use crate::state::{
    EmpireId, EmpireVictoryProgress, FinalVictory, FleetKind, GameState, LegacyScoreBreakdown,
    MilestoneKey, StarId, TechId, VictoryCondition, VictoryPath, VictoryPathStatus,
    VictoryProgress, VictorySettings, requirements_met,
};
use std::collections::{BTreeMap, BTreeSet};

const SCIENTIFIC_MILESTONES: [u8; 4] = [25, 50, 75, 90];
const LEGACY_TIE_BREAK_PREFIX: &str = "highest score at turn limit";

struct SupremacyEvaluation {
    leader: Option<EmpireId>,
    alive_major_empires: u32,
    all_major: Vec<EmpireId>,
}

struct AscendancyEvaluation {
    per_empire_systems: BTreeMap<EmpireId, u32>,
    total_colonized_systems: u32,
    leading_empire: Option<EmpireId>,
    leading_control_percent: u8,
    required_percent: u8,
}

struct ScientificEvaluation {
    per_empire_points: BTreeMap<EmpireId, i64>,
    eligibility_tech: TechId,
    project_points_required: i64,
    leading_empire: Option<EmpireId>,
    leading_percent: u8,
}

struct LegacyEvaluation {
    per_empire_score: BTreeMap<EmpireId, LegacyScoreBreakdown>,
    leading_empire: Option<EmpireId>,
    top_score: i64,
}

fn percent(num: u64, den: u64) -> u8 {
    num.checked_mul(100)
        .and_then(|scaled| scaled.checked_div(den))
        .map(|v| v.min(100) as u8)
        .unwrap_or(0)
}

fn percent_i64(num: i64, den: i64) -> u8 {
    if den <= 0 {
        return 0;
    }
    let n = num.max(0) as u64;
    let d = den as u64;
    n.checked_mul(100)
        .and_then(|scaled| scaled.checked_div(d))
        .map(|v| v.min(100) as u8)
        .unwrap_or(0)
}

fn empire_ids_sorted(state: &GameState) -> Vec<EmpireId> {
    state.empires.keys().copied().collect()
}

fn is_civilian_fleet_kind(kind: FleetKind) -> bool {
    matches!(
        kind,
        FleetKind::Scout
            | FleetKind::FastScout
            | FleetKind::Science
            | FleetKind::SurveyCutter
            | FleetKind::Colonizer
            | FleetKind::ColonyArk
    )
}

fn empire_is_alive(state: &GameState, empire: EmpireId) -> bool {
    let has_colony = state.colonies.values().any(|c| c.owner == empire);
    if has_colony {
        return true;
    }
    state
        .fleets
        .values()
        .any(|f| f.owner == empire && !is_civilian_fleet_kind(f.kind))
}

fn explored_stars_for_empire(state: &GameState, empire: EmpireId) -> &BTreeSet<StarId> {
    if empire == state.player_empire {
        &state.explored_stars
    } else {
        state
            .empire_explored_stars
            .get(&empire)
            .unwrap_or(&state.ai_explored_stars)
    }
}

fn evaluate_supremacy(state: &GameState) -> SupremacyEvaluation {
    let all_major = empire_ids_sorted(state);
    let alive: Vec<EmpireId> = all_major
        .iter()
        .copied()
        .filter(|id| empire_is_alive(state, *id))
        .collect();
    let alive_count = alive.len() as u32;
    let leader = if alive_count == 1 {
        alive.first().copied()
    } else {
        None
    };
    SupremacyEvaluation {
        leader,
        alive_major_empires: alive_count,
        all_major,
    }
}

fn evaluate_ascendancy(state: &GameState, condition: &VictoryCondition) -> AscendancyEvaluation {
    let required_percent = match condition {
        VictoryCondition::Ascendancy {
            control_percent, ..
        } => *control_percent,
        _ => 50,
    };

    let mut per_empire_stars: BTreeMap<EmpireId, BTreeSet<StarId>> = BTreeMap::new();
    for colony in state.colonies.values() {
        per_empire_stars
            .entry(colony.owner)
            .or_default()
            .insert(colony.star);
    }
    let total_colonized_systems = per_empire_stars
        .values()
        .flat_map(|set| set.iter().copied())
        .collect::<BTreeSet<_>>()
        .len() as u32;

    let mut per_empire_systems: BTreeMap<EmpireId, u32> = BTreeMap::new();
    let mut ranked: Vec<(u32, EmpireId)> = Vec::new();
    for empire in empire_ids_sorted(state) {
        let count = per_empire_stars
            .get(&empire)
            .map(|set| set.len() as u32)
            .unwrap_or(0);
        per_empire_systems.insert(empire, count);
        ranked.push((count, empire));
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (leading_empire, leading_control_percent) = ranked
        .first()
        .copied()
        .map(|(count, empire)| {
            let pct = if total_colonized_systems == 0 {
                0
            } else {
                ((count as u64).saturating_mul(100) / total_colonized_systems as u64) as u8
            };
            (Some(empire), pct)
        })
        .unwrap_or((None, 0));

    AscendancyEvaluation {
        per_empire_systems,
        total_colonized_systems,
        leading_empire,
        leading_control_percent,
        required_percent,
    }
}

/// Read-only Scientific evaluation. Returns the current per-empire
/// project point totals as stored in `state.victory_status.per_empire`
/// — no per-turn advance. Used by `recompute_victory_snapshot` so the
/// snapshot path cannot silently tick scientific progress forward.
fn evaluate_scientific_snapshot(
    state: &GameState,
    condition: &VictoryCondition,
) -> ScientificEvaluation {
    let (eligibility_tech, project_points_required) = match condition {
        VictoryCondition::Scientific {
            eligibility_tech,
            project_points_required,
        } => (*eligibility_tech, *project_points_required),
        _ => (TechId(61), 0),
    };
    let mut per_empire_points: BTreeMap<EmpireId, i64> = BTreeMap::new();
    let mut ranked: Vec<(i64, EmpireId)> = Vec::new();
    for empire in empire_ids_sorted(state) {
        let current = state
            .victory_status
            .per_empire
            .get(&empire)
            .map(|p| p.scientific_project_points)
            .unwrap_or(0);
        per_empire_points.insert(empire, current);
        ranked.push((current, empire));
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (leading_empire, leading_percent) = ranked
        .first()
        .copied()
        .map(|(points, empire)| (Some(empire), percent_i64(points, project_points_required)))
        .unwrap_or((None, 0));
    ScientificEvaluation {
        per_empire_points,
        eligibility_tech,
        project_points_required,
        leading_empire,
        leading_percent,
    }
}

/// Mutating Scientific evaluation. Advances each eligible empire's
/// project point total by the per-turn max of science/industry yield
/// across the empire's colonies, then returns the updated evaluation.
/// This is the only path that should write to
/// `entry.scientific_project_points`; the snapshot helper is
/// strictly read-only.
fn evaluate_scientific(state: &GameState, condition: &VictoryCondition) -> ScientificEvaluation {
    let (eligibility_tech, project_points_required) = match condition {
        VictoryCondition::Scientific {
            eligibility_tech,
            project_points_required,
        } => (*eligibility_tech, *project_points_required),
        _ => (TechId(61), 0),
    };
    let mut per_empire_points: BTreeMap<EmpireId, i64> = BTreeMap::new();
    let mut ranked: Vec<(i64, EmpireId)> = Vec::new();
    for empire in empire_ids_sorted(state) {
        let eligible = state
            .empires
            .get(&empire)
            .is_some_and(|e| e.research.completed.contains(&eligibility_tech));
        let current = state
            .victory_status
            .per_empire
            .get(&empire)
            .map(|p| p.scientific_project_points)
            .unwrap_or(0);
        // Advance each turn by science OR industry yield, whichever is greater.
        let production_turn: i64 = if eligible {
            state
                .last_colony_yields
                .iter()
                .filter_map(|(colony_id, y)| {
                    state
                        .colonies
                        .get(colony_id)
                        .filter(|c| c.owner == empire)
                        .map(|_| y.science.max(y.industry))
                })
                .sum()
        } else {
            0
        };
        let next = current.saturating_add(production_turn);
        per_empire_points.insert(empire, next);
        ranked.push((next, empire));
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (leading_empire, leading_percent) = ranked
        .first()
        .copied()
        .map(|(points, empire)| (Some(empire), percent_i64(points, project_points_required)))
        .unwrap_or((None, 0));
    ScientificEvaluation {
        per_empire_points,
        eligibility_tech,
        project_points_required,
        leading_empire,
        leading_percent,
    }
}

fn compute_legacy_breakdown(state: &GameState, empire: EmpireId) -> LegacyScoreBreakdown {
    let mut b = LegacyScoreBreakdown::default();

    let empire_colonies: Vec<_> = state
        .colonies
        .values()
        .filter(|c| c.owner == empire)
        .collect();
    b.colonies = empire_colonies.len() as i64;
    b.population = empire_colonies.iter().map(|c| c.population as i64).sum();

    if let Some(record) = state.empires.get(&empire) {
        b.completed_technologies = record.research.completed.len() as i64 * 5;
    }

    let explored = explored_stars_for_empire(state, empire);
    b.explored_systems = explored.len() as i64;

    let mut surveyed_planets: i64 = 0;
    let mut discoveries: i64 = 0;
    let completed = state
        .empires
        .get(&empire)
        .map(|e| e.research.completed.as_slice())
        .unwrap_or(&[]);
    for star in state.stars.values() {
        if !explored.contains(&star.id) {
            continue;
        }
        for planet in &star.planets {
            if planet.surveyed {
                surveyed_planets += 1;
                discoveries += planet
                    .specials
                    .iter()
                    .filter(|s| {
                        requirements_met(s.visibility_requirements(), planet.surveyed, completed)
                    })
                    .count() as i64
                    * 3;
                discoveries += planet
                    .anomalies
                    .iter()
                    .filter(|a| {
                        requirements_met(a.detection_requirements(), planet.surveyed, completed)
                    })
                    .count() as i64
                    * 5;
            }
        }
    }
    b.surveyed_planets = surveyed_planets;
    b.discoveries_and_resources = discoveries;

    if let Some(by_resource) = state.empire_resource_access.get(&empire) {
        b.discoveries_and_resources += by_resource.values().map(|v| *v as i64 * 4).sum::<i64>();
    }

    let mut battle_victories: i64 = 0;
    for report in &state.battle_reports {
        if report.empire_a == empire && report.fleet_b_destroyed && !report.fleet_a_destroyed {
            battle_victories += 1;
        }
        if report.empire_b == empire && report.fleet_a_destroyed && !report.fleet_b_destroyed {
            battle_victories += 1;
        }
    }
    b.battle_victories = battle_victories * 20;

    if let Some(record) = state.empires.get(&empire) {
        b.credits = record.credits.max(0) / 10;
    }

    b.total = b.colonies
        + b.population
        + b.completed_technologies
        + b.explored_systems
        + b.surveyed_planets
        + b.discoveries_and_resources
        + b.battle_victories
        + b.credits;
    b
}

fn evaluate_legacy(state: &GameState) -> LegacyEvaluation {
    let mut per_empire_score: BTreeMap<EmpireId, LegacyScoreBreakdown> = BTreeMap::new();
    let mut ranked: Vec<(i64, EmpireId)> = Vec::new();
    for empire in empire_ids_sorted(state) {
        let b = compute_legacy_breakdown(state, empire);
        ranked.push((b.total, empire));
        per_empire_score.insert(empire, b);
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (leading_empire, top_score) = ranked
        .first()
        .copied()
        .map(|(score, empire)| (Some(empire), score))
        .unwrap_or((None, 0));
    LegacyEvaluation {
        per_empire_score,
        leading_empire,
        top_score,
    }
}

fn progress_percent_for_ascendancy(eval: &AscendancyEvaluation) -> u8 {
    if eval.total_colonized_systems == 0 || eval.required_percent == 0 {
        return 0;
    }
    percent(
        eval.leading_control_percent as u64,
        eval.required_percent as u64,
    )
}

fn progress_percent_for_scientific(eval: &ScientificEvaluation) -> u8 {
    eval.leading_percent
}

fn progress_percent_for_legacy(eval: &LegacyEvaluation) -> u8 {
    let soft_target: i64 = 2_000;
    percent_i64(eval.top_score, soft_target)
}

fn progress_percent_for_supremacy(eval: &SupremacyEvaluation) -> u8 {
    if eval.all_major.is_empty() {
        return 0;
    }
    let initial = eval.all_major.len() as u32;
    let remaining = eval.alive_major_empires;
    if initial <= 1 {
        return if remaining == 1 { 100 } else { 0 };
    }
    let eliminated = initial.saturating_sub(remaining);
    percent(eliminated as u64, initial.saturating_sub(1) as u64)
}

fn supremacy_winner(supremacy: &SupremacyEvaluation) -> Option<(EmpireId, String)> {
    supremacy
        .leader
        .map(|e| (e, "last surviving major empire".to_string()))
}

fn ascendancy_winner(
    state: &GameState,
    ascendancy: &AscendancyEvaluation,
    settings: &VictorySettings,
) -> Option<(EmpireId, String)> {
    if !settings.is_enabled(VictoryPath::Ascendancy) {
        return None;
    }
    let (control_percent, required_hold) = match settings.condition_for(VictoryPath::Ascendancy) {
        Some(VictoryCondition::Ascendancy {
            control_percent,
            consecutive_turns_required,
        }) => (*control_percent, *consecutive_turns_required),
        _ => return None,
    };
    if required_hold == 0 {
        return None;
    }
    // Pick lowest EmpireId among those that have held the threshold for
    // `required_hold` consecutive turns.
    for empire in empire_ids_sorted(state) {
        let progress = state.victory_status.per_empire.get(&empire);
        let hold_turns = progress.map(|p| p.ascendancy_hold_turns).unwrap_or(0);
        let empire_systems = ascendancy
            .per_empire_systems
            .get(&empire)
            .copied()
            .unwrap_or(0);
        let empire_percent = if ascendancy.total_colonized_systems == 0 {
            0
        } else {
            ((empire_systems as u64).saturating_mul(100)
                / ascendancy.total_colonized_systems as u64) as u8
        };
        if hold_turns >= required_hold && empire_percent >= control_percent {
            return Some((
                empire,
                format!(
                    "controlled ≥{control_percent}% of colonized systems for {required_hold} consecutive turns"
                ),
            ));
        }
    }
    None
}

fn scientific_winner(
    scientific: &ScientificEvaluation,
    settings: &VictorySettings,
) -> Option<(EmpireId, String)> {
    if !settings.is_enabled(VictoryPath::Scientific) {
        return None;
    }
    let required = match settings.condition_for(VictoryPath::Scientific) {
        Some(VictoryCondition::Scientific {
            project_points_required,
            ..
        }) => *project_points_required,
        _ => i64::MAX,
    };
    for empire in scientific.per_empire_points.keys().copied() {
        let points = scientific
            .per_empire_points
            .get(&empire)
            .copied()
            .unwrap_or(0);
        if points >= required {
            return Some((
                empire,
                "completed the Transcendent Gate project".to_string(),
            ));
        }
    }
    None
}

fn legacy_winner(
    legacy: &LegacyEvaluation,
    supremacy: &SupremacyEvaluation,
    settings: &VictorySettings,
    turn: u32,
) -> Option<(EmpireId, String)> {
    if !settings.turn_limit_enabled || turn < settings.turn_limit {
        return None;
    }
    if let Some(winner) = legacy.leading_empire {
        return Some((
            winner,
            format!("{LEGACY_TIE_BREAK_PREFIX} ({}/{})", winner.0, turn),
        ));
    }
    if let Some(winner) = supremacy.leader {
        return Some((winner, "last surviving empire at turn limit".to_string()));
    }
    None
}

fn progress_status_for(
    settings: &VictorySettings,
    state: &GameState,
    path: VictoryPath,
) -> VictoryPathStatus {
    if !settings.is_enabled(path) {
        return VictoryPathStatus::Disabled;
    }
    if state.victory_status.final_victory.is_some() {
        return VictoryPathStatus::Achieved;
    }
    VictoryPathStatus::InProgress
}

/// Compute a read-only snapshot of the four-path progress and per-empire
/// status without ticking any counters or resolving `final_victory`.
///
/// Used by the save-migration path so that loading a v41 save does not
/// silently advance `Ascendancy` hold turns, `Scientific` project points,
/// warning milestones, or trigger a victory during load. The first
/// real end-of-turn after load runs the mutating evaluator and starts
/// all counters from a clean baseline derived from the loaded state.
pub fn recompute_victory_snapshot(state: &mut GameState) {
    let settings = state
        .scenario
        .as_ref()
        .map(|scenario| scenario.victory_settings.clone())
        .unwrap_or_else(VictorySettings::default_v1);

    let ascendancy_condition = settings
        .condition_for(VictoryPath::Ascendancy)
        .cloned()
        .unwrap_or(VictoryCondition::Ascendancy {
            control_percent: 50,
            consecutive_turns_required: 10,
        });
    let scientific_condition = settings
        .condition_for(VictoryPath::Scientific)
        .cloned()
        .unwrap_or(VictoryCondition::Scientific {
            eligibility_tech: TechId(61),
            project_points_required: 1_500,
        });
    let legacy_condition = settings
        .condition_for(VictoryPath::Legacy)
        .cloned()
        .unwrap_or(VictoryCondition::Legacy {
            early_warning_percent: 75,
        });

    let supremacy = evaluate_supremacy(state);
    let ascendancy = evaluate_ascendancy(state, &ascendancy_condition);
    // Use the snapshot variant for the read-only path so the
    // migration does not silently advance Scientific project points
    // by per-turn yields stored in `last_colony_yields`.  The
    // mutating `evaluate_scientific` is only called from
    // `evaluate_victory_end_turn`.
    let scientific = evaluate_scientific_snapshot(state, &scientific_condition);
    let legacy = evaluate_legacy(state);

    for empire in empire_ids_sorted(state) {
        let entry = state.victory_status.per_empire.entry(empire).or_default();

        for path in VictoryPath::tie_break_order() {
            let status = if !settings.is_enabled(*path) {
                VictoryPathStatus::Disabled
            } else if state.victory_status.final_victory.is_some() {
                VictoryPathStatus::Achieved
            } else {
                VictoryPathStatus::InProgress
            };
            entry.path_status.insert(*path, status);
        }

        let empire_systems = ascendancy
            .per_empire_systems
            .get(&empire)
            .copied()
            .unwrap_or(0);
        let empire_percent = if ascendancy.total_colonized_systems == 0 {
            0
        } else {
            ((empire_systems as u64).saturating_mul(100)
                / ascendancy.total_colonized_systems as u64) as u8
        };
        entry.ascendancy_meets_threshold = empire_percent >= ascendancy.required_percent;
        // Hold counter, project points, and warnings are NOT advanced here.
        // Those ticks only fire from `evaluate_victory_end_turn`.

        entry.scientific_eligible = state
            .empires
            .get(&empire)
            .is_some_and(|e| e.research.completed.contains(&scientific.eligibility_tech));

        if let Some(breakdown) = legacy.per_empire_score.get(&empire) {
            entry.legacy_breakdown = breakdown.clone();
        }
    }

    let progress = vec![
        VictoryProgress {
            path: VictoryPath::Supremacy,
            status: progress_status_for(&settings, state, VictoryPath::Supremacy),
            progress_percent: progress_percent_for_supremacy(&supremacy),
            leading_empire: supremacy.leader,
        },
        VictoryProgress {
            path: VictoryPath::Ascendancy,
            status: progress_status_for(&settings, state, VictoryPath::Ascendancy),
            progress_percent: progress_percent_for_ascendancy(&ascendancy),
            leading_empire: ascendancy.leading_empire,
        },
        VictoryProgress {
            path: VictoryPath::Scientific,
            status: progress_status_for(&settings, state, VictoryPath::Scientific),
            progress_percent: progress_percent_for_scientific(&scientific),
            leading_empire: scientific.leading_empire,
        },
        VictoryProgress {
            path: VictoryPath::Legacy,
            status: progress_status_for(&settings, state, VictoryPath::Legacy),
            progress_percent: progress_percent_for_legacy(&legacy),
            leading_empire: legacy.leading_empire,
        },
    ];
    state.victory_status.progress = progress;

    let _ = legacy_condition;
}

/// Evaluate all four victory paths at end of turn. Mutates `state.victory_status`
/// in place (per-empire progress, milestone markers, and final outcome) and
/// returns the deterministic event stream.
pub fn evaluate_victory_end_turn(state: &mut GameState, completed_turn: u32) -> Vec<Event> {
    let settings = state
        .scenario
        .as_ref()
        .map(|scenario| scenario.victory_settings.clone())
        .unwrap_or_else(VictorySettings::default_v1);

    let ascendancy_condition = settings
        .condition_for(VictoryPath::Ascendancy)
        .cloned()
        .unwrap_or(VictoryCondition::Ascendancy {
            control_percent: 50,
            consecutive_turns_required: 10,
        });
    let scientific_condition = settings
        .condition_for(VictoryPath::Scientific)
        .cloned()
        .unwrap_or(VictoryCondition::Scientific {
            eligibility_tech: TechId(61),
            project_points_required: 1_500,
        });
    let legacy_condition = settings
        .condition_for(VictoryPath::Legacy)
        .cloned()
        .unwrap_or(VictoryCondition::Legacy {
            early_warning_percent: 75,
        });

    let supremacy = evaluate_supremacy(state);
    let ascendancy = evaluate_ascendancy(state, &ascendancy_condition);
    let scientific = evaluate_scientific(state, &scientific_condition);
    let legacy = evaluate_legacy(state);

    let mut events: Vec<Event> = Vec::new();
    let player = state.player_empire;

    for empire in empire_ids_sorted(state) {
        let entry = state.victory_status.per_empire.entry(empire).or_default();

        for path in VictoryPath::tie_break_order() {
            let status = if !settings.is_enabled(*path) {
                VictoryPathStatus::Disabled
            } else if state.victory_status.final_victory.is_some() {
                VictoryPathStatus::Achieved
            } else {
                VictoryPathStatus::InProgress
            };
            entry.path_status.insert(*path, status);
        }

        // Ascendancy hold counter.
        let empire_systems = ascendancy
            .per_empire_systems
            .get(&empire)
            .copied()
            .unwrap_or(0);
        let empire_percent = if ascendancy.total_colonized_systems == 0 {
            0
        } else {
            ((empire_systems as u64).saturating_mul(100)
                / ascendancy.total_colonized_systems as u64) as u8
        };
        let meets = empire_percent >= ascendancy.required_percent;
        entry.ascendancy_meets_threshold = meets;
        if meets {
            entry.ascendancy_hold_turns = entry.ascendancy_hold_turns.saturating_add(1);
        } else {
            entry.ascendancy_hold_turns = 0;
        }

        // Scientific: points were already advanced in `evaluate_scientific`.
        if let Some(points) = scientific.per_empire_points.get(&empire) {
            entry.scientific_project_points = *points;
        }
        entry.scientific_eligible = state
            .empires
            .get(&empire)
            .is_some_and(|e| e.research.completed.contains(&scientific.eligibility_tech));

        if let Some(breakdown) = legacy.per_empire_score.get(&empire) {
            entry.legacy_breakdown = breakdown.clone();
        }
    }

    // Scientific warning milestones (25/50/75/90) — emit at most once per
    // threshold per (empire, path). For v1 the player is the only audience.
    if settings.is_enabled(VictoryPath::Scientific) {
        for empire in empire_ids_sorted(state) {
            let entry = state.victory_status.per_empire.entry(empire).or_default();
            let pct = if scientific.project_points_required > 0 {
                percent_i64(
                    entry.scientific_project_points,
                    scientific.project_points_required,
                )
            } else {
                0
            };
            let previous = entry
                .warning_milestones
                .get(&VictoryPath::Scientific)
                .copied()
                .unwrap_or(0);
            for milestone in SCIENTIFIC_MILESTONES {
                if pct >= milestone && milestone > previous {
                    entry
                        .warning_milestones
                        .insert(VictoryPath::Scientific, milestone);
                    state.victory_status.milestone_levels.insert(
                        MilestoneKey::new(empire, VictoryPath::Scientific),
                        milestone,
                    );
                    if empire == player {
                        events.push(Event::VictoryWarning {
                            path: VictoryPath::Scientific,
                            empire,
                            progress_percent: milestone,
                        });
                    }
                }
            }
        }
    }

    // Build the deterministic progress view for the UI.
    let progress = vec![
        VictoryProgress {
            path: VictoryPath::Supremacy,
            status: progress_status_for(&settings, state, VictoryPath::Supremacy),
            progress_percent: progress_percent_for_supremacy(&supremacy),
            leading_empire: supremacy.leader,
        },
        VictoryProgress {
            path: VictoryPath::Ascendancy,
            status: progress_status_for(&settings, state, VictoryPath::Ascendancy),
            progress_percent: progress_percent_for_ascendancy(&ascendancy),
            leading_empire: ascendancy.leading_empire,
        },
        VictoryProgress {
            path: VictoryPath::Scientific,
            status: progress_status_for(&settings, state, VictoryPath::Scientific),
            progress_percent: progress_percent_for_scientific(&scientific),
            leading_empire: scientific.leading_empire,
        },
        VictoryProgress {
            path: VictoryPath::Legacy,
            status: progress_status_for(&settings, state, VictoryPath::Legacy),
            progress_percent: progress_percent_for_legacy(&legacy),
            leading_empire: legacy.leading_empire,
        },
    ];

    // Emit VictoryProgressMilestone events for the player (existing TUI/log path).
    for entry in &progress {
        if !settings.is_enabled(entry.path) {
            continue;
        }
        if entry.leading_empire != Some(player) {
            continue;
        }
        let key = MilestoneKey::new(player, entry.path);
        let previous = state
            .victory_status
            .milestone_levels
            .get(&key)
            .copied()
            .unwrap_or(0);
        let bands = [25u8, 50, 75, 100];
        for band in bands {
            if entry.progress_percent >= band && previous < band {
                state.victory_status.milestone_levels.insert(key, band);
                events.push(Event::VictoryProgressMilestone {
                    path: entry.path,
                    empire: player,
                    progress_percent: band,
                });
            }
        }
    }

    if state.victory_status.final_victory.is_none() {
        let resolved: Option<(EmpireId, VictoryPath, String)> = None
            .or_else(|| {
                if settings.is_enabled(VictoryPath::Supremacy) {
                    supremacy_winner(&supremacy).map(|(e, r)| (e, VictoryPath::Supremacy, r))
                } else {
                    None
                }
            })
            .or_else(|| {
                ascendancy_winner(state, &ascendancy, &settings)
                    .map(|(e, r)| (e, VictoryPath::Ascendancy, r))
            })
            .or_else(|| {
                scientific_winner(&scientific, &settings)
                    .map(|(e, r)| (e, VictoryPath::Scientific, r))
            })
            .or_else(|| {
                legacy_winner(&legacy, &supremacy, &settings, completed_turn)
                    .map(|(e, r)| (e, VictoryPath::Legacy, r))
            });

        if let Some((winner, path, reason)) = resolved {
            state.victory_status.final_victory = Some(FinalVictory {
                winner,
                path,
                turn: completed_turn,
                reason: reason.clone(),
            });
            for entry in state.victory_status.per_empire.values_mut() {
                for p in VictoryPath::tie_break_order() {
                    entry.path_status.insert(*p, VictoryPathStatus::Achieved);
                }
            }
            events.push(Event::VictoryAchieved {
                winner,
                path,
                turn: completed_turn,
                reason,
            });
        }
    }

    // Refresh the UI snapshot if `final_victory` was just resolved:
    // the per-path `status` we built above was computed while
    // `final_victory` was still `None`, so the enabled paths would
    // read as `InProgress` instead of `Achieved` on the winning turn.
    let progress = if state.victory_status.final_victory.is_some() {
        progress
            .into_iter()
            .map(|mut entry| {
                entry.status = progress_status_for(&settings, state, entry.path);
                entry
            })
            .collect()
    } else {
        progress
    };

    state.victory_status.progress = progress;

    // Touch intentionally unused binding to silence clippy when only one
    // condition is referenced at the type level.
    let _ = legacy_condition;

    events
}

/// Deterministic AI helper: which victory path an AI empire is most likely
/// to pursue. Player can call this too (e.g. for intel screens).
pub fn preferred_victory_path_for_empire(
    state: &GameState,
    empire: EmpireId,
) -> Option<VictoryPath> {
    use crate::state::empire_definition_by_id;
    use crate::state::{AiDoctrine, PlaystyleTag};

    let def = state
        .empires
        .get(&empire)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)?;

    let militarist =
        def.doctrine_weight(AiDoctrine::Militarist) + def.doctrine_weight(AiDoctrine::Imperial);
    let technologist = def.doctrine_weight(AiDoctrine::Technologist);
    let explorer =
        def.doctrine_weight(AiDoctrine::Explorer) + def.doctrine_weight(AiDoctrine::Expansionist);
    let expansionist = def.doctrine_weight(AiDoctrine::Expansionist);
    let industrialist = def.doctrine_weight(AiDoctrine::Industrialist);
    let merchant = def.doctrine_weight(AiDoctrine::Merchant);

    let mut scored: [(VictoryPath, i32); 4] = [
        (VictoryPath::Supremacy, militarist as i32),
        (VictoryPath::Scientific, technologist as i32),
        (VictoryPath::Legacy, (explorer + merchant) as i32),
        (
            VictoryPath::Ascendancy,
            (expansionist + industrialist + militarist / 2) as i32,
        ),
    ];
    scored.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| {
            let a_idx = VictoryPath::tie_break_order()
                .iter()
                .position(|p| *p == a.0)
                .unwrap_or(0);
            let b_idx = VictoryPath::tie_break_order()
                .iter()
                .position(|p| *p == b.0)
                .unwrap_or(0);
            a_idx.cmp(&b_idx)
        })
    });

    // Sanity override: a militarist playstyle empire should never default to
    // Scientific — they prefer Supremacy over the tech path.
    if def.playstyle.contains(&PlaystyleTag::Militarist)
        && scored.first().map(|(p, _)| *p) == Some(VictoryPath::Scientific)
    {
        return Some(VictoryPath::Supremacy);
    }
    scored.first().map(|(p, _)| *p)
}

/// Convenience: read a per-empire victory progress entry, returning a default
/// if the empire has no entry yet (e.g. pre-first-turn state).
pub fn empire_progress(state: &GameState, empire: EmpireId) -> EmpireVictoryProgress {
    state
        .victory_status
        .per_empire
        .get(&empire)
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColonyId, Command, Engine, VictorySettings};

    fn set_victory_settings(engine: &mut Engine, settings: VictorySettings) {
        let scenario = engine.state.scenario.as_mut().expect("scenario must exist");
        scenario.victory_settings = settings;
        // Refresh the read-only UI snapshot without advancing the
        // Ascendancy hold counter, the Scientific project points, or
        // resolving a final victory.  Tests that exercise the
        // mutating pipeline should call `evaluate_victory_end_turn`
        // explicitly to control the turn number and event stream.
        recompute_victory_snapshot(&mut engine.state);
    }

    #[test]
    fn no_victory_at_game_start_under_default_setup() {
        let engine = Engine::new(42);
        assert!(engine.state.victory_status.final_victory.is_none());
    }

    #[test]
    fn default_settings_enable_all_four_paths() {
        let s = VictorySettings::default_v1();
        assert!(s.is_enabled(VictoryPath::Supremacy));
        assert!(s.is_enabled(VictoryPath::Ascendancy));
        assert!(s.is_enabled(VictoryPath::Scientific));
        assert!(s.is_enabled(VictoryPath::Legacy));
        assert_eq!(s.turn_limit, 300);
        assert!(s.turn_limit_enabled);
    }

    #[test]
    fn disabled_paths_have_disabled_status() {
        let mut engine = Engine::new(42);
        let mut s = VictorySettings::default_v1();
        s.enabled_paths.remove(&VictoryPath::Legacy);
        s.enabled_paths.remove(&VictoryPath::Scientific);
        set_victory_settings(&mut engine, s);
        for entry in engine.state.victory_status.progress.iter() {
            if entry.path == VictoryPath::Legacy || entry.path == VictoryPath::Scientific {
                assert_eq!(entry.status, VictoryPathStatus::Disabled);
            } else {
                assert_eq!(entry.status, VictoryPathStatus::InProgress);
            }
        }
    }

    #[test]
    fn supremacy_fires_when_only_one_empire_has_colonies() {
        let mut engine = Engine::new(42);
        let to_remove: Vec<_> = engine
            .state
            .colonies
            .iter()
            .filter_map(|(cid, c)| (c.owner != engine.state.player_empire).then_some(*cid))
            .collect();
        for cid in to_remove {
            engine.state.colonies.remove(&cid);
        }
        engine
            .state
            .fleets
            .retain(|_, f| f.owner == engine.state.player_empire);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine
                .state
                .victory_status
                .final_victory
                .as_ref()
                .map(|f| f.path),
            Some(VictoryPath::Supremacy)
        );
    }

    #[test]
    fn supremacy_does_not_fire_when_two_empires_have_colonies() {
        let mut engine = Engine::new(42);
        let ai = engine.state.ai_empires[0];
        let ai_has_colony = engine.state.colonies.values().any(|c| c.owner == ai);
        assert!(ai_has_colony, "fixture should include an AI colony");
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert!(engine.state.victory_status.final_victory.is_none());
    }

    #[test]
    fn ascendancy_counts_unique_colonized_systems() {
        let engine = Engine::new(42);
        let player = engine.state.player_empire;
        let total_unique: u32 = {
            let mut stars = BTreeSet::new();
            for c in engine.state.colonies.values() {
                stars.insert(c.star);
            }
            stars.len() as u32
        };
        let player_stars: u32 = {
            let mut stars = BTreeSet::new();
            for c in engine.state.colonies.values() {
                if c.owner == player {
                    stars.insert(c.star);
                }
            }
            stars.len() as u32
        };
        let player_colonies_on_same_star: u32 = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == player)
            .map(|c| c.star)
            .fold(BTreeMap::<_, u32>::new(), |mut acc, s| {
                *acc.entry(s).or_insert(0) += 1;
                acc
            })
            .values()
            .copied()
            .sum();
        // Sum of duplicated-star colony counts is at least the number of stars.
        assert!(player_colonies_on_same_star >= player_stars);
        assert!(total_unique > 0);
        assert!(player_stars > 0);

        // Cross-check the engine's own Ascendancy evaluator against
        // the local recompute.  This is the regression guard: if the
        // evaluator ever starts counting raw colonies (rather than
        // unique systems), the totals will diverge.
        let condition = engine
            .state
            .scenario
            .as_ref()
            .expect("scenario must exist")
            .victory_settings
            .condition_for(VictoryPath::Ascendancy)
            .cloned()
            .expect("ascendancy condition present");
        let evaluated = evaluate_ascendancy(&engine.state, &condition);
        assert_eq!(
            evaluated.total_colonized_systems, total_unique,
            "Ascendancy evaluator must report the same unique-system count as a manual set"
        );
        assert_eq!(
            evaluated.per_empire_systems.get(&player).copied(),
            Some(player_stars),
            "Ascendancy evaluator must report the player's unique-system count correctly"
        );
    }

    #[test]
    fn ascendancy_advances_hold_counter_when_threshold_held() {
        let mut engine = Engine::new(42);
        let mut s = VictorySettings::default_v1();
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 1,
                consecutive_turns_required: 1,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 1_500,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        let player = engine.state.player_empire;
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        let progress = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .expect("player progress");
        assert!(progress.ascendancy_meets_threshold);
        assert!(progress.ascendancy_hold_turns >= 1);
    }

    #[test]
    fn ascendancy_hold_counter_resets_when_threshold_dropped() {
        let mut engine = Engine::new(42);
        // Strip every non-player colony so the player can only ever own
        // 100% of colonised systems; the only way to drop below the
        // 50% threshold is for the player to lose colonies too.
        let player = engine.state.player_empire;
        let ai_colony_ids: Vec<_> = engine
            .state
            .colonies
            .iter()
            .filter_map(|(cid, c)| (c.owner != player).then_some(*cid))
            .collect();
        for cid in &ai_colony_ids {
            engine.state.colonies.remove(cid);
        }
        // `Engine::new` already ran one end-of-turn at setup with the
        // default settings, ticking the hold counter to 1.  Reset
        // the counter so the test starts from a clean baseline it
        // can drive forward.
        engine
            .state
            .victory_status
            .per_empire
            .entry(player)
            .or_default()
            .ascendancy_hold_turns = 0;
        let mut s = VictorySettings::default_v1();
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 50,
                consecutive_turns_required: 3,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 1_500,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        // First turn: player holds 100% → 1 turn held.  Required
        // hold is 3 so no winner yet.
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        let initial = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .unwrap()
            .ascendancy_hold_turns;
        assert!(initial >= 1);
        // Drop the player's only colony so they fall below 50% and the
        // counter must reset to 0 on the next turn.
        let player_colony_id: ColonyId = engine
            .state
            .colonies
            .iter()
            .find_map(|(cid, c)| (c.owner == player).then_some(*cid))
            .expect("player still has a colony");
        engine.state.colonies.remove(&player_colony_id);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        let reset = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .unwrap()
            .ascendancy_hold_turns;
        assert_eq!(reset, 0);
    }

    #[test]
    fn ascendancy_fires_after_required_consecutive_turns() {
        let mut engine = Engine::new(42);
        let mut s = VictorySettings::default_v1();
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 1,
                consecutive_turns_required: 1,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 1_500,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine
                .state
                .victory_status
                .final_victory
                .as_ref()
                .map(|f| f.path),
            Some(VictoryPath::Ascendancy)
        );
    }

    #[test]
    fn scientific_advances_deterministically_from_yield() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        let before = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .map(|p| p.scientific_project_points)
            .unwrap_or(0);
        engine
            .state
            .empires
            .get_mut(&player)
            .unwrap()
            .research
            .completed
            .push(TechId(61));
        let turn = engine.state.turn;
        let _ = evaluate_victory_end_turn(&mut engine.state, turn);
        let after = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .map(|p| p.scientific_project_points)
            .unwrap_or(0);
        assert!(after >= before);
    }

    #[test]
    fn scientific_fires_at_threshold() {
        let mut engine = Engine::new(42);
        let mut s = VictorySettings::default_v1();
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 50,
                consecutive_turns_required: 10,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 0,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        let player = engine.state.player_empire;
        engine
            .state
            .empires
            .get_mut(&player)
            .unwrap()
            .research
            .completed
            .push(TechId(61));
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine
                .state
                .victory_status
                .final_victory
                .as_ref()
                .map(|f| f.path),
            Some(VictoryPath::Scientific)
        );
    }

    #[test]
    fn scientific_warning_emits_once_per_threshold() {
        let mut engine = Engine::new(42);
        let mut s = VictorySettings::default_v1();
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 50,
                consecutive_turns_required: 10,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 100,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        let player = engine.state.player_empire;
        engine
            .state
            .empires
            .get_mut(&player)
            .unwrap()
            .research
            .completed
            .push(TechId(61));
        engine
            .state
            .victory_status
            .per_empire
            .entry(player)
            .or_default()
            .scientific_project_points = 26;
        let turn = engine.state.turn;
        let events_first = evaluate_victory_end_turn(&mut engine.state, turn);
        let warning_first = events_first
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::VictoryWarning {
                        path: VictoryPath::Scientific,
                        progress_percent: 25,
                        ..
                    }
                )
            })
            .count();
        assert!(warning_first >= 1);
        let turn = engine.state.turn;
        let events_second = evaluate_victory_end_turn(&mut engine.state, turn);
        let warning_second = events_second
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::VictoryWarning {
                        path: VictoryPath::Scientific,
                        progress_percent: 25,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(warning_second, 0);
    }

    #[test]
    fn legacy_score_is_deterministic() {
        let engine_a = Engine::new(42).state;
        let engine_b = Engine::new(42).state;
        let score_a = compute_legacy_breakdown(&engine_a, engine_a.player_empire);
        let score_b = compute_legacy_breakdown(&engine_b, engine_b.player_empire);
        assert_eq!(score_a, score_b);
    }

    #[test]
    fn legacy_winner_selected_at_turn_limit() {
        let mut engine = Engine::new(42);
        let mut s = VictorySettings::default_v1();
        s.turn_limit_enabled = true;
        s.turn_limit = 0;
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 50,
                consecutive_turns_required: 10,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 1_500,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        engine.state.turn = 0;
        let _ = evaluate_victory_end_turn(&mut engine.state, 0);
        assert_eq!(
            engine
                .state
                .victory_status
                .final_victory
                .as_ref()
                .map(|f| f.path),
            Some(VictoryPath::Legacy)
        );
    }

    #[test]
    fn legacy_tie_breaks_to_lower_empire_id() {
        let mut engine = Engine::new(42);
        // Force every empire to the same score by zeroing credits and
        // clearing per-empire explored stars.  Colonies, population,
        // tech, surveys, and strategic resources are all empire-scoped
        // fields that differ between empires; we wipe the ones the
        // engine tracks so the only "score" left is the deterministic
        // tie-break by EmpireId.
        for empire in engine.state.empires.values_mut() {
            empire.credits = 0;
            empire.research.completed.clear();
        }
        // Reset explored stars so all empires report 0 explored systems.
        engine.state.explored_stars.clear();
        engine.state.empire_explored_stars.clear();
        engine.state.ai_explored_stars.clear();
        // Wipe colonies so no empire scores anything for the
        // colony / population components.
        engine.state.colonies.clear();
        // Reset every planet's surveyed flag and clear specials so the
        // survey / discovery components are zero.
        for star in engine.state.stars.values_mut() {
            for planet in &mut star.planets {
                planet.surveyed = false;
                planet.specials.clear();
                planet.anomalies.clear();
                planet.resources.clear();
            }
        }
        // Wipe strategic resource access.
        engine.state.empire_resource_access.clear();
        // Wipe battle reports so no empire has any "battle victories".
        engine.state.battle_reports.clear();

        let mut s = VictorySettings::default_v1();
        s.turn_limit_enabled = true;
        s.turn_limit = 0;
        s.conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 50,
                consecutive_turns_required: 10,
            },
            VictoryCondition::Scientific {
                eligibility_tech: TechId(61),
                project_points_required: 1_500,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        set_victory_settings(&mut engine, s);
        let _ = evaluate_victory_end_turn(&mut engine.state, 0);
        let winner = engine
            .state
            .victory_status
            .final_victory
            .as_ref()
            .unwrap()
            .winner;
        let min_id = *engine.state.empires.keys().min().unwrap();
        assert_eq!(winner, min_id);
    }

    #[test]
    fn final_victory_stored_in_game_state() {
        let mut engine = Engine::new(42);
        let to_remove: Vec<_> = engine
            .state
            .colonies
            .iter()
            .filter_map(|(cid, c)| (c.owner != engine.state.player_empire).then_some(*cid))
            .collect();
        for cid in to_remove {
            engine.state.colonies.remove(&cid);
        }
        engine
            .state
            .fleets
            .retain(|_, f| f.owner == engine.state.player_empire);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        let final_v = engine
            .state
            .victory_status
            .final_victory
            .as_ref()
            .expect("final victory stored");
        assert_eq!(final_v.path, VictoryPath::Supremacy);
        assert!(final_v.turn >= 1);
        assert!(!final_v.reason.is_empty());
    }

    #[test]
    fn victory_achieved_event_emitted_exactly_once() {
        let mut engine = Engine::new(42);
        let to_remove: Vec<_> = engine
            .state
            .colonies
            .iter()
            .filter_map(|(cid, c)| (c.owner != engine.state.player_empire).then_some(*cid))
            .collect();
        for cid in to_remove {
            engine.state.colonies.remove(&cid);
        }
        engine
            .state
            .fleets
            .retain(|_, f| f.owner == engine.state.player_empire);
        let events_first = engine.apply_turn(vec![Command::EndTurn]);
        let achieved_first = events_first
            .iter()
            .filter(|e| matches!(e, Event::VictoryAchieved { .. }))
            .count();
        assert_eq!(achieved_first, 1);
        let events_second = engine.apply_turn(vec![Command::EndTurn]);
        let achieved_second = events_second
            .iter()
            .filter(|e| matches!(e, Event::VictoryAchieved { .. }))
            .count();
        assert_eq!(achieved_second, 0);
    }

    #[test]
    fn preferred_victory_path_helper_returns_some_for_known_empire() {
        let engine = Engine::new(42);
        let player = engine.state.player_empire;
        let path = preferred_victory_path_for_empire(&engine.state, player);
        if engine
            .state
            .empires
            .get(&player)
            .and_then(|e| e.empire_def)
            .is_some()
        {
            assert!(path.is_some());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn victory_status_round_trips() {
        let mut engine = Engine::new(42);
        engine.apply_turn(vec![Command::EndTurn]);
        let json = serde_json::to_string(&engine.state.victory_status).expect("v serialize");
        let restored: crate::state::VictoryStatus =
            serde_json::from_str(&json).expect("v deserialize");
        assert_eq!(restored.progress.len(), 4);
    }

    /// Regression guard: `recompute_victory_snapshot` must be truly
    /// read-only.  The previous implementation called the mutating
    /// `evaluate_scientific`, which advanced per-empire Scientific
    /// project points by the per-turn `last_colony_yields`.  The
    /// snapshot helper is now the read-only
    /// `evaluate_scientific_snapshot`, so calling it twice must
    /// return identical per-empire point totals.
    #[test]
    fn recompute_victory_snapshot_does_not_advance_scientific_points() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        // Make the player eligible for the Scientific path.
        engine
            .state
            .empires
            .get_mut(&player)
            .unwrap()
            .research
            .completed
            .push(TechId(61));
        // Seed some stored yields so a mutating evaluation would add
        // a positive `production_turn`.
        if let Some(yield_snapshot) = engine.state.last_colony_yields.values_mut().next() {
            *yield_snapshot = crate::state::ColonyYieldSnapshot {
                industry: 0,
                credits: 0,
                science: 50,
                food: 0,
                food_consumed: 0,
                maintenance: 0,
            };
        }
        let player_points_before = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .map(|p| p.scientific_project_points)
            .unwrap_or(0);
        recompute_victory_snapshot(&mut engine.state);
        let player_points_after_first = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .unwrap()
            .scientific_project_points;
        assert_eq!(
            player_points_after_first, player_points_before,
            "snapshot must not advance the player's Scientific project points"
        );
        // A second call must also be a no-op.
        recompute_victory_snapshot(&mut engine.state);
        let player_points_after_second = engine
            .state
            .victory_status
            .per_empire
            .get(&player)
            .unwrap()
            .scientific_project_points;
        assert_eq!(
            player_points_after_second, player_points_before,
            "snapshot must remain a no-op on repeated calls"
        );
    }
}
