use crate::events::Event;
use crate::state::{
    tech_by_id, EmpireId, GameState, RelationshipStatus, TechId, VictoryCondition, VictoryPath,
    VictoryProgress, VictoryProgressValue, VictorySettings, VictoryStatus,
};
use std::collections::{BTreeMap, BTreeSet};

const MILESTONES: [u8; 4] = [25, 50, 75, 100];

struct PathEvaluation {
    progress: VictoryProgress,
    achievers: Vec<EmpireId>,
}

fn percent(numerator: u64, denominator: u64) -> u8 {
    if denominator == 0 {
        return 0;
    }
    ((numerator.saturating_mul(100)) / denominator).min(100) as u8
}

fn ratio_percent(min_num: i64, min_den: i64) -> u8 {
    if min_den <= 0 {
        return 0;
    }
    ((min_num.max(0) as u64).saturating_mul(100) / min_den as u64).min(100) as u8
}

fn threshold_percent(current: i64, required: i64) -> u8 {
    if current >= required {
        100
    } else if required <= 0 {
        0
    } else {
        ratio_percent(current.max(0), required)
    }
}

fn empire_candidates(state: &GameState) -> Vec<EmpireId> {
    state.empires.keys().copied().collect()
}

fn explored_stars_for_empire(
    state: &GameState,
    empire: EmpireId,
) -> &BTreeSet<crate::state::StarId> {
    if empire == state.player_empire {
        &state.explored_stars
    } else {
        &state.ai_explored_stars
    }
}

fn evaluate_dominion(
    state: &GameState,
    condition: &VictoryCondition,
    enabled: bool,
) -> PathEvaluation {
    let (required_percent, allow_elimination) = match condition {
        VictoryCondition::Dominion {
            control_percent_required,
            allow_elimination,
        } => (*control_percent_required, *allow_elimination),
        _ => (100, false),
    };

    let mut colonized_by_empire: BTreeMap<EmpireId, BTreeSet<crate::state::StarId>> =
        BTreeMap::new();
    for colony in state.colonies.values() {
        colonized_by_empire
            .entry(colony.owner)
            .or_default()
            .insert(colony.star);
    }
    let total_colonized = colonized_by_empire
        .values()
        .flat_map(|stars| stars.iter().copied())
        .collect::<BTreeSet<_>>()
        .len() as u64;

    let mut leaders: Vec<(u8, EmpireId)> = empire_candidates(state)
        .into_iter()
        .map(|empire| {
            let controlled = colonized_by_empire
                .get(&empire)
                .map(|stars| stars.len() as u64)
                .unwrap_or(0);
            (percent(controlled, total_colonized.max(1)), empire)
        })
        .collect();
    leaders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (leading_percent, leading_empire) = leaders
        .first()
        .map(|(score, empire)| (*score, Some(*empire)))
        .unwrap_or((0, None));

    let active_major = colonized_by_empire
        .iter()
        .filter(|(_, stars)| !stars.is_empty())
        .count() as u32;
    let achievers = if !enabled {
        Vec::new()
    } else {
        empire_candidates(state)
            .into_iter()
            .filter(|empire| {
                let control_pct = leaders
                    .iter()
                    .find(|(_, id)| id == empire)
                    .map(|(value, _)| *value)
                    .unwrap_or(0);
                let control_win = control_pct >= required_percent;
                let elimination_win = allow_elimination
                    && active_major <= 1
                    && colonized_by_empire
                        .get(empire)
                        .map(|stars| !stars.is_empty())
                        .unwrap_or(false);
                control_win || elimination_win
            })
            .collect()
    };

    PathEvaluation {
        progress: VictoryProgress {
            path: VictoryPath::Dominion,
            enabled,
            condition: condition.clone(),
            value: VictoryProgressValue::Dominion {
                controlled_systems: leading_empire
                    .and_then(|empire| {
                        colonized_by_empire
                            .get(&empire)
                            .map(|stars| stars.len() as u32)
                    })
                    .unwrap_or(0),
                total_colonized_systems: total_colonized as u32,
                control_percent: leading_percent,
                active_major_empires: active_major,
            },
            progress_percent: percent(leading_percent as u64, required_percent.max(1) as u64),
            achieved: !achievers.is_empty(),
            leading_empire,
        },
        achievers,
    }
}

