//! Game engine - command processing and turn execution

mod research;
mod setup;

use crate::commands::Command;
use crate::deterministic::sorted_colony_ids;
use crate::events::Event;
use crate::galaxy::{find_home_star, generate_galaxy_with_config, generate_hyperspace_lanes};
use crate::state::{
    all_techs, empire_definition_by_id, is_tech_available, tech_by_id, tech_yield_bonus_per_colony,
    BuildItem, Colony, ColonyId, ColonyRole, ColonySupplyState, Empire, EmpireId, Fleet, FleetId,
    FleetKind, FleetMission, FleetOrder, GameState, HyperspaceLane, OrbitalStructureType,
    RelationshipStatus, ResearchState, ScenarioSetup, ScoutMission, ShipDesignId, StarId,
    SurveyMission, TechId, YieldType,
};
use crate::yield_model::YieldContext;
#[cfg(test)]
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

/// Number of turns for a science ship to survey a planet
pub(crate) const SURVEY_TURNS: u32 = 2;

/// Salt XOR'd with the game seed when shuffling AI empire definitions.
///
/// Using a distinct seed (game_seed ^ EMPIRE_ASSIGN_SALT) for the shuffle RNG
/// keeps the empire identity assignment independent from galaxy-generation and
/// in-game RNG streams while remaining fully deterministic for a given game seed.
const EMPIRE_ASSIGN_SALT: u64 = 0x6172_7473_5f49_5044;

/// Fleet travel speed in galaxy coordinate units per turn.
///
/// Stars are generated in the range `-500..=500` on each axis, giving a maximum
/// possible distance of `≈1414` units.  A speed of `500` yields:
/// * dist  ≤ 500  →  1 turn  (close-range, same or adjacent sectors)
/// * dist  ≤ 1000 →  2 turns (medium-range)
/// * dist  > 1000 →  3 turns (long-range, far sectors)
const FLEET_TRAVEL_SPEED: f64 = 500.0;
/// Direct hyperspace lanes reduce duration to `ceil(base_turns / 2)`.
const HYPERSPACE_TRAVEL_DIVISOR: u32 = 2;
const ISOLATED_YIELD_PERCENT: i64 = 50;
const ISOLATED_STABILITY_PENALTY: u8 = 5;
const MAX_HOUSING_DEFICIT_STABILITY_PENALTY: u8 = 10;
const MAX_UNEMPLOYMENT_STABILITY_PENALTY: u8 = 5;
const MAX_ISOLATED_FOOD_DEFICIT_STABILITY_PENALTY: u8 = 5;
/// Stability penalty applied each turn to a blockaded colony.
/// Blockade yield reduction reuses `ISOLATED_YIELD_PERCENT` via `apply_isolation_penalty`.
const BLOCKADED_STABILITY_PENALTY: u8 = 5;
const MIN_STABILITY_FOR_POP_GROWTH: u8 = 90;
const POP_GROWTH_PERIOD_TURNS: u32 = 12;
/// Fixed invasion strength contributed by one troop transport ship.
const TROOP_TRANSPORT_INVASION_STRENGTH: u32 = 12;
/// Colony stability after a successful capture.
const CAPTURED_UNREST_STABILITY: u8 = 40;
/// Squared-distance threshold for routine border pressure.
///
/// `40_000` corresponds to roughly 200 map units, which is close enough for
/// neighboring home systems and early border colonies to feel contested without
/// treating the entire sector as immediate pressure.
const BORDER_TENSION_DISTANCE_SQ: i64 = 40_000;
/// Squared-distance threshold for severe border pressure.
///
/// `12_000` corresponds to roughly 110 map units, representing very close
/// frontier overlap where the AI is allowed to escalate toward its harshest
/// diplomacy posture.
const SEVERE_BORDER_TENSION_DISTANCE_SQ: i64 = 12_000;

#[derive(Debug, Clone, Copy, Default)]
struct YieldBonuses {
    credits: i64,
    science: i64,
    food: i64,
}

/// Return the number of travel turns for a fleet moving the given squared Euclidean distance.
///
/// Formula: `turns = max(1, ceil(sqrt(sq_dist) / FLEET_TRAVEL_SPEED))`
///
/// This is deterministic for all integer squared-distance inputs because
/// `f64` square-root is IEEE 754 compliant and the inputs are bounded well
/// within the range where `f64` is exact.
pub(crate) fn fleet_travel_turns(squared_distance: i64) -> u32 {
    let dist = (squared_distance as f64).sqrt();
    ((dist / FLEET_TRAVEL_SPEED).ceil() as u32).max(1)
}

fn lane_travel_turns(base_turns: u32) -> u32 {
    base_turns.div_ceil(HYPERSPACE_TRAVEL_DIVISOR).max(1)
}

fn apply_isolation_penalty(credits: i64, research: i64, _food: i64) -> (i64, i64, i64) {
    (
        credits * ISOLATED_YIELD_PERCENT / 100,
        research * ISOLATED_YIELD_PERCENT / 100,
        0,
    )
}

/// Apply blockade penalties: same percentage reduction as isolation, and no food.
///
/// Blockade and isolation share identical yield arithmetic so this delegates
/// to `apply_isolation_penalty` to keep the two in sync if the percentages
/// ever change.
fn apply_blockade_penalty(credits: i64, research: i64, food: i64) -> (i64, i64, i64) {
    apply_isolation_penalty(credits, research, food)
}

/// Apply an empire identity percentage modifier to a production cost.
///
/// Negative percentages reduce the cost, positive percentages increase it, and
/// the result is clamped to at least `1` so extreme discounts never create
/// zero-cost production items.
fn apply_cost_modifier(base_cost: u64, modifier_pct: i8) -> u64 {
    if modifier_pct == 0 {
        return base_cost;
    }
    let adjusted = (base_cost as i64 * (100 + modifier_pct as i64)) / 100;
    adjusted.max(1) as u64
}

/// Map relationship states onto an ordered numeric ladder used for diplomacy drift.
///
/// Lower values are calmer and higher values are more aggressive, allowing the
/// engine to move one step at a time toward an empire's desired stance.
fn relationship_level(status: RelationshipStatus) -> u8 {
    match status {
        RelationshipStatus::Unknown => 0,
        RelationshipStatus::Contacted => 1,
        RelationshipStatus::Neutral => 2,
        RelationshipStatus::Tense => 3,
        RelationshipStatus::Hostile => 4,
        RelationshipStatus::War => 5,
    }
}

fn relationship_from_level(level: u8) -> RelationshipStatus {
    match level {
        0 => RelationshipStatus::Unknown,
        1 => RelationshipStatus::Contacted,
        2 => RelationshipStatus::Neutral,
        3 => RelationshipStatus::Tense,
        4 => RelationshipStatus::Hostile,
        _ => RelationshipStatus::War,
    }
}

