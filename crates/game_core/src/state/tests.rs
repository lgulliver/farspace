use super::*;

#[test]
fn star_id_ordering() {
    let id1 = StarId(1);
    let id2 = StarId(2);
    assert!(id1 < id2);
}

#[test]
fn sector_id_ordering() {
    let id1 = SectorId(1);
    let id2 = SectorId(2);
    assert!(id1 < id2);
}

#[test]
fn sector_id_equality() {
    let id1 = SectorId(42);
    let id2 = SectorId(42);
    let id3 = SectorId(43);
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn hyperspace_lane_normalizes_endpoint_order() {
    let lane = HyperspaceLane::new(StarId(9), StarId(2)).expect("distinct stars");
    assert_eq!(lane.a(), StarId(2));
    assert_eq!(lane.b(), StarId(9));
    assert!(lane.connects(StarId(9), StarId(2)));
    assert!(lane.connects(StarId(2), StarId(9)));
    assert!(HyperspaceLane::new(StarId(7), StarId(7)).is_none());
}

#[test]
fn empire_id_ordering() {
    let id1 = EmpireId(5);
    let id2 = EmpireId(3);
    assert!(id1 > id2);
}

#[test]
fn build_item_costs() {
    assert_eq!(BuildItem::Ship(ShipDesignId::SCOUT).cost(), 50);
    assert_eq!(BuildItem::Ship(ShipDesignId::SCIENCE).cost(), 100);
    assert_eq!(BuildItem::Ship(ShipDesignId::COLONY).cost(), 200);
    assert_eq!(BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT).cost(), 150);
    assert_eq!(BuildItem::Scout.cost(), 50);
    assert_eq!(
        BuildItem::Ship(ShipDesignId::SCIENCE).name(),
        "Science Ship"
    );
    assert_eq!(BuildItem::Colony.cost(), 200);
    assert_eq!(BuildItem::Outpost.cost(), 100);
    assert_eq!(
        BuildItem::Structure(BuildingType::AquacultureBay).cost(),
        60
    );
    assert_eq!(
        BuildItem::Structure(BuildingType::FabricationYard).cost(),
        80
    );
    assert_eq!(BuildItem::Structure(BuildingType::ScienceNexus).cost(), 100);
}

#[test]
fn planet_size_capacities() {
    assert_eq!(PlanetSize::Tiny.base_capacity(), 2);
    assert_eq!(PlanetSize::Massive.base_capacity(), 16);
}

#[test]
fn game_state_next_ids() {
    let mut state = GameState::default();
    let c1 = state.next_colony_id();
    let c2 = state.next_colony_id();
    assert_eq!(c1.0, 1);
    assert_eq!(c2.0, 2);

    let f1 = state.next_fleet_id();
    let f2 = state.next_fleet_id();
    assert_eq!(f1.0, 1);
    assert_eq!(f2.0, 2);
}

#[test]
fn spectral_class_as_char() {
    assert_eq!(SpectralClass::O.as_char(), 'O');
    assert_eq!(SpectralClass::B.as_char(), 'B');
    assert_eq!(SpectralClass::A.as_char(), 'A');
    assert_eq!(SpectralClass::F.as_char(), 'F');
    assert_eq!(SpectralClass::G.as_char(), 'G');
    assert_eq!(SpectralClass::K.as_char(), 'K');
    assert_eq!(SpectralClass::M.as_char(), 'M');
}

#[test]
fn spectral_class_all_contains_all_variants() {
    let all = SpectralClass::all();
    assert_eq!(all.len(), 7);
    assert!(all.contains(&SpectralClass::O));
    assert!(all.contains(&SpectralClass::M));
}

#[test]
fn planet_size_all_contains_all_variants() {
    let all = PlanetSize::all();
    assert_eq!(all.len(), 5);
    assert!(all.contains(&PlanetSize::Tiny));
    assert!(all.contains(&PlanetSize::Massive));
}

#[test]
fn planet_size_all_base_capacities() {
    assert_eq!(PlanetSize::Small.base_capacity(), 4);
    assert_eq!(PlanetSize::Medium.base_capacity(), 8);
    assert_eq!(PlanetSize::Large.base_capacity(), 12);
}

#[test]
fn build_item_names() {
    assert_eq!(BuildItem::Ship(ShipDesignId::SCOUT).name(), "Scout");
    assert_eq!(BuildItem::Ship(ShipDesignId::COLONY).name(), "Colony Ship");
    assert_eq!(
        BuildItem::Ship(ShipDesignId::SCIENCE).name(),
        "Science Ship"
    );
    assert_eq!(
        BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT).name(),
        "Troop Transport"
    );
    assert_eq!(BuildItem::Scout.name(), "Scout");
    assert_eq!(BuildItem::Colony.name(), "Colony Ship");
    assert_eq!(BuildItem::Outpost.name(), "Outpost");
    assert_eq!(
        BuildItem::Structure(BuildingType::AquacultureBay).name(),
        "Aquaculture Bay"
    );
    assert_eq!(
        BuildItem::Structure(BuildingType::FabricationYard).name(),
        "Fabrication Yard"
    );
    assert_eq!(
        BuildItem::Structure(BuildingType::ScienceNexus).name(),
        "Science Nexus"
    );
}

#[test]
fn ship_design_records_are_resolvable() {
    for id in ShipDesignId::all() {
        assert!(id.record().is_some(), "known design ID must resolve");
    }
    assert!(
        ShipDesignId(999).record().is_none(),
        "unknown design ID must be invalid"
    );
}

