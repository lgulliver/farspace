//! Deterministic AI opponent — rule-based decision engine
//!
//! The AI empire follows a fixed priority list each turn:
//! 1. Select cheapest unresearched tech (if none active)
//! 2. Queue builds for each owned colony with an empty queue
//!    (FabricationYard first, then Colony Ship, then Scout)
//! 3. Dispatch the first idle scout to the nearest unexplored star
//! 4. Colonize with any idle colonizer at an AI-explored star

use crate::events::Event;
use crate::state::{
    all_techs, BuildItem, BuildingType, Colony, ColonyId, EmpireId, FleetId, FleetKind, GameState,
    ScoutMission, StarId, TechId,
};

/// Turns a scout takes to reach an unexplored system (mirrors engine constant).
const SCOUT_TRAVEL_TURNS: u32 = 3;

/// Run one AI decision pass for the given empire.
///
/// Mutates `state` and returns events for each action taken.
/// All decisions are deterministic: given the same `state` input, the same
/// actions will be produced.
pub fn run_ai_turn(state: &mut GameState, ai_empire_id: EmpireId) -> Vec<Event> {
    let mut events = Vec::new();

    ai_select_research(state, ai_empire_id, &mut events);
    ai_queue_builds(state, ai_empire_id, &mut events);
    ai_dispatch_scouts(state, ai_empire_id, &mut events);
    ai_colonize(state, ai_empire_id, &mut events);

    events
}

// ---------------------------------------------------------------------------
// Research
// ---------------------------------------------------------------------------

fn ai_select_research(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    let tech_id = match pick_research(state, empire_id) {
        Some(t) => t,
        None => return,
    };

    if let Some(empire) = state.empires.get_mut(&empire_id) {
        // Only reset progress when switching away from a different active tech
        if let Some(active) = empire.research.current_tech {
            if active != tech_id {
                empire.research.progress = 0;
            }
        }
        empire.research.current_tech = Some(tech_id);
    }

    events.push(Event::AiResearchSelected {
        empire: empire_id,
        tech: tech_id,
    });
}

/// Pick the cheapest unresearched tech.
/// Returns `None` if the empire is already researching something.
fn pick_research(state: &GameState, empire_id: EmpireId) -> Option<TechId> {
    let empire = state.empires.get(&empire_id)?;
    if empire.research.current_tech.is_some() {
        return None;
    }
    let completed = &empire.research.completed;
    let mut candidates: Vec<_> = all_techs()
        .iter()
        .filter(|t| !completed.contains(&t.id))
        .collect();
    // Deterministic sort: cheapest first, tie-break by ascending TechId
    candidates.sort_by(|a, b| a.cost.cmp(&b.cost).then(a.id.cmp(&b.id)));
    candidates.first().map(|t| t.id)
}

// ---------------------------------------------------------------------------
// Build queue
// ---------------------------------------------------------------------------

fn ai_queue_builds(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    // Collect colony IDs first to avoid re-borrowing state inside the loop
    let colony_ids: Vec<ColonyId> = state
        .colonies
        .keys()
        .filter(|&&id| {
            state
                .colonies
                .get(&id)
                .is_some_and(|c| c.owner == empire_id)
        })
        .copied()
        .collect();

    for colony_id in colony_ids {
        if let Some(item) = pick_build_item(state, empire_id, colony_id) {
            if let Some(colony) = state.colonies.get_mut(&colony_id) {
                colony.build_queue.push(item);
            }
            events.push(Event::AiBuildQueued {
                empire: empire_id,
                colony: colony_id,
                item,
            });
        }
    }
}

/// Pick what to build at a colony with an empty queue.
///
/// Priority:
/// 1. `FabricationYard` if not yet built at this colony
/// 2. Colony Ship if the empire has no colonizer fleet
/// 3. Scout (to continue exploration)
fn pick_build_item(
    state: &GameState,
    empire_id: EmpireId,
    colony_id: ColonyId,
) -> Option<BuildItem> {
    let colony = state.colonies.get(&colony_id)?;
    if colony.owner != empire_id {
        return None;
    }
    if !colony.build_queue.is_empty() {
        return None;
    }

    // Priority 1: FabricationYard
    if !colony.buildings.contains(&BuildingType::FabricationYard) {
        return Some(BuildItem::Structure(BuildingType::FabricationYard));
    }

    // Priority 2: Colony Ship if no colonizer exists
    let has_colonizer = state
        .fleets
        .values()
        .any(|f| f.owner == empire_id && f.kind == FleetKind::Colonizer);
    if !has_colonizer {
        return Some(BuildItem::Colony);
    }

    // Priority 3: Scout for continued exploration
    Some(BuildItem::Scout)
}

