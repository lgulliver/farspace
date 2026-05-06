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
}