#[test]
fn all_ship_designs_contains_science_ship() {
    let all = all_ship_designs();
    assert_eq!(all.len(), 11);
    assert!(all.iter().any(|d| d.name == "Science Ship"));
    assert!(all.iter().any(|d| d.name == "Troop Transport"));
    assert!(all.iter().any(|d| d.name == "Fast Scout"));
    assert!(all.iter().any(|d| d.name == "Destroyer"));
}

#[test]
fn ship_design_maintenance_values_are_deterministic() {
    let designs = all_ship_designs();
    // Check all known designs have expected deterministic maintenance
    let find = |name: &str| designs.iter().find(|d| d.name == name).unwrap();
    assert_eq!(find("Scout").maintenance, 1);
    assert_eq!(find("Colony Ship").maintenance, 1);
    assert_eq!(find("Science Ship").maintenance, 1);
    assert_eq!(find("Troop Transport").maintenance, 2);
    assert_eq!(find("Fast Scout").maintenance, 1);
    assert_eq!(find("Survey Cutter").maintenance, 2);
    assert_eq!(find("Colony Ark").maintenance, 2);
    assert_eq!(find("Escort Frigate").maintenance, 2);
    assert_eq!(find("Missile Frigate").maintenance, 3);
    assert_eq!(find("Destroyer").maintenance, 4);
    assert_eq!(find("Patrol Corvette").maintenance, 1);
}

#[test]
fn ship_design_role_descriptions_are_non_empty() {
    for design in all_ship_designs() {
        assert!(
            !design.role.is_empty(),
            "design {} has empty role description",
            design.name
        );
    }
}

#[test]
fn ship_design_strength_values_are_positive() {
    for design in all_ship_designs() {
        assert!(
            design.strength >= 1,
            "design {} strength must be >= 1",
            design.name
        );
    }
}

#[test]
fn fleet_kind_maintenance_cost_is_deterministic() {
    // Light ships cost 1
    assert_eq!(FleetKind::Scout.maintenance_cost(), 1);
    assert_eq!(FleetKind::FastScout.maintenance_cost(), 1);
    assert_eq!(FleetKind::Science.maintenance_cost(), 1);
    assert_eq!(FleetKind::Colonizer.maintenance_cost(), 1);
    assert_eq!(FleetKind::PatrolCorvette.maintenance_cost(), 1);
    // Medium ships cost 2
    assert_eq!(FleetKind::TroopTransport.maintenance_cost(), 2);
    assert_eq!(FleetKind::SurveyCutter.maintenance_cost(), 2);
    assert_eq!(FleetKind::ColonyArk.maintenance_cost(), 2);
    assert_eq!(FleetKind::EscortFrigate.maintenance_cost(), 2);
    // Heavy ships cost 3-4
    assert_eq!(FleetKind::MissileFrigate.maintenance_cost(), 3);
    assert_eq!(FleetKind::Destroyer.maintenance_cost(), 4);
}

#[test]
fn new_ship_designs_require_expected_techs() {
    use crate::state::ShipDesignId;
    let find = |id: ShipDesignId| id.record().unwrap();
    // Free designs (no tech required)
    assert!(find(ShipDesignId::SCOUT).required_tech.is_none());
    // Locked designs need a specific tech
    assert_eq!(
        find(ShipDesignId::FAST_SCOUT).required_tech,
        Some(TechId::RAPID_TRANSIT)
    );
    assert_eq!(
        find(ShipDesignId::SURVEY_CUTTER).required_tech,
        Some(TechId::ADVANCED_SURVEY)
    );
    assert_eq!(
        find(ShipDesignId::COLONY_ARK).required_tech,
        Some(TechId::COLONIAL_VANGUARD)
    );
    assert_eq!(
        find(ShipDesignId::ESCORT_FRIGATE).required_tech,
        Some(TechId::PERIMETER_DEFENSE)
    );
    assert_eq!(
        find(ShipDesignId::PATROL_CORVETTE).required_tech,
        Some(TechId::PERIMETER_DEFENSE)
    );
    assert_eq!(
        find(ShipDesignId::MISSILE_FRIGATE).required_tech,
        Some(TechId::STRIKE_DOCTRINE)
    );
    assert_eq!(
        find(ShipDesignId::DESTROYER).required_tech,
        Some(TechId::FLEET_COORDINATION)
    );
}

#[test]
fn fleet_kind_helpers_classify_correctly() {
    assert!(FleetKind::EscortFrigate.is_combat());
    assert!(FleetKind::MissileFrigate.is_combat());
    assert!(FleetKind::Destroyer.is_combat());
    assert!(FleetKind::PatrolCorvette.is_combat());
    assert!(!FleetKind::Scout.is_combat());
    assert!(!FleetKind::Colonizer.is_combat());

    assert!(FleetKind::Colonizer.is_colonizer());
    assert!(FleetKind::ColonyArk.is_colonizer());
    assert!(!FleetKind::Scout.is_colonizer());

    assert!(FleetKind::Scout.is_scout());
    assert!(FleetKind::FastScout.is_scout());
    assert!(!FleetKind::Science.is_scout());
}

#[test]
fn all_new_tech_ids_resolve() {
    assert!(tech_by_id(TechId::RAPID_TRANSIT).is_some());
    assert!(tech_by_id(TechId::ADVANCED_SURVEY).is_some());
    assert!(tech_by_id(TechId::COLONIAL_VANGUARD).is_some());
    assert!(tech_by_id(TechId::PERIMETER_DEFENSE).is_some());
    assert!(tech_by_id(TechId::STRIKE_DOCTRINE).is_some());
    assert!(tech_by_id(TechId::FLEET_COORDINATION).is_some());
}

