//! FARSPACE save/load system
//!
//! This crate provides serialization and deserialization of game state.

mod migrate;
mod schema;

pub use schema::{SaveFile, CURRENT_VERSION};

use game_core::state::GameState;
use thiserror::Error;

/// Errors that can occur during save/load operations
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsupported save version: found {found}, supported up to {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },

    #[error("Save file is empty or corrupted")]
    Empty,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Save game state to JSON bytes
pub fn save(state: &GameState) -> Result<Vec<u8>, SaveError> {
    let save_file = SaveFile::new(state.clone());
    let json = serde_json::to_vec_pretty(&save_file)?;
    Ok(json)
}

/// Save game state to a string
pub fn save_to_string(state: &GameState) -> Result<String, SaveError> {
    let save_file = SaveFile::new(state.clone());
    let json = serde_json::to_string_pretty(&save_file)?;
    Ok(json)
}

/// Load game state from JSON bytes
pub fn load(data: &[u8]) -> Result<GameState, SaveError> {
    if data.is_empty() {
        return Err(SaveError::Empty);
    }

    let save_file: SaveFile = serde_json::from_slice(data)?;
    let migrated = migrate::migrate(save_file)?;
    Ok(migrated.state)
}

/// Load game state from a string
pub fn load_from_string(data: &str) -> Result<GameState, SaveError> {
    if data.is_empty() {
        return Err(SaveError::Empty);
    }

    let save_file: SaveFile = serde_json::from_str(data)?;
    let migrated = migrate::migrate(save_file)?;
    Ok(migrated.state)
}

/// Save game state to a file
pub fn save_to_file(state: &GameState, path: &std::path::Path) -> Result<(), SaveError> {
    let data = save(state)?;
    std::fs::write(path, data)?;
    Ok(())
}

