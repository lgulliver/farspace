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
}