#[test]
fn new_techs_unlock_expected_ship_designs() {
    let rapid_transit = tech_by_id(TechId::RAPID_TRANSIT).unwrap();
    assert!(rapid_transit.unlocks.iter().any(|u| matches!(
        u,
        TechUnlock::ShipDesign(id) if *id == ShipDesignId::FAST_SCOUT
    )));

    let fleet_coord = tech_by_id(TechId::FLEET_COORDINATION).unwrap();
    assert!(fleet_coord.unlocks.iter().any(|u| matches!(
        u,
        TechUnlock::ShipDesign(id) if *id == ShipDesignId::DESTROYER
    )));

    let perimeter = tech_by_id(TechId::PERIMETER_DEFENSE).unwrap();
    assert!(perimeter.unlocks.iter().any(|u| matches!(
        u,
        TechUnlock::ShipDesign(id) if *id == ShipDesignId::ESCORT_FRIGATE
    )));
    assert!(perimeter.unlocks.iter().any(|u| matches!(
        u,
        TechUnlock::ShipDesign(id) if *id == ShipDesignId::PATROL_CORVETTE
    )));
}

#[test]
fn building_type_all_contains_three_variants() {
    let all = BuildingType::all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&BuildingType::AquacultureBay));
    assert!(all.contains(&BuildingType::FabricationYard));
    assert!(all.contains(&BuildingType::ScienceNexus));
}

#[test]
fn building_type_names_and_descriptions_are_non_empty() {
    for bt in BuildingType::all() {
        assert!(!bt.name().is_empty());
        assert!(!bt.description().is_empty());
    }
}

#[test]
fn building_type_costs_are_positive() {
    for bt in BuildingType::all() {
        assert!(bt.cost() > 0);
    }
}

#[test]
fn game_state_partial_eq() {
    let state_a = GameState::default();
    let state_b = GameState::default();
    assert_eq!(state_a, state_b);

    let state_c = GameState {
        turn: 5,
        ..GameState::default()
    };
    assert_ne!(state_a, state_c);
}

#[test]
fn tech_id_ordering() {
    let t1 = TechId(1);
    let t2 = TechId(2);
    assert!(t1 < t2);
}

#[test]
fn all_techs_returns_large_tree_entries() {
    let techs = all_techs();
    assert_eq!(techs.len(), 60);
    assert!(
        techs.iter().any(|t| t.name == "Orbital Engineering"),
        "Orbital Engineering tech must be present"
    );
    assert!(
        techs.iter().any(|t| t.name == "Hyperspace Cartography"),
        "Hyperspace Cartography tech must be present"
    );
    assert!(techs.iter().any(|t| {
        t.id == TechId(11)
            && t.unlocks
                .iter()
                .any(|u| matches!(u, TechUnlock::ShipDesign(ShipDesignId::TROOP_TRANSPORT)))
    }));
    assert!(techs.iter().any(|t| t.domain == TechDomain::Society));
    assert!(techs.iter().any(|t| t.tier == TechTier::VI));
}

#[test]
fn all_techs_have_unique_ids() {
    let techs = all_techs();
    let mut ids: Vec<TechId> = techs.iter().map(|t| t.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), techs.len(), "Tech IDs must be unique");
}

#[test]
fn all_tech_prerequisites_reference_existing_techs() {
    let ids: std::collections::BTreeSet<TechId> = all_techs().iter().map(|t| t.id).collect();
    for tech in all_techs() {
        for req in tech.prerequisites {
            assert!(
                ids.contains(req),
                "Tech {} has missing prerequisite id {}",
                tech.name,
                req.0
            );
        }
    }
}

#[test]
fn tech_graph_is_acyclic_and_has_no_self_dependencies() {
    use std::collections::BTreeSet;

    fn dfs(id: TechId, visiting: &mut BTreeSet<TechId>, visited: &mut BTreeSet<TechId>) -> bool {
        if visited.contains(&id) {
            return true;
        }
        if !visiting.insert(id) {
            return false;
        }
        let Some(node) = tech_by_id(id) else {
            return false;
        };
        if node.prerequisites.contains(&id) {
            return false;
        }
        for req in node.prerequisites {
            if !dfs(*req, visiting, visited) {
                return false;
            }
        }
        visiting.remove(&id);
        visited.insert(id);
        true
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for tech in all_techs() {
        assert!(
            dfs(tech.id, &mut visiting, &mut visited),
            "cycle detected involving tech {}",
            tech.name
        );
    }
}

#[test]
fn all_techs_have_positive_costs() {
    for tech in all_techs() {
        assert!(tech.cost > 0, "Tech {} must have positive cost", tech.name);
    }
}

#[test]
fn all_techs_have_non_empty_names_and_descriptions() {
    for tech in all_techs() {
        assert!(!tech.name.is_empty());
        assert!(!tech.description.is_empty());
    }
}

#[test]
fn tech_display_order_is_stable_and_unique() {
    let mut orders: Vec<u16> = all_techs().iter().map(|t| t.display_order).collect();
    let mut sorted = orders.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), orders.len(), "display_order must be unique");
    orders.sort_unstable();
    assert_eq!(
        orders,
        (1..=all_techs().len() as u16).collect::<Vec<_>>(),
        "display_order must be contiguous for deterministic rendering"
    );
}

#[test]
fn tech_rarity_tag_and_future_hook_metadata_is_present() {
    let techs = all_techs();
    assert!(techs.iter().all(|t| !t.tags.is_empty()));
    assert!(techs.iter().any(|t| t.rarity == TechRarity::Rare));
    assert!(techs.iter().any(|t| t.rarity == TechRarity::Breakthrough));
    assert!(techs.iter().any(|t| t.rarity == TechRarity::Dangerous));
    assert!(techs.iter().any(|t| t.future_hook));
    assert!(techs.iter().any(|t| !t.future_hook));
}

