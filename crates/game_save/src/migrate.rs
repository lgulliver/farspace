//! Save file migration

use crate::schema::{SaveFile, CURRENT_VERSION};
use crate::SaveError;

/// Migrate a save file to the current version
pub fn migrate(save: SaveFile) -> Result<SaveFile, SaveError> {
    match save.version {
        CURRENT_VERSION => Ok(save),
        v if v > CURRENT_VERSION => Err(SaveError::UnsupportedVersion {
            found: v,
            supported: CURRENT_VERSION,
        }),
        1 => {
            // v1 → v2: populate explored_stars with each empire's home star.
            // explored_stars and scout_missions default to empty via serde(default),
            // so we just need to seed the exploration with home stars.
            let mut state = save.state;
            let home_stars: Vec<_> = state.empires.values().map(|e| e.home_star).collect();
            for star_id in home_stars {
                state.explored_stars.insert(star_id);
            }
            Ok(SaveFile {
                version: CURRENT_VERSION,
                state,
            })
        }
        _ => Err(SaveError::UnsupportedVersion {
            found: save.version,
            supported: CURRENT_VERSION,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::state::GameState;

    #[test]
    fn migrate_current_version() {
        let save = SaveFile::new(GameState::default());
        let result = migrate(save);
        assert!(result.is_ok());
    }

    #[test]
    fn migrate_future_version_fails() {
        let save = SaveFile {
            version: CURRENT_VERSION + 1,
            state: GameState::default(),
        };
        let result = migrate(save);
        assert!(matches!(result, Err(SaveError::UnsupportedVersion { .. })));
    }

    #[test]
    fn migrate_v1_to_v2_populates_explored_stars() {
        use game_core::{Empire, EmpireId, StarId};

        let mut state = GameState::default();
        let home_star = StarId(7);
        state.empires.insert(
            EmpireId(1),
            Empire {
                id: EmpireId(1),
                name: "Test".to_string(),
                credits: 0,
                research_points: 0,
                home_star,
                research: Default::default(),
            },
        );
        // explored_stars starts empty
        assert!(state.explored_stars.is_empty());

        let v1_save = SaveFile { version: 1, state };
        let migrated = migrate(v1_save).expect("v1 migration should succeed");

        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(
            migrated.state.explored_stars.contains(&home_star),
            "Home star should be explored after v1→v2 migration"
        );
    }

    #[test]
    fn migrate_old_unsupported_version_fails() {
        let save = SaveFile {
            version: 0,
            state: GameState::default(),
        };
        let result = migrate(save);
        assert!(matches!(result, Err(SaveError::UnsupportedVersion { .. })));
    }
}
