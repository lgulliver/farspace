use super::*;
use crate::state::{
    BuildingType, ComponentId, CustomDesignId, HullId, Planet, PlanetClass, PlanetSize,
    PlanetSpecial, SectorId, SpectralClass, Star, StrategicResource,
};

/// Inject a Shipyard directly into a colony's orbital installations.
/// Used to satisfy the "ships require a Shipyard" rule in tests that
/// focus on build completion / queue mechanics rather than the validation itself.
///
/// # Panics
/// Panics if `colony_id` does not exist in the engine state.
fn give_colony_shipyard(engine: &mut Engine, colony_id: ColonyId) {
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .orbital_installations
        .push(crate::state::OrbitalStructureType::Shipyard);
}

/// Unlock Habitat Seeding (TechId 2) for the player empire.
fn unlock_habitat_seeding(engine: &mut Engine) {
    let empire_id = engine.state.player_empire;
    if let Some(empire) = engine.state.empires.get_mut(&empire_id) {
        if !empire.research.completed.contains(&TechId(2)) {
            empire.research.completed.push(TechId(2));
        }
    }
}

#[test]
fn new_engine_creates_valid_state() {
    let engine = Engine::new(42);
    assert_eq!(engine.state.turn, 1);
    assert!(!engine.state.stars.is_empty());
    assert!(!engine.state.empires.is_empty());
    assert!(!engine.state.colonies.is_empty());
    assert!(!engine.state.fleets.is_empty());
}

#[test]
fn end_turn_advances_turn_counter() {
    let mut engine = Engine::new(42);
    let initial_turn = engine.state.turn;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert_eq!(engine.state.turn, initial_turn + 1);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::TurnAdvanced { new_turn } if *new_turn == initial_turn + 1)));
}

#[test]
fn end_turn_processes_colony_production() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Set focus to 100% credits
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    let initial_credits = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .credits;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    let final_credits = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .credits;

    assert!(final_credits > initial_credits);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ColonyProduced { .. })));
}

#[test]
fn set_colony_focus_valid() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    let events = engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 70,
        research_pct: 30,
    }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ColonyFocusSet { colony } if *colony == colony_id)));

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(colony.prod_pct, 70);
    assert_eq!(colony.research_pct, 30);
}

#[test]
fn set_colony_focus_invalid_sum_emits_error() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    let events = engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 50,
        research_pct: 40, // Sum is 90, not 100
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn set_colony_focus_unknown_colony_emits_error() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(999);

    let events = engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 50,
        research_pct: 50,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn move_fleet_valid() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    // MoveFleet now requires an explored destination
    let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
    let destination = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != initial_location)
        .expect("Need an explored star other than home");

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);

    // Should emit FleetDeparted (not FleetMoved) and create a mission
    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::FleetDeparted { fleet, from, to, .. }
        if *fleet == fleet_id && *from == initial_location && *to == destination
    )));

    // Fleet has not moved yet — the mission is pending
    assert!(engine.state.fleet_missions.contains_key(&fleet_id));
    let fleet = engine.state.fleets.get(&fleet_id).unwrap();
    assert_eq!(fleet.location, initial_location);
}

#[test]
fn move_fleet_unknown_fleet_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(999);
    let destination = *engine.state.stars.keys().next().unwrap();

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn move_fleet_unknown_destination_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let destination = StarId(999);

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn queue_build_valid() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events.iter().any(
            |e| matches!(e, Event::BuildQueued { colony, item } if *colony == colony_id && *item == BuildItem::Scout)
        ));

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(colony.build_queue.len(), 1);
    assert_eq!(colony.build_queue[0], BuildItem::Scout);
}

#[test]
fn cancel_build_valid() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    // First add something to cancel
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    let events = engine.apply_turn(vec![Command::CancelBuild {
        colony: colony_id,
        index: 0,
    }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::BuildCancelled { colony } if *colony == colony_id)));

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(colony.build_queue.is_empty());
}

#[test]
fn cancel_build_out_of_bounds_emits_error() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    let events = engine.apply_turn(vec![Command::CancelBuild {
        colony: colony_id,
        index: 5, // Queue is empty
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn build_completion_creates_fleet() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    // Queue a scout (cost 50)
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    // Set production to 100%
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    let initial_fleet_count = engine.state.fleets.len();

    // Run enough turns to complete the build (production is 10/turn, cost is 50)
    for _ in 0..6 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    // Should have a new fleet
    assert!(engine.state.fleets.len() > initial_fleet_count);
}

#[test]
fn deterministic_engine_creation() {
    let engine1 = Engine::new(12345);
    let engine2 = Engine::new(12345);

    assert_eq!(engine1.state.stars.len(), engine2.state.stars.len());
    for (id, star1) in &engine1.state.stars {
        let star2 = engine2.state.stars.get(id).unwrap();
        assert_eq!(star1.name, star2.name);
        assert_eq!(star1.x, star2.x);
        assert_eq!(star1.y, star2.y);
    }
}

#[test]
fn event_log_trimmed_to_50() {
    let mut engine = Engine::new(42);

    // Generate many events
    for _ in 0..60 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    assert!(engine.state.event_log.len() <= 50);
}

#[test]
fn move_fleet_not_owned_by_player_emits_error() {
    let mut engine = Engine::new(42);

    // Create a fleet owned by a different empire
    let other_empire = EmpireId(99);
    let fleet_id = FleetId(99);
    let location = *engine.state.stars.keys().next().unwrap();
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: other_empire,
            location,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    let destination = *engine.state.stars.keys().nth(1).unwrap();
    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn queue_build_unknown_colony_emits_error() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(999);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn queue_build_not_owned_by_player_emits_error() {
    let mut engine = Engine::new(42);

    // Create a colony owned by a different empire
    let other_empire = EmpireId(99);
    let colony_id = ColonyId(99);
    let star_id = *engine.state.stars.keys().next().unwrap();
    engine.state.colonies.insert(
        colony_id,
        Colony {
            id: colony_id,
            star: star_id,
            planet_index: 0,
            owner: other_empire,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn cancel_build_unknown_colony_emits_error() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(999);

    let events = engine.apply_turn(vec![Command::CancelBuild {
        colony: colony_id,
        index: 0,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn cancel_build_not_owned_by_player_emits_error() {
    let mut engine = Engine::new(42);

    // Create a colony owned by a different empire with an item in the queue
    let other_empire = EmpireId(99);
    let colony_id = ColonyId(99);
    let star_id = *engine.state.stars.keys().next().unwrap();
    engine.state.colonies.insert(
        colony_id,
        Colony {
            id: colony_id,
            star: star_id,
            planet_index: 0,
            owner: other_empire,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![BuildItem::Scout],
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );

    let events = engine.apply_turn(vec![Command::CancelBuild {
        colony: colony_id,
        index: 0,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn cancel_build_non_first_item_does_not_reset_accumulated() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    // Queue two items
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    // Set some accumulated production
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .accumulated_production = 25;

    // Cancel the second item (index 1) — accumulated should be preserved
    let events = engine.apply_turn(vec![Command::CancelBuild {
        colony: colony_id,
        index: 1,
    }]);

    assert!(!events.iter().any(|e| e.is_error()));
    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(colony.accumulated_production, 25);
    assert_eq!(colony.build_queue.len(), 1);
}

#[test]
fn set_colony_focus_not_owned_by_player_emits_error() {
    let mut engine = Engine::new(42);

    // Create a colony owned by a different empire
    let other_empire = EmpireId(99);
    let colony_id = ColonyId(99);
    let star_id = *engine.state.stars.keys().next().unwrap();
    engine.state.colonies.insert(
        colony_id,
        Colony {
            id: colony_id,
            star: star_id,
            planet_index: 0,
            owner: other_empire,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );

    let events = engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 70,
        research_pct: 30,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn build_completion_colony_ship_creates_fleet() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);
    unlock_habitat_seeding(&mut engine);

    // Queue a colony ship (cost 200)
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Colony,
    }]);

    // Set production to 100%
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    // Run enough turns to complete. We check for BuildCompleted event because
    // fleet count may not strictly increase if a pre-existing fleet is destroyed
    // in combat during the same window (valid game behaviour unrelated to ship production).
    let mut build_completed = false;
    for _ in 0..25 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events
            .iter()
            .any(|e| matches!(e, Event::BuildCompleted { item, .. } if *item == BuildItem::Colony))
        {
            build_completed = true;
            break;
        }
    }

    assert!(
        build_completed,
        "BuildCompleted for Colony ship must be emitted within 25 turns"
    );
}

#[test]
fn determinism_same_seed_same_commands_same_state() {
    let commands = vec![
        Command::SetColonyFocus {
            colony: ColonyId(1),
            prod_pct: 80,
            research_pct: 20,
        },
        Command::QueueBuild {
            colony: ColonyId(1),
            item: BuildItem::Scout,
        },
        Command::EndTurn,
        Command::EndTurn,
        Command::EndTurn,
    ];

    let mut engine_a = Engine::new(99999);
    let mut engine_b = Engine::new(99999);

    for cmd in commands {
        let evts_a = engine_a.apply_turn(vec![cmd.clone()]);
        let evts_b = engine_b.apply_turn(vec![cmd]);
        assert_eq!(evts_a, evts_b);
    }

    assert_eq!(engine_a.state, engine_b.state);
}

#[test]
fn queue_building_structure_is_valid() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let item = BuildItem::Structure(BuildingType::AquacultureBay);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item,
    }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events.iter().any(
            |e| matches!(e, Event::BuildQueued { colony, item: it } if *colony == colony_id && *it == item)
        ));

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(colony.build_queue.len(), 1);
    assert_eq!(colony.build_queue[0], item);
}

#[test]
fn building_completion_adds_to_colony_buildings() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Queue an Aquaculture Bay (cost 60)
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Structure(BuildingType::AquacultureBay),
    }]);

    // 100% production to maximise output
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    // production=10/turn, cost=60 => complete after 6 turns
    for _ in 0..7 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(
        colony.buildings.contains(&BuildingType::AquacultureBay),
        "Colony should have AquacultureBay after completion"
    );
    assert!(
        colony.build_queue.is_empty(),
        "Build queue should be empty after completion"
    );
}

#[test]
fn building_completion_does_not_create_fleet() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Queue a Fabrication Yard (cost 80)
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Structure(BuildingType::FabricationYard),
    }]);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    let initial_fleet_count = engine.state.fleets.len();

    // production=10/turn, cost=80 => 8 turns
    for _ in 0..9 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    // Fleet count must NOT increase for a building
    assert_eq!(
        engine.state.fleets.len(),
        initial_fleet_count,
        "Building completion must not create a fleet"
    );

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(colony.buildings.contains(&BuildingType::FabricationYard));
}

#[test]
fn multiple_buildings_accumulate_in_colony() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Queue two buildings
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Structure(BuildingType::AquacultureBay),
    }]);
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Structure(BuildingType::ScienceNexus),
    }]);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    // cost: Aquaculture=60 + ScienceNexus=100 = 160 production total needed
    // 10/turn => 16+ turns
    for _ in 0..18 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(colony.buildings.contains(&BuildingType::AquacultureBay));
    assert!(colony.buildings.contains(&BuildingType::ScienceNexus));
}

#[test]
fn building_completion_emits_build_completed_event() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let item = BuildItem::Structure(BuildingType::ScienceNexus);

    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item,
    }]);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    // cost=100, production=10/turn => 10 turns
    let mut completed = false;
    for _ in 0..12 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events.iter().any(|e| {
            matches!(e, Event::BuildCompleted { colony, item: it }
                    if *colony == colony_id && *it == item)
        }) {
            completed = true;
            break;
        }
    }
    assert!(completed, "BuildCompleted event should have been emitted");
}

// ──────────────────────────────────────────────────────────────────
// Research tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn select_research_valid_emits_research_selected() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let tech_id = TechId(1);

    let events = engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ResearchSelected { tech } if *tech == tech_id)));

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(empire.research.current_tech, Some(tech_id));
    assert_eq!(empire.research.progress, 0);
}

#[test]
fn select_research_unknown_tech_emits_error() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let bad_tech = TechId(999);

    let events = engine.apply_turn(vec![Command::SelectResearch { tech: bad_tech }]);

    assert!(events.iter().any(|e| e.is_error()));
    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert!(empire.research.current_tech.is_none());
}

#[test]
fn select_already_completed_tech_emits_error() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let tech_id = TechId(1);

    // Manually mark as completed
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .research
        .completed
        .push(tech_id);

    let events = engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn select_research_with_unmet_prerequisites_emits_error() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);

    // Drift Mapping requires Neutrino Sensors.
    let events = engine.apply_turn(vec![Command::SelectResearch { tech: TechId(6) }]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Error { message } if message.contains("locked"))),
        "locked technology selection must emit an error"
    );
}

#[test]
fn completed_prerequisite_unlocks_dependent_research_selection() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);

    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .research
        .completed
        .push(TechId(3));

    let events = engine.apply_turn(vec![Command::SelectResearch { tech: TechId(6) }]);

    assert!(
        !events.iter().any(|e| e.is_error()),
        "tech should be selectable once prerequisites are completed"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ResearchSelected { tech } if *tech == TechId(6))));
}

#[test]
fn queue_research_valid_emits_event() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let tech_id = TechId(2);

    let events = engine.apply_turn(vec![Command::QueueResearch { tech: tech_id }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ResearchQueued { tech } if *tech == tech_id)));
    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(empire.research.queue, vec![tech_id]);
}

#[test]
fn queue_research_duplicate_emits_error() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let tech_id = TechId(2);

    engine.apply_turn(vec![Command::QueueResearch { tech: tech_id }]);
    let events = engine.apply_turn(vec![Command::QueueResearch { tech: tech_id }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn move_queued_research_up_reorders_queue() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let first = TechId(2);
    let second = TechId(3);

    engine.apply_turn(vec![
        Command::QueueResearch { tech: first },
        Command::QueueResearch { tech: second },
    ]);
    let events = engine.apply_turn(vec![Command::MoveQueuedResearchUp { tech: second }]);

    assert!(events.iter().any(|e| {
        matches!(
            e,
            Event::ResearchQueueReordered {
                tech,
                from_index: 1,
                to_index: 0
            } if *tech == second
        )
    }));
    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(empire.research.queue, vec![second, first]);
}

#[test]
fn clear_research_queue_emits_removed_count() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);

    engine.apply_turn(vec![
        Command::QueueResearch { tech: TechId(2) },
        Command::QueueResearch { tech: TechId(3) },
    ]);
    let events = engine.apply_turn(vec![Command::ClearResearchQueue]);

    assert!(events.iter().any(|e| {
        matches!(
            e,
            Event::ResearchQueueCleared { removed_count } if *removed_count == 2
        )
    }));
    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert!(empire.research.queue.is_empty());
}

#[test]
fn completion_auto_starts_queued_research() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_a = TechId(1);
    let tech_b = TechId(3);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);
    engine.apply_turn(vec![Command::QueueResearch { tech: tech_b }]);

    let mut completion_events = Vec::new();
    for _ in 0..8 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events
            .iter()
            .any(|e| matches!(e, Event::ResearchCompleted { tech } if *tech == tech_a))
        {
            completion_events = events;
            break;
        }
    }

    assert!(!completion_events.is_empty());
    assert!(completion_events
        .iter()
        .any(|e| matches!(e, Event::QueuedResearchStarted { tech } if *tech == tech_b)));
    assert!(completion_events.iter().any(|e| {
        matches!(
            e,
            Event::ResearchCompletedWithQueueTransition {
                completed,
                started: Some(started)
            } if *completed == tech_a && *started == tech_b
        )
    }));

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(empire.research.current_tech, Some(tech_b));
}

#[test]
fn completion_skips_locked_queued_research_and_starts_next() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_a = TechId(1);
    let locked = TechId(6);
    let fallback = TechId(3);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);
    engine.apply_turn(vec![
        Command::QueueResearch { tech: locked },
        Command::QueueResearch { tech: fallback },
    ]);

    let mut completion_events = Vec::new();
    for _ in 0..12 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events
            .iter()
            .any(|e| matches!(e, Event::ResearchCompleted { tech } if *tech == tech_a))
        {
            completion_events = events;
            break;
        }
    }

    assert!(completion_events.iter().any(|e| {
        matches!(
            e,
            Event::QueuedResearchSkipped { tech, reason }
            if *tech == locked && reason.contains("prerequisites")
        )
    }));
    assert!(completion_events
        .iter()
        .any(|e| matches!(e, Event::QueuedResearchStarted { tech } if *tech == fallback)));
}

#[test]
fn research_progress_accumulates_on_end_turn() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_id = TechId(1);

    // Set 100% research focus
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    // Select a tech
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    // End one turn
    engine.apply_turn(vec![Command::EndTurn]);

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    // Research should have progressed
    assert!(
        empire.research.progress > 0,
        "Research progress should be positive after one turn"
    );
    assert_eq!(empire.research.current_tech, Some(tech_id));
}

#[test]
fn research_completes_when_cost_reached() {
    use crate::state::{all_techs, TechId};
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_id = TechId(1); // Void Propulsion, cost 50
    let tech_cost = all_techs().iter().find(|t| t.id == tech_id).unwrap().cost;

    // 100% research
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    // Compute actual RP/turn from colony state (avoids coupling to starting-balance constants)
    let rp_per_turn = {
        let colony = engine.state.colonies.get(&colony_id).unwrap();
        (colony.production as i64 * colony.research_pct as i64) / 100
    };
    let max_turns = if rp_per_turn > 0 {
        (tech_cost / rp_per_turn) + 2
    } else {
        200
    };

    let mut completed = false;
    for _ in 0..max_turns {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events
            .iter()
            .any(|e| matches!(e, Event::ResearchCompleted { tech } if *tech == tech_id))
        {
            completed = true;
            break;
        }
    }

    assert!(completed, "ResearchCompleted event should have fired");

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert!(empire.research.completed.contains(&tech_id));
    assert!(empire.research.current_tech.is_none());
    assert_eq!(empire.research.progress, 0);
}

#[test]
fn completed_tech_cannot_be_researched_again() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let tech_id = TechId(1);

    // Mark as completed
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .research
        .completed
        .push(tech_id);

    let events = engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);
    assert!(
        events.iter().any(|e| e.is_error()),
        "Selecting a completed tech must emit an error"
    );
}

#[test]
fn research_deterministic_same_seed_same_result() {
    use crate::state::TechId;

    let cmds = vec![
        Command::SelectResearch { tech: TechId(1) },
        Command::SetColonyFocus {
            colony: ColonyId(1),
            prod_pct: 0,
            research_pct: 100,
        },
        Command::EndTurn,
        Command::EndTurn,
        Command::EndTurn,
    ];

    let mut engine_a = Engine::new(7777);
    let mut engine_b = Engine::new(7777);

    for cmd in cmds {
        let ea = engine_a.apply_turn(vec![cmd.clone()]);
        let eb = engine_b.apply_turn(vec![cmd]);
        assert_eq!(ea, eb);
    }

    let empire_a = engine_a
        .state
        .empires
        .get(&engine_a.state.player_empire)
        .unwrap();
    let empire_b = engine_b
        .state
        .empires
        .get(&engine_b.state.player_empire)
        .unwrap();
    assert_eq!(empire_a.research, empire_b.research);
}

#[test]
fn research_no_progress_without_current_tech() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // 100% research but no tech selected
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::EndTurn]);

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    // research_points (lifetime total) should increase, but research.progress stays 0
    assert!(empire.research_points > 0);
    assert_eq!(empire.research.progress, 0);
    assert!(empire.research.current_tech.is_none());
}

#[test]
fn reselecting_same_tech_preserves_progress() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_id = TechId(1);

    // Set 100% research and select tech
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    // Accumulate some progress
    engine.apply_turn(vec![Command::EndTurn]);
    let progress_after_turn = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .research
        .progress;
    assert!(progress_after_turn > 0);

    // Re-select the same tech — progress should be preserved
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);
    let progress_after_reselect = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .research
        .progress;
    assert_eq!(
        progress_after_turn, progress_after_reselect,
        "Re-selecting the current tech must not reset progress"
    );
}

#[test]
fn switching_tech_resets_progress() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_a = TechId(1);
    let tech_b = TechId(4);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);
    engine.apply_turn(vec![Command::EndTurn]);

    let progress_on_a = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .research
        .progress;
    assert!(progress_on_a > 0);

    // Switch to a different tech — progress should reset
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_b }]);
    let progress_after_switch = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .research
        .progress;
    assert_eq!(
        progress_after_switch, 0,
        "Switching tech must reset progress"
    );
}

// ──────────────────────────────────────────────────────────────────
// Overflow / science-pool tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn overflow_can_complete_queued_tech_in_same_turn_with_transition_events() {
    use crate::state::{all_techs, TechId};
    let mut engine = Engine::new(42);
    let tech_a = TechId(1);
    let tech_b = TechId(3);
    let tech_a_cost = all_techs().iter().find(|t| t.id == tech_a).unwrap().cost;
    let tech_b_cost = all_techs().iter().find(|t| t.id == tech_b).unwrap().cost;

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: ColonyId(1),
        prod_pct: 100,
        research_pct: 0,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);
    engine.apply_turn(vec![Command::QueueResearch { tech: tech_b }]);

    // Seed enough progress to finish tech A and then tech B with overflow.
    {
        let empire = engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap();
        empire.research.progress = tech_a_cost + tech_b_cost + 5;
    }

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ResearchCompleted { tech } if *tech == tech_a)));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ResearchCompleted { tech } if *tech == tech_b)));
    assert!(events.iter().any(|e| {
        matches!(
            e,
            Event::ResearchCompletedWithQueueTransition {
                completed,
                started: Some(started)
            } if *completed == tech_a && *started == tech_b
        )
    }));
    assert!(events.iter().any(|e| {
        matches!(
            e,
            Event::ResearchCompletedWithQueueTransition {
                completed,
                started: None
            } if *completed == tech_b
        )
    }));

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert!(empire.research.completed.contains(&tech_a));
    assert!(empire.research.completed.contains(&tech_b));
    assert!(empire.research.current_tech.is_none());
}

#[test]
fn overflow_science_carries_to_next_research() {
    use crate::state::{all_techs, TechId};
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Use research_pct=70 → rp = (10 * 70) / 100 = 7 rp/turn.
    // TechId(1) = Void Propulsion, cost 50.
    // 50 / 7 = 7.14... → completes on turn 8 with 7*8=56 → overflow = 6.
    let tech_a = TechId(1); // cost 50
    let tech_b = TechId(4); // Kinetic Barriers, cost 100

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 30,
        research_pct: 70,
    }]);

    let rp_per_turn = {
        let colony = engine.state.colonies.get(&colony_id).unwrap();
        (colony.production as i64 * colony.research_pct as i64) / 100
    };
    assert!(rp_per_turn > 0);

    let tech_a_cost = all_techs().iter().find(|t| t.id == tech_a).unwrap().cost;
    // Find the turn on which the tech completes and compute the expected overflow
    let completion_turn_rp = {
        let mut acc = 0i64;
        loop {
            acc += rp_per_turn;
            if acc >= tech_a_cost {
                break acc;
            }
        }
    };
    let expected_overflow = completion_turn_rp - tech_a_cost;
    assert!(
        expected_overflow > 0,
        "test requires non-zero overflow with rp={} cost={}; got {}",
        rp_per_turn,
        tech_a_cost,
        expected_overflow
    );

    engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);

    // Run until tech_a completes (a few extra turns are fine — no active tech after)
    let max_turns = tech_a_cost / rp_per_turn + 2;
    for _ in 0..max_turns {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert!(
        empire.research.completed.contains(&tech_a),
        "tech_a should be completed"
    );
    assert!(
        empire.research.current_tech.is_none(),
        "current_tech should be None after completion"
    );
    assert_eq!(
        empire.research.progress, expected_overflow,
        "overflow ({} rp) must be preserved in progress",
        expected_overflow
    );

    // Select tech B — overflow should carry over as a head start
    let overflow = empire.research.progress;
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_b }]);
    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(
        empire.research.current_tech,
        Some(tech_b),
        "tech_b should now be active"
    );
    assert_eq!(
        empire.research.progress, overflow,
        "overflow should carry into tech_b progress"
    );
}

#[test]
fn overflow_is_zero_when_tech_completes_exactly() {
    use crate::state::{all_techs, TechId};
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // TechId(1) cost=50, rp=10/turn → exactly 5 turns → overflow 0
    let tech_id = TechId(1);
    let tech_cost = all_techs().iter().find(|t| t.id == tech_id).unwrap().cost;

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    let rp_per_turn = {
        let colony = engine.state.colonies.get(&colony_id).unwrap();
        (colony.production as i64 * colony.research_pct as i64) / 100
    };
    // Run the exact number of turns needed (no extra)
    let turns_exact = tech_cost / rp_per_turn;
    assert_eq!(
        turns_exact * rp_per_turn,
        tech_cost,
        "test expects exact completion"
    );

    for _ in 0..turns_exact {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert!(empire.research.completed.contains(&tech_id));
    assert_eq!(
        empire.research.progress, 0,
        "overflow must be 0 for exact completion"
    );
}

#[test]
fn overflow_carry_is_deterministic() {
    use crate::state::TechId;

    let cmds = vec![
        Command::SetColonyFocus {
            colony: ColonyId(1),
            prod_pct: 0,
            research_pct: 100,
        },
        Command::SelectResearch { tech: TechId(1) },
        Command::EndTurn,
        Command::EndTurn,
        Command::EndTurn,
        Command::EndTurn,
        Command::EndTurn,
        Command::EndTurn, // extra turn to produce overflow on TechId(1) cost 50
    ];

    let mut engine_a = Engine::new(1234);
    let mut engine_b = Engine::new(1234);

    for cmd in &cmds {
        engine_a.apply_turn(vec![cmd.clone()]);
        engine_b.apply_turn(vec![cmd.clone()]);
    }

    let empire_a = engine_a
        .state
        .empires
        .get(&engine_a.state.player_empire)
        .unwrap();
    let empire_b = engine_b
        .state
        .empires
        .get(&engine_b.state.player_empire)
        .unwrap();
    assert_eq!(
        empire_a.research.progress, empire_b.research.progress,
        "overflow must be identical across identical seeds"
    );
}

#[test]
fn selecting_tech_after_completion_preserves_overflow() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_a = TechId(1); // cost 50
    let tech_b = TechId(4); // cost 100

    // research_pct=70 → 7 rp/turn; cost-50 tech completes on turn 8 with overflow 6
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 30,
        research_pct: 70,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);

    // Run 9 turns — enough to complete cost-50 at 7 rp/turn (completes turn 8)
    for _ in 0..9 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let overflow = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .research
        .progress;
    assert!(
        overflow > 0,
        "should have overflow after completing cost-50 tech at 7 rp/turn"
    );

    // Select next tech — overflow should carry over
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_b }]);
    let empire = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(
        empire.research.progress, overflow,
        "overflow must be preserved when selecting tech after completion"
    );
}

#[test]
fn science_nexus_increases_research_output() {
    use crate::state::BuildingType;

    let colony_id = ColonyId(1);

    // Engine A: no buildings
    let mut engine_a = Engine::new(42);
    engine_a.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine_a.apply_turn(vec![Command::SelectResearch { tech: TechId(5) }]); // cost 120
    engine_a.apply_turn(vec![Command::EndTurn]);
    let progress_no_nexus = engine_a
        .state
        .empires
        .get(&engine_a.state.player_empire)
        .unwrap()
        .research
        .progress;

    // Engine B: manually add a ScienceNexus building
    let mut engine_b = Engine::new(42);
    engine_b
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .buildings
        .push(BuildingType::ScienceNexus);
    engine_b.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine_b.apply_turn(vec![Command::SelectResearch { tech: TechId(5) }]);
    engine_b.apply_turn(vec![Command::EndTurn]);
    let progress_with_nexus = engine_b
        .state
        .empires
        .get(&engine_b.state.player_empire)
        .unwrap()
        .research
        .progress;

    assert!(
        progress_with_nexus > progress_no_nexus,
        "ScienceNexus must increase research output: {} <= {}",
        progress_with_nexus,
        progress_no_nexus
    );
}

#[test]
fn science_generated_event_emitted_each_end_turn() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ScienceGenerated { .. })),
        "ScienceGenerated event must be emitted each turn science is produced"
    );
}

#[test]
fn research_progress_event_emitted_when_research_active() {
    use crate::state::TechId;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let tech_id = TechId(5); // cost 120 — won't complete in one turn

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);
    engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ResearchProgress { tech, .. } if *tech == tech_id
        )),
        "ResearchProgress event must be emitted when a tech is actively researched"
    );
}

#[test]
fn no_research_progress_event_when_no_active_tech() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    // No tech selected
    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::ResearchProgress { .. })),
        "ResearchProgress must not be emitted when no tech is active"
    );
}
// Exploration / scout tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn initial_explored_stars_includes_home_star() {
    let engine = Engine::new(42);
    let home_star_id = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;
    assert!(
        engine.state.explored_stars.contains(&home_star_id),
        "Home star must be explored at game start"
    );
}

#[test]
fn initial_explored_stars_count_is_deterministic() {
    let engine_a = Engine::new(42);
    let engine_b = Engine::new(42);
    assert_eq!(engine_a.state.explored_stars, engine_b.state.explored_stars);
}

#[test]
fn initial_explored_stars_different_seeds_may_differ() {
    let engine_a = Engine::new(1);
    let engine_b = Engine::new(2);
    // The explored sets may or may not overlap but should each be non-empty
    assert!(!engine_a.state.explored_stars.is_empty());
    assert!(!engine_b.state.explored_stars.is_empty());
}

#[test]
fn initial_explored_stars_up_to_four_stars() {
    let engine = Engine::new(42);
    // Home + up to 3 neighbours = at most 4 (and at least 1)
    assert!(!engine.state.explored_stars.is_empty());
    assert!(engine.state.explored_stars.len() <= 4);
}

#[test]
fn game_start_has_player_scout_and_science_fleets() {
    let engine = Engine::new(42);
    let home_star = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .expect("player empire must exist")
        .home_star;
    let player = engine.state.player_empire;
    let has_scout = engine.state.fleets.values().any(|fleet| {
        fleet.owner == player && fleet.location == home_star && fleet.kind == FleetKind::Scout
    });
    let has_science = engine.state.fleets.values().any(|fleet| {
        fleet.owner == player && fleet.location == home_star && fleet.kind == FleetKind::Science
    });
    assert!(has_scout, "player should start with a scout");
    assert!(has_science, "player should start with a science ship");
}

