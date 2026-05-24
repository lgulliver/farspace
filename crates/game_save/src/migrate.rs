//! Save file migration

use crate::schema::{SaveFile, CURRENT_VERSION};
use crate::SaveError;

/// Migrate a save file to the current version
pub fn migrate(save: SaveFile) -> Result<SaveFile, SaveError> {
    let mut save = save;
    loop {
        match save.version {
            CURRENT_VERSION => return Ok(save),
            v if v > CURRENT_VERSION => {
                return Err(SaveError::UnsupportedVersion {
                    found: v,
                    supported: CURRENT_VERSION,
                });
            }
            0 => {
                save.version = 1;
            }
            1 => {
                let home_stars: Vec<_> = save.state.empires.values().map(|e| e.home_star).collect();
                for star_id in home_stars {
                    save.state.explored_stars.insert(star_id);
                }
                save.version = 2;
            }
            2 => save.version = 3,
            3 => save.version = 4,
            4 => save.version = 5,
            5 => save.version = 6,
            6 => save.version = 7,
            7 => save.version = 8,
            8 => save.version = 9,
            9 => save.version = 10,
            10 => save.version = 11,
            11 => save.version = 12,
            12 => save.version = 13,
            13 => save.version = 14,
            14 => save.version = 15,
            15 => save.version = 16,
            16 => {
                let sectors: Vec<_> = save.state.sectors.values().cloned().collect();
                let stars: Vec<_> = save.state.stars.values().cloned().collect();
                save.state.hyperspace_lanes =
                    game_core::galaxy::generate_hyperspace_lanes(save.state.seed, &sectors, &stars)
                        .into_iter()
                        .collect();
                save.state.known_hyperspace_lanes = save
                    .state
                    .hyperspace_lanes
                    .iter()
                    .copied()
                    .filter(|lane| {
                        save.state.explored_stars.contains(&lane.a())
                            && save.state.explored_stars.contains(&lane.b())
                    })
                    .collect();
                save.version = 17;
            }
            17 => {
                let seed = save.state.seed;
                for star in save.state.stars.values_mut() {
                    let star_id = star.id;
                    for (planet_index, planet) in star.planets.iter_mut().enumerate() {
                        let (specials, resources) =
                            game_core::galaxy::generate_planet_specials_and_resources_for_context(
                                seed,
                                star_id,
                                planet_index,
                                game_core::galaxy::ResourceGenerationContext {
                                    planet_class: planet.class,
                                    spectral_class: star.spectral_class,
                                    sector_id: star.sector,
                                    star_x: star.x,
                                    star_y: star.y,
                                },
                            );
                        planet.specials = specials;
                        planet.resources = resources;
                    }
                }
                save.version = 18;
            }
            18 => save.version = 19,
            19 => save.version = 20,
            20 => {
                save.metadata = crate::schema::SaveMetadata {
                    schema_version: 21,
                    game_version: None,
                    created_turn: save.state.turn,
                    seed: save.state.seed,
                };
                save.version = 21;
            }
            21 => {
                save.metadata.schema_version = 22;
                save.version = 22;
            }
            22 => {
                save.state.colony_supply = save.state.recompute_colony_supply();
                save.metadata.schema_version = 23;
                save.version = 23;
            }
            23 => {
                save.state.colony_blockade = save.state.recompute_colony_blockade();
                save.metadata.schema_version = 24;
                save.version = 24;
            }
            24 => {
                save.metadata.schema_version = 25;
                save.version = 25;
            }
            25 => {
                save.metadata.schema_version = 26;
                save.version = 26;
            }
            26 => {
                save.metadata.schema_version = 27;
                save.version = 27;
            }
            27 => {
                save.metadata.schema_version = 28;
                save.version = 28;
            }
            28 => {
                save.metadata.schema_version = 29;
                save.version = 29;
            }
            29 => {
                save.metadata.schema_version = 30;
                save.version = 30;
            }
            30..=33 => {
                save.state.colony_supply = save.state.recompute_colony_supply();
                save.state.colony_blockade = save.state.recompute_colony_blockade();
                save.state.empire_resource_access = save.state.recompute_empire_resource_access();
                save.metadata.schema_version = 34;
                save.version = 34;
            }
            34 => {
                let seed = save.state.seed;
                for star in save.state.stars.values_mut() {
                    let star_id = star.id;
                    for (planet_index, planet) in star.planets.iter_mut().enumerate() {
                        let discoveries =
                            game_core::galaxy::generate_planet_discoveries_for_context(
                                seed,
                                star_id,
                                planet_index,
                                game_core::galaxy::ResourceGenerationContext {
                                    planet_class: planet.class,
                                    spectral_class: star.spectral_class,
                                    sector_id: star.sector,
                                    star_x: star.x,
                                    star_y: star.y,
                                },
                            );
                        planet.anomalies = discoveries.anomalies;
                    }
                }
                save.metadata.schema_version = 35;
                save.version = 35;
            }
            35 => {
                save.state.fleet_supply = save.state.recompute_fleet_supply();
                save.metadata.schema_version = CURRENT_VERSION;
                save.version = CURRENT_VERSION;
            }
            _ => {
                return Err(SaveError::UnsupportedVersion {
                    found: save.version,
                    supported: CURRENT_VERSION,
                });
            }
        }
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
            metadata: Default::default(),
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
                empire_def: None,
            },
        );
        // explored_stars starts empty
        assert!(state.explored_stars.is_empty());

        let v1_save = SaveFile {
            version: 1,
            state,
            metadata: Default::default(),
        };
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
        let v2_save = SaveFile {
            version: 2,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v2_save).expect("v2 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(migrated.state.fleet_missions.is_empty());
    }

    #[test]
    fn migrate_v3_to_v4_succeeds() {
        let state = GameState::default();
        let v3_save = SaveFile {
            version: 3,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v3_save).expect("v3 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v4_to_v5_succeeds() {
        let state = GameState::default();
        let v4_save = SaveFile {
            version: 4,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v4_save).expect("v4 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v5_to_v6_succeeds() {
        let state = GameState::default();
        let v5_save = SaveFile {
            version: 5,
            state,
            metadata: Default::default(),
        };
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
        let v6_save = SaveFile {
            version: 6,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v6_save).expect("v6 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // diplomacy defaults to empty
        assert!(migrated.state.diplomacy.is_empty());
    }

    #[test]
    fn migrate_v7_to_v8_succeeds() {
        let state = GameState::default();
        let v7_save = SaveFile {
            version: 7,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v7_save).expect("v7 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v8_to_v9_succeeds() {
        let state = GameState::default();
        let v8_save = SaveFile {
            version: 8,
            state,
            metadata: Default::default(),
        };
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
        let v9_save = SaveFile {
            version: 9,
            state,
            metadata: Default::default(),
        };
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
        let v13_save = SaveFile {
            version: 13,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v13_save).expect("v13 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v14_to_v15_defaults_survey_missions_to_empty() {
        let state = GameState::default();
        let v14_save = SaveFile {
            version: 14,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v14_save).expect("v14 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(migrated.state.survey_missions.is_empty());
    }

    #[test]
    fn migrate_v15_to_v16_preserves_missions_with_defaults() {
        // v15 saves have ScoutMission/FleetMission without origin/total_duration.
        // Migration should succeed; new fields default safely via serde.
        let state = GameState::default();
        let v15_save = SaveFile {
            version: 15,
            state,
            metadata: Default::default(),
        };
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

        let v16_save = SaveFile {
            version: 16,
            state,
            metadata: Default::default(),
        };
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
                .all(|lane| migrated.state.explored_stars.contains(&lane.a())
                    && migrated.state.explored_stars.contains(&lane.b())),
            "known lane set must only contain fully explored endpoints"
        );
    }

    #[test]
    fn migrate_old_unsupported_version_fails() {
        // Only versions 0..=CURRENT_VERSION and the explicit future-version guard are
        // handled; we can't synthesise a "dead _ arm" version with a u32.  Verify instead
        // that the future-version guard is the expected rejection path (re-tested here for
        // symmetry with the renamed test).
        let save = SaveFile {
            version: CURRENT_VERSION + 1,
            state: GameState::default(),
            metadata: Default::default(),
        };
        let result = migrate(save);
        assert!(matches!(result, Err(SaveError::UnsupportedVersion { .. })));
    }

    #[test]
    fn migrate_v0_to_current_succeeds() {
        let state = GameState::default();
        let v0_save = SaveFile {
            version: 0,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v0_save).expect("v0 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v19_to_current_adds_scenario_and_ai_empires_defaults() {
        // A v19 save without scenario/ai_empires fields should migrate cleanly
        // and default those fields to None / empty respectively.
        let state = GameState::default();
        let v19_save = SaveFile {
            version: 19,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v19_save).expect("v19 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // scenario defaults to None for old saves
        assert!(migrated.state.scenario.is_none());
        // ai_empires defaults to empty for old saves
        assert!(migrated.state.ai_empires.is_empty());
    }

    #[test]
    fn migrate_v20_to_v21_populates_metadata() {
        let engine = game_core::Engine::new(7777);
        let state = engine.state;
        let expected_seed = state.seed;
        let expected_turn = state.turn;
        let v20_save = SaveFile {
            version: 20,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v20_save).expect("v20 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert_eq!(
            migrated.metadata.schema_version, CURRENT_VERSION,
            "migrated save must have schema_version = CURRENT_VERSION"
        );
        assert_eq!(
            migrated.metadata.seed, expected_seed,
            "migrated save must have correct seed in metadata"
        );
        assert_eq!(
            migrated.metadata.created_turn, expected_turn,
            "migrated save must have correct created_turn in metadata"
        );
        // game_version is unknown for migrated saves
        assert!(
            migrated.metadata.game_version.is_none(),
            "migrated save must not claim a known game_version"
        );
    }

    #[test]
    fn migrate_v21_to_v22_preserves_empire_data() {
        // v21 → v22 adds empire_def and player_empire_def (serde defaults = None).
        // Migration is a pass-through; existing empire names and state are preserved.
        //
        // Simulate a real v21 save: create an engine, then clear empire_def on all
        // empires to represent the state before v22 was introduced.
        let engine = game_core::Engine::new(9999);
        let mut state = engine.state;
        let player_empire = state.player_empire;
        let player_name = state.empires.get(&player_empire).unwrap().name.clone();

        // Clear empire_def to mimic a pre-v22 save file
        for empire in state.empires.values_mut() {
            empire.empire_def = None;
        }

        let v21_save = SaveFile {
            version: 21,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v21_save).expect("v21 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // Empire names preserved
        assert_eq!(
            migrated.state.empires.get(&player_empire).unwrap().name,
            player_name,
            "Empire name must be preserved through v21→v22 migration"
        );
        // empire_def remains None — migration is a pass-through for this field
        assert!(
            migrated
                .state
                .empires
                .get(&player_empire)
                .unwrap()
                .empire_def
                .is_none(),
            "Migrated empire must have empire_def = None (pass-through from pre-v22 save)"
        );
    }

    #[test]
    fn empire_identity_round_trip_via_save_load() {
        use game_core::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
        let setup = ScenarioSetup {
            seed: 1111,
            galaxy_size: GalaxySize::Medium,
            ai_empire_count: 1,
            sector_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: Some(EmpireDefinitionId(5)), // Elarith Confluence
            victory_settings: game_core::VictorySettings::default_v1(),
        };
        let engine = game_core::Engine::new_from_setup(setup);
        let save = SaveFile::new(engine.state.clone());

        // Serialise and deserialise via JSON (same as the real save path)
        let json = serde_json::to_string(&save).expect("serialize");
        let restored: SaveFile = serde_json::from_str(&json).expect("deserialize");

        let player_def = restored
            .state
            .empires
            .get(&restored.state.player_empire)
            .unwrap()
            .empire_def;
        assert_eq!(
            player_def,
            Some(EmpireDefinitionId(5)),
            "Player empire def must survive full save/load round-trip"
        );
    }

    #[test]
    fn migrate_v23_derives_colony_blockade() {
        // v23 saves do not have colony_blockade; migration should re-derive it.
        // In this test state there are no hostile fleets so blockade map stays empty.
        let state = GameState::default();
        let v23_save = SaveFile {
            version: 23,
            state,
            metadata: Default::default(),
        };
        let migrated = migrate(v23_save).expect("v23 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        // No hostile fleets in default state → no blockades
        assert!(
            migrated.state.colony_blockade.is_empty(),
            "No blockades expected in default state after v23 migration"
        );
    }

    #[test]
    fn migrate_v24_to_v25_passthrough() {
        let state = GameState::default();
        let metadata = crate::schema::SaveMetadata {
            schema_version: 24,
            ..Default::default()
        };
        let v24_save = SaveFile {
            version: 24,
            state,
            metadata,
        };
        let migrated = migrate(v24_save).expect("v24→v25 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert_eq!(migrated.metadata.schema_version, CURRENT_VERSION);
    }

    #[test]
    fn blockade_state_round_trip_via_save_load() {
        use game_core::{
            state::{Fleet, FleetId, FleetKind, RelationshipStatus},
            ColonyId, Empire, EmpireId, StarId,
        };

        // Build a game state with a war-status enemy fleet at a player colony star
        let engine = game_core::Engine::new(42);
        let mut state = engine.state.clone();
        let player_id = state.player_empire;
        let colony_star = state.empires.get(&player_id).map(|e| e.home_star).unwrap();

        // Add an enemy empire at war — also insert a proper Empire record so state
        // is internally consistent (every fleet owner must be a real empire).
        let enemy_id = EmpireId(99);
        state.diplomacy.insert(enemy_id, RelationshipStatus::War);
        state.empires.insert(
            enemy_id,
            Empire {
                id: enemy_id,
                name: "Hostile Power".to_string(),
                credits: 0,
                research_points: 0,
                home_star: StarId(9999),
                research: Default::default(),
                food: 0,
                empire_def: None,
            },
        );

        // Place an idle enemy fleet at the colony star
        let enemy_fid = FleetId(999);
        state.fleets.insert(
            enemy_fid,
            Fleet {
                id: enemy_fid,
                owner: enemy_id,
                location: colony_star,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 3,
                integrity: 100,
            },
        );

        // Recompute so colony_blockade is populated
        state.colony_blockade = state.recompute_colony_blockade();
        let blockaded_before: std::collections::BTreeSet<ColonyId> =
            state.colony_blockade.keys().copied().collect();

        // Serialise / deserialise
        let save = SaveFile::new(state);
        let json = serde_json::to_string(&save).expect("serialize");
        let restored: SaveFile = serde_json::from_str(&json).expect("deserialize");

        // Re-derive blockade from restored state
        let rederived = restored.state.recompute_colony_blockade();
        let blockaded_after: std::collections::BTreeSet<ColonyId> =
            rederived.keys().copied().collect();

        assert_eq!(
            blockaded_before, blockaded_after,
            "Blockaded colonies must be the same before and after save/load"
        );
    }

    #[test]
    fn migrate_v25_to_v26_passthrough() {
        let state = GameState::default();
        let metadata = crate::schema::SaveMetadata {
            schema_version: 25,
            ..Default::default()
        };
        let v25_save = SaveFile {
            version: 25,
            state,
            metadata,
        };
        let migrated = migrate(v25_save).expect("v25→v26 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert_eq!(migrated.metadata.schema_version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v26_to_v27_passthrough() {
        let state = GameState::default();
        let metadata = crate::schema::SaveMetadata {
            schema_version: 26,
            ..Default::default()
        };
        let v26_save = SaveFile {
            version: 26,
            state,
            metadata,
        };
        let migrated = migrate(v26_save).expect("v26→v27 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert_eq!(migrated.metadata.schema_version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v27_to_v28_passthrough() {
        let state = GameState::default();
        let metadata = crate::schema::SaveMetadata {
            schema_version: 27,
            ..Default::default()
        };
        let v27_save = SaveFile {
            version: 27,
            state,
            metadata,
        };
        let migrated = migrate(v27_save).expect("v27→v28 migration should succeed");
        assert_eq!(migrated.version, CURRENT_VERSION);
        assert_eq!(migrated.metadata.schema_version, CURRENT_VERSION);
    }

    #[test]
    fn save_load_preserves_new_fleet_kinds() {
        use game_core::state::{Fleet, FleetId, FleetKind};

        let engine = game_core::Engine::new(42);
        let mut state = engine.state.clone();
        let star_id = *state.stars.keys().next().unwrap();
        let empire_id = state.player_empire;

        // Insert a fleet of each new archetype kind
        for (i, kind) in [
            FleetKind::FastScout,
            FleetKind::SurveyCutter,
            FleetKind::ColonyArk,
            FleetKind::EscortFrigate,
            FleetKind::MissileFrigate,
            FleetKind::Destroyer,
            FleetKind::PatrolCorvette,
        ]
        .iter()
        .enumerate()
        {
            state.fleets.insert(
                FleetId(200 + i as u64),
                Fleet {
                    id: FleetId(200 + i as u64),
                    owner: empire_id,
                    location: star_id,
                    ships: 1,
                    kind: *kind,
                    strength: 1,
                    integrity: 100,
                },
            );
        }

        let save = SaveFile::new(state.clone());
        let serialized = serde_json::to_string(&save).expect("serialization must succeed");
        let deserialized: SaveFile =
            serde_json::from_str(&serialized).expect("deserialization must succeed");
        let loaded = migrate(deserialized).expect("migration must succeed");

        // Verify all new fleet kinds round-trip correctly
        for (i, kind) in [
            FleetKind::FastScout,
            FleetKind::SurveyCutter,
            FleetKind::ColonyArk,
            FleetKind::EscortFrigate,
            FleetKind::MissileFrigate,
            FleetKind::Destroyer,
            FleetKind::PatrolCorvette,
        ]
        .iter()
        .enumerate()
        {
            let fid = FleetId(200 + i as u64);
            let loaded_fleet = loaded
                .state
                .fleets
                .get(&fid)
                .expect("fleet must survive round-trip");
            assert_eq!(
                loaded_fleet.kind, *kind,
                "FleetKind {:?} must survive save/load round-trip",
                kind
            );
        }
    }

    #[test]
    fn migrate_v28_to_v29() {
        use crate::schema::SaveMetadata;
        let save = SaveFile {
            version: 28,
            metadata: SaveMetadata {
                schema_version: 28,
                ..Default::default()
            },
            state: GameState::default(),
        };
        let result = migrate(save).expect("v28 → v29 migration should succeed");
        assert_eq!(result.version, CURRENT_VERSION);
        assert_eq!(result.metadata.schema_version, CURRENT_VERSION);
    }

    #[test]
    fn migrate_v29_to_v30() {
        use crate::schema::SaveMetadata;
        let save = SaveFile {
            version: 29,
            metadata: SaveMetadata {
                schema_version: 29,
                ..Default::default()
            },
            state: GameState::default(),
        };
        let result = migrate(save).expect("v29 → v30 migration should succeed");
        assert_eq!(result.version, CURRENT_VERSION);
        assert_eq!(result.metadata.schema_version, CURRENT_VERSION);
    }

    #[test]
    fn custom_designs_survive_save_load_roundtrip() {
        use crate::schema::SaveFile;
        use game_core::{ComponentId, CustomDesignId, CustomShipDesign, EmpireId, HullId};

        let mut state = GameState::default();

        let design = CustomShipDesign {
            design_id: CustomDesignId(1),
            hull_id: HullId::SCOUT,
            components: vec![
                ComponentId::CHEMICAL_THRUSTERS,
                ComponentId::LONG_RANGE_SENSORS,
            ],
            owner: EmpireId(1),
            name: "Test Scout".to_string(),
            obsolete: false,
        };
        state.custom_designs.insert(CustomDesignId(1), design);
        state.next_custom_design_id = 1;

        let save = SaveFile::new(state);
        let serialized = serde_json::to_string(&save).expect("serialization must succeed");
        let deserialized: SaveFile =
            serde_json::from_str(&serialized).expect("deserialization must succeed");
        let loaded = migrate(deserialized).expect("migration must succeed");

        assert_eq!(loaded.state.next_custom_design_id, 1);
        let loaded_design = loaded
            .state
            .custom_designs
            .get(&CustomDesignId(1))
            .expect("custom design must survive round-trip");
        assert_eq!(loaded_design.name, "Test Scout");
        assert_eq!(loaded_design.hull_id, HullId::SCOUT);
        assert!(!loaded_design.obsolete);
        assert_eq!(
            loaded_design.components,
            vec![
                ComponentId::CHEMICAL_THRUSTERS,
                ComponentId::LONG_RANGE_SENSORS
            ]
        );
    }

    #[test]
    fn save_load_round_trip_preserves_dispatch_history() {
        use crate::{load, save};
        use game_core::dispatch::{
            DispatchCategory, DispatchItem, DispatchSeverity, GalacticDispatch,
        };

        let mut state = GameState::default();
        // Populate a non-empty dispatch history
        state.galactic_dispatches.push_back(GalacticDispatch {
            turn: 4,
            title: "Galactic Dispatch — Turn 5".to_string(),
            items: vec![DispatchItem {
                category: DispatchCategory::Exploration,
                severity: DispatchSeverity::Notice,
                headline: "Survey Crews Chart New Frontier Worlds".to_string(),
                body: "Scout vessels have confirmed a new system entry.".to_string(),
                related_empire_id: None,
                related_star_id: None,
                related_planet_index: None,
            }],
        });
        state.galactic_dispatches.push_back(GalacticDispatch {
            turn: 9,
            title: "Galactic Dispatch — Turn 10".to_string(),
            items: vec![],
        });

        let bytes = save(&state).expect("save should succeed");
        let loaded = load(&bytes).expect("load should succeed");

        assert_eq!(
            loaded.galactic_dispatches, state.galactic_dispatches,
            "dispatch history must survive a save/load round-trip"
        );
        assert_eq!(
            loaded.galactic_dispatches.len(),
            2,
            "both dispatches must be present after round-trip"
        );
        assert_eq!(
            loaded.galactic_dispatches[0].title,
            "Galactic Dispatch — Turn 5"
        );
        assert_eq!(
            loaded.galactic_dispatches[1].title,
            "Galactic Dispatch — Turn 10"
        );
    }
}