fn evaluate_ascendancy(
    state: &GameState,
    condition: &VictoryCondition,
    enabled: bool,
) -> PathEvaluation {
    let empty: Vec<TechId> = Vec::new();
    let (required_techs, relevant_techs) = match condition {
        VictoryCondition::Ascendancy {
            required_victory_techs,
            victory_tech_ids,
        } => (*required_victory_techs, victory_tech_ids),
        _ => (1, &empty),
    };
    let usable_relevant: Vec<TechId> = relevant_techs
        .iter()
        .copied()
        .filter(|tech| tech_by_id(*tech).is_some())
        .collect();

    let mut counts: Vec<(u32, EmpireId)> = empire_candidates(state)
        .into_iter()
        .map(|empire| {
            let completed = state
                .empires
                .get(&empire)
                .map(|e| {
                    usable_relevant
                        .iter()
                        .filter(|tech| e.research.completed.contains(tech))
                        .count() as u32
                })
                .unwrap_or(0);
            (completed, empire)
        })
        .collect();
    counts.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (leading_count, leading_empire) = counts
        .first()
        .map(|(score, empire)| (*score, Some(*empire)))
        .unwrap_or((0, None));
    let achievers = if !enabled {
        Vec::new()
    } else {
        counts
            .iter()
            .filter(|(count, _)| *count >= required_techs)
            .map(|(_, empire)| *empire)
            .collect()
    };

    PathEvaluation {
        progress: VictoryProgress {
            path: VictoryPath::Ascendancy,
            enabled,
            condition: condition.clone(),
            value: VictoryProgressValue::Ascendancy {
                completed_victory_techs: leading_count,
                required_victory_techs: required_techs,
            },
            progress_percent: percent(leading_count as u64, required_techs.max(1) as u64),
            achieved: !achievers.is_empty(),
            leading_empire,
        },
        achievers,
    }
}