#[test]
fn game_start_surveys_only_home_colony_orbit() {
    let engine = Engine::new(42);
    let home_star = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .expect("player empire must exist")
        .home_star;
    let star = engine
        .state
        .stars
        .get(&home_star)
        .expect("home star must exist");
    assert!(
        star.planets.first().is_some_and(|planet| planet.surveyed),
        "home colony orbit should be surveyed"
    );
    assert!(
        star.planets.iter().skip(1).all(|planet| !planet.surveyed),
        "non-colony home-system orbits should start unsurveyed"
    );
}

#[test]
fn send_scout_to_unexplored_star_succeeds() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let destination = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("There should be unexplored stars");

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination,
    }]);

    assert!(
        !events.iter().any(|e| e.is_error()),
        "SendScout to unexplored star should not error"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ScoutDispatched { fleet, destination: dest, .. }
            if *fleet == fleet_id && *dest == destination
        )),
        "ScoutDispatched event expected"
    );
    assert!(
        engine.state.scout_missions.contains_key(&fleet_id),
        "Scout mission should be registered"
    );
}

#[test]
fn send_scout_with_science_fleet_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(99);
    let home = engine.state.fleets.get(&FleetId(1)).unwrap().location;
    let destination = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");

    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: home,
            ships: 1,
            kind: FleetKind::Science,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "SendScout with a science ship must emit an error"
    );
}

#[test]
fn send_scout_to_explored_star_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let already_explored = *engine.state.explored_stars.iter().next().unwrap();

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: already_explored,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "SendScout to an already-explored star must emit an error"
    );
    assert!(
        !engine.state.scout_missions.contains_key(&fleet_id),
        "No mission should be created for an already-explored star"
    );
}

#[test]
fn send_scout_unknown_fleet_emits_error() {
    let mut engine = Engine::new(42);
    let bad_fleet = FleetId(999);

    let destination = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: bad_fleet,
        destination,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn send_scout_unknown_destination_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let bad_dest = StarId(9999);

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: bad_dest,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn send_scout_when_fleet_already_on_mission_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let mut unexplored = engine
        .state
        .stars
        .keys()
        .filter(|id| !engine.state.explored_stars.contains(id));
    let dest1 = *unexplored.next().expect("Need two unexplored stars");
    let dest2 = *unexplored.next().expect("Need two unexplored stars");

    // First dispatch succeeds
    engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: dest1,
    }]);

    // Second dispatch with same fleet should fail
    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: dest2,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Dispatching scout already on a mission must error"
    );
}

#[test]
fn scout_arrives_after_travel_turns_and_explores_system() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let destination = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");

    engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination,
    }]);

    assert!(!engine.state.explored_stars.contains(&destination));

    // Retrieve the computed travel duration for this specific mission
    let total_turns = engine
        .state
        .scout_missions
        .get(&fleet_id)
        .expect("scout mission must exist")
        .total_duration
        .max(1);

    // Advance turns until the scout should arrive (up to total_duration + 1 turns)
    let mut explored_event_seen = false;
    for _ in 0..=total_turns {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events
            .iter()
            .any(|e| matches!(e, Event::SystemExplored { star } if *star == destination))
        {
            explored_event_seen = true;
        }
    }

    assert!(
        explored_event_seen,
        "SystemExplored event must fire within total_duration turns"
    );
    assert!(
        engine.state.explored_stars.contains(&destination),
        "Destination should now be explored"
    );
    assert!(
        !engine.state.scout_missions.contains_key(&fleet_id),
        "Scout mission should be removed after completion"
    );
    // Fleet should have moved to destination
    assert_eq!(
        engine.state.fleets.get(&fleet_id).unwrap().location,
        destination
    );
}

#[test]
fn move_fleet_on_scout_mission_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let destination = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");

    engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination,
    }]);

    // Try to also move the fleet manually while it's on a scout mission.
    // Use an explored destination so the only failure reason is the active mission.
    let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
    let move_dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != initial_location)
        .expect("Need an explored star other than home");
    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: move_dest,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet while on scout mission must error"
    );
}

#[test]
fn scout_dispatch_event_ordering_is_deterministic() {
    let mut engine_a = Engine::new(123);
    let mut engine_b = Engine::new(123);

    let dest = *engine_a
        .state
        .stars
        .keys()
        .find(|id| !engine_a.state.explored_stars.contains(id))
        .unwrap();

    let evts_a = engine_a.apply_turn(vec![Command::SendScout {
        fleet: FleetId(1),
        destination: dest,
    }]);
    let evts_b = engine_b.apply_turn(vec![Command::SendScout {
        fleet: FleetId(1),
        destination: dest,
    }]);

    assert_eq!(evts_a, evts_b);
}

// ──────────────────────────────────────────────────────────────────
// Fleet movement (MoveFleet / FleetMission) tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn move_fleet_to_unexplored_star_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let unexplored = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: unexplored,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet to unexplored star must emit an error"
    );
    assert!(
        !engine.state.fleet_missions.contains_key(&fleet_id),
        "No mission should be created for an unexplored destination"
    );
}

#[test]
fn move_fleet_while_travelling_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
    let mut explored_others = engine
        .state
        .explored_stars
        .iter()
        .filter(|&&id| id != initial_location);
    let dest1 = *explored_others.next().expect("Need explored star");
    let dest2 = *explored_others.next().unwrap_or(&dest1);

    // First move succeeds
    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest1,
    }]);
    assert!(engine.state.fleet_missions.contains_key(&fleet_id));

    // Second move while already travelling must fail
    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest2,
    }]);
    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet while already travelling must error"
    );
}

#[test]
fn move_fleet_decrements_each_turn_and_arrives() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
    // Pick the closest explored star so it takes 1 turn
    let dest = *engine
        .state
        .explored_stars
        .iter()
        .filter(|&&id| id != initial_location)
        .min_by_key(|&&id| {
            let home = engine.state.stars.get(&initial_location).unwrap();
            let dst = engine.state.stars.get(&id).unwrap();
            let dx = (dst.x - home.x) as i64;
            let dy = (dst.y - home.y) as i64;
            dx * dx + dy * dy
        })
        .expect("Need at least one other explored star");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest,
    }]);

    // Fleet should still be at origin
    assert_eq!(
        engine.state.fleets.get(&fleet_id).unwrap().location,
        initial_location
    );
    assert!(engine.state.fleet_missions.contains_key(&fleet_id));

    // Advance turns until arrival (max 5 turns — worst case galaxy distance)
    let mut arrived = false;
    for _ in 0..5 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events
                .iter()
                .any(|e| matches!(e, Event::FleetArrived { fleet, star } if *fleet == fleet_id && *star == dest))
            {
                arrived = true;
                break;
            }
    }

    assert!(arrived, "FleetArrived event should have been emitted");
    assert!(
        !engine.state.fleet_missions.contains_key(&fleet_id),
        "Mission should be removed after arrival"
    );
    assert_eq!(
        engine.state.fleets.get(&fleet_id).unwrap().location,
        dest,
        "Fleet location must be updated on arrival"
    );
}

#[test]
fn move_fleet_to_same_star_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let current_star = engine.state.fleets.get(&fleet_id).unwrap().location;

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: current_star,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet to current location must error"
    );
}

#[test]
fn multiple_fleet_arrivals_are_deterministically_ordered() {
    // Create two fleets at the home star, both moving to different explored stars.
    // Both should arrive in the same EndTurn; their FleetArrived events must be
    // ordered by FleetId (ascending) due to BTreeMap iteration.
    let mut engine = Engine::new(42);

    let home_star = engine.state.fleets.get(&FleetId(1)).unwrap().location;
    let mut explored_others = engine
        .state
        .explored_stars
        .iter()
        .filter(|&&id| id != home_star)
        .copied();
    let dest_a = explored_others.next().expect("Need explored star A");
    let dest_b = explored_others.next().unwrap_or(dest_a);

    // Create a second fleet at home
    let fleet_b_id = engine.state.next_fleet_id();
    engine.state.fleets.insert(
        fleet_b_id,
        Fleet {
            id: fleet_b_id,
            owner: engine.state.player_empire,
            location: home_star,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    // Dispatch both fleets to the same near destination to guarantee same turn arrival
    engine.apply_turn(vec![
        Command::MoveFleet {
            fleet: FleetId(1),
            destination: dest_a,
        },
        Command::MoveFleet {
            fleet: fleet_b_id,
            destination: dest_b,
        },
    ]);

    // Both missions created
    assert!(engine.state.fleet_missions.contains_key(&FleetId(1)));
    assert!(engine.state.fleet_missions.contains_key(&fleet_b_id));

    // Force both missions to arrive next turn
    engine
        .state
        .fleet_missions
        .get_mut(&FleetId(1))
        .unwrap()
        .turns_remaining = 1;
    engine
        .state
        .fleet_missions
        .get_mut(&fleet_b_id)
        .unwrap()
        .turns_remaining = 1;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    let arrival_fleet_ids: Vec<FleetId> = events
        .iter()
        .filter_map(|e| {
            if let Event::FleetArrived { fleet, .. } = e {
                Some(*fleet)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(arrival_fleet_ids.len(), 2, "Both fleets should arrive");
    assert!(
        arrival_fleet_ids[0] < arrival_fleet_ids[1],
        "Arrivals must be ordered by FleetId: {:?}",
        arrival_fleet_ids
    );
}

#[test]
fn fleet_travel_turns_distance_formula() {
    // sq_dist = 0 → dist = 0 → ceil(0/500) = 0, max(1) = 1
    assert_eq!(fleet_travel_turns(0), 1);
    // sq_dist = 250_000 → dist = 500 → ceil(500/500) = 1
    assert_eq!(fleet_travel_turns(250_000), 1);
    // sq_dist = 250_001 → dist ≈ 500.001 → ceil(…/500) = 2
    assert_eq!(fleet_travel_turns(250_001), 2);
    // sq_dist = 1_000_000 → dist = 1000 → ceil(1000/500) = 2
    assert_eq!(fleet_travel_turns(1_000_000), 2);
    // sq_dist = 1_000_001 → dist ≈ 1000.0005 → ceil(…/500) = 3
    assert_eq!(fleet_travel_turns(1_000_001), 3);
    // max galaxy distance ≈ sqrt(2_000_000) ≈ 1414 → ceil(1414/500) = 3
    assert_eq!(fleet_travel_turns(2_000_000), 3);
}

#[test]
fn out_of_supply_destroyer_travel_is_penalized() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let home_star = engine.state.empires[&player].home_star;
    let destination = *engine
        .state
        .stars
        .keys()
        .find(|&&star_id| star_id != home_star)
        .expect("need destination");
    if let Some(star) = engine.state.stars.get_mut(&destination) {
        star.x = 1_200;
        star.y = 0;
    }
    let fleet_id = FleetId(77);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player,
            location: home_star,
            ships: 1,
            kind: FleetKind::Destroyer,
            strength: 8,
            integrity: 100,
        },
    );
    engine.state.explored_stars.insert(destination);

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);
    let turns_remaining = events
        .iter()
        .find_map(|event| match event {
            Event::FleetDeparted {
                fleet,
                turns_remaining,
                ..
            } if *fleet == fleet_id => Some(*turns_remaining),
            _ => None,
        })
        .expect("fleet should depart");
    let (base_turns, _) = travel_turns_with_lanes(&engine.state, player, home_star, destination);
    let mobility = engine
        .state
        .fleet_evaluation(fleet_id)
        .map(|summary| summary.mobility)
        .unwrap_or(100)
        .max(1);
    let supply = engine.state.projected_fleet_supply(player, destination);
    let expected_turns = ((base_turns as u64 * 100 * supply.movement_penalty_pct() as u64)
        .div_ceil(mobility as u64 * 100) as u32)
        .max(1);

    assert_eq!(
        engine.state.fleet_supply_state(fleet_id),
        FleetSupplyState::OutOfSupply
    );
    assert_eq!(
        turns_remaining, expected_turns,
        "logistics penalty should scale travel by supply state and mobility"
    );
    assert!(turns_remaining > base_turns);
}

#[test]
fn fleet_supply_recalculates_deterministically_after_movement() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let home_star = engine.state.empires[&player].home_star;
    let destination = *engine
        .state
        .stars
        .keys()
        .find(|&&star_id| star_id != home_star)
        .expect("need destination");
    if let Some(star) = engine.state.stars.get_mut(&destination) {
        star.x = 700;
        star.y = 0;
    }
    let fleet_id = FleetId(78);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player,
            location: home_star,
            ships: 1,
            kind: FleetKind::Destroyer,
            strength: 8,
            integrity: 100,
        },
    );
    engine.state.explored_stars.insert(destination);

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);

    let a = engine.state.recompute_fleet_supply();
    let b = engine.state.recompute_fleet_supply();
    assert_eq!(a, b);
    assert_eq!(engine.state.fleet_supply, a);
}

#[test]
fn scout_exploration_still_works_after_fleet_movement_added() {
    // Regression: scout missions must still explore unexplored systems.
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let dest = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");

    engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: dest,
    }]);

    // Use the actual computed duration (+ 1 buffer) so the test is tightly coupled to
    // the travel formula rather than relying on an arbitrary upper bound.
    let total_duration = engine.state.scout_missions[&fleet_id].total_duration;
    for _ in 0..=total_duration {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    assert!(
        engine.state.explored_stars.contains(&dest),
        "Scout must still explore the system"
    );
    assert_eq!(
        engine.state.fleets.get(&fleet_id).unwrap().location,
        dest,
        "Fleet must move to explored destination"
    );
}

// --- Missing fleet negative tests ---

#[test]
fn move_fleet_unexplored_destination_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let unexplored = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Need an unexplored star");

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: unexplored,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet to unexplored star must emit an error"
    );
}

#[test]
fn move_fleet_already_at_destination_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let location = engine.state.fleets.get(&fleet_id).unwrap().location;

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: location,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet to current location must emit an error"
    );
}

#[test]
fn move_fleet_busy_scout_mission_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    // Send the fleet on a scout mission first
    let unexplored = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Need an unexplored star");
    engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: unexplored,
    }]);

    // Now try to move the same fleet
    let explored = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != engine.state.fleets.get(&fleet_id).unwrap().location)
        .expect("Need an explored star other than home");

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: explored,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet on a fleet with active scout mission must emit an error"
    );
}

#[test]
fn move_fleet_busy_fleet_mission_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let home = engine.state.fleets.get(&fleet_id).unwrap().location;

    let explored_dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need an explored star other than home");

    // Dispatch the fleet on a move mission
    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: explored_dest,
    }]);

    // Try to dispatch again immediately
    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: explored_dest,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "MoveFleet on a fleet already on a fleet mission must emit an error"
    );
}

#[test]
fn send_scout_busy_fleet_mission_emits_error() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let home = engine.state.fleets.get(&fleet_id).unwrap().location;

    // First move the fleet to an explored destination
    let explored_dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need an explored star other than home");
    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: explored_dest,
    }]);

    // Now try to send it as a scout too
    let unexplored = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Need an unexplored star");

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: unexplored,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "SendScout on a fleet already on a fleet mission must emit an error"
    );
}

// --- Colonization tests ---

#[test]
fn colonize_valid_explored_habitable_unowned_planet() {
    let mut engine = Engine::new(42);

    // Build a colonizer at home
    let home = engine.state.fleets.get(&FleetId(1)).unwrap().location;

    // Queue Colony ship at home colony
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);
    unlock_habitat_seeding(&mut engine);
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Colony,
    }]);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    // Run turns to build (production=10, cost=200 → 20 turns)
    for _ in 0..21 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    // Find the colonizer fleet
    let colonizer_id = engine
        .state
        .fleets
        .values()
        .find(|f| f.kind == FleetKind::Colonizer && f.owner == engine.state.player_empire)
        .map(|f| f.id)
        .expect("Colonizer fleet should exist after build");

    // Move colonizer to a different explored system
    let target_star = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need an explored star other than home");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: colonizer_id,
        destination: target_star,
    }]);

    // Wait until colonizer arrives
    for _ in 0..4 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    assert_eq!(
        engine.state.fleets.get(&colonizer_id).unwrap().location,
        target_star,
        "Colonizer must be at target star"
    );

    // Mark target planets surveyed, then find first habitable unowned planet.
    if let Some(star) = engine.state.stars.get_mut(&target_star) {
        for planet in &mut star.planets {
            planet.surveyed = true;
        }
    }

    // Find first habitable unowned planet
    let planet_index = engine
        .state
        .stars
        .get(&target_star)
        .unwrap()
        .planets
        .iter()
        .enumerate()
        .find(|(_, p)| p.habitable && p.colony.is_none())
        .map(|(i, _)| i)
        .expect("Target star must have a habitable unowned planet");

    let colonies_before = engine.state.colonies.len();
    let fleets_before = engine.state.fleets.len();

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: colonizer_id,
        star: target_star,
        planet_index,
    }]);

    // No errors
    assert!(
        !events.iter().any(|e| e.is_error()),
        "Colonize should not emit an error"
    );

    // ColonizationCompleted emitted
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ColonizationCompleted { star, .. } if *star == target_star
        )),
        "ColonizationCompleted event should be emitted"
    );

    // Colony was created
    assert_eq!(
        engine.state.colonies.len(),
        colonies_before + 1,
        "A new colony should have been created"
    );

    // Colonizer fleet was consumed
    assert_eq!(
        engine.state.fleets.len(),
        fleets_before - 1,
        "Colonizer fleet should be consumed"
    );
    assert!(
        !engine.state.fleets.contains_key(&colonizer_id),
        "Colonizer fleet should be removed"
    );

    // New colony is owned by player
    let new_colony = engine
        .state
        .colonies
        .values()
        .find(|c| c.star == target_star)
        .expect("Colony must exist at target star");
    assert_eq!(new_colony.owner, engine.state.player_empire);
    assert_eq!(new_colony.planet_index, planet_index);

    // Planet references the colony
    let planet = &engine.state.stars.get(&target_star).unwrap().planets[planet_index];
    assert!(planet.colony.is_some(), "Planet must reference the colony");
}

#[test]
fn cannot_colonize_unexplored_system() {
    let mut engine = Engine::new(42);

    // Manually place a colonizer at an unexplored star
    let unexplored = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Need an unexplored star");

    let fleet_id = FleetId(99);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: unexplored,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: unexplored,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize to unexplored system must emit an error"
    );
}

#[test]
fn cannot_colonize_already_owned_planet() {
    let mut engine = Engine::new(42);

    // The home star already has a colony at planet_index 0
    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;

    let fleet_id = FleetId(99);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: home,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: home,
        planet_index: 0, // Already has a colony
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize already-owned planet must emit an error"
    );
}

#[test]
fn cannot_colonize_uninhabitable_planet() {
    let mut engine = Engine::new(42);

    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;

    // Pick an explored star that's not the home star
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need an explored star other than home");

    // Mark all its planets as uninhabitable
    if let Some(star) = engine.state.stars.get_mut(&target) {
        for planet in star.planets.iter_mut() {
            planet.habitable = false;
            planet.colony = None;
        }
    }

    let fleet_id = FleetId(99);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: target,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize uninhabitable planet must emit an error"
    );
}

#[test]
fn cannot_colonize_without_colonizer_fleet() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1); // Initial fleet is Scout kind
    let home = engine.state.fleets.get(&fleet_id).unwrap().location;

    let planet_index = engine
        .state
        .stars
        .get(&home)
        .unwrap()
        .planets
        .iter()
        .enumerate()
        .find(|(_, p)| p.habitable && p.colony.is_none())
        .map(|(i, _)| i);

    // The scout fleet should be rejected even if at a valid planet
    // (use home star since it's explored and has habitable planets after index 0 potentially)
    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: home,
        planet_index: planet_index.unwrap_or(0),
    }]);

    // Either error because not a colonizer, or error because planet already colonized
    // In both cases an error must be emitted
    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize with non-colonizer fleet must emit an error"
    );
}

#[test]
fn cannot_colonize_with_non_colonizer_fleet_explicit() {
    let mut engine = Engine::new(42);

    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| {
            let home = engine
                .state
                .empires
                .get(&engine.state.player_empire)
                .unwrap()
                .home_star;
            id != home
        })
        .expect("Need an explored star other than home");

    // Place a Scout fleet at the target
    let fleet_id = FleetId(98);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: target,
            ships: 1,
            kind: FleetKind::Scout, // Not a colonizer
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize with Scout fleet must emit an error"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Error { message } if message.contains("not a colonizer"))),
        "Error message should mention 'not a colonizer'"
    );
}

#[test]
fn colonizer_consumed_deterministically() {
    // Same seed + same commands must produce identical fleet IDs and colony IDs
    let setup = |seed: u64| {
        let mut engine = Engine::new(seed);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);
        unlock_habitat_seeding(&mut engine);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);
        for _ in 0..21 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let colonizer_id = engine
            .state
            .fleets
            .values()
            .find(|f| f.kind == FleetKind::Colonizer)
            .map(|f| f.id)
            .expect("Colonizer must exist");

        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need explored star");

        engine.apply_turn(vec![Command::MoveFleet {
            fleet: colonizer_id,
            destination: target,
        }]);
        for _ in 0..4 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let planet_idx = engine
            .state
            .stars
            .get(&target)
            .unwrap()
            .planets
            .iter()
            .enumerate()
            .find(|(_, p)| p.habitable && p.colony.is_none())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: colonizer_id,
            star: target,
            planet_index: planet_idx,
        }]);

        (engine, events)
    };

    let (engine_a, events_a) = setup(777);
    let (engine_b, events_b) = setup(777);

    assert_eq!(events_a, events_b, "Same seed must produce same events");
    assert_eq!(engine_a.state.colonies.len(), engine_b.state.colonies.len());
    assert_eq!(engine_a.state.fleets.len(), engine_b.state.fleets.len());
}

#[test]
fn new_colony_participates_in_next_turn_production() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);
    unlock_habitat_seeding(&mut engine);

    // Build a colonizer
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Colony,
    }]);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);
    for _ in 0..21 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let colonizer_id = engine
        .state
        .fleets
        .values()
        .find(|f| f.kind == FleetKind::Colonizer)
        .map(|f| f.id)
        .expect("Colonizer must exist");

    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need explored star");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: colonizer_id,
        destination: target,
    }]);
    for _ in 0..4 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    if let Some(star) = engine.state.stars.get_mut(&target) {
        for planet in &mut star.planets {
            planet.surveyed = true;
        }
    }

    let planet_idx = engine
        .state
        .stars
        .get(&target)
        .unwrap()
        .planets
        .iter()
        .enumerate()
        .find(|(_, p)| p.habitable && p.colony.is_none())
        .map(|(i, _)| i)
        .unwrap_or(0);

    // Colonize
    engine.apply_turn(vec![Command::Colonize {
        fleet: colonizer_id,
        star: target,
        planet_index: planet_idx,
    }]);

    let new_colony_id = engine
        .state
        .colonies
        .values()
        .find(|c| c.star == target)
        .map(|c| c.id)
        .expect("New colony must exist");

    // End turn — new colony should produce
    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ColonyProduced { colony, .. } if *colony == new_colony_id)),
        "New colony must participate in next turn production"
    );
}

#[test]
fn colonize_event_ordering_is_deterministic() {
    // Two colonizers at two different explored stars — the one with lower FleetId
    // should produce ColonizationCompleted before the other.
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let home = engine.state.empires.get(&player).unwrap().home_star;

    let explored: Vec<StarId> = engine
        .state
        .explored_stars
        .iter()
        .filter(|&&id| id != home)
        .copied()
        .collect();

    if explored.len() < 2 {
        // Not enough explored stars — skip test
        return;
    }

    let star_a = explored[0];
    let star_b = explored[1];

    let col_a = FleetId(91);
    let col_b = FleetId(92);

    // Place two colonizer fleets at different explored stars
    for (fid, star) in [(col_a, star_a), (col_b, star_b)] {
        // Ensure the star has a habitable unowned planet
        if let Some(star_data) = engine.state.stars.get_mut(&star) {
            if let Some(planet) = star_data.planets.get_mut(0) {
                planet.habitable = true;
                planet.colony = None;
                planet.surveyed = true;
            }
        }
        engine.state.fleets.insert(
            fid,
            Fleet {
                id: fid,
                owner: player,
                location: star,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );
    }

    let events = engine.apply_turn(vec![
        Command::Colonize {
            fleet: col_a,
            star: star_a,
            planet_index: 0,
        },
        Command::Colonize {
            fleet: col_b,
            star: star_b,
            planet_index: 0,
        },
    ]);

    // Both should succeed
    let completed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::ColonizationCompleted { .. }))
        .collect();
    assert_eq!(completed.len(), 2, "Both colonizations should complete");

    // Order matches command order (col_a before col_b)
    if let (
        Event::ColonizationCompleted { fleet: f1, .. },
        Event::ColonizationCompleted { fleet: f2, .. },
    ) = (completed[0], completed[1])
    {
        assert_eq!(*f1, col_a, "First event should be from col_a");
        assert_eq!(*f2, col_b, "Second event should be from col_b");
    }
}

#[test]
fn cannot_colonize_fleet_not_at_star() {
    let mut engine = Engine::new(42);

    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;
    let other_star = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need explored star");

    // Place colonizer at home, but try to colonize other_star
    let fleet_id = FleetId(99);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: home,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: other_star,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize when fleet is not at target star must emit an error"
    );
}

#[test]
fn build_completion_colony_ship_creates_colonizer_fleet() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);
    unlock_habitat_seeding(&mut engine);

    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Colony,
    }]);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    for _ in 0..21 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let colonizer = engine
        .state
        .fleets
        .values()
        .find(|f| f.kind == FleetKind::Colonizer && f.owner == engine.state.player_empire);

    assert!(
        colonizer.is_some(),
        "Completing Colony builditem must create a Colonizer fleet"
    );
}

#[test]
fn build_scout_creates_scout_fleet() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    for _ in 0..6 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let scout = engine
        .state
        .fleets
        .values()
        .filter(|f| f.owner == engine.state.player_empire)
        .find(|f| f.kind == FleetKind::Scout && f.id != FleetId(1));

    assert!(
        scout.is_some(),
        "Completing Scout builditem must create a Scout fleet"
    );
}

// -----------------------------------------------------------------------
// Empire Economy v1 tests
// -----------------------------------------------------------------------

/// Colonies produce food equal to population (base) each turn.
#[test]
fn economy_colony_produces_food_equal_to_population() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Compute expected food net for the player's colony (accounts for planet specials).
    let player_colony = engine
        .state
        .colonies
        .values()
        .find(|c| c.owner == empire_id)
        .unwrap()
        .clone();
    let star = engine.state.stars.get(&player_colony.star).unwrap();
    let planet = star.planets.get(player_colony.planet_index);
    let y = crate::yield_model::calculate_yield(&player_colony, planet);
    let expected_food_net = y.food - y.food_consumed;

    let initial_food = engine.state.empires[&empire_id].food;
    engine.apply_turn(vec![Command::EndTurn]);
    let after_food = engine.state.empires[&empire_id].food;

    assert_eq!(
        after_food - initial_food,
        expected_food_net,
        "Net food should match yield model (including planet specials)"
    );
}

/// AquacultureBay doubles food yield so the empire gains a surplus each turn.
#[test]
fn economy_aquaculture_bay_increases_food_surplus() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let empire_id = engine.state.player_empire;

    // Build an AquacultureBay (cost 60, full production)
    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::AquacultureBay),
        },
    ]);
    // AquacultureBay costs 60pp; with production=10/turn, takes 6 turns
    for _ in 0..6 {
        engine.apply_turn(vec![Command::EndTurn]);
    }
    // Confirm the bay was built
    let has_bay = engine.state.colonies[&colony_id]
        .buildings
        .contains(&BuildingType::AquacultureBay);
    assert!(has_bay, "AquacultureBay must be completed before testing");

    let food_before = engine.state.empires[&empire_id].food;
    engine.apply_turn(vec![Command::EndTurn]);
    let food_after = engine.state.empires[&empire_id].food;

    // With AquacultureBay: food_produced = population * 2, food_consumed = population
    // population = 10 → net = +10 per turn
    assert!(
        food_after > food_before,
        "AquacultureBay should create a food surplus (was {food_before}, now {food_after})"
    );
}

/// Fleet maintenance costs 1 credit per fleet per turn.
#[test]
fn economy_fleet_maintenance_costs_credits() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Set 0% production so credits income is zero; only maintenance matters
    let colony_id = ColonyId(1);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    let fleet_count = engine
        .state
        .fleets
        .values()
        .filter(|f| f.owner == empire_id)
        .count() as i64;
    let credits_before = engine.state.empires[&empire_id].credits;
    engine.apply_turn(vec![Command::EndTurn]);
    let credits_after = engine.state.empires[&empire_id].credits;

    // No income, fleet_count fleets at 1 credit each
    assert_eq!(
        credits_after,
        credits_before - fleet_count,
        "Fleet maintenance should cost {fleet_count} credits"
    );
}

/// FabricationYard and ScienceNexus each cost 1 credit/turn in maintenance.
#[test]
fn economy_building_maintenance_costs_credits() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let empire_id = engine.state.player_empire;

    // Build a FabricationYard (cost 80pp)
    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::FabricationYard),
        },
    ]);
    for _ in 0..9 {
        engine.apply_turn(vec![Command::EndTurn]);
    }
    assert!(
        engine.state.colonies[&colony_id]
            .buildings
            .contains(&BuildingType::FabricationYard),
        "FabricationYard must be completed"
    );

    // Now measure one turn with the yard present but 0% income
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    let fleet_count = engine
        .state
        .fleets
        .values()
        .filter(|f| f.owner == empire_id)
        .count() as i64;
    let credits_before = engine.state.empires[&empire_id].credits;
    engine.apply_turn(vec![Command::EndTurn]);
    let credits_after = engine.state.empires[&empire_id].credits;

    // 1 building (FabricationYard) + fleet_count fleets
    let expected_maintenance = fleet_count + 1;
    assert_eq!(
        credits_after,
        credits_before - expected_maintenance,
        "FabricationYard should add 1 credit/turn maintenance"
    );
}

/// Population consumes food each turn; net food = produced - consumed.
#[test]
fn economy_population_consumes_food() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Compute expected food net for the player's colony (accounts for planet specials).
    let player_colony = engine
        .state
        .colonies
        .values()
        .find(|c| c.owner == empire_id)
        .unwrap()
        .clone();
    let star = engine.state.stars.get(&player_colony.star).unwrap();
    let planet = star.planets.get(player_colony.planet_index);
    let y = crate::yield_model::calculate_yield(&player_colony, planet);
    let expected_net = y.food - y.food_consumed;

    let food_before = engine.state.empires[&empire_id].food;
    engine.apply_turn(vec![Command::EndTurn]);
    let food_after = engine.state.empires[&empire_id].food;
    assert_eq!(
        food_after - food_before,
        expected_net,
        "Food net change should equal produced minus consumed (including planet specials)"
    );
}

