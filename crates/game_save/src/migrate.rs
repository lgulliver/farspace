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
        0 => {
            // v0 → v1: schema before explored_stars was tracked.
            // v1 handles populating explored_stars from each empire's home star — pass through.
            migrate(SaveFile {
                version: 1,
                state: save.state,
                ..save
            })
        }
        1 => {
            // v1 → v2: populate explored_stars with each empire's home star.
            let mut state = save.state;
            let home_stars: Vec<_> = state.empires.values().map(|e| e.home_star).collect();
            for star_id in home_stars {
                state.explored_stars.insert(star_id);
            }
            // Continue migrating v2 → v3
            migrate(SaveFile {
                version: 2,
                state,
                metadata: save.metadata,
            })
        }
        2 => {
            // v2 -> v3: fleet_missions field added; defaults to empty via serde(default).
            // Nothing to populate -- just bump the version and continue.
            migrate(SaveFile { version: 3, ..save })
        }
        3 => {
            // v3 -> v4: FleetKind added to Fleet (serde default = Scout) and
            // habitable added to Planet (serde default = true).
            // Nothing to populate — just bump the version.
            migrate(SaveFile { version: 4, ..save })
        }
        4 => {
            // v4 -> v5: Empire.food field added (serde default = 0).
            // Nothing to populate — just bump the version.
            migrate(SaveFile { version: 5, ..save })
        }
        5 => {
            // v5 -> v6: GameState.ai_empire (Option<EmpireId>, default None) and
            // GameState.ai_explored_stars (BTreeSet<StarId>, default empty) added.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            // Note: saves migrated from v5 will have ai_empire=None until a new game is
            // started, meaning no AI opponent will be active for existing saves.
            migrate(SaveFile { version: 6, ..save })
        }
        6 => {
            // v6 -> v7: GameState.diplomacy (BTreeMap<EmpireId, RelationshipStatus>, default empty)
            // added.  Relies on serde default — nothing to populate explicitly.
            // Existing saves will have diplomacy=empty (all empires start Unknown).
            migrate(SaveFile { version: 7, ..save })
        }
        7 => {
            // v7 -> v8: Fleet.strength (u32, default 1) and Fleet.integrity (u32, default 100)
            // added for combat auto-resolve.  Both rely on serde defaults — nothing to populate
            // explicitly.  Existing fleets will have full health and base strength on load.
            migrate(SaveFile { version: 8, ..save })
        }
        8 => {
            // v8 -> v9: Planet.class (PlanetClass, default Terran) and Colony.surface_installations /
            // orbital_installations (Vec<BuildingType>, default empty) added for infrastructure system.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            // Existing planets will default to Terran class; colonies start with no installed infrastructure.
            migrate(SaveFile { version: 9, ..save })
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
                ..save
            })
        }
        10 => {
            // v10 -> v11: Colony.stability: u8 added (serde default = 100).
            // All existing colonies default to neutral stability.
            // Nothing to populate explicitly — just bump the version.
            migrate(SaveFile {
                version: 11,
                ..save
            })
        }
        11 => {
            // v11 -> v12: Colony.role: ColonyRole added (serde default = Balanced).
            // All existing colonies default to Balanced role (no modifiers).
            // Nothing to populate explicitly — continue to v13 migration.
            migrate(SaveFile {
                version: 12,
                ..save
            })
        }
        12 => {
            // v12 -> v13: Planet.surveyed: bool added (serde default = false).
            // Existing planets default to unsurveyed via serde default.
            migrate(SaveFile {
                version: 13,
                ..save
            })
        }
        13 => {
            // v13 -> v14: Sector and SectorId added; GameState.sectors (BTreeMap<SectorId, Sector>)
            // and Star.sector (SectorId) added.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            // v13 saves will have empty sectors and SectorId(0) on stars until a new game is started.
            migrate(SaveFile {
                version: 14,
                ..save
            })
        }
        14 => {
            // v14 -> v15: GameState.survey_missions added (serde default = empty).
            // Science ships are encoded as FleetKind::Science, which old saves can load
            // as long as the new field defaults to empty.
            migrate(SaveFile {
                version: 15,
                ..save
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
                ..save
            })
        }
        16 => {
            // v16 -> v17: GameState.hyperspace_lanes and
            // GameState.known_hyperspace_lanes added.
            //
            // Populate deterministic lane topology from seed + sectors + stars and
            // derive player-known lanes from current explored stars.
            let metadata = save.metadata;
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
                    state.explored_stars.contains(&lane.a())
                        && state.explored_stars.contains(&lane.b())
                })
                .collect();

            migrate(SaveFile {
                version: 17,
                state,
                metadata,
            })
        }
        17 => {
            // v17 -> v18: Planet.specials (Vec<PlanetSpecial>), Planet.resources
            // (Vec<StrategicResource>), and Planet.ancient_ruins_collected (bool) added.
            //
            // All three fields carry serde defaults (empty Vec / false) so existing saves
            // deserialise safely.  We repopulate specials and resources deterministically
            // from the galaxy seed so that old saves gain the new content immediately.
            // ancient_ruins_collected stays false — already-surveyed ruins will passively
            // provide the science bonus without re-emitting the discovery event.
            let metadata = save.metadata;
            let mut state = save.state;
            let seed = state.seed;
            for star in state.stars.values_mut() {
                let star_id = star.id;
                for (planet_index, planet) in star.planets.iter_mut().enumerate() {
                    let (specials, resources) =
                        game_core::galaxy::generate_planet_specials_and_resources(
                            seed,
                            star_id,
                            planet_index,
                        );
                    planet.specials = specials;
                    planet.resources = resources;
                    // ancient_ruins_collected stays false (default) — no history available.
                }
            }
            migrate(SaveFile {
                version: 18,
                state,
                metadata,
            })
        }
        18 => {
            // v18 -> v19: Colony.rally_point (Option<StarId>, default None) and
            // GameState.fleet_orders (BTreeMap<FleetId, FleetOrder>, default empty) added.
            // Both fields carry serde defaults — nothing to populate explicitly.
            // Existing colonies start without a rally point; existing fleets start with no order.
            migrate(SaveFile {
                version: 19,
                state: save.state,
                ..save
            })
        }
        19 => {
            // v19 -> v20: GameState.scenario (Option<ScenarioSetup>, default None) and
            // GameState.ai_empires (Vec<EmpireId>, default empty) added.
            //
            // Both fields carry serde defaults:
            //   - scenario: None  (setup metadata not available for old saves)
            //   - ai_empires: []  (empty — the legacy ai_empire field still drives AI turns
            //     for saves migrated from v19; process_end_turn falls back to it when
            //     ai_empires is empty)
            migrate(SaveFile {
                version: 20,
                ..save
            })
        }
        20 => {
            // v20 → v21: SaveMetadata added to SaveFile.
            // Populate metadata from the current state; game_version is unknown for migrated saves.
            let metadata = crate::schema::SaveMetadata {
                schema_version: 21,
                game_version: None,
                created_turn: save.state.turn,
                seed: save.state.seed,
            };
            migrate(SaveFile {
                version: 21,
                metadata,
                state: save.state,
            })
        }
        21 => {
            // v21 → v22: Empire.empire_def (Option<EmpireDefinitionId>, default None) and
            // ScenarioSetup.player_empire_def (Option<EmpireDefinitionId>, default None) added.
            // Both fields rely on serde defaults — nothing to populate explicitly.
            // Existing empires start without an empire identity; their names are preserved.
            // Also update metadata.schema_version to reflect the new version.
            let state = save.state;
            let mut metadata = save.metadata;
            metadata.schema_version = 22;
            migrate(SaveFile {
                version: 22,
                metadata,
                state,
            })
        }
        22 => {
            // v22 → v23: GameState.colony_supply (BTreeMap<ColonyId, ColonySupplyState>) added.
            // This is derivable from current state; recompute deterministically on load.
            let mut state = save.state;
            state.colony_supply = state.recompute_colony_supply();
            let mut metadata = save.metadata;
            metadata.schema_version = 23;
            migrate(SaveFile {
                version: 23,
                metadata,
                state,
            })
        }
        23 => {
            // v23 → v24: GameState.colony_blockade (BTreeMap<ColonyId, EmpireId>) added.
            // Fully derivable from current fleet positions and diplomacy state;
            // recompute deterministically on load.
            let mut state = save.state;
            state.colony_blockade = state.recompute_colony_blockade();
            let mut metadata = save.metadata;
            metadata.schema_version = 24;
            migrate(SaveFile {
                version: 24,
                metadata,
                state,
            })
        }
        24 => {
            // v24 → v25: FleetKind gained `TroopTransport`.
            // v24 saves cannot contain the new variant, so deserialization remains valid
            // and no state rewrite is needed.
            // Keep this explicit version step so post-invasion saves are distinguishable.
            let mut metadata = save.metadata;
            metadata.schema_version = 25;
            migrate(SaveFile {
                version: 25,
                metadata,
                state: save.state,
            })
        }
        25 => {
            // v25 → v26: FleetKind gained FastScout, SurveyCutter, ColonyArk,
            // EscortFrigate, MissileFrigate, Destroyer, PatrolCorvette.
            // v25 saves cannot contain the new variants, so deserialization remains
            // valid and no state rewrite is needed.
            let mut metadata = save.metadata;
            metadata.schema_version = 26;
            migrate(SaveFile {
                version: 26,
                metadata,
                state: save.state,
            })
        }
        26 => {
            // v26 → v27: ResearchState gained `queue: Vec<TechId>` with serde default.
            let mut metadata = save.metadata;
            metadata.schema_version = 27;
            migrate(SaveFile {
                version: 27,
                metadata,
                state: save.state,
            })
        }
        27 => {
            // v27 → v28: ScenarioSetup/GameState gained victory settings and status fields.
            // All new fields have serde defaults; this is a passthrough version bump.
            // v27 saves deserialize safely with defaults for victory fields.
            let mut metadata = save.metadata;
            metadata.schema_version = 28;
            migrate(SaveFile {
                version: 28,
                metadata,
                state: save.state,
            })
        }
        28 => {
            // v28 → v29: GameState gained galactic_dispatches (VecDeque<GalacticDispatch>).
            // All new fields have serde defaults; this is a passthrough version bump.
            let mut metadata = save.metadata;
            metadata.schema_version = 29;
            migrate(SaveFile {
                version: 29,
                metadata,
                state: save.state,
            })
        }
        29 => {
            // v29 → v30: GameState gained custom_designs (BTreeMap<CustomDesignId,
            // CustomShipDesign>) and next_custom_design_id (u32) for Ship Designer Lite v1.
            // Both fields carry #[serde(default)] so old saves deserialise them as empty
            // map / 0 respectively.  This is a passthrough version bump.
            let mut metadata = save.metadata;
            metadata.schema_version = 30;
            migrate(SaveFile {
                version: 30,
                metadata,
                state: save.state,
            })
        }
        30 => {
            // v30 → v31: GameState gained fleet_roles, fleet_formations, and fleet_names
            // for Fleet Roles and Formations v1. All fields carry serde defaults, so this
            // remains a passthrough bump preserving existing saves.
            let mut metadata = save.metadata;
            metadata.schema_version = 31;
            migrate(SaveFile {
                version: 31,
                metadata,
                state: save.state,
            })
        }
        31 => {
            // v31 → v32: GameState gained diplomacy_relationships,
            // diplomacy_pending_communications, and diplomacy_next_communication_id.
            // All fields carry serde defaults; this remains a passthrough bump.
            let mut metadata = save.metadata;
            metadata.schema_version = CURRENT_VERSION;
            Ok(SaveFile {
                version: CURRENT_VERSION,
                metadata,
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
