//! Game engine - command processing and turn execution

use crate::commands::Command;
use crate::deterministic::sorted_colony_ids;
use crate::events::Event;
use crate::galaxy::{find_home_star, generate_galaxy};
use crate::state::{
    all_techs, BuildItem, Colony, ColonyId, Empire, EmpireId, Fleet, FleetId, GameState,
    ResearchState, StarId, TechId,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeMap;

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

        // Find a suitable home star
        let home_star =
            find_home_star(&stars_vec).expect("Galaxy should have at least one habitable star");
        let home_star_id = home_star.id;

        // Create player empire
        let player_empire_id = EmpireId(1);
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
            },
        );

        // Create initial colony
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
            },
        );

        // Update star's planet to reference the colony
        if let Some(star) = stars.get_mut(&home_star_id) {
            if let Some(planet) = star.planets.get_mut(0) {
                planet.colony = Some(colony_id);
            }
        }

        // Create initial scout fleet
        let fleet_id = FleetId(1);
        let mut fleets = BTreeMap::new();
        fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: player_empire_id,
                location: home_star_id,
                ships: 1,
            },
        );

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
            next_colony_id: 2,
            next_fleet_id: 2,
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

        // Track per-empire research generated this turn (owner_id -> research_points)
        let mut empire_research: std::collections::BTreeMap<EmpireId, i64> =
            std::collections::BTreeMap::new();

        for colony_id in colony_ids {
            // Get colony data
            let (
                owner,
                production,
                prod_pct,
                research_pct,
                star_id,
                build_queue_front,
                accumulated,
            ) = {
                let colony = self.state.colonies.get(&colony_id).unwrap();
                (
                    colony.owner,
                    colony.production,
                    colony.prod_pct,
                    colony.research_pct,
                    colony.star,
                    colony.build_queue.first().copied(),
                    colony.accumulated_production,
                )
            };

            // Calculate output
            let total_output = production as i64;
            let credits = (total_output * prod_pct as i64) / 100;
            let research = (total_output * research_pct as i64) / 100;

            // Update empire credits and lifetime research total
            if let Some(empire) = self.state.empires.get_mut(&owner) {
                empire.credits += credits;
                empire.research_points += research;
            }

            // Accumulate research for this empire for tech progress
            *empire_research.entry(owner).or_insert(0) += research;

            events.push(Event::ColonyProduced {
                colony: colony_id,
                credits,
                research,
            });

            // Process build queue
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
                            self.state.fleets.insert(
                                fleet_id,
                                Fleet {
                                    id: fleet_id,
                                    owner: owner_id,
                                    location: star_id,
                                    ships: 1,
                                },
                            );
                            events.push(Event::FleetCreated {
                                fleet: fleet_id,
                                location: star_id,
                            });
                        }
                        // Add permanent buildings to the colony
                        BuildItem::Structure(bt) => {
                            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                                colony.buildings.push(bt);
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
                    // Tech completed
                    if let Some(empire) = self.state.empires.get_mut(empire_id) {
                        empire.research.completed.push(tech_id);
                        empire.research.current_tech = None;
                        empire.research.progress = 0;
                    }
                    events.push(Event::ResearchCompleted { tech: tech_id });
                } else {
                    if let Some(empire) = self.state.empires.get_mut(empire_id) {
                        empire.research.progress = new_progress;
                    }
                }
            }
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

        let from = fleet.location;

        // Validate destination exists
        if !self.state.stars.contains_key(&destination) {
            events.push(Event::error(format!(
                "Destination star {} not found",
                destination.0
            )));
            return;
        }

        // Apply move
        if let Some(fleet) = self.state.fleets.get_mut(&fleet_id) {
            fleet.location = destination;
        }

        events.push(Event::FleetMoved {
            fleet: fleet_id,
            from,
            to: destination,
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

        // Select the tech; only reset progress when switching to a different tech
        if let Some(empire) = self.state.empires.get_mut(&empire_id) {
            if empire.research.current_tech != Some(tech_id) {
                empire.research.progress = 0;
            }
            empire.research.current_tech = Some(tech_id);
        }

        events.push(Event::ResearchSelected { tech: tech_id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BuildingType;

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

        // Find a different star to move to
        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        let destination = engine
            .state
            .stars
            .keys()
            .find(|&id| *id != initial_location)
            .copied()
            .unwrap();

        let events = engine.apply_turn(vec![Command::MoveFleet {
            fleet: fleet_id,
            destination,
        }]);

        assert!(!events.iter().any(|e| e.is_error()));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::FleetMoved { fleet, from, to }
            if *fleet == fleet_id && *from == initial_location && *to == destination)));

        let fleet = engine.state.fleets.get(&fleet_id).unwrap();
        assert_eq!(fleet.location, destination);
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
}
