//! Game engine - command processing and turn execution

use crate::commands::Command;
use crate::deterministic::sorted_colony_ids;
use crate::events::Event;
use crate::galaxy::{find_home_star, generate_galaxy};
use crate::state::{
    all_techs, BuildItem, Colony, ColonyId, Empire, EmpireId, Fleet, FleetId, FleetKind,
    FleetMission, GameState, RelationshipStatus, ResearchState, ScoutMission, StarId, TechId,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

/// Number of turns for a scout to travel to an unexplored system
pub(crate) const SCOUT_TRAVEL_TURNS: u32 = 3;

/// Return the number of travel turns for a fleet moving the given squared distance.
///
/// Buckets (squared distance):
/// * <= 100_000  -> 1 turn
/// * <= 400_000  -> 2 turns
/// * else        -> 3 turns
fn fleet_travel_turns(squared_distance: i64) -> u32 {
    if squared_distance <= 100_000 {
        1
    } else if squared_distance <= 400_000 {
        2
    } else {
        3
    }
}

/// The game engine processes commands and manages game state
#[derive(Debug)]
pub struct Engine {
    pub state: GameState,
}

impl Engine {
    /// Create a new game engine with the given seed
    pub fn new(seed: u64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let stars_vec = generate_galaxy(seed, 20);

        let mut stars = BTreeMap::new();
        for star in stars_vec.iter() {
            stars.insert(star.id, star.clone());
        }

        // Find a suitable home star for the player
        let home_star =
            find_home_star(&stars_vec).expect("Galaxy should have at least one habitable star");
        let home_star_id = home_star.id;

        // Find the AI home star: farthest habitable star from the player
        let ai_home_star_id = find_ai_home_star(&stars_vec, home_star_id)
            .expect("Galaxy must have at least two habitable stars");

        // Create player empire
        let player_empire_id = EmpireId(1);
        let ai_empire_id = EmpireId(2);
        let mut empires = BTreeMap::new();
        empires.insert(
            player_empire_id,
            Empire {
                id: player_empire_id,
                name: "Terran Federation".to_string(),
                credits: 100,
                research_points: 0,
                home_star: home_star_id,
                research: ResearchState::default(),
                food: 0,
            },
        );
        empires.insert(
            ai_empire_id,
            Empire {
                id: ai_empire_id,
                name: "Veth Dominion".to_string(),
                credits: 100,
                research_points: 0,
                home_star: ai_home_star_id,
                research: ResearchState::default(),
                food: 0,
            },
        );

        // Create initial player colony (ColonyId 1)
        let colony_id = ColonyId(1);
        let mut colonies = BTreeMap::new();
        colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
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
            },
        );

        // Create initial AI colony (ColonyId 2)
        let ai_colony_id = ColonyId(2);
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
            },
        );

        // Update player home star's planet 0 to reference the player colony
        if let Some(star) = stars.get_mut(&home_star_id) {
            if let Some(planet) = star.planets.get_mut(0) {
                planet.colony = Some(colony_id);
            }
        }

        // Update AI home star's planet 0 to reference the AI colony
        if let Some(star) = stars.get_mut(&ai_home_star_id) {
            if let Some(planet) = star.planets.get_mut(0) {
                planet.colony = Some(ai_colony_id);
            }
        }

        // Create initial player scout fleet (FleetId 1)
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

        // Create initial AI scout fleet (FleetId 2)
        let ai_fleet_id = FleetId(2);
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

        // Determine initially explored stars for player and AI
        let explored_stars = initial_explored_stars(&stars_vec, home_star_id);
        let ai_explored_stars = initial_explored_stars(&stars_vec, ai_home_star_id);

        let state = GameState {
            seed,
            turn: 1,
            stars,
            empires,
            colonies,
            fleets,
            player_empire: player_empire_id,
            rng,
            event_log: Vec::new(),
            next_colony_id: 3,
            next_fleet_id: 3,
            explored_stars,
            scout_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: Some(ai_empire_id),
            ai_explored_stars,
            diplomacy: BTreeMap::new(),
        };

        Engine { state }
    }

    /// Create an engine from existing state
    pub fn from_state(state: GameState) -> Self {
        Engine { state }
    }

    /// Apply a list of commands and return generated events
    pub fn apply_turn(&mut self, commands: Vec<Command>) -> Vec<Event> {
        let mut events = Vec::new();

        for command in commands {
            match command {
                Command::EndTurn => {
                    self.process_end_turn(&mut events);
                }
                Command::SetColonyFocus {
                    colony,
                    prod_pct,
                    research_pct,
                } => {
                    self.process_set_colony_focus(colony, prod_pct, research_pct, &mut events);
                }
                Command::MoveFleet { fleet, destination } => {
                    self.process_move_fleet(fleet, destination, &mut events);
                }
                Command::QueueBuild { colony, item } => {
                    self.process_queue_build(colony, item, &mut events);
                }
                Command::CancelBuild { colony, index } => {
                    self.process_cancel_build(colony, index, &mut events);
                }
                Command::SelectResearch { tech } => {
                    self.process_select_research(tech, &mut events);
                }
                Command::SendScout { fleet, destination } => {
                    self.process_send_scout(fleet, destination, &mut events);
                }
                Command::Colonize {
                    fleet,
                    star,
                    planet_index,
                } => {
                    self.process_colonize(fleet, star, planet_index, &mut events);
                }
            }
        }

        // Add events to log
        for event in &events {
            self.state.event_log.push(event.to_log_message());
        }

        // Trim log to last 50 entries
        if self.state.event_log.len() > 50 {
            let excess = self.state.event_log.len() - 50;
            self.state.event_log.drain(0..excess);
        }

        events
    }

    fn process_end_turn(&mut self, events: &mut Vec<Event>) {
        // Process colonies in deterministic order
        let colony_ids = sorted_colony_ids(&self.state.colonies);

        // Track per-empire aggregates for this turn.
        // Keys are EmpireId; BTreeMap ensures deterministic iteration order.
        let mut empire_research: std::collections::BTreeMap<EmpireId, i64> =
            std::collections::BTreeMap::new();
        // Credits income generated by colonies (before maintenance)
        let mut empire_credits_income: std::collections::BTreeMap<EmpireId, i64> =
            std::collections::BTreeMap::new();
        // Food produced by colonies
        let mut empire_food_produced: std::collections::BTreeMap<EmpireId, i64> =
            std::collections::BTreeMap::new();
        // Food consumed by population
        let mut empire_food_consumed: std::collections::BTreeMap<EmpireId, i64> =
            std::collections::BTreeMap::new();
        // Credit maintenance from buildings and orbital structures per colony
        let mut empire_colony_maintenance: std::collections::BTreeMap<EmpireId, i64> =
            std::collections::BTreeMap::new();

        for colony_id in colony_ids {
            // Get colony data needed for yield calculation and build queue
            let (owner, production, star_id, build_queue_front, accumulated) = {
                let colony = self.state.colonies.get(&colony_id).unwrap();
                (
                    colony.owner,
                    colony.production,
                    colony.star,
                    colony.build_queue.first().copied(),
                    colony.accumulated_production,
                )
            };

            // Look up the planet this colony occupies for class bonuses
            let planet = self
                .state
                .stars
                .get(&star_id)
                .and_then(|s| {
                    let idx = self.state.colonies.get(&colony_id).unwrap().planet_index;
                    s.planets.get(idx)
                })
                .cloned();

            // Calculate yield via the v2 model
            let colony_yield = {
                let colony = self.state.colonies.get(&colony_id).unwrap();
                crate::yield_model::calculate_yield(colony, planet.as_ref())
            };

            let credits = colony_yield.credits;
            let research = colony_yield.science;

            // Update empire credits and lifetime research total
            if let Some(empire) = self.state.empires.get_mut(&owner) {
                empire.credits += credits;
                empire.research_points += research;
            }

            // Accumulate per-empire totals
            *empire_research.entry(owner).or_insert(0) += research;
            *empire_credits_income.entry(owner).or_insert(0) += credits;
            *empire_food_produced.entry(owner).or_insert(0) += colony_yield.food;
            *empire_food_consumed.entry(owner).or_insert(0) += colony_yield.food_consumed;
            *empire_colony_maintenance.entry(owner).or_insert(0) += colony_yield.maintenance;

            events.push(Event::ColonyProduced {
                colony: colony_id,
                credits,
                research,
                food: colony_yield.food,
                industry: colony_yield.industry,
                maintenance: colony_yield.maintenance,
            });

            // Process build queue — still uses colony.production for build speed
            if let Some(item) = build_queue_front {
                let new_accumulated = accumulated + production;
                let cost = item.cost();

                if new_accumulated >= cost {
                    // Item completed
                    if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                        colony.build_queue.remove(0);
                        colony.accumulated_production = new_accumulated - cost;
                    }

                    events.push(Event::BuildCompleted {
                        colony: colony_id,
                        item,
                    });

                    match item {
                        // Create a fleet for ship items
                        BuildItem::Scout | BuildItem::Colony => {
                            let fleet_id = self.state.next_fleet_id();
                            let owner_id = owner;
                            let fleet_kind = if item == BuildItem::Colony {
                                FleetKind::Colonizer
                            } else {
                                FleetKind::Scout
                            };
                            self.state.fleets.insert(
                                fleet_id,
                                Fleet {
                                    id: fleet_id,
                                    owner: owner_id,
                                    location: star_id,
                                    ships: 1,
                                    kind: fleet_kind,
                                    strength: 1,
                                    integrity: 100,
                                },
                            );
                            events.push(Event::FleetCreated {
                                fleet: fleet_id,
                                location: star_id,
                            });
                        }
                        // Add permanent surface buildings to the colony
                        BuildItem::Structure(bt) => {
                            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                                colony.buildings.push(bt);
                                colony.surface_installations.push(bt);
                            }
                        }
                        // Add orbital installations to the colony
                        BuildItem::OrbitalStructure(ot) => {
                            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                                colony.orbital_installations.push(ot);
                            }
                        }
                        // Outpost: no extra action needed
                        BuildItem::Outpost => {}
                    }
                } else {
                    // Update accumulated production
                    if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                        colony.accumulated_production = new_accumulated;
                    }
                }
            }
        }

        // Apply research progress for each empire that has a current tech
        let techs = all_techs();
        for (empire_id, research_gained) in &empire_research {
            // Emit science-generated event for this empire
            events.push(Event::ScienceGenerated {
                empire: *empire_id,
                amount: *research_gained,
            });

            let (current_tech, current_progress) = {
                let empire = match self.state.empires.get(empire_id) {
                    Some(e) => e,
                    None => continue,
                };
                (empire.research.current_tech, empire.research.progress)
            };

            if let Some(tech_id) = current_tech {
                let tech_cost = match techs.iter().find(|t| t.id == tech_id) {
                    Some(t) => t.cost,
                    None => continue,
                };

                let new_progress = current_progress + research_gained;

                if new_progress >= tech_cost {
                    // Tech completed — overflow is preserved in progress for the next tech
                    let overflow = new_progress - tech_cost;
                    if let Some(empire) = self.state.empires.get_mut(empire_id) {
                        empire.research.completed.push(tech_id);
                        empire.research.current_tech = None;
                        empire.research.progress = overflow;
                    }
                    events.push(Event::ResearchCompleted { tech: tech_id });
                } else {
                    if let Some(empire) = self.state.empires.get_mut(empire_id) {
                        empire.research.progress = new_progress;
                    }
                    events.push(Event::ResearchProgress {
                        tech: tech_id,
                        gained: *research_gained,
                        total: new_progress,
                        cost: tech_cost,
                    });
                }
            }
        }

        // Apply economy: food balance and maintenance costs per empire.
        // Collect all empire IDs that have colonies or fleets, in deterministic BTreeMap order.
        let empire_ids: std::collections::BTreeSet<EmpireId> =
            self.state.empires.keys().copied().collect();
        for empire_id in empire_ids {
            let credits_income = empire_credits_income.get(&empire_id).copied().unwrap_or(0);
            let food_produced = empire_food_produced.get(&empire_id).copied().unwrap_or(0);
            let food_consumed = empire_food_consumed.get(&empire_id).copied().unwrap_or(0);
            let colony_maint = empire_colony_maintenance
                .get(&empire_id)
                .copied()
                .unwrap_or(0);

            // Fleet maintenance: 1 credit per fleet owned by this empire
            let fleet_maintenance = self
                .state
                .fleets
                .values()
                .filter(|f| f.owner == empire_id)
                .count() as i64;

            let maintenance = fleet_maintenance + colony_maint;

            // Update empire food and credit balance
            if let Some(empire) = self.state.empires.get_mut(&empire_id) {
                empire.food += food_produced - food_consumed;
                empire.credits -= maintenance;
            }

            // Emit summary event
            events.push(Event::EconomySummary {
                empire: empire_id,
                credits_income,
                maintenance,
                food_produced,
                food_consumed,
            });

            // Emit warning events if balance is negative
            let (food_balance, credits_balance) = {
                let empire = &self.state.empires[&empire_id];
                (empire.food, empire.credits)
            };
            if food_balance < 0 {
                events.push(Event::FoodShortage {
                    empire: empire_id,
                    deficit: -food_balance,
                });
            }
            if credits_balance < 0 {
                events.push(Event::CreditDeficit {
                    empire: empire_id,
                    deficit: -credits_balance,
                });
            }
        }

        // Tick scout missions: decrement and resolve completed ones.
        // Collect keys first to avoid a borrow conflict when removing entries mid-loop.
        let mission_fleet_ids: Vec<FleetId> = self.state.scout_missions.keys().copied().collect();
        for fleet_id in mission_fleet_ids {
            let (destination, new_remaining) = {
                let mission = self.state.scout_missions.get_mut(&fleet_id).unwrap();
                mission.turns_remaining = mission.turns_remaining.saturating_sub(1);
                (mission.destination, mission.turns_remaining)
            };

            if new_remaining == 0 {
                self.state.scout_missions.remove(&fleet_id);

                // Route the explored star to the correct empire's set
                let is_ai_fleet = self
                    .state
                    .fleets
                    .get(&fleet_id)
                    .map(|f| Some(f.owner) == self.state.ai_empire)
                    .unwrap_or(false);
                if is_ai_fleet {
                    self.state.ai_explored_stars.insert(destination);
                    // Symmetric contact: AI scout arriving at a player colony
                    self.check_ai_contact_at_star(destination, events);
                } else {
                    self.state.explored_stars.insert(destination);
                }

                // Move the fleet to the destination
                if let Some(fleet) = self.state.fleets.get_mut(&fleet_id) {
                    fleet.location = destination;
                }

                events.push(Event::SystemExplored { star: destination });

                // Check if this exploration brings a player scout into contact with
                // a foreign empire colony.
                if !is_ai_fleet {
                    self.check_contact_at_star(destination, events);
                }

                // Check for hostile fleet encounters after arrival
                self.check_combat_at_star(destination, fleet_id, events);
            }
        }

        // Tick fleet movement missions: decrement and resolve completed ones.
        // Ordered by FleetId (BTreeMap) for deterministic event ordering.
        let fleet_mission_ids: Vec<FleetId> = self.state.fleet_missions.keys().copied().collect();
        for fleet_id in fleet_mission_ids {
            let (destination, new_remaining) = {
                let mission = self.state.fleet_missions.get_mut(&fleet_id).unwrap();
                mission.turns_remaining = mission.turns_remaining.saturating_sub(1);
                (mission.destination, mission.turns_remaining)
            };

            if new_remaining == 0 {
                self.state.fleet_missions.remove(&fleet_id);

                // Move the fleet to the destination
                if let Some(fleet) = self.state.fleets.get_mut(&fleet_id) {
                    fleet.location = destination;
                }

                events.push(Event::FleetArrived {
                    fleet: fleet_id,
                    star: destination,
                });

                // Check if this fleet arrival brings the player into contact with a
                // foreign empire colony at the destination.
                let is_player_fleet = self
                    .state
                    .fleets
                    .get(&fleet_id)
                    .map(|f| f.owner == self.state.player_empire)
                    .unwrap_or(false);
                if is_player_fleet {
                    self.check_contact_at_star(destination, events);
                }

                // Check for hostile fleet encounters after arrival
                self.check_combat_at_star(destination, fleet_id, events);
            }
        }

        // Run AI turn decisions (before advancing the turn counter)
        if let Some(ai_empire_id) = self.state.ai_empire {
            let ai_events = crate::ai::run_ai_turn(&mut self.state, ai_empire_id);
            events.extend(ai_events);
        }

        // Advance turn
        self.state.turn += 1;
        events.push(Event::TurnAdvanced {
            new_turn: self.state.turn,
        });
    }

    fn process_set_colony_focus(
        &mut self,
        colony_id: ColonyId,
        prod_pct: u8,
        research_pct: u8,
        events: &mut Vec<Event>,
    ) {
        // Validate colony exists
        let colony = match self.state.colonies.get(&colony_id) {
            Some(c) => c,
            None => {
                events.push(Event::error(format!("Colony {} not found", colony_id.0)));
                return;
            }
        };

        // Validate owner
        if colony.owner != self.state.player_empire {
            events.push(Event::error("Colony not owned by player"));
            return;
        }

        // Validate percentages
        if prod_pct as u16 + research_pct as u16 != 100 {
            events.push(Event::error(
                "Production and research percentages must sum to 100",
            ));
            return;
        }

        // Apply change
        if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
            colony.prod_pct = prod_pct;
            colony.research_pct = research_pct;
        }

        events.push(Event::ColonyFocusSet { colony: colony_id });
    }

    fn process_move_fleet(
        &mut self,
        fleet_id: FleetId,
        destination: StarId,
        events: &mut Vec<Event>,
    ) {
        // Validate fleet exists
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(f) => f,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };

        // Validate owner
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }

        // Block move if fleet is on an active scout mission
        if self.state.scout_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }

        // Block move if fleet already has a fleet mission
        if self.state.fleet_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }

        let from = fleet.location;

        // Validate destination exists
        if !self.state.stars.contains_key(&destination) {
            events.push(Event::error(format!(
                "Destination star {} not found",
                destination.0
            )));
            return;
        }

        // MoveFleet requires the destination to be explored
        if !self.state.explored_stars.contains(&destination) {
            events.push(Event::error(format!(
                "Star {} is not explored — use SendScout to explore it first",
                destination.0
            )));
            return;
        }

        // Cannot move to the star you're already at
        if from == destination {
            events.push(Event::error(format!(
                "Fleet {} is already at star {}",
                fleet_id.0, destination.0
            )));
            return;
        }

        // Calculate travel time from distance bucket
        let (src_x, src_y) = {
            let src = self.state.stars.get(&from).unwrap();
            (src.x, src.y)
        };
        let (dst_x, dst_y) = {
            let dst = self.state.stars.get(&destination).unwrap();
            (dst.x, dst.y)
        };
        let dx = (dst_x - src_x) as i64;
        let dy = (dst_y - src_y) as i64;
        let sq_dist = dx * dx + dy * dy;
        let turns = fleet_travel_turns(sq_dist);

        // Create the fleet mission
        self.state.fleet_missions.insert(
            fleet_id,
            FleetMission {
                fleet: fleet_id,
                destination,
                turns_remaining: turns,
            },
        );

        events.push(Event::FleetDeparted {
            fleet: fleet_id,
            from,
            to: destination,
            turns_remaining: turns,
        });
    }

    fn process_queue_build(
        &mut self,
        colony_id: ColonyId,
        item: BuildItem,
        events: &mut Vec<Event>,
    ) {
        // Validate colony exists
        let colony = match self.state.colonies.get(&colony_id) {
            Some(c) => c,
            None => {
                events.push(Event::error(format!("Colony {} not found", colony_id.0)));
                return;
            }
        };

        // Validate owner
        if colony.owner != self.state.player_empire {
            events.push(Event::error("Colony not owned by player"));
            return;
        }

        // Validate tech requirement
        if let Some(required_tech) = item.required_tech() {
            let empire = match self.state.empires.get(&self.state.player_empire) {
                Some(e) => e,
                None => {
                    events.push(Event::error("Player empire not found"));
                    return;
                }
            };
            if !empire.research.completed.contains(&required_tech) {
                let tech_name = all_techs()
                    .iter()
                    .find(|t| t.id == required_tech)
                    .map(|t| t.name)
                    .unwrap_or("Unknown tech");
                events.push(Event::error(format!(
                    "Cannot build {} — requires {} (tech {})",
                    item.name(),
                    tech_name,
                    required_tech.0
                )));
                return;
            }
        }

        // Ships require a Shipyard in orbit
        if matches!(item, BuildItem::Scout | BuildItem::Colony) && !colony.has_shipyard() {
            events.push(Event::error(format!(
                "Cannot build {} — colony {} has no Shipyard",
                item.name(),
                colony_id.0
            )));
            return;
        }

        // Surface buildings require a free surface slot
        if let BuildItem::Structure(_) = item {
            let planet_size = self
                .state
                .stars
                .get(&colony.star)
                .and_then(|s| s.planets.get(colony.planet_index))
                .map(|p| p.size);
            match planet_size {
                Some(size) if colony.can_place_surface_building(size) => {}
                Some(_) => {
                    events.push(Event::error(format!(
                        "Colony {} has no free surface slots",
                        colony_id.0
                    )));
                    return;
                }
                None => {
                    events.push(Event::error(format!(
                        "Colony {} planet not found",
                        colony_id.0
                    )));
                    return;
                }
            }
        }

        // Validate orbital slot capacity for orbital structures
        if let BuildItem::OrbitalStructure(_) = item {
            let planet_size = self
                .state
                .stars
                .get(&colony.star)
                .and_then(|s| s.planets.get(colony.planet_index))
                .map(|p| p.size);
            match planet_size {
                Some(size) if colony.can_place_orbital_installation(size) => {}
                Some(_) => {
                    events.push(Event::error(format!(
                        "Colony {} has no free orbital slots",
                        colony_id.0
                    )));
                    return;
                }
                None => {
                    events.push(Event::error(format!(
                        "Colony {} planet not found",
                        colony_id.0
                    )));
                    return;
                }
            }
        }

        // Add to queue
        if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
            colony.build_queue.push(item);
        }

        events.push(Event::BuildQueued {
            colony: colony_id,
            item,
        });
    }

    fn process_cancel_build(&mut self, colony_id: ColonyId, index: usize, events: &mut Vec<Event>) {
        // Validate colony exists
        let colony = match self.state.colonies.get(&colony_id) {
            Some(c) => c,
            None => {
                events.push(Event::error(format!("Colony {} not found", colony_id.0)));
                return;
            }
        };

        // Validate owner
        if colony.owner != self.state.player_empire {
            events.push(Event::error("Colony not owned by player"));
            return;
        }

        // Validate index
        if index >= colony.build_queue.len() {
            events.push(Event::error(format!(
                "Build queue index {} out of bounds",
                index
            )));
            return;
        }

        // Remove from queue
        if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
            colony.build_queue.remove(index);
            // Reset accumulated production if removing first item
            if index == 0 {
                colony.accumulated_production = 0;
            }
        }

        events.push(Event::BuildCancelled { colony: colony_id });
    }

    fn process_select_research(&mut self, tech_id: TechId, events: &mut Vec<Event>) {
        let empire_id = self.state.player_empire;

        // Validate tech exists
        let tech_exists = all_techs().iter().any(|t| t.id == tech_id);
        if !tech_exists {
            events.push(Event::error(format!("Tech {} not found", tech_id.0)));
            return;
        }

        // Validate empire exists
        let empire = match self.state.empires.get(&empire_id) {
            Some(e) => e,
            None => {
                events.push(Event::error("Player empire not found"));
                return;
            }
        };

        // Validate tech not already completed
        if empire.research.completed.contains(&tech_id) {
            events.push(Event::error(format!(
                "Tech {} is already completed",
                tech_id.0
            )));
            return;
        }

        // Select the tech; only reset progress when switching FROM a different active tech.
        // If current_tech is None (no research or just completed with overflow), preserve
        // progress so the overflow carries into the newly selected technology.
        if let Some(empire) = self.state.empires.get_mut(&empire_id) {
            if let Some(active_tech) = empire.research.current_tech {
                if active_tech != tech_id {
                    empire.research.progress = 0;
                }
            }
            empire.research.current_tech = Some(tech_id);
        }

        events.push(Event::ResearchSelected { tech: tech_id });
    }

    fn process_send_scout(
        &mut self,
        fleet_id: FleetId,
        destination: StarId,
        events: &mut Vec<Event>,
    ) {
        // Validate fleet exists
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(f) => f,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };

        // Validate owner
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }

        // Validate fleet is not already on a mission
        if self.state.scout_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already on a scout mission",
                fleet_id.0
            )));
            return;
        }

        // Block if fleet already has a fleet movement mission
        if self.state.fleet_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }

        // Validate destination star exists
        if !self.state.stars.contains_key(&destination) {
            events.push(Event::error(format!(
                "Destination star {} not found",
                destination.0
            )));
            return;
        }

        // Validate destination is not already explored
        if self.state.explored_stars.contains(&destination) {
            events.push(Event::error(format!(
                "Star {} is already explored",
                destination.0
            )));
            return;
        }

        // Create the scout mission
        self.state.scout_missions.insert(
            fleet_id,
            ScoutMission {
                fleet: fleet_id,
                destination,
                turns_remaining: SCOUT_TRAVEL_TURNS,
            },
        );

        events.push(Event::ScoutDispatched {
            fleet: fleet_id,
            destination,
            turns_remaining: SCOUT_TRAVEL_TURNS,
        });
    }

    fn process_colonize(
        &mut self,
        fleet_id: FleetId,
        star_id: StarId,
        planet_index: usize,
        events: &mut Vec<Event>,
    ) {
        // Validate fleet exists
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(f) => f,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };

        // Validate owner
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }

        // Validate fleet is a colonizer
        if fleet.kind != FleetKind::Colonizer {
            events.push(Event::error(format!(
                "Fleet {} is not a colonizer fleet",
                fleet_id.0
            )));
            return;
        }

        // Validate fleet is idle (not on any mission)
        if self.state.scout_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }
        if self.state.fleet_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }

        // Validate fleet is at the target star
        if fleet.location != star_id {
            events.push(Event::error(format!(
                "Fleet {} is not at star {}",
                fleet_id.0, star_id.0
            )));
            return;
        }

        // Validate star is explored
        if !self.state.explored_stars.contains(&star_id) {
            events.push(Event::error(format!("Star {} is not explored", star_id.0)));
            return;
        }

        // Validate star exists and get planet info
        let (planet_habitable, planet_colony) = {
            let star = match self.state.stars.get(&star_id) {
                Some(s) => s,
                None => {
                    events.push(Event::error(format!("Star {} not found", star_id.0)));
                    return;
                }
            };

            if planet_index >= star.planets.len() {
                events.push(Event::error(format!(
                    "Planet index {} out of bounds for star {}",
                    planet_index, star_id.0
                )));
                return;
            }

            let planet = &star.planets[planet_index];
            (planet.habitable, planet.colony)
        };

        // Validate planet is habitable
        if !planet_habitable {
            events.push(Event::error(format!(
                "Planet {} at star {} is not habitable",
                planet_index, star_id.0
            )));
            return;
        }

        // Validate planet is not already colonized
        if planet_colony.is_some() {
            events.push(Event::error(format!(
                "Planet {} at star {} is already colonized",
                planet_index, star_id.0
            )));
            return;
        }

        // All checks pass — create the colony
        let empire_id = self.state.player_empire;
        let colony_id = self.state.next_colony_id();

        let new_colony = Colony {
            id: colony_id,
            star: star_id,
            planet_index,
            owner: empire_id,
            population: 1,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
        };
        self.state.colonies.insert(colony_id, new_colony);

        // Update the planet's colony reference
        if let Some(star) = self.state.stars.get_mut(&star_id) {
            if let Some(planet) = star.planets.get_mut(planet_index) {
                planet.colony = Some(colony_id);
            }
        }

        // Consume the colonizer fleet.
        // The mission maps are cleared defensively so that no stale entries can
        // accumulate if the fleet was somehow in an inconsistent state on load.
        self.state.fleets.remove(&fleet_id);
        self.state.scout_missions.remove(&fleet_id);
        self.state.fleet_missions.remove(&fleet_id);

        events.push(Event::ColonizationCompleted {
            empire: empire_id,
            fleet: fleet_id,
            star: star_id,
            planet_index,
            colony: colony_id,
        });
    }

    /// Check whether arriving at `star_id` brings the player empire into first contact
    /// with any foreign empire that has a colony there.
    ///
    /// Iterates colonies in `BTreeMap` (deterministic) order.  Emits at most one
    /// `FirstContact` event per empire per call; duplicate contact (already
    /// `Contacted`) is silently ignored.
    fn check_contact_at_star(&mut self, star_id: StarId, events: &mut Vec<Event>) {
        // Collect foreign empire IDs that own a colony at this star.
        // Use sorted iteration (BTreeMap) for deterministic event ordering.
        let foreign_empires: Vec<EmpireId> = self
            .state
            .colonies
            .values()
            .filter(|c| c.star == star_id && c.owner != self.state.player_empire)
            .map(|c| c.owner)
            // Deduplicate while preserving sort order
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        for empire_id in foreign_empires {
            let status = self
                .state
                .diplomacy
                .get(&empire_id)
                .copied()
                .unwrap_or(RelationshipStatus::Unknown);

            if status == RelationshipStatus::Unknown {
                self.state
                    .diplomacy
                    .insert(empire_id, RelationshipStatus::Contacted);
                events.push(Event::FirstContact {
                    with_empire: empire_id,
                });
            }
        }
    }

    /// Symmetric contact check: called when an AI fleet arrives at a star.
    ///
    /// If the star has a player colony and the empires are not yet contacted,
    /// establishes first contact and emits `FirstContact`.
    fn check_ai_contact_at_star(&mut self, star_id: StarId, events: &mut Vec<Event>) {
        let player = self.state.player_empire;
        let has_player_colony = self
            .state
            .colonies
            .values()
            .any(|c| c.star == star_id && c.owner == player);

        if !has_player_colony {
            return;
        }

        // Check if the single AI empire (stored in ai_empire) needs first contact established.
        if let Some(ai_empire_id) = self.state.ai_empire {
            let status = self
                .state
                .diplomacy
                .get(&ai_empire_id)
                .copied()
                .unwrap_or(RelationshipStatus::Unknown);

            if status == RelationshipStatus::Unknown {
                self.state
                    .diplomacy
                    .insert(ai_empire_id, RelationshipStatus::Contacted);
                events.push(Event::FirstContact {
                    with_empire: ai_empire_id,
                });
            }
        }
    }

    /// Check for and resolve hostile fleet encounters at `star_id`.
    ///
    /// `arrived_fleet_id` is the fleet that just arrived.  It fights each
    /// idle enemy fleet at the star in ascending FleetId order (deterministic).
    /// Combat is simultaneous: both sides deal damage proportional to the
    /// opposing fleet's strength.  The arrived fleet stops fighting if it is
    /// destroyed.
    fn check_combat_at_star(
        &mut self,
        star_id: StarId,
        arrived_fleet_id: FleetId,
        events: &mut Vec<Event>,
    ) {
        // Get the owner of the arriving fleet (may no longer exist if destroyed in a
        // previous loop iteration — bail out silently).
        let arrived_owner = match self.state.fleets.get(&arrived_fleet_id) {
            Some(f) => f.owner,
            None => return,
        };

        // Collect hostile idle fleet IDs at this star.
        // BTreeMap iteration is already sorted by FleetId — deterministic ordering.
        let enemy_fleet_ids: Vec<FleetId> = self
            .state
            .fleets
            .iter()
            .filter(|(fid, f)| {
                **fid != arrived_fleet_id
                    && f.location == star_id
                    && f.owner != arrived_owner
                    && !self.state.fleet_missions.contains_key(*fid)
                    && !self.state.scout_missions.contains_key(*fid)
                    && is_contacted(&self.state, arrived_owner, f.owner)
            })
            .map(|(fid, _)| *fid)
            .collect();

        for enemy_id in enemy_fleet_ids {
            // Re-fetch arriving fleet — may have been destroyed in a prior iteration.
            let (a_str, a_int) = match self.state.fleets.get(&arrived_fleet_id) {
                Some(f) => (f.strength, f.integrity),
                None => break,
            };
            let (d_str, d_int, d_owner) = match self.state.fleets.get(&enemy_id) {
                Some(f) => (f.strength, f.integrity, f.owner),
                None => continue,
            };

            // Simultaneous damage: each fleet takes damage proportional to the
            // opponent's strength relative to its own.
            // Formula: damage = (opponent_strength * 100) / own_strength
            // This means equal strengths → both take 100 damage (destroyed).
            // Use u64 intermediates to avoid overflow when strength is large.
            let damage_to_arrived: u32 =
                ((d_str as u64 * 100) / (a_str as u64).max(1)).min(u32::MAX as u64) as u32;
            let damage_to_enemy: u32 =
                ((a_str as u64 * 100) / (d_str as u64).max(1)).min(u32::MAX as u64) as u32;

            let new_a_int = a_int.saturating_sub(damage_to_arrived);
            let new_d_int = d_int.saturating_sub(damage_to_enemy);

            let fleet_a_destroyed = new_a_int == 0;
            let fleet_b_destroyed = new_d_int == 0;

            events.push(Event::CombatResolved {
                star: star_id,
                fleet_a: arrived_fleet_id,
                empire_a: arrived_owner,
                fleet_b: enemy_id,
                empire_b: d_owner,
                strength_a: a_str,
                strength_b: d_str,
                integrity_a_remaining: new_a_int,
                integrity_b_remaining: new_d_int,
                fleet_a_destroyed,
                fleet_b_destroyed,
            });

            if fleet_a_destroyed {
                self.state.fleets.remove(&arrived_fleet_id);
            } else if let Some(f) = self.state.fleets.get_mut(&arrived_fleet_id) {
                f.integrity = new_a_int;
            }

            if fleet_b_destroyed {
                self.state.fleets.remove(&enemy_id);
            } else if let Some(f) = self.state.fleets.get_mut(&enemy_id) {
                f.integrity = new_d_int;
            }

            if fleet_a_destroyed {
                break;
            }
        }
    }
}