fn evaluate_prosperity(
    state: &GameState,
    condition: &VictoryCondition,
    enabled: bool,
) -> PathEvaluation {
    let (
        population_required,
        credits_required,
        connected_required,
        stability_required,
        food_required,
    ) = match condition {
        VictoryCondition::Prosperity {
            population_required,
            credits_required,
            connected_colonies_required,
            avg_stability_required,
            food_surplus_required,
        } => (
            *population_required,
            *credits_required,
            *connected_colonies_required,
            *avg_stability_required,
            *food_surplus_required,
        ),
        _ => (1, 0, 1, 0, None),
    };

    let mut empire_stats: Vec<(i64, EmpireId, VictoryProgressValue, bool)> =
        empire_candidates(state)
            .into_iter()
            .map(|empire| {
                let colonies: Vec<_> = state
                    .colonies
                    .values()
                    .filter(|colony| colony.owner == empire)
                    .collect();
                let population = colonies.iter().map(|c| c.population).sum::<u64>();
                let connected = colonies
                    .iter()
                    .filter(|c| {
                        state.colony_supply_state(c.id)
                            == crate::state::ColonySupplyState::Connected
                    })
                    .count() as u32;
                let avg_stability = if colonies.is_empty() {
                    0
                } else {
                    let effective_stability_sum = colonies
                        .iter()
                        .map(|c| {
                            let unrest_penalty = match state.colony_unrest_state(c.id) {
                                crate::state::ColonyUnrestState::Calm => 0u32,
                                crate::state::ColonyUnrestState::Strained => 5u32,
                                crate::state::ColonyUnrestState::Unrest => 12u32,
                                crate::state::ColonyUnrestState::RevoltRisk => 20u32,
                            };
                            u32::from(c.stability).saturating_sub(unrest_penalty)
                        })
                        .sum::<u32>();
                    (effective_stability_sum / colonies.len() as u32) as u8
                };
                let (credits, food) = state
                    .empires
                    .get(&empire)
                    .map(|e| (e.credits, e.food))
                    .unwrap_or((0, 0));
                let meets = population >= population_required
                    && credits >= credits_required
                    && connected >= connected_required
                    && avg_stability >= stability_required
                    && food_required
                        .map(|required| food >= required)
                        .unwrap_or(true);

                let progress_floor = [
                    ratio_percent(population as i64, population_required.max(1) as i64),
                    threshold_percent(credits, credits_required),
                    ratio_percent(connected as i64, connected_required.max(1) as i64),
                    ratio_percent(avg_stability as i64, stability_required.max(1) as i64),
                    food_required
                        .map(|required| threshold_percent(food, required))
                        .unwrap_or(100),
                ]
                .into_iter()
                .min()
                .unwrap_or(0) as i64;

                (
                    progress_floor,
                    empire,
                    VictoryProgressValue::Prosperity {
                        population,
                        population_required,
                        credits,
                        credits_required,
                        connected_colonies: connected,
                        connected_colonies_required: connected_required,
                        avg_stability,
                        avg_stability_required: stability_required,
                        food_surplus: food,
                        food_surplus_required: food_required,
                    },
                    meets,
                )
            })
            .collect();
    empire_stats.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let leading_empire = empire_stats.first().map(|(_, empire, _, _)| *empire);
    let leading_progress = empire_stats
        .first()
        .map(|(progress, _, _, _)| (*progress).clamp(0, 100) as u8)
        .unwrap_or(0);
    let leading_value = empire_stats
        .first()
        .map(|(_, _, value, _)| value.clone())
        .unwrap_or(VictoryProgressValue::Prosperity {
            population: 0,
            population_required,
            credits: 0,
            credits_required,
            connected_colonies: 0,
            connected_colonies_required: connected_required,
            avg_stability: 0,
            avg_stability_required: stability_required,
            food_surplus: 0,
            food_surplus_required: food_required,
        });
    let achievers = if !enabled {
        Vec::new()
    } else {
        empire_stats
            .iter()
            .filter(|(_, _, _, meets)| *meets)
            .map(|(_, empire, _, _)| *empire)
            .collect()
    };

    PathEvaluation {
        progress: VictoryProgress {
            path: VictoryPath::Prosperity,
            enabled,
            condition: condition.clone(),
            value: leading_value,
            progress_percent: leading_progress,
            achieved: !achievers.is_empty(),
            leading_empire,
        },
        achievers,
    }
}

fn evaluate_discovery(
    state: &GameState,
    condition: &VictoryCondition,
    enabled: bool,
) -> PathEvaluation {
    let empty: Vec<TechId> = Vec::new();
    let (required_system_pct, required_planet_pct, required_techs) = match condition {
        VictoryCondition::Discovery {
            systems_explored_percent_required,
            planets_surveyed_percent_required,
            required_tech_ids,
        } => (
            *systems_explored_percent_required,
            *planets_surveyed_percent_required,
            required_tech_ids,
        ),
        _ => (100, 100, &empty),
    };
    let total_systems = state.stars.len() as u64;
    let total_planets = state
        .stars
        .values()
        .map(|star| star.planets.len() as u64)
        .sum::<u64>();

    let mut scores: Vec<(u8, EmpireId, VictoryProgressValue, bool)> = empire_candidates(state)
        .into_iter()
        .map(|empire| {
            let explored = explored_stars_for_empire(state, empire);
            let explored_pct = percent(explored.len() as u64, total_systems.max(1));
            let surveyed_in_known = state
                .stars
                .values()
                .filter(|star| explored.contains(&star.id))
                .flat_map(|star| star.planets.iter())
                .filter(|planet| planet.surveyed)
                .count() as u64;
            let surveyed_pct = percent(surveyed_in_known, total_planets.max(1));
            let tech_done = state
                .empires
                .get(&empire)
                .map(|e| {
                    required_techs
                        .iter()
                        .filter(|tech| e.research.completed.contains(tech))
                        .count() as u32
                })
                .unwrap_or(0);
            let tech_required = required_techs.len() as u32;
            let tech_ok = tech_required == 0 || tech_done >= tech_required;
            let meets = explored_pct >= required_system_pct
                && surveyed_pct >= required_planet_pct
                && tech_ok;
            let aggregate = [
                ratio_percent(explored_pct as i64, required_system_pct.max(1) as i64),
                ratio_percent(surveyed_pct as i64, required_planet_pct.max(1) as i64),
                if tech_required == 0 {
                    100
                } else {
                    ratio_percent(tech_done as i64, tech_required as i64)
                },
            ]
            .into_iter()
            .min()
            .unwrap_or(0);

            (
                aggregate,
                empire,
                VictoryProgressValue::Discovery {
                    explored_systems_percent: explored_pct,
                    required_explored_systems_percent: required_system_pct,
                    surveyed_planets_percent: surveyed_pct,
                    required_surveyed_planets_percent: required_planet_pct,
                    required_techs_total: tech_required,
                    required_techs_completed: tech_done,
                },
                meets,
            )
        })
        .collect();
    scores.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let leading_empire = scores.first().map(|(_, empire, _, _)| *empire);
    let leading_progress = scores.first().map(|(v, _, _, _)| *v).unwrap_or(0);
    let leading_value = scores
        .first()
        .map(|(_, _, value, _)| value.clone())
        .unwrap_or(VictoryProgressValue::Discovery {
            explored_systems_percent: 0,
            required_explored_systems_percent: required_system_pct,
            surveyed_planets_percent: 0,
            required_surveyed_planets_percent: required_planet_pct,
            required_techs_total: required_techs.len() as u32,
            required_techs_completed: 0,
        });
    let achievers = if !enabled {
        Vec::new()
    } else {
        scores
            .iter()
            .filter(|(_, _, _, meets)| *meets)
            .map(|(_, empire, _, _)| *empire)
            .collect()
    };

    PathEvaluation {
        progress: VictoryProgress {
            path: VictoryPath::Discovery,
            enabled,
            condition: condition.clone(),
            value: leading_value,
            progress_percent: leading_progress,
            achieved: !achievers.is_empty(),
            leading_empire,
        },
        achievers,
    }
}