// ---------------------------------------------------------------------------
// Scout dispatch
// ---------------------------------------------------------------------------

fn ai_dispatch_scouts(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    if let Some((fleet_id, destination)) = pick_scout_target(state, empire_id) {
        state.scout_missions.insert(
            fleet_id,
            ScoutMission {
                fleet: fleet_id,
                destination,
                turns_remaining: SCOUT_TRAVEL_TURNS,
            },
        );
        events.push(Event::AiScoutDispatched {
            empire: empire_id,
            fleet: fleet_id,
            destination,
        });
    }
}

/// Find the nearest unexplored star for an idle AI scout fleet.
///
/// Returns `(fleet_id, destination)` or `None` if no valid target exists.
fn pick_scout_target(state: &GameState, empire_id: EmpireId) -> Option<(FleetId, StarId)> {
    // Find first idle scout fleet owned by this empire (deterministic: BTreeMap key order)
    let fleet_id = state.fleets.keys().copied().find(|&fid| {
        let f = &state.fleets[&fid];
        f.owner == empire_id
            && f.kind == FleetKind::Scout
            && !state.scout_missions.contains_key(&fid)
            && !state.fleet_missions.contains_key(&fid)
    })?;

    let fleet_loc = state.fleets.get(&fleet_id)?.location;
    let fleet_star = state.stars.get(&fleet_loc)?;

    // Stars already targeted by any AI scout
    let already_targeted: std::collections::BTreeSet<StarId> = state
        .scout_missions
        .values()
        .filter(|m| {
            state
                .fleets
                .get(&m.fleet)
                .is_some_and(|f| f.owner == empire_id)
        })
        .map(|m| m.destination)
        .collect();

    let mut candidates: Vec<(i64, StarId)> = state
        .stars
        .keys()
        .filter(|&sid| !state.ai_explored_stars.contains(sid) && !already_targeted.contains(sid))
        .filter_map(|&sid| {
            let s = state.stars.get(&sid)?;
            let dx = (s.x - fleet_star.x) as i64;
            let dy = (s.y - fleet_star.y) as i64;
            Some((dx * dx + dy * dy, sid))
        })
        .collect();

    // Nearest first; tie-break by ascending StarId for full determinism
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    candidates.first().map(|&(_, sid)| (fleet_id, sid))
}

// ---------------------------------------------------------------------------
// Colonization
// ---------------------------------------------------------------------------

fn ai_colonize(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    if let Some((fleet_id, star_id, planet_index)) = pick_colonize_target(state, empire_id) {
        let colony_id = state.next_colony_id();

        let new_colony = Colony {
            id: colony_id,
            star: star_id,
            planet_index,
            owner: empire_id,
            population: 1,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
        };
        state.colonies.insert(colony_id, new_colony);

        // Update planet reference
        if let Some(star) = state.stars.get_mut(&star_id) {
            if let Some(planet) = star.planets.get_mut(planet_index) {
                planet.colony = Some(colony_id);
            }
        }

        // Consume the colonizer fleet
        state.fleets.remove(&fleet_id);
        state.scout_missions.remove(&fleet_id);
        state.fleet_missions.remove(&fleet_id);

        events.push(Event::AiColonized {
            empire: empire_id,
            star: star_id,
            planet_index,
            colony: colony_id,
        });
    }
}