/// Returns true if `empire_a` and `empire_b` are in a `Contacted` relationship.
///
/// Diplomacy in v1 is stored from the player's perspective.  If neither empire
/// is the player, the function returns `false` (AI-vs-AI not applicable).
fn is_contacted(state: &GameState, empire_a: EmpireId, empire_b: EmpireId) -> bool {
    let player = state.player_empire;
    let other = if empire_a == player {
        empire_b
    } else if empire_b == player {
        empire_a
    } else {
        return false;
    };
    state
        .diplomacy
        .get(&other)
        .copied()
        .unwrap_or(RelationshipStatus::Unknown)
        == RelationshipStatus::Contacted
}

/// Compute the set of initially explored stars: home star + up to 3 nearest neighbours.
///
/// Distances are compared by squared Euclidean distance to remain deterministic
/// (no floating-point arithmetic).
fn initial_explored_stars(stars: &[crate::state::Star], home_id: StarId) -> BTreeSet<StarId> {
    let mut explored = BTreeSet::new();
    explored.insert(home_id);

    let home = match stars.iter().find(|s| s.id == home_id) {
        Some(s) => s,
        None => return explored,
    };

    // Sort all other stars by squared distance to home
    let mut neighbours: Vec<(i64, StarId)> = stars
        .iter()
        .filter(|s| s.id != home_id)
        .map(|s| {
            let dx = (s.x - home.x) as i64;
            let dy = (s.y - home.y) as i64;
            (dx * dx + dy * dy, s.id)
        })
        .collect();

    // Sort by distance, then by StarId for tie-breaking determinism
    neighbours.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (_, star_id) in neighbours.into_iter().take(3) {
        explored.insert(star_id);
    }

    explored
}