#[test]
fn tech_with_no_prerequisites_is_available() {
    assert!(
        is_tech_available(&[], TechId(1)),
        "tier-1 root tech should be available with no completed prerequisites"
    );
}

#[test]
fn tech_with_unmet_prerequisites_is_locked() {
    assert!(
        !is_tech_available(&[], TechId(6)),
        "Drift Mapping should be locked until Neutrino Sensors is completed"
    );
}

#[test]
fn completed_prerequisite_unlocks_dependent_tech() {
    assert!(
        is_tech_available(&[TechId(3)], TechId(6)),
        "completing Neutrino Sensors should unlock Drift Mapping"
    );
}

#[test]
fn available_tech_ids_order_is_deterministic() {
    let completed_unsorted = vec![TechId(5), TechId(2), TechId(3)];
    let first = available_tech_ids(&completed_unsorted);
    let second = available_tech_ids(&completed_unsorted);
    assert_eq!(
        first, second,
        "available tech ordering must be deterministic"
    );
    assert_eq!(
        first,
        vec![
            TechId(1),
            TechId(4),
            TechId(6),
            TechId(7),
            TechId(9),
            TechId(10),
            TechId(12),
            TechId(22),
            TechId(30),
            TechId(35),
            TechId(45),
            TechId(52),
        ],
        "available tech order should follow static deterministic tech definition order"
    );
}

#[test]
fn important_unlock_chain_targets_are_reachable() {
    let ids: std::collections::BTreeSet<TechId> = all_techs().iter().map(|t| t.id).collect();
    for id in [
        TechId::VOID_PROPULSION,
        TechId::HABITAT_SEEDING,
        TechId::SURVEY_DRONES,
        TechId::ORBITAL_ENGINEERING,
        TechId::PERIMETER_DEFENSE,
        TechId::FLEET_COORDINATION,
        TechId::BATTLE_DOCTRINE,
        TechId::HYPERSPACE_CARTOGRAPHY,
        TechId(10),
    ] {
        assert!(
            ids.contains(&id),
            "required unlock chain tech {:?} missing",
            id
        );
    }
}

#[test]
fn research_state_default_is_empty() {
    let rs = ResearchState::default();
    assert!(rs.current_tech.is_none());
    assert_eq!(rs.progress, 0);
    assert!(rs.queue.is_empty());
    assert!(rs.completed.is_empty());
}

#[test]
fn empire_research_defaults_to_empty() {
    let state = GameState::default();
    // Default state has no empires, but we can construct one directly
    let empire = Empire {
        id: EmpireId(1),
        name: "Test".to_string(),
        credits: 0,
        research_points: 0,
        home_star: StarId(1),
        research: ResearchState::default(),
        food: 0,
        empire_def: None,
    };
    assert!(empire.research.current_tech.is_none());
    assert!(empire.research.queue.is_empty());
    assert!(empire.research.completed.is_empty());
    let _ = state;
}

#[test]
fn scout_mission_fields() {
    let mission = ScoutMission {
        fleet: FleetId(1),
        destination: StarId(5),
        turns_remaining: 3,
        origin: StarId(0),
        total_duration: 3,
    };
    assert_eq!(mission.fleet, FleetId(1));
    assert_eq!(mission.destination, StarId(5));
    assert_eq!(mission.turns_remaining, 3);
}

#[test]
fn game_state_default_has_empty_exploration() {
    let state = GameState::default();
    assert!(state.explored_stars.is_empty());
    assert!(state.scout_missions.is_empty());
    assert!(state.survey_missions.is_empty());
}

#[test]
fn game_state_partial_eq_considers_explored_stars() {
    let mut state_a = GameState::default();
    let state_b = GameState::default();
    assert_eq!(state_a, state_b);

    state_a.explored_stars.insert(StarId(1));
    assert_ne!(state_a, state_b);
}

#[test]
fn game_state_partial_eq_considers_scout_missions() {
    let mut state_a = GameState::default();
    let state_b = GameState::default();
    assert_eq!(state_a, state_b);

    state_a.scout_missions.insert(
        FleetId(1),
        ScoutMission {
            fleet: FleetId(1),
            destination: StarId(2),
            turns_remaining: 2,
            origin: StarId(0),
            total_duration: 2,
        },
    );
    assert_ne!(state_a, state_b);
}

#[test]
fn game_state_partial_eq_considers_survey_missions() {
    let mut state_a = GameState::default();
    let state_b = GameState::default();
    assert_eq!(state_a, state_b);

    state_a.survey_missions.insert(
        FleetId(1),
        SurveyMission {
            fleet: FleetId(1),
            star: StarId(2),
            planet_index: 0,
            turns_remaining: 2,
        },
    );
    assert_ne!(state_a, state_b);
}

#[test]
fn game_state_partial_eq_considers_fleet_missions() {
    let mut state_a = GameState::default();
    let state_b = GameState::default();
    assert_eq!(state_a, state_b);

    state_a.fleet_missions.insert(
        FleetId(1),
        FleetMission {
            fleet: FleetId(1),
            destination: StarId(2),
            turns_remaining: 2,
            origin: StarId(0),
            total_duration: 2,
        },
    );
    assert_ne!(state_a, state_b);
}

