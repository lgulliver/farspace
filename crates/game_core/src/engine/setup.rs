use super::*;
use crate::victory::evaluate_victory_end_turn;
use rand::SeedableRng;
use rand::rngs::ChaCha8Rng;
use rand::seq::SliceRandom;

impl Engine {
    /// Create a new game engine with the given seed (default setup, 1 AI empire).
    pub fn new(seed: u64) -> Self {
        Self::new_from_setup(ScenarioSetup::default_for_seed(seed))
    }

    /// Create a new game engine from a validated `ScenarioSetup`.
    ///
    /// # Panics
    /// Panics if the setup is invalid (call `setup.validate()` first).
    pub fn new_from_setup(setup: ScenarioSetup) -> Self {
        setup
            .validate()
            .expect("ScenarioSetup must be valid before calling Engine::new_from_setup");
        let seed = setup.seed;
        let star_count = setup.effective_star_count();
        let sector_count = setup.effective_sector_count();
        let ai_count = setup.ai_empire_count as usize;

        let rng = ChaCha8Rng::seed_from_u64(seed);
        let galaxy = generate_galaxy_with_config(seed, star_count, sector_count);

        let mut sectors = BTreeMap::new();
        for sector in &galaxy.sectors {
            sectors.insert(sector.id, sector.clone());
        }

        let mut stars = BTreeMap::new();
        for star in &galaxy.stars {
            stars.insert(star.id, star.clone());
        }

        let stars_vec = &galaxy.stars;
        let home_star =
            find_home_star(stars_vec).expect("Galaxy should have at least one habitable star");
        let home_star_id = home_star.id;

        let ai_home_star_ids = find_ai_home_stars(stars_vec, home_star_id, ai_count);
        assert!(
            ai_home_star_ids.len() == ai_count,
            "Galaxy does not have enough habitable stars for {} AI empires",
            ai_count
        );

        const AI_EMPIRE_NAMES: &[&str] = &[
            "Veth Dominion",
            "Keth Ascendancy",
            "Sorn Collective",
            "Drosan Republic",
        ];

        let all_defs = crate::state::all_empire_definitions();
        let player_def_id = setup
            .player_empire_def
            .unwrap_or(crate::state::EmpireDefinitionId(0));
        let mut remaining_def_ids: Vec<crate::state::EmpireDefinitionId> = all_defs
            .iter()
            .filter(|d| d.id != player_def_id)
            .map(|d| d.id)
            .collect();
        let mut empire_assign_rng = ChaCha8Rng::seed_from_u64(seed ^ EMPIRE_ASSIGN_SALT);
        remaining_def_ids.shuffle(&mut empire_assign_rng);
        let ai_def_ids: Vec<crate::state::EmpireDefinitionId> =
            remaining_def_ids.into_iter().take(ai_count).collect();

        let player_empire_id = EmpireId(1);
        let mut empires = BTreeMap::new();
        let player_def = crate::state::empire_definition_by_id(player_def_id)
            .expect("player empire definition must be valid");
        empires.insert(
            player_empire_id,
            Empire {
                id: player_empire_id,
                name: player_def.name.to_string(),
                credits: 100,
                research_points: 0,
                home_star: home_star_id,
                research: ResearchState::default(),
                food: 0,
                empire_def: Some(player_def_id),
            },
        );

        let mut ai_empire_ids: Vec<EmpireId> = Vec::with_capacity(ai_count);
        for i in 0..ai_count {
            let ai_empire_id = EmpireId(2 + i as u64);
            ai_empire_ids.push(ai_empire_id);
            let ai_def_id = ai_def_ids.get(i).copied();
            let ai_name = ai_def_id
                .and_then(crate::state::empire_definition_by_id)
                .map(|d| d.name.to_string())
                .unwrap_or_else(|| AI_EMPIRE_NAMES[i].to_string());
            empires.insert(
                ai_empire_id,
                Empire {
                    id: ai_empire_id,
                    name: ai_name,
                    credits: 100,
                    research_points: 0,
                    home_star: ai_home_star_ids[i],
                    research: ResearchState::default(),
                    food: 0,
                    empire_def: ai_def_id,
                },
            );
        }

        let mut colonies = BTreeMap::new();
        let player_colony_id = ColonyId(1);
        colonies.insert(
            player_colony_id,
            Colony {
                id: player_colony_id,
                star: home_star_id,
                planet_index: 0,
                owner: player_empire_id,
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
            },
        );

        for (i, &ai_empire_id) in ai_empire_ids.iter().enumerate() {
            let ai_colony_id = ColonyId(2 + i as u64);
            let ai_home_star_id = ai_home_star_ids[i];
            colonies.insert(
                ai_colony_id,
                Colony {
                    id: ai_colony_id,
                    star: ai_home_star_id,
                    planet_index: 0,
                    owner: ai_empire_id,
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
                },
            );
        }

        if let Some(star) = stars.get_mut(&home_star_id)
            && let Some(planet) = star.planets.get_mut(0)
        {
            planet.colony = Some(player_colony_id);
            planet.surveyed = true;
        }

        for (i, _ai_empire_id) in ai_empire_ids.iter().enumerate() {
            let ai_colony_id = ColonyId(2 + i as u64);
            let ai_home_star_id = ai_home_star_ids[i];
            if let Some(star) = stars.get_mut(&ai_home_star_id)
                && let Some(planet) = star.planets.get_mut(0)
            {
                planet.colony = Some(ai_colony_id);
                planet.surveyed = true;
            }
        }

        let next_colony_id = 2 + ai_count as u64;
        let mut next_fleet_id = 2 + ai_count as u64;

        let fleet_id = FleetId(1);
        let mut fleets = BTreeMap::new();
        fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: player_empire_id,
                location: home_star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );

        for (i, &ai_empire_id) in ai_empire_ids.iter().enumerate() {
            let ai_fleet_id = FleetId(2 + i as u64);
            let ai_home_star_id = ai_home_star_ids[i];
            fleets.insert(
                ai_fleet_id,
                Fleet {
                    id: ai_fleet_id,
                    owner: ai_empire_id,
                    location: ai_home_star_id,
                    ships: 1,
                    kind: FleetKind::Scout,
                    strength: 1,
                    integrity: 100,
                },
            );
        }

        let player_science_fleet_id = FleetId(next_fleet_id);
        next_fleet_id += 1;
        fleets.insert(
            player_science_fleet_id,
            Fleet {
                id: player_science_fleet_id,
                owner: player_empire_id,
                location: home_star_id,
                ships: 1,
                kind: FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );

        let explored_stars = initial_explored_stars(stars_vec, home_star_id);
        // Each AI empire starts with its own fog of war seeded around its
        // home star. The legacy shared set mirrors the first AI for old
        // consumers and pre-v40 save compatibility.
        let empire_explored_stars: BTreeMap<EmpireId, BTreeSet<StarId>> = ai_empire_ids
            .iter()
            .zip(ai_home_star_ids.iter())
            .map(|(&ai_id, &home)| (ai_id, initial_explored_stars(stars_vec, home)))
            .collect();
        let ai_explored_stars_first = ai_empire_ids
            .first()
            .and_then(|id| empire_explored_stars.get(id).cloned())
            .unwrap_or_default();

        let hyperspace_lanes: BTreeSet<HyperspaceLane> =
            generate_hyperspace_lanes(seed, &galaxy.sectors, stars_vec)
                .into_iter()
                .collect();
        let known_hyperspace_lanes = hyperspace_lanes
            .iter()
            .copied()
            .filter(|lane| explored_stars.contains(&lane.a()) && explored_stars.contains(&lane.b()))
            .collect();

        let legacy_ai_empire = ai_empire_ids.first().copied();

        let mut state = GameState {
            seed,
            turn: 1,
            sectors,
            stars,
            empires,
            colonies,
            fleets,
            player_empire: player_empire_id,
            rng,
            event_log: Vec::new(),
            next_colony_id,
            next_fleet_id,
            explored_stars,
            scout_missions: BTreeMap::new(),
            survey_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: legacy_ai_empire,
            ai_explored_stars: ai_explored_stars_first,
            empire_explored_stars,
            ai_relations: BTreeMap::new(),
            diplomacy: BTreeMap::new(),
            diplomacy_relationships: BTreeMap::new(),
            diplomacy_pending_communications: std::collections::VecDeque::new(),
            diplomacy_next_communication_id: 1,
            hyperspace_lanes,
            known_hyperspace_lanes,
            fleet_orders: BTreeMap::new(),
            fleet_roles: BTreeMap::new(),
            fleet_formations: BTreeMap::new(),
            fleet_names: BTreeMap::new(),
            scenario: Some(setup),
            ai_empires: ai_empire_ids,
            colony_supply: BTreeMap::new(),
            fleet_supply: BTreeMap::new(),
            colony_blockade: BTreeMap::new(),
            colony_unrest: BTreeMap::new(),
            colony_unrest_causes: BTreeMap::new(),
            colony_rebellion_risk_bp: BTreeMap::new(),
            colony_recent_conquest_turn: BTreeMap::new(),
            empire_resource_access: BTreeMap::new(),
            victory_status: crate::state::VictoryStatus::default(),
            galactic_dispatches: std::collections::VecDeque::new(),
            custom_designs: std::collections::BTreeMap::new(),
            next_custom_design_id: 0,
            fleet_custom_designs: std::collections::BTreeMap::new(),
            next_battle_report_id: 1,
            battle_reports: std::collections::VecDeque::new(),
            empire_intel: std::collections::BTreeMap::new(),
            sector_directives: std::collections::BTreeMap::new(),
            colony_automation: std::collections::BTreeMap::new(),
            last_colony_yields: std::collections::BTreeMap::new(),
            empire_trade_routes: std::collections::BTreeMap::new(),
            empire_trade_income: std::collections::BTreeMap::new(),
        };

        // Generate initial ship designs for all AI empires
        let ai_empire_ids_copy: Vec<_> = state.ai_empires.clone();
        for ai_empire_id in &ai_empire_ids_copy {
            crate::ai::ai_generate_designs(&mut state, *ai_empire_id);
        }

        let mut engine = Engine {
            state,
            last_turn_colony_supply: BTreeMap::new(),
            last_turn_colony_blockade: BTreeMap::new(),
            last_turn_trade_disrupted: BTreeSet::new(),
        };
        engine.refresh_colony_supply_statuses();
        engine.refresh_unrest_statuses();
        engine.state.empire_resource_access = engine.state.recompute_empire_resource_access();
        engine.last_turn_colony_supply = engine.state.colony_supply.clone();
        engine.last_turn_colony_blockade = engine.state.colony_blockade.clone();
        let (trade_routes, trade_income) = engine.state.recompute_empire_trade_routes();
        engine.state.empire_trade_routes = trade_routes;
        engine.state.empire_trade_income = trade_income;
        let completed_turn = engine.state.turn;
        let _ = evaluate_victory_end_turn(&mut engine.state, completed_turn);
        engine
    }