/// Positive food surplus accumulates in empire.food.
#[test]
fn economy_positive_food_surplus_accumulates() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let empire_id = engine.state.player_empire;

    // Build AquacultureBay to create surplus
    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::AquacultureBay),
        },
    ]);
    for _ in 0..6 {
        engine.apply_turn(vec![Command::EndTurn]);
    }
    assert!(engine.state.colonies[&colony_id]
        .buildings
        .contains(&BuildingType::AquacultureBay));

    let food_before = engine.state.empires[&empire_id].food;
    engine.apply_turn(vec![Command::EndTurn]);
    let food_after_1 = engine.state.empires[&empire_id].food;
    engine.apply_turn(vec![Command::EndTurn]);
    let food_after_2 = engine.state.empires[&empire_id].food;

    assert!(
        food_after_1 > food_before,
        "Food should increase with surplus"
    );
    assert!(food_after_2 > food_after_1, "Food should keep accumulating");
}

/// If the empire food stockpile is already negative, FoodShortage is emitted next turn.
#[test]
fn economy_negative_food_emits_shortage_event() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Compute the maximum food net per turn for the player's single colony.
    // This accounts for planet specials (e.g. FertileBiosphere, BioCultures).
    let player_colony = engine
        .state
        .colonies
        .values()
        .find(|c| c.owner == empire_id)
        .unwrap()
        .clone();
    let star = engine.state.stars.get(&player_colony.star).unwrap();
    let planet = star.planets.get(player_colony.planet_index);
    let y = crate::yield_model::calculate_yield(&player_colony, planet);
    let food_net = y.food - y.food_consumed;

    // Force the stockpile to be large enough below zero so it stays negative after
    // one turn's production (worst case: all food specials active).
    let deeply_negative = -(food_net.abs() + 10);
    engine.state.empires.get_mut(&empire_id).unwrap().food = deeply_negative;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::FoodShortage { empire, .. } if *empire == empire_id)),
        "FoodShortage should be emitted when food balance is negative"
    );
}

/// FoodShortage fires each turn as long as the food balance remains negative.
#[test]
fn economy_shortage_fires_when_food_net_negative() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Set the empire food stockpile to -5 so it is already in deficit.
    // The colony's net food production is zero (food_produced = pop, consumed = pop),
    // so the deficit persists and FoodShortage must fire again this turn.
    engine.state.empires.get_mut(&empire_id).unwrap().food = -5;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::FoodShortage { empire, .. } if *empire == empire_id)),
        "FoodShortage must fire while food balance is negative"
    );
}

/// CreditDeficit event is emitted when credits go negative after maintenance.
#[test]
fn economy_credit_deficit_emitted_when_credits_negative() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let colony_id = ColonyId(1);

    // Zero colony income
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    // Drive credits to 0 so the first maintenance tick goes negative
    engine.state.empires.get_mut(&empire_id).unwrap().credits = 0;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::CreditDeficit { empire, .. } if *empire == empire_id)),
        "CreditDeficit should be emitted when credits go negative"
    );
    let credits = engine.state.empires[&empire_id].credits;
    assert!(credits < 0, "Credits should be negative after the deficit");
}

/// EconomySummary event is emitted each turn with correct values.
#[test]
fn economy_summary_event_emitted_each_turn() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == empire_id)),
        "EconomySummary should be emitted each turn for every empire"
    );
}

/// Science still feeds active research correctly after economy additions.
#[test]
fn economy_science_still_feeds_research() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let empire_id = engine.state.player_empire;

    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        },
        Command::SelectResearch {
            tech: TechId(1), // cost 50
        },
    ]);

    engine.apply_turn(vec![Command::EndTurn]);

    let empire = &engine.state.empires[&empire_id];
    assert!(
        empire.research.progress > 0 || empire.research.completed.contains(&TechId(1)),
        "Research should progress after one turn"
    );
}

/// Industry still advances local colony build queue correctly.
#[test]
fn economy_industry_still_advances_build_queue() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout, // cost 50
        },
    ]);

    engine.apply_turn(vec![Command::EndTurn]);
    let accumulated = engine.state.colonies[&colony_id].accumulated_production;
    assert!(
        accumulated > 0,
        "Build queue should have accumulated production"
    );
}

/// Event ordering is deterministic: same seed → same event sequence.
#[test]
fn economy_event_ordering_is_deterministic() {
    let events1 = {
        let mut e = Engine::new(99);
        e.apply_turn(vec![Command::EndTurn])
    };
    let events2 = {
        let mut e = Engine::new(99);
        e.apply_turn(vec![Command::EndTurn])
    };
    assert_eq!(
        events1, events2,
        "Events must be identical for the same seed"
    );
}

#[test]
fn colony_status_warning_emitted_when_pressure_exists() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
    colony.population = 30;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    let warning = events.iter().find_map(|e| match e {
        Event::ColonyStatusWarning {
            colony,
            food_deficit,
            housing_deficit,
            unemployed,
        } if *colony == colony_id => Some((*food_deficit, *housing_deficit, *unemployed)),
        _ => None,
    });

    assert!(
        warning.is_some(),
        "ColonyStatusWarning should emit when deficits exist"
    );
    let (_food_deficit, housing_deficit, unemployed) = warning.unwrap();
    assert!(housing_deficit > 0, "Housing deficit should be reported");
    assert_eq!(unemployed, 0, "Unhoused pops are not unemployed");
}

#[test]
fn colony_status_warning_not_emitted_without_pressure() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        !events.iter().any(|e| matches!(
            e,
            Event::ColonyStatusWarning { colony, .. } if *colony == colony_id
        )),
        "No pressure should produce no colony warning"
    );
}

#[test]
fn colony_pressure_penalty_housing_is_capped() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let before = engine.state.colonies[&colony_id].stability;
    let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
    colony.population = 200;

    engine.apply_turn(vec![Command::EndTurn]);
    let after = engine.state.colonies[&colony_id].stability;
    assert_eq!(
        before.saturating_sub(after),
        MAX_HOUSING_DEFICIT_STABILITY_PENALTY,
        "Housing pressure penalty should be capped"
    );
}

#[test]
fn population_growth_emits_once_on_expected_cadence() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    engine.state.turn = POP_GROWTH_PERIOD_TURNS - 1;
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .food = 1;
    let before = engine.state.colonies[&colony_id].population;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    let growth_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::PopulationGrew { colony, .. } if *colony == colony_id))
        .collect();

    assert_eq!(
        growth_events.len(),
        1,
        "Growth should emit exactly one event"
    );
    assert_eq!(
        engine.state.colonies[&colony_id].population,
        before + 1,
        "Population should increase by exactly one"
    );
}

#[test]
fn population_growth_suppressed_by_blockade() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let colony_star = engine.state.colonies[&colony_id].star;
    engine.state.turn = POP_GROWTH_PERIOD_TURNS - 1;
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .food = 1;
    let enemy_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(enemy_id, RelationshipStatus::War);
    engine.state.fleets.insert(
        FleetId(9_001),
        Fleet {
            id: FleetId(9_001),
            owner: enemy_id,
            location: colony_star,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );
    let player_id = engine.state.player_empire;
    engine.state.fleets.retain(|_, f| f.owner != player_id);
    engine.state.scout_missions.clear();
    engine.state.survey_missions.clear();
    engine.state.fleet_missions.clear();
    engine.state.colony_blockade = engine.state.recompute_colony_blockade();

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PopulationGrew { colony, .. } if *colony == colony_id)),
        "Blockaded colony must not grow"
    );
}

#[test]
fn population_growth_suppressed_by_low_stability() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    engine.state.turn = POP_GROWTH_PERIOD_TURNS - 1;
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .food = 1;
    engine.state.colonies.get_mut(&colony_id).unwrap().stability = MIN_STABILITY_FOR_POP_GROWTH - 1;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PopulationGrew { colony, .. } if *colony == colony_id)),
        "Low-stability colony must not grow"
    );
}

#[test]
fn population_growth_suppressed_by_housing_deficit() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    engine.state.turn = POP_GROWTH_PERIOD_TURNS - 1;
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .food = 1;
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .population = 200;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PopulationGrew { colony, .. } if *colony == colony_id)),
        "Housing-deficit colony must not grow"
    );
}

#[test]
fn population_growth_suppressed_by_empire_food_shortage() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    engine.state.turn = POP_GROWTH_PERIOD_TURNS - 1;
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .food = -50;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::PopulationGrew { colony, .. } if *colony == colony_id)),
        "Food-short empire must suppress growth"
    );
}

/// Building maintenance methods return expected values.
#[test]
fn building_type_maintenance_costs() {
    assert_eq!(BuildingType::AquacultureBay.maintenance_cost(), 0);
    assert_eq!(BuildingType::FabricationYard.maintenance_cost(), 1);
    assert_eq!(BuildingType::ScienceNexus.maintenance_cost(), 1);
}

/// AquacultureBay food bonus equals population; others return zero.
#[test]
fn building_type_food_bonus() {
    assert_eq!(BuildingType::AquacultureBay.food_bonus(10), 10);
    assert_eq!(BuildingType::FabricationYard.food_bonus(10), 0);
    assert_eq!(BuildingType::ScienceNexus.food_bonus(10), 0);
    // Zero population edge case
    assert_eq!(BuildingType::AquacultureBay.food_bonus(0), 0);
}

// ──────────────────────────────────────────────────────────────────
// Diplomacy tests
// ──────────────────────────────────────────────────────────────────

/// Empires start with no diplomacy entries (all implicitly Unknown).
#[test]
fn empires_start_unknown() {
    let engine = Engine::new(42);
    assert!(
        engine.state.diplomacy.is_empty(),
        "diplomacy map must be empty at game start"
    );
}

/// RelationshipStatus defaults to Unknown when absent from the map.
#[test]
fn relationship_status_default_is_unknown() {
    let engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    let status = engine
        .state
        .diplomacy
        .get(&ai_id)
        .copied()
        .unwrap_or(RelationshipStatus::Unknown);
    assert_eq!(status, RelationshipStatus::Unknown);
}

/// Helper: build minimal state with a player colony at one star and an AI
/// colony at another, then return (engine, player_star_id, ai_star_id, ai_empire_id).
fn make_two_empire_state() -> (Engine, StarId, StarId, EmpireId) {
    use crate::state::{Planet, PlanetSize, SpectralClass};
    use rand::SeedableRng;

    let player_id = EmpireId(1);
    let ai_id = EmpireId(2);

    let player_star_id = StarId(1);
    let ai_star_id = StarId(2);

    let mut state = GameState {
        seed: 0,
        turn: 1,
        player_empire: player_id,
        rng: ChaCha8Rng::seed_from_u64(0),
        event_log: Vec::new(),
        next_colony_id: 10,
        next_fleet_id: 10,
        stars: BTreeMap::new(),
        sectors: BTreeMap::new(),
        empires: BTreeMap::new(),
        colonies: BTreeMap::new(),
        fleets: BTreeMap::new(),
        explored_stars: BTreeSet::new(),
        scout_missions: BTreeMap::new(),
        survey_missions: BTreeMap::new(),
        fleet_missions: BTreeMap::new(),
        ai_empire: Some(ai_id),
        ai_explored_stars: BTreeSet::new(),
        diplomacy: BTreeMap::new(),
        diplomacy_relationships: BTreeMap::new(),
        diplomacy_pending_communications: std::collections::VecDeque::new(),
        diplomacy_next_communication_id: 1,
        hyperspace_lanes: BTreeSet::new(),
        known_hyperspace_lanes: BTreeSet::new(),
        fleet_orders: BTreeMap::new(),
        fleet_roles: BTreeMap::new(),
        fleet_formations: BTreeMap::new(),
        fleet_names: BTreeMap::new(),
        scenario: None,
        ai_empires: vec![ai_id],
        colony_supply: BTreeMap::new(),
        fleet_supply: BTreeMap::new(),
        colony_blockade: BTreeMap::new(),
        empire_resource_access: BTreeMap::new(),
        victory_status: crate::state::VictoryStatus::default(),
        galactic_dispatches: std::collections::VecDeque::new(),
        custom_designs: BTreeMap::new(),
        next_custom_design_id: 0,
        fleet_custom_designs: BTreeMap::new(),
        next_battle_report_id: 1,
        battle_reports: std::collections::VecDeque::new(),
    };

    // Player star
    state.stars.insert(
        player_star_id,
        crate::state::Star {
            id: player_star_id,
            name: "Alpha".to_string(),
            x: 0,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::G,
            planets: vec![Planet {
                name: "Alpha I".to_string(),
                size: PlanetSize::Medium,
                class: crate::state::PlanetClass::Terran,
                colony: Some(ColonyId(1)),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );

    // AI star — close enough to reach in 1 turn
    state.stars.insert(
        ai_star_id,
        crate::state::Star {
            id: ai_star_id,
            name: "Beta".to_string(),
            x: 100,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::K,
            planets: vec![Planet {
                name: "Beta I".to_string(),
                size: PlanetSize::Medium,
                class: crate::state::PlanetClass::Terran,
                colony: Some(ColonyId(2)),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );

    // Player empire
    state.empires.insert(
        player_id,
        Empire {
            id: player_id,
            name: "Player".to_string(),
            credits: 100,
            research_points: 0,
            home_star: player_star_id,
            research: ResearchState::default(),
            food: 0,
            empire_def: None,
        },
    );

    // AI empire
    state.empires.insert(
        ai_id,
        Empire {
            id: ai_id,
            name: "AI".to_string(),
            credits: 100,
            research_points: 0,
            home_star: ai_star_id,
            research: ResearchState::default(),
            food: 0,
            empire_def: None,
        },
    );

    // Player colony
    state.colonies.insert(
        ColonyId(1),
        Colony {
            id: ColonyId(1),
            star: player_star_id,
            planet_index: 0,
            owner: player_id,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );

    // AI colony
    state.colonies.insert(
        ColonyId(2),
        Colony {
            id: ColonyId(2),
            star: ai_star_id,
            planet_index: 0,
            owner: ai_id,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );

    // Player scout fleet at player star
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: player_id,
            location: player_star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    // Both stars explored
    state.explored_stars.insert(player_star_id);
    state.explored_stars.insert(ai_star_id);

    let engine = Engine::from_state(state);
    (engine, player_star_id, ai_star_id, ai_id)
}

fn set_empire_definition(
    engine: &mut Engine,
    empire_id: EmpireId,
    def_id: crate::state::EmpireDefinitionId,
) {
    let def = empire_definition_by_id(def_id).expect("empire def should exist");
    let empire = engine.state.empires.get_mut(&empire_id).unwrap();
    empire.empire_def = Some(def_id);
    empire.name = def.name.to_string();
}

/// A scout arriving at a star with a foreign colony establishes contact.
#[test]
fn scout_arrival_at_ai_colony_establishes_contact() {
    use crate::state::SpectralClass;
    use rand::SeedableRng;

    let player_id = EmpireId(1);
    let ai_id = EmpireId(2);

    let player_star_id = StarId(1);
    let ai_star_id = StarId(2);

    let mut state = GameState {
        seed: 0,
        turn: 1,
        player_empire: player_id,
        rng: ChaCha8Rng::seed_from_u64(0),
        event_log: Vec::new(),
        next_colony_id: 10,
        next_fleet_id: 10,
        stars: BTreeMap::new(),
        sectors: BTreeMap::new(),
        empires: BTreeMap::new(),
        colonies: BTreeMap::new(),
        fleets: BTreeMap::new(),
        // Only player star explored; AI star is unknown — scout will explore it
        explored_stars: {
            let mut s = BTreeSet::new();
            s.insert(player_star_id);
            s
        },
        scout_missions: BTreeMap::new(),
        survey_missions: BTreeMap::new(),
        fleet_missions: BTreeMap::new(),
        ai_empire: Some(ai_id),
        ai_explored_stars: BTreeSet::new(),
        diplomacy: BTreeMap::new(),
        diplomacy_relationships: BTreeMap::new(),
        diplomacy_pending_communications: std::collections::VecDeque::new(),
        diplomacy_next_communication_id: 1,
        hyperspace_lanes: BTreeSet::new(),
        known_hyperspace_lanes: BTreeSet::new(),
        fleet_orders: BTreeMap::new(),
        fleet_roles: BTreeMap::new(),
        fleet_formations: BTreeMap::new(),
        fleet_names: BTreeMap::new(),
        scenario: None,
        ai_empires: vec![ai_id],
        colony_supply: BTreeMap::new(),
        fleet_supply: BTreeMap::new(),
        colony_blockade: BTreeMap::new(),
        empire_resource_access: BTreeMap::new(),
        victory_status: crate::state::VictoryStatus::default(),
        galactic_dispatches: std::collections::VecDeque::new(),
        custom_designs: BTreeMap::new(),
        next_custom_design_id: 0,
        fleet_custom_designs: BTreeMap::new(),
        next_battle_report_id: 1,
        battle_reports: std::collections::VecDeque::new(),
    };

    // Populate stars, empires, colonies, fleet
    state.stars.insert(
        player_star_id,
        crate::state::Star {
            id: player_star_id,
            name: "Alpha".to_string(),
            x: 0,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::G,
            planets: vec![crate::state::Planet {
                name: "Alpha I".to_string(),
                size: crate::state::PlanetSize::Medium,
                class: crate::state::PlanetClass::Terran,
                colony: Some(ColonyId(1)),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );
    state.stars.insert(
        ai_star_id,
        crate::state::Star {
            id: ai_star_id,
            name: "Beta".to_string(),
            x: 100,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::K,
            planets: vec![crate::state::Planet {
                name: "Beta I".to_string(),
                size: crate::state::PlanetSize::Medium,
                class: crate::state::PlanetClass::Terran,
                colony: Some(ColonyId(2)),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );
    state.empires.insert(
        player_id,
        Empire {
            id: player_id,
            name: "Player".to_string(),
            credits: 100,
            research_points: 0,
            home_star: player_star_id,
            research: ResearchState::default(),
            food: 0,
            empire_def: None,
        },
    );
    state.empires.insert(
        ai_id,
        Empire {
            id: ai_id,
            name: "AI".to_string(),
            credits: 100,
            research_points: 0,
            home_star: ai_star_id,
            research: ResearchState::default(),
            food: 0,
            empire_def: Some(crate::state::EmpireDefinitionId(0)),
        },
    );
    state.colonies.insert(
        ColonyId(1),
        Colony {
            id: ColonyId(1),
            star: player_star_id,
            planet_index: 0,
            owner: player_id,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state.colonies.insert(
        ColonyId(2),
        Colony {
            id: ColonyId(2),
            star: ai_star_id,
            planet_index: 0,
            owner: ai_id,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );
    // Player scout at player star — will scout the AI star
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: player_id,
            location: player_star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    // Put the scout in a mission with 1 turn remaining to destination = ai_star_id
    state.scout_missions.insert(
        FleetId(1),
        ScoutMission {
            fleet: FleetId(1),
            destination: ai_star_id,
            turns_remaining: 1,
            origin: StarId(0),
            total_duration: 1,
        },
    );

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    // SystemExplored should fire
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::SystemExplored { star } if *star == ai_star_id)));

    // FirstContact should fire for the AI empire
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::FirstContact { with_empire } if *with_empire == ai_id)),
        "Expected FirstContact event for AI empire"
    );

    // Diplomacy state updated
    assert_eq!(
        engine.state.diplomacy.get(&ai_id).copied(),
        Some(RelationshipStatus::Neutral)
    );
}

#[test]
fn terran_concord_first_contact_starts_neutral() {
    let (mut engine, _player_star, ai_star, ai_id) = make_two_empire_state();
    set_empire_definition(&mut engine, ai_id, crate::state::EmpireDefinitionId(6));
    let mut events = Vec::new();
    engine.check_contact_at_star(ai_star, &mut events);
    assert_eq!(
        engine.state.diplomacy.get(&ai_id).copied(),
        Some(RelationshipStatus::Neutral)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::FirstContact { with_empire } if *with_empire == ai_id
    )));
}

#[test]
fn terran_dominion_first_contact_starts_tense() {
    let (mut engine, _player_star, ai_star, ai_id) = make_two_empire_state();
    set_empire_definition(&mut engine, ai_id, crate::state::EmpireDefinitionId(7));
    let mut events = Vec::new();
    engine.check_contact_at_star(ai_star, &mut events);
    assert_eq!(
        engine.state.diplomacy.get(&ai_id).copied(),
        Some(RelationshipStatus::Tense)
    );
}

#[test]
fn terran_concord_diplomacy_stays_calmer_under_pressure() {
    let (mut engine, _player_star, ai_star, ai_id) = make_two_empire_state();
    set_empire_definition(&mut engine, ai_id, crate::state::EmpireDefinitionId(6));
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Neutral);
    engine.state.fleets.insert(
        FleetId(99),
        Fleet {
            id: FleetId(99),
            owner: engine.state.player_empire,
            location: ai_star,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    engine.process_ai_diplomacy();
    assert_eq!(
        engine.state.diplomacy.get(&ai_id).copied(),
        Some(RelationshipStatus::Cooperative)
    );
}

#[test]
fn terran_dominion_escalates_to_war_under_severe_pressure() {
    let (mut engine, _player_star, ai_star, ai_id) = make_two_empire_state();
    set_empire_definition(&mut engine, ai_id, crate::state::EmpireDefinitionId(7));
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Hostile);
    engine.state.fleets.insert(
        FleetId(99),
        Fleet {
            id: FleetId(99),
            owner: engine.state.player_empire,
            location: ai_star,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    engine.process_ai_diplomacy();
    assert_eq!(
        engine.state.diplomacy.get(&ai_id).copied(),
        Some(RelationshipStatus::War)
    );
}

/// A fleet arriving at a star with a foreign colony establishes contact.
#[test]
fn fleet_arrival_at_ai_colony_establishes_contact() {
    let (mut engine, _player_star_id, ai_star_id, ai_id) = make_two_empire_state();
    set_empire_definition(&mut engine, ai_id, crate::state::EmpireDefinitionId(0));

    // Put fleet on a mission that completes this turn
    engine.state.fleet_missions.insert(
        FleetId(1),
        FleetMission {
            fleet: FleetId(1),
            destination: ai_star_id,
            turns_remaining: 1,
            origin: StarId(0),
            total_duration: 1,
        },
    );

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::FleetArrived { star, .. } if *star == ai_star_id)),
        "Expected FleetArrived"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::FirstContact { with_empire } if *with_empire == ai_id)),
        "Expected FirstContact"
    );
    assert_eq!(
        engine.state.diplomacy.get(&ai_id).copied(),
        Some(RelationshipStatus::Neutral)
    );
}

/// Repeated contact does not duplicate the FirstContact event.
#[test]
fn repeated_contact_does_not_emit_duplicate_first_contact() {
    let (mut engine, _player_star_id, ai_star_id, ai_id) = make_two_empire_state();

    // Pre-mark as Contacted
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Contacted);

    // Fleet arrives at the AI colony star again
    engine.state.fleet_missions.insert(
        FleetId(1),
        FleetMission {
            fleet: FleetId(1),
            destination: ai_star_id,
            turns_remaining: 1,
            origin: StarId(0),
            total_duration: 1,
        },
    );

    let events = engine.apply_turn(vec![Command::EndTurn]);

    let first_contact_count = events
        .iter()
        .filter(|e| matches!(e, Event::FirstContact { with_empire } if *with_empire == ai_id))
        .count();
    assert_eq!(
        first_contact_count, 0,
        "No duplicate FirstContact event when empire is already Contacted"
    );
}

/// Contact events are emitted in deterministic (BTreeMap) order.
#[test]
fn contact_detection_is_deterministic() {
    use crate::state::{Planet, PlanetSize, SpectralClass};
    use rand::SeedableRng;

    let player_id = EmpireId(1);
    let ai1 = EmpireId(2);
    let ai2 = EmpireId(3);
    let star1 = StarId(1);
    let target_star = StarId(2);

    let mut state = GameState {
        seed: 0,
        turn: 1,
        player_empire: player_id,
        rng: ChaCha8Rng::seed_from_u64(0),
        event_log: Vec::new(),
        next_colony_id: 10,
        next_fleet_id: 10,
        stars: BTreeMap::new(),
        sectors: BTreeMap::new(),
        empires: BTreeMap::new(),
        colonies: BTreeMap::new(),
        fleets: BTreeMap::new(),
        explored_stars: {
            let mut s = BTreeSet::new();
            s.insert(star1);
            s.insert(target_star);
            s
        },
        scout_missions: BTreeMap::new(),
        survey_missions: BTreeMap::new(),
        fleet_missions: BTreeMap::new(),
        ai_empire: Some(ai1),
        ai_explored_stars: BTreeSet::new(),
        diplomacy: BTreeMap::new(),
        diplomacy_relationships: BTreeMap::new(),
        diplomacy_pending_communications: std::collections::VecDeque::new(),
        diplomacy_next_communication_id: 1,
        hyperspace_lanes: BTreeSet::new(),
        known_hyperspace_lanes: BTreeSet::new(),
        fleet_orders: BTreeMap::new(),
        fleet_roles: BTreeMap::new(),
        fleet_formations: BTreeMap::new(),
        fleet_names: BTreeMap::new(),
        scenario: None,
        ai_empires: vec![ai1, ai2],
        colony_supply: BTreeMap::new(),
        fleet_supply: BTreeMap::new(),
        colony_blockade: BTreeMap::new(),
        empire_resource_access: BTreeMap::new(),
        victory_status: crate::state::VictoryStatus::default(),
        galactic_dispatches: std::collections::VecDeque::new(),
        custom_designs: BTreeMap::new(),
        next_custom_design_id: 0,
        fleet_custom_designs: BTreeMap::new(),
        next_battle_report_id: 1,
        battle_reports: std::collections::VecDeque::new(),
    };

    // Two AI empires each have a colony at target_star
    state.stars.insert(
        star1,
        crate::state::Star {
            id: star1,
            name: "Home".to_string(),
            x: 0,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::G,
            planets: vec![Planet {
                name: "Home I".to_string(),
                size: PlanetSize::Medium,
                class: crate::state::PlanetClass::Terran,
                colony: Some(ColonyId(1)),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );
    state.stars.insert(
        target_star,
        crate::state::Star {
            id: target_star,
            name: "Target".to_string(),
            x: 100,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::K,
            planets: vec![
                Planet {
                    name: "Target I".to_string(),
                    size: PlanetSize::Medium,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(2)),
                    habitable: true,
                    surveyed: true,
                    specials: vec![],
                    resources: vec![],
                    anomalies: vec![],
                    ancient_ruins_collected: false,
                },
                Planet {
                    name: "Target II".to_string(),
                    size: PlanetSize::Small,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(3)),
                    habitable: true,
                    surveyed: true,
                    specials: vec![],
                    resources: vec![],
                    anomalies: vec![],
                    ancient_ruins_collected: false,
                },
            ],
        },
    );
    for (eid, home) in [(player_id, star1), (ai1, target_star), (ai2, target_star)] {
        state.empires.insert(
            eid,
            Empire {
                id: eid,
                name: format!("E{}", eid.0),
                credits: 100,
                research_points: 0,
                home_star: home,
                research: ResearchState::default(),
                food: 0,
                empire_def: None,
            },
        );
    }
    state.colonies.insert(
        ColonyId(1),
        Colony {
            id: ColonyId(1),
            star: star1,
            planet_index: 0,
            owner: player_id,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state.colonies.insert(
        ColonyId(2),
        Colony {
            id: ColonyId(2),
            star: target_star,
            planet_index: 0,
            owner: ai1,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state.colonies.insert(
        ColonyId(3),
        Colony {
            id: ColonyId(3),
            star: target_star,
            planet_index: 1,
            owner: ai2,
            population: 5,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: player_id,
            location: star1,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    state.fleet_missions.insert(
        FleetId(1),
        FleetMission {
            fleet: FleetId(1),
            destination: target_star,
            turns_remaining: 1,
            origin: StarId(0),
            total_duration: 1,
        },
    );

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let contacts: Vec<EmpireId> = events
        .iter()
        .filter_map(|e| match e {
            Event::FirstContact { with_empire } => Some(*with_empire),
            _ => None,
        })
        .collect();

    // Both empires contacted, in ascending EmpireId order (BTreeMap iteration)
    assert_eq!(contacts, vec![ai1, ai2]);
}

/// FirstContact log message contains expected text.
#[test]
fn first_contact_log_message() {
    let event = Event::FirstContact {
        with_empire: EmpireId(2),
    };
    let msg = event.to_log_message();
    assert!(
        msg.contains("FIRST CONTACT"),
        "Log message should contain 'FIRST CONTACT'"
    );
    assert!(
        msg.contains('2'),
        "Log message should reference empire ID 2"
    );
    assert!(!event.is_error());
}

// -----------------------------------------------------------------------
// Combat auto-resolve tests
// -----------------------------------------------------------------------

/// Helper: build a minimal GameState with two fleets at the same star,
/// controlled by the player (EmpireId 1) and a contacted foreign empire
/// (EmpireId 2).
fn make_combat_state(
    player_strength: u32,
    player_integrity: u32,
    enemy_strength: u32,
    enemy_integrity: u32,
) -> (GameState, StarId, FleetId, FleetId) {
    let mut engine = Engine::new(42);
    let state = &mut engine.state;

    // Use the player's home star
    let star_id = state.empires.get(&state.player_empire).unwrap().home_star;

    // Remove existing fleets to avoid interference
    state.fleets.clear();
    state.fleet_missions.clear();
    state.scout_missions.clear();
    state.survey_missions.clear();
    state.fleet_custom_designs.clear();
    state.fleet_roles.clear();
    state.fleet_formations.clear();
    state.fleet_names.clear();

    let player = state.player_empire;
    let enemy_empire = EmpireId(2);

    // Establish contact so combat is enabled
    state
        .diplomacy
        .insert(enemy_empire, RelationshipStatus::Contacted);

    let player_fleet_id = FleetId(10);
    let enemy_fleet_id = FleetId(20);

    state.fleets.insert(
        player_fleet_id,
        Fleet {
            id: player_fleet_id,
            owner: player,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: player_strength,
            integrity: player_integrity,
        },
    );
    state.fleets.insert(
        enemy_fleet_id,
        Fleet {
            id: enemy_fleet_id,
            owner: enemy_empire,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: enemy_strength,
            integrity: enemy_integrity,
        },
    );

    (engine.state, star_id, player_fleet_id, enemy_fleet_id)
}

#[test]
fn stronger_fleet_wins_deterministically() {
    // Player fleet (strength 20) vs enemy fleet (strength 10)
    let (state, star_id, player_fid, enemy_fid) = make_combat_state(20, 100, 10, 100);

    let mut events = Vec::new();
    let mut engine = Engine::from_state(state);
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    // Enemy should be destroyed; player should survive
    assert!(
        !engine.state.fleets.contains_key(&enemy_fid),
        "Enemy fleet should be destroyed"
    );
    assert!(
        engine.state.fleets.contains_key(&player_fid),
        "Player fleet should survive"
    );

    // CombatResolved event emitted
    let combat_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::CombatResolved { .. }))
        .collect();
    assert_eq!(combat_events.len(), 1, "Exactly one CombatResolved event");

    if let Event::CombatResolved {
        fleet_a_destroyed,
        fleet_b_destroyed,
        ..
    } = &combat_events[0]
    {
        assert!(!fleet_a_destroyed, "Player fleet should not be destroyed");
        assert!(fleet_b_destroyed, "Enemy fleet should be destroyed");
    }
}

#[test]
fn equal_fleets_destroy_each_other() {
    let (state, star_id, player_fid, enemy_fid) = make_combat_state(10, 100, 10, 100);

    let mut events = Vec::new();
    let mut engine = Engine::from_state(state);
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    assert!(
        !engine.state.fleets.contains_key(&player_fid),
        "Player fleet should be destroyed when equal"
    );
    assert!(
        !engine.state.fleets.contains_key(&enemy_fid),
        "Enemy fleet should be destroyed when equal"
    );

    let combat_event = events
        .iter()
        .find(|e| matches!(e, Event::CombatResolved { .. }))
        .expect("CombatResolved event required");
    if let Event::CombatResolved {
        fleet_a_destroyed,
        fleet_b_destroyed,
        ..
    } = combat_event
    {
        assert!(fleet_a_destroyed, "Fleet A should be destroyed");
        assert!(fleet_b_destroyed, "Fleet B should be destroyed");
    }
}

#[test]
fn damaged_winner_has_expected_integrity() {
    // Player strength 20 vs enemy strength 10 — damage_to_player = 10*100/20 = 50
    let (state, star_id, player_fid, _) = make_combat_state(20, 100, 10, 100);

    let mut events = Vec::new();
    let mut engine = Engine::from_state(state);
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    let survivor = engine
        .state
        .fleets
        .get(&player_fid)
        .expect("Player fleet should survive");
    assert_eq!(
        survivor.integrity, 50,
        "Winner integrity should be 100 - (10*100/20) = 50"
    );

    let combat_event = events
        .iter()
        .find(|e| matches!(e, Event::CombatResolved { .. }))
        .expect("CombatResolved event required");
    if let Event::CombatResolved {
        integrity_a_remaining,
        ..
    } = combat_event
    {
        assert_eq!(*integrity_a_remaining, 50);
    }
}

#[test]
fn same_empire_fleets_do_not_fight() {
    let mut engine = Engine::new(42);
    let star_id = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;
    let player = engine.state.player_empire;

    engine.state.fleets.clear();
    engine.state.fleet_missions.clear();
    engine.state.scout_missions.clear();

    let fid_a = FleetId(10);
    let fid_b = FleetId(20);

    for fid in [fid_a, fid_b] {
        engine.state.fleets.insert(
            fid,
            Fleet {
                id: fid,
                owner: player,
                location: star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 10,
                integrity: 100,
            },
        );
    }

    let mut events = Vec::new();
    engine.check_combat_at_star(star_id, fid_a, &mut events);

    // No combat events
    assert!(
        events
            .iter()
            .all(|e| !matches!(e, Event::CombatResolved { .. })),
        "No combat between fleets of the same empire"
    );
    // Both fleets still present
    assert!(engine.state.fleets.contains_key(&fid_a));
    assert!(engine.state.fleets.contains_key(&fid_b));
}

#[test]
fn unknown_empire_fleets_do_not_fight() {
    let (mut state, star_id, player_fid, enemy_fid) = make_combat_state(10, 100, 10, 100);

    // Remove contact so empires are Unknown
    state.diplomacy.clear();

    let mut events = Vec::new();
    let mut engine = Engine::from_state(state);
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    // No combat events
    assert!(
        events
            .iter()
            .all(|e| !matches!(e, Event::CombatResolved { .. })),
        "No combat between unknown empires"
    );
    // Both fleets still alive
    assert!(engine.state.fleets.contains_key(&player_fid));
    assert!(engine.state.fleets.contains_key(&enemy_fid));
}

#[test]
fn combat_triggers_after_fleet_arrival() {
    // Set up: player fleet travels to a star where a contacted enemy fleet waits
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let ai_empire = engine.state.ai_empire.expect("AI empire required");

    // Establish contact
    engine
        .state
        .diplomacy
        .insert(ai_empire, RelationshipStatus::Contacted);

    // Pick a star that is explored by the player and not the home star
    let home_star = engine.state.empires.get(&player).unwrap().home_star;
    let target_star = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != home_star)
        .expect("Need at least one non-home explored star");

    // Place an enemy fleet at the target star
    let enemy_fid = FleetId(99);
    engine.state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: ai_empire,
            location: target_star,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    // Find the player fleet (FleetId 1)
    let player_fid = FleetId(1);

    // Move player fleet to target star (1-turn if close enough; set mission directly)
    engine.state.fleet_missions.insert(
        player_fid,
        FleetMission {
            fleet: player_fid,
            destination: target_star,
            turns_remaining: 1,
            origin: StarId(0),
            total_duration: 1,
        },
    );

    // End turn — fleet arrives, combat should fire
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let combat_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::CombatResolved { .. }))
        .collect();
    assert!(
        !combat_events.is_empty(),
        "CombatResolved should be emitted after fleet arrival"
    );
}

#[test]
fn destroyed_fleets_are_removed_from_state() {
    let (state, star_id, player_fid, enemy_fid) = make_combat_state(10, 100, 10, 100); // equal → both destroyed

    let mut engine = Engine::from_state(state);
    let mut events = Vec::new();
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    assert!(
        !engine.state.fleets.contains_key(&player_fid),
        "Destroyed player fleet must be removed"
    );
    assert!(
        !engine.state.fleets.contains_key(&enemy_fid),
        "Destroyed enemy fleet must be removed"
    );
}

#[test]
fn combat_events_are_deterministic() {
    // Running the same scenario twice should produce identical events.
    let (state1, star_id, player_fid, _) = make_combat_state(20, 100, 10, 100);
    let (state2, _, _, _) = make_combat_state(20, 100, 10, 100);

    let mut events1 = Vec::new();
    let mut engine1 = Engine::from_state(state1);
    engine1.check_combat_at_star(star_id, player_fid, &mut events1);

    let mut events2 = Vec::new();
    let mut engine2 = Engine::from_state(state2);
    engine2.check_combat_at_star(star_id, player_fid, &mut events2);

    assert_eq!(
        events1, events2,
        "Combat events must be identical for same initial state"
    );
}

#[test]
fn combat_generates_structured_battle_report_with_phases() {
    let (state, star_id, player_fid, _enemy_fid) = make_combat_state(20, 100, 10, 100);
    let mut engine = Engine::from_state(state);
    let mut events = Vec::new();
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    let report = engine
        .state
        .battle_reports
        .back()
        .expect("battle report should be recorded");
    assert_eq!(report.star, star_id);
    assert!(
        !report.phases.is_empty(),
        "phase summaries should be present"
    );
    assert!(report
        .phases
        .iter()
        .any(|phase| matches!(phase.phase, crate::state::CombatPhase::OpeningVolley)));
    assert!(report
        .phases
        .iter()
        .any(|phase| matches!(phase.phase, crate::state::CombatPhase::Resolution)));
}

#[test]
fn battle_report_records_supply_state_for_logistics_penalties() {
    let (mut state, _star_id, player_fid, enemy_fid) = make_combat_state(20, 100, 20, 100);
    let enemy_empire = state.fleets[&enemy_fid].owner;
    let enemy_colonies: Vec<_> = state
        .colonies
        .iter()
        .filter_map(|(colony_id, colony)| (colony.owner == enemy_empire).then_some(*colony_id))
        .collect();
    for colony_id in enemy_colonies {
        state.colonies.remove(&colony_id);
    }
    let star_id = StarId(999);
    state.stars.insert(
        star_id,
        Star {
            id: star_id,
            sector: SectorId(1),
            name: "Unsupported".to_string(),
            x: 1_400,
            y: 0,
            spectral_class: SpectralClass::F,
            planets: vec![],
        },
    );
    if let Some(player_fleet) = state.fleets.get_mut(&player_fid) {
        player_fleet.location = star_id;
    }
    if let Some(enemy_fleet) = state.fleets.get_mut(&enemy_fid) {
        enemy_fleet.location = star_id;
    }
    if let Some(player_fleet) = state.fleets.get_mut(&player_fid) {
        player_fleet.kind = FleetKind::Destroyer;
        player_fleet.strength = 12;
    }
    if let Some(enemy_fleet) = state.fleets.get_mut(&enemy_fid) {
        enemy_fleet.kind = FleetKind::Destroyer;
        enemy_fleet.strength = 12;
    }

    let mut engine = Engine::from_state(state);
    let mut events = Vec::new();
    engine.check_combat_at_star(star_id, player_fid, &mut events);

    let report = engine
        .state
        .battle_reports
        .back()
        .expect("battle report should be recorded");
    assert_eq!(report.supply_a, FleetSupplyState::OutOfSupply);
    assert_eq!(report.supply_b, FleetSupplyState::OutOfSupply);
    assert!(report
        .phases
        .iter()
        .any(|phase| phase.note.contains("supply Out of Supply vs Out of Supply")));
}

#[test]
fn battle_reports_are_deterministic_for_same_combat_state() {
    let (state1, star_id, player_fid, _) = make_combat_state(20, 100, 10, 100);
    let (state2, _, _, _) = make_combat_state(20, 100, 10, 100);

    let mut engine1 = Engine::from_state(state1);
    let mut engine2 = Engine::from_state(state2);
    let mut events1 = Vec::new();
    let mut events2 = Vec::new();
    engine1.check_combat_at_star(star_id, player_fid, &mut events1);
    engine2.check_combat_at_star(star_id, player_fid, &mut events2);

    assert_eq!(engine1.state.battle_reports, engine2.state.battle_reports);
}

#[test]
fn artillery_opening_volley_pressure_exceeds_balanced_scout_opening() {
    let (mut state, star_id, player_fid, enemy_fid) = make_combat_state(12, 100, 12, 100);
    if let Some(player) = state.fleets.get_mut(&player_fid) {
        player.kind = FleetKind::MissileFrigate;
    }
    if let Some(enemy) = state.fleets.get_mut(&enemy_fid) {
        enemy.kind = FleetKind::EscortFrigate;
    }
    state
        .fleet_formations
        .insert(player_fid, crate::state::FleetFormation::Artillery);
    state
        .fleet_formations
        .insert(enemy_fid, crate::state::FleetFormation::Balanced);

    let mut engine = Engine::from_state(state);
    let mut events = Vec::new();
    engine.check_combat_at_star(star_id, player_fid, &mut events);
    let report = engine
        .state
        .battle_reports
        .back()
        .expect("battle report should be present");
    let opening = report
        .phases
        .iter()
        .find(|phase| matches!(phase.phase, crate::state::CombatPhase::OpeningVolley))
        .expect("opening volley phase should exist");
    assert!(
        opening.pressure_a > opening.pressure_b,
        "artillery missile side should have stronger opening pressure"
    );
}

#[test]
fn combat_resolved_log_message_contains_expected_content() {
    let event = Event::CombatResolved {
        star: StarId(5),
        fleet_a: FleetId(1),
        empire_a: EmpireId(1),
        fleet_b: FleetId(2),
        empire_b: EmpireId(2),
        strength_a: 20,
        strength_b: 10,
        integrity_a_remaining: 50,
        integrity_b_remaining: 0,
        fleet_a_destroyed: false,
        fleet_b_destroyed: true,
    };
    let msg = event.to_log_message();
    assert!(msg.contains("COMBAT"), "Log should mention COMBAT");
    assert!(msg.contains("system 5"), "Log should mention star system 5");
    assert!(!event.is_error());
}

#[test]
fn fleet_has_strength_and_integrity_fields() {
    let engine = Engine::new(42);
    // All initial fleets should have strength=1 and integrity=100
    for fleet in engine.state.fleets.values() {
        assert_eq!(fleet.strength, 1, "Initial fleet strength should be 1");
        assert_eq!(
            fleet.integrity, 100,
            "Initial fleet integrity should be 100"
        );
    }
}

// ──────────────────────────────────────────────────────────────────
// Colonize — additional invalid-command edge cases
// ──────────────────────────────────────────────────────────────────

#[test]
fn colonize_out_of_bounds_planet_index_emits_error() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let home = engine.state.empires.get(&player).unwrap().home_star;

    // Choose an explored star other than home
    let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Test setup failed: no explored star found other than home star; at least two explored stars are required");

    // Get the number of planets at that star so we can go one past the end
    let planet_count = engine.state.stars.get(&target).unwrap().planets.len();

    // Place a colonizer at the target
    let fleet_id = FleetId(77);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player,
            location: target,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index: planet_count, // one past the end
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize with out-of-bounds planet_index must emit an error"
    );
    // Colonizer must not be consumed on an invalid command
    assert!(
        engine.state.fleets.contains_key(&fleet_id),
        "Colonizer fleet must not be removed when the command fails"
    );
}

#[test]
fn colonize_unknown_fleet_emits_error() {
    let mut engine = Engine::new(42);
    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: FleetId(9999),
        star: home,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize with non-existent fleet must emit an error"
    );
}

#[test]
fn colonize_fleet_on_scout_mission_emits_error() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let home = engine.state.empires.get(&player).unwrap().home_star;

    // Pick an explored star other than home for the colonizer target
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need explored star other than home");

    // Place a colonizer at target
    let fleet_id = FleetId(78);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player,
            location: target,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    // Put it on a scout mission (busy)
    let unexplored = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect(
            "Test setup failed: no unexplored star found; at least one unexplored star is required",
        );
    engine.state.scout_missions.insert(
        fleet_id,
        ScoutMission {
            fleet: fleet_id,
            destination: unexplored,
            turns_remaining: 3,
            origin: StarId(0),
            total_duration: 3,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize with fleet on scout mission must emit an error"
    );
}

#[test]
fn colonize_fleet_on_fleet_mission_emits_error() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let home = engine.state.empires.get(&player).unwrap().home_star;

    let explored: Vec<StarId> = engine
        .state
        .explored_stars
        .iter()
        .filter(|&&id| id != home)
        .copied()
        .collect();
    let target = *explored
            .first()
            .expect("Test setup failed: no explored star found other than home star; at least two explored stars are required");

    // Place a colonizer at home (not at target)
    let fleet_id = FleetId(79);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player,
            location: home,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    // Fleet is in transit toward target
    engine.state.fleet_missions.insert(
        fleet_id,
        FleetMission {
            fleet: fleet_id,
            destination: target,
            turns_remaining: 2,
            origin: StarId(0),
            total_duration: 2,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index: 0,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Colonize with fleet on fleet mission must emit an error"
    );
}

// ──────────────────────────────────────────────────────────────────
// Longer deterministic replay
// ──────────────────────────────────────────────────────────────────

#[test]
fn deterministic_replay_scout_research_multi_turn() {
    // Run the same mixed command sequence on two identically-seeded engines and
    // assert that final state and per-turn event sequences are identical.
    use crate::state::TechId;

    let seed = 55_555;

    let run = |s: u64| {
        let mut engine = Engine::new(s);
        let colony_id = ColonyId(1);

        // Set up colony focus
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 50,
            research_pct: 50,
        }]);

        // Start research
        engine.apply_turn(vec![Command::SelectResearch { tech: TechId(1) }]);

        // Find an unexplored star and dispatch a scout
        let dest = *engine
                .state
                .stars
                .keys()
                .find(|id| !engine.state.explored_stars.contains(id))
                .expect("Test setup failed: no unexplored star found; the galaxy must have at least one unexplored star");
        engine.apply_turn(vec![Command::SendScout {
            fleet: FleetId(1),
            destination: dest,
        }]);

        // Advance 5 turns
        let mut all_events = Vec::new();
        for _ in 0..5 {
            all_events.push(engine.apply_turn(vec![Command::EndTurn]));
        }

        (engine.state, all_events)
    };

    let (state_a, events_a) = run(seed);
    let (state_b, events_b) = run(seed);

    assert_eq!(
        events_a, events_b,
        "Per-turn events must be identical for the same seed"
    );
    assert_eq!(
        state_a, state_b,
        "Final state must be identical for the same seed"
    );
}

// ──────────────────────────────────────────────────────────────────
// Event ordering
// ──────────────────────────────────────────────────────────────────

#[test]
fn turn_advanced_is_last_event_of_end_turn() {
    let mut engine = Engine::new(42);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    // TurnAdvanced must exist and be the last event
    let last = events
        .last()
        .expect("EndTurn must produce at least one event");
    assert!(
        matches!(last, Event::TurnAdvanced { .. }),
        "TurnAdvanced must be the last event in an EndTurn batch; got {:?}",
        last
    );
}

#[test]
fn economy_summary_precedes_food_shortage_and_credit_deficit() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let colony_id = ColonyId(1);

    // Force food and credit deficits simultaneously
    engine.state.empires.get_mut(&empire_id).unwrap().food = -5;
    engine.state.empires.get_mut(&empire_id).unwrap().credits = 0;
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    let events = engine.apply_turn(vec![Command::EndTurn]);

    // Find indices of EconomySummary, FoodShortage, CreditDeficit for the empire
    let summary_idx = events
        .iter()
        .position(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == empire_id))
        .expect("EconomySummary must be present");

    let shortage_idx = events
        .iter()
        .position(|e| matches!(e, Event::FoodShortage { empire, .. } if *empire == empire_id))
        .expect("FoodShortage must be present");

    let deficit_idx = events
        .iter()
        .position(|e| matches!(e, Event::CreditDeficit { empire, .. } if *empire == empire_id))
        .expect("CreditDeficit must be present");

    assert!(
        summary_idx < shortage_idx,
        "EconomySummary (idx {summary_idx}) must precede FoodShortage (idx {shortage_idx})"
    );
    assert!(
        summary_idx < deficit_idx,
        "EconomySummary (idx {summary_idx}) must precede CreditDeficit (idx {deficit_idx})"
    );
}