#[test]
fn fleet_mission_fields() {
    let mission = FleetMission {
        fleet: FleetId(3),
        destination: StarId(7),
        turns_remaining: 2,
        origin: StarId(1),
        total_duration: 2,
    };
    assert_eq!(mission.fleet, FleetId(3));
    assert_eq!(mission.destination, StarId(7));
    assert_eq!(mission.turns_remaining, 2);
}

#[test]
fn fleet_location_at_star() {
    let mut state = GameState::default();
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: EmpireId(1),
            location: StarId(5),
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    match state.fleet_location(FleetId(1)) {
        Some(FleetLocation::AtStar(id)) => assert_eq!(id, StarId(5)),
        other => panic!("Expected AtStar, got {:?}", other),
    }
}

#[test]
fn fleet_location_travelling_via_fleet_mission() {
    let mut state = GameState::default();
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: EmpireId(1),
            location: StarId(5),
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    state.fleet_missions.insert(
        FleetId(1),
        FleetMission {
            fleet: FleetId(1),
            destination: StarId(9),
            turns_remaining: 2,
            origin: StarId(5),
            total_duration: 2,
        },
    );
    match state.fleet_location(FleetId(1)) {
        Some(FleetLocation::Travelling {
            destination,
            turns_remaining,
        }) => {
            assert_eq!(destination, StarId(9));
            assert_eq!(turns_remaining, 2);
        }
        other => panic!("Expected Travelling, got {:?}", other),
    }
}

#[test]
fn fleet_location_travelling_via_scout_mission() {
    let mut state = GameState::default();
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: EmpireId(1),
            location: StarId(5),
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    state.scout_missions.insert(
        FleetId(1),
        ScoutMission {
            fleet: FleetId(1),
            destination: StarId(11),
            turns_remaining: 3,
            origin: StarId(5),
            total_duration: 3,
        },
    );
    match state.fleet_location(FleetId(1)) {
        Some(FleetLocation::Travelling {
            destination,
            turns_remaining,
        }) => {
            assert_eq!(destination, StarId(11));
            assert_eq!(turns_remaining, 3);
        }
        other => panic!("Expected Travelling, got {:?}", other),
    }
}

#[test]
fn fleet_location_at_star_via_survey_mission() {
    let mut state = GameState::default();
    state.fleets.insert(
        FleetId(1),
        Fleet {
            id: FleetId(1),
            owner: EmpireId(1),
            location: StarId(5),
            ships: 1,
            kind: FleetKind::Science,
            strength: 1,
            integrity: 100,
        },
    );
    state.survey_missions.insert(
        FleetId(1),
        SurveyMission {
            fleet: FleetId(1),
            star: StarId(5),
            planet_index: 1,
            turns_remaining: 2,
        },
    );
    match state.fleet_location(FleetId(1)) {
        Some(FleetLocation::AtStar(id)) => assert_eq!(id, StarId(5)),
        other => panic!("Expected AtStar, got {:?}", other),
    }
}

#[test]
fn survey_mission_fields() {
    let mission = SurveyMission {
        fleet: FleetId(3),
        star: StarId(7),
        planet_index: 2,
        turns_remaining: 2,
    };
    assert_eq!(mission.fleet, FleetId(3));
    assert_eq!(mission.star, StarId(7));
    assert_eq!(mission.planet_index, 2);
    assert_eq!(mission.turns_remaining, 2);
}

#[test]
fn fleet_location_none_for_missing_fleet() {
    let state = GameState::default();
    assert!(state.fleet_location(FleetId(999)).is_none());
}

#[test]
fn game_state_default_has_empty_fleet_missions() {
    let state = GameState::default();
    assert!(state.fleet_missions.is_empty());
}

#[test]
fn planet_class_all_contains_all_variants() {
    let all = PlanetClass::all();
    assert_eq!(all.len(), 6);
    assert!(all.contains(&PlanetClass::Terran));
    assert!(all.contains(&PlanetClass::Desert));
    assert!(all.contains(&PlanetClass::Oceanic));
    assert!(all.contains(&PlanetClass::Volcanic));
    assert!(all.contains(&PlanetClass::Frozen));
    assert!(all.contains(&PlanetClass::Barren));
}

#[test]
fn planet_class_names_are_non_empty() {
    for class in PlanetClass::all() {
        assert!(!class.name().is_empty());
    }
}

#[test]
fn planet_size_infrastructure_capacities() {
    assert_eq!(PlanetSize::Tiny.surface_slots(), 3);
    assert_eq!(PlanetSize::Tiny.orbital_slots(), 1);
    assert_eq!(PlanetSize::Small.surface_slots(), 5);
    assert_eq!(PlanetSize::Small.orbital_slots(), 1);
    assert_eq!(PlanetSize::Medium.surface_slots(), 7);
    assert_eq!(PlanetSize::Medium.orbital_slots(), 2);
    assert_eq!(PlanetSize::Large.surface_slots(), 10);
    assert_eq!(PlanetSize::Large.orbital_slots(), 3);
    assert_eq!(PlanetSize::Massive.surface_slots(), 14);
    assert_eq!(PlanetSize::Massive.orbital_slots(), 4);
}

#[test]
fn colony_surface_slot_availability_starts_empty() {
    let colony = Colony {
        id: ColonyId(1),
        star: StarId(1),
        planet_index: 0,
        owner: EmpireId(1),
        population: 10,
        production: 10,
        prod_pct: 50,
        research_pct: 50,
        build_queue: Vec::new(),
        accumulated_production: 0,
        buildings: Vec::new(),
        surface_installations: Vec::new(),
        orbital_installations: Vec::new(),
        stability: 100,
        role: ColonyRole::Balanced,
        rally_point: None,
    };

    assert!(colony.can_place_surface_building(PlanetSize::Medium));
    assert!(colony.can_place_orbital_installation(PlanetSize::Medium));
    assert_eq!(colony.available_surface_slots(PlanetSize::Medium), 7);
    assert_eq!(colony.available_orbital_slots(PlanetSize::Medium), 2);
}