    /// Create an engine from existing state
    pub fn from_state(state: GameState) -> Self {
        let mut engine = Engine {
            state,
            last_turn_colony_supply: BTreeMap::new(),
            last_turn_colony_blockade: BTreeMap::new(),
            last_turn_trade_disrupted: BTreeSet::new(),
        };
        engine.refresh_colony_supply_statuses();
        engine.refresh_unrest_statuses();
        engine.state.empire_resource_access = engine.state.recompute_empire_resource_access();
        engine.last_turn_colony_supply = engine.state.colony_supply.clone();
        engine.last_turn_colony_blockade = engine.state.colony_blockade.clone();
        let (trade_routes, trade_income) = engine.state.recompute_empire_trade_routes();
        engine.state.empire_trade_routes = trade_routes;
        engine.state.empire_trade_income = trade_income;
        let completed_turn = engine.state.turn;
        let _ = evaluate_victory_end_turn(&mut engine.state, completed_turn);
        engine
    }
}

fn initial_explored_stars(stars: &[crate::state::Star], home_id: StarId) -> BTreeSet<StarId> {
    let mut explored = BTreeSet::new();
    explored.insert(home_id);

    let home = match stars.iter().find(|s| s.id == home_id) {
        Some(s) => s,
        None => return explored,
    };

    let mut neighbours: Vec<(i64, StarId)> = stars
        .iter()
        .filter(|s| s.id != home_id)
        .map(|s| {
            let dx = (s.x - home.x) as i64;
            let dy = (s.y - home.y) as i64;
            (dx * dx + dy * dy, s.id)
        })
        .collect();
    neighbours.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (_, star_id) in neighbours.into_iter().take(3) {
        explored.insert(star_id);
    }

    explored
}

fn find_ai_home_stars(stars: &[crate::state::Star], player_home: StarId, n: usize) -> Vec<StarId> {
    if n == 0 {
        return vec![];
    }

    let mut candidates: Vec<&crate::state::Star> = stars
        .iter()
        .filter(|s| s.id != player_home && s.planets.iter().any(|p| p.habitable))
        .collect();
    candidates.sort_by_key(|s| s.id);

    let player_star = match stars.iter().find(|s| s.id == player_home) {
        Some(s) => s,
        None => return vec![],
    };

    let sq_dist = |a: &crate::state::Star, b: &crate::state::Star| -> i64 {
        let dx = (a.x - b.x) as i64;
        let dy = (a.y - b.y) as i64;
        dx * dx + dy * dy
    };

    let mut chosen: Vec<StarId> = Vec::with_capacity(n);
    let mut chosen_stars: Vec<&crate::state::Star> = vec![player_star];

    for _ in 0..n {
        let best = candidates.iter().max_by_key(|&&c| {
            let min_dist = chosen_stars
                .iter()
                .map(|&cs| sq_dist(c, cs))
                .min()
                .unwrap_or(0);
            (min_dist, c.id.0)
        });
        match best {
            Some(&best_star) => {
                chosen.push(best_star.id);
                chosen_stars.push(best_star);
                candidates.retain(|c| c.id != best_star.id);
            }
            None => break,
        }
    }

    chosen
}