/// Find the AI home star: the habitable star farthest from the player's home.
///
/// A star qualifies only if it has at least one habitable planet.
/// Tie-breaking is by descending StarId so the choice is fully deterministic.
fn find_ai_home_star(stars: &[crate::state::Star], player_home: StarId) -> Option<StarId> {
    let player_star = stars.iter().find(|s| s.id == player_home)?;

    stars
        .iter()
        .filter(|s| s.id != player_home && s.planets.iter().any(|p| p.habitable))
        .max_by_key(|s| {
            let dx = (s.x - player_star.x) as i64;
            let dy = (s.y - player_star.y) as i64;
            // Primary key: distance; secondary: StarId for determinism
            (dx * dx + dy * dy, s.id.0)
        })
        .map(|s| s.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BuildingType;

    /// Inject a Shipyard directly into a colony's orbital installations.
    /// Used to satisfy the "ships require a Shipyard" rule in tests that
    /// focus on build completion / queue mechanics rather than the validation itself.
    ///
    /// # Panics
    /// Panics if `colony_id` does not exist in the engine state.
    fn give_colony_shipyard(engine: &mut Engine, colony_id: ColonyId) {
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .orbital_installations
            .push(crate::state::OrbitalStructureType::Shipyard);
    }

    #[test]
    fn new_engine_creates_valid_state() {
        let engine = Engine::new(42);
        assert_eq!(engine.state.turn, 1);
        assert!(!engine.state.stars.is_empty());
        assert!(!engine.state.empires.is_empty());
        assert!(!engine.state.colonies.is_empty());
        assert!(!engine.state.fleets.is_empty());
    }

    #[test]
    fn end_turn_advances_turn_counter() {
        let mut engine = Engine::new(42);
        let initial_turn = engine.state.turn;

        let events = engine.apply_turn(vec![Command::EndTurn]);

        assert_eq!(engine.state.turn, initial_turn + 1);
        assert!(events.iter().any(
            |e| matches!(e, Event::TurnAdvanced { new_turn } if *new_turn == initial_turn + 1)
        ));
    }

    #[test]
    fn end_turn_processes_colony_production() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Set focus to 100% credits
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        let initial_credits = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .credits;

        let events = engine.apply_turn(vec![Command::EndTurn]);

        let final_credits = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .credits;

        assert!(final_credits > initial_credits);
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ColonyProduced { .. })));
    }

    #[test]
    fn set_colony_focus_valid() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        let events = engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 70,
            research_pct: 30,
        }]);

        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ColonyFocusSet { colony } if *colony == colony_id)));

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert_eq!(colony.prod_pct, 70);
        assert_eq!(colony.research_pct, 30);
    }

    #[test]
    fn set_colony_focus_invalid_sum_emits_error() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        let events = engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 50,
            research_pct: 40, // Sum is 90, not 100
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn set_colony_focus_unknown_colony_emits_error() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(999);

        let events = engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 50,
            research_pct: 50,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn move_fleet_valid() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        // MoveFleet now requires an explored destination
        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        let destination = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != initial_location)
            .expect("Need an explored star other than home");

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination,
        }]);

        // Should emit FleetDeparted (not FleetMoved) and create a mission
        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::FleetDeparted { fleet, from, to, .. }
            if *fleet == fleet_id && *from == initial_location && *to == destination
        )));

        // Fleet has not moved yet — the mission is pending
        assert!(engine.state.fleet_missions.contains_key(&fleet_id));
        let fleet = engine.state.fleets.get(&fleet_id).unwrap();
        assert_eq!(fleet.location, initial_location);
    }

    #[test]
    fn move_fleet_unknown_fleet_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(999);
        let destination = *engine.state.stars.keys().next().unwrap();

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn move_fleet_unknown_destination_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);
        let destination = StarId(999);

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn queue_build_valid() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events.iter().any(
            |e| matches!(e, Event::BuildQueued { colony, item } if *colony == colony_id && *item == BuildItem::Scout)
        ));

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert_eq!(colony.build_queue.len(), 1);
        assert_eq!(colony.build_queue[0], BuildItem::Scout);
    }

    #[test]
    fn cancel_build_valid() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        // First add something to cancel
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        let events = engine.apply_turn(vec![Command::CancelBuild {
            colony: colony_id,
            index: 0,
        }]);

        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::BuildCancelled { colony } if *colony == colony_id)));

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(colony.build_queue.is_empty());
    }

    #[test]
    fn cancel_build_out_of_bounds_emits_error() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        let events = engine.apply_turn(vec![Command::CancelBuild {
            colony: colony_id,
            index: 5, // Queue is empty
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn build_completion_creates_fleet() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        // Queue a scout (cost 50)
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        // Set production to 100%
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        let initial_fleet_count = engine.state.fleets.len();

        // Run enough turns to complete the build (production is 10/turn, cost is 50)
        for _ in 0..6 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        // Should have a new fleet
        assert!(engine.state.fleets.len() > initial_fleet_count);
    }

    #[test]
    fn deterministic_engine_creation() {
        let engine1 = Engine::new(12345);
        let engine2 = Engine::new(12345);

        assert_eq!(engine1.state.stars.len(), engine2.state.stars.len());
        for (id, star1) in &engine1.state.stars {
            let star2 = engine2.state.stars.get(id).unwrap();
            assert_eq!(star1.name, star2.name);
            assert_eq!(star1.x, star2.x);
            assert_eq!(star1.y, star2.y);
        }
    }

    #[test]
    fn event_log_trimmed_to_50() {
        let mut engine = Engine::new(42);

        // Generate many events
        for _ in 0..60 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        assert!(engine.state.event_log.len() <= 50);
    }

    #[test]
    fn move_fleet_not_owned_by_player_emits_error() {
        let mut engine = Engine::new(42);

        // Create a fleet owned by a different empire
        let other_empire = EmpireId(99);
        let fleet_id = FleetId(99);
        let location = *engine.state.stars.keys().next().unwrap();
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: other_empire,
                location,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );

        let destination = *engine.state.stars.keys().nth(1).unwrap();
        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn queue_build_unknown_colony_emits_error() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(999);

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn queue_build_not_owned_by_player_emits_error() {
        let mut engine = Engine::new(42);

        // Create a colony owned by a different empire
        let other_empire = EmpireId(99);
        let colony_id = ColonyId(99);
        let star_id = *engine.state.stars.keys().next().unwrap();
        engine.state.colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
                star: star_id,
                planet_index: 0,
                owner: other_empire,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn cancel_build_unknown_colony_emits_error() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(999);

        let events = engine.apply_turn(vec![Command::CancelBuild {
            colony: colony_id,
            index: 0,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn cancel_build_not_owned_by_player_emits_error() {
        let mut engine = Engine::new(42);

        // Create a colony owned by a different empire with an item in the queue
        let other_empire = EmpireId(99);
        let colony_id = ColonyId(99);
        let star_id = *engine.state.stars.keys().next().unwrap();
        engine.state.colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
                star: star_id,
                planet_index: 0,
                owner: other_empire,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: vec![BuildItem::Scout],
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::CancelBuild {
            colony: colony_id,
            index: 0,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn cancel_build_non_first_item_does_not_reset_accumulated() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        // Queue two items
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);

        // Set some accumulated production
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .accumulated_production = 25;

        // Cancel the second item (index 1) — accumulated should be preserved
        let events = engine.apply_turn(vec![Command::CancelBuild {
            colony: colony_id,
            index: 1,
        }]);

        assert!(!events.iter().any(|e| e.is_error()));
        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert_eq!(colony.accumulated_production, 25);
        assert_eq!(colony.build_queue.len(), 1);
    }

    #[test]
    fn set_colony_focus_not_owned_by_player_emits_error() {
        let mut engine = Engine::new(42);

        // Create a colony owned by a different empire
        let other_empire = EmpireId(99);
        let colony_id = ColonyId(99);
        let star_id = *engine.state.stars.keys().next().unwrap();
        engine.state.colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
                star: star_id,
                planet_index: 0,
                owner: other_empire,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 70,
            research_pct: 30,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn build_completion_colony_ship_creates_fleet() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        // Queue a colony ship (cost 200)
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);

        // Set production to 100%
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        let initial_fleet_count = engine.state.fleets.len();

        // Run enough turns to complete (production 10/turn, cost 200 => 20 turns)
        for _ in 0..21 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        assert!(engine.state.fleets.len() > initial_fleet_count);
    }

    #[test]
    fn determinism_same_seed_same_commands_same_state() {
        let commands = vec![
            Command::SetColonyFocus {
                colony: ColonyId(1),
                prod_pct: 80,
                research_pct: 20,
            },
            Command::QueueBuild {
                colony: ColonyId(1),
                item: BuildItem::Scout,
            },
            Command::EndTurn,
            Command::EndTurn,
            Command::EndTurn,
        ];

        let mut engine_a = Engine::new(99999);
        let mut engine_b = Engine::new(99999);

        for cmd in commands {
            let evts_a = engine_a.apply_turn(vec![cmd.clone()]);
            let evts_b = engine_b.apply_turn(vec![cmd]);
            assert_eq!(evts_a, evts_b);
        }

        assert_eq!(engine_a.state, engine_b.state);
    }

    #[test]
    fn queue_building_structure_is_valid() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let item = BuildItem::Structure(BuildingType::AquacultureBay);

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item,
        }]);

        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events.iter().any(
            |e| matches!(e, Event::BuildQueued { colony, item: it } if *colony == colony_id && *it == item)
        ));

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert_eq!(colony.build_queue.len(), 1);
        assert_eq!(colony.build_queue[0], item);
    }

    #[test]
    fn building_completion_adds_to_colony_buildings() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Queue an Aquaculture Bay (cost 60)
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::AquacultureBay),
        }]);

        // 100% production to maximise output
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        // production=10/turn, cost=60 => complete after 6 turns
        for _ in 0..7 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(
            colony.buildings.contains(&BuildingType::AquacultureBay),
            "Colony should have AquacultureBay after completion"
        );
        assert!(
            colony.build_queue.is_empty(),
            "Build queue should be empty after completion"
        );
    }

    #[test]
    fn building_completion_does_not_create_fleet() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Queue a Fabrication Yard (cost 80)
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::FabricationYard),
        }]);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        let initial_fleet_count = engine.state.fleets.len();

        // production=10/turn, cost=80 => 8 turns
        for _ in 0..9 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        // Fleet count must NOT increase for a building
        assert_eq!(
            engine.state.fleets.len(),
            initial_fleet_count,
            "Building completion must not create a fleet"
        );

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(colony.buildings.contains(&BuildingType::FabricationYard));
    }

    #[test]
    fn multiple_buildings_accumulate_in_colony() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Queue two buildings
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::AquacultureBay),
        }]);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::ScienceNexus),
        }]);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        // cost: Aquaculture=60 + ScienceNexus=100 = 160 production total needed
        // 10/turn => 16+ turns
        for _ in 0..18 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(colony.buildings.contains(&BuildingType::AquacultureBay));
        assert!(colony.buildings.contains(&BuildingType::ScienceNexus));
    }

    #[test]
    fn building_completion_emits_build_completed_event() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let item = BuildItem::Structure(BuildingType::ScienceNexus);

        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        // cost=100, production=10/turn => 10 turns
        let mut completed = false;
        for _ in 0..12 {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events.iter().any(|e| {
                matches!(e, Event::BuildCompleted { colony, item: it }
                    if *colony == colony_id && *it == item)
            }) {
                completed = true;
                break;
            }
        }
        assert!(completed, "BuildCompleted event should have been emitted");
    }

    // ──────────────────────────────────────────────────────────────────
    // Research tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn select_research_valid_emits_research_selected() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let tech_id = TechId(1);

        let events = engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ResearchSelected { tech } if *tech == tech_id)));

        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert_eq!(empire.research.current_tech, Some(tech_id));
        assert_eq!(empire.research.progress, 0);
    }

    #[test]
    fn select_research_unknown_tech_emits_error() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let bad_tech = TechId(999);

        let events = engine.apply_turn(vec![Command::SelectResearch { tech: bad_tech }]);

        assert!(events.iter().any(|e| e.is_error()));
        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert!(empire.research.current_tech.is_none());
    }

    #[test]
    fn select_already_completed_tech_emits_error() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let tech_id = TechId(1);

        // Manually mark as completed
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .push(tech_id);

        let events = engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn research_progress_accumulates_on_end_turn() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let tech_id = TechId(1);

        // Set 100% research focus
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        // Select a tech
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        // End one turn
        engine.apply_turn(vec![Command::EndTurn]);

        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        // Research should have progressed
        assert!(
            empire.research.progress > 0,
            "Research progress should be positive after one turn"
        );
        assert_eq!(empire.research.current_tech, Some(tech_id));
    }

    #[test]
    fn research_completes_when_cost_reached() {
        use crate::state::{all_techs, TechId};
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let tech_id = TechId(1); // Void Propulsion, cost 50
        let tech_cost = all_techs().iter().find(|t| t.id == tech_id).unwrap().cost;

        // 100% research
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        // Compute actual RP/turn from colony state (avoids coupling to starting-balance constants)
        let rp_per_turn = {
            let colony = engine.state.colonies.get(&colony_id).unwrap();
            (colony.production as i64 * colony.research_pct as i64) / 100
        };
        let max_turns = if rp_per_turn > 0 {
            (tech_cost / rp_per_turn) + 2
        } else {
            200
        };

        let mut completed = false;
        for _ in 0..max_turns {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events
                .iter()
                .any(|e| matches!(e, Event::ResearchCompleted { tech } if *tech == tech_id))
            {
                completed = true;
                break;
            }
        }

        assert!(completed, "ResearchCompleted event should have fired");

        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert!(empire.research.completed.contains(&tech_id));
        assert!(empire.research.current_tech.is_none());
        assert_eq!(empire.research.progress, 0);
    }

    #[test]
    fn completed_tech_cannot_be_researched_again() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let tech_id = TechId(1);

        // Mark as completed
        engine
            .state
            .empires
            .get_mut(&engine.state.player_empire)
            .unwrap()
            .research
            .completed
            .push(tech_id);

        let events = engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);
        assert!(
            events.iter().any(|e| e.is_error()),
            "Selecting a completed tech must emit an error"
        );
    }

    #[test]
    fn research_deterministic_same_seed_same_result() {
        use crate::state::TechId;

        let cmds = vec![
            Command::SelectResearch { tech: TechId(1) },
            Command::SetColonyFocus {
                colony: ColonyId(1),
                prod_pct: 0,
                research_pct: 100,
            },
            Command::EndTurn,
            Command::EndTurn,
            Command::EndTurn,
        ];

        let mut engine_a = Engine::new(7777);
        let mut engine_b = Engine::new(7777);

        for cmd in cmds {
            let ea = engine_a.apply_turn(vec![cmd.clone()]);
            let eb = engine_b.apply_turn(vec![cmd]);
            assert_eq!(ea, eb);
        }

        let empire_a = engine_a
            .state
            .empires
            .get(&engine_a.state.player_empire)
            .unwrap();
        let empire_b = engine_b
            .state
            .empires
            .get(&engine_b.state.player_empire)
            .unwrap();
        assert_eq!(empire_a.research, empire_b.research);
    }

    #[test]
    fn research_no_progress_without_current_tech() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // 100% research but no tech selected
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![Command::EndTurn]);

        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        // research_points (lifetime total) should increase, but research.progress stays 0
        assert!(empire.research_points > 0);
        assert_eq!(empire.research.progress, 0);
        assert!(empire.research.current_tech.is_none());
    }

    #[test]
    fn reselecting_same_tech_preserves_progress() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let tech_id = TechId(1);

        // Set 100% research and select tech
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        // Accumulate some progress
        engine.apply_turn(vec![Command::EndTurn]);
        let progress_after_turn = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .research
            .progress;
        assert!(progress_after_turn > 0);

        // Re-select the same tech — progress should be preserved
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);
        let progress_after_reselect = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .research
            .progress;
        assert_eq!(
            progress_after_turn, progress_after_reselect,
            "Re-selecting the current tech must not reset progress"
        );
    }

    #[test]
    fn switching_tech_resets_progress() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let tech_a = TechId(1);
        let tech_b = TechId(2);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);
        engine.apply_turn(vec![Command::EndTurn]);

        let progress_on_a = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .research
            .progress;
        assert!(progress_on_a > 0);

        // Switch to a different tech — progress should reset
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_b }]);
        let progress_after_switch = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .research
            .progress;
        assert_eq!(
            progress_after_switch, 0,
            "Switching tech must reset progress"
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Overflow / science-pool tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn overflow_science_carries_to_next_research() {
        use crate::state::{all_techs, TechId};
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // Use research_pct=70 → rp = (10 * 70) / 100 = 7 rp/turn.
        // TechId(1) = Void Propulsion, cost 50.
        // 50 / 7 = 7.14... → completes on turn 8 with 7*8=56 → overflow = 6.
        let tech_a = TechId(1); // cost 50
        let tech_b = TechId(2); // Habitat Seeding, cost 80

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 30,
            research_pct: 70,
        }]);

        let rp_per_turn = {
            let colony = engine.state.colonies.get(&colony_id).unwrap();
            (colony.production as i64 * colony.research_pct as i64) / 100
        };
        assert!(rp_per_turn > 0);

        let tech_a_cost = all_techs().iter().find(|t| t.id == tech_a).unwrap().cost;
        // Find the turn on which the tech completes and compute the expected overflow
        let completion_turn_rp = {
            let mut acc = 0i64;
            loop {
                acc += rp_per_turn;
                if acc >= tech_a_cost {
                    break acc;
                }
            }
        };
        let expected_overflow = completion_turn_rp - tech_a_cost;
        assert!(
            expected_overflow > 0,
            "test requires non-zero overflow with rp={} cost={}; got {}",
            rp_per_turn,
            tech_a_cost,
            expected_overflow
        );

        engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);

        // Run until tech_a completes (a few extra turns are fine — no active tech after)
        let max_turns = tech_a_cost / rp_per_turn + 2;
        for _ in 0..max_turns {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert!(
            empire.research.completed.contains(&tech_a),
            "tech_a should be completed"
        );
        assert!(
            empire.research.current_tech.is_none(),
            "current_tech should be None after completion"
        );
        assert_eq!(
            empire.research.progress, expected_overflow,
            "overflow ({} rp) must be preserved in progress",
            expected_overflow
        );

        // Select tech B — overflow should carry over as a head start
        let overflow = empire.research.progress;
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_b }]);
        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert_eq!(
            empire.research.current_tech,
            Some(tech_b),
            "tech_b should now be active"
        );
        assert_eq!(
            empire.research.progress, overflow,
            "overflow should carry into tech_b progress"
        );
    }

    #[test]
    fn overflow_is_zero_when_tech_completes_exactly() {
        use crate::state::{all_techs, TechId};
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        // TechId(1) cost=50, rp=10/turn → exactly 5 turns → overflow 0
        let tech_id = TechId(1);
        let tech_cost = all_techs().iter().find(|t| t.id == tech_id).unwrap().cost;

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        let rp_per_turn = {
            let colony = engine.state.colonies.get(&colony_id).unwrap();
            (colony.production as i64 * colony.research_pct as i64) / 100
        };
        // Run the exact number of turns needed (no extra)
        let turns_exact = tech_cost / rp_per_turn;
        assert_eq!(
            turns_exact * rp_per_turn,
            tech_cost,
            "test expects exact completion"
        );

        for _ in 0..turns_exact {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert!(empire.research.completed.contains(&tech_id));
        assert_eq!(
            empire.research.progress, 0,
            "overflow must be 0 for exact completion"
        );
    }

    #[test]
    fn overflow_carry_is_deterministic() {
        use crate::state::TechId;

        let cmds = vec![
            Command::SetColonyFocus {
                colony: ColonyId(1),
                prod_pct: 0,
                research_pct: 100,
            },
            Command::SelectResearch { tech: TechId(1) },
            Command::EndTurn,
            Command::EndTurn,
            Command::EndTurn,
            Command::EndTurn,
            Command::EndTurn,
            Command::EndTurn, // extra turn to produce overflow on TechId(1) cost 50
        ];

        let mut engine_a = Engine::new(1234);
        let mut engine_b = Engine::new(1234);

        for cmd in &cmds {
            engine_a.apply_turn(vec![cmd.clone()]);
            engine_b.apply_turn(vec![cmd.clone()]);
        }

        let empire_a = engine_a
            .state
            .empires
            .get(&engine_a.state.player_empire)
            .unwrap();
        let empire_b = engine_b
            .state
            .empires
            .get(&engine_b.state.player_empire)
            .unwrap();
        assert_eq!(
            empire_a.research.progress, empire_b.research.progress,
            "overflow must be identical across identical seeds"
        );
    }

    #[test]
    fn selecting_tech_after_completion_preserves_overflow() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let tech_a = TechId(1); // cost 50
        let tech_b = TechId(2); // cost 80

        // research_pct=70 → 7 rp/turn; cost-50 tech completes on turn 8 with overflow 6
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 30,
            research_pct: 70,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_a }]);

        // Run 9 turns — enough to complete cost-50 at 7 rp/turn (completes turn 8)
        for _ in 0..9 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let overflow = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .research
            .progress;
        assert!(
            overflow > 0,
            "should have overflow after completing cost-50 tech at 7 rp/turn"
        );

        // Select next tech — overflow should carry over
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_b }]);
        let empire = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap();
        assert_eq!(
            empire.research.progress, overflow,
            "overflow must be preserved when selecting tech after completion"
        );
    }

    #[test]
    fn science_nexus_increases_research_output() {
        use crate::state::BuildingType;

        let colony_id = ColonyId(1);

        // Engine A: no buildings
        let mut engine_a = Engine::new(42);
        engine_a.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine_a.apply_turn(vec![Command::SelectResearch { tech: TechId(5) }]); // cost 120
        engine_a.apply_turn(vec![Command::EndTurn]);
        let progress_no_nexus = engine_a
            .state
            .empires
            .get(&engine_a.state.player_empire)
            .unwrap()
            .research
            .progress;

        // Engine B: manually add a ScienceNexus building
        let mut engine_b = Engine::new(42);
        engine_b
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::ScienceNexus);
        engine_b.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine_b.apply_turn(vec![Command::SelectResearch { tech: TechId(5) }]);
        engine_b.apply_turn(vec![Command::EndTurn]);
        let progress_with_nexus = engine_b
            .state
            .empires
            .get(&engine_b.state.player_empire)
            .unwrap()
            .research
            .progress;

        assert!(
            progress_with_nexus > progress_no_nexus,
            "ScienceNexus must increase research output: {} <= {}",
            progress_with_nexus,
            progress_no_nexus
        );
    }

    #[test]
    fn science_generated_event_emitted_each_end_turn() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        let events = engine.apply_turn(vec![Command::EndTurn]);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::ScienceGenerated { .. })),
            "ScienceGenerated event must be emitted each turn science is produced"
        );
    }

    #[test]
    fn research_progress_event_emitted_when_research_active() {
        use crate::state::TechId;
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let tech_id = TechId(5); // cost 120 — won't complete in one turn

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);
        engine.apply_turn(vec![Command::SelectResearch { tech: tech_id }]);

        let events = engine.apply_turn(vec![Command::EndTurn]);
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::ResearchProgress { tech, .. } if *tech == tech_id
            )),
            "ResearchProgress event must be emitted when a tech is actively researched"
        );
    }

    #[test]
    fn no_research_progress_event_when_no_active_tech() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        // No tech selected
        let events = engine.apply_turn(vec![Command::EndTurn]);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::ResearchProgress { .. })),
            "ResearchProgress must not be emitted when no tech is active"
        );
    }
    // Exploration / scout tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn initial_explored_stars_includes_home_star() {
        let engine = Engine::new(42);
        let home_star_id = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;
        assert!(
            engine.state.explored_stars.contains(&home_star_id),
            "Home star must be explored at game start"
        );
    }

    #[test]
    fn initial_explored_stars_count_is_deterministic() {
        let engine_a = Engine::new(42);
        let engine_b = Engine::new(42);
        assert_eq!(engine_a.state.explored_stars, engine_b.state.explored_stars);
    }

    #[test]
    fn initial_explored_stars_different_seeds_may_differ() {
        let engine_a = Engine::new(1);
        let engine_b = Engine::new(2);
        // The explored sets may or may not overlap but should each be non-empty
        assert!(!engine_a.state.explored_stars.is_empty());
        assert!(!engine_b.state.explored_stars.is_empty());
    }

    #[test]
    fn initial_explored_stars_up_to_four_stars() {
        let engine = Engine::new(42);
        // Home + up to 3 neighbours = at most 4 (and at least 1)
        assert!(!engine.state.explored_stars.is_empty());
        assert!(engine.state.explored_stars.len() <= 4);
    }

    #[test]
    fn send_scout_to_unexplored_star_succeeds() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let destination = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("There should be unexplored stars");

        let events = engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination,
        }]);

        assert!(
            !events.iter().any(|e| e.is_error()),
            "SendScout to unexplored star should not error"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::ScoutDispatched { fleet, destination: dest, .. }
                if *fleet == fleet_id && *dest == destination
            )),
            "ScoutDispatched event expected"
        );
        assert!(
            engine.state.scout_missions.contains_key(&fleet_id),
            "Scout mission should be registered"
        );
    }

    #[test]
    fn send_scout_to_explored_star_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let already_explored = *engine.state.explored_stars.iter().next().unwrap();

        let events = engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: already_explored,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "SendScout to an already-explored star must emit an error"
        );
        assert!(
            !engine.state.scout_missions.contains_key(&fleet_id),
            "No mission should be created for an already-explored star"
        );
    }

    #[test]
    fn send_scout_unknown_fleet_emits_error() {
        let mut engine = Engine::new(42);
        let bad_fleet = FleetId(999);

        let destination = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        let events = engine.apply_turn(vec![Command::SendScout {
            fleet: bad_fleet,
            destination,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn send_scout_unknown_destination_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);
        let bad_dest = StarId(9999);

        let events = engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: bad_dest,
        }]);

        assert!(events.iter().any(|e| e.is_error()));
    }

    #[test]
    fn send_scout_when_fleet_already_on_mission_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let mut unexplored = engine
            .state
            .stars
            .keys()
            .filter(|id| !engine.state.explored_stars.contains(id));
        let dest1 = *unexplored.next().expect("Need two unexplored stars");
        let dest2 = *unexplored.next().expect("Need two unexplored stars");

        // First dispatch succeeds
        engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: dest1,
        }]);

        // Second dispatch with same fleet should fail
        let events = engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: dest2,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Dispatching scout already on a mission must error"
        );
    }

    #[test]
    fn scout_arrives_after_travel_turns_and_explores_system() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let destination = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination,
        }]);

        assert!(!engine.state.explored_stars.contains(&destination));

        // Advance turns until the scout should arrive
        let mut explored_event_seen = false;
        for _ in 0..SCOUT_TRAVEL_TURNS {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events
                .iter()
                .any(|e| matches!(e, Event::SystemExplored { star } if *star == destination))
            {
                explored_event_seen = true;
            }
        }

        assert!(
            explored_event_seen,
            "SystemExplored event must fire after SCOUT_TRAVEL_TURNS"
        );
        assert!(
            engine.state.explored_stars.contains(&destination),
            "Destination should now be explored"
        );
        assert!(
            !engine.state.scout_missions.contains_key(&fleet_id),
            "Scout mission should be removed after completion"
        );
        // Fleet should have moved to destination
        assert_eq!(
            engine.state.fleets.get(&fleet_id).unwrap().location,
            destination
        );
    }

    #[test]
    fn move_fleet_on_scout_mission_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let destination = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination,
        }]);

        // Try to also move the fleet manually while it's on a scout mission.
        // Use an explored destination so the only failure reason is the active mission.
        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        let move_dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != initial_location)
            .expect("Need an explored star other than home");
        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: move_dest,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet while on scout mission must error"
        );
    }

    #[test]
    fn scout_dispatch_event_ordering_is_deterministic() {
        let mut engine_a = Engine::new(123);
        let mut engine_b = Engine::new(123);

        let dest = *engine_a
            .state
            .stars
            .keys()
            .find(|id| !engine_a.state.explored_stars.contains(id))
            .unwrap();

        let evts_a = engine_a.apply_turn(vec![Command::SendScout {
            fleet: FleetId(1),
            destination: dest,
        }]);
        let evts_b = engine_b.apply_turn(vec![Command::SendScout {
            fleet: FleetId(1),
            destination: dest,
        }]);

        assert_eq!(evts_a, evts_b);
    }

    // ──────────────────────────────────────────────────────────────────
    // Fleet movement (MoveFleet / FleetMission) tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn move_fleet_to_unexplored_star_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let unexplored = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: unexplored,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet to unexplored star must emit an error"
        );
        assert!(
            !engine.state.fleet_missions.contains_key(&fleet_id),
            "No mission should be created for an unexplored destination"
        );
    }

    #[test]
    fn move_fleet_while_travelling_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        let mut explored_others = engine
            .state
            .explored_stars
            .iter()
            .filter(|&&id| id != initial_location);
        let dest1 = *explored_others.next().expect("Need explored star");
        let dest2 = *explored_others.next().unwrap_or(&dest1);

        // First move succeeds
        engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: dest1,
        }]);
        assert!(engine.state.fleet_missions.contains_key(&fleet_id));

        // Second move while already travelling must fail
        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: dest2,
        }]);
        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet while already travelling must error"
        );
    }

    #[test]
    fn move_fleet_decrements_each_turn_and_arrives() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        // Pick the closest explored star so it takes 1 turn
        let dest = *engine
            .state
            .explored_stars
            .iter()
            .filter(|&&id| id != initial_location)
            .min_by_key(|&&id| {
                let home = engine.state.stars.get(&initial_location).unwrap();
                let dst = engine.state.stars.get(&id).unwrap();
                let dx = (dst.x - home.x) as i64;
                let dy = (dst.y - home.y) as i64;
                dx * dx + dy * dy
            })
            .expect("Need at least one other explored star");

        engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: dest,
        }]);

        // Fleet should still be at origin
        assert_eq!(
            engine.state.fleets.get(&fleet_id).unwrap().location,
            initial_location
        );
        assert!(engine.state.fleet_missions.contains_key(&fleet_id));

        // Advance turns until arrival (max 3 turns)
        let mut arrived = false;
        for _ in 0..3 {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events
                .iter()
                .any(|e| matches!(e, Event::FleetArrived { fleet, star } if *fleet == fleet_id && *star == dest))
            {
                arrived = true;
                break;
            }
        }

        assert!(arrived, "FleetArrived event should have been emitted");
        assert!(
            !engine.state.fleet_missions.contains_key(&fleet_id),
            "Mission should be removed after arrival"
        );
        assert_eq!(
            engine.state.fleets.get(&fleet_id).unwrap().location,
            dest,
            "Fleet location must be updated on arrival"
        );
    }

    #[test]
    fn move_fleet_to_same_star_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);
        let current_star = engine.state.fleets.get(&fleet_id).unwrap().location;

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: current_star,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet to current location must error"
        );
    }

    #[test]
    fn multiple_fleet_arrivals_are_deterministically_ordered() {
        // Create two fleets at the home star, both moving to different explored stars.
        // Both should arrive in the same EndTurn; their FleetArrived events must be
        // ordered by FleetId (ascending) due to BTreeMap iteration.
        let mut engine = Engine::new(42);

        let home_star = engine.state.fleets.get(&FleetId(1)).unwrap().location;
        let mut explored_others = engine
            .state
            .explored_stars
            .iter()
            .filter(|&&id| id != home_star)
            .copied();
        let dest_a = explored_others.next().expect("Need explored star A");
        let dest_b = explored_others.next().unwrap_or(dest_a);

        // Create a second fleet at home
        let fleet_b_id = engine.state.next_fleet_id();
        engine.state.fleets.insert(
            fleet_b_id,
            Fleet {
                id: fleet_b_id,
                owner: engine.state.player_empire,
                location: home_star,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );

        // Dispatch both fleets to the same near destination to guarantee same turn arrival
        engine.apply_turn(vec![
            Command::MoveFleet {
                fleet: FleetId(1),
                destination: dest_a,
            },
            Command::MoveFleet {
                fleet: fleet_b_id,
                destination: dest_b,
            },
        ]);

        // Both missions created
        assert!(engine.state.fleet_missions.contains_key(&FleetId(1)));
        assert!(engine.state.fleet_missions.contains_key(&fleet_b_id));

        // Force both missions to arrive next turn
        engine
            .state
            .fleet_missions
            .get_mut(&FleetId(1))
            .unwrap()
            .turns_remaining = 1;
        engine
            .state
            .fleet_missions
            .get_mut(&fleet_b_id)
            .unwrap()
            .turns_remaining = 1;

        let events = engine.apply_turn(vec![Command::EndTurn]);

        let arrival_fleet_ids: Vec<FleetId> = events
            .iter()
            .filter_map(|e| {
                if let Event::FleetArrived { fleet, .. } = e {
                    Some(*fleet)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(arrival_fleet_ids.len(), 2, "Both fleets should arrive");
        assert!(
            arrival_fleet_ids[0] < arrival_fleet_ids[1],
            "Arrivals must be ordered by FleetId: {:?}",
            arrival_fleet_ids
        );
    }

    #[test]
    fn fleet_travel_turns_buckets() {
        // Verify the distance buckets independently
        assert_eq!(fleet_travel_turns(0), 1);
        assert_eq!(fleet_travel_turns(100_000), 1);
        assert_eq!(fleet_travel_turns(100_001), 2);
        assert_eq!(fleet_travel_turns(400_000), 2);
        assert_eq!(fleet_travel_turns(400_001), 3);
        assert_eq!(fleet_travel_turns(2_000_000), 3);
    }

    #[test]
    fn scout_exploration_still_works_after_fleet_movement_added() {
        // Regression: scout missions must still explore unexplored systems.
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let dest = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");

        engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: dest,
        }]);

        for _ in 0..SCOUT_TRAVEL_TURNS {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        assert!(
            engine.state.explored_stars.contains(&dest),
            "Scout must still explore the system"
        );
        assert_eq!(
            engine.state.fleets.get(&fleet_id).unwrap().location,
            dest,
            "Fleet must move to explored destination"
        );
    }

    // --- Missing fleet negative tests ---

    #[test]
    fn move_fleet_unexplored_destination_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        let unexplored = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Need an unexplored star");

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: unexplored,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet to unexplored star must emit an error"
        );
    }

    #[test]
    fn move_fleet_already_at_destination_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);
        let location = engine.state.fleets.get(&fleet_id).unwrap().location;

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: location,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet to current location must emit an error"
        );
    }

    #[test]
    fn move_fleet_busy_scout_mission_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);

        // Send the fleet on a scout mission first
        let unexplored = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Need an unexplored star");
        engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: unexplored,
        }]);

        // Now try to move the same fleet
        let explored = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != engine.state.fleets.get(&fleet_id).unwrap().location)
            .expect("Need an explored star other than home");

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: explored,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet on a fleet with active scout mission must emit an error"
        );
    }

    #[test]
    fn move_fleet_busy_fleet_mission_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);
        let home = engine.state.fleets.get(&fleet_id).unwrap().location;

        let explored_dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need an explored star other than home");

        // Dispatch the fleet on a move mission
        engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: explored_dest,
        }]);

        // Try to dispatch again immediately
        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: explored_dest,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "MoveFleet on a fleet already on a fleet mission must emit an error"
        );
    }

    #[test]
    fn send_scout_busy_fleet_mission_emits_error() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1);
        let home = engine.state.fleets.get(&fleet_id).unwrap().location;

        // First move the fleet to an explored destination
        let explored_dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need an explored star other than home");
        engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination: explored_dest,
        }]);

        // Now try to send it as a scout too
        let unexplored = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Need an unexplored star");

        let events = engine.apply_turn(vec![Command::SendScout {
            fleet: fleet_id,
            destination: unexplored,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "SendScout on a fleet already on a fleet mission must emit an error"
        );
    }

    // --- Colonization tests ---

    #[test]
    fn colonize_valid_explored_habitable_unowned_planet() {
        let mut engine = Engine::new(42);

        // Build a colonizer at home
        let home = engine.state.fleets.get(&FleetId(1)).unwrap().location;

        // Queue Colony ship at home colony
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        // Run turns to build (production=10, cost=200 → 20 turns)
        for _ in 0..21 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        // Find the colonizer fleet
        let colonizer_id = engine
            .state
            .fleets
            .values()
            .find(|f| f.kind == FleetKind::Colonizer && f.owner == engine.state.player_empire)
            .map(|f| f.id)
            .expect("Colonizer fleet should exist after build");

        // Move colonizer to a different explored system
        let target_star = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need an explored star other than home");

        engine.apply_turn(vec![Command::MoveFleet {
            fleet: colonizer_id,
            destination: target_star,
        }]);

        // Wait until colonizer arrives
        for _ in 0..4 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        assert_eq!(
            engine.state.fleets.get(&colonizer_id).unwrap().location,
            target_star,
            "Colonizer must be at target star"
        );

        // Find first habitable unowned planet
        let planet_index = engine
            .state
            .stars
            .get(&target_star)
            .unwrap()
            .planets
            .iter()
            .enumerate()
            .find(|(_, p)| p.habitable && p.colony.is_none())
            .map(|(i, _)| i)
            .expect("Target star must have a habitable unowned planet");

        let colonies_before = engine.state.colonies.len();
        let fleets_before = engine.state.fleets.len();

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: colonizer_id,
            star: target_star,
            planet_index,
        }]);

        // No errors
        assert!(
            !events.iter().any(|e| e.is_error()),
            "Colonize should not emit an error"
        );

        // ColonizationCompleted emitted
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::ColonizationCompleted { star, .. } if *star == target_star
            )),
            "ColonizationCompleted event should be emitted"
        );

        // Colony was created
        assert_eq!(
            engine.state.colonies.len(),
            colonies_before + 1,
            "A new colony should have been created"
        );

        // Colonizer fleet was consumed
        assert_eq!(
            engine.state.fleets.len(),
            fleets_before - 1,
            "Colonizer fleet should be consumed"
        );
        assert!(
            !engine.state.fleets.contains_key(&colonizer_id),
            "Colonizer fleet should be removed"
        );

        // New colony is owned by player
        let new_colony = engine
            .state
            .colonies
            .values()
            .find(|c| c.star == target_star)
            .expect("Colony must exist at target star");
        assert_eq!(new_colony.owner, engine.state.player_empire);
        assert_eq!(new_colony.planet_index, planet_index);

        // Planet references the colony
        let planet = &engine.state.stars.get(&target_star).unwrap().planets[planet_index];
        assert!(planet.colony.is_some(), "Planet must reference the colony");
    }

    #[test]
    fn cannot_colonize_unexplored_system() {
        let mut engine = Engine::new(42);

        // Manually place a colonizer at an unexplored star
        let unexplored = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Need an unexplored star");

        let fleet_id = FleetId(99);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: engine.state.player_empire,
                location: unexplored,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: unexplored,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize to unexplored system must emit an error"
        );
    }

    #[test]
    fn cannot_colonize_already_owned_planet() {
        let mut engine = Engine::new(42);

        // The home star already has a colony at planet_index 0
        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;

        let fleet_id = FleetId(99);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: engine.state.player_empire,
                location: home,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: home,
            planet_index: 0, // Already has a colony
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize already-owned planet must emit an error"
        );
    }

    #[test]
    fn cannot_colonize_uninhabitable_planet() {
        let mut engine = Engine::new(42);

        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;

        // Pick an explored star that's not the home star
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need an explored star other than home");

        // Mark all its planets as uninhabitable
        if let Some(star) = engine.state.stars.get_mut(&target) {
            for planet in star.planets.iter_mut() {
                planet.habitable = false;
                planet.colony = None;
            }
        }

        let fleet_id = FleetId(99);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: engine.state.player_empire,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: target,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize uninhabitable planet must emit an error"
        );
    }

    #[test]
    fn cannot_colonize_without_colonizer_fleet() {
        let mut engine = Engine::new(42);
        let fleet_id = FleetId(1); // Initial fleet is Scout kind
        let home = engine.state.fleets.get(&fleet_id).unwrap().location;

        let planet_index = engine
            .state
            .stars
            .get(&home)
            .unwrap()
            .planets
            .iter()
            .enumerate()
            .find(|(_, p)| p.habitable && p.colony.is_none())
            .map(|(i, _)| i);

        // The scout fleet should be rejected even if at a valid planet
        // (use home star since it's explored and has habitable planets after index 0 potentially)
        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: home,
            planet_index: planet_index.unwrap_or(0),
        }]);

        // Either error because not a colonizer, or error because planet already colonized
        // In both cases an error must be emitted
        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize with non-colonizer fleet must emit an error"
        );
    }

    #[test]
    fn cannot_colonize_with_non_colonizer_fleet_explicit() {
        let mut engine = Engine::new(42);

        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| {
                let home = engine
                    .state
                    .empires
                    .get(&engine.state.player_empire)
                    .unwrap()
                    .home_star;
                id != home
            })
            .expect("Need an explored star other than home");

        // Place a Scout fleet at the target
        let fleet_id = FleetId(98);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: engine.state.player_empire,
                location: target,
                ships: 1,
                kind: FleetKind::Scout, // Not a colonizer
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: target,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize with Scout fleet must emit an error"
        );
        assert!(
            events.iter().any(
                |e| matches!(e, Event::Error { message } if message.contains("not a colonizer"))
            ),
            "Error message should mention 'not a colonizer'"
        );
    }

    #[test]
    fn colonizer_consumed_deterministically() {
        // Same seed + same commands must produce identical fleet IDs and colony IDs
        let setup = |seed: u64| {
            let mut engine = Engine::new(seed);
            let colony_id = ColonyId(1);
            give_colony_shipyard(&mut engine, colony_id);
            engine.apply_turn(vec![Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Colony,
            }]);
            engine.apply_turn(vec![Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            }]);
            for _ in 0..21 {
                engine.apply_turn(vec![Command::EndTurn]);
            }

            let colonizer_id = engine
                .state
                .fleets
                .values()
                .find(|f| f.kind == FleetKind::Colonizer)
                .map(|f| f.id)
                .expect("Colonizer must exist");

            let home = engine
                .state
                .empires
                .get(&engine.state.player_empire)
                .unwrap()
                .home_star;
            let target = *engine
                .state
                .explored_stars
                .iter()
                .find(|&&id| id != home)
                .expect("Need explored star");

            engine.apply_turn(vec![Command::MoveFleet {
                fleet: colonizer_id,
                destination: target,
            }]);
            for _ in 0..4 {
                engine.apply_turn(vec![Command::EndTurn]);
            }

            let planet_idx = engine
                .state
                .stars
                .get(&target)
                .unwrap()
                .planets
                .iter()
                .enumerate()
                .find(|(_, p)| p.habitable && p.colony.is_none())
                .map(|(i, _)| i)
                .unwrap_or(0);

            let events = engine.apply_turn(vec![Command::Colonize {
                fleet: colonizer_id,
                star: target,
                planet_index: planet_idx,
            }]);

            (engine, events)
        };

        let (engine_a, events_a) = setup(777);
        let (engine_b, events_b) = setup(777);

        assert_eq!(events_a, events_b, "Same seed must produce same events");
        assert_eq!(engine_a.state.colonies.len(), engine_b.state.colonies.len());
        assert_eq!(engine_a.state.fleets.len(), engine_b.state.fleets.len());
    }

    #[test]
    fn new_colony_participates_in_next_turn_production() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        // Build a colonizer
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);
        for _ in 0..21 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let colonizer_id = engine
            .state
            .fleets
            .values()
            .find(|f| f.kind == FleetKind::Colonizer)
            .map(|f| f.id)
            .expect("Colonizer must exist");

        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need explored star");

        engine.apply_turn(vec![Command::MoveFleet {
            fleet: colonizer_id,
            destination: target,
        }]);
        for _ in 0..4 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let planet_idx = engine
            .state
            .stars
            .get(&target)
            .unwrap()
            .planets
            .iter()
            .enumerate()
            .find(|(_, p)| p.habitable && p.colony.is_none())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Colonize
        engine.apply_turn(vec![Command::Colonize {
            fleet: colonizer_id,
            star: target,
            planet_index: planet_idx,
        }]);

        let new_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.star == target)
            .map(|c| c.id)
            .expect("New colony must exist");

        // End turn — new colony should produce
        let events = engine.apply_turn(vec![Command::EndTurn]);

        assert!(
            events.iter().any(
                |e| matches!(e, Event::ColonyProduced { colony, .. } if *colony == new_colony_id)
            ),
            "New colony must participate in next turn production"
        );
    }

    #[test]
    fn colonize_event_ordering_is_deterministic() {
        // Two colonizers at two different explored stars — the one with lower FleetId
        // should produce ColonizationCompleted before the other.
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let home = engine.state.empires.get(&player).unwrap().home_star;

        let explored: Vec<StarId> = engine
            .state
            .explored_stars
            .iter()
            .filter(|&&id| id != home)
            .copied()
            .collect();

        if explored.len() < 2 {
            // Not enough explored stars — skip test
            return;
        }

        let star_a = explored[0];
        let star_b = explored[1];

        let col_a = FleetId(91);
        let col_b = FleetId(92);

        // Place two colonizer fleets at different explored stars
        for (fid, star) in [(col_a, star_a), (col_b, star_b)] {
            // Ensure the star has a habitable unowned planet
            if let Some(star_data) = engine.state.stars.get_mut(&star) {
                if let Some(planet) = star_data.planets.get_mut(0) {
                    planet.habitable = true;
                    planet.colony = None;
                }
            }
            engine.state.fleets.insert(
                fid,
                Fleet {
                    id: fid,
                    owner: player,
                    location: star,
                    ships: 1,
                    kind: FleetKind::Colonizer,
                    strength: 1,
                    integrity: 100,
                },
            );
        }

        let events = engine.apply_turn(vec![
            Command::Colonize {
                fleet: col_a,
                star: star_a,
                planet_index: 0,
            },
            Command::Colonize {
                fleet: col_b,
                star: star_b,
                planet_index: 0,
            },
        ]);

        // Both should succeed
        let completed: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::ColonizationCompleted { .. }))
            .collect();
        assert_eq!(completed.len(), 2, "Both colonizations should complete");

        // Order matches command order (col_a before col_b)
        if let (
            Event::ColonizationCompleted { fleet: f1, .. },
            Event::ColonizationCompleted { fleet: f2, .. },
        ) = (completed[0], completed[1])
        {
            assert_eq!(*f1, col_a, "First event should be from col_a");
            assert_eq!(*f2, col_b, "Second event should be from col_b");
        }
    }

    #[test]
    fn cannot_colonize_fleet_not_at_star() {
        let mut engine = Engine::new(42);

        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;
        let other_star = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need explored star");

        // Place colonizer at home, but try to colonize other_star
        let fleet_id = FleetId(99);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: engine.state.player_empire,
                location: home,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: other_star,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize when fleet is not at target star must emit an error"
        );
    }

    #[test]
    fn build_completion_colony_ship_creates_colonizer_fleet() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        for _ in 0..21 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let colonizer = engine
            .state
            .fleets
            .values()
            .find(|f| f.kind == FleetKind::Colonizer && f.owner == engine.state.player_empire);

        assert!(
            colonizer.is_some(),
            "Completing Colony builditem must create a Colonizer fleet"
        );
    }

    #[test]
    fn build_scout_creates_scout_fleet() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        for _ in 0..6 {
            engine.apply_turn(vec![Command::EndTurn]);
        }

        let scout = engine
            .state
            .fleets
            .values()
            .filter(|f| f.owner == engine.state.player_empire)
            .find(|f| f.kind == FleetKind::Scout && f.id != FleetId(1));

        assert!(
            scout.is_some(),
            "Completing Scout builditem must create a Scout fleet"
        );
    }

    // -----------------------------------------------------------------------
    // Empire Economy v1 tests
    // -----------------------------------------------------------------------

    /// Colonies produce food equal to population (base) each turn.
    #[test]
    fn economy_colony_produces_food_equal_to_population() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;

        // Starting colony: population = 10, no AquacultureBay
        // Food produced = 10, consumed = 10 → net = 0
        let initial_food = engine.state.empires[&empire_id].food;
        engine.apply_turn(vec![Command::EndTurn]);
        let after_food = engine.state.empires[&empire_id].food;

        // Net food per turn = population - population = 0 (no aquaculture)
        assert_eq!(
            after_food, initial_food,
            "Food should be neutral with no AquacultureBay"
        );
    }

    /// AquacultureBay doubles food yield so the empire gains a surplus each turn.
    #[test]
    fn economy_aquaculture_bay_increases_food_surplus() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let empire_id = engine.state.player_empire;

        // Build an AquacultureBay (cost 60, full production)
        engine.apply_turn(vec![
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Structure(BuildingType::AquacultureBay),
            },
        ]);
        // AquacultureBay costs 60pp; with production=10/turn, takes 6 turns
        for _ in 0..6 {
            engine.apply_turn(vec![Command::EndTurn]);
        }
        // Confirm the bay was built
        let has_bay = engine.state.colonies[&colony_id]
            .buildings
            .contains(&BuildingType::AquacultureBay);
        assert!(has_bay, "AquacultureBay must be completed before testing");

        let food_before = engine.state.empires[&empire_id].food;
        engine.apply_turn(vec![Command::EndTurn]);
        let food_after = engine.state.empires[&empire_id].food;

        // With AquacultureBay: food_produced = population * 2, food_consumed = population
        // population = 10 → net = +10 per turn
        assert!(
            food_after > food_before,
            "AquacultureBay should create a food surplus (was {food_before}, now {food_after})"
        );
    }

    /// Fleet maintenance costs 1 credit per fleet per turn.
    #[test]
    fn economy_fleet_maintenance_costs_credits() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;

        // Set 0% production so credits income is zero; only maintenance matters
        let colony_id = ColonyId(1);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        let fleet_count = engine
            .state
            .fleets
            .values()
            .filter(|f| f.owner == empire_id)
            .count() as i64;
        let credits_before = engine.state.empires[&empire_id].credits;
        engine.apply_turn(vec![Command::EndTurn]);
        let credits_after = engine.state.empires[&empire_id].credits;

        // No income, fleet_count fleets at 1 credit each
        assert_eq!(
            credits_after,
            credits_before - fleet_count,
            "Fleet maintenance should cost {fleet_count} credits"
        );
    }

    /// FabricationYard and ScienceNexus each cost 1 credit/turn in maintenance.
    #[test]
    fn economy_building_maintenance_costs_credits() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let empire_id = engine.state.player_empire;

        // Build a FabricationYard (cost 80pp)
        engine.apply_turn(vec![
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Structure(BuildingType::FabricationYard),
            },
        ]);
        for _ in 0..9 {
            engine.apply_turn(vec![Command::EndTurn]);
        }
        assert!(
            engine.state.colonies[&colony_id]
                .buildings
                .contains(&BuildingType::FabricationYard),
            "FabricationYard must be completed"
        );

        // Now measure one turn with the yard present but 0% income
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        let fleet_count = engine
            .state
            .fleets
            .values()
            .filter(|f| f.owner == empire_id)
            .count() as i64;
        let credits_before = engine.state.empires[&empire_id].credits;
        engine.apply_turn(vec![Command::EndTurn]);
        let credits_after = engine.state.empires[&empire_id].credits;

        // 1 building (FabricationYard) + fleet_count fleets
        let expected_maintenance = fleet_count + 1;
        assert_eq!(
            credits_after,
            credits_before - expected_maintenance,
            "FabricationYard should add 1 credit/turn maintenance"
        );
    }

    /// Population consumes food each turn; net food = produced - consumed.
    #[test]
    fn economy_population_consumes_food() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;

        // Base: population=10, no buildings → net = 0 → food unchanged
        let food_before = engine.state.empires[&empire_id].food;
        engine.apply_turn(vec![Command::EndTurn]);
        let food_after = engine.state.empires[&empire_id].food;
        // food_produced = 10, food_consumed = 10
        assert_eq!(
            food_after, food_before,
            "Base food balance should be neutral"
        );
    }

    /// Positive food surplus accumulates in empire.food.
    #[test]
    fn economy_positive_food_surplus_accumulates() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let empire_id = engine.state.player_empire;

        // Build AquacultureBay to create surplus
        engine.apply_turn(vec![
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Structure(BuildingType::AquacultureBay),
            },
        ]);
        for _ in 0..6 {
            engine.apply_turn(vec![Command::EndTurn]);
        }
        assert!(engine.state.colonies[&colony_id]
            .buildings
            .contains(&BuildingType::AquacultureBay));

        let food_before = engine.state.empires[&empire_id].food;
        engine.apply_turn(vec![Command::EndTurn]);
        let food_after_1 = engine.state.empires[&empire_id].food;
        engine.apply_turn(vec![Command::EndTurn]);
        let food_after_2 = engine.state.empires[&empire_id].food;

        assert!(
            food_after_1 > food_before,
            "Food should increase with surplus"
        );
        assert!(food_after_2 > food_after_1, "Food should keep accumulating");
    }

    /// If the empire food stockpile is already negative, FoodShortage is emitted next turn.
    #[test]
    fn economy_negative_food_emits_shortage_event() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;

        // The starting colony has population=10, which produces 10 food and consumes 10 food
        // per turn — net zero.  We force the stockpile to -1 directly so that the engine
        // emits a FoodShortage warning on the very next turn.
        engine.state.empires.get_mut(&empire_id).unwrap().food = -1;

        let events = engine.apply_turn(vec![Command::EndTurn]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FoodShortage { empire, .. } if *empire == empire_id)),
            "FoodShortage should be emitted when food balance is negative"
        );
    }

    /// FoodShortage fires each turn as long as the food balance remains negative.
    #[test]
    fn economy_shortage_fires_when_food_net_negative() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;

        // Set the empire food stockpile to -5 so it is already in deficit.
        // The colony's net food production is zero (food_produced = pop, consumed = pop),
        // so the deficit persists and FoodShortage must fire again this turn.
        engine.state.empires.get_mut(&empire_id).unwrap().food = -5;

        let events = engine.apply_turn(vec![Command::EndTurn]);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FoodShortage { empire, .. } if *empire == empire_id)),
            "FoodShortage must fire while food balance is negative"
        );
    }

    /// CreditDeficit event is emitted when credits go negative after maintenance.
    #[test]
    fn economy_credit_deficit_emitted_when_credits_negative() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;
        let colony_id = ColonyId(1);

        // Zero colony income
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        // Drive credits to 0 so the first maintenance tick goes negative
        engine.state.empires.get_mut(&empire_id).unwrap().credits = 0;

        let events = engine.apply_turn(vec![Command::EndTurn]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::CreditDeficit { empire, .. } if *empire == empire_id)),
            "CreditDeficit should be emitted when credits go negative"
        );
        let credits = engine.state.empires[&empire_id].credits;
        assert!(credits < 0, "Credits should be negative after the deficit");
    }

    /// EconomySummary event is emitted each turn with correct values.
    #[test]
    fn economy_summary_event_emitted_each_turn() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;

        let events = engine.apply_turn(vec![Command::EndTurn]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == empire_id)),
            "EconomySummary should be emitted each turn for every empire"
        );
    }

    /// Science still feeds active research correctly after economy additions.
    #[test]
    fn economy_science_still_feeds_research() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        let empire_id = engine.state.player_empire;

        engine.apply_turn(vec![
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 0,
                research_pct: 100,
            },
            Command::SelectResearch {
                tech: TechId(1), // cost 50
            },
        ]);

        engine.apply_turn(vec![Command::EndTurn]);

        let empire = &engine.state.empires[&empire_id];
        assert!(
            empire.research.progress > 0 || empire.research.completed.contains(&TechId(1)),
            "Research should progress after one turn"
        );
    }

    /// Industry still advances local colony build queue correctly.
    #[test]
    fn economy_industry_still_advances_build_queue() {
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        engine.apply_turn(vec![
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Scout, // cost 50
            },
        ]);

        engine.apply_turn(vec![Command::EndTurn]);
        let accumulated = engine.state.colonies[&colony_id].accumulated_production;
        assert!(
            accumulated > 0,
            "Build queue should have accumulated production"
        );
    }

    /// Event ordering is deterministic: same seed → same event sequence.
    #[test]
    fn economy_event_ordering_is_deterministic() {
        let events1 = {
            let mut e = Engine::new(99);
            e.apply_turn(vec![Command::EndTurn])
        };
        let events2 = {
            let mut e = Engine::new(99);
            e.apply_turn(vec![Command::EndTurn])
        };
        assert_eq!(
            events1, events2,
            "Events must be identical for the same seed"
        );
    }

    /// Building maintenance methods return expected values.
    #[test]
    fn building_type_maintenance_costs() {
        assert_eq!(BuildingType::AquacultureBay.maintenance_cost(), 0);
        assert_eq!(BuildingType::FabricationYard.maintenance_cost(), 1);
        assert_eq!(BuildingType::ScienceNexus.maintenance_cost(), 1);
    }

    /// AquacultureBay food bonus equals population; others return zero.
    #[test]
    fn building_type_food_bonus() {
        assert_eq!(BuildingType::AquacultureBay.food_bonus(10), 10);
        assert_eq!(BuildingType::FabricationYard.food_bonus(10), 0);
        assert_eq!(BuildingType::ScienceNexus.food_bonus(10), 0);
        // Zero population edge case
        assert_eq!(BuildingType::AquacultureBay.food_bonus(0), 0);
    }

    // ──────────────────────────────────────────────────────────────────
    // Diplomacy tests
    // ──────────────────────────────────────────────────────────────────

    /// Empires start with no diplomacy entries (all implicitly Unknown).
    #[test]
    fn empires_start_unknown() {
        let engine = Engine::new(42);
        assert!(
            engine.state.diplomacy.is_empty(),
            "diplomacy map must be empty at game start"
        );
    }

    /// RelationshipStatus defaults to Unknown when absent from the map.
    #[test]
    fn relationship_status_default_is_unknown() {
        let engine = Engine::new(42);
        let ai_id = engine.state.ai_empire.expect("AI empire must exist");
        let status = engine
            .state
            .diplomacy
            .get(&ai_id)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown);
        assert_eq!(status, RelationshipStatus::Unknown);
    }

    /// Helper: build minimal state with a player colony at one star and an AI
    /// colony at another, then return (engine, player_star_id, ai_star_id, ai_empire_id).
    fn make_two_empire_state() -> (Engine, StarId, StarId, EmpireId) {
        use crate::state::{Planet, PlanetSize, SpectralClass};
        use rand::SeedableRng;

        let player_id = EmpireId(1);
        let ai_id = EmpireId(2);

        let player_star_id = StarId(1);
        let ai_star_id = StarId(2);

        let mut state = GameState {
            seed: 0,
            turn: 1,
            player_empire: player_id,
            rng: ChaCha8Rng::seed_from_u64(0),
            event_log: Vec::new(),
            next_colony_id: 10,
            next_fleet_id: 10,
            stars: BTreeMap::new(),
            empires: BTreeMap::new(),
            colonies: BTreeMap::new(),
            fleets: BTreeMap::new(),
            explored_stars: BTreeSet::new(),
            scout_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: Some(ai_id),
            ai_explored_stars: BTreeSet::new(),
            diplomacy: BTreeMap::new(),
        };

        // Player star
        state.stars.insert(
            player_star_id,
            crate::state::Star {
                id: player_star_id,
                name: "Alpha".to_string(),
                x: 0,
                y: 0,
                spectral_class: SpectralClass::G,
                planets: vec![Planet {
                    name: "Alpha I".to_string(),
                    size: PlanetSize::Medium,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(1)),
                    habitable: true,
                }],
            },
        );

        // AI star — close enough to reach in 1 turn
        state.stars.insert(
            ai_star_id,
            crate::state::Star {
                id: ai_star_id,
                name: "Beta".to_string(),
                x: 100,
                y: 0,
                spectral_class: SpectralClass::K,
                planets: vec![Planet {
                    name: "Beta I".to_string(),
                    size: PlanetSize::Medium,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(2)),
                    habitable: true,
                }],
            },
        );

        // Player empire
        state.empires.insert(
            player_id,
            Empire {
                id: player_id,
                name: "Player".to_string(),
                credits: 100,
                research_points: 0,
                home_star: player_star_id,
                research: ResearchState::default(),
                food: 0,
            },
        );

        // AI empire
        state.empires.insert(
            ai_id,
            Empire {
                id: ai_id,
                name: "AI".to_string(),
                credits: 100,
                research_points: 0,
                home_star: ai_star_id,
                research: ResearchState::default(),
                food: 0,
            },
        );

        // Player colony
        state.colonies.insert(
            ColonyId(1),
            Colony {
                id: ColonyId(1),
                star: player_star_id,
                planet_index: 0,
                owner: player_id,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );

        // AI colony
        state.colonies.insert(
            ColonyId(2),
            Colony {
                id: ColonyId(2),
                star: ai_star_id,
                planet_index: 0,
                owner: ai_id,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );

        // Player scout fleet at player star
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: player_id,
                location: player_star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );

        // Both stars explored
        state.explored_stars.insert(player_star_id);
        state.explored_stars.insert(ai_star_id);

        let engine = Engine::from_state(state);
        (engine, player_star_id, ai_star_id, ai_id)
    }

    /// A scout arriving at a star with a foreign colony establishes contact.
    #[test]
    fn scout_arrival_at_ai_colony_establishes_contact() {
        use crate::state::SpectralClass;
        use rand::SeedableRng;

        let player_id = EmpireId(1);
        let ai_id = EmpireId(2);

        let player_star_id = StarId(1);
        let ai_star_id = StarId(2);

        let mut state = GameState {
            seed: 0,
            turn: 1,
            player_empire: player_id,
            rng: ChaCha8Rng::seed_from_u64(0),
            event_log: Vec::new(),
            next_colony_id: 10,
            next_fleet_id: 10,
            stars: BTreeMap::new(),
            empires: BTreeMap::new(),
            colonies: BTreeMap::new(),
            fleets: BTreeMap::new(),
            // Only player star explored; AI star is unknown — scout will explore it
            explored_stars: {
                let mut s = BTreeSet::new();
                s.insert(player_star_id);
                s
            },
            scout_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: Some(ai_id),
            ai_explored_stars: BTreeSet::new(),
            diplomacy: BTreeMap::new(),
        };

        // Populate stars, empires, colonies, fleet
        state.stars.insert(
            player_star_id,
            crate::state::Star {
                id: player_star_id,
                name: "Alpha".to_string(),
                x: 0,
                y: 0,
                spectral_class: SpectralClass::G,
                planets: vec![crate::state::Planet {
                    name: "Alpha I".to_string(),
                    size: crate::state::PlanetSize::Medium,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(1)),
                    habitable: true,
                }],
            },
        );
        state.stars.insert(
            ai_star_id,
            crate::state::Star {
                id: ai_star_id,
                name: "Beta".to_string(),
                x: 100,
                y: 0,
                spectral_class: SpectralClass::K,
                planets: vec![crate::state::Planet {
                    name: "Beta I".to_string(),
                    size: crate::state::PlanetSize::Medium,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(2)),
                    habitable: true,
                }],
            },
        );
        state.empires.insert(
            player_id,
            Empire {
                id: player_id,
                name: "Player".to_string(),
                credits: 100,
                research_points: 0,
                home_star: player_star_id,
                research: ResearchState::default(),
                food: 0,
            },
        );
        state.empires.insert(
            ai_id,
            Empire {
                id: ai_id,
                name: "AI".to_string(),
                credits: 100,
                research_points: 0,
                home_star: ai_star_id,
                research: ResearchState::default(),
                food: 0,
            },
        );
        state.colonies.insert(
            ColonyId(1),
            Colony {
                id: ColonyId(1),
                star: player_star_id,
                planet_index: 0,
                owner: player_id,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );
        state.colonies.insert(
            ColonyId(2),
            Colony {
                id: ColonyId(2),
                star: ai_star_id,
                planet_index: 0,
                owner: ai_id,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );
        // Player scout at player star — will scout the AI star
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: player_id,
                location: player_star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );
        // Put the scout in a mission with 1 turn remaining to destination = ai_star_id
        state.scout_missions.insert(
            FleetId(1),
            ScoutMission {
                fleet: FleetId(1),
                destination: ai_star_id,
                turns_remaining: 1,
            },
        );

        let mut engine = Engine::from_state(state);
        let events = engine.apply_turn(vec![Command::EndTurn]);

        // SystemExplored should fire
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::SystemExplored { star } if *star == ai_star_id)));

        // FirstContact should fire for the AI empire
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FirstContact { with_empire } if *with_empire == ai_id)),
            "Expected FirstContact event for AI empire"
        );

        // Diplomacy state updated
        assert_eq!(
            engine.state.diplomacy.get(&ai_id).copied(),
            Some(RelationshipStatus::Contacted)
        );
    }

    /// A fleet arriving at a star with a foreign colony establishes contact.
    #[test]
    fn fleet_arrival_at_ai_colony_establishes_contact() {
        let (mut engine, _player_star_id, ai_star_id, ai_id) = make_two_empire_state();

        // Put fleet on a mission that completes this turn
        engine.state.fleet_missions.insert(
            FleetId(1),
            FleetMission {
                fleet: FleetId(1),
                destination: ai_star_id,
                turns_remaining: 1,
            },
        );

        let events = engine.apply_turn(vec![Command::EndTurn]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FleetArrived { star, .. } if *star == ai_star_id)),
            "Expected FleetArrived"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FirstContact { with_empire } if *with_empire == ai_id)),
            "Expected FirstContact"
        );
        assert_eq!(
            engine.state.diplomacy.get(&ai_id).copied(),
            Some(RelationshipStatus::Contacted)
        );
    }

    /// Repeated contact does not duplicate the FirstContact event.
    #[test]
    fn repeated_contact_does_not_emit_duplicate_first_contact() {
        let (mut engine, _player_star_id, ai_star_id, ai_id) = make_two_empire_state();

        // Pre-mark as Contacted
        engine
            .state
            .diplomacy
            .insert(ai_id, RelationshipStatus::Contacted);

        // Fleet arrives at the AI colony star again
        engine.state.fleet_missions.insert(
            FleetId(1),
            FleetMission {
                fleet: FleetId(1),
                destination: ai_star_id,
                turns_remaining: 1,
            },
        );

        let events = engine.apply_turn(vec![Command::EndTurn]);

        let first_contact_count = events
            .iter()
            .filter(|e| matches!(e, Event::FirstContact { with_empire } if *with_empire == ai_id))
            .count();
        assert_eq!(
            first_contact_count, 0,
            "No duplicate FirstContact event when empire is already Contacted"
        );
    }

    /// Contact events are emitted in deterministic (BTreeMap) order.
    #[test]
    fn contact_detection_is_deterministic() {
        use crate::state::{Planet, PlanetSize, SpectralClass};
        use rand::SeedableRng;

        let player_id = EmpireId(1);
        let ai1 = EmpireId(2);
        let ai2 = EmpireId(3);
        let star1 = StarId(1);
        let target_star = StarId(2);

        let mut state = GameState {
            seed: 0,
            turn: 1,
            player_empire: player_id,
            rng: ChaCha8Rng::seed_from_u64(0),
            event_log: Vec::new(),
            next_colony_id: 10,
            next_fleet_id: 10,
            stars: BTreeMap::new(),
            empires: BTreeMap::new(),
            colonies: BTreeMap::new(),
            fleets: BTreeMap::new(),
            explored_stars: {
                let mut s = BTreeSet::new();
                s.insert(star1);
                s.insert(target_star);
                s
            },
            scout_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: Some(ai1),
            ai_explored_stars: BTreeSet::new(),
            diplomacy: BTreeMap::new(),
        };

        // Two AI empires each have a colony at target_star
        state.stars.insert(
            star1,
            crate::state::Star {
                id: star1,
                name: "Home".to_string(),
                x: 0,
                y: 0,
                spectral_class: SpectralClass::G,
                planets: vec![Planet {
                    name: "Home I".to_string(),
                    size: PlanetSize::Medium,
                    class: crate::state::PlanetClass::Terran,
                    colony: Some(ColonyId(1)),
                    habitable: true,
                }],
            },
        );
        state.stars.insert(
            target_star,
            crate::state::Star {
                id: target_star,
                name: "Target".to_string(),
                x: 100,
                y: 0,
                spectral_class: SpectralClass::K,
                planets: vec![
                    Planet {
                        name: "Target I".to_string(),
                        size: PlanetSize::Medium,
                        class: crate::state::PlanetClass::Terran,
                        colony: Some(ColonyId(2)),
                        habitable: true,
                    },
                    Planet {
                        name: "Target II".to_string(),
                        size: PlanetSize::Small,
                        class: crate::state::PlanetClass::Terran,
                        colony: Some(ColonyId(3)),
                        habitable: true,
                    },
                ],
            },
        );
        for (eid, home) in [(player_id, star1), (ai1, target_star), (ai2, target_star)] {
            state.empires.insert(
                eid,
                Empire {
                    id: eid,
                    name: format!("E{}", eid.0),
                    credits: 100,
                    research_points: 0,
                    home_star: home,
                    research: ResearchState::default(),
                    food: 0,
                },
            );
        }
        state.colonies.insert(
            ColonyId(1),
            Colony {
                id: ColonyId(1),
                star: star1,
                planet_index: 0,
                owner: player_id,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );
        state.colonies.insert(
            ColonyId(2),
            Colony {
                id: ColonyId(2),
                star: target_star,
                planet_index: 0,
                owner: ai1,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );
        state.colonies.insert(
            ColonyId(3),
            Colony {
                id: ColonyId(3),
                star: target_star,
                planet_index: 1,
                owner: ai2,
                population: 5,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
            },
        );
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: player_id,
                location: star1,
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
                destination: target_star,
                turns_remaining: 1,
            },
        );

        let mut engine = Engine::from_state(state);
        let events = engine.apply_turn(vec![Command::EndTurn]);

        let contacts: Vec<EmpireId> = events
            .iter()
            .filter_map(|e| match e {
                Event::FirstContact { with_empire } => Some(*with_empire),
                _ => None,
            })
            .collect();

        // Both empires contacted, in ascending EmpireId order (BTreeMap iteration)
        assert_eq!(contacts, vec![ai1, ai2]);
    }

    /// FirstContact log message contains expected text.
    #[test]
    fn first_contact_log_message() {
        let event = Event::FirstContact {
            with_empire: EmpireId(2),
        };
        let msg = event.to_log_message();
        assert!(
            msg.contains("FIRST CONTACT"),
            "Log message should contain 'FIRST CONTACT'"
        );
        assert!(
            msg.contains('2'),
            "Log message should reference empire ID 2"
        );
        assert!(!event.is_error());
    }

    // -----------------------------------------------------------------------
    // Combat auto-resolve tests
    // -----------------------------------------------------------------------

    /// Helper: build a minimal GameState with two fleets at the same star,
    /// controlled by the player (EmpireId 1) and a contacted foreign empire
    /// (EmpireId 2).
    fn make_combat_state(
        player_strength: u32,
        player_integrity: u32,
        enemy_strength: u32,
        enemy_integrity: u32,
    ) -> (GameState, StarId, FleetId, FleetId) {
        let mut engine = Engine::new(42);
        let state = &mut engine.state;

        // Use the player's home star
        let star_id = state.empires.get(&state.player_empire).unwrap().home_star;

        // Remove existing fleets to avoid interference
        state.fleets.clear();
        state.fleet_missions.clear();
        state.scout_missions.clear();

        let player = state.player_empire;
        let enemy_empire = EmpireId(2);

        // Establish contact so combat is enabled
        state
            .diplomacy
            .insert(enemy_empire, RelationshipStatus::Contacted);

        let player_fleet_id = FleetId(10);
        let enemy_fleet_id = FleetId(20);

        state.fleets.insert(
            player_fleet_id,
            Fleet {
                id: player_fleet_id,
                owner: player,
                location: star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: player_strength,
                integrity: player_integrity,
            },
        );
        state.fleets.insert(
            enemy_fleet_id,
            Fleet {
                id: enemy_fleet_id,
                owner: enemy_empire,
                location: star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: enemy_strength,
                integrity: enemy_integrity,
            },
        );

        (engine.state, star_id, player_fleet_id, enemy_fleet_id)
    }

    #[test]
    fn stronger_fleet_wins_deterministically() {
        // Player fleet (strength 20) vs enemy fleet (strength 10)
        let (state, star_id, player_fid, enemy_fid) = make_combat_state(20, 100, 10, 100);

        let mut events = Vec::new();
        let mut engine = Engine::from_state(state);
        engine.check_combat_at_star(star_id, player_fid, &mut events);

        // Enemy should be destroyed; player should survive
        assert!(
            !engine.state.fleets.contains_key(&enemy_fid),
            "Enemy fleet should be destroyed"
        );
        assert!(
            engine.state.fleets.contains_key(&player_fid),
            "Player fleet should survive"
        );

        // CombatResolved event emitted
        let combat_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::CombatResolved { .. }))
            .collect();
        assert_eq!(combat_events.len(), 1, "Exactly one CombatResolved event");

        if let Event::CombatResolved {
            fleet_a_destroyed,
            fleet_b_destroyed,
            ..
        } = &combat_events[0]
        {
            assert!(!fleet_a_destroyed, "Player fleet should not be destroyed");
            assert!(fleet_b_destroyed, "Enemy fleet should be destroyed");
        }
    }

    #[test]
    fn equal_fleets_destroy_each_other() {
        let (state, star_id, player_fid, enemy_fid) = make_combat_state(10, 100, 10, 100);

        let mut events = Vec::new();
        let mut engine = Engine::from_state(state);
        engine.check_combat_at_star(star_id, player_fid, &mut events);

        assert!(
            !engine.state.fleets.contains_key(&player_fid),
            "Player fleet should be destroyed when equal"
        );
        assert!(
            !engine.state.fleets.contains_key(&enemy_fid),
            "Enemy fleet should be destroyed when equal"
        );

        let combat_event = events
            .iter()
            .find(|e| matches!(e, Event::CombatResolved { .. }))
            .expect("CombatResolved event required");
        if let Event::CombatResolved {
            fleet_a_destroyed,
            fleet_b_destroyed,
            ..
        } = combat_event
        {
            assert!(fleet_a_destroyed, "Fleet A should be destroyed");
            assert!(fleet_b_destroyed, "Fleet B should be destroyed");
        }
    }

    #[test]
    fn damaged_winner_has_expected_integrity() {
        // Player strength 20 vs enemy strength 10 — damage_to_player = 10*100/20 = 50
        let (state, star_id, player_fid, _) = make_combat_state(20, 100, 10, 100);

        let mut events = Vec::new();
        let mut engine = Engine::from_state(state);
        engine.check_combat_at_star(star_id, player_fid, &mut events);

        let survivor = engine
            .state
            .fleets
            .get(&player_fid)
            .expect("Player fleet should survive");
        assert_eq!(
            survivor.integrity, 50,
            "Winner integrity should be 100 - (10*100/20) = 50"
        );

        let combat_event = events
            .iter()
            .find(|e| matches!(e, Event::CombatResolved { .. }))
            .expect("CombatResolved event required");
        if let Event::CombatResolved {
            integrity_a_remaining,
            ..
        } = combat_event
        {
            assert_eq!(*integrity_a_remaining, 50);
        }
    }

    #[test]
    fn same_empire_fleets_do_not_fight() {
        let mut engine = Engine::new(42);
        let star_id = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;
        let player = engine.state.player_empire;

        engine.state.fleets.clear();
        engine.state.fleet_missions.clear();
        engine.state.scout_missions.clear();

        let fid_a = FleetId(10);
        let fid_b = FleetId(20);

        for fid in [fid_a, fid_b] {
            engine.state.fleets.insert(
                fid,
                Fleet {
                    id: fid,
                    owner: player,
                    location: star_id,
                    ships: 1,
                    kind: FleetKind::Scout,
                    strength: 10,
                    integrity: 100,
                },
            );
        }

        let mut events = Vec::new();
        engine.check_combat_at_star(star_id, fid_a, &mut events);

        // No combat events
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, Event::CombatResolved { .. })),
            "No combat between fleets of the same empire"
        );
        // Both fleets still present
        assert!(engine.state.fleets.contains_key(&fid_a));
        assert!(engine.state.fleets.contains_key(&fid_b));
    }

    #[test]
    fn unknown_empire_fleets_do_not_fight() {
        let (mut state, star_id, player_fid, enemy_fid) = make_combat_state(10, 100, 10, 100);

        // Remove contact so empires are Unknown
        state.diplomacy.clear();

        let mut events = Vec::new();
        let mut engine = Engine::from_state(state);
        engine.check_combat_at_star(star_id, player_fid, &mut events);

        // No combat events
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, Event::CombatResolved { .. })),
            "No combat between unknown empires"
        );
        // Both fleets still alive
        assert!(engine.state.fleets.contains_key(&player_fid));
        assert!(engine.state.fleets.contains_key(&enemy_fid));
    }

    #[test]
    fn combat_triggers_after_fleet_arrival() {
        // Set up: player fleet travels to a star where a contacted enemy fleet waits
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let ai_empire = engine.state.ai_empire.expect("AI empire required");

        // Establish contact
        engine
            .state
            .diplomacy
            .insert(ai_empire, RelationshipStatus::Contacted);

        // Pick a star that is explored by the player and not the home star
        let home_star = engine.state.empires.get(&player).unwrap().home_star;
        let target_star = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&sid| sid != home_star)
            .expect("Need at least one non-home explored star");

        // Place an enemy fleet at the target star
        let enemy_fid = FleetId(99);
        engine.state.fleets.insert(
            enemy_fid,
            Fleet {
                id: enemy_fid,
                owner: ai_empire,
                location: target_star,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );

        // Find the player fleet (FleetId 1)
        let player_fid = FleetId(1);

        // Move player fleet to target star (1-turn if close enough; set mission directly)
        engine.state.fleet_missions.insert(
            player_fid,
            FleetMission {
                fleet: player_fid,
                destination: target_star,
                turns_remaining: 1,
            },
        );

        // End turn — fleet arrives, combat should fire
        let events = engine.apply_turn(vec![Command::EndTurn]);

        let combat_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::CombatResolved { .. }))
            .collect();
        assert!(
            !combat_events.is_empty(),
            "CombatResolved should be emitted after fleet arrival"
        );
    }

    #[test]
    fn destroyed_fleets_are_removed_from_state() {
        let (state, star_id, player_fid, enemy_fid) = make_combat_state(10, 100, 10, 100); // equal → both destroyed

        let mut engine = Engine::from_state(state);
        let mut events = Vec::new();
        engine.check_combat_at_star(star_id, player_fid, &mut events);

        assert!(
            !engine.state.fleets.contains_key(&player_fid),
            "Destroyed player fleet must be removed"
        );
        assert!(
            !engine.state.fleets.contains_key(&enemy_fid),
            "Destroyed enemy fleet must be removed"
        );
    }

    #[test]
    fn combat_events_are_deterministic() {
        // Running the same scenario twice should produce identical events.
        let (state1, star_id, player_fid, _) = make_combat_state(20, 100, 10, 100);
        let (state2, _, _, _) = make_combat_state(20, 100, 10, 100);

        let mut events1 = Vec::new();
        let mut engine1 = Engine::from_state(state1);
        engine1.check_combat_at_star(star_id, player_fid, &mut events1);

        let mut events2 = Vec::new();
        let mut engine2 = Engine::from_state(state2);
        engine2.check_combat_at_star(star_id, player_fid, &mut events2);

        assert_eq!(
            events1, events2,
            "Combat events must be identical for same initial state"
        );
    }

    #[test]
    fn combat_resolved_log_message_contains_expected_content() {
        let event = Event::CombatResolved {
            star: StarId(5),
            fleet_a: FleetId(1),
            empire_a: EmpireId(1),
            fleet_b: FleetId(2),
            empire_b: EmpireId(2),
            strength_a: 20,
            strength_b: 10,
            integrity_a_remaining: 50,
            integrity_b_remaining: 0,
            fleet_a_destroyed: false,
            fleet_b_destroyed: true,
        };
        let msg = event.to_log_message();
        assert!(msg.contains("COMBAT"), "Log should mention COMBAT");
        assert!(msg.contains("system 5"), "Log should mention star system 5");
        assert!(!event.is_error());
    }

    #[test]
    fn fleet_has_strength_and_integrity_fields() {
        let engine = Engine::new(42);
        // All initial fleets should have strength=1 and integrity=100
        for fleet in engine.state.fleets.values() {
            assert_eq!(fleet.strength, 1, "Initial fleet strength should be 1");
            assert_eq!(
                fleet.integrity, 100,
                "Initial fleet integrity should be 100"
            );
        }
    }

    // ──────────────────────────────────────────────────────────────────
    // Colonize — additional invalid-command edge cases
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn colonize_out_of_bounds_planet_index_emits_error() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let home = engine.state.empires.get(&player).unwrap().home_star;

        // Choose an explored star other than home
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Test setup failed: no explored star found other than home star; at least two explored stars are required");

        // Get the number of planets at that star so we can go one past the end
        let planet_count = engine.state.stars.get(&target).unwrap().planets.len();

        // Place a colonizer at the target
        let fleet_id = FleetId(77);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: player,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: target,
            planet_index: planet_count, // one past the end
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize with out-of-bounds planet_index must emit an error"
        );
        // Colonizer must not be consumed on an invalid command
        assert!(
            engine.state.fleets.contains_key(&fleet_id),
            "Colonizer fleet must not be removed when the command fails"
        );
    }

    #[test]
    fn colonize_unknown_fleet_emits_error() {
        let mut engine = Engine::new(42);
        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: FleetId(9999),
            star: home,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize with non-existent fleet must emit an error"
        );
    }

    #[test]
    fn colonize_fleet_on_scout_mission_emits_error() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let home = engine.state.empires.get(&player).unwrap().home_star;

        // Pick an explored star other than home for the colonizer target
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need explored star other than home");

        // Place a colonizer at target
        let fleet_id = FleetId(78);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: player,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        // Put it on a scout mission (busy)
        let unexplored = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Test setup failed: no unexplored star found; at least one unexplored star is required");
        engine.state.scout_missions.insert(
            fleet_id,
            ScoutMission {
                fleet: fleet_id,
                destination: unexplored,
                turns_remaining: 3,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: target,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize with fleet on scout mission must emit an error"
        );
    }

    #[test]
    fn colonize_fleet_on_fleet_mission_emits_error() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let home = engine.state.empires.get(&player).unwrap().home_star;

        let explored: Vec<StarId> = engine
            .state
            .explored_stars
            .iter()
            .filter(|&&id| id != home)
            .copied()
            .collect();
        let target = *explored
            .first()
            .expect("Test setup failed: no explored star found other than home star; at least two explored stars are required");

        // Place a colonizer at home (not at target)
        let fleet_id = FleetId(79);
        engine.state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: player,
                location: home,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        // Fleet is in transit toward target
        engine.state.fleet_missions.insert(
            fleet_id,
            FleetMission {
                fleet: fleet_id,
                destination: target,
                turns_remaining: 2,
            },
        );

        let events = engine.apply_turn(vec![Command::Colonize {
            fleet: fleet_id,
            star: target,
            planet_index: 0,
        }]);

        assert!(
            events.iter().any(|e| e.is_error()),
            "Colonize with fleet on fleet mission must emit an error"
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Longer deterministic replay
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn deterministic_replay_scout_research_multi_turn() {
        // Run the same mixed command sequence on two identically-seeded engines and
        // assert that final state and per-turn event sequences are identical.
        use crate::state::TechId;

        let seed = 55_555;

        let run = |s: u64| {
            let mut engine = Engine::new(s);
            let colony_id = ColonyId(1);

            // Set up colony focus
            engine.apply_turn(vec![Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 50,
                research_pct: 50,
            }]);

            // Start research
            engine.apply_turn(vec![Command::SelectResearch { tech: TechId(1) }]);

            // Find an unexplored star and dispatch a scout
            let dest = *engine
                .state
                .stars
                .keys()
                .find(|id| !engine.state.explored_stars.contains(id))
                .expect("Test setup failed: no unexplored star found; the galaxy must have at least one unexplored star");
            engine.apply_turn(vec![Command::SendScout {
                fleet: FleetId(1),
                destination: dest,
            }]);

            // Advance 5 turns
            let mut all_events = Vec::new();
            for _ in 0..5 {
                all_events.push(engine.apply_turn(vec![Command::EndTurn]));
            }

            (engine.state, all_events)
        };

        let (state_a, events_a) = run(seed);
        let (state_b, events_b) = run(seed);

        assert_eq!(
            events_a, events_b,
            "Per-turn events must be identical for the same seed"
        );
        assert_eq!(
            state_a, state_b,
            "Final state must be identical for the same seed"
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Event ordering
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn turn_advanced_is_last_event_of_end_turn() {
        let mut engine = Engine::new(42);
        let events = engine.apply_turn(vec![Command::EndTurn]);

        // TurnAdvanced must exist and be the last event
        let last = events
            .last()
            .expect("EndTurn must produce at least one event");
        assert!(
            matches!(last, Event::TurnAdvanced { .. }),
            "TurnAdvanced must be the last event in an EndTurn batch; got {:?}",
            last
        );
    }

    #[test]
    fn economy_summary_precedes_food_shortage_and_credit_deficit() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;
        let colony_id = ColonyId(1);

        // Force food and credit deficits simultaneously
        engine.state.empires.get_mut(&empire_id).unwrap().food = -5;
        engine.state.empires.get_mut(&empire_id).unwrap().credits = 0;
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        let events = engine.apply_turn(vec![Command::EndTurn]);

        // Find indices of EconomySummary, FoodShortage, CreditDeficit for the empire
        let summary_idx = events
            .iter()
            .position(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == empire_id))
            .expect("EconomySummary must be present");

        let shortage_idx = events
            .iter()
            .position(|e| matches!(e, Event::FoodShortage { empire, .. } if *empire == empire_id))
            .expect("FoodShortage must be present");

        let deficit_idx = events
            .iter()
            .position(|e| matches!(e, Event::CreditDeficit { empire, .. } if *empire == empire_id))
            .expect("CreditDeficit must be present");

        assert!(
            summary_idx < shortage_idx,
            "EconomySummary (idx {summary_idx}) must precede FoodShortage (idx {shortage_idx})"
        );
        assert!(
            summary_idx < deficit_idx,
            "EconomySummary (idx {summary_idx}) must precede CreditDeficit (idx {deficit_idx})"
        );
    }

    #[test]
    fn colony_produced_precedes_economy_summary() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;
        let colony_id = ColonyId(1);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 50,
            research_pct: 50,
        }]);

        let events = engine.apply_turn(vec![Command::EndTurn]);

        let produced_idx = events
            .iter()
            .position(|e| matches!(e, Event::ColonyProduced { colony, .. } if *colony == colony_id))
            .expect("ColonyProduced must be present");

        let summary_idx = events
            .iter()
            .position(|e| matches!(e, Event::EconomySummary { empire, .. } if *empire == empire_id))
            .expect("EconomySummary must be present");

        assert!(
            produced_idx < summary_idx,
            "ColonyProduced (idx {produced_idx}) must come before EconomySummary (idx {summary_idx})"
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Production edge cases
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn build_queue_advances_at_full_production_regardless_of_focus() {
        // The build queue always accumulates at the raw `production` rate;
        // prod_pct only controls how much of that becomes credits.
        let mut engine = Engine::new(42);
        let colony_id = ColonyId(1);
        give_colony_shipyard(&mut engine, colony_id);

        let production = engine.state.colonies[&colony_id].production;

        // Queue a scout (cost 50), set 0% production (100% research)
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        engine.apply_turn(vec![Command::EndTurn]);

        // Build queue advances by `production` regardless of prod_pct
        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert_eq!(
            colony.accumulated_production, production,
            "Build queue must advance by raw production even when prod_pct = 0"
        );
    }

    #[test]
    fn credits_income_is_zero_when_prod_pct_is_zero() {
        let mut engine = Engine::new(42);
        let empire_id = engine.state.player_empire;
        let colony_id = ColonyId(1);

        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 0,
            research_pct: 100,
        }]);

        // Record credits before, subtract fleet maintenance manually to find income
        let fleet_count = engine
            .state
            .fleets
            .values()
            .filter(|f| f.owner == empire_id)
            .count() as i64;
        let credits_before = engine.state.empires[&empire_id].credits;

        engine.apply_turn(vec![Command::EndTurn]);

        let credits_after = engine.state.empires[&empire_id].credits;

        // With prod_pct=0 the only credit change should be negative maintenance
        assert_eq!(
            credits_after,
            credits_before - fleet_count,
            "Credits should only decrease by maintenance when prod_pct=0"
        );
    }

    // -------------------------------------------------------------------------
    // Shipyard / orbital structure regression tests
    // -------------------------------------------------------------------------

    /// Helper: unlock Orbital Engineering (TechId 7) for the player empire.
    fn unlock_orbital_engineering(engine: &mut Engine) {
        let empire_id = engine.state.player_empire;
        if let Some(empire) = engine.state.empires.get_mut(&empire_id) {
            if !empire.research.completed.contains(&TechId(7)) {
                empire.research.completed.push(TechId(7));
            }
        }
    }

    #[test]
    fn shipyard_requires_orbital_engineering_tech() {
        // Without the tech, queuing a Shipyard should emit an Error event.
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .expect("player colony must exist");

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::OrbitalStructure(crate::state::OrbitalStructureType::Shipyard),
        }]);

        assert!(
            events.iter().any(
                |e| matches!(e, Event::Error { message } if message.contains("Orbital Engineering"))
            ),
            "expected an error mentioning Orbital Engineering, got: {:?}",
            events
        );

        // Queue should be empty — build was rejected
        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(
            colony.build_queue.is_empty(),
            "build queue must be empty after rejected Shipyard"
        );
    }

    #[test]
    fn shipyard_can_be_queued_with_orbital_engineering() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .expect("player colony must exist");

        unlock_orbital_engineering(&mut engine);

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::OrbitalStructure(crate::state::OrbitalStructureType::Shipyard),
        }]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::BuildQueued { .. })),
            "expected BuildQueued event, got: {:?}",
            events
        );

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert_eq!(
            colony.build_queue.last(),
            Some(&BuildItem::OrbitalStructure(
                crate::state::OrbitalStructureType::Shipyard
            )),
            "Shipyard must be in the build queue"
        );
    }

    #[test]
    fn shipyard_blocked_when_no_orbital_slots_available() {
        use crate::state::OrbitalStructureType;

        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .expect("player colony must exist");

        unlock_orbital_engineering(&mut engine);

        // Find the colony's planet size and fill all orbital slots
        let (star_id, planet_index) = {
            let c = engine.state.colonies.get(&colony_id).unwrap();
            (c.star, c.planet_index)
        };
        let planet_size = engine
            .state
            .stars
            .get(&star_id)
            .unwrap()
            .planets
            .get(planet_index)
            .unwrap()
            .size;
        let max_slots = planet_size.orbital_slots();

        // Fill all orbital slots by directly mutating the colony
        {
            let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
            for _ in 0..max_slots {
                colony
                    .orbital_installations
                    .push(OrbitalStructureType::Shipyard);
            }
        }

        // Now try to queue another Shipyard — should be rejected
        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard),
        }]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Error { message } if message.contains("no free orbital slots"))),
            "expected no-orbital-slots error, got: {:?}",
            events
        );
    }

    #[test]
    fn shipyard_build_completes_and_enters_orbital_installations() {
        use crate::state::OrbitalStructureType;

        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .expect("player colony must exist");

        unlock_orbital_engineering(&mut engine);

        // Queue the Shipyard
        engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard),
        }]);

        // Force 100% production focus and run enough turns for it to complete
        engine.apply_turn(vec![Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        }]);

        // Shipyard costs 200pp; run enough turns to build it (production is at least 5/turn)
        let mut completed = false;
        for _ in 0..50 {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events.iter().any(|e| {
                matches!(e, Event::BuildCompleted { item, .. } if *item == BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard))
            }) {
                completed = true;
                break;
            }
        }

        assert!(completed, "Shipyard should complete within 50 turns");

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(
            colony
                .orbital_installations
                .contains(&OrbitalStructureType::Shipyard),
            "completed Shipyard must appear in orbital_installations"
        );
    }

    #[test]
    fn surface_building_enters_surface_installations_on_completion() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .expect("player colony must exist");

        engine.apply_turn(vec![
            Command::QueueBuild {
                colony: colony_id,
                item: BuildItem::Structure(BuildingType::FabricationYard),
            },
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
        ]);

        let mut completed = false;
        for _ in 0..20 {
            let events = engine.apply_turn(vec![Command::EndTurn]);
            if events.iter().any(|e| {
                matches!(e, Event::BuildCompleted { item, .. } if *item == BuildItem::Structure(BuildingType::FabricationYard))
            }) {
                completed = true;
                break;
            }
        }

        assert!(completed, "FabricationYard should complete within 20 turns");

        let colony = engine.state.colonies.get(&colony_id).unwrap();
        assert!(
            colony
                .surface_installations
                .contains(&BuildingType::FabricationYard),
            "completed FabricationYard must appear in surface_installations"
        );
        assert!(
            colony.buildings.contains(&BuildingType::FabricationYard),
            "completed FabricationYard must appear in buildings for effect tracking"
        );
    }

    #[test]
    fn orbital_engineering_tech_exists_in_all_techs() {
        let techs = all_techs();
        assert!(
            techs
                .iter()
                .any(|t| t.id == TechId(7) && t.name == "Orbital Engineering"),
            "Orbital Engineering must be TechId(7)"
        );
    }

    #[test]
    fn orbital_structure_type_shipyard_has_correct_metadata() {
        use crate::state::OrbitalStructureType;
        let ot = OrbitalStructureType::Shipyard;
        assert_eq!(ot.name(), "Shipyard");
        assert_eq!(ot.required_tech(), Some(TechId(7)));
        assert!(ot.cost() > 0, "cost must be positive");
        assert!(ot.maintenance_cost() > 0, "maintenance must be positive");
    }

    #[test]
    fn build_item_orbital_structure_required_tech_matches() {
        use crate::state::OrbitalStructureType;
        let item = BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard);
        assert_eq!(item.required_tech(), Some(TechId(7)));
        // Surface structures have no required tech
        assert_eq!(
            BuildItem::Structure(BuildingType::FabricationYard).required_tech(),
            None
        );
        assert_eq!(BuildItem::Scout.required_tech(), None);
    }

    // -------------------------------------------------------------------------
    // Shipyard requirement for ships
    // -------------------------------------------------------------------------

    #[test]
    fn scout_requires_shipyard_to_queue() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .unwrap();

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Error { message } if message.contains("no Shipyard"))),
            "Scout must be rejected without a Shipyard, got: {:?}",
            events
        );
        assert!(
            engine
                .state
                .colonies
                .get(&colony_id)
                .unwrap()
                .build_queue
                .is_empty(),
            "build queue must stay empty when Scout is rejected"
        );
    }

    #[test]
    fn colony_ship_requires_shipyard_to_queue() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .unwrap();

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Error { message } if message.contains("no Shipyard"))),
            "Colony Ship must be rejected without a Shipyard, got: {:?}",
            events
        );
    }

    #[test]
    fn scout_allowed_when_shipyard_present() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .unwrap();

        give_colony_shipyard(&mut engine, colony_id);

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Scout,
        }]);

        assert!(
            !events.iter().any(|e| e.is_error()),
            "Scout must be accepted with a Shipyard, got errors: {:?}",
            events
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::BuildQueued { item, .. } if *item == BuildItem::Scout)),);
    }

    #[test]
    fn colony_ship_allowed_when_shipyard_present() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .unwrap();

        give_colony_shipyard(&mut engine, colony_id);

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Colony,
        }]);

        assert!(
            !events.iter().any(|e| e.is_error()),
            "Colony Ship must be accepted with a Shipyard, got errors: {:?}",
            events
        );
    }

    // -------------------------------------------------------------------------
    // Surface slot cap enforcement
    // -------------------------------------------------------------------------

    #[test]
    fn surface_structure_rejected_when_slots_full() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .unwrap();

        // Fill all surface slots
        let (star_id, planet_index) = {
            let c = engine.state.colonies.get(&colony_id).unwrap();
            (c.star, c.planet_index)
        };
        let max_slots = engine
            .state
            .stars
            .get(&star_id)
            .unwrap()
            .planets
            .get(planet_index)
            .unwrap()
            .size
            .surface_slots();

        {
            let colony = engine.state.colonies.get_mut(&colony_id).unwrap();
            for _ in 0..max_slots {
                colony
                    .surface_installations
                    .push(BuildingType::FabricationYard);
            }
        }

        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::ScienceNexus),
        }]);

        assert!(
            events.iter().any(
                |e| matches!(e, Event::Error { message } if message.contains("no free surface slots"))
            ),
            "Surface structure must be rejected when all slots are full, got: {:?}",
            events
        );
        assert!(
            engine
                .state
                .colonies
                .get(&colony_id)
                .unwrap()
                .build_queue
                .is_empty(),
            "build queue must stay empty after surface-slot rejection"
        );
    }

    #[test]
    fn surface_structure_allowed_when_slot_available() {
        let mut engine = Engine::new(42);
        let colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == engine.state.player_empire)
            .map(|(id, _)| *id)
            .unwrap();

        // Fresh colony has available slots
        let events = engine.apply_turn(vec![Command::QueueBuild {
            colony: colony_id,
            item: BuildItem::Structure(BuildingType::AquacultureBay),
        }]);

        assert!(
            !events.iter().any(|e| e.is_error()),
            "Surface structure must be accepted when slots are available, got: {:?}",
            events
        );
    }
}
