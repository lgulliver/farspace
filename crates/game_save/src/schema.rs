//! Save file schema

use game_core::state::GameState;
use serde::{Deserialize, Serialize};

/// Current save file version
pub const CURRENT_VERSION: u32 = 6;

/// Save file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub state: GameState,
}

impl SaveFile {
    /// Create a new save file from game state
    pub fn new(state: GameState) -> Self {
        SaveFile {
            version: CURRENT_VERSION,
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
}