fn evaluate_unity(
    state: &GameState,
    condition: &VictoryCondition,
    enabled: bool,
) -> PathEvaluation {
    let (contacted_required, non_war_required, connected_required) = match condition {
        VictoryCondition::Unity {
            contacted_empires_required,
            non_war_relations_required,
            connected_colonies_required,
        } => (
            *contacted_empires_required,
            *non_war_relations_required,
            *connected_colonies_required,
        ),
        _ => (1, 1, 1),
    };
    let player = state.player_empire;
    let contacted = state
        .diplomacy
        .values()
        .filter(|status| **status != RelationshipStatus::Unknown)
        .count() as u32;
    let non_war = state
        .diplomacy
        .values()
        .filter(|status| {
            matches!(
                status,
                RelationshipStatus::Contacted
                    | RelationshipStatus::Neutral
                    | RelationshipStatus::Tense
            )
        })
        .count() as u32;
    let connected = state
        .colonies
        .values()
        .filter(|c| c.owner == player)
        .filter(|c| state.colony_supply_state(c.id) == crate::state::ColonySupplyState::Connected)
        .count() as u32;
    let meets = enabled
        && contacted_required > 0
        && non_war_required > 0
        && contacted >= contacted_required
        && non_war >= non_war_required
        && connected >= connected_required;
    let progress = [
        ratio_percent(contacted as i64, contacted_required.max(1) as i64),
        ratio_percent(non_war as i64, non_war_required.max(1) as i64),
        ratio_percent(connected as i64, connected_required.max(1) as i64),
    ]
    .into_iter()
    .min()
    .unwrap_or(0);
    let leading_empire = if progress > 0 { Some(player) } else { None };

    PathEvaluation {
        progress: VictoryProgress {
            path: VictoryPath::Unity,
            enabled,
            condition: condition.clone(),
            value: VictoryProgressValue::Unity {
                contacted_empires: contacted,
                contacted_empires_required: contacted_required,
                non_war_relations: non_war,
                non_war_relations_required: non_war_required,
                connected_colonies: connected,
                connected_colonies_required: connected_required,
            },
            progress_percent: progress,
            achieved: meets,
            leading_empire,
        },
        achievers: if meets { vec![player] } else { Vec::new() },
    }
}