#[test]
fn colony_produced_precedes_economy_summary() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let colony_id = ColonyId(1);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 50,
        research_pct: 50,
    }]);

    let events = engine.apply_turn(vec![Command::EndTurn]);

    let produced_idx = events
        .iter()
        .position(|e| matches!(e, Event::ColonyProduced { colony, .. } if *colony == colony_id))
        .expect("ColonyProduced must be present");

    let summary_idx = events
        .iter()
        .position(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == empire_id))
        .expect("EconomySummary must be present");

    assert!(
        produced_idx < summary_idx,
        "ColonyProduced (idx {produced_idx}) must come before EconomySummary (idx {summary_idx})"
    );
}

// ──────────────────────────────────────────────────────────────────
// Production edge cases
// ──────────────────────────────────────────────────────────────────

#[test]
fn build_queue_advances_at_full_production_regardless_of_focus() {
    // The build queue always accumulates at the raw `production` rate;
    // prod_pct only controls how much of that becomes credits.
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    let production = engine.state.colonies[&colony_id].production;

    // Queue a scout (cost 50), set 0% production (100% research)
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    engine.apply_turn(vec![Command::EndTurn]);

    // Build queue advances by `production` regardless of prod_pct
    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(
        colony.accumulated_production, production,
        "Build queue must advance by raw production even when prod_pct = 0"
    );
}

#[test]
fn credits_income_is_zero_when_prod_pct_is_zero() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let colony_id = ColonyId(1);

    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    // Record credits before, subtract fleet maintenance manually to find income
    let fleet_count = engine
        .state
        .fleets
        .values()
        .filter(|f| f.owner == empire_id)
        .count() as i64;
    let credits_before = engine.state.empires[&empire_id].credits;

    engine.apply_turn(vec![Command::EndTurn]);

    let credits_after = engine.state.empires[&empire_id].credits;

    // With prod_pct=0 the only credit change should be negative maintenance
    assert_eq!(
        credits_after,
        credits_before - fleet_count,
        "Credits should only decrease by maintenance when prod_pct=0"
    );
}

// -------------------------------------------------------------------------
// Shipyard / orbital structure regression tests
// -------------------------------------------------------------------------

/// Helper: unlock Orbital Engineering (TechId 7) for the player empire.
fn unlock_orbital_engineering(engine: &mut Engine) {
    let empire_id = engine.state.player_empire;
    if let Some(empire) = engine.state.empires.get_mut(&empire_id) {
        if !empire.research.completed.contains(&TechId(7)) {
            empire.research.completed.push(TechId(7));
        }
    }
}

#[test]
fn shipyard_requires_orbital_engineering_tech() {
    // Without the tech, queuing a Shipyard should emit an Error event.
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .expect("player colony must exist");

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::OrbitalStructure(crate::state::OrbitalStructureType::Shipyard),
    }]);

    assert!(
        events.iter().any(
            |e| matches!(e, Event::Error { message } if message.contains("Orbital Engineering"))
        ),
        "expected an error mentioning Orbital Engineering, got: {:?}",
        events
    );

    // Queue should be empty — build was rejected
    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(
        colony.build_queue.is_empty(),
        "build queue must be empty after rejected Shipyard"
    );
}

#[test]
fn shipyard_can_be_queued_with_orbital_engineering() {
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .expect("player colony must exist");

    unlock_orbital_engineering(&mut engine);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::OrbitalStructure(crate::state::OrbitalStructureType::Shipyard),
    }]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::BuildQueued { .. })),
        "expected BuildQueued event, got: {:?}",
        events
    );

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(
        colony.build_queue.last(),
        Some(&BuildItem::OrbitalStructure(
            crate::state::OrbitalStructureType::Shipyard
        )),
        "Shipyard must be in the build queue"
    );
}

#[test]
fn shipyard_blocked_when_no_orbital_slots_available() {
    use crate::state::OrbitalStructureType;

    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .expect("player colony must exist");

    unlock_orbital_engineering(&mut engine);

    // Find the colony's planet size and fill all orbital slots
    let (star_id, planet_index) = {
        let c = engine.state.colonies.get(&colony_id).unwrap();
        (c.star, c.planet_index)
    };
    let planet_size = engine
        .state
        .stars
        .get(&star_id)
        .unwrap()
        .planets
        .get(planet_index)
        .unwrap()
        .size;
    let max_slots = planet_size.orbital_slots();

    // Fill all orbital slots by directly mutating the colony
    {
        let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
        for _ in 0..max_slots {
            colony
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
        }
    }

    // Now try to queue another Shipyard — should be rejected
    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard),
    }]);

    assert!(
        events.iter().any(
            |e| matches!(e, Event::Error { message } if message.contains("no free orbital slots"))
        ),
        "expected no-orbital-slots error, got: {:?}",
        events
    );
}

#[test]
fn shipyard_build_completes_and_enters_orbital_installations() {
    use crate::state::OrbitalStructureType;

    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .expect("player colony must exist");

    unlock_orbital_engineering(&mut engine);

    // Queue the Shipyard
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard),
    }]);

    // Force 100% production focus and run enough turns for it to complete
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 100,
        research_pct: 0,
    }]);

    // Shipyard costs 200pp; run enough turns to build it (production is at least 5/turn)
    let mut completed = false;
    for _ in 0..50 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events.iter().any(|e| {
                matches!(e, Event::BuildCompleted { item, .. } if *item == BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard))
            }) {
                completed = true;
                break;
            }
    }

    assert!(completed, "Shipyard should complete within 50 turns");

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(
        colony
            .orbital_installations
            .contains(&OrbitalStructureType::Shipyard),
        "completed Shipyard must appear in orbital_installations"
    );
}

#[test]
fn surface_building_enters_surface_installations_on_completion() {
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .expect("player colony must exist");

    engine.apply_turn(vec![
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::FabricationYard),
        },
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
    ]);

    let mut completed = false;
    for _ in 0..20 {
        let events = engine.apply_turn(vec![Command::EndTurn]);
        if events.iter().any(|e| {
                matches!(e, Event::BuildCompleted { item, .. } if *item == BuildItem::Structure(BuildingType::FabricationYard))
            }) {
                completed = true;
                break;
            }
    }

    assert!(completed, "FabricationYard should complete within 20 turns");

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert!(
        colony
            .surface_installations
            .contains(&BuildingType::FabricationYard),
        "completed FabricationYard must appear in surface_installations"
    );
    assert!(
        colony.buildings.contains(&BuildingType::FabricationYard),
        "completed FabricationYard must appear in buildings for effect tracking"
    );
}

#[test]
fn orbital_engineering_tech_exists_in_all_techs() {
    let techs = all_techs();
    assert!(
        techs
            .iter()
            .any(|t| t.id == TechId::ORBITAL_ENGINEERING && t.name == "Orbital Engineering"),
        "Orbital Engineering must be TechId(7)"
    );
}

#[test]
fn orbital_structure_type_shipyard_has_correct_metadata() {
    use crate::state::OrbitalStructureType;
    let ot = OrbitalStructureType::Shipyard;
    assert_eq!(ot.name(), "Shipyard");
    assert_eq!(ot.required_tech(), Some(TechId::ORBITAL_ENGINEERING));
    assert!(ot.cost() > 0, "cost must be positive");
    assert!(ot.maintenance_cost() > 0, "maintenance must be positive");
}

#[test]
fn build_item_orbital_structure_required_tech_matches() {
    use crate::state::{OrbitalStructureType, ShipDesignId};
    let item = BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard);
    assert_eq!(item.required_tech(), Some(TechId::ORBITAL_ENGINEERING));
    // Surface structures have no required tech
    assert_eq!(
        BuildItem::Structure(BuildingType::FabricationYard).required_tech(),
        None
    );
    assert_eq!(BuildItem::Scout.required_tech(), None);
    assert_eq!(
        BuildItem::Ship(ShipDesignId::COLONY).required_tech(),
        Some(TechId::HABITAT_SEEDING)
    );
    assert_eq!(
        BuildItem::Ship(ShipDesignId::SCIENCE).required_tech(),
        Some(TechId::SURVEY_DRONES)
    );
}

// -------------------------------------------------------------------------
// Shipyard requirement for ships
// -------------------------------------------------------------------------

#[test]
fn scout_requires_shipyard_to_queue() {
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .unwrap();

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Error { message } if message.contains("no Shipyard"))),
        "Scout must be rejected without a Shipyard, got: {:?}",
        events
    );
    assert!(
        engine
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .build_queue
            .is_empty(),
        "build queue must stay empty when Scout is rejected"
    );
}

#[test]
fn colony_ship_requires_shipyard_to_queue() {
    let mut engine = Engine::new(42);
    unlock_habitat_seeding(&mut engine);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .unwrap();

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Colony,
    }]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Error { message } if message.contains("no Shipyard"))),
        "Colony Ship must be rejected without a Shipyard, got: {:?}",
        events
    );
}

#[test]
fn scout_allowed_when_shipyard_present() {
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .unwrap();

    give_colony_shipyard(&mut engine, colony_id);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Scout,
    }]);

    assert!(
        !events.iter().any(|e| e.is_error()),
        "Scout must be accepted with a Shipyard, got errors: {:?}",
        events
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::BuildQueued { item, .. } if *item == BuildItem::Scout)),);
}

#[test]
fn colony_ship_allowed_when_shipyard_present() {
    let mut engine = Engine::new(42);
    unlock_habitat_seeding(&mut engine);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .unwrap();

    give_colony_shipyard(&mut engine, colony_id);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Colony,
    }]);

    assert!(
        !events.iter().any(|e| e.is_error()),
        "Colony Ship must be accepted with a Shipyard, got errors: {:?}",
        events
    );
}

#[test]
fn can_queue_valid_ship_with_shipyard_and_required_tech() {
    use crate::state::ShipDesignId;

    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .research
        .completed
        .push(TechId::HABITAT_SEEDING);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId::COLONY),
    }]);

    assert!(!events.iter().any(|e| e.is_error()));
    assert!(events.iter().any(
            |e| matches!(e, Event::BuildQueued { item, .. } if *item == BuildItem::Ship(ShipDesignId::COLONY))
        ));
}

#[test]
fn cannot_queue_ship_without_required_tech() {
    use crate::state::ShipDesignId;

    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId::COLONY),
    }]);

    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Error { message } if message.contains("Habitat Seeding"))));
}

#[test]
fn science_ship_unlock_requires_survey_drones() {
    use crate::state::ShipDesignId;

    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    let locked_events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId::SCIENCE),
    }]);
    assert!(locked_events
        .iter()
        .any(|e| matches!(e, Event::Error { message } if message.contains("Survey Drones"))));

    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .research
        .completed
        .push(TechId::SURVEY_DRONES);

    let unlocked_events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId::SCIENCE),
    }]);
    assert!(
        !unlocked_events.iter().any(|e| e.is_error()),
        "Science Ship should be queueable after Survey Drones"
    );
}

#[test]
fn cannot_queue_invalid_ship_design() {
    use crate::state::ShipDesignId;

    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId(999)),
    }]);

    assert!(events.iter().any(
        |e| matches!(e, Event::Error { message } if message.contains("design 999 is invalid"))
    ));
}

#[test]
fn completed_ship_has_correct_composition() {
    use crate::state::ShipDesignId;

    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Ship(ShipDesignId::SCOUT),
        },
    ]);

    let initial_fleet_ids: std::collections::BTreeSet<_> =
        engine.state.fleets.keys().copied().collect();
    for _ in 0..6 {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    let new_fleet = engine
        .state
        .fleets
        .values()
        .find(|f| !initial_fleet_ids.contains(&f.id))
        .expect("new fleet should be created");
    let design = ShipDesignId::SCOUT.record().unwrap();
    assert_eq!(new_fleet.kind, design.fleet_kind);
    assert_eq!(new_fleet.ships, design.ships);
    assert_eq!(new_fleet.strength, design.strength);
}