#[test]
fn colony_surface_slots_fill_and_reject_overflow() {
    let mut colony = Colony {
        id: ColonyId(1),
        star: StarId(1),
        planet_index: 0,
        owner: EmpireId(1),
        population: 10,
        production: 10,
        prod_pct: 50,
        research_pct: 50,
        build_queue: Vec::new(),
        accumulated_production: 0,
        buildings: Vec::new(),
        surface_installations: vec![BuildingType::FabricationYard],
        orbital_installations: Vec::new(),
        stability: 100,
        role: ColonyRole::Balanced,
        rally_point: None,
    };

    // With 1 surface building on Tiny (capacity 3), we have 2 left
    assert!(colony.can_place_surface_building(PlanetSize::Tiny));
    assert_eq!(colony.available_surface_slots(PlanetSize::Tiny), 2);

    // Fill to capacity
    colony
        .surface_installations
        .push(BuildingType::ScienceNexus);
    colony
        .surface_installations
        .push(BuildingType::FabricationYard);
    assert!(!colony.can_place_surface_building(PlanetSize::Tiny));
    assert_eq!(colony.available_surface_slots(PlanetSize::Tiny), 0);
}

// ── ScenarioSetup / GalaxySize tests ───────────────────────────────────

#[test]
fn scenario_setup_validate_accepts_valid_configs() {
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_ok());

    let setup4 = ScenarioSetup {
        ai_empire_count: 4,
        ..setup.clone()
    };
    assert!(setup4.validate().is_ok());
}

#[test]
fn scenario_setup_validate_rejects_zero_ai_count() {
    let setup = ScenarioSetup {
        seed: 1,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 0,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_err());
}

#[test]
fn scenario_setup_validate_rejects_too_many_ai() {
    let setup = ScenarioSetup {
        seed: 1,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 5,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_err());
}

#[test]
fn scenario_setup_validate_rejects_bad_sector_count() {
    let setup_low = ScenarioSetup {
        seed: 1,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: Some(1),
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup_low.validate().is_err());

    let setup_high = ScenarioSetup {
        sector_count_override: Some(9),
        ..setup_low.clone()
    };
    assert!(setup_high.validate().is_err());

    let setup_ok = ScenarioSetup {
        sector_count_override: Some(4),
        ..setup_low
    };
    assert!(setup_ok.validate().is_ok());
}

#[test]
fn galaxy_size_star_and_sector_counts() {
    assert_eq!(GalaxySize::Small.default_star_count(), 10);
    assert_eq!(GalaxySize::Small.default_sector_count(), 2);

    assert_eq!(GalaxySize::Medium.default_star_count(), 20);
    assert_eq!(GalaxySize::Medium.default_sector_count(), 4);

    assert_eq!(GalaxySize::Large.default_star_count(), 40);
    assert_eq!(GalaxySize::Large.default_sector_count(), 6);
}