/// Load game state from a file
pub fn load_from_file(path: &std::path::Path) -> Result<GameState, SaveError> {
    let data = std::fs::read(path)?;
    load(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;

    #[test]
    fn save_load_round_trip_preserves_state() {
        let engine = Engine::new(42);
        let original = engine.state.clone();

        let saved = save(&original).expect("Save should succeed");
        let loaded = load(&saved).expect("Load should succeed");

        assert_eq!(original.seed, loaded.seed);
        assert_eq!(original.turn, loaded.turn);
        assert_eq!(original.stars.len(), loaded.stars.len());
        assert_eq!(original.empires.len(), loaded.empires.len());
        assert_eq!(original.colonies.len(), loaded.colonies.len());
        assert_eq!(original.fleets.len(), loaded.fleets.len());
    }

    #[test]
    fn save_load_preserves_turn_and_seed() {
        let mut engine = Engine::new(12345);
        engine.state.turn = 42;

        let saved = save(&engine.state).expect("Save should succeed");
        let loaded = load(&saved).expect("Load should succeed");

        assert_eq!(loaded.seed, 12345);
        assert_eq!(loaded.turn, 42);
    }

    #[test]
    fn load_empty_bytes_returns_error() {
        let result = load(&[]);
        assert!(matches!(result, Err(SaveError::Empty)));
    }

    #[test]
    fn load_truncated_json_returns_error() {
        let result = load(b"{\"version\":1,\"state\":");
        assert!(result.is_err());
    }

    #[test]
    fn load_wrong_version_returns_unsupported_error() {
        // Create a valid state, serialize it, then modify the version
        let engine = Engine::new(0);
        let saved = save_to_string(&engine.state).expect("Save should work");

        // Parse the JSON, modify version, and re-serialize
        let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        json["version"] = serde_json::json!(999);
        let modified = serde_json::to_string(&json).unwrap();

        let result = load(modified.as_bytes());
        assert!(matches!(result, Err(SaveError::UnsupportedVersion { .. })));
    }

    #[test]
    fn save_to_string_and_load_from_string() {
        let engine = Engine::new(42);
        let original = engine.state.clone();

        let saved = save_to_string(&original).expect("Save should succeed");
        let loaded = load_from_string(&saved).expect("Load should succeed");

        assert_eq!(original.seed, loaded.seed);
        assert_eq!(original.turn, loaded.turn);
    }

    #[test]
    fn load_from_empty_string_returns_error() {
        let result = load_from_string("");
        assert!(matches!(result, Err(SaveError::Empty)));
    }

    #[test]
    fn save_to_file_and_load_from_file_round_trip() {
        let engine = Engine::new(42);
        let original = engine.state.clone();

        let dir = std::env::temp_dir();
        let path = dir.join("farspace_test_save.json");

        save_to_file(&original, &path).expect("save_to_file should succeed");
        let loaded = load_from_file(&path).expect("load_from_file should succeed");

        assert_eq!(original.seed, loaded.seed);
        assert_eq!(original.turn, loaded.turn);
        assert_eq!(original.stars.len(), loaded.stars.len());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_from_file_missing_file_returns_error() {
        let path = std::path::Path::new("/tmp/farspace_nonexistent_save_file.json");
        let result = load_from_file(path);
        assert!(matches!(result, Err(SaveError::Io(_))));
    }

    #[test]
    fn save_load_after_turn_advances_preserves_state() {
        let mut engine = Engine::new(77);
        engine.apply_turn(vec![game_core::Command::EndTurn]);
        engine.apply_turn(vec![game_core::Command::EndTurn]);

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert_eq!(engine.state.turn, loaded.turn);
        assert_eq!(engine.state.seed, loaded.seed);
        assert_eq!(engine.state.colonies.len(), loaded.colonies.len());
        assert_eq!(engine.state.fleets.len(), loaded.fleets.len());
    }

    #[test]
    fn save_load_preserves_research_state() {
        use game_core::TechId;

        let mut engine = Engine::new(42);

        // Select a tech and run enough turns to accumulate some progress
        engine.apply_turn(vec![game_core::Command::SetColonyFocus {
            colony: game_core::ColonyId(1),
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![game_core::Command::SelectResearch { tech: TechId(2) }]);
        engine.apply_turn(vec![game_core::Command::EndTurn]);

        let original = engine.state.clone();
        let saved = save(&original).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let orig_empire = original.empires.get(&original.player_empire).unwrap();
        let load_empire = loaded.empires.get(&loaded.player_empire).unwrap();

        assert_eq!(
            orig_empire.research.current_tech,
            load_empire.research.current_tech
        );
        assert_eq!(orig_empire.research.progress, load_empire.research.progress);
        assert_eq!(
            orig_empire.research.completed,
            load_empire.research.completed
        );
    }

    #[test]
    fn save_load_preserves_completed_techs() {
        use game_core::TechId;

        let mut engine = Engine::new(42);

        // Manually complete a tech to verify persistence
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .push(TechId(1));

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let load_empire = loaded.empires.get(&loaded.player_empire).unwrap();
        assert!(load_empire.research.completed.contains(&TechId(1)));
    }

    #[test]
    fn save_load_preserves_research_overflow() {
        use game_core::{ColonyId, Command, TechId};

        let mut engine = Engine::new(42);

        // Use research_pct=70 → 7 rp/turn; TechId(1) cost=50.
        // Completes on turn 8 (7*8=56 ≥ 50) with overflow = 6.
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: ColonyId(1),
            prod_pct: 30,
            research_pct: 70,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: TechId(1) }]);

        // Run 9 turns — enough to complete and leave overflow unchanged
        for _ in 0..9 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let original_empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .clone();
        assert!(
            original_empire.research.completed.contains(&TechId(1)),
            "tech must be completed before saving overflow"
        );
        assert!(
            original_empire.research.current_tech.is_none(),
            "no active tech expected after completion"
        );
        let overflow = original_empire.research.progress;
        assert!(overflow > 0, "overflow must be positive for this test");

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let loaded_empire = loaded.empires.get(&loaded.player_empire).unwrap();
        assert_eq!(
            loaded_empire.research.progress, overflow,
            "overflow in research.progress must survive save/load"
        );
        assert_eq!(
            loaded_empire.research.current_tech, original_empire.research.current_tech,
            "current_tech must survive save/load"
        );
        assert_eq!(
            loaded_empire.research.completed, original_empire.research.completed,
            "completed techs must survive save/load"
        );
    }

    #[test]
    fn save_load_preserves_explored_stars() {
        use game_core::{Command, FleetId};

        let mut engine = Engine::new(42);

        // Find an unexplored star and dispatch a scout, then advance turns until it arrives
        let dest = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("There should be unexplored stars");

        engine.apply_turn(vec![Command::SendScout {
            fleet: FleetId(1),
            destination: dest,
        }]);

        // Advance enough turns for the scout to arrive (SCOUT_TRAVEL_TURNS = 3)
        for _ in 0..3 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        assert!(
            engine.state.explored_stars.contains(&dest),
            "Star should be explored before saving"
        );

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert!(
            loaded.explored_stars.contains(&dest),
            "Explored stars must survive save/load round-trip"
        );
        assert_eq!(
            engine.state.explored_stars.len(),
            loaded.explored_stars.len(),
            "explored_stars count must match after round-trip"
        );
    }

    #[test]
    fn save_load_preserves_active_scout_mission() {
        use game_core::{Command, FleetId, ScoutMission};

        let mut engine = Engine::new(42);

        let dest = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        engine.apply_turn(vec![Command::SendScout {
            fleet: FleetId(1),
            destination: dest,
        }]);

        // Don't advance turns — mission should still be active
        assert!(!engine.state.scout_missions.is_empty());

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert!(
            !loaded.scout_missions.is_empty(),
            "Active scout mission must survive save/load round-trip"
        );

        let mission: &ScoutMission = loaded.scout_missions.get(&FleetId(1)).unwrap();
        assert_eq!(mission.destination, dest);
    }

    #[test]
    fn save_load_preserves_active_fleet_mission() {
        use game_core::{Command, FleetId, FleetMission};

        let mut engine = Engine::new(42);

        // Need an explored star other than home to move to
        let fleet_id = FleetId(1);
        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        let dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != initial_location)
            .expect("Need explored star other than home");

        engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: dest,
        }]);

        // Mission should be in-flight
        assert!(!engine.state.fleet_missions.is_empty());

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert!(
            !loaded.fleet_missions.is_empty(),
            "Active fleet mission must survive save/load round-trip"
        );

        let mission: &FleetMission = loaded.fleet_missions.get(&fleet_id).unwrap();
        assert_eq!(mission.destination, dest);
    }

    #[test]
    fn save_load_preserves_colonized_planet_and_fleet_changes() {
        use game_core::{Command, FleetKind, OrbitalStructureType};

        let mut engine = Engine::new(42);

        // Build a colonizer fleet
        let colony_id = game_core::ColonyId(1);
        // Inject Shipyard so Colony Ship can be queued
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .orbital_installations
            .push(OrbitalStructureType::Shipyard);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: game_core::BuildItem::Colony,
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

        // Move colonizer to target
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

        // Colonize
        engine.apply_turn(vec![Command::Colonize {
            fleet: colonizer_id,
            star: target,
            planet_index: planet_idx,
        }]);

        let colonies_after = engine.state.colonies.len();
        let fleets_after = engine.state.fleets.len();
        let new_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.star == target)
            .map(|c| c.id)
            .expect("New colony must exist");

        // Save and load
        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        // Colony survived
        assert_eq!(loaded.colonies.len(), colonies_after);
        assert!(
            loaded.colonies.contains_key(&new_colony_id),
            "New colony must survive save/load"
        );
        let loaded_colony = loaded.colonies.get(&new_colony_id).unwrap();
        assert_eq!(loaded_colony.star, target);
        assert_eq!(loaded_colony.planet_index, planet_idx);
        assert_eq!(loaded_colony.owner, engine.state.player_empire);

        // Colonizer fleet consumed
        assert_eq!(loaded.fleets.len(), fleets_after);
        assert!(
            !loaded.fleets.contains_key(&colonizer_id),
            "Colonizer fleet must be consumed after save/load"
        );

        // Planet references the colony
        let planet = &loaded.stars.get(&target).unwrap().planets[planet_idx];
        assert_eq!(planet.colony, Some(new_colony_id));
    }

    #[test]
    fn save_load_preserves_fleet_kind() {
        use game_core::{Command, FleetKind, OrbitalStructureType};

        let mut engine = Engine::new(42);
        let colony_id = game_core::ColonyId(1);

        // Inject Shipyard so Colony Ship can be queued
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .orbital_installations
            .push(OrbitalStructureType::Shipyard);

        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: game_core::BuildItem::Colony,
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

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let loaded_fleet = loaded.fleets.get(&colonizer_id).expect("Fleet must exist");
        assert_eq!(
            loaded_fleet.kind,
            FleetKind::Colonizer,
            "FleetKind::Colonizer must survive save/load"
        );
    }

    #[test]
    fn save_load_preserves_empire_food() {
        let mut engine = Engine::new(42);

        // Manually set a non-default food value so we can verify persistence
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .food = 42;

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let loaded_empire = loaded.empires.get(&loaded.player_empire).unwrap();
        assert_eq!(
            loaded_empire.food, 42,
            "Empire food must survive save/load round-trip"
        );
    }

    #[test]
    fn save_load_food_default_on_old_save() {
        // Simulate a v4 save (no food field) → migration sets food = 0 via serde default
        let engine = Engine::new(42);
        let saved_str = save_to_string(&engine.state).expect("save should succeed");

        // Patch version to 4 to simulate an old save
        let mut json: serde_json::Value = serde_json::from_str(&saved_str).unwrap();
        json["version"] = serde_json::json!(4);
        // Remove food from empires to simulate old format
        if let Some(empires) = json["state"]["empires"].as_object_mut() {
            for emp in empires.values_mut() {
                emp.as_object_mut().map(|o| o.remove("food"));
            }
        }
        let patched = serde_json::to_string(&json).unwrap();

        let loaded = load_from_string(&patched).expect("v4 migration should succeed");
        let empire = loaded.empires.get(&loaded.player_empire).unwrap();
        assert_eq!(empire.food, 0, "Missing food field should default to 0");
    }

    #[test]
    fn save_load_preserves_ai_empire_and_explored_stars() {
        let mut engine = Engine::new(42);
        // Advance a few turns so the AI makes decisions and explores stars
        for _ in 0..5 {
            engine.apply_turn(vec![game_core::Command::EndTurn]);
        }

        let original = engine.state.clone();
        let saved = save(&original).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert_eq!(
            original.ai_empire, loaded.ai_empire,
            "ai_empire must survive round-trip"
        );
        assert_eq!(
            original.ai_explored_stars, loaded.ai_explored_stars,
            "ai_explored_stars must survive round-trip"
        );
        // Both player + AI empires must be present
        assert_eq!(original.empires.len(), loaded.empires.len());
    }

    #[test]
    fn migrate_v5_defaults_ai_fields() {
        // Simulate a v5 save without ai_empire / ai_explored_stars fields
        let engine = Engine::new(42);
        let saved_str = save_to_string(&engine.state).expect("save should succeed");

        let mut json: serde_json::Value = serde_json::from_str(&saved_str).unwrap();
        json["version"] = serde_json::json!(5);
        // Remove AI fields to simulate a v5 save
        if let Some(state) = json["state"].as_object_mut() {
            state.remove("ai_empire");
            state.remove("ai_explored_stars");
        }
        let patched = serde_json::to_string(&json).unwrap();

        let loaded = load_from_string(&patched).expect("v5 migration should succeed");
        // ai_empire should default to None
        assert!(
            loaded.ai_empire.is_none(),
            "ai_empire should default to None from v5"
        );
        // ai_explored_stars should default to empty
        assert!(
            loaded.ai_explored_stars.is_empty(),
            "ai_explored_stars should default to empty from v5"
        );
    }

    /// Save/load round-trip preserves diplomacy contact state.
    #[test]
    fn save_load_preserves_diplomacy_contact_state() {
        use game_core::RelationshipStatus;

        let mut engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");

        // Manually set contact
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Contacted);

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert_eq!(
            loaded.diplomacy.get(&ai_id).copied(),
            Some(RelationshipStatus::Contacted),
            "Diplomacy contact status must survive save/load"
        );
    }

    /// v6 saves (without diplomacy field) load correctly with diplomacy defaulting to empty.
    #[test]
    fn load_v6_save_defaults_diplomacy_to_empty() {
        let engine = Engine::new(42);
        let saved = save_to_string(&engine.state).expect("save should succeed");

        // Patch version down to 6 and strip the diplomacy field
        let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        json["version"] = serde_json::json!(6);
        if let Some(state) = json["state"].as_object_mut() {
            state.remove("diplomacy");
        }
        let patched = serde_json::to_string(&json).unwrap();

        let loaded = load_from_string(&patched).expect("v6 migration should succeed");
        assert!(
            loaded.diplomacy.is_empty(),
            "diplomacy should default to empty from v6"
        );
    }

    /// Save/load round-trip preserves damaged fleet state (combat damage persists).
    #[test]
    fn save_load_preserves_damaged_fleet() {
        use game_core::{Fleet, FleetId, FleetKind};

        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let star_id = engine.state.empires.get(&player).unwrap().home_star;

        // Insert a fleet with reduced integrity (simulating post-combat damage)
        let damaged_fid = FleetId(50);
        engine.state.fleets.insert(
            damaged_fid,
            Fleet {
                id: damaged_fid,
                owner: player,
                location: star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 5,
                integrity: 42, // damaged
            },
        );

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let fleet = loaded
            .fleets
            .get(&damaged_fid)
            .expect("Damaged fleet must be present after load");
        assert_eq!(
            fleet.integrity, 42,
            "Fleet integrity must survive save/load"
        );
        assert_eq!(fleet.strength, 5, "Fleet strength must survive save/load");
    }

    /// v7 saves (without strength/integrity fields) load correctly using serde defaults.
    #[test]
    fn load_v7_save_defaults_strength_and_integrity() {
        let engine = Engine::new(42);
        let saved = save_to_string(&engine.state).expect("save should succeed");

        // Patch version down to 7 and strip strength/integrity from all fleets
        let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        json["version"] = serde_json::json!(7);
        if let Some(fleets) = json["state"]["fleets"].as_object_mut() {
            for fleet in fleets.values_mut() {
                if let Some(obj) = fleet.as_object_mut() {
                    obj.remove("strength");
                    obj.remove("integrity");
                }
            }
        }
        let patched = serde_json::to_string(&json).unwrap();

        let loaded = load_from_string(&patched).expect("v7 migration should succeed");
        // All fleets should have default strength=1 and integrity=100
        for fleet in loaded.fleets.values() {
            assert_eq!(fleet.strength, 1, "Fleet strength should default to 1");
            assert_eq!(
                fleet.integrity, 100,
                "Fleet integrity should default to 100"
            );
        }
    }

    /// Garbled JSON that is syntactically valid but not a SaveFile returns an error.
    #[test]
    fn load_garbled_json_returns_error() {
        // Valid JSON but doesn't match SaveFile schema at all
        let garbled = r#"{"completely": "wrong", "structure": 42}"#;
        let result = load_from_string(garbled);
        assert!(
            result.is_err(),
            "Loading JSON that doesn't match SaveFile must return an error"
        );
    }

    /// `null` as the JSON value returns an error.
    #[test]
    fn load_null_json_returns_error() {
        let result = load_from_string("null");
        assert!(result.is_err(), "Loading 'null' JSON must return an error");
    }

    /// A save file with an array at the top level returns an error.
    #[test]
    fn load_json_array_returns_error() {
        let result = load_from_string("[1, 2, 3]");
        assert!(
            result.is_err(),
            "Loading a JSON array instead of a SaveFile must return an error"
        );
    }

    /// Continue playing after a save/load round trip and verify results match
    /// playing straight through without saving.
    #[test]
    fn continue_play_after_save_load_is_deterministic() {
        use game_core::Command;

        // Arbitrary seed that differs from the default (42) used in most other tests,
        // to ensure the round-trip is exercised on an independently-generated galaxy.
        let seed = 31_337u64;

        // Baseline: play 3 turns, then 2 more
        let baseline_events = {
            let mut engine = Engine::new(seed);
            for _ in 0..3 {
                engine.apply_turn(vec![Command::EndTurn]);
            }
            let mut evts = Vec::new();
            for _ in 0..2 {
                evts.push(engine.apply_turn(vec![Command::EndTurn]));
            }
            evts
        };

        // Round-trip: play 3 turns, save, load, then play 2 more
        let after_load_events = {
            let mut engine = Engine::new(seed);
            for _ in 0..3 {
                engine.apply_turn(vec![Command::EndTurn]);
            }

            let saved = save(&engine.state).expect("save should succeed");
            let loaded_state = load(&saved).expect("load should succeed");

            let mut engine2 = Engine::from_state(loaded_state);
            let mut evts = Vec::new();
            for _ in 0..2 {
                evts.push(engine2.apply_turn(vec![Command::EndTurn]));
            }
            evts
        };

        assert_eq!(
            baseline_events, after_load_events,
            "Events after save/load round-trip must match straight-through play"
        );
    }

    /// Save/load round trip preserves the full set of buildings in each colony.
    #[test]
    fn save_load_preserves_colony_buildings() {
        use game_core::{BuildItem, ColonyId, Command};

        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Build a ScienceNexus (cost 100) with 100% production focus
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(game_core::BuildingType::ScienceNexus),
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);
        for _ in 0..11 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let has_nexus = engine.state.colonies[&colony_id]
            .buildings
            .contains(&game_core::BuildingType::ScienceNexus);
        assert!(has_nexus, "ScienceNexus must be built before saving");

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert!(
            loaded.colonies[&colony_id]
                .buildings
                .contains(&game_core::BuildingType::ScienceNexus),
            "ScienceNexus must survive save/load round-trip"
        );
    }

    #[test]
    fn save_load_preserves_mixed_production_queue() {
        use game_core::{BuildItem, ColonyId, OrbitalStructureType, ShipDesignId};

        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        {
            let colony = engine
                .state
                .colonies
                .get_mut(&colony_id)
                .expect("player colony must exist");
            colony.build_queue = vec![
                BuildItem::SurfaceStructure(game_core::BuildingType::ScienceNexus),
                BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard),
                BuildItem::Ship(ShipDesignId::SCOUT),
            ];
            colony.accumulated_production = 37;
        }

        let saved = save(&engine.state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let original_colony = engine.state.colonies.get(&colony_id).unwrap();
        let loaded_colony = loaded.colonies.get(&colony_id).unwrap();
        assert_eq!(
            loaded_colony.build_queue, original_colony.build_queue,
            "mixed production queue must survive save/load"
        );
        assert_eq!(
            loaded_colony.accumulated_production, original_colony.accumulated_production,
            "accumulated production must survive save/load"
        );
    }

    /// Saving state with no explored stars, then loading, preserves the empty set.
    #[test]
    fn save_load_empty_explored_stars_is_valid() {
        let engine = Engine::new(42);
        let mut state = engine.state.clone();
        // Manually clear explored stars (unusual but should not corrupt the save)
        state.explored_stars.clear();

        let saved = save(&state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        assert!(
            loaded.explored_stars.is_empty(),
            "Empty explored_stars must survive save/load"
        );
    }

    /// Planet classes and infrastructure tracking are preserved in save/load round-trip.
    #[test]
    fn save_load_preserves_planet_classes_and_infrastructure() {
        let engine = Engine::new(42);
        let original = engine.state.clone();

        // Verify original has planet classes
        let mut original_classes = Vec::new();
        for star in original.stars.values() {
            for planet in &star.planets {
                original_classes.push(planet.class);
            }
        }
        assert!(!original_classes.is_empty(), "Galaxy should have planets");

        // Verify original colonies have infrastructure tracking fields
        for colony in original.colonies.values() {
            // These fields should exist and be serializable
            let _surface = &colony.surface_installations;
            let _orbital = &colony.orbital_installations;
        }

        // Save and load
        let saved = save(&original).expect("Save should succeed");
        let loaded = load(&saved).expect("Load should succeed");

        // Verify planet classes are preserved
        let mut loaded_classes = Vec::new();
        for star in loaded.stars.values() {
            for planet in &star.planets {
                loaded_classes.push(planet.class);
            }
        }
        assert_eq!(
            original_classes, loaded_classes,
            "Planet classes must be preserved in save/load"
        );

        // Verify infrastructure tracking is preserved
        for (col_id, colony) in &loaded.colonies {
            let original_colony = &original.colonies[col_id];
            assert_eq!(
                colony.surface_installations, original_colony.surface_installations,
                "Surface installations must be preserved for colony {:?}",
                col_id
            );
            assert_eq!(
                colony.orbital_installations, original_colony.orbital_installations,
                "Orbital installations must be preserved for colony {:?}",
                col_id
            );
        }
    }

    /// Orbital installations (including Shipyard) survive a save/load round-trip.
    #[test]
    fn save_load_preserves_orbital_installations_with_shipyard() {
        use game_core::OrbitalStructureType;

        let engine = Engine::new(42);
        let mut state = engine.state.clone();

        // Directly inject a Shipyard into the first player colony
        let colony_id = state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == state.player_empire)
            .map(|(id, _)| *id)
            .expect("player colony must exist");
        state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .orbital_installations
            .push(OrbitalStructureType::Shipyard);

        let saved = save(&state).expect("save should succeed");
        let loaded = load(&saved).expect("load should succeed");

        let loaded_colony = &loaded.colonies[&colony_id];
        assert!(
            loaded_colony
                .orbital_installations
                .contains(&OrbitalStructureType::Shipyard),
            "Shipyard orbital installation must survive save/load round-trip"
        );
    }
}