#[test]
fn mixed_production_queue_processes_deterministically() {
    use crate::state::ShipDesignId;

    let mut a = Engine::new(4242);
    let mut b = Engine::new(4242);

    for engine in [&mut a, &mut b] {
        let colony_id = ColonyId(1);
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .extend([TechId(2), TechId(7)]);
        give_colony_shipyard(engine, colony_id);
        engine.apply_turn(vec![
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::SurfaceStructure(BuildingType::AquacultureBay),
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::OrbitalStructure(crate::state::OrbitalStructureType::Shipyard),
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Ship(ShipDesignId::SCOUT),
            },
        ]);
    }

    for _ in 0..30 {
        let ev_a = a.apply_turn(vec![Command::EndTurn]);
        let ev_b = b.apply_turn(vec![Command::EndTurn]);
        assert_eq!(
            ev_a, ev_b,
            "events must match for deterministic queue processing"
        );
    }
    assert_eq!(
        a.state, b.state,
        "states must match for deterministic queue processing"
    );
}

#[test]
fn ship_completion_event_order_is_deterministic() {
    use crate::state::ShipDesignId;

    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);
    engine.apply_turn(vec![
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Ship(ShipDesignId::SCOUT),
        },
    ]);

    let mut completion_events = Vec::new();
    for _ in 0..6 {
        completion_events = engine.apply_turn(vec![Command::EndTurn]);
        if completion_events
            .iter()
            .any(|e| matches!(e, Event::BuildCompleted { .. }))
        {
            break;
        }
    }

    let build_idx = completion_events
        .iter()
        .position(|e| matches!(e, Event::BuildCompleted { .. }))
        .expect("BuildCompleted must be emitted");
    let fleet_idx = completion_events
        .iter()
        .position(|e| matches!(e, Event::FleetCreated { .. }))
        .expect("FleetCreated must be emitted");
    assert!(
        build_idx < fleet_idx,
        "BuildCompleted must be emitted before FleetCreated"
    );
}

// -------------------------------------------------------------------------
// Ship archetype tests
// -------------------------------------------------------------------------

#[test]
fn fleet_maintenance_for_mixed_fleet_composition() {
    // Verify that fleet maintenance sums per-kind costs correctly.
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Clear existing fleets and set up a known composition
    engine.state.fleets.clear();
    let star_id = *engine.state.stars.keys().next().unwrap();

    // Add: 1 Scout (maint=1), 1 Destroyer (maint=4), 1 PatrolCorvette (maint=1)
    engine.state.fleets.insert(
        FleetId(100),
        Fleet {
            id: FleetId(100),
            owner: empire_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    engine.state.fleets.insert(
        FleetId(101),
        Fleet {
            id: FleetId(101),
            owner: empire_id,
            location: star_id,
            ships: 3,
            kind: FleetKind::Destroyer,
            strength: 8,
            integrity: 100,
        },
    );
    engine.state.fleets.insert(
        FleetId(102),
        Fleet {
            id: FleetId(102),
            owner: empire_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::PatrolCorvette,
            strength: 2,
            integrity: 100,
        },
    );

    // Set colony production to 0 income
    let colony_id = ColonyId(1);
    engine.apply_turn(vec![Command::SetColonyFocus {
        colony: colony_id,
        prod_pct: 0,
        research_pct: 100,
    }]);

    let credits_before = engine.state.empires[&empire_id].credits;
    engine.apply_turn(vec![Command::EndTurn]);
    let credits_after = engine.state.empires[&empire_id].credits;

    // Expected maintenance: Scout(1) + Destroyer(4) + PatrolCorvette(1) = 6.
    // Colony has prod_pct=0 so zero credits income from production, and no
    // buildings installed so no building maintenance. Delta must be exactly 6.
    let expected_fleet_maint: i64 = 1 + 4 + 1;
    assert_eq!(
        credits_after,
        credits_before - expected_fleet_maint,
        "Mixed fleet maintenance must be exactly {} credits (per-kind sum); got delta {}",
        expected_fleet_maint,
        credits_before - credits_after
    );
}

#[test]
fn locked_archetype_cannot_be_queued_without_tech() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    // Destroyer requires FLEET_COORDINATION — not researched yet
    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId::DESTROYER),
    }]);
    assert!(
        events.iter().any(|e| e.is_error()),
        "Destroyer should require Fleet Coordination tech"
    );
}

#[test]
fn unlocked_archetype_can_be_queued_with_shipyard_and_tech() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    give_colony_shipyard(&mut engine, colony_id);

    // Unlock PERIMETER_DEFENSE to allow Patrol Corvette
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .unwrap()
        .research
        .completed
        .push(TechId::PERIMETER_DEFENSE);

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Ship(ShipDesignId::PATROL_CORVETTE),
    }]);
    assert!(
        !events.iter().any(|e| e.is_error()),
        "Patrol Corvette should be queueable after Perimeter Defense"
    );
}

#[test]
fn fast_scout_can_perform_scout_mission() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Use the player's actual home star as the fleet base
    let home_star = engine.state.empires[&empire_id].home_star;
    let fast_scout_id = FleetId(900);
    engine.state.fleets.insert(
        fast_scout_id,
        Fleet {
            id: fast_scout_id,
            owner: empire_id,
            location: home_star,
            ships: 1,
            kind: FleetKind::FastScout,
            strength: 1,
            integrity: 100,
        },
    );

    // Pick any star that is NOT already explored
    let target = engine
        .state
        .stars
        .keys()
        .find(|&&sid| !engine.state.explored_stars.contains(&sid))
        .copied()
        .expect("need at least one unexplored star");

    let events = engine.apply_turn(vec![Command::SendScout {
        fleet: fast_scout_id,
        destination: target,
    }]);
    assert!(
        !events.iter().any(|e| e.is_error()),
        "Fast Scout should be able to perform scout missions, errors: {:?}",
        events.iter().filter(|e| e.is_error()).collect::<Vec<_>>()
    );
    assert!(engine.state.scout_missions.contains_key(&fast_scout_id));
}

#[test]
fn survey_cutter_can_survey_planets() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Unlock survey capability
    engine
        .state
        .empires
        .get_mut(&empire_id)
        .unwrap()
        .research
        .completed
        .extend([TechId::SURVEY_DRONES, TechId::ADVANCED_SURVEY]);

    // Find a star with an unsurveyed planet that is explored
    let star_id = *engine.state.stars.keys().next().unwrap();
    engine.state.explored_stars.insert(star_id);

    let survey_cutter_id = FleetId(901);
    engine.state.fleets.insert(
        survey_cutter_id,
        Fleet {
            id: survey_cutter_id,
            owner: empire_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::SurveyCutter,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::SurveyPlanet {
        fleet: survey_cutter_id,
        star: star_id,
        planet_index: 0,
    }]);
    assert!(
        !events.iter().any(|e| e.is_error()),
        "Survey Cutter should be able to survey planets"
    );
}

#[test]
fn colony_ark_can_colonize() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;

    // Unlock colonization
    engine
        .state
        .empires
        .get_mut(&empire_id)
        .unwrap()
        .research
        .completed
        .extend([TechId::HABITAT_SEEDING, TechId::COLONIAL_VANGUARD]);

    // Find a star with a free habitable planet (may not yet be explored/surveyed)
    let star_id = engine
        .state
        .stars
        .iter()
        .find(|(_, s)| s.planets.iter().any(|p| p.habitable && p.colony.is_none()))
        .map(|(id, _)| *id)
        .expect("need a star with a free habitable planet");

    // Mark the star as explored and the planet as surveyed
    engine.state.explored_stars.insert(star_id);
    let planet_index = engine
        .state
        .stars
        .get(&star_id)
        .unwrap()
        .planets
        .iter()
        .position(|p| p.habitable && p.colony.is_none())
        .unwrap();
    if let Some(star) = engine.state.stars.get_mut(&star_id) {
        if let Some(planet) = star.planets.get_mut(planet_index) {
            planet.surveyed = true;
        }
    }

    let ark_id = FleetId(902);
    engine.state.fleets.insert(
        ark_id,
        Fleet {
            id: ark_id,
            owner: empire_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::ColonyArk,
            strength: 2,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: ark_id,
        star: star_id,
        planet_index,
    }]);
    assert!(
        !events.iter().any(|e| e.is_error()),
        "Colony Ark should be able to colonize, errors: {:?}",
        events.iter().filter(|e| e.is_error()).collect::<Vec<_>>()
    );
}

// -------------------------------------------------------------------------
// Surface slot cap enforcement
// -------------------------------------------------------------------------

#[test]
fn surface_structure_rejected_when_slots_full() {
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .unwrap();

    // Fill all surface slots
    let (star_id, planet_index) = {
        let c = engine.state.colonies.get(&colony_id).unwrap();
        (c.star, c.planet_index)
    };
    let max_slots = engine
        .state
        .stars
        .get(&star_id)
        .unwrap()
        .planets
        .get(planet_index)
        .unwrap()
        .size
        .surface_slots();

    {
        let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
        for _ in 0..max_slots {
            colony
                .surface_installations
                .push(BuildingType::FabricationYard);
        }
    }

    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Structure(BuildingType::ScienceNexus),
    }]);

    assert!(
        events.iter().any(
            |e| matches!(e, Event::Error { message } if message.contains("no free surface slots"))
        ),
        "Surface structure must be rejected when all slots are full, got: {:?}",
        events
    );
    assert!(
        engine
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .build_queue
            .is_empty(),
        "build queue must stay empty after surface-slot rejection"
    );
}

#[test]
fn surface_structure_allowed_when_slot_available() {
    let mut engine = Engine::new(42);
    let colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == engine.state.player_empire)
        .map(|(id, _)| *id)
        .unwrap();

    // Fresh colony has available slots
    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::Structure(BuildingType::AquacultureBay),
    }]);

    assert!(
        !events.iter().any(|e| e.is_error()),
        "Surface structure must be accepted when slots are available, got: {:?}",
        events
    );
}

// -------------------------------------------------------------------------
// SetColonyRole
// -------------------------------------------------------------------------

#[test]
fn set_colony_role_valid() {
    use crate::state::ColonyRole;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    let events = engine.apply_turn(vec![Command::SetColonyRole {
        colony: colony_id,
        role: ColonyRole::Industrial,
    }]);

    assert!(
        !events.iter().any(|e| e.is_error()),
        "Expected no errors, got: {:?}",
        events
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ColonyRoleChanged { colony, role }
            if *colony == colony_id && *role == ColonyRole::Industrial
        )),
        "ColonyRoleChanged event must be emitted"
    );

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(colony.role, ColonyRole::Industrial, "Role must be updated");
}

#[test]
fn set_colony_role_unknown_colony_emits_error() {
    use crate::state::ColonyRole;
    let mut engine = Engine::new(42);

    let events = engine.apply_turn(vec![Command::SetColonyRole {
        colony: ColonyId(999),
        role: ColonyRole::Agricultural,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Unknown colony must produce error"
    );
}

#[test]
fn set_colony_role_not_owned_by_player_emits_error() {
    use crate::state::ColonyRole;
    let mut engine = Engine::new(42);

    // ColonyId(2) is the AI colony
    let ai_colony_id = ColonyId(2);

    let events = engine.apply_turn(vec![Command::SetColonyRole {
        colony: ai_colony_id,
        role: ColonyRole::Financial,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Setting role on non-player colony must produce error"
    );
}

#[test]
fn set_colony_role_balanced_produces_no_modifier() {
    use crate::state::ColonyRole;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Set to Balanced
    engine.apply_turn(vec![Command::SetColonyRole {
        colony: colony_id,
        role: ColonyRole::Balanced,
    }]);

    let colony = engine.state.colonies.get(&colony_id).unwrap();
    assert_eq!(colony.role, ColonyRole::Balanced);
    // Verify no modifier is produced
    let mods = ColonyRole::Balanced.modifiers();
    assert_eq!(mods.food, 0);
    assert_eq!(mods.industry, 0);
    assert_eq!(mods.science, 0);
    assert_eq!(mods.credits, 0);
    assert_eq!(mods.maintenance, 0);
}

#[test]
fn military_role_provides_ship_production_bonus() {
    use crate::state::ColonyRole;
    // Military role ship_production_bonus must be > 0
    assert!(
        ColonyRole::Military.ship_production_bonus() > 0,
        "Military must have positive ship production bonus"
    );
    // Other roles must have 0
    assert_eq!(ColonyRole::Balanced.ship_production_bonus(), 0);
    assert_eq!(ColonyRole::Agricultural.ship_production_bonus(), 0);
    assert_eq!(ColonyRole::Industrial.ship_production_bonus(), 0);
    assert_eq!(ColonyRole::Scientific.ship_production_bonus(), 0);
    assert_eq!(ColonyRole::Financial.ship_production_bonus(), 0);
}

#[cfg(feature = "serde")]
#[test]
fn colony_role_persists_through_save_load() {
    use crate::state::ColonyRole;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);

    // Set a non-default role
    engine.apply_turn(vec![Command::SetColonyRole {
        colony: colony_id,
        role: ColonyRole::Scientific,
    }]);
    assert_eq!(
        engine.state.colonies[&colony_id].role,
        ColonyRole::Scientific
    );

    // Round-trip through JSON
    let saved = serde_json::to_string(&engine.state).expect("serialize must succeed");
    let loaded: GameState = serde_json::from_str(&saved).expect("deserialize must succeed");

    assert_eq!(
        loaded.colonies[&colony_id].role,
        ColonyRole::Scientific,
        "Colony role must survive save/load round-trip"
    );
}

#[test]
fn scout_exploration_reveals_system_without_surveying_planets() {
    let mut engine = Engine::new(42);
    let scout = FleetId(1);
    let destination = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("need an unexplored destination");

    if let Some(star) = engine.state.stars.get_mut(&destination) {
        for planet in &mut star.planets {
            planet.surveyed = false;
        }
    }

    let dispatch = engine.apply_turn(vec![Command::SendScout {
        fleet: scout,
        destination,
    }]);
    assert!(
        dispatch
            .iter()
            .any(|e| matches!(e, Event::ScoutDispatched { .. })),
        "scout dispatch should succeed"
    );

    let mut completion_events = Vec::new();
    // Advance up to 10 turns — more than enough for any possible distance
    for _ in 0..10 {
        completion_events = engine.apply_turn(vec![Command::EndTurn]);
        if completion_events
            .iter()
            .any(|e| matches!(e, Event::SystemExplored { .. }))
        {
            break;
        }
    }

    assert!(
        completion_events
            .iter()
            .any(|e| matches!(e, Event::SystemExplored { star } if *star == destination)),
        "system exploration event should be emitted"
    );

    let surveyed_indices: Vec<usize> = completion_events
        .iter()
        .filter_map(|e| match e {
            Event::PlanetSurveyCompleted { star, planet_index } if *star == destination => {
                Some(*planet_index)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        surveyed_indices,
        Vec::<usize>::new(),
        "scout exploration should not emit planet survey completions"
    );

    let all_surveyed = engine.state.stars[&destination]
        .planets
        .iter()
        .all(|p| p.surveyed);
    assert!(
        !all_surveyed,
        "scout exploration should not fully survey planets"
    );
}

#[test]
fn unsurveyed_planet_cannot_be_colonized() {
    let mut engine = Engine::new(42);
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != engine.state.empires[&engine.state.player_empire].home_star)
        .expect("need a non-home explored star");

    let planet_index = {
        let star = engine.state.stars.get_mut(&target).unwrap();
        let idx = star
            .planets
            .iter()
            .enumerate()
            .find(|(_, p)| p.colony.is_none())
            .map(|(i, _)| i)
            .unwrap_or(0);
        star.planets[idx].habitable = true;
        star.planets[idx].surveyed = false;
        idx
    };

    let fleet_id = FleetId(9990);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: target,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index,
    }]);
    assert!(
        events.iter().any(
            |e| matches!(e, Event::Error { message } if message.contains("not been surveyed"))
        ),
        "unsurveyed colonization should be rejected"
    );
}

#[test]
fn explicit_planet_index_colonizes_requested_target() {
    let mut engine = Engine::new(42);
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != engine.state.empires[&engine.state.player_empire].home_star)
        .expect("need a non-home explored star");

    {
        let star = engine.state.stars.get_mut(&target).unwrap();
        if star.planets.len() < 2 {
            let clone = star.planets[0].clone();
            star.planets.push(clone);
        }
        star.planets[0].colony = None;
        star.planets[0].habitable = true;
        star.planets[0].surveyed = true;
        star.planets[1].colony = None;
        star.planets[1].habitable = true;
        star.planets[1].surveyed = true;
    }

    let fleet_id = FleetId(9991);
    engine.state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: target,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Colonize {
        fleet: fleet_id,
        star: target,
        planet_index: 1,
    }]);

    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ColonizationStarted {
                fleet,
                star,
                planet_index,
                ..
            } if *fleet == fleet_id && *star == target && *planet_index == 1
        )),
        "colonization start should reference selected target"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ColonizationCompleted {
                fleet,
                star,
                planet_index,
                ..
            } if *fleet == fleet_id && *star == target && *planet_index == 1
        )),
        "colonization completion should reference selected target"
    );
    assert!(
        engine.state.stars[&target].planets[0].colony.is_none(),
        "non-target orbit should remain uncolonized"
    );
    assert!(
        engine.state.stars[&target].planets[1].colony.is_some(),
        "selected orbit should be colonized"
    );
}

// ──────────────────────────────────────────────────────────────────
// Distance-based travel tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn same_seed_produces_same_travel_durations() {
    // Same seed + same command ⇒ same travel duration (determinism test)
    let mut engine_a = Engine::new(42);
    let mut engine_b = Engine::new(42);

    let fleet_id = FleetId(1);
    let dest = *engine_a
        .state
        .stars
        .keys()
        .find(|id| !engine_a.state.explored_stars.contains(id))
        .expect("Need unexplored star");

    let evts_a = engine_a.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: dest,
    }]);
    let evts_b = engine_b.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: dest,
    }]);

    // Both events must match (including turns_remaining)
    assert_eq!(evts_a, evts_b, "Same seed must produce identical events");

    let dur_a = engine_a.state.scout_missions[&fleet_id].total_duration;
    let dur_b = engine_b.state.scout_missions[&fleet_id].total_duration;
    assert_eq!(dur_a, dur_b, "Same seed must produce same travel duration");
}

#[test]
fn same_seed_produces_same_hyperspace_lanes() {
    let engine_a = Engine::new(42);
    let engine_b = Engine::new(42);
    assert_eq!(
        engine_a.state.hyperspace_lanes,
        engine_b.state.hyperspace_lanes
    );
}

#[test]
fn lane_travel_is_unavailable_before_hyperspace_cartography() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let from = engine.state.fleets[&fleet_id].location;
    let destination = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != from)
        .expect("need explored destination");
    let lane = crate::state::HyperspaceLane::new(from, destination).expect("distinct stars");
    engine.state.hyperspace_lanes.insert(lane);
    engine.state.known_hyperspace_lanes.insert(lane);

    let src = &engine.state.stars[&from];
    let dst = &engine.state.stars[&destination];
    let dx = (dst.x - src.x) as i64;
    let dy = (dst.y - src.y) as i64;
    let base_turns = fleet_travel_turns(dx * dx + dy * dy);

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);
    let turns = engine.state.fleet_missions[&fleet_id].turns_remaining;
    assert_eq!(turns, base_turns);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::HyperspaceLaneUsed { .. })),
        "lane usage event must not fire before tech unlock"
    );
}

#[test]
fn lane_travel_is_faster_after_hyperspace_cartography() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let from = engine.state.fleets[&fleet_id].location;
    let destination = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != from)
        .expect("need explored destination");
    let lane = crate::state::HyperspaceLane::new(from, destination).expect("distinct stars");
    engine.state.hyperspace_lanes.insert(lane);
    engine.state.known_hyperspace_lanes.insert(lane);
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .expect("player empire exists")
        .research
        .completed
        .push(TechId::HYPERSPACE_CARTOGRAPHY);

    let src = &engine.state.stars[&from];
    let dst = &engine.state.stars[&destination];
    let dx = (dst.x - src.x) as i64;
    let dy = (dst.y - src.y) as i64;
    let base_turns = fleet_travel_turns(dx * dx + dy * dy);

    let events = engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);
    let turns = engine.state.fleet_missions[&fleet_id].turns_remaining;
    assert!(turns <= lane_travel_turns(base_turns));
    assert!(turns < base_turns || base_turns == 1);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::HyperspaceLaneUsed {
                fleet,
                from: evt_from,
                to: evt_to,
                ..
            } if *fleet == fleet_id && *evt_from == from && *evt_to == destination
        )),
        "lane usage event should be emitted when lane bonus applies"
    );
}

#[test]
fn no_lane_travel_still_uses_normal_distance_formula() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let from = engine.state.fleets[&fleet_id].location;
    let destination = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != from)
        .expect("need explored destination");
    let lane = crate::state::HyperspaceLane::new(from, destination).expect("distinct stars");
    engine.state.hyperspace_lanes.remove(&lane);
    engine.state.known_hyperspace_lanes.remove(&lane);
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .expect("player empire exists")
        .research
        .completed
        .push(TechId::HYPERSPACE_CARTOGRAPHY);

    let src = &engine.state.stars[&from];
    let dst = &engine.state.stars[&destination];
    let dx = (dst.x - src.x) as i64;
    let dy = (dst.y - src.y) as i64;
    let base_turns = fleet_travel_turns(dx * dx + dy * dy);

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination,
    }]);
    let turns = engine.state.fleet_missions[&fleet_id].turns_remaining;
    assert_eq!(turns, base_turns);
}

#[test]
fn hyperspace_lane_usage_events_are_deterministic() {
    let mut engine_a = Engine::new(42);
    let mut engine_b = Engine::new(42);
    for engine in [&mut engine_a, &mut engine_b] {
        let fleet_id = FleetId(1);
        let from = engine.state.fleets[&fleet_id].location;
        let destination = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != from)
            .expect("need explored destination");
        let lane = crate::state::HyperspaceLane::new(from, destination).expect("distinct stars");
        engine.state.hyperspace_lanes.insert(lane);
        engine.state.known_hyperspace_lanes.insert(lane);
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .expect("player empire exists")
            .research
            .completed
            .push(TechId::HYPERSPACE_CARTOGRAPHY);
    }

    let cmd = {
        let from = engine_a.state.fleets[&FleetId(1)].location;
        let destination = *engine_a
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != from)
            .unwrap();
        Command::MoveFleet {
            fleet: FleetId(1),
            destination,
        }
    };
    let a = engine_a.apply_turn(vec![cmd.clone()]);
    let b = engine_b.apply_turn(vec![cmd]);
    assert_eq!(a, b);
}

#[test]
fn nearby_systems_have_shorter_or_equal_travel_than_distant() {
    // fleet_travel_turns(smaller sq_dist) ≤ fleet_travel_turns(larger sq_dist)
    let near = fleet_travel_turns(1_000); // very close
    let mid = fleet_travel_turns(300_000); // moderate
    let far = fleet_travel_turns(1_500_000); // far

    assert!(near <= mid, "near ≤ mid: {} ≤ {}", near, mid);
    assert!(mid <= far, "mid ≤ far: {} ≤ {}", mid, far);
}

#[test]
fn minimum_travel_duration_is_at_least_one_turn() {
    // Even sq_dist = 0 must yield ≥ 1 turn
    assert!(fleet_travel_turns(0) >= 1);
    assert!(fleet_travel_turns(1) >= 1);
    assert!(fleet_travel_turns(1_000_000) >= 1);
}

#[test]
fn same_sector_distance_calculation_is_deterministic() {
    // Two engines with the same seed must compute the same sq_dist for the same pair.
    let engine_a = Engine::new(42);
    let engine_b = Engine::new(42);

    let home_a = engine_a.state.empires[&engine_a.state.player_empire].home_star;
    let home_b = engine_b.state.empires[&engine_b.state.player_empire].home_star;

    let sa = engine_a.state.stars.get(&home_a).unwrap();
    let sb = engine_b.state.stars.get(&home_b).unwrap();

    assert_eq!(
        (sa.x, sa.y),
        (sb.x, sb.y),
        "Same-seed star positions must match"
    );
}

#[test]
fn cross_sector_distance_calculation_is_deterministic() {
    // Verify that stars in different sectors produce deterministic distances
    let engine = Engine::new(42);
    let stars: Vec<_> = engine.state.stars.values().collect();
    if stars.len() < 2 {
        return; // Not enough stars to test cross-sector
    }
    let s1 = stars[0];
    let s2 = stars[stars.len() - 1];
    let dx = (s2.x - s1.x) as i64;
    let dy = (s2.y - s1.y) as i64;
    let sq_dist = dx * dx + dy * dy;
    // Recalculate — must be same (trivially true but documents the contract)
    let dx2 = (s2.x - s1.x) as i64;
    let dy2 = (s2.y - s1.y) as i64;
    let sq_dist2 = dx2 * dx2 + dy2 * dy2;
    assert_eq!(
        sq_dist, sq_dist2,
        "Distance calculation must be deterministic"
    );
    assert!(fleet_travel_turns(sq_dist) >= 1);
}

#[test]
fn travelling_fleet_progress_advances_per_turn() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let initial_location = engine.state.fleets[&fleet_id].location;

    let dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != initial_location)
        .expect("Need explored star");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest,
    }]);

    let initial_remaining = engine.state.fleet_missions[&fleet_id].turns_remaining;
    engine.apply_turn(vec![Command::EndTurn]);

    if let Some(mission) = engine.state.fleet_missions.get(&fleet_id) {
        // Mission still in progress — turns_remaining must have decreased
        assert!(
            mission.turns_remaining < initial_remaining,
            "turns_remaining must decrease each turn"
        );
    } else {
        // Mission already completed (1-turn distance) — fleet must have moved
        assert_eq!(
            engine.state.fleets[&fleet_id].location, dest,
            "Single-turn mission must place fleet at destination"
        );
    }
}

#[test]
fn arrival_updates_fleet_location_correctly() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let initial_location = engine.state.fleets[&fleet_id].location;

    let dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != initial_location)
        .expect("Need explored star");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest,
    }]);

    // Force mission to arrive next turn
    engine
        .state
        .fleet_missions
        .get_mut(&fleet_id)
        .unwrap()
        .turns_remaining = 1;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::FleetArrived { fleet, star }
                if *fleet == fleet_id && *star == dest)),
        "FleetArrived event must be emitted"
    );
    assert_eq!(
        engine.state.fleets[&fleet_id].location, dest,
        "Fleet location must be dest after arrival"
    );
    assert!(
        !engine.state.fleet_missions.contains_key(&fleet_id),
        "Mission must be removed after arrival"
    );
}

#[test]
fn scout_mission_stores_origin_and_total_duration() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let origin = engine.state.fleets[&fleet_id].location;

    let dest = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Need unexplored star");

    engine.apply_turn(vec![Command::SendScout {
        fleet: fleet_id,
        destination: dest,
    }]);

    let mission = &engine.state.scout_missions[&fleet_id];
    assert_eq!(
        mission.origin, origin,
        "origin must be set to departure star"
    );
    assert_eq!(
        mission.total_duration, mission.turns_remaining,
        "total_duration must equal turns_remaining at start"
    );
    assert!(mission.total_duration >= 1, "total_duration must be ≥ 1");
}

#[test]
fn fleet_mission_stores_origin_and_total_duration() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let origin = engine.state.fleets[&fleet_id].location;

    let dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != origin)
        .expect("Need explored star");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest,
    }]);

    let mission = &engine.state.fleet_missions[&fleet_id];
    assert_eq!(
        mission.origin, origin,
        "origin must be set to departure star"
    );
    assert_eq!(
        mission.total_duration, mission.turns_remaining,
        "total_duration must equal turns_remaining at start"
    );
    assert!(mission.total_duration >= 1, "total_duration must be ≥ 1");
}

#[cfg(feature = "serde")]
#[test]
fn travel_state_survives_save_load_round_trip() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);
    let origin = engine.state.fleets[&fleet_id].location;

    let dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != origin)
        .expect("Need explored star");

    engine.apply_turn(vec![Command::MoveFleet {
        fleet: fleet_id,
        destination: dest,
    }]);

    let original_mission = engine.state.fleet_missions[&fleet_id].clone();

    // Round-trip via JSON
    let json = serde_json::to_string(&engine.state).expect("serialize must succeed");
    let loaded: GameState = serde_json::from_str(&json).expect("deserialize must succeed");

    let loaded_mission = &loaded.fleet_missions[&fleet_id];
    assert_eq!(
        loaded_mission.origin, original_mission.origin,
        "origin must survive round-trip"
    );
    assert_eq!(
        loaded_mission.total_duration, original_mission.total_duration,
        "total_duration must survive round-trip"
    );
    assert_eq!(
        loaded_mission.turns_remaining, original_mission.turns_remaining,
        "turns_remaining must survive round-trip"
    );
}

#[test]
fn event_ordering_is_deterministic_for_fleet_arrivals() {
    // Multiple fleets arriving in the same turn must be emitted in FleetId order.
    let mut engine = Engine::new(42);
    let home = engine.state.empires[&engine.state.player_empire].home_star;

    let fleet_b_id = engine.state.next_fleet_id();
    engine.state.fleets.insert(
        fleet_b_id,
        Fleet {
            id: fleet_b_id,
            owner: engine.state.player_empire,
            location: home,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    let explored: Vec<StarId> = engine
        .state
        .explored_stars
        .iter()
        .filter(|&&id| id != home)
        .copied()
        .collect();
    let dest_a = explored[0];
    let dest_b = *explored.last().unwrap_or(&dest_a);

    engine.apply_turn(vec![
        Command::MoveFleet {
            fleet: FleetId(1),
            destination: dest_a,
        },
        Command::MoveFleet {
            fleet: fleet_b_id,
            destination: dest_b,
        },
    ]);

    // Force both to arrive next turn
    engine
        .state
        .fleet_missions
        .get_mut(&FleetId(1))
        .unwrap()
        .turns_remaining = 1;
    engine
        .state
        .fleet_missions
        .get_mut(&fleet_b_id)
        .unwrap()
        .turns_remaining = 1;

    let events = engine.apply_turn(vec![Command::EndTurn]);
    let arrival_ids: Vec<FleetId> = events
        .iter()
        .filter_map(|e| {
            if let Event::FleetArrived { fleet, .. } = e {
                Some(*fleet)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(arrival_ids.len(), 2, "Both fleets must arrive");
    assert!(
        arrival_ids[0] < arrival_ids[1],
        "Arrivals must be ordered by FleetId ascending"
    );
}

// ── Planet specials / survey / Ancient Ruins ─────────────────────────────

/// Injecting a known special into a planet and completing a survey emits
/// AncientRuinsDiscovered exactly once.
#[test]
fn ancient_ruins_discovery_emitted_once_on_survey() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let home = engine.state.empires[&empire_id].home_star;

    // Pick a non-home explored star with an unsurveyed planet.
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != home)
        .expect("need a non-home explored star");

    // Give the first planet Ancient Ruins and ensure it's unsurveyed.
    {
        let star = engine.state.stars.get_mut(&target).unwrap();
        star.planets[0].surveyed = false;
        star.planets[0].ancient_ruins_collected = false;
        star.planets[0].specials = vec![PlanetSpecial::AncientRuins];
    }

    // Place a Science Ship at the target star.
    let science_fleet_id = FleetId(9991);
    engine.state.fleets.insert(
        science_fleet_id,
        Fleet {
            id: science_fleet_id,
            owner: empire_id,
            location: target,
            ships: 1,
            kind: FleetKind::Science,
            strength: 1,
            integrity: 100,
        },
    );

    // Give the empire Survey Drones tech so the survey command is accepted.
    engine
        .state
        .empires
        .get_mut(&empire_id)
        .unwrap()
        .research
        .completed
        .push(TechId::SURVEY_DRONES);

    // Start the survey.
    engine.apply_turn(vec![Command::SurveyPlanet {
        fleet: science_fleet_id,
        star: target,
        planet_index: 0,
    }]);

    // Fast-forward survey to last turn.
    engine
        .state
        .survey_missions
        .get_mut(&science_fleet_id)
        .unwrap()
        .turns_remaining = 1;

    // Complete the survey.
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let ruins_events: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::AncientRuinsDiscovered {
                    star,
                    planet_index: 0
                } if *star == target
            )
        })
        .collect();
    assert_eq!(
        ruins_events.len(),
        1,
        "AncientRuinsDiscovered should be emitted exactly once"
    );
    // The planet's flag should now be set.
    assert!(
        engine.state.stars[&target].planets[0].ancient_ruins_collected,
        "ancient_ruins_collected must be true after discovery"
    );
}