/// Find an idle colonizer fleet at an AI-explored star with a habitable free planet.
fn pick_colonize_target(
    state: &GameState,
    empire_id: EmpireId,
) -> Option<(FleetId, StarId, usize)> {
    // Deterministic iteration order via BTreeMap keys
    let fleet_id = state.fleets.keys().copied().find(|&fid| {
        let f = &state.fleets[&fid];
        f.owner == empire_id
            && f.kind == FleetKind::Colonizer
            && !state.scout_missions.contains_key(&fid)
            && !state.fleet_missions.contains_key(&fid)
    })?;

    let fleet_loc = state.fleets.get(&fleet_id)?.location;

    // Colonizer must be at an AI-explored star
    if !state.ai_explored_stars.contains(&fleet_loc) {
        return None;
    }

    let star = state.stars.get(&fleet_loc)?;
    let planet_index = star
        .planets
        .iter()
        .enumerate()
        .find(|(_, p)| p.habitable && p.colony.is_none())
        .map(|(i, _)| i)?;

    Some((fleet_id, fleet_loc, planet_index))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::state::{BuildingType, EmpireId, FleetKind, TechId};

    /// Helper: get the AI empire ID from an engine, panicking if absent.
    fn ai_id(engine: &Engine) -> EmpireId {
        engine.state.ai_empire.expect("Engine must have AI empire")
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn same_seed_produces_same_ai_decisions() {
        let mut engine_a = Engine::new(42);
        let mut engine_b = Engine::new(42);

        // End turn once so the AI runs
        engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
        engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Both states must be identical
        assert_eq!(engine_a.state, engine_b.state);
    }

    #[test]
    fn ai_turn_is_deterministic_across_multiple_turns() {
        let mut engine_a = Engine::new(999);
        let mut engine_b = Engine::new(999);

        for _ in 0..5 {
            engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
            engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);
        }

        assert_eq!(engine_a.state, engine_b.state);
    }

    // -----------------------------------------------------------------------
    // Research
    // -----------------------------------------------------------------------

    #[test]
    fn ai_selects_valid_research() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Initially no research selected
        assert!(engine.state.empires[&ai].research.current_tech.is_none());

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // AI should have selected a tech
        let selected = engine.state.empires[&ai].research.current_tech;
        assert!(
            selected.is_some(),
            "AI must select a research tech after first turn"
        );

        // The selected tech must be in all_techs()
        let tech_id = selected.unwrap();
        assert!(
            all_techs().iter().any(|t| t.id == tech_id),
            "AI must select a valid tech"
        );

        // Event must be emitted
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AiResearchSelected { empire, tech } if *empire == ai && *tech == tech_id)),
            "AiResearchSelected event must be emitted"
        );
    }

    #[test]
    fn ai_selects_cheapest_unresearched_tech() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // The cheapest tech (cost=50, TechId(1) "Void Propulsion") should be chosen
        let cheapest = all_techs()
            .iter()
            .min_by_key(|t| (t.cost, t.id.0))
            .unwrap()
            .id;

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let selected = engine.state.empires[&ai].research.current_tech.unwrap();
        assert_eq!(
            selected, cheapest,
            "AI must pick cheapest unresearched tech"
        );
    }

    #[test]
    fn ai_does_not_select_completed_tech() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Pre-complete all techs except TechId(5) (cost 120) and TechId(6) (cost 90)
        {
            let empire = engine.state.empires.get_mut(&ai).unwrap();
            for t in all_techs() {
                if t.id != TechId(5) && t.id != TechId(6) {
                    empire.research.completed.push(t.id);
                }
            }
        }

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let selected = engine.state.empires[&ai].research.current_tech.unwrap();
        // Should pick TechId(6) (cost 90) over TechId(5) (cost 120)
        assert_eq!(
            selected,
            TechId(6),
            "AI must pick next cheapest uncompleted tech"
        );
    }

    #[test]
    fn ai_does_not_reselect_when_already_researching() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Force a specific tech
        engine
            .state
            .empires
            .get_mut(&ai)
            .unwrap()
            .research
            .current_tech = Some(TechId(4));

        let events_before = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // No AiResearchSelected should be emitted
        assert!(
            !events_before
                .iter()
                .any(|e| matches!(e, Event::AiResearchSelected { empire, .. } if *empire == ai)),
            "AI must not change research when already active"
        );
        assert_eq!(
            engine.state.empires[&ai].research.current_tech,
            Some(TechId(4))
        );
    }

    // -----------------------------------------------------------------------
    // Build queue
    // -----------------------------------------------------------------------

    #[test]
    fn ai_queues_fabrication_yard_first() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .expect("AI must have a colony");

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // AI colony should have FabricationYard queued
        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            colony
                .build_queue
                .contains(&BuildItem::Structure(BuildingType::FabricationYard)),
            "AI must queue FabricationYard first"
        );

        // AiBuildQueued event emitted
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::AiBuildQueued { empire, colony, item }
                if *empire == ai && *colony == ai_colony_id
                    && *item == BuildItem::Structure(BuildingType::FabricationYard)
            )),
            "AiBuildQueued event must be emitted for FabricationYard"
        );
    }

    #[test]
    fn ai_does_not_queue_when_queue_non_empty() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Pre-fill the AI colony queue
        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .build_queue
            .push(BuildItem::Scout);

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Should not emit an extra AiBuildQueued for this colony
        let build_events: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(e, Event::AiBuildQueued { empire, colony, .. }
                    if *empire == ai && *colony == ai_colony_id)
            })
            .collect();
        assert!(
            build_events.is_empty(),
            "AI must not queue when colony already has a queue item"
        );
    }

    #[test]
    fn ai_queues_colony_ship_if_no_colonizer() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Pre-build a FabricationYard so AI skips that priority
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::FabricationYard);

        // Ensure no colonizer exists
        assert!(!engine
            .state
            .fleets
            .values()
            .any(|f| f.owner == ai && f.kind == FleetKind::Colonizer));

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            colony.build_queue.contains(&BuildItem::Colony),
            "AI must queue Colony Ship when no colonizer exists"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::AiBuildQueued { empire, item, .. }
            if *empire == ai && *item == BuildItem::Colony
        )));
    }

    #[test]
    fn ai_queues_scout_when_has_colonizer() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Pre-build FabricationYard
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::FabricationYard);

        // Add a fake colonizer fleet for the AI
        let fake_colonizer_id = crate::state::FleetId(99);
        engine.state.fleets.insert(
            fake_colonizer_id,
            crate::state::Fleet {
                id: fake_colonizer_id,
                owner: ai,
                location: engine.state.empires[&ai].home_star,
                ships: 1,
                kind: FleetKind::Colonizer,
            },
        );

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            colony.build_queue.contains(&BuildItem::Scout),
            "AI must queue Scout when FabricationYard built and colonizer exists"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::AiBuildQueued { empire, item, .. }
            if *empire == ai && *item == BuildItem::Scout
        )));
    }

    // -----------------------------------------------------------------------
    // Scout dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn ai_scout_dispatch_is_deterministic() {
        // Run two engines with the same seed; scout destinations must match
        let mut engine_a = Engine::new(7);
        let mut engine_b = Engine::new(7);

        engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
        engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);

        assert_eq!(engine_a.state.scout_missions, engine_b.state.scout_missions);
    }

    #[test]
    fn ai_dispatches_scout_to_unexplored_star() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // AI scout missions must target unexplored stars
        for mission in engine.state.scout_missions.values() {
            let fleet = engine.state.fleets.get(&mission.fleet);
            if let Some(f) = fleet {
                if f.owner == ai {
                    assert!(
                        !engine
                            .state
                            .ai_explored_stars
                            .contains(&mission.destination),
                        "AI must not scout an already-explored star"
                    );
                }
            }
        }

        // AiScoutDispatched event must be emitted
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AiScoutDispatched { empire, .. } if *empire == ai)),
            "AiScoutDispatched event must be emitted"
        );
    }

    // -----------------------------------------------------------------------
    // Colonization
    // -----------------------------------------------------------------------

    #[test]
    fn ai_does_not_colonize_without_colonizer() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // No colonizer for AI initially
        assert!(!engine
            .state
            .fleets
            .values()
            .any(|f| f.owner == ai && f.kind == FleetKind::Colonizer));

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_before, colonies_after,
            "AI must not colonize without a colonizer"
        );
    }

    #[test]
    fn ai_does_not_colonize_unexplored_planet() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Place a colonizer at an unexplored star
        let unexplored = engine
            .state
            .stars
            .keys()
            .find(|&&id| !engine.state.ai_explored_stars.contains(&id))
            .copied()
            .expect("Must have unexplored stars");

        let colonizer_id = crate::state::FleetId(50);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: unexplored,
                ships: 1,
                kind: FleetKind::Colonizer,
            },
        );

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_before, colonies_after,
            "AI must not colonize an unexplored star"
        );
    }

    #[test]
    fn ai_colonizes_valid_explored_planet() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Find an explored AI star that is not the home star and has a free habitable planet
        let ai_home = engine.state.empires[&ai].home_star;
        let target =
            engine.state.ai_explored_stars.iter().copied().find(|&sid| {
                sid != ai_home
                    && engine.state.stars.get(&sid).is_some_and(|s| {
                        s.planets.iter().any(|p| p.habitable && p.colony.is_none())
                    })
            });

        let target = match target {
            Some(t) => t,
            None => return, // No valid target — test is not applicable for this seed
        };

        // Place a colonizer there
        let colonizer_id = crate::state::FleetId(50);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
            },
        );

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_after,
            colonies_before + 1,
            "AI must colonize the valid planet"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AiColonized { empire, star, .. } if *empire == ai && *star == target)),
            "AiColonized event must be emitted"
        );

        // Colonizer must be consumed
        assert!(
            !engine.state.fleets.contains_key(&colonizer_id),
            "Colonizer must be consumed after colonization"
        );
    }

    #[test]
    fn ai_does_not_colonize_already_colonized_planet() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Use AI home star (already has a colony at planet 0)
        let ai_home = engine.state.empires[&ai].home_star;

        let colonizer_id = crate::state::FleetId(50);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: ai_home,
                ships: 1,
                kind: FleetKind::Colonizer,
            },
        );

        // Check whether all planets at AI home are colonized
        let all_colonized = engine
            .state
            .stars
            .get(&ai_home)
            .is_none_or(|s| s.planets.iter().all(|p| !p.habitable || p.colony.is_some()));

        if !all_colonized {
            // AI home has a free planet — skip this test (it would successfully colonize)
            return;
        }

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_before, colonies_after,
            "AI must not colonize when all planets are occupied"
        );
    }

    // -----------------------------------------------------------------------
    // Player / AI isolation
    // -----------------------------------------------------------------------

    #[test]
    fn player_and_ai_state_are_isolated() {
        let mut engine = Engine::new(42);

        let player = engine.state.player_empire;
        let ai = ai_id(&engine);

        // Verify that player and AI have separate empires with different home stars
        let player_home = engine.state.empires[&player].home_star;
        let ai_home = engine.state.empires[&ai].home_star;
        assert_ne!(
            player_home, ai_home,
            "Player and AI must have different home stars"
        );

        // Player colonies must not be owned by AI and vice-versa
        for colony in engine.state.colonies.values() {
            assert!(
                colony.owner == player || colony.owner == ai,
                "Colony must be owned by player or AI"
            );
        }

        // Verify explored star sets are separate
        // (no requirement they be disjoint, but they must be independent BTreeSets)
        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Player explored stars must not change when AI scouts
        let player_explored = engine.state.explored_stars.clone();

        // Check that AI scout mission completions go to ai_explored_stars only
        // by running several more turns and verifying independence
        for _ in 0..5 {
            engine.apply_turn(vec![crate::commands::Command::EndTurn]);
        }

        // Player explored set must only grow if player scouts
        // (player hasn't scouted, so it stays the same)
        assert_eq!(
            engine.state.explored_stars, player_explored,
            "Player explored_stars must not be modified by AI actions"
        );
    }

    // -----------------------------------------------------------------------
    // Save / load round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn save_load_preserves_ai_empire_state() {
        let mut engine = Engine::new(42);
        // Run a few turns so AI makes decisions
        for _ in 0..3 {
            engine.apply_turn(vec![crate::commands::Command::EndTurn]);
        }

        let original = engine.state.clone();
        // Use serde_json directly (game_core cannot depend on game_save)
        let saved = serde_json::to_string(&original).expect("serialize must succeed");
        let loaded: GameState = serde_json::from_str(&saved).expect("deserialize must succeed");

        assert_eq!(
            original.ai_empire, loaded.ai_empire,
            "ai_empire must survive round-trip"
        );
        assert_eq!(
            original.ai_explored_stars, loaded.ai_explored_stars,
            "ai_explored_stars must survive round-trip"
        );
        assert_eq!(
            original.empires.len(),
            loaded.empires.len(),
            "All empires (player + AI) must survive round-trip"
        );

        // AI empire research state must survive
        let ai = original.ai_empire.unwrap();
        let orig_ai = original.empires.get(&ai).unwrap();
        let load_ai = loaded.empires.get(&ai).unwrap();
        assert_eq!(
            orig_ai.research.current_tech, load_ai.research.current_tech,
            "AI research current_tech must survive round-trip"
        );
    }

    #[test]
    fn save_load_preserves_ai_colonies() {
        let engine = Engine::new(42);
        let ai = ai_id(&engine);

        let saved = serde_json::to_string(&engine.state).expect("serialize must succeed");
        let loaded: GameState = serde_json::from_str(&saved).expect("deserialize must succeed");

        let original_ai_colonies = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();
        let loaded_ai_colonies = loaded.colonies.values().filter(|c| c.owner == ai).count();

        assert_eq!(
            original_ai_colonies, loaded_ai_colonies,
            "AI colonies must survive save/load round-trip"
        );
    }
}
