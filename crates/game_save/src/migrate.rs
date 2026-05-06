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
            let mut state = save.state;
            let home_stars: Vec<_> = state.empires.values().map(|e| e.home_star).collect();
            for star_id in home_stars {
                state.explored_stars.insert(star_id);
            }
            // Continue migrating v2 → v3
            migrate(SaveFile { version: 2, state })
        }
        2 => {
            // v2 -> v3: fleet_missions field added; defaults to empty via serde(default).
            // Nothing to populate -- just bump the version and continue.
            migrate(SaveFile {
                version: 3,
                state: save.state,
            })
        }
        3 => {
            // v3 -> v4: FleetKind added to Fleet (serde default = Scout) and
            // habitable added to Planet (serde default = true).
            // Nothing to populate — just bump the version.
            migrate(SaveFile {
                version: 4,
                state: save.state,
            })
        }
        4 => {
            // v4 -> v5: Empire.food field added (serde default = 0).
            // Nothing to populate — just bump the version.
            migrate(SaveFile {
                version: 5,
                state: save.state,
            })
        }
        5 => {
            // v5 -> v6: GameState.ai_empire (Option<EmpireId>, default None) and
            // GameState.ai_explored_stars (BTreeSet<StarId>, default empty) added.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            Ok(SaveFile {
                version: CURRENT_VERSION,
                state: save.state,
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
    fn migrate_v1_to_current_populates_explored_stars() {
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
                food: 0,
            },
        );
        // explored_stars starts empty
        assert!(state.explored_stars.is_empty());

        let v1_save = SaveFile { version: 1, state };
        let migrated = migrate(v1_save).expect("v1 migration should succeed");

        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(
            migrated.state.explored_stars.contains(&home_star),
            "Home star should be explored after v1→current migration"
        );
    }

    #[test]
    fn migrate_v2_to_v3_succeeds() {
        let state = GameState::default();
        let v2_save = SaveFile { version: 2, state };
        let migrated = migrate(v2_save).expect("v2 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(migrated.state.fleet_missions.is_empty());
    }

    #[test]
    fn migrate_v3_to_v4_succeeds() {
        let state = GameState::default();
        let v3_save = SaveFile { version: 3, state };
        let migrated = migrate(v3_save).expect("v3 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v4_to_v5_succeeds() {
        let state = GameState::default();
        let v4_save = SaveFile { version: 4, state };
        let migrated = migrate(v4_save).expect("v4 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v5_to_v6_succeeds() {
        let state = GameState::default();
        let v5_save = SaveFile { version: 5, state };
        let migrated = migrate(v5_save).expect("v5 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // ai_empire defaults to None
        assert!(migrated.state.ai_empire.is_none());
        // ai_explored_stars defaults to empty
        assert!(migrated.state.ai_explored_stars.is_empty());
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