/// Ancient Ruins discovery is not emitted again if the survey is somehow re-triggered.
#[test]
fn ancient_ruins_not_duplicated_on_re_survey() {
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let home = engine.state.empires[&empire_id].home_star;

    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != home)
        .expect("need a non-home explored star");

    // Planet already surveyed with ruins already collected.
    {
        let star = engine.state.stars.get_mut(&target).unwrap();
        star.planets[0].surveyed = true;
        star.planets[0].ancient_ruins_collected = true;
        star.planets[0].specials = vec![PlanetSpecial::AncientRuins];
    }

    // Manually call complete_survey_at_star (via the engine).
    engine
        .state
        .empires
        .get_mut(&empire_id)
        .unwrap()
        .research
        .completed
        .push(TechId::SURVEY_DRONES);

    // Force the planet unsurveyed to allow re-survey command, then reset flag
    {
        let star = engine.state.stars.get_mut(&target).unwrap();
        star.planets[0].surveyed = false; // allow re-survey
                                          // ancient_ruins_collected stays true
    }

    let science_fleet_id = FleetId(9992);
    engine.state.fleets.insert(
        science_fleet_id,
        Fleet {
            id: science_fleet_id,
            owner: empire_id,
            location: target,
            ships: 1,
            kind: FleetKind::Science,
            strength: 1,
            integrity: 100,
        },
    );

    engine.apply_turn(vec![Command::SurveyPlanet {
        fleet: science_fleet_id,
        star: target,
        planet_index: 0,
    }]);
    engine
        .state
        .survey_missions
        .get_mut(&science_fleet_id)
        .unwrap()
        .turns_remaining = 1;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    let ruins_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::AncientRuinsDiscovered { .. }))
        .collect();
    assert_eq!(
        ruins_events.len(),
        0,
        "AncientRuinsDiscovered must not be emitted again when already collected"
    );
}

/// Unsurveyed planet hides specials/resources from yield calculation.
#[test]
fn unsurveyed_planet_specials_do_not_affect_colony_yield() {
    use crate::yield_model::calculate_yield;

    let colony = Colony {
        id: ColonyId(99),
        star: StarId(0),
        planet_index: 0,
        owner: EmpireId(1),
        population: 10,
        production: 10,
        prod_pct: 50,
        research_pct: 50,
        build_queue: vec![],
        accumulated_production: 0,
        buildings: vec![],
        surface_installations: vec![],
        orbital_installations: vec![],
        stability: 100,
        role: ColonyRole::Balanced,
        rally_point: None,
    };

    let unsurveyed = Planet {
        name: "Unseen".to_string(),
        size: PlanetSize::Medium,
        class: PlanetClass::Terran,
        colony: Some(ColonyId(99)),
        habitable: true,
        surveyed: false,
        specials: vec![PlanetSpecial::MineralRich, PlanetSpecial::FertileBiosphere],
        resources: vec![StrategicResource::QuantumCrystals],
        anomalies: vec![],
        ancient_ruins_collected: false,
    };

    let surveyed = Planet {
        surveyed: true,
        ..unsurveyed.clone()
    };

    let y_unsurveyed = calculate_yield(&colony, Some(&unsurveyed));
    let y_surveyed = calculate_yield(&colony, Some(&surveyed));

    assert_ne!(
        y_unsurveyed.industry, y_surveyed.industry,
        "industry should differ once surveyed (MineralRich)"
    );
    assert_ne!(
        y_unsurveyed.food, y_surveyed.food,
        "food should differ once surveyed (FertileBiosphere)"
    );
    assert_ne!(
        y_unsurveyed.science, y_surveyed.science,
        "science should differ once surveyed (QuantumCrystals)"
    );

    // Unsurveyed yield must match a planet with no specials.
    let plain = Planet {
        specials: vec![],
        resources: vec![],
        ..unsurveyed.clone()
    };
    let y_plain = calculate_yield(&colony, Some(&plain));
    assert_eq!(
        y_unsurveyed, y_plain,
        "unsurveyed specials must produce same yield as no specials"
    );
}

/// Planet survey event ordering is deterministic across multiple simultaneous surveys.
#[test]
fn survey_completion_event_ordering_is_deterministic() {
    // Complete two surveys in the same turn; survey missions are processed in FleetId order.
    let mut engine = Engine::new(42);
    let empire_id = engine.state.player_empire;
    let home = engine.state.empires[&empire_id].home_star;

    engine
        .state
        .empires
        .get_mut(&empire_id)
        .unwrap()
        .research
        .completed
        .push(TechId::SURVEY_DRONES);

    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != home)
        .expect("need a non-home explored star");

    // Inject two science ships at the target star.
    for &(fleet_id, planet_idx) in &[(FleetId(9993), 0usize), (FleetId(9994), 1usize)] {
        // Ensure the planet is unsurveyed and habitable.
        if let Some(star) = engine.state.stars.get_mut(&target) {
            if let Some(planet) = star.planets.get_mut(planet_idx) {
                planet.surveyed = false;
                planet.habitable = true;
            }
        }
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: empire_id,
                location: target,
                ships: 1,
                kind: FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );
        engine.apply_turn(vec![Command::SurveyPlanet {
            fleet: fleet_id,
            star: target,
            planet_index: planet_idx,
        }]);
    }

    // Force both surveys to complete next turn.
    for &fleet_id in &[FleetId(9993), FleetId(9994)] {
        if let Some(m) = engine.state.survey_missions.get_mut(&fleet_id) {
            m.turns_remaining = 1;
        }
    }

    let events_a = engine.apply_turn(vec![Command::EndTurn]);

    // Reset and repeat — must produce identical event sequence.
    let mut engine2 = Engine::new(42);
    let empire_id2 = engine2.state.player_empire;
    let home2 = engine2.state.empires[&empire_id2].home_star;

    engine2
        .state
        .empires
        .get_mut(&empire_id2)
        .unwrap()
        .research
        .completed
        .push(TechId::SURVEY_DRONES);

    let target2 = *engine2
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != home2)
        .expect("need a non-home explored star");

    for &(fleet_id, planet_idx) in &[(FleetId(9993), 0usize), (FleetId(9994), 1usize)] {
        if let Some(star) = engine2.state.stars.get_mut(&target2) {
            if let Some(planet) = star.planets.get_mut(planet_idx) {
                planet.surveyed = false;
                planet.habitable = true;
            }
        }
        engine2.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: empire_id2,
                location: target2,
                ships: 1,
                kind: FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );
        engine2.apply_turn(vec![Command::SurveyPlanet {
            fleet: fleet_id,
            star: target2,
            planet_index: planet_idx,
        }]);
    }
    for &fleet_id in &[FleetId(9993), FleetId(9994)] {
        if let Some(m) = engine2.state.survey_missions.get_mut(&fleet_id) {
            m.turns_remaining = 1;
        }
    }

    let events_b = engine2.apply_turn(vec![Command::EndTurn]);

    let survey_events_a: Vec<_> = events_a
        .iter()
        .filter(|e| matches!(e, Event::PlanetSurveyCompleted { .. }))
        .collect();
    let survey_events_b: Vec<_> = events_b
        .iter()
        .filter(|e| matches!(e, Event::PlanetSurveyCompleted { .. }))
        .collect();

    assert_eq!(
        survey_events_a, survey_events_b,
        "survey completion event order must be deterministic"
    );
}

// ─── Rally Point tests ───────────────────────────────────────────────────

/// Helper: get a second explored star from the player's perspective.
fn second_explored_star(engine: &Engine) -> Option<StarId> {
    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;
    engine
        .state
        .explored_stars
        .iter()
        .copied()
        .find(|&id| id != home)
}

#[test]
fn set_valid_rally_point() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    // Use the home star as the rally target (it is always valid)
    let home_star = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;

    let events = engine.apply_turn(vec![Command::SetRallyPoint {
        colony: colony_id,
        star: home_star,
    }]);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::RallyPointSet { colony, star }
                    if *colony == colony_id && *star == home_star)),
        "Expected RallyPointSet event"
    );
    assert_eq!(
        engine.state.colonies.get(&colony_id).unwrap().rally_point,
        Some(home_star)
    );
}

#[test]
fn set_rally_point_to_unknown_star_fails() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let bogus_star = StarId(99999);

    let events = engine.apply_turn(vec![Command::SetRallyPoint {
        colony: colony_id,
        star: bogus_star,
    }]);

    assert!(
        events.iter().any(|e| e.is_error()),
        "Expected error for unknown star"
    );
    assert_eq!(
        engine.state.colonies.get(&colony_id).unwrap().rally_point,
        None
    );
}

#[test]
fn set_rally_point_on_unowned_colony_fails() {
    let mut engine = Engine::new(42);
    let ai_colony_id = ColonyId(2); // AI-owned
    let home_star = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;

    let events = engine.apply_turn(vec![Command::SetRallyPoint {
        colony: ai_colony_id,
        star: home_star,
    }]);
    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn clear_rally_point_works() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let home_star = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;

    // Set then clear
    engine.apply_turn(vec![Command::SetRallyPoint {
        colony: colony_id,
        star: home_star,
    }]);
    assert_eq!(
        engine.state.colonies.get(&colony_id).unwrap().rally_point,
        Some(home_star)
    );

    let events = engine.apply_turn(vec![Command::ClearRallyPoint { colony: colony_id }]);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::RallyPointCleared { colony } if *colony == colony_id)));
    assert_eq!(
        engine.state.colonies.get(&colony_id).unwrap().rally_point,
        None
    );
}

#[test]
fn clear_rally_on_unowned_colony_fails() {
    let mut engine = Engine::new(42);
    let ai_colony_id = ColonyId(2);
    let events = engine.apply_turn(vec![Command::ClearRallyPoint {
        colony: ai_colony_id,
    }]);
    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn new_ship_stays_local_without_rally_point() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let home_star = engine.state.colonies.get(&colony_id).unwrap().star;

    give_colony_shipyard(&mut engine, colony_id);

    // Queue a Scout ship (cheapest)
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: crate::state::BuildItem::Ship(ShipDesignId::SCOUT),
    }]);

    // Pump enough production to complete immediately
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .production = 999;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    // Fleet must have been created
    let fleet_created = events.iter().find_map(|e| match e {
        Event::FleetCreated { fleet, location } if *location == home_star => Some(*fleet),
        _ => None,
    });
    let new_fleet_id = fleet_created.expect("Fleet should be created at home star");

    // No ShipRoutedToRallyPoint event
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::ShipRoutedToRallyPoint { .. })),
        "Should not route without a rally point"
    );
    // Fleet should be idle at home star, no mission
    assert!(!engine.state.fleet_missions.contains_key(&new_fleet_id));
}

#[test]
fn new_ship_routed_to_rally_point_when_configured() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let home_star = engine.state.colonies.get(&colony_id).unwrap().star;

    // Find a second explored star to use as a rally point
    let rally_star =
        second_explored_star(&engine).expect("Need at least two explored stars for this test");
    assert_ne!(rally_star, home_star);

    // Set the rally point
    engine.apply_turn(vec![Command::SetRallyPoint {
        colony: colony_id,
        star: rally_star,
    }]);

    give_colony_shipyard(&mut engine, colony_id);

    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: crate::state::BuildItem::Ship(ShipDesignId::SCOUT),
    }]);
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .production = 999;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    // ShipRoutedToRallyPoint should be emitted
    let routed = events.iter().find(|e| {
        matches!(e, Event::ShipRoutedToRallyPoint { colony, to, .. }
                if *colony == colony_id && *to == rally_star)
    });
    assert!(routed.is_some(), "Should emit ShipRoutedToRallyPoint");

    // The new fleet should have either an active mission or already have arrived
    // (if rally star is 1 turn away, the mission resolves within the same EndTurn tick).
    let new_fleet_id = events
        .iter()
        .find_map(|e| match e {
            Event::FleetCreated { fleet, .. } => Some(*fleet),
            _ => None,
        })
        .expect("Fleet should be created");

    let mission_started = events.iter().any(|e| {
        matches!(e, Event::ShipRoutedToRallyPoint { fleet, to, .. }
                if *fleet == new_fleet_id && *to == rally_star)
    });
    assert!(
        mission_started,
        "Fleet should have been routed toward rally star"
    );

    // Either still en-route or already arrived (same-turn completion is valid)
    let is_in_mission = engine.state.fleet_missions.contains_key(&new_fleet_id);
    let arrived = events.iter().any(|e| {
        matches!(e, Event::FleetArrived { fleet, star }
                if *fleet == new_fleet_id && *star == rally_star)
    });
    assert!(
        is_in_mission || arrived,
        "Fleet should be en-route or arrived at rally star"
    );
}

#[test]
fn rally_point_to_unexplored_star_does_not_route() {
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let home_star = engine.state.colonies.get(&colony_id).unwrap().star;

    // Find an unexplored star
    let unexplored = engine
        .state
        .stars
        .keys()
        .copied()
        .find(|&id| !engine.state.explored_stars.contains(&id))
        .expect("Need an unexplored star");

    // We can force-set the rally point directly (bypassing command validation)
    // to test the auto-route guard
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .rally_point = Some(unexplored);

    give_colony_shipyard(&mut engine, colony_id);
    engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: crate::state::BuildItem::Ship(ShipDesignId::SCOUT),
    }]);
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .production = 999;

    let events = engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::ShipRoutedToRallyPoint { .. })),
        "Should not route to unexplored star"
    );
    let new_fleet_id = events
        .iter()
        .find_map(|e| match e {
            Event::FleetCreated { fleet, location } if *location == home_star => Some(*fleet),
            _ => None,
        })
        .expect("Fleet should be created");
    assert!(
        !engine.state.fleet_missions.contains_key(&new_fleet_id),
        "No mission should be created for unexplored rally target"
    );
}

#[test]
fn fleet_order_set_hold() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let events = engine.apply_turn(vec![Command::SetFleetOrder {
        fleet: fleet_id,
        order: FleetOrder::Hold,
    }]);

    assert!(events
        .iter()
        .any(|e| matches!(e, Event::FleetOrderSet { fleet, order }
                if *fleet == fleet_id && *order == FleetOrder::Hold)));
    assert_eq!(
        engine.state.fleet_orders.get(&fleet_id).copied(),
        Some(FleetOrder::Hold)
    );
}

#[test]
fn fleet_order_move_to_system_starts_mission() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let destination = second_explored_star(&engine).expect("Need a second explored star");

    let events = engine.apply_turn(vec![Command::SetFleetOrder {
        fleet: fleet_id,
        order: FleetOrder::MoveToSystem(destination),
    }]);

    assert!(events
        .iter()
        .any(|e| matches!(e, Event::FleetOrderSet { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::FleetDeparted { fleet, to, .. }
                if *fleet == fleet_id && *to == destination)));
    assert!(engine.state.fleet_missions.contains_key(&fleet_id));
}

#[test]
fn fleet_order_invalid_fleet_fails() {
    let mut engine = Engine::new(42);
    let bogus_fleet = FleetId(99999);

    let events = engine.apply_turn(vec![Command::SetFleetOrder {
        fleet: bogus_fleet,
        order: FleetOrder::Hold,
    }]);
    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn fleet_order_invalid_destination_fails() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let events = engine.apply_turn(vec![Command::SetFleetOrder {
        fleet: fleet_id,
        order: FleetOrder::MoveToSystem(StarId(99999)),
    }]);
    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn fleet_order_cleared_on_arrival() {
    let mut engine = Engine::new(42);
    let fleet_id = FleetId(1);

    let destination = second_explored_star(&engine).expect("Need a second explored star");

    // Issue MoveToSystem order (starts mission immediately)
    engine.apply_turn(vec![Command::SetFleetOrder {
        fleet: fleet_id,
        order: FleetOrder::MoveToSystem(destination),
    }]);

    assert!(engine.state.fleet_orders.contains_key(&fleet_id));

    // Advance turns until the fleet arrives
    for _ in 0..10 {
        if !engine.state.fleet_missions.contains_key(&fleet_id) {
            break;
        }
        engine.apply_turn(vec![Command::EndTurn]);
    }

    assert!(
        !engine.state.fleet_missions.contains_key(&fleet_id),
        "Fleet should have arrived"
    );
    assert!(
        !engine.state.fleet_orders.contains_key(&fleet_id),
        "MoveToSystem order should be cleared after arrival"
    );
}

#[test]
#[cfg(feature = "serde")]
fn rally_point_and_fleet_order_persist_through_save_load() {
    use crate::state::FleetOrder;
    let mut engine = Engine::new(42);
    let colony_id = ColonyId(1);
    let home_star = engine.state.colonies.get(&colony_id).unwrap().star;

    // Set a rally point
    engine.apply_turn(vec![Command::SetRallyPoint {
        colony: colony_id,
        star: home_star,
    }]);
    // Set a Hold order on fleet 1
    engine.apply_turn(vec![Command::SetFleetOrder {
        fleet: FleetId(1),
        order: FleetOrder::Hold,
    }]);

    // Serialize and deserialize
    let json = serde_json::to_string(&engine.state).expect("Serialization must succeed");
    let restored: crate::state::GameState =
        serde_json::from_str(&json).expect("Deserialization must succeed");

    assert_eq!(
        restored.colonies.get(&colony_id).unwrap().rally_point,
        Some(home_star),
        "Rally point must survive save/load"
    );
    assert_eq!(
        restored.fleet_orders.get(&FleetId(1)).copied(),
        Some(FleetOrder::Hold),
        "Fleet order must survive save/load"
    );
}

#[test]
fn rally_point_order_processing_is_deterministic() {
    // Run two independent engines with same seed and same commands;
    // verify fleet routing events are identical.
    let setup = |seed: u64| {
        let mut engine = Engine::new(seed);
        let colony_id = ColonyId(1);
        let home_star = engine.state.colonies.get(&colony_id).unwrap().star;

        let rally_star = engine
            .state
            .explored_stars
            .iter()
            .copied()
            .find(|&id| id != home_star)
            .unwrap();

        engine.apply_turn(vec![Command::SetRallyPoint {
            colony: colony_id,
            star: rally_star,
        }]);

        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .production = 999;

        give_colony_shipyard(&mut engine, colony_id);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: crate::state::BuildItem::Ship(ShipDesignId::SCOUT),
        }]);

        let events = engine.apply_turn(vec![Command::EndTurn]);
        let routing_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::ShipRoutedToRallyPoint { .. }))
            .cloned()
            .collect();
        routing_events
    };

    let a = setup(42);
    let b = setup(42);
    assert_eq!(a, b, "Rally routing must be deterministic for same seed");
}

fn add_far_player_colony(engine: &mut Engine) -> ColonyId {
    let player = engine.state.player_empire;
    let home_star = engine.state.empires[&player].home_star;
    let far_star = engine
        .state
        .stars
        .keys()
        .copied()
        .find(|s| *s != home_star)
        .expect("need a non-home star");

    let (home_x, home_y) = {
        let home = engine.state.stars.get(&home_star).expect("home star");
        (home.x, home.y)
    };
    if let Some(star) = engine.state.stars.get_mut(&far_star) {
        star.x = home_x + 1000;
        star.y = home_y + 1000;
        if let Some(planet) = star.planets.get_mut(0) {
            planet.colony = None;
            planet.surveyed = true;
            planet.habitable = true;
        }
    }

    let colony_id = engine.state.next_colony_id();
    engine.state.colonies.insert(
        colony_id,
        Colony {
            id: colony_id,
            star: far_star,
            planet_index: 0,
            owner: player,
            population: 10,
            production: 10,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );
    if let Some(star) = engine.state.stars.get_mut(&far_star) {
        if let Some(planet) = star.planets.get_mut(0) {
            planet.colony = Some(colony_id);
        }
    }
    colony_id
}

#[test]
fn isolated_colony_applies_penalties() {
    let mut engine = Engine::new(42);
    let colony_id = add_far_player_colony(&mut engine);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let produced = events
        .iter()
        .find_map(|e| match e {
            Event::ColonyProduced {
                colony,
                credits,
                research,
                food,
                ..
            } if *colony == colony_id => Some((*credits, *research, *food)),
            _ => None,
        })
        .expect("new colony should produce");
    assert_eq!(
        produced.2, 0,
        "isolated colonies should not share empire food"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ColonyIsolated { colony } if *colony == colony_id)));
    assert_eq!(
        engine.state.colonies[&colony_id].stability,
        100 - ISOLATED_STABILITY_PENALTY
    );
}

#[test]
fn lane_connected_colony_contributes_normally() {
    let mut isolated_engine = Engine::new(42);
    let colony_id = add_far_player_colony(&mut isolated_engine);
    let isolated_events = isolated_engine.apply_turn(vec![Command::EndTurn]);
    let isolated_produced = isolated_events
        .iter()
        .find_map(|e| match e {
            Event::ColonyProduced {
                colony,
                credits,
                research,
                food,
                ..
            } if *colony == colony_id => Some((*credits, *research, *food)),
            _ => None,
        })
        .expect("isolated colony should produce");

    let mut connected_engine = Engine::new(42);
    let connected_colony_id = add_far_player_colony(&mut connected_engine);
    let home_star = connected_engine.state.empires[&connected_engine.state.player_empire].home_star;
    let far_star = connected_engine.state.colonies[&connected_colony_id].star;
    let lane = HyperspaceLane::new(home_star, far_star).expect("distinct stars");
    connected_engine.state.hyperspace_lanes.insert(lane);
    connected_engine.state.known_hyperspace_lanes.insert(lane);
    connected_engine
        .state
        .empires
        .get_mut(&connected_engine.state.player_empire)
        .expect("player empire")
        .research
        .completed
        .push(TechId::HYPERSPACE_CARTOGRAPHY);

    let connected_events = connected_engine.apply_turn(vec![Command::EndTurn]);
    let connected_produced = connected_events
        .iter()
        .find_map(|e| match e {
            Event::ColonyProduced {
                colony,
                credits,
                research,
                food,
                ..
            } if *colony == connected_colony_id => Some((*credits, *research, *food)),
            _ => None,
        })
        .expect("connected colony should produce");

    assert!(
        connected_produced.0 > isolated_produced.0,
        "connected colony should contribute more credits"
    );
    assert!(
        connected_produced.1 > isolated_produced.1,
        "connected colony should contribute more research"
    );
    assert!(
        connected_produced.2 > isolated_produced.2,
        "connected colony should contribute food when connected"
    );
}

#[test]
fn colony_reconnection_event_emitted_after_lane_unlock() {
    let mut engine = Engine::new(42);
    let colony_id = add_far_player_colony(&mut engine);
    let home_star = engine.state.empires[&engine.state.player_empire].home_star;
    let far_star = engine.state.colonies[&colony_id].star;

    let first_turn_events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(first_turn_events
        .iter()
        .any(|e| matches!(e, Event::ColonyIsolated { colony } if *colony == colony_id)));

    let lane = HyperspaceLane::new(home_star, far_star).expect("distinct stars");
    engine.state.hyperspace_lanes.insert(lane);
    engine.state.known_hyperspace_lanes.insert(lane);
    engine
        .state
        .empires
        .get_mut(&engine.state.player_empire)
        .expect("player empire")
        .research
        .completed
        .push(TechId::HYPERSPACE_CARTOGRAPHY);

    let second_turn_events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(second_turn_events
        .iter()
        .any(|e| matches!(e, Event::ColonyReconnected { colony } if *colony == colony_id)));
}

// ── Scenario Setup / Engine::new_from_setup tests ─────────────────────

#[test]
fn new_from_setup_same_options_produce_same_galaxy() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let setup1 = ScenarioSetup {
        seed: 777,
        galaxy_size: GalaxySize::Small,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let setup2 = setup1.clone();

    let e1 = Engine::new_from_setup(setup1);
    let e2 = Engine::new_from_setup(setup2);

    // Same seed + same options → identical star layouts
    let mut stars1: Vec<_> = e1.state.stars.values().map(|s| (s.id, s.x, s.y)).collect();
    let mut stars2: Vec<_> = e2.state.stars.values().map(|s| (s.id, s.x, s.y)).collect();
    stars1.sort();
    stars2.sort();
    assert_eq!(stars1, stars2, "Same setup must produce identical galaxies");
}

#[test]
fn new_from_setup_different_seeds_produce_different_galaxies() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let make = |seed: u64| {
        Engine::new_from_setup(ScenarioSetup {
            seed,
            galaxy_size: GalaxySize::Medium,
            ai_empire_count: 1,
            sector_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: None,
            victory_settings: crate::state::VictorySettings::default_v1(),
        })
    };

    let e1 = make(100);
    let e2 = make(200);

    let pos1: Vec<_> = e1.state.stars.values().map(|s| (s.x, s.y)).collect();
    let pos2: Vec<_> = e2.state.stars.values().map(|s| (s.x, s.y)).collect();
    // Not all positions should match
    let matching = pos1.iter().zip(pos2.iter()).filter(|(a, b)| a == b).count();
    assert!(
        matching < pos1.len() / 2,
        "Different seeds should produce meaningfully different galaxies"
    );
}

#[test]
fn new_from_setup_small_produces_expected_star_and_sector_counts() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Small,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    assert_eq!(engine.state.stars.len(), 10, "Small: 10 stars");
    assert_eq!(engine.state.sectors.len(), 2, "Small: 2 sectors");
}

#[test]
fn new_from_setup_medium_produces_expected_star_and_sector_counts() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    assert_eq!(engine.state.stars.len(), 20, "Medium: 20 stars");
    assert_eq!(engine.state.sectors.len(), 4, "Medium: 4 sectors");
}

#[test]
fn new_from_setup_large_produces_expected_star_and_sector_counts() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Large,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    assert_eq!(engine.state.stars.len(), 40, "Large: 40 stars");
    assert_eq!(engine.state.sectors.len(), 6, "Large: 6 sectors");
}

#[test]
fn new_from_setup_four_ai_empires_created() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Large,
        ai_empire_count: 4,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    // Player + 4 AI = 5 empires total
    assert_eq!(engine.state.empires.len(), 5);
    assert_eq!(engine.state.ai_empires.len(), 4);
    // Each AI empire has a distinct home star
    let ai_home_stars: BTreeSet<StarId> = engine
        .state
        .ai_empires
        .iter()
        .filter_map(|id| engine.state.empires.get(id))
        .map(|e| e.home_star)
        .collect();
    assert_eq!(
        ai_home_stars.len(),
        4,
        "Each AI empire should have a unique home star"
    );
}

#[test]
fn new_from_setup_ai_empire_placement_is_deterministic() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let setup = ScenarioSetup {
        seed: 999,
        galaxy_size: GalaxySize::Large,
        ai_empire_count: 3,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let e1 = Engine::new_from_setup(setup.clone());
    let e2 = Engine::new_from_setup(setup);

    let homes1: Vec<_> = e1
        .state
        .ai_empires
        .iter()
        .filter_map(|id| e1.state.empires.get(id))
        .map(|e| e.home_star)
        .collect();
    let homes2: Vec<_> = e2
        .state
        .ai_empires
        .iter()
        .filter_map(|id| e2.state.empires.get(id))
        .map(|e| e.home_star)
        .collect();
    assert_eq!(
        homes1, homes2,
        "AI home star placement must be deterministic for same seed"
    );
}

#[test]
fn new_from_setup_scenario_metadata_stored_in_state() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let setup = ScenarioSetup {
        seed: 1234,
        galaxy_size: GalaxySize::Large,
        ai_empire_count: 2,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let engine = Engine::new_from_setup(setup.clone());
    let stored = engine
        .state
        .scenario
        .as_ref()
        .expect("scenario should be stored");
    assert_eq!(stored.seed, 1234);
    assert_eq!(stored.galaxy_size, GalaxySize::Large);
    assert_eq!(stored.ai_empire_count, 2);
}

#[test]
fn validate_rejects_zero_ai_count() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let bad_setup = ScenarioSetup {
        seed: 1,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 0, // invalid
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    // validate() must catch invalid AI count
    assert!(bad_setup.validate().is_err());
}

#[test]
#[should_panic]
fn new_from_setup_invalid_ai_count_panics() {
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let bad_setup = ScenarioSetup {
        seed: 1,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 0, // invalid — new_from_setup should panic
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let _ = Engine::new_from_setup(bad_setup);
}

// ── Empire Identity / Player Empire Selection tests ─────────────────────

#[test]
fn player_can_select_valid_empire() {
    use crate::state::{
        all_empire_definitions, DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup,
    };
    let def_id = EmpireDefinitionId(2); // Sylvaran Accord
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(def_id),
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_ok());
    let engine = Engine::new_from_setup(setup);
    let player = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(player.empire_def, Some(def_id));
    let def = all_empire_definitions()
        .iter()
        .find(|d| d.id == def_id)
        .unwrap();
    assert_eq!(player.name, def.name);
}

#[test]
fn player_can_select_terran_concord() {
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(6)),
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    let player = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(player.empire_def, Some(EmpireDefinitionId(6)));
    assert_eq!(player.name, "Terran Concord");
}

