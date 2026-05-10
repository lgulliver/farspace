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
            // Note: saves migrated from v5 will have ai_empire=None until a new game is
            // started, meaning no AI opponent will be active for existing saves.
            migrate(SaveFile {
                version: 6,
                state: save.state,
            })
        }
        6 => {
            // v6 -> v7: GameState.diplomacy (BTreeMap<EmpireId, RelationshipStatus>, default empty)
            // added.  Relies on serde default — nothing to populate explicitly.
            // Existing saves will have diplomacy=empty (all empires start Unknown).
            migrate(SaveFile {
                version: 7,
                state: save.state,
            })
        }
        7 => {
            // v7 -> v8: Fleet.strength (u32, default 1) and Fleet.integrity (u32, default 100)
            // added for combat auto-resolve.  Both rely on serde defaults — nothing to populate
            // explicitly.  Existing fleets will have full health and base strength on load.
            migrate(SaveFile {
                version: 8,
                state: save.state,
            })
        }
        8 => {
            // v8 -> v9: Planet.class (PlanetClass, default Terran) and Colony.surface_installations /
            // orbital_installations (Vec<BuildingType>, default empty) added for infrastructure system.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            // Existing planets will default to Terran class; colonies start with no installed infrastructure.
            migrate(SaveFile {
                version: 9,
                state: save.state,
            })
        }
        9 => {
            // v9 -> v10: OrbitalStructureType enum added; Colony.orbital_installations type changed
            // from Vec<BuildingType> to Vec<OrbitalStructureType>.
            // Since orbital_installations was never populated by the engine before v10, all v9 saves
            // have orbital_installations: [] — an empty array deserialises safely to any Vec<T>.
            // BuildItem::OrbitalStructure variant added; Shipyard added as OrbitalStructureType::Shipyard.
            // TechId(7) "Orbital Engineering" added to all_techs().
            // Nothing to populate explicitly — just bump the version.
            migrate(SaveFile {
                version: 10,
                state: save.state,
            })
        }
        10 => {
            // v10 -> v11: Colony.stability: u8 added (serde default = 100).
            // All existing colonies default to neutral stability.
            // Nothing to populate explicitly — just bump the version.
            migrate(SaveFile {
                version: 11,
                state: save.state,
            })
        }
        11 => {
            // v11 -> v12: Colony.role: ColonyRole added (serde default = Balanced).
            // All existing colonies default to Balanced role (no modifiers).
            // Nothing to populate explicitly — continue to v13 migration.
            migrate(SaveFile {
                version: 12,
                state: save.state,
            })
        }
        12 => {
            // v12 -> v13: Planet.surveyed: bool added (serde default = false).
            // Existing planets default to unsurveyed via serde default.
            migrate(SaveFile {
                version: 13,
                state: save.state,
            })
        }
        13 => {
            // v13 -> v14: Sector and SectorId added; GameState.sectors (BTreeMap<SectorId, Sector>)
            // and Star.sector (SectorId) added.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            // v13 saves will have empty sectors and SectorId(0) on stars until a new game is started.
            migrate(SaveFile {
                version: 14,
                state: save.state,
            })
        }
        14 => {
            // v14 -> v15: GameState.survey_missions added (serde default = empty).
            // Science ships are encoded as FleetKind::Science, which old saves can load
            // as long as the new field defaults to empty.
            migrate(SaveFile {
                version: 15,
                state: save.state,
            })
        }
        15 => {
            // v15 -> v16: ScoutMission.origin, ScoutMission.total_duration,
            // FleetMission.origin, and FleetMission.total_duration added.
            // All four fields carry serde defaults (StarId(0) and 0 respectively)
            // so existing missions deserialise safely.  Animation will show no
            // interpolation for old in-flight missions, which is an acceptable
            // visual-only trade-off.
            migrate(SaveFile {
                version: 16,
                state: save.state,
            })
        }
        16 => {
            // v16 -> v17: GameState.hyperspace_lanes and
            // GameState.known_hyperspace_lanes added.
            //
            // Populate deterministic lane topology from seed + sectors + stars and
            // derive player-known lanes from current explored stars.
            let mut state = save.state;
            let sectors: Vec<_> = state.sectors.values().cloned().collect();
            let stars: Vec<_> = state.stars.values().cloned().collect();
            state.hyperspace_lanes =
                game_core::galaxy::generate_hyperspace_lanes(state.seed, &sectors, &stars)
                    .into_iter()
                    .collect();
            state.known_hyperspace_lanes = state
                .hyperspace_lanes
                .iter()
                .copied()
                .filter(|lane| {
                    state.explored_stars.contains(&lane.a) && state.explored_stars.contains(&lane.b)
                })
                .collect();

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
    fn migrate_v6_to_v7_succeeds() {
        let state = GameState::default();
        let v6_save = SaveFile { version: 6, state };
        let migrated = migrate(v6_save).expect("v6 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // diplomacy defaults to empty
        assert!(migrated.state.diplomacy.is_empty());
    }

    #[test]
    fn migrate_v7_to_v8_succeeds() {
        let state = GameState::default();
        let v7_save = SaveFile { version: 7, state };
        let migrated = migrate(v7_save).expect("v7 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v8_to_v9_succeeds() {
        let state = GameState::default();
        let v8_save = SaveFile { version: 8, state };
        let migrated = migrate(v8_save).expect("v8 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // orbital_installations defaults to empty on all colonies
        for colony in migrated.state.colonies.values() {
            assert!(
                colony.orbital_installations.is_empty(),
                "orbital_installations must default to empty"
            );
        }
    }

    #[test]
    fn migrate_v9_to_v10_succeeds() {
        let state = GameState::default();
        let v9_save = SaveFile { version: 9, state };
        let migrated = migrate(v9_save).expect("v9 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v10_to_v11_succeeds_and_stability_defaults_to_100() {
        // Build a v10 save JSON that omits the `stability` field on a colony,
        // simulating a real save written before v11 introduced the field.
        // serde must apply the default of 100 when the field is absent.
        let v10_json = r#"
        {
            "version": 10,
            "state": {
                "seed": 42,
                "turn": 1,
                "stars": {},
                "empires": {},
                "colonies": {
                    "1": {
                        "id": 1,
                        "star": 1,
                        "planet_index": 0,
                        "owner": 1,
                        "population": 5,
                        "production": 5,
                        "prod_pct": 50,
                        "research_pct": 50,
                        "build_queue": [],
                        "accumulated_production": 0,
                        "buildings": [],
                        "surface_installations": [],
                        "orbital_installations": []
                    }
                },
                "fleets": {},
                "player_empire": 1,
                "rng": {"seed": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "stream": 0, "word_pos": 0},
                "event_log": [],
                "next_colony_id": 2,
                "next_fleet_id": 1
            }
        }"#;

        let save: SaveFile =
            serde_json::from_str(v10_json).expect("v10 JSON should deserialize successfully");
        assert_eq!(save.version, 10, "parsed version should be 10");

        let migrated = migrate(save).expect("v10 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);

        // The colony lacked `stability` in the JSON — serde default must apply 100.
        let colony = migrated
            .state
            .colonies
            .values()
            .next()
            .expect("state should contain the test colony");
        assert_eq!(
            colony.stability, 100,
            "colony migrated from v10 (no stability field) must default to neutral stability 100"
        );
    }

    #[test]
    fn migrate_v11_to_v12_succeeds_and_role_defaults_to_balanced() {
        // Build a v11 save JSON that omits the `role` field on a colony,
        // simulating a real save written before v12 introduced the field.
        // serde must apply the default of Balanced when the field is absent.
        let v11_json = r#"
        {
            "version": 11,
            "state": {
                "seed": 42,
                "turn": 1,
                "stars": {},
                "empires": {},
                "colonies": {
                    "1": {
                        "id": 1,
                        "star": 1,
                        "planet_index": 0,
                        "owner": 1,
                        "population": 5,
                        "production": 5,
                        "prod_pct": 50,
                        "research_pct": 50,
                        "build_queue": [],
                        "accumulated_production": 0,
                        "buildings": [],
                        "surface_installations": [],
                        "orbital_installations": [],
                        "stability": 100
                    }
                },
                "fleets": {},
                "player_empire": 1,
                "rng": {"seed": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "stream": 0, "word_pos": 0},
                "event_log": [],
                "next_colony_id": 2,
                "next_fleet_id": 1
            }
        }"#;

        let save: SaveFile =
            serde_json::from_str(v11_json).expect("v11 JSON should deserialize successfully");
        assert_eq!(save.version, 11, "parsed version should be 11");

        let migrated = migrate(save).expect("v11 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);

        // The colony lacked `role` in the JSON — serde default must apply Balanced.
        use game_core::ColonyRole;
        let colony = migrated
            .state
            .colonies
            .values()
            .next()
            .expect("state should contain the test colony");
        assert_eq!(
            colony.role,
            ColonyRole::Balanced,
            "colony migrated from v11 (no role field) must default to Balanced"
        );
    }

    #[test]
    fn migrate_v12_to_v13_defaults_planets_to_unsurveyed() {
        let v12_json = r#"
        {
            "version": 12,
            "state": {
                "seed": 42,
                "turn": 1,
                "stars": {
                    "1": {
                        "id": 1,
                        "name": "Test",
                        "x": 0,
                        "y": 0,
                        "spectral_class": "G",
                        "planets": [
                            {
                                "name": "Test I",
                                "size": "Medium",
                                "class": "Terran",
                                "colony": null,
                                "habitable": true
                            }
                        ]
                    }
                },
                "empires": {},
                "colonies": {},
                "fleets": {},
                "player_empire": 1,
                "rng": {"seed": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "stream": 0, "word_pos": 0},
                "event_log": [],
                "next_colony_id": 2,
                "next_fleet_id": 1
            }
        }"#;

        let save: SaveFile =
            serde_json::from_str(v12_json).expect("v12 JSON should deserialize successfully");
        let migrated = migrate(save).expect("v12 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        let surveyed = migrated
            .state
            .stars
            .values()
            .flat_map(|s| s.planets.iter())
            .all(|p| !p.surveyed);
        assert!(surveyed, "v12 planets should default to unsurveyed in v13");
    }

    #[test]
    fn migrate_v13_to_v14_succeeds() {
        let state = GameState::default();
        let v13_save = SaveFile { version: 13, state };
        let migrated = migrate(v13_save).expect("v13 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v14_to_v15_defaults_survey_missions_to_empty() {
        let state = GameState::default();
        let v14_save = SaveFile { version: 14, state };
        let migrated = migrate(v14_save).expect("v14 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(migrated.state.survey_missions.is_empty());
    }

    #[test]
    fn migrate_v15_to_v16_preserves_missions_with_defaults() {
        // v15 saves have ScoutMission/FleetMission without origin/total_duration.
        // Migration should succeed; new fields default safely via serde.
        let state = GameState::default();
        let v15_save = SaveFile { version: 15, state };
        let migrated = migrate(v15_save).expect("v15 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // Both mission maps empty in default state — just check it round-trips
        assert!(migrated.state.scout_missions.is_empty());
        assert!(migrated.state.fleet_missions.is_empty());
    }

    #[test]
    fn migrate_v16_to_v17_populates_hyperspace_lanes() {
        let mut state = game_core::Engine::new(42).state;
        state.hyperspace_lanes.clear();
        state.known_hyperspace_lanes.clear();

        let v16_save = SaveFile { version: 16, state };
        let migrated = migrate(v16_save).expect("v16 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(
            !migrated.state.hyperspace_lanes.is_empty(),
            "lane topology should be regenerated from save state"
        );
        assert!(
            migrated
                .state
                .known_hyperspace_lanes
                .iter()
                .all(|lane| migrated.state.explored_stars.contains(&lane.a)
                    && migrated.state.explored_stars.contains(&lane.b)),
            "known lane set must only contain fully explored endpoints"
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
