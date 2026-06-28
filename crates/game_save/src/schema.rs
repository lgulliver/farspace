//! Save file schema

use game_core::state::GameState;
use serde::{Deserialize, Serialize};

/// Current save file version
pub const CURRENT_VERSION: u32 = 42;

/// Metadata embedded in every save file.
///
/// All fields use `#[serde(default)]` so that older saves without a `metadata`
/// section (or with only some fields) still deserialise safely.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SaveMetadata {
    /// Schema version at the time of saving (mirrors `SaveFile::version`).
    #[serde(default)]
    pub schema_version: u32,
    /// Game binary version at the time of saving (`CARGO_PKG_VERSION`).
    /// `None` for saves that were migrated from an older schema without this field.
    #[serde(default)]
    pub game_version: Option<String>,
    /// Turn number when the save was created.
    #[serde(default)]
    pub created_turn: u32,
    /// Galaxy seed used for this game.
    #[serde(default)]
    pub seed: u64,
    /// Player empire name, recorded so save listings can be built without
    /// deserialising the full game state. `None` on saves written before
    /// these fields existed; readers fall back to a full load.
    #[serde(default)]
    pub empire_name: Option<String>,
    /// Galaxy size label (e.g. "Medium") from the scenario, if any.
    #[serde(default)]
    pub galaxy_size: Option<String>,
    /// Number of AI empires from the scenario, if any.
    #[serde(default)]
    pub ai_empires: Option<u8>,
    /// Difficulty label from the scenario, if any.
    #[serde(default)]
    pub difficulty: Option<String>,
}

/// Save file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    /// Per-save metadata (absent in pre-v21 saves; defaults to `SaveMetadata::default()`).
    #[serde(default)]
    pub metadata: SaveMetadata,
    pub state: GameState,
}

impl SaveFile {
    /// Create a new save file from game state, populating metadata automatically.
    pub fn new(state: GameState) -> Self {
        let empire_name = state
            .empires
            .get(&state.player_empire)
            .map(|e| e.name.clone());
        let (galaxy_size, ai_empires, difficulty) = match &state.scenario {
            Some(scenario) => (
                Some(scenario.galaxy_size.label().to_string()),
                Some(scenario.ai_empire_count),
                Some(crate::difficulty_label(scenario.difficulty).to_string()),
            ),
            None => (None, None, None),
        };
        let metadata = SaveMetadata {
            schema_version: CURRENT_VERSION,
            game_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            created_turn: state.turn,
            seed: state.seed,
            empire_name,
            galaxy_size,
            ai_empires,
            difficulty,
        };
        SaveFile {
            version: CURRENT_VERSION,
            metadata,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_file_creation() {
        let state = GameState::default();
        let save = SaveFile::new(state);
        assert_eq!(save.version, CURRENT_VERSION);
    }

    #[test]
    fn save_file_metadata_populated_on_new() {
        use game_core::Engine;
        let engine = Engine::new(999);
        let save = SaveFile::new(engine.state.clone());
        assert_eq!(save.metadata.schema_version, CURRENT_VERSION);
        assert_eq!(save.metadata.seed, 999);
        assert_eq!(save.metadata.created_turn, engine.state.turn);
        assert!(
            save.metadata.game_version.is_some(),
            "game_version should be set"
        );
    }

    #[test]
    fn save_metadata_default_is_zero() {
        let m = SaveMetadata::default();
        assert_eq!(m.schema_version, 0);
        assert_eq!(m.game_version, None);
        assert_eq!(m.created_turn, 0);
        assert_eq!(m.seed, 0);
    }
}