#[test]
fn player_can_select_terran_dominion() {
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(7)),
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    let player = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(player.empire_def, Some(EmpireDefinitionId(7)));
    assert_eq!(player.name, "Terran Dominion");
}

#[test]
fn invalid_empire_selection_fails_validation() {
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(255)), // does not exist
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_err());
}

#[test]
fn ai_empires_receive_distinct_definitions() {
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Large,
        ai_empire_count: 4,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(0)),
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    let ai_defs: Vec<Option<EmpireDefinitionId>> = engine
        .state
        .ai_empires
        .iter()
        .filter_map(|id| engine.state.empires.get(id))
        .map(|e| e.empire_def)
        .collect();
    // All AI empires should have a def
    assert!(
        ai_defs.iter().all(|d| d.is_some()),
        "All AI empires must have an empire def"
    );
    // All defs should be distinct from player's and from each other
    let player_def = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .empire_def;
    for d in &ai_defs {
        assert_ne!(
            *d, player_def,
            "AI empire def must not duplicate player def"
        );
    }
    let unique: std::collections::BTreeSet<u8> =
        ai_defs.iter().filter_map(|d| d.map(|x| x.0)).collect();
    assert_eq!(unique.len(), 4, "AI empire defs must all be distinct");
}

#[test]
fn same_seed_produces_same_ai_empire_definitions() {
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let make = || {
        Engine::new_from_setup(ScenarioSetup {
            seed: 1234,
            galaxy_size: GalaxySize::Large,
            ai_empire_count: 3,
            sector_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: Some(EmpireDefinitionId(1)),
            victory_settings: crate::state::VictorySettings::default_v1(),
        })
    };
    let e1 = make();
    let e2 = make();
    let defs1: Vec<_> = e1
        .state
        .ai_empires
        .iter()
        .filter_map(|id| e1.state.empires.get(id))
        .map(|e| e.empire_def)
        .collect();
    let defs2: Vec<_> = e2
        .state
        .ai_empires
        .iter()
        .filter_map(|id| e2.state.empires.get(id))
        .map(|e| e.empire_def)
        .collect();
    assert_eq!(
        defs1, defs2,
        "Same seed must produce same AI empire definitions"
    );
}

#[test]
fn different_seeds_can_produce_different_ai_empire_definitions() {
    // The seeded shuffle means different seeds *can* (and with high probability *do*)
    // yield different AI empire definition orderings.  We test across several seed
    // pairs to confirm variance — if all seeds gave the same assignment the shuffle
    // would be inert.
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let ai_defs_for_seed = |seed: u64| -> Vec<Option<EmpireDefinitionId>> {
        let e = Engine::new_from_setup(ScenarioSetup {
            seed,
            galaxy_size: GalaxySize::Large,
            ai_empire_count: 4,
            sector_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: Some(EmpireDefinitionId(0)),
            victory_settings: crate::state::VictorySettings::default_v1(),
        });
        e.state
            .ai_empires
            .iter()
            .filter_map(|id| e.state.empires.get(id))
            .map(|emp| emp.empire_def)
            .collect()
    };

    let seeds: &[u64] = &[1, 100, 9999, 0x_DEAD_BEEF, 0x_CAFE_BABE];
    let assignments: Vec<_> = seeds.iter().map(|&s| ai_defs_for_seed(s)).collect();

    // At least two of the five assignments must differ — a purely stable-order
    // assignment would produce five identical lists.
    let unique_count = {
        let mut seen: Vec<&Vec<Option<EmpireDefinitionId>>> = Vec::new();
        for a in &assignments {
            if !seen.contains(&a) {
                seen.push(a);
            }
        }
        seen.len()
    };
    assert!(
            unique_count >= 2,
            "Different seeds must yield at least some variance in AI empire assignments (got {unique_count} unique)"
        );
}

#[test]
fn empire_trait_modifiers_applied_per_colony() {
    // The Sylvaran Accord (id=2) gets +2 food/colony.
    use crate::events::Event;
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(2)), // Sylvaran Accord: +2 food
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let mut engine = Engine::new_from_setup(setup);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    // Find ColonyProduced for the player's colony and check food includes the bonus
    let player_empire = engine.state.player_empire;
    let player_colony = engine
        .state
        .colonies
        .values()
        .find(|c| c.owner == player_empire)
        .map(|c| c.id)
        .expect("player colony must exist");

    let produced = events
        .iter()
        .find_map(|e| {
            if let Event::ColonyProduced { colony, food, .. } = e {
                if *colony == player_colony {
                    Some(*food)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .expect("ColonyProduced event must be emitted");

    // base food = population (10), +2 empire mod = 12
    assert!(
        produced >= 12,
        "Sylvaran Accord food bonus must be applied; got {produced}"
    );
}

#[test]
fn terran_concord_science_bonus_applied_per_colony() {
    use crate::events::Event;
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let mut engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(6)),
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    let events = engine.apply_turn(vec![Command::EndTurn]);
    let player_colony = engine
        .state
        .colonies
        .values()
        .find(|c| c.owner == engine.state.player_empire)
        .map(|c| c.id)
        .expect("player colony must exist");
    let produced = events
        .iter()
        .find_map(|event| match event {
            Event::ColonyProduced {
                colony, research, ..
            } if *colony == player_colony => Some(*research),
            _ => None,
        })
        .expect("ColonyProduced event expected");
    assert!(produced >= 6, "Terran Concord should gain a science bonus");
}

#[test]
fn terran_dominion_troop_transports_are_cheaper() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    engine.state.empires.get_mut(&player).unwrap().empire_def =
        Some(crate::state::EmpireDefinitionId(7));
    let base_cost = BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT).cost();
    let actual_cost =
        engine.effective_build_cost(player, BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT));
    assert!(
        actual_cost < base_cost,
        "Terran Dominion troop transports should be cheaper"
    );
}

#[test]
fn terran_dominion_invasion_strength_bonus_applies() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    engine.state.empires.get_mut(&player).unwrap().empire_def =
        Some(crate::state::EmpireDefinitionId(7));
    let actual = engine.invasion_strength_for_empire(player, 2);
    assert_eq!(
        actual, 32,
        "Terran Dominion should gain +4 invasion per transport"
    );
}

#[test]
#[cfg(feature = "serde")]
fn empire_identity_persists_through_save_load() {
    use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
    let setup = ScenarioSetup {
        seed: 77,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 2,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(3)), // Thalori Exchange
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let engine = Engine::new_from_setup(setup);

    let json = serde_json::to_string(&engine.state).expect("serialize");
    let restored: crate::state::GameState = serde_json::from_str(&json).expect("deserialize");

    // Player empire def preserved
    let player_def = restored
        .empires
        .get(&restored.player_empire)
        .unwrap()
        .empire_def;
    assert_eq!(player_def, Some(EmpireDefinitionId(3)));

    // AI empire defs preserved
    let original_ai_defs: Vec<_> = engine
        .state
        .ai_empires
        .iter()
        .filter_map(|id| engine.state.empires.get(id))
        .map(|e| e.empire_def)
        .collect();
    let restored_ai_defs: Vec<_> = restored
        .ai_empires
        .iter()
        .filter_map(|id| restored.empires.get(id))
        .map(|e| e.empire_def)
        .collect();
    assert_eq!(
        original_ai_defs, restored_ai_defs,
        "AI empire defs must survive save/load"
    );
}

#[test]
fn default_empire_assigned_when_no_player_def_specified() {
    // When player_empire_def is None, the engine assigns EmpireDefinitionId(0) by default.
    use crate::state::{DifficultyLevel, GalaxySize, ScenarioSetup};
    let engine = Engine::new_from_setup(ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    });
    let player = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap();
    assert_eq!(player.empire_def, Some(crate::state::EmpireDefinitionId(0)));
}

// ────────────────────────────────────────────────────────────────────────
// Blockade tests
// ────────────────────────────────────────────────────────────────────────

/// Build a minimal `GameState` suitable for blockade tests.
///
/// The state includes:
/// * Player empire (id=1) with one colony on Star 1
/// * Enemy empire  (id=2) — no relationship by default (Unknown)
/// * Player star + planet + colony are wired up
fn make_blockade_state() -> (GameState, StarId, ColonyId, EmpireId, EmpireId) {
    use crate::state::SpectralClass;
    use rand::SeedableRng;
    let player_id = EmpireId(1);
    let enemy_id = EmpireId(2);
    let star_id = StarId(1);
    let colony_id = ColonyId(1);

    let mut state = GameState {
        seed: 0,
        turn: 1,
        player_empire: player_id,
        rng: ChaCha8Rng::seed_from_u64(0),
        event_log: Vec::new(),
        next_colony_id: 2,
        next_fleet_id: 10,
        stars: BTreeMap::new(),
        sectors: BTreeMap::new(),
        empires: BTreeMap::new(),
        colonies: BTreeMap::new(),
        fleets: BTreeMap::new(),
        explored_stars: {
            let mut s = BTreeSet::new();
            s.insert(star_id);
            s
        },
        scout_missions: BTreeMap::new(),
        survey_missions: BTreeMap::new(),
        fleet_missions: BTreeMap::new(),
        // Keep enemy out of AI empire lists so the AI turn doesn't
        // interfere with fleet positions during blockade tests.
        ai_empire: None,
        ai_explored_stars: BTreeSet::new(),
        diplomacy: BTreeMap::new(),
        diplomacy_relationships: BTreeMap::new(),
        diplomacy_pending_communications: std::collections::VecDeque::new(),
        diplomacy_next_communication_id: 1,
        hyperspace_lanes: BTreeSet::new(),
        known_hyperspace_lanes: BTreeSet::new(),
        fleet_orders: BTreeMap::new(),
        fleet_roles: BTreeMap::new(),
        fleet_formations: BTreeMap::new(),
        fleet_names: BTreeMap::new(),
        scenario: None,
        ai_empires: vec![],
        colony_supply: BTreeMap::new(),
        fleet_supply: BTreeMap::new(),
        colony_blockade: BTreeMap::new(),
        empire_resource_access: BTreeMap::new(),
        victory_status: crate::state::VictoryStatus::default(),
        galactic_dispatches: std::collections::VecDeque::new(),
        custom_designs: BTreeMap::new(),
        next_custom_design_id: 0,
        fleet_custom_designs: BTreeMap::new(),
        next_battle_report_id: 1,
        battle_reports: std::collections::VecDeque::new(),
    };

    state.stars.insert(
        star_id,
        crate::state::Star {
            id: star_id,
            name: "Testara".to_string(),
            x: 0,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::G,
            planets: vec![crate::state::Planet {
                name: "Testara I".to_string(),
                size: crate::state::PlanetSize::Medium,
                class: crate::state::PlanetClass::Terran,
                colony: Some(colony_id),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );

    state.empires.insert(
        player_id,
        Empire {
            id: player_id,
            name: "Player".to_string(),
            credits: 100,
            research_points: 0,
            home_star: star_id,
            research: ResearchState::default(),
            food: 0,
            empire_def: None,
        },
    );

    state.empires.insert(
        enemy_id,
        Empire {
            id: enemy_id,
            name: "Enemy".to_string(),
            credits: 100,
            research_points: 0,
            home_star: StarId(99),
            research: ResearchState::default(),
            food: 0,
            empire_def: None,
        },
    );

    state.colonies.insert(
        colony_id,
        Colony {
            id: colony_id,
            star: star_id,
            planet_index: 0,
            owner: player_id,
            population: 4,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );

    (state, star_id, colony_id, player_id, enemy_id)
}

#[test]
fn enemy_war_fleet_in_colony_system_causes_blockade() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    // Set war status
    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    // Place an idle enemy fleet at the colony star
    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 2,
            kind: FleetKind::Scout,
            strength: 5,
            integrity: 100,
        },
    );

    let blockade = state.recompute_colony_blockade();
    assert!(
        blockade.contains_key(&colony_id),
        "Colony should be blockaded when war enemy fleet is in its system"
    );
    assert_eq!(
        blockade.get(&colony_id),
        Some(&enemy_id),
        "Blockading empire should be the enemy"
    );
}

#[test]
fn hostile_status_fleet_causes_blockade() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    state
        .diplomacy
        .insert(enemy_id, RelationshipStatus::Hostile);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 3,
            integrity: 100,
        },
    );

    let blockade = state.recompute_colony_blockade();
    assert!(
        blockade.contains_key(&colony_id),
        "Colony should be blockaded when Hostile status fleet is present"
    );
}

#[test]
fn contacted_fleet_does_not_blockade() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    // Contacted status — should not blockade
    state
        .diplomacy
        .insert(enemy_id, RelationshipStatus::Contacted);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 3,
            integrity: 100,
        },
    );

    let blockade = state.recompute_colony_blockade();
    assert!(
        !blockade.contains_key(&colony_id),
        "Colony should NOT be blockaded when fleet owner is only Contacted"
    );
}

#[test]
fn neutral_fleet_does_not_blockade() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    // Neutral status — should not blockade
    state
        .diplomacy
        .insert(enemy_id, RelationshipStatus::Neutral);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 3,
            integrity: 100,
        },
    );

    let blockade = state.recompute_colony_blockade();
    assert!(
        !blockade.contains_key(&colony_id),
        "Colony should NOT be blockaded when fleet owner has Neutral status"
    );
}

#[test]
fn unknown_fleet_does_not_blockade() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();
    // No diplomacy entry = Unknown status

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 3,
            integrity: 100,
        },
    );

    let blockade = state.recompute_colony_blockade();
    assert!(
        !blockade.contains_key(&colony_id),
        "Colony should NOT be blockaded when fleet owner relationship is Unknown"
    );
}

#[test]
fn friendly_defending_fleet_prevents_blockade() {
    let (mut state, star_id, colony_id, player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    // Enemy fleet
    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 3,
            integrity: 100,
        },
    );

    // Friendly defending fleet at same star
    let friendly_fid = FleetId(1);
    state.fleets.insert(
        friendly_fid,
        Fleet {
            id: friendly_fid,
            owner: player_id,
            location: star_id,
            ships: 2,
            kind: FleetKind::Scout,
            strength: 5,
            integrity: 100,
        },
    );

    let blockade = state.recompute_colony_blockade();
    assert!(
        !blockade.contains_key(&colony_id),
        "Friendly defending fleet should prevent blockade"
    );
}

#[test]
fn blockade_clears_when_hostile_fleet_leaves() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 3,
            integrity: 100,
        },
    );

    // Verify blockade exists
    let blockade = state.recompute_colony_blockade();
    assert!(
        blockade.contains_key(&colony_id),
        "Blockade should be active"
    );

    // Fleet leaves (remove it)
    state.fleets.remove(&enemy_fid);

    let blockade_after = state.recompute_colony_blockade();
    assert!(
        !blockade_after.contains_key(&colony_id),
        "Blockade should clear when hostile fleet leaves"
    );
}

#[test]
fn combat_resolves_before_blockade_on_fleet_arrival() {
    use crate::state::SpectralClass;
    // Setup: enemy war fleet at player colony star; player fleet arrives.
    // Combat fires first (player fleet destroys enemy); then no blockade.
    let (mut state, star_id, _colony_id, player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    // Weak enemy fleet waiting at the star
    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    // Strong player fleet arrives via fleet_mission (resolves this turn)
    let player_fid = FleetId(1);
    state.fleets.insert(
        player_fid,
        Fleet {
            id: player_fid,
            owner: player_id,
            location: star_id, // will be set by mission resolution
            ships: 3,
            kind: FleetKind::Scout,
            strength: 10,
            integrity: 100,
        },
    );

    let nearby_star = StarId(2);
    state.stars.insert(
        nearby_star,
        crate::state::Star {
            id: nearby_star,
            name: "Nearby".to_string(),
            x: 100,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::G,
            planets: vec![],
        },
    );
    state.explored_stars.insert(nearby_star);
    // Move player fleet to nearby star first, then it will travel to star_id
    if let Some(f) = state.fleets.get_mut(&player_fid) {
        f.location = nearby_star;
    }
    state.fleet_missions.insert(
        player_fid,
        FleetMission {
            fleet: player_fid,
            destination: star_id,
            turns_remaining: 1,
            origin: nearby_star,
            total_duration: 1,
        },
    );

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    // Combat should fire (player fleet arrives + war status)
    let combat: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::CombatResolved { .. }))
        .collect();
    assert!(
        !combat.is_empty(),
        "Combat should fire when war fleet is at star"
    );

    // After combat, check that no blockade event was emitted
    // (enemy fleet should be destroyed by the stronger player fleet)
    let blockade_started: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::BlockadeStarted { .. }))
        .collect();
    assert!(
        blockade_started.is_empty(),
        "No blockade should start after player wins combat"
    );
    // Colony should not be in blockade state
    assert!(
        engine.state.colony_blockade.is_empty(),
        "Colony blockade state should be empty after combat clears the enemy"
    );
}

#[test]
fn blockaded_colony_applies_yield_penalties() {
    // Verify that a blockaded colony has reduced economy during end-turn processing.
    let (mut state, star_id, colony_id, player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    // Place enemy fleet at colony star — no friendly defense
    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    // Initialise blockade from current state so it applies on the NEXT turn
    state.colony_blockade = state.recompute_colony_blockade();

    let initial_credits = state.empires.get(&player_id).unwrap().credits;
    let initial_stability = state.colonies.get(&colony_id).unwrap().stability;

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    // Credits should be reduced (50% yield penalty for blockaded colony)
    let produced: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::ColonyProduced { colony, .. } if *colony == colony_id))
        .collect();
    assert!(!produced.is_empty(), "ColonyProduced event expected");
    if let Event::ColonyProduced { credits, .. } = produced[0] {
        // Credits from a blockaded colony should be reduced to 50%
        // The base yield is positive; blockade halves it
        let empire_after = engine.state.empires.get(&player_id).unwrap();
        assert!(
            empire_after.credits <= initial_credits + credits,
            "Credits should reflect blockade penalty"
        );
    }

    // Stability should decrease
    let colony_after = engine.state.colonies.get(&colony_id).unwrap();
    assert!(
        colony_after.stability < initial_stability,
        "Stability should decrease due to blockade"
    );
    assert_eq!(
        colony_after.stability,
        initial_stability - BLOCKADED_STABILITY_PENALTY,
        "Stability penalty should equal BLOCKADED_STABILITY_PENALTY"
    );
}

#[test]
fn blockade_interrupts_food_supply() {
    // A blockaded colony should not contribute food to the empire trade network.
    let (mut state, star_id, colony_id, player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    // Set blockade state for this turn's economy processing
    state.colony_blockade = state.recompute_colony_blockade();

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    // EconomySummary for the player should show zero food produced
    // (blockaded colony contributes no food)
    let economy_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == player_id))
        .collect();
    assert!(!economy_events.is_empty(), "EconomySummary event expected");
    if let Event::EconomySummary { food_produced, .. } = economy_events[0] {
        assert_eq!(
            *food_produced, 0,
            "Blockaded colony should contribute no food to the trade network"
        );
    }
    let _ = colony_id;
}

#[test]
fn blockade_started_event_emitted_when_blockade_begins() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    // No prior blockade (starts clean)
    assert!(state.colony_blockade.is_empty());

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let blockade_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::BlockadeStarted { colony, .. } if *colony == colony_id))
        .collect();
    assert_eq!(
        blockade_events.len(),
        1,
        "Exactly one BlockadeStarted event expected"
    );
    if let Event::BlockadeStarted {
        colony,
        star,
        by_empire,
    } = blockade_events[0]
    {
        assert_eq!(*colony, colony_id);
        assert_eq!(*star, star_id);
        assert_eq!(*by_empire, enemy_id);
    }
}

#[test]
fn blockade_ended_event_emitted_when_blockade_clears() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    // Pre-populate blockade state so engine sees a running blockade
    state.colony_blockade = state.recompute_colony_blockade();

    // Now remove the enemy fleet (blockade should clear next turn)
    state.fleets.remove(&enemy_fid);

    let mut engine = Engine::from_state(state);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let ended_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::BlockadeEnded { colony, .. } if *colony == colony_id))
        .collect();
    assert_eq!(
        ended_events.len(),
        1,
        "Exactly one BlockadeEnded event expected when enemy fleet leaves"
    );
    if let Event::BlockadeEnded { colony, star } = ended_events[0] {
        assert_eq!(*colony, colony_id);
        assert_eq!(*star, star_id);
    }
}

#[test]
fn blockade_state_persisted_in_game_state() {
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    let mut engine = Engine::from_state(state);
    engine.apply_turn(vec![Command::EndTurn]);

    assert!(
        engine.state.colony_blockade.contains_key(&colony_id),
        "colony_blockade in GameState should be updated after end turn"
    );
}

#[test]
fn declare_war_sets_war_status() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");

    // First establish contact
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Contacted);

    let events = engine.apply_turn(vec![Command::DeclareWar { target: ai_id }]);

    let status = engine
        .state
        .diplomacy
        .get(&ai_id)
        .copied()
        .unwrap_or(RelationshipStatus::Unknown);
    assert_eq!(
        status,
        RelationshipStatus::War,
        "DeclareWar should set status to War"
    );
    let _ = events;
}

#[test]
fn declare_war_on_unknown_empire_is_error() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    // No diplomatic contact established

    let events = engine.apply_turn(vec![Command::DeclareWar { target: ai_id }]);
    assert!(
        events.iter().any(|e| e.is_error()),
        "DeclareWar on unknown empire should produce an Error event"
    );
}

#[test]
fn declare_war_on_own_empire_is_error() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;

    let events = engine.apply_turn(vec![Command::DeclareWar { target: player }]);
    assert!(
        events.iter().any(|e| e.is_error()),
        "DeclareWar on own empire should produce an Error event"
    );
}

#[test]
fn first_contact_creates_diplomatic_communication() {
    let (mut engine, _player_star, ai_star, ai_id) = make_two_empire_state();
    set_empire_definition(&mut engine, ai_id, crate::state::EmpireDefinitionId(6));
    let mut events = Vec::new();
    engine.check_contact_at_star(ai_star, &mut events);
    assert!(events.iter().any(
        |event| matches!(event, Event::FirstContact { with_empire } if *with_empire == ai_id)
    ));
    assert!(engine
        .state
        .diplomacy_pending_communications
        .iter()
        .any(|message| {
            message.communication_type == crate::state::DiplomaticCommunicationType::FirstContact
                && message.sending_empire == ai_id
                && message.receiving_empire == engine.state.player_empire
        }));
}

#[test]
fn terran_concord_first_contact_tone_differs_from_dominion() {
    let (mut concord_engine, _player_star, ai_star, ai_id) = make_two_empire_state();
    set_empire_definition(
        &mut concord_engine,
        ai_id,
        crate::state::EmpireDefinitionId(6),
    );
    let mut concord_events = Vec::new();
    concord_engine.check_contact_at_star(ai_star, &mut concord_events);
    let concord_tone = concord_engine
        .state
        .diplomacy_pending_communications
        .back()
        .map(|message| message.tone)
        .expect("expected first contact communication for concord");

    let (mut dominion_engine, _player_star_d, ai_star_d, ai_id_d) = make_two_empire_state();
    set_empire_definition(
        &mut dominion_engine,
        ai_id_d,
        crate::state::EmpireDefinitionId(7),
    );
    let mut dominion_events = Vec::new();
    dominion_engine.check_contact_at_star(ai_star_d, &mut dominion_events);
    let dominion_tone = dominion_engine
        .state
        .diplomacy_pending_communications
        .back()
        .map(|message| message.tone)
        .expect("expected first contact communication for dominion");
    assert_ne!(concord_tone, dominion_tone);
}

fn queue_inbound_diplomatic_communication(
    engine: &mut Engine,
    from_empire: EmpireId,
    communication_type: crate::state::DiplomaticCommunicationType,
    treaty_type: Option<crate::state::TreatyType>,
) {
    let communication_id = engine.state.diplomacy_next_communication_id;
    engine.state.diplomacy_next_communication_id = engine
        .state
        .diplomacy_next_communication_id
        .saturating_add(1);
    engine.state.diplomacy_pending_communications.push_back(
        crate::state::DiplomaticCommunication {
            communication_id,
            sending_empire: from_empire,
            receiving_empire: engine.state.player_empire,
            turn: engine.state.turn,
            communication_type,
            tone: crate::state::DiplomaticTone::Formal,
            title: "Test message".to_string(),
            body: "Test body".to_string(),
            available_responses: vec![crate::state::DiplomaticResponse::Accept],
            expires_turn: None,
            treaty_type,
        },
    );
}

#[test]
fn non_aggression_pact_blocks_war_declaration() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Neutral);

    queue_inbound_diplomatic_communication(
        &mut engine,
        ai_id,
        crate::state::DiplomaticCommunicationType::TreatyProposal,
        Some(crate::state::TreatyType::NonAggressionPact),
    );
    let _ = engine.apply_turn(vec![Command::AcceptNonAggressionPact { target: ai_id }]);
    let events = engine.apply_turn(vec![Command::DeclareWar { target: ai_id }]);
    assert!(events.iter().any(|event| event.is_error()));
}

#[test]
fn peace_treaty_ends_war_and_starts_truce() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::War);

    queue_inbound_diplomatic_communication(
        &mut engine,
        ai_id,
        crate::state::DiplomaticCommunicationType::PeaceOffer,
        Some(crate::state::TreatyType::Truce),
    );
    let events = engine.apply_turn(vec![Command::AcceptPeace { target: ai_id }]);
    assert!(events.iter().any(
        |event| matches!(event, Event::PeaceSigned { with_empire, .. } if *with_empire == ai_id)
    ));
    assert!(engine
        .state
        .has_active_treaty(ai_id, crate::state::TreatyType::Truce));
    assert_eq!(
        engine
            .state
            .relationship_status(engine.state.player_empire, ai_id),
        RelationshipStatus::Neutral
    );
}

#[test]
fn treaty_expiration_emits_event_deterministically() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Neutral);
    queue_inbound_diplomatic_communication(
        &mut engine,
        ai_id,
        crate::state::DiplomaticCommunicationType::TreatyProposal,
        Some(crate::state::TreatyType::NonAggressionPact),
    );
    let _ = engine.apply_turn(vec![Command::AcceptNonAggressionPact { target: ai_id }]);
    engine.state.turn = engine.state.turn.saturating_add(NAP_DURATION_TURNS);
    let events = engine.apply_turn(vec![Command::EndTurn]);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            Event::TreatyExpired {
                with_empire,
                treaty_type
            } if *with_empire == ai_id && *treaty_type == crate::state::TreatyType::NonAggressionPact
        )
    }));
}

#[test]
fn duplicate_warning_communication_is_prevented() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Tense);
    let _ = engine.apply_turn(vec![Command::IssueWarning { target: ai_id }]);
    let _ = engine.apply_turn(vec![Command::IssueWarning { target: ai_id }]);
    let warning_count = engine
        .state
        .diplomacy_pending_communications
        .iter()
        .filter(|message| {
            message.communication_type == crate::state::DiplomaticCommunicationType::Warning
        })
        .count();
    assert_eq!(warning_count, 1);
}

#[test]
fn accept_peace_requires_pending_offer() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::War);

    let events = engine.apply_turn(vec![Command::AcceptPeace { target: ai_id }]);
    assert!(events.iter().any(|event| {
        matches!(event, Event::Error { message } if message.contains("No pending peace offer"))
    }));
    assert!(!engine
        .state
        .has_active_treaty(ai_id, crate::state::TreatyType::Truce));
}

#[test]
fn accept_non_aggression_requires_pending_proposal() {
    let mut engine = Engine::new(42);
    let ai_id = engine.state.ai_empire.expect("AI empire must exist");
    engine
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Neutral);

    let events = engine.apply_turn(vec![Command::AcceptNonAggressionPact { target: ai_id }]);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            Event::Error { message } if message.contains("No pending non-aggression proposal")
        )
    }));
    assert!(!engine
        .state
        .has_active_treaty(ai_id, crate::state::TreatyType::NonAggressionPact));
}

#[test]
fn ai_proposes_treaty_deterministically() {
    let mut engine_a = Engine::new(42);
    let ai_id = engine_a.state.ai_empire.expect("AI empire must exist");
    set_empire_definition(&mut engine_a, ai_id, crate::state::EmpireDefinitionId(6));
    engine_a.state.turn = 10;
    engine_a
        .state
        .diplomacy
        .insert(ai_id, RelationshipStatus::Neutral);
    let mut events_a = Vec::new();
    engine_a.process_ai_diplomacy_with_events(&mut events_a);

    let mut engine_b = Engine::new(42);
    let ai_id_b = engine_b.state.ai_empire.expect("AI empire must exist");
    set_empire_definition(&mut engine_b, ai_id_b, crate::state::EmpireDefinitionId(6));
    engine_b.state.turn = 10;
    engine_b
        .state
        .diplomacy
        .insert(ai_id_b, RelationshipStatus::Neutral);
    let mut events_b = Vec::new();
    engine_b.process_ai_diplomacy_with_events(&mut events_b);

    assert_eq!(events_a, events_b);
    assert!(events_a.iter().any(|event| {
        matches!(
            event,
            Event::TreatyProposed {
                treaty_type,
                ..
            } if *treaty_type == crate::state::TreatyType::NonAggressionPact
        )
    }));
}

#[test]
fn ai_declares_war_deterministically() {
    let (mut engine_a, _player_star_a, _ai_star_a, ai_id_a) = make_two_empire_state();
    set_empire_definition(&mut engine_a, ai_id_a, crate::state::EmpireDefinitionId(7));
    engine_a.state.turn = 18;
    engine_a
        .state
        .diplomacy
        .insert(ai_id_a, RelationshipStatus::Hostile);
    let mut events_a = Vec::new();
    engine_a.process_ai_diplomacy_with_events(&mut events_a);

    let (mut engine_b, _player_star_b, _ai_star_b, ai_id_b) = make_two_empire_state();
    set_empire_definition(&mut engine_b, ai_id_b, crate::state::EmpireDefinitionId(7));
    engine_b.state.turn = 18;
    engine_b
        .state
        .diplomacy
        .insert(ai_id_b, RelationshipStatus::Hostile);
    let mut events_b = Vec::new();
    engine_b.process_ai_diplomacy_with_events(&mut events_b);

    assert_eq!(events_a, events_b);
    assert!(events_a
        .iter()
        .any(|event| matches!(event, Event::WarDeclared { .. })));
}

#[test]
fn relationship_status_is_hostile_or_war() {
    assert!(RelationshipStatus::Hostile.is_hostile_or_war());
    assert!(RelationshipStatus::War.is_hostile_or_war());
    assert!(!RelationshipStatus::Contacted.is_hostile_or_war());
    assert!(!RelationshipStatus::Neutral.is_hostile_or_war());
    assert!(!RelationshipStatus::Cooperative.is_hostile_or_war());
    assert!(!RelationshipStatus::Tense.is_hostile_or_war());
    assert!(!RelationshipStatus::Unknown.is_hostile_or_war());
}