/// Move one or more diplomatic steps from `current` toward `desired`.
///
/// This keeps diplomacy drift deterministic and bounded: each turn can only
/// improve or worsen a relationship by up to `steps` status levels.
fn step_toward_relationship(
    current: RelationshipStatus,
    desired: RelationshipStatus,
    steps: u8,
) -> RelationshipStatus {
    let current_level = relationship_level(current);
    let desired_level = relationship_level(desired);
    if current_level == desired_level {
        return current;
    }
    let delta = steps.max(1);
    if current_level < desired_level {
        return relationship_from_level(current_level.saturating_add(delta));
    }
    relationship_from_level(current_level.saturating_sub(delta))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorderPressure {
    Calm,
    Tense,
    Severe,
}

fn has_tech(state: &GameState, empire_id: EmpireId, tech: TechId) -> bool {
    state
        .empires
        .get(&empire_id)
        .is_some_and(|e| e.research.completed.contains(&tech))
}

fn empire_knows_lane(state: &GameState, empire_id: EmpireId, lane: HyperspaceLane) -> bool {
    if empire_id == state.player_empire {
        return state.known_hyperspace_lanes.contains(&lane);
    }
    // AI empires (any empire in ai_empires, or the legacy ai_empire field) can
    // reason over the full lane topology once they have the required technology.
    let is_ai = state.ai_empires.contains(&empire_id) || Some(empire_id) == state.ai_empire;
    if is_ai {
        return true;
    }
    false
}

fn can_use_lane_for_route(
    state: &GameState,
    empire_id: EmpireId,
    from: StarId,
    to: StarId,
) -> bool {
    let Some(lane) = HyperspaceLane::new(from, to) else {
        return false;
    };
    state.hyperspace_lanes.contains(&lane)
        && empire_knows_lane(state, empire_id, lane)
        && has_tech(state, empire_id, TechId::HYPERSPACE_CARTOGRAPHY)
}

pub(crate) fn travel_turns_with_lanes(
    state: &GameState,
    empire_id: EmpireId,
    from: StarId,
    to: StarId,
) -> (u32, bool) {
    let src = state.stars.get(&from).expect("origin star must exist");
    let dst = state.stars.get(&to).expect("destination star must exist");
    let dx = (dst.x - src.x) as i64;
    let dy = (dst.y - src.y) as i64;
    let base_turns = fleet_travel_turns(dx * dx + dy * dy);
    if !can_use_lane_for_route(state, empire_id, from, to) {
        return (base_turns, false);
    }
    (lane_travel_turns(base_turns), true)
}

/// The game engine processes commands and manages game state
#[derive(Debug)]
pub struct Engine {
    pub state: GameState,
    last_turn_colony_supply: BTreeMap<ColonyId, ColonySupplyState>,
    last_turn_colony_blockade: BTreeMap<ColonyId, EmpireId>,
}

impl Engine {
    fn empire_definition(
        &self,
        empire_id: EmpireId,
    ) -> Option<&'static crate::state::EmpireDefinition> {
        self.state
            .empires
            .get(&empire_id)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
    }

    fn effective_build_cost(&self, empire_id: EmpireId, item: BuildItem) -> u64 {
        let Some(def) = self.empire_definition(empire_id) else {
            return item.cost();
        };
        match item {
            BuildItem::Ship(ShipDesignId::SCOUT) => {
                apply_cost_modifier(item.cost(), def.military_modifiers.scout_cost_modifier_pct)
            }
            BuildItem::Ship(ShipDesignId::FAST_SCOUT) => {
                // Fast Scout benefits from the same scout cost modifier
                apply_cost_modifier(item.cost(), def.military_modifiers.scout_cost_modifier_pct)
            }
            BuildItem::Ship(ShipDesignId::SCIENCE) => apply_cost_modifier(
                item.cost(),
                def.military_modifiers.science_ship_cost_modifier_pct,
            ),
            BuildItem::Ship(ShipDesignId::SURVEY_CUTTER) => apply_cost_modifier(
                item.cost(),
                def.military_modifiers.science_ship_cost_modifier_pct,
            ),
            BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT) => apply_cost_modifier(
                item.cost(),
                def.military_modifiers.troop_transport_cost_modifier_pct,
            ),
            BuildItem::Ship(ShipDesignId::ESCORT_FRIGATE)
            | BuildItem::Ship(ShipDesignId::MISSILE_FRIGATE)
            | BuildItem::Ship(ShipDesignId::DESTROYER)
            | BuildItem::Ship(ShipDesignId::PATROL_CORVETTE) => apply_cost_modifier(
                item.cost(),
                def.military_modifiers.combat_ship_cost_modifier_pct,
            ),
            BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard) => apply_cost_modifier(
                item.cost(),
                def.military_modifiers.shipyard_cost_modifier_pct,
            ),
            _ => item.cost(),
        }
    }

    fn first_contact_status_for_empire(&self, empire_id: EmpireId) -> RelationshipStatus {
        self.empire_definition(empire_id)
            .map(|def| def.diplomacy_profile.first_contact_status)
            .unwrap_or(RelationshipStatus::Contacted)
    }

    /// Compute total fleet maintenance for an empire.
    ///
    /// Each fleet contributes its kind's base maintenance cost, adjusted by
    /// the empire's `fleet_maintenance_modifier_per_fleet` (flat per-fleet delta).
    fn fleet_maintenance_for_empire(&self, empire_id: EmpireId) -> i64 {
        let modifier = self
            .empire_definition(empire_id)
            .map(|def| def.military_modifiers.fleet_maintenance_modifier_per_fleet)
            .unwrap_or(0);
        self.state
            .fleets
            .values()
            .filter(|f| f.owner == empire_id)
            .map(|f| (f.kind.maintenance_cost() as i64 + modifier).max(0))
            .sum()
    }

    fn invasion_strength_for_empire(&self, empire_id: EmpireId, ships: u32) -> u32 {
        let bonus = self
            .empire_definition(empire_id)
            .map(|def| def.military_modifiers.invasion_strength_bonus_per_transport)
            .unwrap_or(0);
        ships.saturating_mul(TROOP_TRANSPORT_INVASION_STRENGTH.saturating_add(bonus))
    }

    fn ai_border_pressure(&self, ai_empire_id: EmpireId) -> BorderPressure {
        let player = self.state.player_empire;
        let ai_colony_stars: Vec<StarId> = self
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai_empire_id)
            .map(|c| c.star)
            .collect();
        if ai_colony_stars.is_empty() {
            return BorderPressure::Calm;
        }

        let player_fleet_at_ai_colony = self
            .state
            .fleets
            .values()
            .any(|fleet| fleet.owner == player && ai_colony_stars.contains(&fleet.location));
        if player_fleet_at_ai_colony {
            return BorderPressure::Severe;
        }

        let player_colony_stars: Vec<StarId> = self
            .state
            .colonies
            .values()
            .filter(|c| c.owner == player)
            .map(|c| c.star)
            .collect();
        let min_sq_dist = player_colony_stars
            .iter()
            .flat_map(|player_star| {
                ai_colony_stars.iter().filter_map(move |ai_star| {
                    let src = self.state.stars.get(player_star)?;
                    let dst = self.state.stars.get(ai_star)?;
                    let dx = (dst.x - src.x) as i64;
                    let dy = (dst.y - src.y) as i64;
                    Some(dx * dx + dy * dy)
                })
            })
            .min();

        match min_sq_dist {
            Some(dist) if dist <= SEVERE_BORDER_TENSION_DISTANCE_SQ => BorderPressure::Severe,
            Some(dist) if dist <= BORDER_TENSION_DISTANCE_SQ => BorderPressure::Tense,
            _ => BorderPressure::Calm,
        }
    }

    fn process_ai_diplomacy(&mut self) {
        let player = self.state.player_empire;
        let ai_ids = if !self.state.ai_empires.is_empty() {
            self.state.ai_empires.clone()
        } else {
            self.state.ai_empire.into_iter().collect()
        };

        for ai_empire_id in ai_ids {
            let current = self.state.relationship_status(player, ai_empire_id);
            if matches!(
                current,
                RelationshipStatus::Unknown | RelationshipStatus::War
            ) {
                continue;
            }

            let pressure = self.ai_border_pressure(ai_empire_id);
            let desired = self
                .empire_definition(ai_empire_id)
                .map(|def| match pressure {
                    BorderPressure::Calm => def.diplomacy_profile.resting_status,
                    BorderPressure::Tense => def.diplomacy_profile.border_tension_status,
                    BorderPressure::Severe => def.diplomacy_profile.severe_border_tension_status,
                })
                .unwrap_or(RelationshipStatus::Neutral);
            let step_size = self
                .empire_definition(ai_empire_id)
                .map(|def| {
                    let escalating = relationship_level(desired) > relationship_level(current);
                    let aggression = def.doctrine_weight(AiDoctrine::Militarist)
                        + def.doctrine_weight(AiDoctrine::Imperial);
                    let caution = def.doctrine_weight(AiDoctrine::Isolationist)
                        + def.doctrine_weight(AiDoctrine::Merchant);
                    let aggressive_jump = escalating
                        && matches!(pressure, BorderPressure::Severe)
                        && aggression >= caution.saturating_add(6);
                    let calming_jump = !escalating
                        && (def.doctrine_weight(AiDoctrine::Isolationist) >= 8
                            || def.doctrine_weight(AiDoctrine::Merchant) >= 8);
                    if aggressive_jump || calming_jump {
                        2
                    } else {
                        1
                    }
                })
                .unwrap_or(1);
            let next = step_toward_relationship(current, desired, step_size);
            if next != current {
                self.state.diplomacy.insert(ai_empire_id, next);
            }
        }
    }

    fn refresh_known_hyperspace_lanes(&mut self) {
        for lane in &self.state.hyperspace_lanes {
            if self.state.explored_stars.contains(&lane.a())
                && self.state.explored_stars.contains(&lane.b())
            {
                self.state.known_hyperspace_lanes.insert(*lane);
            }
        }
    }

    fn refresh_colony_supply_statuses(&mut self) {
        self.state.colony_supply = self.state.recompute_colony_supply();
    }

    /// Apply a list of commands and return generated events
    pub fn apply_turn(&mut self, commands: Vec<Command>) -> Vec<Event> {
        let mut events = Vec::new();
        let mut processed_end_turn = false;

        for command in commands {
            match command {
                Command::EndTurn => {
                    processed_end_turn = true;
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
                Command::QueueResearch { tech } => {
                    self.process_queue_research(tech, &mut events);
                }
                Command::RemoveQueuedResearch { tech } => {
                    self.process_remove_queued_research(tech, &mut events);
                }
                Command::MoveQueuedResearchUp { tech } => {
                    self.process_move_queued_research_up(tech, &mut events);
                }
                Command::MoveQueuedResearchDown { tech } => {
                    self.process_move_queued_research_down(tech, &mut events);
                }
                Command::ClearResearchQueue => {
                    self.process_clear_research_queue(&mut events);
                }
                Command::SendScout { fleet, destination } => {
                    self.process_send_scout(fleet, destination, &mut events);
                }
                Command::SurveyPlanet {
                    fleet,
                    star,
                    planet_index,
                } => {
                    self.process_survey_planet(fleet, star, planet_index, &mut events);
                }
                Command::Colonize {
                    fleet,
                    star,
                    planet_index,
                } => {
                    self.process_colonize(fleet, star, planet_index, &mut events);
                }
                Command::Invade {
                    fleet,
                    star,
                    planet_index,
                } => {
                    self.process_invade(fleet, star, planet_index, &mut events);
                }
                Command::SetColonyRole { colony, role } => {
                    self.process_set_colony_role(colony, role, &mut events);
                }
                Command::SetRallyPoint { colony, star } => {
                    self.process_set_rally_point(colony, star, &mut events);
                }
                Command::ClearRallyPoint { colony } => {
                    self.process_clear_rally_point(colony, &mut events);
                }
                Command::SetFleetOrder { fleet, order } => {
                    self.process_set_fleet_order(fleet, order, &mut events);
                }
                Command::DeclareWar { target } => {
                    self.process_declare_war(target, &mut events);
                }
            }
        }

        if !processed_end_turn {
            self.refresh_colony_supply_statuses();
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
        let previous_supply = self.last_turn_colony_supply.clone();
        self.refresh_colony_supply_statuses();
        let current_turn_supply = self.state.colony_supply.clone();
        // Blockade state from last turn (persisted in GameState): use for economy penalties.
        let current_turn_blockade = self.state.colony_blockade.clone();

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
        // Precompute per-colony tech yield bonuses per empire once for this turn.
        let empire_tech_yield_bonus_per_colony: std::collections::BTreeMap<EmpireId, YieldBonuses> =
            self.state
                .empires
                .iter()
                .map(|(empire_id, empire)| {
                    let completed = &empire.research.completed;
                    (
                        *empire_id,
                        YieldBonuses {
                            credits: tech_yield_bonus_per_colony(completed, YieldType::Credits),
                            science: tech_yield_bonus_per_colony(completed, YieldType::Science),
                            food: tech_yield_bonus_per_colony(completed, YieldType::Food),
                        },
                    )
                })
                .collect();

        for colony_id in colony_ids {
            // Get colony data needed for yield calculation and build queue
            let (
                owner,
                production,
                star_id,
                build_queue_front,
                accumulated,
                colony_role,
                colony_stability,
            ) = {
                let colony = self.state.colonies.get(&colony_id).unwrap();
                (
                    colony.owner,
                    colony.production,
                    colony.star,
                    colony.build_queue.first().copied(),
                    colony.accumulated_production,
                    colony.role,
                    colony.stability,
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

            let is_connected = matches!(
                current_turn_supply.get(&colony_id),
                Some(ColonySupplyState::Connected)
            );
            let is_blockaded = current_turn_blockade.contains_key(&colony_id);
            let empire_food_shortage = self
                .state
                .empires
                .get(&owner)
                .map(|e| e.food < 0)
                .unwrap_or(false);
            let context = YieldContext {
                food_shortage: empire_food_shortage || !is_connected || is_blockaded,
                stability_pressure: colony_stability < 85 || is_blockaded,
            };

            // Calculate yield via the pop/jobs model.
            let colony_yield = {
                let colony = self.state.colonies.get(&colony_id).unwrap();
                crate::yield_model::calculate_yield_with_context(colony, planet.as_ref(), context)
            };

            let bonuses = empire_tech_yield_bonus_per_colony
                .get(&owner)
                .copied()
                .unwrap_or_default();

            // Apply empire identity trait modifiers on top of tech bonuses.
            let empire_def_mods = self
                .state
                .empires
                .get(&owner)
                .and_then(|e| e.empire_def)
                .and_then(crate::state::empire_definition_by_id)
                .map(|d| d.trait_modifiers)
                .unwrap_or_default();

            let mut credits =
                colony_yield.credits + bonuses.credits + empire_def_mods.credits_per_colony;
            let mut research =
                colony_yield.science + bonuses.science + empire_def_mods.science_per_colony;
            let mut food = colony_yield.food + bonuses.food + empire_def_mods.food_per_colony;
            // Industry modifier from empire def is already informational here; the
            // yield model computed the base industry.  We expose the bonus via
            // the ColonyProduced event so the UI can show "empire bonus" detail.
            let industry = colony_yield.industry + empire_def_mods.industry_per_colony;
            // Blockade takes effect first; if blockaded, treat as effectively isolated
            // (same yield penalty + stability hit). If already isolated but not blockaded
            // the normal isolation path below handles it — no double penalty.
            if is_blockaded {
                (credits, research, food) = apply_blockade_penalty(credits, research, food);
                if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                    colony.stability = colony.stability.saturating_sub(BLOCKADED_STABILITY_PENALTY);
                }
            } else if !is_connected {
                (credits, research, food) = apply_isolation_penalty(credits, research, food);
                if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                    colony.stability = colony.stability.saturating_sub(ISOLATED_STABILITY_PENALTY);
                }
            }

            let food_deficit = (colony_yield.food_consumed - colony_yield.food).max(0);
            let housing_deficit = colony_yield.workforce.housing_deficit;
            let unemployed = colony_yield.workforce.unemployed;
            if food_deficit > 0 || housing_deficit > 0 || unemployed > 0 {
                events.push(Event::ColonyStatusWarning {
                    colony: colony_id,
                    food_deficit,
                    housing_deficit,
                    unemployed,
                });
            }
            let mut pressure_penalty = 0u8;
            if housing_deficit > 0 {
                pressure_penalty = pressure_penalty.saturating_add(
                    housing_deficit.min(MAX_HOUSING_DEFICIT_STABILITY_PENALTY as u64) as u8,
                );
            }
            if unemployed > 0 {
                pressure_penalty =
                    pressure_penalty.saturating_add(
                        unemployed.min(MAX_UNEMPLOYMENT_STABILITY_PENALTY as u64) as u8,
                    );
            }
            if !is_connected && food_deficit > 0 {
                pressure_penalty = pressure_penalty.saturating_add(
                    food_deficit.min(MAX_ISOLATED_FOOD_DEFICIT_STABILITY_PENALTY as i64) as u8,
                );
            }
            if pressure_penalty > 0 {
                if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                    colony.stability = colony.stability.saturating_sub(pressure_penalty);
                }
            }

            // Update empire credits and lifetime research total
            if let Some(empire) = self.state.empires.get_mut(&owner) {
                empire.credits += credits;
                empire.research_points += research;
            }

            // Accumulate per-empire totals.
            // Blockaded or isolated colonies do not contribute food to the empire trade network.
            let contributes_food = is_connected && !is_blockaded;
            *empire_research.entry(owner).or_insert(0) += research;
            *empire_credits_income.entry(owner).or_insert(0) += credits;
            if contributes_food {
                *empire_food_produced.entry(owner).or_insert(0) += food;
                *empire_food_consumed.entry(owner).or_insert(0) += colony_yield.food_consumed;
            }
            *empire_colony_maintenance.entry(owner).or_insert(0) += colony_yield.maintenance;

            events.push(Event::ColonyProduced {
                colony: colony_id,
                credits,
                research,
                food,
                industry,
                maintenance: colony_yield.maintenance,
            });

            // Process production queue — one active item at a time, with deterministic
            // overflow carry into subsequent queued items in the same turn.
            if let Some(item) = build_queue_front {
                let ship_bonus = if item.is_ship() {
                    colony_role.ship_production_bonus()
                } else {
                    0
                };
                let mut production_pool = accumulated + production + ship_bonus;

                // Determine how many items complete this turn and collect them.
                // We read the queue once, computing completions, then drain the prefix.
                let completed_items: Vec<BuildItem> = {
                    let queue = self
                        .state
                        .colonies
                        .get(&colony_id)
                        .map(|c| c.build_queue.as_slice())
                        .unwrap_or(&[]);
                    let mut completed = Vec::new();
                    for &q_item in queue {
                        let cost = self.effective_build_cost(owner, q_item);
                        if production_pool < cost {
                            break;
                        }
                        production_pool -= cost;
                        completed.push(q_item);
                    }
                    completed
                };

                // Drain the completed prefix in one O(n) pass.
                let n_completed = completed_items.len();
                if n_completed > 0 {
                    if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                        colony.build_queue.drain(..n_completed);
                    }
                }

                for current_item in completed_items {
                    events.push(Event::BuildCompleted {
                        colony: colony_id,
                        item: current_item,
                    });

                    match current_item {
                        BuildItem::Ship(design_id) => {
                            if let Some(design) = design_id.record() {
                                let fleet_id = self.state.next_fleet_id();
                                self.state.fleets.insert(
                                    fleet_id,
                                    Fleet {
                                        id: fleet_id,
                                        owner,
                                        location: star_id,
                                        ships: design.ships,
                                        kind: design.fleet_kind,
                                        strength: design.strength.max(1),
                                        integrity: 100,
                                    },
                                );
                                events.push(Event::FleetCreated {
                                    fleet: fleet_id,
                                    location: star_id,
                                });
                                self.maybe_route_to_rally_point(
                                    fleet_id, colony_id, star_id, events,
                                );
                            } else {
                                // Defensive guard: QueueBuild validation rejects unknown design IDs,
                                // so this should only occur if a corrupted save injected bad data.
                                events.push(Event::error(format!(
                                    "Invalid ship design {} completed at colony {}",
                                    design_id.0, colony_id.0
                                )));
                            }
                        }
                        // Legacy save compatibility paths.
                        BuildItem::Scout | BuildItem::Colony => {
                            let legacy_design = if matches!(current_item, BuildItem::Colony) {
                                ShipDesignId::COLONY
                            } else {
                                ShipDesignId::SCOUT
                            };
                            if let Some(design) = legacy_design.record() {
                                let fleet_id = self.state.next_fleet_id();
                                self.state.fleets.insert(
                                    fleet_id,
                                    Fleet {
                                        id: fleet_id,
                                        owner,
                                        location: star_id,
                                        ships: design.ships,
                                        kind: design.fleet_kind,
                                        strength: design.strength.max(1),
                                        integrity: 100,
                                    },
                                );
                                events.push(Event::FleetCreated {
                                    fleet: fleet_id,
                                    location: star_id,
                                });
                                self.maybe_route_to_rally_point(
                                    fleet_id, colony_id, star_id, events,
                                );
                            }
                        }
                        BuildItem::SurfaceStructure(bt) | BuildItem::Structure(bt) => {
                            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                                colony.buildings.push(bt);
                                colony.surface_installations.push(bt);
                            }
                        }
                        BuildItem::OrbitalStructure(ot) => {
                            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                                colony.orbital_installations.push(ot);
                            }
                        }
                        BuildItem::Outpost => {}
                    }
                }

                if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                    colony.accumulated_production = production_pool;
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
                    // Tech completed — overflow carries into deterministic queue processing.
                    let overflow = new_progress - tech_cost;
                    self.process_research_completion_with_queue(
                        *empire_id, tech_id, overflow, events,
                    );
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

            let fleet_maintenance = self.fleet_maintenance_for_empire(empire_id);

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
                let fleet_owner = self.state.fleets.get(&fleet_id).map(|f| f.owner);
                let is_ai_fleet = fleet_owner
                    .map(|owner| {
                        self.state.ai_empires.contains(&owner)
                            || Some(owner) == self.state.ai_empire
                    })
                    .unwrap_or(false);
                if is_ai_fleet {
                    self.state.ai_explored_stars.insert(destination);
                    // Symmetric contact: AI scout arriving at a player colony
                    self.check_ai_contact_at_star(
                        destination,
                        fleet_owner.unwrap_or(EmpireId(2)),
                        events,
                    );
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

        // Tick survey missions: decrement and resolve completed ones.
        // Ordered by FleetId for deterministic completion order.
        let survey_mission_ids: Vec<FleetId> = self.state.survey_missions.keys().copied().collect();
        for fleet_id in survey_mission_ids {
            let (star, planet_index, new_remaining) = {
                let mission = self.state.survey_missions.get_mut(&fleet_id).unwrap();
                mission.turns_remaining = mission.turns_remaining.saturating_sub(1);
                (mission.star, mission.planet_index, mission.turns_remaining)
            };

            if new_remaining == 0 {
                self.state.survey_missions.remove(&fleet_id);
                self.complete_survey_at_star(star, planet_index, events);
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

                // Clear a MoveToSystem fleet order only when its target matches the
                // mission that just completed.  This prevents accidentally removing a
                // standing order that was issued for a *different* destination while
                // a previous mission (scout/survey/earlier move) was already in flight.
                if matches!(
                    self.state.fleet_orders.get(&fleet_id),
                    Some(FleetOrder::MoveToSystem(s)) if *s == destination
                ) {
                    self.state.fleet_orders.remove(&fleet_id);
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

        // Discovery update: any lane whose endpoints are both explored becomes known.
        self.refresh_known_hyperspace_lanes();

        // Run AI turn decisions for all AI empires (before advancing the turn counter).
        // Prefer the explicit ai_empires list; fall back to the legacy ai_empire field
        // for saves created before the multi-empire field was added (pre-v20).
        let ai_empire_ids: Vec<EmpireId> = if !self.state.ai_empires.is_empty() {
            self.state.ai_empires.clone()
        } else if let Some(id) = self.state.ai_empire {
            vec![id]
        } else {
            vec![]
        };
        for ai_empire_id in ai_empire_ids {
            let ai_events = crate::ai::run_ai_turn(&mut self.state, ai_empire_id);
            events.extend(ai_events);
        }
        self.process_ai_diplomacy();

        let updated_supply = self.state.recompute_colony_supply();
        let tracked_colonies: BTreeSet<ColonyId> = previous_supply
            .keys()
            .chain(updated_supply.keys())
            .copied()
            .collect();
        for colony_id in tracked_colonies {
            let prev = previous_supply.get(&colony_id).copied();
            let next = updated_supply.get(&colony_id).copied();
            match (prev, next) {
                (Some(ColonySupplyState::Isolated), Some(ColonySupplyState::Connected)) => {
                    events.push(Event::ColonyReconnected { colony: colony_id });
                }
                (Some(ColonySupplyState::Connected), Some(ColonySupplyState::Isolated))
                | (None, Some(ColonySupplyState::Isolated)) => {
                    events.push(Event::ColonyIsolated { colony: colony_id });
                }
                _ => {}
            }
        }
        self.state.colony_supply = updated_supply.clone();
        self.last_turn_colony_supply = updated_supply;

        // Recompute blockade state after all fleet movements and combat have resolved.
        // Emit BlockadeStarted / BlockadeEnded transition events in deterministic
        // (ColonyId) order.
        let updated_blockade = self.state.recompute_colony_blockade();
        let previous_blockade = self.last_turn_colony_blockade.clone();
        let colonies_with_blockade_changes: BTreeSet<ColonyId> = previous_blockade
            .keys()
            .chain(updated_blockade.keys())
            .copied()
            .collect();
        for colony_id in colonies_with_blockade_changes {
            let was_blockaded = previous_blockade.contains_key(&colony_id);
            let now_blockaded = updated_blockade.get(&colony_id).copied();
            let star_id = self
                .state
                .colonies
                .get(&colony_id)
                .map(|c| c.star)
                .unwrap_or_default();
            match (was_blockaded, now_blockaded) {
                (false, Some(by_empire)) => {
                    events.push(Event::BlockadeStarted {
                        colony: colony_id,
                        star: star_id,
                        by_empire,
                    });
                }
                (true, None) => {
                    events.push(Event::BlockadeEnded {
                        colony: colony_id,
                        star: star_id,
                    });
                }
                _ => {}
            }
        }
        self.state.colony_blockade = updated_blockade.clone();
        self.last_turn_colony_blockade = updated_blockade;

        // Deterministic population growth tick (lite v1):
        // - requires available housing
        // - requires colony stability >= 90
        // - suppressed while blockaded
        // - fixed periodic cadence based on (turn + colony_id)
        for colony_id in sorted_colony_ids(&self.state.colonies) {
            let (star_id, planet_index, stability, owner) =
                match self.state.colonies.get(&colony_id) {
                    Some(c) => (c.star, c.planet_index, c.stability, c.owner),
                    None => continue,
                };
            if self.state.colony_blockade.contains_key(&colony_id)
                || stability < MIN_STABILITY_FOR_POP_GROWTH
            {
                continue;
            }
            if self
                .state
                .empires
                .get(&owner)
                .map(|e| e.food <= 0)
                .unwrap_or(true)
            {
                continue;
            }
            let planet = self
                .state
                .stars
                .get(&star_id)
                .and_then(|s| s.planets.get(planet_index));
            let Some(colony) = self.state.colonies.get(&colony_id) else {
                continue;
            };
            let y = crate::yield_model::calculate_yield(colony, planet);
            if y.workforce.housing_deficit > 0 || y.food < y.food_consumed {
                continue;
            }
            let cadence = u64::from(self.state.turn) + colony_id.0;
            if !cadence.is_multiple_of(u64::from(POP_GROWTH_PERIOD_TURNS)) {
                continue;
            }
            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                colony.population = colony.population.saturating_add(1);
                events.push(Event::PopulationGrew {
                    colony: colony_id,
                    new_population: colony.population,
                });
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

    fn process_set_colony_role(
        &mut self,
        colony_id: ColonyId,
        role: ColonyRole,
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

        // Validate owner — only the player may issue this command
        if colony.owner != self.state.player_empire {
            events.push(Event::error("Colony not owned by player"));
            return;
        }

        // Apply the new role
        if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
            colony.role = role;
        }

        events.push(Event::ColonyRoleChanged {
            colony: colony_id,
            role,
        });
    }

    fn process_set_rally_point(
        &mut self,
        colony_id: ColonyId,
        star_id: StarId,
        events: &mut Vec<Event>,
    ) {
        // Validate colony exists and is player-owned
        let colony = match self.state.colonies.get(&colony_id) {
            Some(c) => c,
            None => {
                events.push(Event::error(format!("Colony {} not found", colony_id.0)));
                return;
            }
        };
        if colony.owner != self.state.player_empire {
            events.push(Event::error("Colony not owned by player"));
            return;
        }

        // Validate target star exists
        if !self.state.stars.contains_key(&star_id) {
            events.push(Event::error(format!(
                "Rally point star {} not found",
                star_id.0
            )));
            return;
        }

        // Apply the rally point
        if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
            colony.rally_point = Some(star_id);
        }

        events.push(Event::RallyPointSet {
            colony: colony_id,
            star: star_id,
        });
    }

    fn process_clear_rally_point(&mut self, colony_id: ColonyId, events: &mut Vec<Event>) {
        // Validate colony exists and is player-owned
        let colony = match self.state.colonies.get(&colony_id) {
            Some(c) => c,
            None => {
                events.push(Event::error(format!("Colony {} not found", colony_id.0)));
                return;
            }
        };
        if colony.owner != self.state.player_empire {
            events.push(Event::error("Colony not owned by player"));
            return;
        }

        if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
            colony.rally_point = None;
        }

        events.push(Event::RallyPointCleared { colony: colony_id });
    }

    fn process_set_fleet_order(
        &mut self,
        fleet_id: FleetId,
        order: FleetOrder,
        events: &mut Vec<Event>,
    ) {
        // Validate fleet exists and is player-owned
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(f) => f,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }

        // For MoveToSystem: validate the destination and start movement if idle
        if let FleetOrder::MoveToSystem(destination) = order {
            // Destination star must exist
            if !self.state.stars.contains_key(&destination) {
                events.push(Event::error(format!(
                    "Destination star {} not found",
                    destination.0
                )));
                return;
            }
            // Destination must be explored
            if !self.state.explored_stars.contains(&destination) {
                events.push(Event::error(format!(
                    "Star {} is not explored — explore it first",
                    destination.0
                )));
                return;
            }
            // Cannot order movement to the current location
            let from = fleet.location;
            if from == destination {
                events.push(Event::error(format!(
                    "Fleet {} is already at star {}",
                    fleet_id.0, destination.0
                )));
                return;
            }
            // If the fleet is idle (no active missions), start a FleetMission immediately
            let is_idle = !self.state.scout_missions.contains_key(&fleet_id)
                && !self.state.survey_missions.contains_key(&fleet_id)
                && !self.state.fleet_missions.contains_key(&fleet_id);
            if is_idle {
                let (turns, used_lane) = travel_turns_with_lanes(
                    &self.state,
                    self.state.player_empire,
                    from,
                    destination,
                );
                self.state.fleet_missions.insert(
                    fleet_id,
                    FleetMission {
                        fleet: fleet_id,
                        destination,
                        turns_remaining: turns,
                        origin: from,
                        total_duration: turns,
                    },
                );
                events.push(Event::FleetDeparted {
                    fleet: fleet_id,
                    from,
                    to: destination,
                    turns_remaining: turns,
                });
                if used_lane {
                    events.push(Event::HyperspaceLaneUsed {
                        empire: self.state.player_empire,
                        fleet: fleet_id,
                        from,
                        to: destination,
                    });
                }
            }
        }

        // Store the fleet order
        self.state.fleet_orders.insert(fleet_id, order);
        events.push(Event::FleetOrderSet {
            fleet: fleet_id,
            order,
        });
    }

    /// Declare war on `target_empire`, setting the diplomatic relationship to `War`.
    ///
    /// Validation:
    /// - The player must have made at least first contact with the target.
    /// - The target must be a real, non-player empire.
    ///
    /// On success, the diplomacy map is updated silently (no success event is emitted).
    /// Validation errors are surfaced as `Event::Error`.
    fn process_declare_war(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        let player = self.state.player_empire;

        if target == player {
            events.push(Event::error("Cannot declare war on your own empire"));
            return;
        }

        if !self.state.empires.contains_key(&target) {
            events.push(Event::error(format!("Empire {} not found", target.0)));
            return;
        }

        let current_status = self
            .state
            .diplomacy
            .get(&target)
            .copied()
            .unwrap_or(crate::state::RelationshipStatus::Unknown);

        if current_status == crate::state::RelationshipStatus::Unknown {
            events.push(Event::error(
                "Cannot declare war on an empire you have not yet contacted",
            ));
            return;
        }

        if current_status == crate::state::RelationshipStatus::War {
            events.push(Event::error("Already at war with this empire"));
            return;
        }

        self.state
            .diplomacy
            .insert(target, crate::state::RelationshipStatus::War);
    }

    /// If `colony_id` has a rally point set, auto-route the newly created `fleet_id`
    /// (produced at `from_star`) toward that rally point.
    ///
    /// Silently skips routing when:
    /// * The rally point equals the production star (no movement required).
    /// * The rally point star has not yet been explored.
    /// * The fleet already has an active `FleetMission` (defensive guard).
    fn maybe_route_to_rally_point(
        &mut self,
        fleet_id: FleetId,
        colony_id: ColonyId,
        from_star: StarId,
        events: &mut Vec<Event>,
    ) {
        let rally_star = match self.state.colonies.get(&colony_id) {
            Some(c) => match c.rally_point {
                Some(s) => s,
                None => return,
            },
            None => return,
        };

        // No movement needed if rally is at the same star as production
        if rally_star == from_star {
            return;
        }

        // Only route to explored stars
        if !self.state.explored_stars.contains(&rally_star) {
            return;
        }

        // Rally star must still exist (defensive)
        if !self.state.stars.contains_key(&rally_star) {
            return;
        }

        // Do not overwrite an existing mission (defensive guard described in the doc comment).
        // In practice this guard is never triggered for freshly produced fleets, but it
        // protects against corrupted state on load.
        if self.state.fleet_missions.contains_key(&fleet_id) {
            return;
        }

        // Determine the empire owner of the fleet for lane calculation
        let empire_id = match self.state.fleets.get(&fleet_id) {
            Some(f) => f.owner,
            None => return,
        };

        let (turns, used_lane) =
            travel_turns_with_lanes(&self.state, empire_id, from_star, rally_star);

        self.state.fleet_missions.insert(
            fleet_id,
            FleetMission {
                fleet: fleet_id,
                destination: rally_star,
                turns_remaining: turns,
                origin: from_star,
                total_duration: turns,
            },
        );
        // Set fleet order for tracking / display
        self.state
            .fleet_orders
            .insert(fleet_id, FleetOrder::MoveToSystem(rally_star));

        events.push(Event::ShipRoutedToRallyPoint {
            fleet: fleet_id,
            colony: colony_id,
            from: from_star,
            to: rally_star,
            turns_remaining: turns,
        });
        if used_lane {
            events.push(Event::HyperspaceLaneUsed {
                empire: empire_id,
                fleet: fleet_id,
                from: from_star,
                to: rally_star,
            });
        }
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

        // Block move if fleet is on an active survey mission
        if self.state.survey_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already surveying",
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

        // Calculate travel time from distance, with direct-lane bonus when available.
        let (turns, used_lane) =
            travel_turns_with_lanes(&self.state, self.state.player_empire, from, destination);

        // Create the fleet mission
        self.state.fleet_missions.insert(
            fleet_id,
            FleetMission {
                fleet: fleet_id,
                destination,
                turns_remaining: turns,
                origin: from,
                total_duration: turns,
            },
        );

        events.push(Event::FleetDeparted {
            fleet: fleet_id,
            from,
            to: destination,
            turns_remaining: turns,
        });
        if used_lane {
            events.push(Event::HyperspaceLaneUsed {
                empire: self.state.player_empire,
                fleet: fleet_id,
                from,
                to: destination,
            });
        }
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

        if let BuildItem::Ship(design_id) = item {
            if design_id.record().is_none() {
                events.push(Event::error(format!(
                    "Cannot build ship — design {} is invalid",
                    design_id.0
                )));
                return;
            }
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
        if item.is_ship() && !colony.has_shipyard() {
            events.push(Event::error(format!(
                "Cannot build Ship {} — colony {} has no Shipyard",
                item.name(),
                colony_id.0
            )));
            return;
        }

        // Surface buildings require a free surface slot
        if matches!(
            item,
            BuildItem::SurfaceStructure(_) | BuildItem::Structure(_)
        ) {
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

        if fleet.kind != FleetKind::Scout && fleet.kind != FleetKind::FastScout {
            events.push(Event::error(format!("Fleet {} is not a scout", fleet_id.0)));
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

        // Block if fleet already has a survey mission
        if self.state.survey_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already surveying",
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

        // Calculate travel time from distance, with direct-lane bonus when available.
        let origin = fleet.location;
        let (turns, used_lane) =
            travel_turns_with_lanes(&self.state, self.state.player_empire, origin, destination);

        // Create the scout mission
        self.state.scout_missions.insert(
            fleet_id,
            ScoutMission {
                fleet: fleet_id,
                destination,
                turns_remaining: turns,
                origin,
                total_duration: turns,
            },
        );

        events.push(Event::ScoutDispatched {
            fleet: fleet_id,
            destination,
            turns_remaining: turns,
        });
        if used_lane {
            events.push(Event::HyperspaceLaneUsed {
                empire: self.state.player_empire,
                fleet: fleet_id,
                from: origin,
                to: destination,
            });
        }
    }

    fn process_survey_planet(
        &mut self,
        fleet_id: FleetId,
        star_id: StarId,
        planet_index: usize,
        events: &mut Vec<Event>,
    ) {
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(f) => f,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };

        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }

        if fleet.kind != FleetKind::Science && fleet.kind != FleetKind::SurveyCutter {
            events.push(Event::error(format!(
                "Fleet {} is not a science ship",
                fleet_id.0
            )));
            return;
        }

        if self.state.scout_missions.contains_key(&fleet_id)
            || self.state.fleet_missions.contains_key(&fleet_id)
            || self.state.survey_missions.contains_key(&fleet_id)
        {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }

        if fleet.location != star_id {
            events.push(Event::error(format!(
                "Fleet {} is not at star {}",
                fleet_id.0, star_id.0
            )));
            return;
        }

        if !self.state.explored_stars.contains(&star_id) {
            events.push(Event::error(format!("Star {} is not explored", star_id.0)));
            return;
        }

        let planet = match self
            .state
            .stars
            .get(&star_id)
            .and_then(|star| star.planets.get(planet_index))
        {
            Some(planet) => planet,
            None => {
                events.push(Event::error(format!(
                    "Orbit {} out of bounds for star {}",
                    planet_index + 1,
                    star_id.0
                )));
                return;
            }
        };

        if planet.surveyed {
            events.push(Event::error(format!(
                "Orbit {} at star {} has already been surveyed",
                planet_index + 1,
                star_id.0
            )));
            return;
        }

        self.state.survey_missions.insert(
            fleet_id,
            SurveyMission {
                fleet: fleet_id,
                star: star_id,
                planet_index,
                turns_remaining: SURVEY_TURNS,
            },
        );

        events.push(Event::SurveyStarted {
            fleet: fleet_id,
            star: star_id,
            planet_index,
            turns_remaining: SURVEY_TURNS,
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
        if fleet.kind != FleetKind::Colonizer && fleet.kind != FleetKind::ColonyArk {
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
        if self.state.survey_missions.contains_key(&fleet_id) {
            events.push(Event::error(format!(
                "Fleet {} is already surveying",
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
        let (planet_habitable, planet_colony, planet_surveyed) = {
            let star = match self.state.stars.get(&star_id) {
                Some(s) => s,
                None => {
                    events.push(Event::error(format!("Star {} not found", star_id.0)));
                    return;
                }
            };

            if planet_index >= star.planets.len() {
                events.push(Event::error(format!(
                    "Orbit {} out of bounds for star {}",
                    planet_index + 1,
                    star_id.0
                )));
                return;
            }

            let planet = &star.planets[planet_index];
            (planet.habitable, planet.colony, planet.surveyed)
        };

        // Validate planet is surveyed
        if !planet_surveyed {
            events.push(Event::error(format!(
                "Orbit {} at star {} has not been surveyed",
                planet_index + 1,
                star_id.0
            )));
            return;
        }

        // Validate planet is habitable
        if !planet_habitable {
            events.push(Event::error(format!(
                "Orbit {} at star {} is not habitable",
                planet_index + 1,
                star_id.0
            )));
            return;
        }

        // Validate planet is not already colonized
        if planet_colony.is_some() {
            events.push(Event::error(format!(
                "Orbit {} at star {} is already colonized",
                planet_index + 1,
                star_id.0
            )));
            return;
        }

        // All checks pass — create the colony
        let empire_id = self.state.player_empire;
        events.push(Event::ColonizationStarted {
            empire: empire_id,
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
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
            role: ColonyRole::Balanced,
            rally_point: None,
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

    fn process_invade(
        &mut self,
        fleet_id: FleetId,
        star_id: StarId,
        planet_index: usize,
        events: &mut Vec<Event>,
    ) {
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(f) => f.clone(),
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };

        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }
        if fleet.kind != FleetKind::TroopTransport {
            events.push(Event::error(format!(
                "Fleet {} is not a troop transport",
                fleet_id.0
            )));
            return;
        }
        if self.state.scout_missions.contains_key(&fleet_id)
            || self.state.survey_missions.contains_key(&fleet_id)
            || self.state.fleet_missions.contains_key(&fleet_id)
        {
            events.push(Event::error(format!(
                "Fleet {} is already travelling",
                fleet_id.0
            )));
            return;
        }
        if fleet.location != star_id {
            events.push(Event::error(format!(
                "Fleet {} is not at star {}",
                fleet_id.0, star_id.0
            )));
            return;
        }

        let (planet_colony, colony_owner) = {
            let Some(star) = self.state.stars.get(&star_id) else {
                events.push(Event::error(format!("Star {} not found", star_id.0)));
                return;
            };
            if planet_index >= star.planets.len() {
                events.push(Event::error(format!(
                    "Orbit {} out of bounds for star {}",
                    planet_index + 1,
                    star_id.0
                )));
                return;
            }
            let Some(colony_id) = star.planets[planet_index].colony else {
                events.push(Event::error(format!(
                    "Orbit {} at star {} is uncolonized",
                    planet_index + 1,
                    star_id.0
                )));
                return;
            };
            let Some(colony) = self.state.colonies.get(&colony_id) else {
                events.push(Event::error(format!("Colony {} not found", colony_id.0)));
                return;
            };
            (colony_id, colony.owner)
        };

        if colony_owner == self.state.player_empire {
            events.push(Event::error("Cannot invade your own colony"));
            return;
        }
        if !self
            .state
            .relationship_status(self.state.player_empire, colony_owner)
            .is_hostile_or_war()
        {
            events.push(Event::error(
                "Cannot invade — target empire is not Hostile or At War",
            ));
            return;
        }

        let hostile_orbital_defenders = self.state.fleets.iter().any(|(fid, f)| {
            *fid != fleet_id
                && f.location == star_id
                && f.owner == colony_owner
                && !self.state.fleet_missions.contains_key(fid)
                && !self.state.scout_missions.contains_key(fid)
                && !self.state.survey_missions.contains_key(fid)
        });

        let Some(target_colony) = self.state.colonies.get(&planet_colony).cloned() else {
            events.push(Event::error(format!(
                "Colony {} not found",
                planet_colony.0
            )));
            return;
        };
        let defense_strength = Self::colony_defense_strength(&target_colony);
        let invasion_strength = self.invasion_strength_for_empire(fleet.owner, fleet.ships);

        if hostile_orbital_defenders {
            events.push(Event::InvasionFailed {
                attacker: self.state.player_empire,
                defender: colony_owner,
                fleet: fleet_id,
                star: star_id,
                planet_index,
                colony: planet_colony,
                invasion_strength,
                defense_strength,
                transports_lost: 0,
                reason: "Hostile orbital defenses remain".to_string(),
            });
            return;
        }

        if invasion_strength > defense_strength {
            if let Some(colony) = self.state.colonies.get_mut(&planet_colony) {
                colony.owner = self.state.player_empire;
                colony.stability = CAPTURED_UNREST_STABILITY;
                colony.build_queue.clear();
                colony.accumulated_production = 0;
                colony.rally_point = None;
            }

            self.state.fleets.remove(&fleet_id);
            self.state.fleet_missions.remove(&fleet_id);
            self.state.scout_missions.remove(&fleet_id);
            self.state.survey_missions.remove(&fleet_id);
            self.state.fleet_orders.remove(&fleet_id);
            // Ownership and fleet changes can invalidate cached blockade state.
            // Recompute now so the next end-turn economy pass does not use stale blockade entries.
            self.state.colony_blockade = self.state.recompute_colony_blockade();

            events.push(Event::InvasionSucceeded {
                attacker: self.state.player_empire,
                defender: colony_owner,
                fleet: fleet_id,
                star: star_id,
                planet_index,
                colony: planet_colony,
                transports_lost: fleet.ships,
            });
            return;
        }

        let transports_lost = self.reduce_transport_fleet(fleet_id, 1);
        // Fleet attrition can remove the last blockading transport; keep cache in sync.
        self.state.colony_blockade = self.state.recompute_colony_blockade();
        events.push(Event::InvasionFailed {
            attacker: self.state.player_empire,
            defender: colony_owner,
            fleet: fleet_id,
            star: star_id,
            planet_index,
            colony: planet_colony,
            invasion_strength,
            defense_strength,
            transports_lost,
            reason: "Defenses held".to_string(),
        });
    }

    fn colony_defense_strength(colony: &Colony) -> u32 {
        let population_strength = (colony.population as u32).saturating_mul(5);
        let stability_strength = (colony.stability as u32) / 10;
        let surface_strength = (colony.surface_installations.len() as u32).saturating_mul(2);
        let orbital_strength = (colony.orbital_installations.len() as u32).saturating_mul(4);
        population_strength
            .saturating_add(stability_strength)
            .saturating_add(surface_strength)
            .saturating_add(orbital_strength)
    }

    fn reduce_transport_fleet(&mut self, fleet_id: FleetId, loss: u32) -> u32 {
        let mut transports_lost = 0;
        let mut should_remove = false;
        if let Some(fleet) = self.state.fleets.get_mut(&fleet_id) {
            let actual_loss = fleet.ships.min(loss);
            fleet.ships = fleet.ships.saturating_sub(actual_loss);
            transports_lost = actual_loss;
            if fleet.ships == 0 {
                should_remove = true;
            }
        }
        if should_remove {
            self.state.fleets.remove(&fleet_id);
            self.state.fleet_orders.remove(&fleet_id);
            self.state.fleet_missions.remove(&fleet_id);
            self.state.scout_missions.remove(&fleet_id);
            self.state.survey_missions.remove(&fleet_id);
        }
        transports_lost
    }

    /// Mark a specific planet as surveyed and emit a `PlanetSurveyCompleted` event.
    fn complete_survey_at_star(
        &mut self,
        star_id: StarId,
        planet_index: usize,
        events: &mut Vec<Event>,
    ) {
        if let Some(star) = self.state.stars.get_mut(&star_id) {
            if let Some(planet) = star.planets.get_mut(planet_index) {
                if !planet.surveyed {
                    planet.surveyed = true;

                    // Emit one-time Ancient Ruins discovery event (deterministic, no duplicates).
                    let has_ruins = planet
                        .specials
                        .contains(&crate::state::PlanetSpecial::AncientRuins);
                    let already_collected = planet.ancient_ruins_collected;
                    if has_ruins && !already_collected {
                        planet.ancient_ruins_collected = true;
                        events.push(Event::AncientRuinsDiscovered {
                            star: star_id,
                            planet_index,
                        });
                    }

                    events.push(Event::PlanetSurveyCompleted {
                        star: star_id,
                        planet_index,
                    });
                }
            }
        }
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
                    .insert(empire_id, self.first_contact_status_for_empire(empire_id));
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
    fn check_ai_contact_at_star(
        &mut self,
        star_id: StarId,
        ai_empire_id: EmpireId,
        events: &mut Vec<Event>,
    ) {
        let player = self.state.player_empire;
        let has_player_colony = self
            .state
            .colonies
            .values()
            .any(|c| c.star == star_id && c.owner == player);

        if !has_player_colony {
            return;
        }

        let status = self
            .state
            .diplomacy
            .get(&ai_empire_id)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown);

        if status == RelationshipStatus::Unknown {
            self.state.diplomacy.insert(
                ai_empire_id,
                self.first_contact_status_for_empire(ai_empire_id),
            );
            events.push(Event::FirstContact {
                with_empire: ai_empire_id,
            });
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
                    && !self.state.survey_missions.contains_key(*fid)
                    && is_combat_eligible(&self.state, arrived_owner, f.owner)
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

/// Returns true if `empire_a` and `empire_b` are combat-eligible against each other.
///
/// Diplomacy in v1 is stored from the player's perspective.  If neither empire
/// is the player, the function returns `false` (AI-vs-AI not applicable).
///
/// Combat is eligible for `Contacted`, `Tense`, `Hostile`, and `War` statuses.
/// `Contacted` is kept eligible for backward compatibility with v1 saves and tests.
fn is_combat_eligible(state: &GameState, empire_a: EmpireId, empire_b: EmpireId) -> bool {
    state
        .relationship_status(empire_a, empire_b)
        .is_combat_eligible()
}

#[cfg(test)]
mod tests;