#[test]
fn scenario_setup_effective_counts_respect_override() {
    let setup = ScenarioSetup {
        seed: 0,
        galaxy_size: GalaxySize::Small,
        ai_empire_count: 1,
        sector_count_override: Some(5),
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    // Star count comes from galaxy_size
    assert_eq!(setup.effective_star_count(), 10);
    // Sector count comes from override
    assert_eq!(setup.effective_sector_count(), 5);
}

#[test]
fn scenario_setup_effective_sector_count_clamped() {
    let setup_low = ScenarioSetup {
        seed: 0,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: Some(1), // below min
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert_eq!(setup_low.effective_sector_count(), 2); // clamped to 2

    let setup_high = ScenarioSetup {
        sector_count_override: Some(20), // above max
        ..setup_low
    };
    assert_eq!(setup_high.effective_sector_count(), 8); // clamped to 8
}

// ── Empire Definition tests ─────────────────────────────────────────────

#[test]
fn all_empire_definitions_returns_eight_entries() {
    assert_eq!(all_empire_definitions().len(), 8);
}

#[test]
fn empire_definition_ids_are_unique_and_sequential() {
    let defs = all_empire_definitions();
    for (i, def) in defs.iter().enumerate() {
        assert_eq!(def.id.0 as usize, i, "Empire def #{i} has wrong id");
    }
}

#[test]
fn empire_definition_by_id_finds_existing() {
    for def in all_empire_definitions() {
        let found = empire_definition_by_id(def.id);
        assert!(
            found.is_some(),
            "empire_definition_by_id should find id {}",
            def.id.0
        );
        assert_eq!(found.unwrap().name, def.name);
    }
}

#[test]
fn empire_definition_by_id_returns_none_for_unknown() {
    assert!(empire_definition_by_id(EmpireDefinitionId(99)).is_none());
}

#[test]
fn empire_names_are_distinct() {
    let defs = all_empire_definitions();
    let names: std::collections::BTreeSet<_> = defs.iter().map(|d| d.name).collect();
    assert_eq!(names.len(), defs.len(), "All empire names must be unique");
}

#[test]
fn empire_trait_modifiers_default_is_zero() {
    let m = EmpireTraitModifiers::default();
    assert_eq!(m.industry_per_colony, 0);
    assert_eq!(m.science_per_colony, 0);
    assert_eq!(m.credits_per_colony, 0);
    assert_eq!(m.food_per_colony, 0);
}

#[test]
fn playstyle_tag_labels_are_nonempty() {
    let tags = [
        PlaystyleTag::Industrial,
        PlaystyleTag::Scientific,
        PlaystyleTag::Expansionist,
        PlaystyleTag::Militarist,
        PlaystyleTag::Agrarian,
        PlaystyleTag::Diplomatic,
    ];
    for tag in &tags {
        assert!(!tag.label().is_empty());
    }
}

#[test]
fn ai_doctrine_helpers_are_stable_and_non_empty() {
    let doctrines = [
        AiDoctrine::Explorer,
        AiDoctrine::Technologist,
        AiDoctrine::Merchant,
        AiDoctrine::Imperial,
        AiDoctrine::Militarist,
        AiDoctrine::Industrialist,
        AiDoctrine::Expansionist,
        AiDoctrine::Isolationist,
        AiDoctrine::Biologist,
    ];
    for doctrine in doctrines {
        assert!(!doctrine.label().is_empty());
        assert!(!doctrine.short_code().is_empty());
        assert!(!doctrine.short_summary().is_empty());
    }
}

#[test]
fn empire_doctrine_weights_match_faction_intent() {
    let concord = empire_definition_by_id(EmpireDefinitionId(6)).expect("Terran Concord");
    let dominion = empire_definition_by_id(EmpireDefinitionId(7)).expect("Terran Dominion");
    let elarith = empire_definition_by_id(EmpireDefinitionId(5)).expect("Elarith Confluence");
    let thalori = empire_definition_by_id(EmpireDefinitionId(3)).expect("Thalori Exchange");

    assert!(concord.doctrine_weight(AiDoctrine::Explorer) >= 8);
    assert!(concord.doctrine_weight(AiDoctrine::Technologist) >= 7);
    assert!(concord.doctrine_weight(AiDoctrine::Merchant) >= 6);
    assert!(dominion.doctrine_weight(AiDoctrine::Imperial) >= 8);
    assert!(dominion.doctrine_weight(AiDoctrine::Militarist) >= 8);
    assert!(dominion.doctrine_weight(AiDoctrine::Industrialist) >= 7);
    assert!(
        concord.doctrine_weight(AiDoctrine::Militarist)
            < dominion.doctrine_weight(AiDoctrine::Militarist)
    );
    assert!(
        concord.doctrine_weight(AiDoctrine::Imperial)
            < dominion.doctrine_weight(AiDoctrine::Imperial)
    );
    assert!(elarith.doctrine_weight(AiDoctrine::Isolationist) >= 7);
    assert!(thalori.doctrine_weight(AiDoctrine::Merchant) >= 8);
}

#[test]
fn scenario_setup_validates_valid_empire_def() {
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(0)),
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_ok());
}

#[test]
fn scenario_setup_rejects_unknown_empire_def() {
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: Some(EmpireDefinitionId(99)),
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    let err = setup.validate();
    assert!(err.is_err(), "Unknown empire def should fail validation");
    assert!(
        err.unwrap_err().contains("99"),
        "Error should mention the invalid id"
    );
}

#[test]
fn scenario_setup_none_empire_def_is_valid() {
    let setup = ScenarioSetup {
        seed: 42,
        galaxy_size: GalaxySize::Medium,
        ai_empire_count: 1,
        sector_count_override: None,
        difficulty: DifficultyLevel::Standard,
        player_empire_def: None,
        victory_settings: crate::state::VictorySettings::default_v1(),
    };
    assert!(setup.validate().is_ok());
}