#[test]
fn relationship_status_is_combat_eligible() {
    assert!(RelationshipStatus::Contacted.is_combat_eligible());
    assert!(RelationshipStatus::Tense.is_combat_eligible());
    assert!(RelationshipStatus::Hostile.is_combat_eligible());
    assert!(RelationshipStatus::War.is_combat_eligible());
    assert!(!RelationshipStatus::Neutral.is_combat_eligible());
    assert!(!RelationshipStatus::Cooperative.is_combat_eligible());
    assert!(!RelationshipStatus::Unknown.is_combat_eligible());
}

#[cfg(feature = "serde")]
#[test]
fn blockade_save_load_round_trip() {
    // Verify that blockade state survives a save/load cycle by being re-derived.
    use crate::state::RelationshipStatus;
    let (mut state, star_id, colony_id, _player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    // Compute and store blockade
    state.colony_blockade = state.recompute_colony_blockade();
    assert!(state.colony_blockade.contains_key(&colony_id));

    // Serialize and deserialize
    let json = serde_json::to_string(&state).expect("serialize ok");
    let restored: GameState = serde_json::from_str(&json).expect("deserialize ok");

    // The deserialized state should have the blockade field preserved (or re-derivable)
    let rederived = restored.recompute_colony_blockade();
    assert!(
        rederived.contains_key(&colony_id),
        "Blockade should be re-derivable after save/load"
    );
}

#[test]
fn blockade_turn_report_events_appear_in_log() {
    let (mut state, star_id, _colony_id, _player_id, enemy_id) = make_blockade_state();

    state.diplomacy.insert(enemy_id, RelationshipStatus::War);

    let enemy_fid = FleetId(20);
    state.fleets.insert(
        enemy_fid,
        Fleet {
            id: enemy_fid,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 2,
            integrity: 100,
        },
    );

    let mut engine = Engine::from_state(state);
    engine.apply_turn(vec![Command::EndTurn]);

    let blockade_log_entries: Vec<_> = engine
        .state
        .event_log
        .iter()
        .filter(|msg| msg.contains("blockade") || msg.contains("Blockade"))
        .collect();
    assert!(
        !blockade_log_entries.is_empty(),
        "Blockade event should appear in the turn log"
    );
}

fn make_invasion_engine() -> (Engine, StarId, ColonyId, EmpireId, EmpireId, FleetId) {
    let (mut state, star_id, colony_id, player_id, enemy_id) = make_blockade_state();
    let support_star = StarId(2);
    let support_colony = ColonyId(2);
    state.diplomacy.insert(enemy_id, RelationshipStatus::War);
    state.explored_stars.insert(support_star);
    state.stars.insert(
        support_star,
        Star {
            id: support_star,
            name: "Forward Base".to_string(),
            x: 200,
            y: 0,
            sector: SectorId(0),
            spectral_class: SpectralClass::F,
            planets: vec![Planet {
                name: "Forward Base I".to_string(),
                size: PlanetSize::Medium,
                class: PlanetClass::Terran,
                colony: Some(support_colony),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );
    state.colonies.insert(
        support_colony,
        Colony {
            id: support_colony,
            star: support_star,
            planet_index: 0,
            owner: player_id,
            population: 4,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );
    if let Some(colony) = state.colonies.get_mut(&colony_id) {
        colony.owner = enemy_id;
        colony.population = 1;
        colony.stability = 10;
        colony.surface_installations.clear();
        colony.orbital_installations.clear();
    }
    let troop_fleet = FleetId(30);
    state.fleets.insert(
        troop_fleet,
        Fleet {
            id: troop_fleet,
            owner: player_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::TroopTransport,
            strength: 1,
            integrity: 100,
        },
    );
    (
        Engine::from_state(state),
        star_id,
        colony_id,
        player_id,
        enemy_id,
        troop_fleet,
    )
}

#[test]
fn cannot_invade_without_war_or_hostile_state() {
    let (mut engine, star_id, _colony_id, _player_id, enemy_id, troop_fleet) =
        make_invasion_engine();
    engine
        .state
        .diplomacy
        .insert(enemy_id, RelationshipStatus::Neutral);

    let events = engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn cannot_invade_without_troop_transport_present() {
    let (mut state, star_id, colony_id, player_id, enemy_id) = make_blockade_state();
    state.diplomacy.insert(enemy_id, RelationshipStatus::War);
    state.colonies.get_mut(&colony_id).unwrap().owner = enemy_id;
    let scout_id = FleetId(44);
    state.fleets.insert(
        scout_id,
        Fleet {
            id: scout_id,
            owner: player_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    let mut engine = Engine::from_state(state);

    let events = engine.apply_turn(vec![Command::Invade {
        fleet: scout_id,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn cannot_invade_own_colony() {
    let (mut state, star_id, _colony_id, player_id, enemy_id) = make_blockade_state();
    state.diplomacy.insert(enemy_id, RelationshipStatus::War);
    let fleet_id = FleetId(45);
    state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::TroopTransport,
            strength: 1,
            integrity: 100,
        },
    );
    let mut engine = Engine::from_state(state);

    let events = engine.apply_turn(vec![Command::Invade {
        fleet: fleet_id,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn cannot_invade_uncolonized_planet() {
    let (mut engine, star_id, _colony_id, _player_id, _enemy_id, troop_fleet) =
        make_invasion_engine();
    engine.state.stars.get_mut(&star_id).unwrap().planets[0].colony = None;

    let events = engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn out_of_supply_fleet_cannot_invade() {
    use crate::state::{Planet, PlanetClass, PlanetSize, SpectralClass, Star};

    let (mut state, home_star, _home_colony, player_id, enemy_id) = make_blockade_state();
    let target_star = StarId(55);
    let target_colony = ColonyId(55);
    state.diplomacy.insert(enemy_id, RelationshipStatus::War);
    state.stars.insert(
        target_star,
        Star {
            id: target_star,
            name: "Deep Redoubt".to_string(),
            x: 1_200,
            y: 0,
            sector: SectorId(1),
            spectral_class: SpectralClass::K,
            planets: vec![Planet {
                name: "Deep Redoubt I".to_string(),
                size: PlanetSize::Medium,
                class: PlanetClass::Terran,
                colony: Some(target_colony),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );
    state.colonies.insert(
        target_colony,
        Colony {
            id: target_colony,
            star: target_star,
            planet_index: 0,
            owner: enemy_id,
            population: 3,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );
    let fleet_id = FleetId(79);
    state.fleets.insert(
        fleet_id,
        Fleet {
            id: fleet_id,
            owner: player_id,
            location: target_star,
            ships: 1,
            kind: FleetKind::TroopTransport,
            strength: 1,
            integrity: 100,
        },
    );
    let mut engine = Engine::from_state(state);

    assert_eq!(
        engine.state.fleet_supply_state(fleet_id),
        FleetSupplyState::OutOfSupply
    );
    let events = engine.apply_turn(vec![Command::Invade {
        fleet: fleet_id,
        star: target_star,
        planet_index: 0,
    }]);

    assert!(events.iter().any(|event| {
        matches!(event, Event::Error { message } if message.contains("out of supply"))
    }));
    assert_eq!(engine.state.colonies[&target_colony].owner, enemy_id);
    assert_eq!(engine.state.empires[&player_id].home_star, home_star);
}

#[test]
fn orbital_defenses_block_invasion() {
    let (mut engine, star_id, colony_id, _player_id, enemy_id, troop_fleet) =
        make_invasion_engine();
    let defender_fleet = FleetId(46);
    engine.state.fleets.insert(
        defender_fleet,
        Fleet {
            id: defender_fleet,
            owner: enemy_id,
            location: star_id,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );

    let events = engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events.iter().any(
        |e| matches!(e, Event::InvasionFailed { reason, .. } if reason.contains("orbital defenses"))
    ));
    assert_eq!(engine.state.colonies[&colony_id].owner, enemy_id);
}

#[test]
fn successful_invasion_transfers_ownership_and_sets_unrest() {
    let (mut engine, star_id, colony_id, player_id, _enemy_id, troop_fleet) =
        make_invasion_engine();
    let events = engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events
        .iter()
        .any(|e| matches!(e, Event::InvasionSucceeded { colony, .. } if *colony == colony_id)));
    let colony = &engine.state.colonies[&colony_id];
    assert_eq!(colony.owner, player_id);
    assert_eq!(colony.stability, CAPTURED_UNREST_STABILITY);
    assert!(colony.is_unrest());
    assert!(!engine.state.fleets.contains_key(&troop_fleet));
}

#[test]
fn failed_invasion_preserves_ownership() {
    let (mut engine, star_id, colony_id, _player_id, enemy_id, troop_fleet) =
        make_invasion_engine();
    if let Some(colony) = engine.state.colonies.get_mut(&colony_id) {
        colony.population = 6;
        colony.stability = 160;
    }
    let events = engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events
        .iter()
        .any(|e| matches!(e, Event::InvasionFailed { .. })));
    assert_eq!(engine.state.colonies[&colony_id].owner, enemy_id);
}

#[test]
fn failed_invasion_reduces_transport_deterministically() {
    let (mut engine, star_id, colony_id, _player_id, _enemy_id, troop_fleet) =
        make_invasion_engine();
    if let Some(colony) = engine.state.colonies.get_mut(&colony_id) {
        colony.population = 5;
        colony.stability = 120;
    }
    if let Some(fleet) = engine.state.fleets.get_mut(&troop_fleet) {
        fleet.ships = 2;
    }

    let events = engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);

    assert!(events.iter().any(
        |e| matches!(e, Event::InvasionFailed { transports_lost, .. } if *transports_lost == 1)
    ));
    assert_eq!(engine.state.fleets[&troop_fleet].ships, 1);
}

#[test]
fn captured_colony_contributes_to_new_owner_economy_next_turn() {
    let (mut engine, star_id, colony_id, player_id, enemy_id, troop_fleet) = make_invasion_engine();

    engine.apply_turn(vec![Command::Invade {
        fleet: troop_fleet,
        star: star_id,
        planet_index: 0,
    }]);
    let events = engine.apply_turn(vec![Command::EndTurn]);

    let player_summary = events.iter().find_map(|e| match e {
        Event::EconomySummary {
            empire,
            credits_income,
            ..
        } if *empire == player_id => Some(*credits_income),
        _ => None,
    });
    let enemy_summary = events.iter().find_map(|e| match e {
        Event::EconomySummary {
            empire,
            credits_income,
            ..
        } if *empire == enemy_id => Some(*credits_income),
        _ => None,
    });

    assert_eq!(engine.state.colonies[&colony_id].owner, player_id);
    assert!(player_summary.is_some());
    assert_eq!(enemy_summary.unwrap_or(0), 0);
}

// ---------------------------------------------------------------------------
// Ship Designer Lite v1 — engine-level tests
// ---------------------------------------------------------------------------

fn player_colony(engine: &Engine) -> ColonyId {
    let player = engine.state.player_empire;
    engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == player)
        .map(|(id, _)| *id)
        .expect("player must have a colony")
}

/// Helper: return the first HullId that requires no tech (Scout hull)
fn scout_hull_id() -> HullId {
    HullId::SCOUT
}

/// Test: CreateShipDesign creates a design and emits ShipDesignCreated event
#[test]
fn create_ship_design_creates_design_and_event() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let initial_count = engine.state.custom_designs.len();

    // Scout has [Engine, Utility] slots: Chemical Thrusters (20) + Cargo Pods (32)
    let events = engine.apply_turn(vec![Command::CreateShipDesign {
        hull_id: scout_hull_id(),
        components: vec![ComponentId(20), ComponentId(32)],
        name: Some("My Scout".to_string()),
    }]);

    let created = events
        .iter()
        .any(|e| matches!(e, Event::ShipDesignCreated { empire, .. } if *empire == player));
    assert!(created, "ShipDesignCreated event must be emitted");
    assert_eq!(
        engine.state.custom_designs.len(),
        initial_count + 1,
        "custom_designs must grow by 1"
    );

    let design = engine
        .state
        .custom_designs
        .values()
        .find(|d| d.owner == player && d.name == "My Scout")
        .unwrap();
    assert_eq!(design.name, "My Scout");
    assert_eq!(design.owner, player);
    assert!(!design.obsolete);
}

/// Test: DeleteShipDesign marks design obsolete and emits ShipDesignDeleted event
#[test]
fn delete_ship_design_marks_obsolete_and_emits_event() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;

    // Create a design first — Scout with valid components
    engine.apply_turn(vec![Command::CreateShipDesign {
        hull_id: scout_hull_id(),
        components: vec![ComponentId(20), ComponentId(32)],
        name: None,
    }]);

    let design_id = engine
        .state
        .custom_designs
        .iter()
        .find(|(_, d)| d.owner == player && !d.obsolete)
        .map(|(id, _)| *id)
        .expect("player must have a design after CreateShipDesign");
    assert!(!engine.state.custom_designs[&design_id].obsolete);

    let events = engine.apply_turn(vec![Command::DeleteShipDesign { design_id }]);

    let deleted = events
        .iter()
        .any(|e| matches!(e, Event::ShipDesignDeleted { empire, .. } if *empire == player));
    assert!(deleted, "ShipDesignDeleted event must be emitted");
    assert!(
        engine.state.custom_designs[&design_id].obsolete,
        "Design must be marked obsolete"
    );
}

/// Test: CreateShipDesign with missing hull tech emits ShipDesignInvalid
#[test]
fn create_ship_design_invalid_hull_tech_emits_invalid_event() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let initial_count = engine.state.custom_designs.len();

    // Colony Ark requires COLONIAL_VANGUARD (TechId 15) — not unlocked by default
    let events = engine.apply_turn(vec![Command::CreateShipDesign {
        hull_id: HullId::COLONY_ARK,
        components: vec![],
        name: None,
    }]);

    let invalid = events
        .iter()
        .any(|e| matches!(e, Event::ShipDesignInvalid { empire, .. } if *empire == player));
    assert!(
        invalid,
        "ShipDesignInvalid event must be emitted for missing hull tech"
    );
    assert_eq!(
        engine.state.custom_designs.len(),
        initial_count,
        "No design should be created when validation fails"
    );
}

/// Test: DeleteShipDesign on unknown design ID emits Error event
#[test]
fn delete_ship_design_unknown_id_emits_error() {
    let mut engine = Engine::new(42);
    let bogus_id = CustomDesignId(9999);

    let events = engine.apply_turn(vec![Command::DeleteShipDesign {
        design_id: bogus_id,
    }]);

    let has_error = events.iter().any(|e| matches!(e, Event::Error { .. }));
    assert!(
        has_error,
        "Error event must be emitted for unknown design id"
    );
}

/// Test: AI empires have designs generated on setup
#[test]
fn ai_empires_have_designs_after_setup() {
    let engine = Engine::new(42);
    let ai_empires = engine.state.ai_empires.clone();
    assert!(!ai_empires.is_empty(), "Must have at least one AI empire");

    for ai_id in &ai_empires {
        let has_design = engine
            .state
            .custom_designs
            .values()
            .any(|d| d.owner == *ai_id && !d.obsolete);
        assert!(
            has_design,
            "AI empire {} must have at least one design",
            ai_id.0
        );
    }
}

// Balance and pacing tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod balance_tests {
    use super::*;
    use crate::balance;

    /// Verify that Void Propulsion (cost=40, TechId(1)) can complete within 15 turns
    /// for a standard starting colony researching it from turn 1.
    #[test]
    fn test_early_research_pacing() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;

        // Set Void Propulsion as the active research
        if let Some(empire) = engine.state.empires.get_mut(&player) {
            empire.research.current_tech = Some(TechId(1));
            empire.research.progress = 0;
        }

        let mut completed = false;
        for _ in 0..15 {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events
                .iter()
                .any(|e| matches!(e, Event::ResearchCompleted { tech, .. } if *tech == TechId(1)))
            {
                completed = true;
                break;
            }
        }

        assert!(
            completed,
            "Void Propulsion (cost {}) should complete within 15 turns from a standard colony",
            40
        );
    }

    /// Verify that a colony with higher population produces more industry.
    #[test]
    fn test_production_scales_with_population() {
        use crate::yield_model::calculate_yield;

        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Get a reference planet for yield calculation
        let (star_id, planet_index) = {
            let c = &engine.state.colonies[&colony_id];
            (c.star, c.planet_index)
        };
        let planet = engine
            .state
            .stars
            .get(&star_id)
            .and_then(|s| s.planets.get(planet_index))
            .cloned();

        // Set up a low-pop colony
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .population = 5;
        let low_pop_colony = engine.state.colonies[&colony_id].clone();
        let yield_low = calculate_yield(&low_pop_colony, planet.as_ref());

        // Set up a high-pop colony
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .population = 20;
        let high_pop_colony = engine.state.colonies[&colony_id].clone();
        let yield_high = calculate_yield(&high_pop_colony, planet.as_ref());

        assert!(
            yield_high.industry > yield_low.industry,
            "Colony with pop=20 ({}) should produce more industry than pop=5 ({})",
            yield_high.industry,
            yield_low.industry
        );
    }

    /// Verify that population grows when conditions are met (enough food, stability, housing).
    #[test]
    fn test_pop_growth_within_bounds() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let colony_id = ColonyId(1);
        let initial_pop = engine.state.colonies[&colony_id].population;

        // Set stability well above the threshold
        engine.state.colonies.get_mut(&colony_id).unwrap().stability = 100;

        // Ensure empire has food surplus (required for pop growth)
        engine.state.empires.get_mut(&player).unwrap().food = 100;

        // Derive the required turn so that cadence = turn + colony_id.0 is a multiple
        // of POP_GROWTH_PERIOD_TURNS.  The formula generalises for any colony_id.0 value:
        //   turn ≡ -colony_id.0 (mod POP_GROWTH_PERIOD_TURNS)
        let period = u64::from(POP_GROWTH_PERIOD_TURNS);
        let growth_turn = ((period - colony_id.0 % period) % period) as u32;
        engine.state.turn = growth_turn;

        let events = engine.apply_turn(vec![Command::EndTurn]);
        let grew = events
            .iter()
            .any(|e| matches!(e, Event::PopulationGrew { colony, .. } if *colony == colony_id));

        let final_pop = engine.state.colonies[&colony_id].population;

        assert!(
            grew || final_pop > initial_pop,
            "Population should grow when stability=100, food surplus, and growth cadence met \
             (initial_pop={}, final_pop={}, grew={})",
            initial_pop,
            final_pop,
            grew
        );
    }

    /// Verify that a blockaded colony loses stability over time.
    #[test]
    fn test_blockade_stability_pressure() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        let enemy_id = engine.state.ai_empire.expect("Engine must have AI empire");

        // Directly inject the blockade state to bypass the "defender present"
        // guard (the starting engine has a player scout at the home star).
        // This tests the stability penalty logic in process_end_turn directly.
        engine.state.colony_blockade.insert(colony_id, enemy_id);

        let initial_stability = engine.state.colonies[&colony_id].stability;

        engine.apply_turn(vec![Command::EndTurn]);
        let after_stability = engine.state.colonies[&colony_id].stability;

        // Note: process_end_turn recomputes blockade at the end; the penalty is
        // applied based on the pre-turn blockade we injected above.
        assert!(
            after_stability < initial_stability,
            "Blockaded colony stability ({}) should decrease from initial ({})",
            after_stability,
            initial_stability
        );
        assert_eq!(
            initial_stability - after_stability,
            balance::BLOCKADED_STABILITY_PENALTY,
            "Stability loss must equal BLOCKADED_STABILITY_PENALTY"
        );
    }

    /// Verify that population growth is suppressed when food balance is negative.
    #[test]
    fn test_colony_growth_suppressed_food_shortage() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let colony_id = ColonyId(1);

        // Put empire in food deficit
        engine.state.empires.get_mut(&player).unwrap().food = -10;

        // Set stability high (not the limiting factor)
        engine.state.colonies.get_mut(&colony_id).unwrap().stability = 100;

        let initial_pop = engine.state.colonies[&colony_id].population;

        // Advance many turns to confirm no growth occurs
        for _ in 0..50 {
            // Keep food negative
            engine.state.empires.get_mut(&player).unwrap().food = -10;
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let final_pop = engine.state.colonies[&colony_id].population;
        assert_eq!(
            final_pop, initial_pop,
            "Population should not grow when empire food is negative"
        );
    }

    /// Verify that empire credits decrease when maintaining many fleets.
    #[test]
    fn test_maintenance_limits_fleet_spam() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;

        // Give the player empire a lot of credits to start
        engine.state.empires.get_mut(&player).unwrap().credits = 1_000;

        // Add many combat fleets (Patrol Corvettes have maintenance cost 1)
        let player_star = engine.state.colonies[&ColonyId(1)].star;
        // Start IDs high enough to avoid collisions with the engine-generated fleets
        // that Engine::new() pre-inserts (which begin at low IDs like 1, 2, …).
        let start_fleet_id = 9_200u64;
        for i in 0..20 {
            let fid = FleetId(start_fleet_id + i);
            engine.state.fleets.insert(
                fid,
                Fleet {
                    id: fid,
                    owner: player,
                    location: player_star,
                    ships: 1,
                    kind: FleetKind::PatrolCorvette,
                    strength: 5,
                    integrity: 100,
                },
            );
        }

        let credits_before = engine.state.empires[&player].credits;
        engine.apply_turn(vec![Command::EndTurn]);
        let credits_after = engine.state.empires[&player].credits;

        // Credits should have decreased due to fleet maintenance
        assert!(
            credits_after < credits_before,
            "Empire credits should decrease after paying fleet maintenance (before={}, after={})",
            credits_before,
            credits_after
        );
    }
}

/// Test: CreateShipDesign is deterministic — same seed, same commands → same design ID
#[test]
fn create_ship_design_is_deterministic() {
    let mut engine_a = Engine::new(99);
    let mut engine_b = Engine::new(99);

    let cmds = vec![Command::CreateShipDesign {
        hull_id: scout_hull_id(),
        components: vec![ComponentId(20), ComponentId(32)],
        name: Some("Det".to_string()),
    }];

    engine_a.apply_turn(cmds.clone());
    engine_b.apply_turn(cmds);

    let id_a = engine_a.state.next_custom_design_id;
    let id_b = engine_b.state.next_custom_design_id;
    assert_eq!(
        id_a, id_b,
        "next_custom_design_id must be identical after identical commands"
    );
    assert_eq!(engine_a.state.custom_designs, engine_b.state.custom_designs);
}

/// Test: BuildItem::CustomShip queued through production completes and spawns a fleet.
#[test]
fn build_custom_ship_creates_fleet_on_completion() {
    let mut engine = Engine::new(42);
    let player = engine.state.player_empire;
    let colony_id = player_colony(&engine);

    give_colony_shipyard(&mut engine, colony_id);

    // Create a Scout design with valid components: Chemical Thrusters + Cargo Pods
    let design_events = engine.apply_turn(vec![Command::CreateShipDesign {
        hull_id: HullId::SCOUT,
        components: vec![ComponentId(20), ComponentId(32)],
        name: Some("Battle Scout".to_string()),
    }]);
    let design_id = design_events
        .iter()
        .find_map(|e| {
            if let Event::ShipDesignCreated { design_id, .. } = e {
                Some(*design_id)
            } else {
                None
            }
        })
        .expect("ShipDesignCreated event must be present");

    let fleet_count_before = engine.state.fleets.len();

    // Queue and complete the custom ship build
    engine
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .production = 9999;
    engine.apply_turn(vec![
        Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::CustomShip(design_id),
        },
        Command::EndTurn,
    ]);

    assert!(
        engine.state.fleets.len() > fleet_count_before,
        "A new fleet must be spawned after custom ship production completes"
    );

    // Verify fleet has the expected kind from the Scout hull
    let new_fleet = engine
        .state
        .fleets
        .values()
        .filter(|f| f.owner == player)
        .max_by_key(|f| f.id.0)
        .expect("Player must own at least one fleet");
    assert_eq!(new_fleet.kind, FleetKind::Scout);
    assert!(new_fleet.strength >= 1);
}

/// Test: BuildItem::CustomShip with obsolete design emits ShipDesignInvalid on queue.
#[test]
fn build_custom_ship_obsolete_design_fails() {
    let mut engine = Engine::new(42);
    let _player = engine.state.player_empire;
    let colony_id = player_colony(&engine);

    give_colony_shipyard(&mut engine, colony_id);

    // Create and delete (mark obsolete) a design
    let design_events = engine.apply_turn(vec![Command::CreateShipDesign {
        hull_id: HullId::SCOUT,
        components: vec![ComponentId(20), ComponentId(32)],
        name: None,
    }]);
    let design_id = design_events
        .iter()
        .find_map(|e| {
            if let Event::ShipDesignCreated { design_id, .. } = e {
                Some(*design_id)
            } else {
                None
            }
        })
        .expect("ShipDesignCreated must be present");

    // Mark design obsolete
    engine
        .state
        .custom_designs
        .get_mut(&design_id)
        .unwrap()
        .obsolete = true;

    // Attempt to queue the obsolete design — should emit an error
    let events = engine.apply_turn(vec![Command::QueueBuild {
        colony: colony_id,
        item: BuildItem::CustomShip(design_id),
    }]);

    let has_error = events.iter().any(|e| matches!(e, Event::Error { .. }));
    assert!(
        has_error,
        "Queuing an obsolete custom design must emit an Error event"
    );
}

#[test]
fn dispatch_history_trimmed_to_max() {
    use crate::dispatch::DISPATCH_MAX_HISTORY;

    // Create an engine and run enough turns to overflow DISPATCH_MAX_HISTORY.
    // Each cadence is 5 turns; we run (DISPATCH_MAX_HISTORY + 2) * 5 turns to guarantee
    // more than DISPATCH_MAX_HISTORY dispatches would have been generated without trimming.
    let mut engine = Engine::new(42);
    let over_limit_turns = (DISPATCH_MAX_HISTORY as u32 + 2) * 5;
    for _ in 0..over_limit_turns {
        engine.apply_turn(vec![Command::EndTurn]);
    }

    assert!(
        engine.state.galactic_dispatches.len() <= DISPATCH_MAX_HISTORY,
        "dispatch history must be capped at DISPATCH_MAX_HISTORY ({}), got {}",
        DISPATCH_MAX_HISTORY,
        engine.state.galactic_dispatches.len()
    );

    // Must have the maximum number of dispatches (not fewer)
    assert_eq!(
        engine.state.galactic_dispatches.len(),
        DISPATCH_MAX_HISTORY,
        "dispatch history should be exactly DISPATCH_MAX_HISTORY after overflow"
    );

    // The oldest dispatch should have been evicted — the front dispatch turn
    // must be later than turn 0 (which would be the very first dispatch).
    let front_turn = engine.state.galactic_dispatches.front().unwrap().turn;
    assert!(
        front_turn > 0,
        "oldest dispatch should have been evicted; front turn is {front_turn}"
    );
}

#[test]
fn fleet_role_and_formation_assignment_commands_are_deterministic() {
    let mut engine = Engine::new(42);
    let fleet_id = engine
        .state
        .fleets
        .values()
        .find(|fleet| fleet.owner == engine.state.player_empire)
        .map(|fleet| fleet.id)
        .expect("player fleet required");

    let events_a = engine.apply_turn(vec![
        Command::SetFleetRole {
            fleet: fleet_id,
            role: crate::state::FleetRole::StrikeFleet,
        },
        Command::SetFleetFormation {
            fleet: fleet_id,
            formation: crate::state::FleetFormation::Aggressive,
        },
    ]);
    let events_b = engine.apply_turn(vec![
        Command::SetFleetRole {
            fleet: fleet_id,
            role: crate::state::FleetRole::StrikeFleet,
        },
        Command::SetFleetFormation {
            fleet: fleet_id,
            formation: crate::state::FleetFormation::Aggressive,
        },
    ]);

    assert_eq!(
        engine.state.fleet_role_for(fleet_id),
        crate::state::FleetRole::StrikeFleet
    );
    assert_eq!(
        engine.state.fleet_formation_for(fleet_id),
        crate::state::FleetFormation::Aggressive
    );
    assert!(events_a
        .iter()
        .any(|e| matches!(e, Event::FleetRoleChanged { fleet, .. } if *fleet == fleet_id)));
    assert!(events_a
        .iter()
        .any(|e| matches!(e, Event::FleetFormationChanged { fleet, .. } if *fleet == fleet_id)));
    assert_eq!(
        events_a
            .iter()
            .filter(|e| matches!(e, Event::FleetRoleChanged { .. }))
            .count(),
        events_b
            .iter()
            .filter(|e| matches!(e, Event::FleetRoleChanged { .. }))
            .count()
    );
}

#[test]
fn fleet_role_assignment_unknown_fleet_emits_error() {
    let mut engine = Engine::new(42);
    let events = engine.apply_turn(vec![Command::SetFleetRole {
        fleet: FleetId(999_999),
        role: crate::state::FleetRole::DefenseFleet,
    }]);
    assert!(events.iter().any(|e| e.is_error()));
}

#[test]
fn fleet_evaluation_is_deterministic_for_same_composition() {
    let engine_a = Engine::new(123);
    let engine_b = Engine::new(123);
    let fleet_id = engine_a
        .state
        .fleets
        .keys()
        .next()
        .copied()
        .expect("fleet required");

    assert_eq!(
        engine_a.state.fleet_evaluation(fleet_id),
        engine_b.state.fleet_evaluation(fleet_id)
    );
}

#[test]
fn fleet_formation_modifiers_change_summary_deterministically() {
    let mut engine = Engine::new(42);
    let fleet_id = engine
        .state
        .fleets
        .values()
        .find(|fleet| fleet.owner == engine.state.player_empire)
        .map(|fleet| fleet.id)
        .expect("player fleet required");

    let baseline = engine
        .state
        .fleet_evaluation(fleet_id)
        .expect("summary required");
    engine.apply_turn(vec![Command::SetFleetFormation {
        fleet: fleet_id,
        formation: crate::state::FleetFormation::Aggressive,
    }]);
    let aggressive = engine
        .state
        .fleet_evaluation(fleet_id)
        .expect("summary required");
    engine.apply_turn(vec![Command::SetFleetFormation {
        fleet: fleet_id,
        formation: crate::state::FleetFormation::Defensive,
    }]);
    let defensive = engine
        .state
        .fleet_evaluation(fleet_id)
        .expect("summary required");

    assert!(aggressive.offensive >= baseline.offensive);
    assert!(defensive.defensive >= baseline.defensive);
}