fn choose_winner(evaluations: &[PathEvaluation]) -> Option<(EmpireId, VictoryPath)> {
    for path in VictoryPath::tie_break_order() {
        if let Some(eval) = evaluations.iter().find(|eval| eval.progress.path == *path) {
            if !eval.progress.enabled {
                continue;
            }
            if eval.achievers.is_empty() {
                continue;
            }
            let winner = *eval.achievers.iter().min()?;
            return Some((winner, *path));
        }
    }
    None
}

pub fn evaluate_victory_end_turn(state: &mut GameState, completed_turn: u32) -> Vec<Event> {
    let settings = state
        .scenario
        .as_ref()
        .map(|scenario| scenario.victory_settings.clone())
        .unwrap_or_else(VictorySettings::default_v1);

    let mut evaluations = Vec::new();
    for path in VictoryPath::tie_break_order() {
        let condition = settings
            .condition_for(*path)
            .cloned()
            .unwrap_or(match path {
                VictoryPath::Dominion => VictoryCondition::Dominion {
                    control_percent_required: 100,
                    allow_elimination: false,
                },
                VictoryPath::Ascendancy => VictoryCondition::Ascendancy {
                    required_victory_techs: 1,
                    victory_tech_ids: Vec::new(),
                },
                VictoryPath::Prosperity => VictoryCondition::Prosperity {
                    population_required: u64::MAX,
                    credits_required: i64::MAX,
                    connected_colonies_required: u32::MAX,
                    avg_stability_required: u8::MAX,
                    food_surplus_required: None,
                },
                VictoryPath::Discovery => VictoryCondition::Discovery {
                    systems_explored_percent_required: 100,
                    planets_surveyed_percent_required: 100,
                    required_tech_ids: Vec::new(),
                },
                VictoryPath::Unity => VictoryCondition::Unity {
                    contacted_empires_required: u32::MAX,
                    non_war_relations_required: u32::MAX,
                    connected_colonies_required: u32::MAX,
                },
            });
        let enabled = settings.is_enabled(*path);
        let evaluation = match path {
            VictoryPath::Dominion => evaluate_dominion(state, &condition, enabled),
            VictoryPath::Ascendancy => evaluate_ascendancy(state, &condition, enabled),
            VictoryPath::Prosperity => evaluate_prosperity(state, &condition, enabled),
            VictoryPath::Discovery => evaluate_discovery(state, &condition, enabled),
            VictoryPath::Unity => evaluate_unity(state, &condition, enabled),
        };
        evaluations.push(evaluation);
    }

    let previous = state.victory_status.clone();
    let mut status = VictoryStatus {
        progress: evaluations
            .iter()
            .map(|evaluation| evaluation.progress.clone())
            .collect(),
        winner: previous.winner,
        winning_path: previous.winning_path,
        turn_achieved: previous.turn_achieved,
        milestone_levels: previous.milestone_levels.clone(),
    };

    let mut events = Vec::new();
    let player = state.player_empire;
    for evaluation in &evaluations {
        if !evaluation.progress.enabled {
            continue;
        }
        if evaluation.progress.leading_empire != Some(player) {
            continue;
        }
        let previous_level = previous
            .milestone_levels
            .get(&evaluation.progress.path)
            .copied()
            .unwrap_or(0);
        let mut latest_level = previous_level;
        for milestone in MILESTONES {
            if evaluation.progress.progress_percent >= milestone && milestone > latest_level {
                latest_level = milestone;
                events.push(Event::VictoryProgressMilestone {
                    path: evaluation.progress.path,
                    empire: player,
                    progress_percent: milestone,
                });
            }
        }
        if latest_level > previous_level {
            status
                .milestone_levels
                .insert(evaluation.progress.path, latest_level);
        }
    }

    if status.winner.is_none() {
        if let Some((winner, path)) = choose_winner(&evaluations) {
            status.winner = Some(winner);
            status.winning_path = Some(path);
            status.turn_achieved = Some(completed_turn);
            events.push(Event::VictoryAchieved {
                winner,
                path,
                turn: completed_turn,
            });
        }
    }

    state.victory_status = status;
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Engine, VictoryCondition, VictorySettings};

    fn set_victory_settings(engine: &mut Engine, settings: VictorySettings) {
        let scenario = engine.state.scenario.as_mut().expect("scenario must exist");
        scenario.victory_settings = settings;
        let turn = engine.state.turn;
        let _ = evaluate_victory_end_turn(&mut engine.state, turn);
    }

    #[test]
    fn no_victory_at_game_start_under_default_setup() {
        let engine = Engine::new(42);
        assert!(engine.state.victory_status.winner.is_none());
    }

    #[test]
    fn dominion_control_victory_triggers() {
        let mut engine = Engine::new(42);
        let mut settings = VictorySettings::default_v1();
        settings.conditions = vec![
            VictoryCondition::Dominion {
                control_percent_required: 50,
                allow_elimination: false,
            },
            settings
                .condition_for(VictoryPath::Ascendancy)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Prosperity)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Discovery)
                .unwrap()
                .clone(),
            settings.condition_for(VictoryPath::Unity).unwrap().clone(),
        ];
        set_victory_settings(&mut engine, settings);
        let ai = engine.state.ai_empires[0];
        let ai_colony = engine
            .state
            .colonies
            .values()
            .find(|colony| colony.owner == ai)
            .map(|colony| colony.id)
            .unwrap();
        engine.state.colonies.get_mut(&ai_colony).unwrap().owner = engine.state.player_empire;
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine.state.victory_status.winning_path,
            Some(VictoryPath::Dominion)
        );
    }

    #[test]
    fn dominion_elimination_victory_triggers() {
        let mut engine = Engine::new(42);
        let mut settings = VictorySettings::default_v1();
        settings.conditions = vec![
            VictoryCondition::Dominion {
                control_percent_required: 100,
                allow_elimination: true,
            },
            settings
                .condition_for(VictoryPath::Ascendancy)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Prosperity)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Discovery)
                .unwrap()
                .clone(),
            settings.condition_for(VictoryPath::Unity).unwrap().clone(),
        ];
        set_victory_settings(&mut engine, settings);
        let ai = engine.state.ai_empires[0];
        let ai_colony_ids: Vec<_> = engine
            .state
            .colonies
            .values()
            .filter(|colony| colony.owner == ai)
            .map(|colony| colony.id)
            .collect();
        for colony_id in ai_colony_ids {
            engine.state.colonies.remove(&colony_id);
        }
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine.state.victory_status.winning_path,
            Some(VictoryPath::Dominion)
        );
    }

    #[test]
    fn ascendancy_victory_triggers() {
        let mut engine = Engine::new(42);
        let mut settings = VictorySettings::default_v1();
        settings.conditions = vec![
            settings
                .condition_for(VictoryPath::Dominion)
                .unwrap()
                .clone(),
            VictoryCondition::Ascendancy {
                required_victory_techs: 1,
                victory_tech_ids: vec![TechId(34)],
            },
            settings
                .condition_for(VictoryPath::Prosperity)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Discovery)
                .unwrap()
                .clone(),
            settings.condition_for(VictoryPath::Unity).unwrap().clone(),
        ];
        settings.enabled_paths = [VictoryPath::Ascendancy].into_iter().collect();
        set_victory_settings(&mut engine, settings);
        let player = engine.state.player_empire;
        engine
            .state
            .empires
            .get_mut(&player)
            .unwrap()
            .research
            .completed
            .push(TechId(34));
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine.state.victory_status.winning_path,
            Some(VictoryPath::Ascendancy)
        );
    }

    #[test]
    fn prosperity_victory_triggers() {
        let mut engine = Engine::new(42);
        let settings = VictorySettings {
            enabled_paths: [VictoryPath::Prosperity].into_iter().collect(),
            conditions: vec![
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Dominion)
                    .unwrap()
                    .clone(),
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Ascendancy)
                    .unwrap()
                    .clone(),
                VictoryCondition::Prosperity {
                    population_required: 1,
                    credits_required: 0,
                    connected_colonies_required: 1,
                    avg_stability_required: 1,
                    food_surplus_required: None,
                },
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Discovery)
                    .unwrap()
                    .clone(),
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Unity)
                    .unwrap()
                    .clone(),
            ],
        };
        set_victory_settings(&mut engine, settings);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine.state.victory_status.winning_path,
            Some(VictoryPath::Prosperity)
        );
    }

    #[test]
    fn discovery_victory_triggers() {
        let mut engine = Engine::new(42);
        let settings = VictorySettings {
            enabled_paths: [VictoryPath::Discovery].into_iter().collect(),
            conditions: vec![
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Dominion)
                    .unwrap()
                    .clone(),
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Ascendancy)
                    .unwrap()
                    .clone(),
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Prosperity)
                    .unwrap()
                    .clone(),
                VictoryCondition::Discovery {
                    systems_explored_percent_required: 0,
                    planets_surveyed_percent_required: 0,
                    required_tech_ids: Vec::new(),
                },
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Unity)
                    .unwrap()
                    .clone(),
            ],
        };
        set_victory_settings(&mut engine, settings);
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine.state.victory_status.winning_path,
            Some(VictoryPath::Discovery)
        );
    }

    #[test]
    fn unity_does_not_trigger_when_disabled() {
        let mut engine = Engine::new(42);
        let mut settings = VictorySettings::default_v1();
        settings.enabled_paths.remove(&VictoryPath::Unity);
        set_victory_settings(&mut engine, settings);
        let events = engine.apply_turn(vec![Command::EndTurn]);
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::VictoryAchieved {
                path: VictoryPath::Unity,
                ..
            }
        )));
    }

    #[test]
    fn unity_has_no_leader_when_progress_is_zero() {
        let engine = Engine::new(42);
        let unity = engine
            .state
            .victory_status
            .progress
            .iter()
            .find(|progress| progress.path == VictoryPath::Unity)
            .expect("unity progress should exist");
        assert_eq!(unity.progress_percent, 0);
        assert_eq!(unity.leading_empire, None);
    }

    #[test]
    fn disabled_victory_path_does_not_trigger() {
        let mut engine = Engine::new(42);
        let mut settings = VictorySettings::default_v1();
        settings.enabled_paths = [VictoryPath::Discovery].into_iter().collect();
        settings.conditions = vec![
            VictoryCondition::Dominion {
                control_percent_required: 1,
                allow_elimination: true,
            },
            settings
                .condition_for(VictoryPath::Ascendancy)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Prosperity)
                .unwrap()
                .clone(),
            settings
                .condition_for(VictoryPath::Discovery)
                .unwrap()
                .clone(),
            settings.condition_for(VictoryPath::Unity).unwrap().clone(),
        ];
        set_victory_settings(&mut engine, settings);
        let events = engine.apply_turn(vec![Command::EndTurn]);
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::VictoryAchieved {
                path: VictoryPath::Dominion,
                ..
            }
        )));
    }

    #[test]
    fn simultaneous_victories_use_tie_break_order() {
        let mut engine = Engine::new(42);
        let settings = VictorySettings {
            enabled_paths: [VictoryPath::Dominion, VictoryPath::Ascendancy]
                .into_iter()
                .collect(),
            conditions: vec![
                VictoryCondition::Dominion {
                    control_percent_required: 50,
                    allow_elimination: true,
                },
                VictoryCondition::Ascendancy {
                    required_victory_techs: 1,
                    victory_tech_ids: vec![TechId(34)],
                },
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Prosperity)
                    .unwrap()
                    .clone(),
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Discovery)
                    .unwrap()
                    .clone(),
                VictorySettings::default_v1()
                    .condition_for(VictoryPath::Unity)
                    .unwrap()
                    .clone(),
            ],
        };
        set_victory_settings(&mut engine, settings);
        let player = engine.state.player_empire;
        engine
            .state
            .empires
            .get_mut(&player)
            .unwrap()
            .research
            .completed
            .push(TechId(34));
        let ai = engine.state.ai_empires[0];
        let ai_colony = engine
            .state
            .colonies
            .values()
            .find(|colony| colony.owner == ai)
            .map(|colony| colony.id)
            .unwrap();
        engine.state.colonies.get_mut(&ai_colony).unwrap().owner = player;
        let _ = engine.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            engine.state.victory_status.winning_path,
            Some(VictoryPath::Dominion)
        );
    }
}
