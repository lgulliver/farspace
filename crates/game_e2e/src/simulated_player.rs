use game_core::{
    available_tech_ids, BuildItem, BuildingType, ColonyId, ColonyRole, Command, DiplomaticResponse,
    EmpireId, FleetId, FleetKind, FleetRole, GameState, StarId, TechId,
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulatedPlayerPolicy {
    BalancedExplorer,
}

pub trait SimulatedPlayer {
    fn choose_actions(
        &mut self,
        observation: &PlayerObservation,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Command>;
}

#[derive(Debug, Clone)]
pub struct PlayerObservation {
    pub turn: u32,
    pub player_empire: EmpireId,
    pub known_stars: Vec<StarId>,
    pub known_colonies: Vec<ColonyId>,
    pub idle_player_fleets: Vec<FleetId>,
    pub visible_unsurveyed_planets: Vec<(StarId, usize)>,
    pub colonizable_planets: Vec<(StarId, usize)>,
    pub available_research: Vec<TechId>,
    pub active_research: Option<TechId>,
    pub pending_player_communications: Vec<u64>,
    pub communication_responses: Vec<(u64, DiplomaticResponse)>,
    pub idle_scouts: Vec<FleetId>,
    pub idle_science_fleets: Vec<(FleetId, StarId)>,
    pub idle_colonizers: Vec<(FleetId, StarId)>,
    pub unknown_stars: Vec<StarId>,
    pub colonies_without_queue: Vec<ColonyId>,
    pub colonies_role_candidates: Vec<ColonyId>,
}

impl PlayerObservation {
    pub fn from_state(state: &GameState) -> Self {
        let player_empire = state.player_empire;
        let known_stars = state.explored_stars.iter().copied().collect::<Vec<_>>();
        let known_star_set = state.explored_stars.clone();
        let unknown_stars = state
            .stars
            .keys()
            .filter(|star| !known_star_set.contains(star))
            .copied()
            .collect::<Vec<_>>();

        let known_colonies = state
            .colonies
            .values()
            .filter(|colony| colony.owner == player_empire)
            .map(|colony| colony.id)
            .collect::<Vec<_>>();

        let idle_player_fleets = state
            .fleets
            .values()
            .filter(|fleet| {
                fleet.owner == player_empire
                    && !state.scout_missions.contains_key(&fleet.id)
                    && !state.survey_missions.contains_key(&fleet.id)
                    && !state.fleet_missions.contains_key(&fleet.id)
            })
            .map(|fleet| fleet.id)
            .collect::<Vec<_>>();
        let idle_player_fleet_set = idle_player_fleets.iter().copied().collect::<HashSet<_>>();

        let idle_scouts = state
            .fleets
            .values()
            .filter(|fleet| {
                idle_player_fleet_set.contains(&fleet.id) && fleet.kind == FleetKind::Scout
            })
            .map(|fleet| fleet.id)
            .collect::<Vec<_>>();

        let idle_science_fleets = state
            .fleets
            .values()
            .filter(|fleet| {
                idle_player_fleet_set.contains(&fleet.id) && fleet.kind == FleetKind::Science
            })
            .map(|fleet| (fleet.id, fleet.location))
            .collect::<Vec<_>>();

        let idle_colonizers = state
            .fleets
            .values()
            .filter(|fleet| {
                idle_player_fleet_set.contains(&fleet.id) && fleet.kind == FleetKind::Colonizer
            })
            .map(|fleet| (fleet.id, fleet.location))
            .collect::<Vec<_>>();

        let visible_unsurveyed_planets = known_stars
            .iter()
            .flat_map(|star_id| {
                state.stars.get(star_id).into_iter().flat_map(move |star| {
                    star.planets
                        .iter()
                        .enumerate()
                        .filter(|(_, planet)| !planet.surveyed)
                        .map(move |(index, _)| (*star_id, index))
                })
            })
            .collect::<Vec<_>>();

        let colonizable_planets = known_stars
            .iter()
            .flat_map(|star_id| {
                state.stars.get(star_id).into_iter().flat_map(move |star| {
                    star.planets
                        .iter()
                        .enumerate()
                        .filter(|(_, planet)| {
                            planet.habitable && planet.colony.is_none() && planet.surveyed
                        })
                        .map(move |(index, _)| (*star_id, index))
                })
            })
            .collect::<Vec<_>>();

        let available_research = state
            .empires
            .get(&player_empire)
            .map(|empire| {
                available_tech_ids(&empire.research.completed)
                    .into_iter()
                    .filter(|tech| !empire.research.queue.contains(tech))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let active_research = state
            .empires
            .get(&player_empire)
            .and_then(|empire| empire.research.current_tech);

        let pending_player_communications = state
            .diplomacy_pending_communications
            .iter()
            .filter(|communication| communication.receiving_empire == player_empire)
            .map(|communication| communication.communication_id)
            .collect::<Vec<_>>();

        let communication_responses = state
            .diplomacy_pending_communications
            .iter()
            .filter(|communication| communication.receiving_empire == player_empire)
            .map(|communication| {
                let response = communication.available_responses.iter().find(|response| {
                    matches!(
                        response,
                        DiplomaticResponse::Accept | DiplomaticResponse::Acknowledge
                    )
                });
                (
                    communication.communication_id,
                    response
                        .cloned()
                        .or_else(|| communication.available_responses.first().cloned())
                        .unwrap_or(DiplomaticResponse::Acknowledge),
                )
            })
            .collect::<Vec<_>>();

        let colonies_without_queue = state
            .colonies
            .values()
            .filter(|colony| colony.owner == player_empire && colony.build_queue.is_empty())
            .map(|colony| colony.id)
            .collect::<Vec<_>>();

        let colonies_role_candidates = state
            .colonies
            .values()
            .filter(|colony| colony.owner == player_empire)
            .map(|colony| colony.id)
            .collect::<Vec<_>>();

        Self {
            turn: state.turn,
            player_empire,
            known_stars,
            known_colonies,
            idle_player_fleets,
            visible_unsurveyed_planets,
            colonizable_planets,
            available_research,
            active_research,
            pending_player_communications,
            communication_responses,
            idle_scouts,
            idle_science_fleets,
            idle_colonizers,
            unknown_stars,
            colonies_without_queue,
            colonies_role_candidates,
        }
    }
}

pub struct BalancedExplorerPlayer;

impl BalancedExplorerPlayer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BalancedExplorerPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedPlayer for BalancedExplorerPlayer {
    fn choose_actions(
        &mut self,
        observation: &PlayerObservation,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Command> {
        let mut actions = Vec::new();

        for (communication_id, response) in &observation.communication_responses {
            actions.push(Command::RespondToCommunication {
                communication_id: *communication_id,
                response: *response,
            });
        }

        if observation.active_research.is_none() {
            if let Some(tech) = observation.available_research.first() {
                actions.push(Command::SelectResearch { tech: *tech });
            }
        }

        if observation.available_research.len() > 1 {
            if let Some(tech) = observation.available_research.get(1) {
                actions.push(Command::QueueResearch { tech: *tech });
            }
        }

        for colony in observation.colonies_without_queue.iter().take(2) {
            let build_item = if observation.turn.is_multiple_of(7) {
                BuildItem::Ship(game_core::ShipDesignId::COLONY)
            } else if observation.turn.is_multiple_of(3) {
                BuildItem::SurfaceStructure(BuildingType::ScienceNexus)
            } else {
                BuildItem::SurfaceStructure(BuildingType::FabricationYard)
            };
            actions.push(Command::QueueBuild {
                colony: *colony,
                item: build_item,
            });
        }

        if observation.turn.is_multiple_of(5) {
            if let Some(colony) = observation.colonies_role_candidates.first() {
                let role = match (observation.turn / 5) % 3 {
                    0 => ColonyRole::Balanced,
                    1 => ColonyRole::Industrial,
                    _ => ColonyRole::Scientific,
                };
                actions.push(Command::SetColonyRole {
                    colony: *colony,
                    role,
                });
                actions.push(Command::SetColonyFocus {
                    colony: *colony,
                    prod_pct: if matches!(role, ColonyRole::Industrial) {
                        70
                    } else if matches!(role, ColonyRole::Scientific) {
                        30
                    } else {
                        50
                    },
                    research_pct: if matches!(role, ColonyRole::Scientific) {
                        70
                    } else if matches!(role, ColonyRole::Industrial) {
                        30
                    } else {
                        50
                    },
                });
            }
        }

        if let Some((fleet, location)) = observation.idle_colonizers.first() {
            if let Some((star, planet_index)) = observation
                .colonizable_planets
                .iter()
                .find(|(star, _)| star == location)
            {
                actions.push(Command::Colonize {
                    fleet: *fleet,
                    star: *star,
                    planet_index: *planet_index,
                });
            }
        }

        for (fleet, location) in observation.idle_science_fleets.iter().take(1) {
            if let Some((star, planet_index)) = observation
                .visible_unsurveyed_planets
                .iter()
                .find(|(star, _)| star == location)
            {
                actions.push(Command::SurveyPlanet {
                    fleet: *fleet,
                    star: *star,
                    planet_index: *planet_index,
                });
            }
        }

        if let (Some(fleet), Some(destination)) = (
            observation.idle_scouts.first(),
            observation.unknown_stars.first(),
        ) {
            actions.push(Command::SendScout {
                fleet: *fleet,
                destination: *destination,
            });
            actions.push(Command::SetFleetRole {
                fleet: *fleet,
                role: FleetRole::ExplorationFleet,
            });
        }

        if observation.turn.is_multiple_of(8) {
            if let Some(fleet) = observation.idle_player_fleets.first() {
                if rng.gen_bool(0.5) {
                    actions.push(Command::SetFleetRole {
                        fleet: *fleet,
                        role: FleetRole::ExplorationFleet,
                    });
                }
            }
        }

        actions
    }
}