fn make_supply_test_state() -> GameState {
    let mut state = GameState::default();
    let empire_id = EmpireId(1);
    state.player_empire = empire_id;
    state.empires.insert(
        empire_id,
        Empire {
            id: empire_id,
            name: "Player".to_string(),
            credits: 0,
            research_points: 0,
            home_star: StarId(1),
            research: ResearchState::default(),
            food: 0,
            empire_def: None,
        },
    );
    state.stars.insert(
        StarId(1),
        Star {
            id: StarId(1),
            sector: SectorId(1),
            name: "Home".to_string(),
            x: 0,
            y: 0,
            spectral_class: SpectralClass::G,
            planets: vec![],
        },
    );
    state.stars.insert(
        StarId(2),
        Star {
            id: StarId(2),
            sector: SectorId(1),
            name: "Near".to_string(),
            x: 200,
            y: 0,
            spectral_class: SpectralClass::K,
            planets: vec![],
        },
    );
    state.stars.insert(
        StarId(3),
        Star {
            id: StarId(3),
            sector: SectorId(2),
            name: "Far".to_string(),
            x: 900,
            y: 0,
            spectral_class: SpectralClass::M,
            planets: vec![],
        },
    );
    state.colonies.insert(
        ColonyId(1),
        Colony {
            id: ColonyId(1),
            star: StarId(1),
            planet_index: 0,
            owner: empire_id,
            population: 10,
            production: 10,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state.colonies.insert(
        ColonyId(2),
        Colony {
            id: ColonyId(2),
            star: StarId(2),
            planet_index: 0,
            owner: empire_id,
            population: 8,
            production: 8,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state.colonies.insert(
        ColonyId(3),
        Colony {
            id: ColonyId(3),
            star: StarId(3),
            planet_index: 0,
            owner: empire_id,
            population: 8,
            production: 8,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );
    state
}

#[test]
fn visible_resources_require_survey_and_discovery_tech() {
    let mut planet = Planet {
        name: "Probe I".to_string(),
        class: PlanetClass::Frozen,
        size: PlanetSize::Medium,
        colony: None,
        habitable: true,
        surveyed: false,
        specials: vec![],
        resources: vec![
            StrategicResource::Helium3,
            StrategicResource::DarkMatter,
            StrategicResource::PrecursorDatacores,
        ],
        anomalies: vec![],
        ancient_ruins_collected: false,
    };

    let none_visible = visible_resources_for_empire(&planet, &[]);
    assert!(
        none_visible.is_empty(),
        "unsurveyed planets must not reveal resources"
    );

    planet.surveyed = true;
    let early_visible = visible_resources_for_empire(&planet, &[TechId::ADVANCED_SURVEY]);
    assert!(early_visible.contains(&StrategicResource::Helium3));
    assert!(
        !early_visible.contains(&StrategicResource::DarkMatter),
        "dark matter should stay hidden before advanced sensor net"
    );

    let late_visible = visible_resources_for_empire(
        &planet,
        &[TechId::ADVANCED_SURVEY, TechId::PAN_GALACTIC_SENSOR_NET],
    );
    assert!(late_visible.contains(&StrategicResource::DarkMatter));
    assert!(late_visible.contains(&StrategicResource::PrecursorDatacores));
}

#[test]
fn resource_extraction_requires_control_supply_and_is_blockade_sensitive() {
    let mut state = GameState::default();
    let owner = EmpireId(1);
    let star_id = StarId(1);
    let colony_id = ColonyId(1);

    state.player_empire = owner;
    state.empires.insert(
        owner,
        Empire {
            id: owner,
            name: "Owner".to_string(),
            credits: 0,
            research_points: 0,
            home_star: star_id,
            research: ResearchState {
                completed: vec![TechId::ADVANCED_SURVEY, TechId(14)],
                ..ResearchState::default()
            },
            food: 0,
            empire_def: None,
        },
    );

    state.stars.insert(
        star_id,
        Star {
            id: star_id,
            sector: SectorId(0),
            name: "Anchor".to_string(),
            x: 0,
            y: 0,
            spectral_class: SpectralClass::F,
            planets: vec![Planet {
                name: "Anchor I".to_string(),
                class: PlanetClass::Frozen,
                size: PlanetSize::Medium,
                colony: Some(colony_id),
                habitable: true,
                surveyed: true,
                specials: vec![],
                resources: vec![StrategicResource::QuantumCrystals],
                anomalies: vec![],
                ancient_ruins_collected: false,
            }],
        },
    );
    state.colonies.insert(
        colony_id,
        Colony {
            id: colony_id,
            star: star_id,
            planet_index: 0,
            owner,
            population: 3,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: vec![],
            accumulated_production: 0,
            buildings: vec![BuildingType::ScienceNexus],
            surface_installations: vec![],
            orbital_installations: vec![],
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
        },
    );

    state
        .colony_supply
        .insert(colony_id, ColonySupplyState::Connected);
    assert!(
        state.colony_can_extract_resource(colony_id, StrategicResource::QuantumCrystals),
        "connected colony with required building should extract"
    );

    state
        .colony_supply
        .insert(colony_id, ColonySupplyState::Isolated);
    assert!(
        !state.colony_can_extract_resource(colony_id, StrategicResource::QuantumCrystals),
        "isolated colony should lose strategic extraction access"
    );

    state
        .colony_supply
        .insert(colony_id, ColonySupplyState::Connected);
    state.colony_blockade.insert(colony_id, EmpireId(2));
    assert!(
        !state.colony_can_extract_resource(colony_id, StrategicResource::QuantumCrystals),
        "blockaded colony should have extraction disrupted"
    );
}

#[test]
fn supply_connectivity_marks_capital_connected() {
    let state = make_supply_test_state();
    let supply = state.recompute_colony_supply();
    assert_eq!(
        supply.get(&ColonyId(1)),
        Some(&ColonySupplyState::Connected)
    );
}

#[test]
fn supply_connectivity_marks_nearby_valid_route_connected() {
    let state = make_supply_test_state();
    let supply = state.recompute_colony_supply();
    assert_eq!(
        supply.get(&ColonyId(2)),
        Some(&ColonySupplyState::Connected)
    );
}

#[test]
fn supply_connectivity_marks_no_route_isolated() {
    let state = make_supply_test_state();
    let supply = state.recompute_colony_supply();
    assert_eq!(supply.get(&ColonyId(3)), Some(&ColonySupplyState::Isolated));
}

#[test]
fn supply_connectivity_lane_enables_connection_with_tech() {
    let mut state = make_supply_test_state();
    let lane = HyperspaceLane::new(StarId(2), StarId(3)).expect("distinct stars");
    state.hyperspace_lanes.insert(lane);
    state.known_hyperspace_lanes.insert(lane);
    state
        .empires
        .get_mut(&state.player_empire)
        .expect("player empire")
        .research
        .completed
        .push(TechId::HYPERSPACE_CARTOGRAPHY);

    let supply = state.recompute_colony_supply();
    assert_eq!(
        supply.get(&ColonyId(3)),
        Some(&ColonySupplyState::Connected)
    );
}

#[test]
fn supply_connectivity_is_deterministic_for_same_state() {
    let mut state = make_supply_test_state();
    let lane = HyperspaceLane::new(StarId(2), StarId(3)).expect("distinct stars");
    state.hyperspace_lanes.insert(lane);
    state.known_hyperspace_lanes.insert(lane);
    state
        .empires
        .get_mut(&state.player_empire)
        .expect("player empire")
        .research
        .completed
        .push(TechId::HYPERSPACE_CARTOGRAPHY);

    let a = state.recompute_colony_supply();
    let b = state.recompute_colony_supply();
    assert_eq!(a, b);
}
