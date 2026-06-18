//! Game engine - command processing and turn execution

mod research;
mod setup;

use crate::commands::Command;
use crate::deterministic::sorted_colony_ids;
use crate::events::Event;
use crate::galaxy::{find_home_star, generate_galaxy_with_config, generate_hyperspace_lanes};
#[cfg(test)]
use crate::state::SeededRng;
use crate::state::{
    AiDoctrine, BattleReport, BuildItem, BuildingType, Colony, ColonyAutomation, ColonyId,
    ColonyRole, ColonySupplyState, ColonyUnrestState, CombatPhase, CombatPhaseSummary, ComponentId,
    CustomDesignId, CustomShipDesign, DiplomaticCommunication, DiplomaticCommunicationType,
    DiplomaticRelationship, DiplomaticResponse, DiplomaticTone, DiplomaticTreaty, Empire, EmpireId,
    EmpireIntel, EspionageMission, Fleet, FleetFormation, FleetId, FleetKind, FleetMission,
    FleetOrder, FleetRole, FleetSupplyState, GameState, HullId, HyperspaceLane, IntelLevel,
    IntelSource, OrbitalStructureType, RelationshipStatus, ResearchState, ScenarioSetup,
    ScoutMission, SectorDirective, SectorId, ShipDesignId, StarId, StrategicResource,
    SurveyMission, TechId, TradeDisruptionReason, TreatyType, UnrestCause, YieldType, all_techs,
    empire_definition_by_id, is_tech_available, tech_by_id, tech_yield_bonus_per_colony,
};
use crate::victory::evaluate_victory_end_turn;
use crate::yield_model::YieldContext;
use std::collections::{BTreeMap, BTreeSet};

// Import balance constants directly so they are available as bare names in
// this module and (via `use super::*`) in the test sub-module, preserving
// backward compatibility with all existing tests.
use crate::balance::{
    BLOCKADED_STABILITY_PENALTY, BORDER_TENSION_DISTANCE_SQ, CAPTURED_UNREST_STABILITY,
    CONQUEST_UNREST_TURNS, FLEET_TRAVEL_SPEED, HYPERSPACE_TRAVEL_DIVISOR,
    ISOLATED_STABILITY_PENALTY, ISOLATED_YIELD_PERCENT, MAX_HOUSING_DEFICIT_STABILITY_PENALTY,
    MAX_ISOLATED_FOOD_DEFICIT_STABILITY_PENALTY, MAX_UNEMPLOYMENT_STABILITY_PENALTY,
    MIN_STABILITY_FOR_POP_GROWTH, OVEREXTENSION_COLONY_THRESHOLD, POP_GROWTH_PERIOD_TURNS,
    SEVERE_BORDER_TENSION_DISTANCE_SQ, TROOP_TRANSPORT_INVASION_STRENGTH,
};

/// Number of turns for a science ship to survey a planet
pub(crate) const SURVEY_TURNS: u32 = 2;

/// Salt XOR'd with the game seed when shuffling AI empire definitions.
///
/// Using a distinct seed (game_seed ^ EMPIRE_ASSIGN_SALT) for the shuffle RNG
/// keeps the empire identity assignment independent from galaxy-generation and
/// in-game RNG streams while remaining fully deterministic for a given game seed.
const EMPIRE_ASSIGN_SALT: u64 = 0x6172_7473_5f49_5044;
const MAX_FLEET_NAME_LENGTH: usize = 32;
const EXPLORER_AVOID_ENGAGEMENT_MULTIPLIER: u32 = 2;
const AI_WARNING_FREQUENCY: u32 = 6;
const AI_TRIBUTE_DEMAND_FREQUENCY: u32 = 9;
const TRUCE_DURATION_TURNS: u32 = 8;
const NAP_DURATION_TURNS: u32 = 12;
const BATTLE_REPORT_MAX_HISTORY: usize = 40;
const ATTRITION_DIVISOR: u64 = 400;
const BATTLE_REPORT_ID_PENDING: u64 = 0;
/// 4 points = about 4 turns from contact alone, or faster with treaties / sensors.
const INTEL_THRESHOLD_BASIC: u16 = 4;
/// 9 points = mid-tier read on a rival after sustained contact or one active gather.
const INTEL_THRESHOLD_INFORMED: u16 = 9;
/// 16 points = deep read after prolonged passive tracking or repeated gathers.
const INTEL_THRESHOLD_DEEP: u16 = 16;
/// One active gather roughly equals several turns of passive contact.
const INTEL_GATHER_POINTS: u16 = 4;

#[derive(Debug, Clone, Copy, Default)]
struct YieldBonuses {
    credits: i64,
    science: i64,
    food: i64,
}

#[derive(Debug, Clone, Copy, Default)]
struct EmpireUnrestPressure {
    war_exhaustion: u8,
    overextension: u8,
}

#[derive(Debug, Clone)]
struct UnrestEvaluation {
    state: ColonyUnrestState,
    causes: Vec<UnrestCause>,
    rebellion_risk_bp: u16,
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

fn fleet_uses_logistics_penalties(kind: FleetKind) -> bool {
    kind.is_combat() || kind == FleetKind::TroopTransport
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
        RelationshipStatus::Cooperative => 3,
        RelationshipStatus::Tense => 4,
        RelationshipStatus::Hostile => 5,
        RelationshipStatus::War => 6,
    }
}

fn relationship_from_level(level: u8) -> RelationshipStatus {
    match level {
        0 => RelationshipStatus::Unknown,
        1 => RelationshipStatus::Contacted,
        2 => RelationshipStatus::Neutral,
        3 => RelationshipStatus::Cooperative,
        4 => RelationshipStatus::Tense,
        5 => RelationshipStatus::Hostile,
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
    /// Set of (from, to) trade routes that were disrupted last turn.
    /// Used for transition detection (disrupted ↔ restored event emission).
    last_turn_trade_disrupted: BTreeSet<(StarId, StarId)>,
}

/// Compute total fleet maintenance for an empire from game state.
///
/// Each fleet contributes its kind's base maintenance cost, adjusted by:
/// - the empire's `fleet_maintenance_modifier_per_fleet` (flat per-fleet delta), and
/// - the `maintenance_modifier` from the custom design used to build the fleet (if any).
pub fn fleet_maintenance_for_empire(state: &GameState, empire_id: EmpireId) -> i64 {
    let modifier = state
        .empires
        .get(&empire_id)
        .and_then(|empire| empire.empire_def)
        .and_then(empire_definition_by_id)
        .map(|def| def.military_modifiers.fleet_maintenance_modifier_per_fleet)
        .unwrap_or(0);

    let helium_discount = if state.empire_resource_count(empire_id, StrategicResource::Helium3) > 0
    {
        1
    } else {
        0
    };

    state
        .fleets
        .values()
        .filter(|f| f.owner == empire_id)
        .map(|f| {
            // For custom-designed fleets, use the design's computed maintenance cost.
            // For static ship types, fall back to the kind's base cost.
            let base = state
                .fleet_custom_designs
                .get(&f.id)
                .and_then(|did| state.custom_designs.get(did))
                .map(|d| d.derived_stats().maintenance as i64)
                .unwrap_or_else(|| f.kind.maintenance_cost() as i64);
            (base + modifier - helium_discount).max(0)
        })
        .sum()
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
            BuildItem::CustomShip(design_id) => {
                if let Some(design) = self.state.custom_designs.get(&design_id) {
                    let base_cost = design.derived_stats().production_cost;
                    // Apply empire cost modifier based on the hull's fleet role
                    let modifier_pct = match design.derived_stats().fleet_kind {
                        FleetKind::Scout | FleetKind::FastScout => {
                            def.military_modifiers.scout_cost_modifier_pct
                        }
                        FleetKind::Science | FleetKind::SurveyCutter => {
                            def.military_modifiers.science_ship_cost_modifier_pct
                        }
                        FleetKind::TroopTransport => {
                            def.military_modifiers.troop_transport_cost_modifier_pct
                        }
                        FleetKind::EscortFrigate
                        | FleetKind::MissileFrigate
                        | FleetKind::Destroyer
                        | FleetKind::PatrolCorvette => {
                            def.military_modifiers.combat_ship_cost_modifier_pct
                        }
                        _ => 0,
                    };
                    apply_cost_modifier(base_cost, modifier_pct)
                } else {
                    u64::MAX
                }
            }
            _ => item.cost(),
        }
    }

    fn first_contact_status_for_empire(&self, empire_id: EmpireId) -> RelationshipStatus {
        self.empire_definition(empire_id)
            .map(|def| def.diplomacy_profile.first_contact_status)
            .unwrap_or(RelationshipStatus::Contacted)
    }

    fn ensure_relationship_entry(&mut self, empire_id: EmpireId) {
        let status = self
            .state
            .diplomacy
            .get(&empire_id)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown);
        self.state
            .diplomacy_relationships
            .entry(empire_id)
            .or_insert_with(|| DiplomaticRelationship::from_status(status));
    }

    fn set_relationship_status(
        &mut self,
        empire_id: EmpireId,
        next: RelationshipStatus,
        events: &mut Vec<Event>,
    ) {
        let previous = self
            .state
            .relationship_status(self.state.player_empire, empire_id);
        self.state.diplomacy.insert(empire_id, next);
        self.ensure_relationship_entry(empire_id);
        if let Some(relationship) = self.state.diplomacy_relationships.get_mut(&empire_id) {
            relationship.state = next;
        }
        if previous != next {
            events.push(Event::RelationshipStateChanged {
                with_empire: empire_id,
                from: previous,
                to: next,
            });
        }
        if next == RelationshipStatus::Unknown {
            // Unknown means contact is lost or never established, so rival intel returns to hidden.
            self.state.empire_intel.remove(&empire_id);
        } else {
            self.ensure_empire_intel_contact(empire_id);
        }
    }

    fn ensure_empire_intel_contact(&mut self, empire_id: EmpireId) {
        self.state
            .empire_intel
            .entry(empire_id)
            .and_modify(|intel| {
                if intel.level == IntelLevel::Unknown {
                    intel.level = IntelLevel::Contacted;
                }
            })
            .or_insert_with(EmpireIntel::new_contacted);
    }

    fn intel_level_for_points(points: u16) -> IntelLevel {
        match points {
            p if p >= INTEL_THRESHOLD_DEEP => IntelLevel::Deep,
            p if p >= INTEL_THRESHOLD_INFORMED => IntelLevel::Informed,
            p if p >= INTEL_THRESHOLD_BASIC => IntelLevel::Basic,
            _ => IntelLevel::Contacted,
        }
    }

    fn sensor_intel_bonus(&self) -> u16 {
        let Some(player) = self.state.empires.get(&self.state.player_empire) else {
            return 0;
        };
        let completed = &player.research.completed;
        let has_tech = |tech_id| completed.contains(&tech_id);

        let mut bonus = 0;
        if has_tech(TechId::SURVEY_DRONES) || has_tech(TechId::ADVANCED_SURVEY) {
            bonus += 1;
        }
        if has_tech(TechId::SECTOR_CARTOGRAPHY) {
            bonus += 1;
        }
        if has_tech(TechId::PAN_GALACTIC_SENSOR_NET)
            || completed
                .iter()
                .filter_map(|tech_id| tech_by_id(*tech_id))
                .any(|record| {
                    record
                        .tags
                        .contains(&crate::state::TechTag::EspionageFuture)
                })
        {
            bonus += 1;
        }
        bonus
    }

    fn has_nearby_intel_presence(&self, empire_id: EmpireId) -> bool {
        let player_stars: BTreeSet<_> = self
            .state
            .fleets
            .values()
            .filter(|fleet| fleet.owner == self.state.player_empire)
            .map(|fleet| fleet.location)
            .collect();
        if player_stars.is_empty() {
            return false;
        }
        self.state
            .fleets
            .values()
            .any(|fleet| fleet.owner == empire_id && player_stars.contains(&fleet.location))
            || self
                .state
                .colonies
                .values()
                .any(|colony| colony.owner == empire_id && player_stars.contains(&colony.star))
    }

    fn refresh_empire_intelligence(&mut self, events: &mut Vec<Event>) {
        let targets: Vec<_> = self
            .state
            .diplomacy
            .iter()
            .filter(|(_, status)| **status != RelationshipStatus::Unknown)
            .map(|(empire_id, _)| *empire_id)
            .collect();
        let sensor_bonus = self.sensor_intel_bonus();

        for empire_id in targets {
            self.ensure_empire_intel_contact(empire_id);

            let mut sources = vec![IntelSource::Contact];
            let mut gained: u16 = 1;
            if self
                .state
                .relationship_data(empire_id)
                .is_some_and(|relationship| {
                    relationship
                        .active_treaties
                        .iter()
                        .any(|treaty| treaty.is_active(self.state.turn))
                })
            {
                gained += 1;
                sources.push(IntelSource::Treaty);
            }
            if self.has_nearby_intel_presence(empire_id) {
                gained += 1;
                sources.push(IntelSource::NearbyFleets);
            }
            if sensor_bonus > 0 {
                gained += sensor_bonus;
                sources.push(IntelSource::SensorTech);
            }

            let Some(intel) = self.state.empire_intel.get_mut(&empire_id) else {
                continue;
            };
            let previous_level = intel.level;
            intel.points = intel.points.saturating_add(gained);
            intel.level = Self::intel_level_for_points(intel.points);

            if intel.level > previous_level || sources.len() > 1 {
                events.push(Event::IntelGained {
                    with_empire: empire_id,
                    gained,
                    new_level: intel.level,
                    sources,
                });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn queue_diplomatic_communication(
        &mut self,
        from_empire: EmpireId,
        to_empire: EmpireId,
        communication_type: DiplomaticCommunicationType,
        tone: DiplomaticTone,
        title: String,
        body: String,
        available_responses: Vec<DiplomaticResponse>,
        expires_turn: Option<u32>,
        treaty_type: Option<TreatyType>,
        events: &mut Vec<Event>,
    ) -> u64 {
        if let Some(existing) = self
            .state
            .diplomacy_pending_communications
            .iter()
            .find(|message| {
                message.sending_empire == from_empire
                    && message.receiving_empire == to_empire
                    && message.communication_type == communication_type
                    && message.treaty_type == treaty_type
                    && message.expires_turn.is_none_or(|t| t >= self.state.turn)
            })
        {
            return existing.communication_id;
        }
        let id = self.state.diplomacy_next_communication_id.max(1);
        self.state.diplomacy_next_communication_id = id.saturating_add(1);
        self.state
            .diplomacy_pending_communications
            .push_back(DiplomaticCommunication {
                communication_id: id,
                sending_empire: from_empire,
                receiving_empire: to_empire,
                turn: self.state.turn,
                communication_type,
                tone,
                title,
                body,
                available_responses,
                expires_turn,
                treaty_type,
            });
        events.push(Event::DiplomaticCommunicationCreated {
            communication_id: id,
            from_empire,
            to_empire,
            communication_type,
        });
        id
    }

    fn communication_tone_for(
        &self,
        empire_id: EmpireId,
        status: RelationshipStatus,
        communication_type: DiplomaticCommunicationType,
    ) -> DiplomaticTone {
        let Some(def) = self.empire_definition(empire_id) else {
            return DiplomaticTone::Formal;
        };
        let imperial = def.doctrine_weight(AiDoctrine::Imperial);
        let militarist = def.doctrine_weight(AiDoctrine::Militarist);
        let merchant = def.doctrine_weight(AiDoctrine::Merchant);
        let technologist = def.doctrine_weight(AiDoctrine::Technologist);
        let explorer = def.doctrine_weight(AiDoctrine::Explorer);
        let isolationist = def.doctrine_weight(AiDoctrine::Isolationist);

        match communication_type {
            DiplomaticCommunicationType::WarDeclaration => {
                if imperial + militarist >= 14 {
                    DiplomaticTone::Triumphant
                } else {
                    DiplomaticTone::Hostile
                }
            }
            DiplomaticCommunicationType::TributeDemand => DiplomaticTone::Threatening,
            DiplomaticCommunicationType::Warning => {
                if isolationist >= 8 {
                    DiplomaticTone::Suspicious
                } else {
                    DiplomaticTone::Threatening
                }
            }
            DiplomaticCommunicationType::PeaceOffer => {
                if status == RelationshipStatus::War {
                    DiplomaticTone::Desperate
                } else {
                    DiplomaticTone::Formal
                }
            }
            DiplomaticCommunicationType::TreatyProposal => {
                if merchant >= 8 || technologist >= 8 || explorer >= 8 {
                    DiplomaticTone::Cooperative
                } else {
                    DiplomaticTone::Formal
                }
            }
            DiplomaticCommunicationType::FirstContact => {
                if imperial + militarist >= 14 {
                    DiplomaticTone::Suspicious
                } else if merchant >= 8 || technologist >= 8 || explorer >= 8 {
                    DiplomaticTone::Cooperative
                } else {
                    DiplomaticTone::Formal
                }
            }
            DiplomaticCommunicationType::TreatyAccepted => DiplomaticTone::Formal,
            DiplomaticCommunicationType::TreatyRejected => DiplomaticTone::Suspicious,
        }
    }

    fn add_treaty(
        &mut self,
        with_empire: EmpireId,
        treaty_type: TreatyType,
        duration_turns: u32,
        events: &mut Vec<Event>,
    ) {
        self.ensure_relationship_entry(with_empire);
        let start_turn = self.state.turn;
        if let Some(relationship) = self.state.diplomacy_relationships.get_mut(&with_empire) {
            relationship
                .active_treaties
                .retain(|t| !(t.treaty_type == treaty_type && t.is_active(start_turn)));
            relationship.active_treaties.push(DiplomaticTreaty {
                treaty_type,
                with_empire,
                start_turn,
                duration_turns,
            });
            relationship.last_major_diplomatic_event_turn = start_turn;
        }
        if treaty_type == TreatyType::Truce {
            self.set_relationship_status(with_empire, RelationshipStatus::Neutral, events);
        }
        let expires_turn = start_turn.saturating_add(duration_turns);
        events.push(Event::TreatySigned {
            with_empire,
            treaty_type,
            expires_turn,
        });
    }

    fn cancel_treaty(
        &mut self,
        with_empire: EmpireId,
        treaty_type: TreatyType,
        events: &mut Vec<Event>,
    ) -> bool {
        let mut cancelled = false;
        if let Some(relationship) = self.state.diplomacy_relationships.get_mut(&with_empire) {
            let before = relationship.active_treaties.len();
            relationship.active_treaties.retain(|treaty| {
                treaty.treaty_type != treaty_type || !treaty.is_active(self.state.turn)
            });
            cancelled = relationship.active_treaties.len() != before;
            if cancelled {
                relationship.last_major_diplomatic_event_turn = self.state.turn;
            }
        }
        if cancelled {
            events.push(Event::TreatyCancelled {
                with_empire,
                treaty_type,
            });
        }
        cancelled
    }

    fn process_treaty_expirations(&mut self, events: &mut Vec<Event>) {
        for (empire_id, relationship) in &mut self.state.diplomacy_relationships {
            let expired: Vec<_> = relationship
                .active_treaties
                .iter()
                .filter(|treaty| !treaty.is_active(self.state.turn))
                .map(|treaty| treaty.treaty_type)
                .collect();
            relationship
                .active_treaties
                .retain(|treaty| treaty.is_active(self.state.turn));
            for treaty_type in expired {
                events.push(Event::TreatyExpired {
                    with_empire: *empire_id,
                    treaty_type,
                });
            }
        }
        self.state
            .diplomacy_pending_communications
            .retain(|message| {
                message
                    .expires_turn
                    .is_none_or(|turn| turn >= self.state.turn)
            });
    }

    /// Compute total fleet maintenance for an empire.
    ///
    /// Each fleet contributes its kind's base maintenance cost, adjusted by
    /// the empire's `fleet_maintenance_modifier_per_fleet` (flat per-fleet delta).
    fn fleet_maintenance_for_empire(&self, empire_id: EmpireId) -> i64 {
        fleet_maintenance_for_empire(&self.state, empire_id)
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

    /// Symmetric border pressure between any two empires, used for AI↔AI
    /// relations. Unlike [`Engine::ai_border_pressure`] (which is
    /// player-perspective and one-directional), a fleet of either empire
    /// parked at the other's colony counts as severe pressure.
    fn border_pressure_between(&self, empire_a: EmpireId, empire_b: EmpireId) -> BorderPressure {
        let colony_stars = |owner: EmpireId| -> Vec<StarId> {
            self.state
                .colonies
                .values()
                .filter(|c| c.owner == owner)
                .map(|c| c.star)
                .collect()
        };
        let a_stars = colony_stars(empire_a);
        let b_stars = colony_stars(empire_b);
        if a_stars.is_empty() || b_stars.is_empty() {
            return BorderPressure::Calm;
        }

        let fleet_at_enemy_colony = self.state.fleets.values().any(|fleet| {
            (fleet.owner == empire_a && b_stars.contains(&fleet.location))
                || (fleet.owner == empire_b && a_stars.contains(&fleet.location))
        });
        if fleet_at_enemy_colony {
            return BorderPressure::Severe;
        }

        let min_sq_dist = a_stars
            .iter()
            .flat_map(|a_star| {
                b_stars.iter().filter_map(move |b_star| {
                    let src = self.state.stars.get(a_star)?;
                    let dst = self.state.stars.get(b_star)?;
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

    /// The relationship an empire's faction profile wants under a given
    /// level of border pressure.
    fn desired_status_for_pressure(
        &self,
        empire: EmpireId,
        pressure: BorderPressure,
    ) -> RelationshipStatus {
        self.empire_definition(empire)
            .map(|def| match pressure {
                BorderPressure::Calm => def.diplomacy_profile.resting_status,
                BorderPressure::Tense => def.diplomacy_profile.border_tension_status,
                BorderPressure::Severe => def.diplomacy_profile.severe_border_tension_status,
            })
            .unwrap_or(RelationshipStatus::Neutral)
    }

    /// AI invasion pass: each idle troop transport parked at a star with an
    /// enemy colony presses the assault once the orbit is clear and the
    /// numbers favour it. Invasion resolution is deterministic, so the AI
    /// only attacks when it will win — a doomed assault would just bleed
    /// transports every turn.
    ///
    /// Invasion events are surfaced to the player when they are the
    /// defender, or when they have contact with both empires; conquests
    /// between unmet empires happen silently (the map change itself becomes
    /// visible through normal exploration).
    fn process_ai_invasions(&mut self, events: &mut Vec<Event>) {
        let ai_ids: Vec<EmpireId> = if !self.state.ai_empires.is_empty() {
            self.state.ai_empires.clone()
        } else {
            self.state.ai_empire.into_iter().collect()
        };

        for ai in ai_ids {
            // Idle troop transports in deterministic (BTreeMap key) order.
            let transports: Vec<FleetId> = self
                .state
                .fleets
                .iter()
                .filter(|(fid, f)| {
                    f.owner == ai
                        && f.kind == FleetKind::TroopTransport
                        && !self.state.fleet_missions.contains_key(*fid)
                        && !self.state.scout_missions.contains_key(*fid)
                        && !self.state.survey_missions.contains_key(*fid)
                })
                .map(|(fid, _)| *fid)
                .collect();

            for fleet_id in transports {
                let Some(fleet) = self.state.fleets.get(&fleet_id).cloned() else {
                    continue;
                };
                let star_id = fleet.location;
                let Some(star) = self.state.stars.get(&star_id) else {
                    continue;
                };

                // War-opponent colonies at this star, in planet order.
                let candidates: Vec<(usize, ColonyId, EmpireId)> = star
                    .planets
                    .iter()
                    .enumerate()
                    .filter_map(|(planet_index, planet)| {
                        let colony_id = planet.colony?;
                        let colony = self.state.colonies.get(&colony_id)?;
                        (colony.owner != ai
                            && self
                                .state
                                .relationship_status(ai, colony.owner)
                                .is_hostile_or_war())
                        .then_some((planet_index, colony_id, colony.owner))
                    })
                    .collect();

                for (planet_index, colony_id, colony_owner) in candidates {
                    let Some(target_colony) = self.state.colonies.get(&colony_id).cloned() else {
                        continue;
                    };
                    let assessment =
                        self.assess_invasion(&fleet, fleet_id, star_id, &target_colony);
                    if assessment.supply_state == FleetSupplyState::OutOfSupply
                        || assessment.hostile_orbital_defenders
                        || assessment.invasion_strength <= assessment.defense_strength
                    {
                        continue;
                    }

                    let mut invasion_events = Vec::new();
                    self.execute_invasion(
                        ai,
                        fleet_id,
                        star_id,
                        planet_index,
                        colony_id,
                        colony_owner,
                        &assessment,
                        &mut invasion_events,
                    );
                    let visible = colony_owner == self.state.player_empire
                        || (self.state.player_knows_empire(ai)
                            && self.state.player_knows_empire(colony_owner));
                    if visible {
                        events.extend(invasion_events);
                    }
                    // The transport is consumed by a successful assault.
                    break;
                }
            }
        }
    }

    /// Evolve relationships between AI empire pairs.
    ///
    /// AI↔AI diplomacy is resolved without the player-facing communication
    /// machinery: each contacted pair steps one level per turn toward the
    /// more hostile of the two factions' pressure-desired statuses. Entering
    /// `War` additionally requires rough fleet parity — a heavily outmatched
    /// empire holds at `Hostile` — and existing wars end (drop to `Tense`)
    /// when either side's fleet strength collapses below half the other's.
    /// `WarDeclared` events are only emitted when the player has contact
    /// with both empires, so unknown empire names never leak into the UI.
    fn process_inter_ai_relations(&mut self, events: &mut Vec<Event>) {
        let ai_ids: Vec<EmpireId> = if !self.state.ai_empires.is_empty() {
            self.state.ai_empires.clone()
        } else {
            self.state.ai_empire.into_iter().collect()
        };
        if ai_ids.len() < 2 {
            return;
        }

        let fleet_strength = |state: &GameState, owner: EmpireId| -> u32 {
            state
                .fleets
                .values()
                .filter(|fleet| fleet.owner == owner)
                .map(|fleet| fleet.strength)
                .sum()
        };

        for i in 0..ai_ids.len() {
            for j in (i + 1)..ai_ids.len() {
                let (a, b) = (ai_ids[i], ai_ids[j]);
                let current = self.state.ai_relation(a, b);
                let pressure = self.border_pressure_between(a, b);

                // First contact happens when borders meet (or fleets visit,
                // which reads as severe pressure).
                if current == RelationshipStatus::Unknown {
                    if pressure != BorderPressure::Calm {
                        self.state
                            .set_ai_relation(a, b, RelationshipStatus::Contacted);
                    }
                    continue;
                }

                let a_strength = fleet_strength(&self.state, a);
                let b_strength = fleet_strength(&self.state, b);
                // Multiply rather than divide so a zero-strength side still
                // reads as outmatched against a strength-1 opponent.
                let outmatched = a_strength.saturating_mul(2) < b_strength
                    || b_strength.saturating_mul(2) < a_strength;

                // War exhaustion: a collapsed side sues for peace, resolved
                // directly as a truce-like de-escalation.
                if current == RelationshipStatus::War && outmatched {
                    self.state.set_ai_relation(a, b, RelationshipStatus::Tense);
                    continue;
                }

                let desired_a = self.desired_status_for_pressure(a, pressure);
                let desired_b = self.desired_status_for_pressure(b, pressure);
                // The more hostile of the two profiles drives the pair.
                let desired = if relationship_level(desired_a) >= relationship_level(desired_b) {
                    desired_a
                } else {
                    desired_b
                };

                let mut next = step_toward_relationship(current, desired, 1);
                // No new wars without rough parity: the weaker side keeps its
                // head down, which also stops declare/peace flapping.
                if next == RelationshipStatus::War && outmatched {
                    next = RelationshipStatus::Hostile;
                }
                if next == current {
                    continue;
                }

                self.state.set_ai_relation(a, b, next);

                if next == RelationshipStatus::War
                    && self.state.player_knows_empire(a)
                    && self.state.player_knows_empire(b)
                {
                    // The empire whose profile demanded war is the attacker;
                    // ties resolve to the lower id for determinism.
                    let attacker = if relationship_level(desired_b) > relationship_level(desired_a)
                    {
                        b
                    } else {
                        a
                    };
                    let defender = if attacker == a { b } else { a };
                    events.push(Event::WarDeclared { attacker, defender });
                }
            }
        }
    }

    fn process_ai_diplomacy_with_events(&mut self, events: &mut Vec<Event>) {
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
                if current == RelationshipStatus::War {
                    let ai_strength: u32 = self
                        .state
                        .fleets
                        .values()
                        .filter(|fleet| fleet.owner == ai_empire_id)
                        .map(|fleet| fleet.strength)
                        .sum();
                    let player_strength: u32 = self
                        .state
                        .fleets
                        .values()
                        .filter(|fleet| fleet.owner == player)
                        .map(|fleet| fleet.strength)
                        .sum();
                    if ai_strength < player_strength / 2
                        && !self
                            .state
                            .has_active_treaty(ai_empire_id, TreatyType::Truce)
                    {
                        self.queue_diplomatic_communication(
                            ai_empire_id,
                            player,
                            DiplomaticCommunicationType::PeaceOffer,
                            self.communication_tone_for(
                                ai_empire_id,
                                current,
                                DiplomaticCommunicationType::PeaceOffer,
                            ),
                            "Ceasefire Offer".to_string(),
                            "We propose immediate cessation of hostilities and a temporary truce."
                                .to_string(),
                            vec![DiplomaticResponse::Accept, DiplomaticResponse::Reject],
                            Some(self.state.turn.saturating_add(4)),
                            Some(TreatyType::Truce),
                            events,
                        );
                    }
                }
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
                self.set_relationship_status(ai_empire_id, next, events);
            }

            let Some(def) = self.empire_definition(ai_empire_id) else {
                continue;
            };
            let imperial = def.doctrine_weight(AiDoctrine::Imperial);
            let militarist = def.doctrine_weight(AiDoctrine::Militarist);
            let merchant = def.doctrine_weight(AiDoctrine::Merchant);
            let technologist = def.doctrine_weight(AiDoctrine::Technologist);
            let explorer = def.doctrine_weight(AiDoctrine::Explorer);
            let isolationist = def.doctrine_weight(AiDoctrine::Isolationist);

            let relation = self
                .state
                .relationship_data(ai_empire_id)
                .cloned()
                .unwrap_or_else(|| DiplomaticRelationship::from_status(current));
            let recently_significant =
                self.state.turn <= relation.last_major_diplomatic_event_turn.saturating_add(3);
            if recently_significant {
                continue;
            }

            let has_nap = self
                .state
                .has_active_treaty(ai_empire_id, TreatyType::NonAggressionPact);
            let has_truce = self
                .state
                .has_active_treaty(ai_empire_id, TreatyType::Truce);

            if !has_nap
                && !has_truce
                && (merchant >= 8 || technologist >= 8 || explorer >= 8)
                && matches!(
                    next,
                    RelationshipStatus::Neutral | RelationshipStatus::Cooperative
                )
            {
                events.push(Event::TreatyProposed {
                    from_empire: ai_empire_id,
                    to_empire: player,
                    treaty_type: TreatyType::NonAggressionPact,
                });
                self.queue_diplomatic_communication(
                    ai_empire_id,
                    player,
                    DiplomaticCommunicationType::TreatyProposal,
                    self.communication_tone_for(
                        ai_empire_id,
                        next,
                        DiplomaticCommunicationType::TreatyProposal,
                    ),
                    "Proposal: Non-Aggression Pact".to_string(),
                    "We propose a fixed-duration non-aggression pact to stabilize our shared frontier."
                        .to_string(),
                    vec![DiplomaticResponse::Accept, DiplomaticResponse::Reject],
                    Some(self.state.turn.saturating_add(5)),
                    Some(TreatyType::NonAggressionPact),
                    events,
                );
                if let Some(relationship) =
                    self.state.diplomacy_relationships.get_mut(&ai_empire_id)
                {
                    relationship.last_major_diplomatic_event_turn = self.state.turn;
                }
                continue;
            }

            if matches!(
                next,
                RelationshipStatus::Tense | RelationshipStatus::Hostile
            ) && (militarist >= 8 || imperial >= 8 || isolationist >= 9)
                && self.state.turn.is_multiple_of(AI_WARNING_FREQUENCY)
            {
                events.push(Event::WarningIssued {
                    from_empire: ai_empire_id,
                    to_empire: player,
                });
                self.queue_diplomatic_communication(
                    ai_empire_id,
                    player,
                    DiplomaticCommunicationType::Warning,
                    self.communication_tone_for(
                        ai_empire_id,
                        next,
                        DiplomaticCommunicationType::Warning,
                    ),
                    "Border Conduct Warning".to_string(),
                    "Reduce military pressure near our frontier or expect direct countermeasures."
                        .to_string(),
                    vec![DiplomaticResponse::Acknowledge],
                    Some(self.state.turn.saturating_add(4)),
                    None,
                    events,
                );
                if let Some(relationship) =
                    self.state.diplomacy_relationships.get_mut(&ai_empire_id)
                {
                    relationship.last_major_diplomatic_event_turn = self.state.turn;
                }
                continue;
            }

            if matches!(next, RelationshipStatus::Hostile)
                && imperial + militarist >= 14
                && self.state.turn.is_multiple_of(AI_TRIBUTE_DEMAND_FREQUENCY)
            {
                events.push(Event::TributeDemanded {
                    from_empire: ai_empire_id,
                    to_empire: player,
                });
                self.queue_diplomatic_communication(
                    ai_empire_id,
                    player,
                    DiplomaticCommunicationType::TributeDemand,
                    self.communication_tone_for(
                        ai_empire_id,
                        next,
                        DiplomaticCommunicationType::TributeDemand,
                    ),
                    "Tribute Demand".to_string(),
                    "Transfer tribute this cycle or prepare for escalation.".to_string(),
                    vec![DiplomaticResponse::Comply, DiplomaticResponse::Refuse],
                    Some(self.state.turn.saturating_add(3)),
                    None,
                    events,
                );
                if let Some(relationship) =
                    self.state.diplomacy_relationships.get_mut(&ai_empire_id)
                {
                    relationship.last_major_diplomatic_event_turn = self.state.turn;
                }
                continue;
            }

            if matches!(next, RelationshipStatus::Hostile | RelationshipStatus::War)
                && imperial + militarist >= 15
                && !has_nap
                && !has_truce
                && pressure == BorderPressure::Severe
            {
                self.set_relationship_status(ai_empire_id, RelationshipStatus::War, events);
                events.push(Event::WarDeclared {
                    attacker: ai_empire_id,
                    defender: player,
                });
                self.queue_diplomatic_communication(
                    ai_empire_id,
                    player,
                    DiplomaticCommunicationType::WarDeclaration,
                    self.communication_tone_for(
                        ai_empire_id,
                        RelationshipStatus::War,
                        DiplomaticCommunicationType::WarDeclaration,
                    ),
                    "Formal War Declaration".to_string(),
                    "Diplomatic relations are terminated. Military conflict is now active."
                        .to_string(),
                    vec![DiplomaticResponse::Acknowledge],
                    None,
                    None,
                    events,
                );
                if let Some(relationship) =
                    self.state.diplomacy_relationships.get_mut(&ai_empire_id)
                {
                    relationship.last_major_diplomatic_event_turn = self.state.turn;
                }
            }
        }
    }

    #[cfg(test)]
    fn process_ai_diplomacy(&mut self) {
        let mut ignored = Vec::new();
        self.process_ai_diplomacy_with_events(&mut ignored);
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
        self.state.fleet_supply = self.state.recompute_fleet_supply();
    }

    fn empire_unrest_pressure_map(&self) -> BTreeMap<EmpireId, EmpireUnrestPressure> {
        let mut colony_counts: BTreeMap<EmpireId, usize> = BTreeMap::new();
        for colony in self.state.colonies.values() {
            *colony_counts.entry(colony.owner).or_insert(0) += 1;
        }
        let player = self.state.player_empire;
        let war_with_player: BTreeMap<EmpireId, bool> = self
            .state
            .empires
            .keys()
            .copied()
            .filter(|empire_id| *empire_id != player)
            .map(|empire_id| {
                (
                    empire_id,
                    self.state.relationship_status(player, empire_id) == RelationshipStatus::War,
                )
            })
            .collect();
        let player_at_war = war_with_player.values().copied().any(|at_war| at_war);

        self.state
            .empires
            .keys()
            .copied()
            .map(|empire_id| {
                let colony_count = colony_counts.get(&empire_id).copied().unwrap_or(0);
                let overextension = colony_count.saturating_sub(OVEREXTENSION_COLONY_THRESHOLD);
                let overextension_points = overextension.min(3) as u8;
                let at_war = if empire_id == player {
                    player_at_war
                } else {
                    war_with_player.get(&empire_id).copied().unwrap_or(false)
                };
                let war_exhaustion_points = if at_war { 1 } else { 0 };
                (
                    empire_id,
                    EmpireUnrestPressure {
                        war_exhaustion: war_exhaustion_points,
                        overextension: overextension_points,
                    },
                )
            })
            .collect()
    }

    fn colony_unrest_yield_context(
        empire_food_shortage: bool,
        colony_stability: u8,
        is_connected: bool,
        is_blockaded: bool,
    ) -> YieldContext {
        YieldContext {
            food_shortage: empire_food_shortage || !is_connected || is_blockaded,
            stability_pressure: colony_stability < 85 || is_blockaded,
        }
    }

    fn unrest_state_from_score(score: u8) -> ColonyUnrestState {
        match score {
            0..=1 => ColonyUnrestState::Calm,
            2..=3 => ColonyUnrestState::Strained,
            4..=6 => ColonyUnrestState::Unrest,
            _ => ColonyUnrestState::RevoltRisk,
        }
    }

    fn evaluate_colony_unrest(
        &self,
        colony: &Colony,
        food_deficit: i64,
        housing_deficit: u64,
        is_connected: bool,
        is_blockaded: bool,
        empire_pressure: EmpireUnrestPressure,
    ) -> UnrestEvaluation {
        let mut score = 0u8;
        let mut causes = Vec::new();

        if food_deficit > 0 {
            score = score.saturating_add(2);
            causes.push(UnrestCause::FoodShortage);
        }
        if housing_deficit > 0 {
            score = score.saturating_add(2);
            causes.push(UnrestCause::HousingShortage);
        }
        let low_stability_points = if colony.stability < 55 {
            3
        } else if colony.stability < 70 {
            2
        } else if colony.stability < 85 {
            1
        } else {
            0
        };
        if low_stability_points > 0 {
            score = score.saturating_add(low_stability_points);
            causes.push(UnrestCause::LowStability);
        }
        if is_blockaded {
            score = score.saturating_add(3);
            causes.push(UnrestCause::Blockade);
        }
        if !is_connected {
            score = score.saturating_add(2);
            causes.push(UnrestCause::Isolated);
        }
        if empire_pressure.war_exhaustion > 0 {
            score = score.saturating_add(empire_pressure.war_exhaustion);
            causes.push(UnrestCause::WarExhaustion);
        }
        if empire_pressure.overextension > 0 {
            score = score.saturating_add(empire_pressure.overextension);
            causes.push(UnrestCause::Overextension);
        }

        if let Some(conquered_turn) = self
            .state
            .colony_recent_conquest_turn
            .get(&colony.id)
            .copied()
        {
            let age = self.state.turn.saturating_sub(conquered_turn);
            let conquest_points = if age <= 3 {
                3
            } else if age <= 7 {
                2
            } else if age <= CONQUEST_UNREST_TURNS {
                1
            } else {
                0
            };
            if conquest_points > 0 {
                score = score.saturating_add(conquest_points);
                causes.push(UnrestCause::RecentConquest);
            }
        }

        let state = Self::unrest_state_from_score(score);
        let rebellion_risk_bp = state
            .base_rebellion_risk_bp()
            .saturating_add(u16::from(score).saturating_mul(20));
        UnrestEvaluation {
            state,
            causes,
            rebellion_risk_bp,
        }
    }

    fn apply_unrest_penalties(
        unrest_state: ColonyUnrestState,
        industry: i64,
        credits: i64,
        research: i64,
        maintenance: i64,
    ) -> (i64, i64, i64, i64) {
        let penalized_industry = (industry * unrest_state.production_percent()) / 100;
        let penalized_credits = (credits * unrest_state.economy_percent()) / 100;
        let penalized_research = (research * unrest_state.economy_percent()) / 100;
        let penalized_maintenance = (maintenance * unrest_state.maintenance_percent()) / 100;
        (
            penalized_industry.max(0),
            penalized_credits.max(0),
            penalized_research.max(0),
            penalized_maintenance.max(0),
        )
    }

    fn refresh_unrest_statuses(&mut self) {
        let pressure_by_empire = self.empire_unrest_pressure_map();
        let mut colony_unrest = BTreeMap::new();
        let mut colony_causes = BTreeMap::new();
        let mut colony_risk = BTreeMap::new();

        for colony_id in sorted_colony_ids(&self.state.colonies) {
            let Some(colony) = self.state.colonies.get(&colony_id) else {
                continue;
            };
            let planet = self
                .state
                .stars
                .get(&colony.star)
                .and_then(|s| s.planets.get(colony.planet_index));
            let is_connected =
                self.state.colony_supply_state(colony_id) == ColonySupplyState::Connected;
            let is_blockaded = self.state.colony_blockade_state(colony_id).is_some();
            let empire_food_shortage = self
                .state
                .empires
                .get(&colony.owner)
                .map(|e| e.food < 0)
                .unwrap_or(false);
            let context = Self::colony_unrest_yield_context(
                empire_food_shortage,
                colony.stability,
                is_connected,
                is_blockaded,
            );
            let yield_snapshot =
                crate::yield_model::calculate_yield_with_context(colony, planet, context);
            let food_deficit = (yield_snapshot.food_consumed - yield_snapshot.food).max(0);
            let housing_deficit = yield_snapshot.workforce.housing_deficit;
            let unrest = self.evaluate_colony_unrest(
                colony,
                food_deficit,
                housing_deficit,
                is_connected,
                is_blockaded,
                pressure_by_empire
                    .get(&colony.owner)
                    .copied()
                    .unwrap_or_default(),
            );
            colony_unrest.insert(colony_id, unrest.state);
            colony_causes.insert(colony_id, unrest.causes);
            colony_risk.insert(colony_id, unrest.rebellion_risk_bp);
        }

        self.state.colony_unrest = colony_unrest;
        self.state.colony_unrest_causes = colony_causes;
        self.state.colony_rebellion_risk_bp = colony_risk;
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
                Command::SetSectorDirective { sector, directive } => {
                    self.process_set_sector_directive(sector, directive, &mut events);
                }
                Command::SetColonyAutomation { colony, automation } => {
                    self.process_set_colony_automation(colony, automation, &mut events);
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
                Command::SetFleetRole { fleet, role } => {
                    self.process_set_fleet_role(fleet, role, &mut events);
                }
                Command::SetFleetFormation { fleet, formation } => {
                    self.process_set_fleet_formation(fleet, formation, &mut events);
                }
                Command::RenameFleet { fleet, name } => {
                    self.process_rename_fleet(fleet, name, &mut events);
                }
                Command::DeclareWar { target } => {
                    self.process_declare_war(target, &mut events);
                }
                Command::OfferPeace { target } => {
                    self.process_offer_peace(target, &mut events);
                }
                Command::AcceptPeace { target } => {
                    self.process_accept_peace(target, &mut events);
                }
                Command::RejectPeace { target } => {
                    self.process_reject_peace(target, &mut events);
                }
                Command::ProposeNonAggressionPact { target } => {
                    self.process_propose_non_aggression(target, &mut events);
                }
                Command::AcceptNonAggressionPact { target } => {
                    self.process_accept_non_aggression(target, &mut events);
                }
                Command::RejectNonAggressionPact { target } => {
                    self.process_reject_non_aggression(target, &mut events);
                }
                Command::CancelTreaty {
                    target,
                    treaty_type,
                } => {
                    self.process_cancel_treaty(target, treaty_type, &mut events);
                }
                Command::IssueWarning { target } => {
                    self.process_issue_warning(target, &mut events);
                }
                Command::DemandTribute { target } => {
                    self.process_demand_tribute(target, &mut events);
                }
                Command::SendGreeting { target } => {
                    self.process_send_greeting(target, &mut events);
                }
                Command::RespondToCommunication {
                    communication_id,
                    response,
                } => {
                    self.process_respond_to_communication(communication_id, response, &mut events);
                }
                Command::GatherIntelligence { target } => {
                    self.process_gather_intelligence(target, &mut events);
                }
                Command::SabotageProduction { target } => {
                    self.process_placeholder_espionage(
                        target,
                        EspionageMission::SabotageProduction,
                        &mut events,
                    );
                }
                Command::StealResearch { target } => {
                    self.process_placeholder_espionage(
                        target,
                        EspionageMission::StealResearch,
                        &mut events,
                    );
                }
                Command::CreateShipDesign {
                    hull_id,
                    components,
                    name,
                } => {
                    self.create_ship_design(hull_id, components, name, &mut events);
                }
                Command::DeleteShipDesign { design_id } => {
                    self.delete_ship_design(design_id, &mut events);
                }
                Command::PlayBattleCard {
                    session_id,
                    card_index,
                } => {
                    let v3_events =
                        crate::combat_v3::play_card(&mut self.state, session_id, card_index);
                    events.extend(v3_events);
                }
                Command::RetreatFromBattle { session_id } => {
                    let v3_events = crate::combat_v3::player_retreat(&mut self.state, session_id);
                    events.extend(v3_events);
                }
            }
        }

        if !processed_end_turn {
            self.refresh_colony_supply_statuses();
            self.refresh_unrest_statuses();
            self.state.empire_resource_access = self.state.recompute_empire_resource_access();
        }

        // Auto-finalise any pending v3 battle session.  In v1 we don't
        // support pause-for-input across turns; if a player session is
        // still open at the end of a turn batch, we close it
        // deterministically using the AI's hand + the current integrity.
        let finalise_events = crate::combat_v3::finalise_pending(&mut self.state);
        events.extend(finalise_events);

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

    /// Dispatch a single command without ending the turn.  Used by the
    /// TUI for mid-turn interactions (e.g. playing battle cards while a
    /// `pending_battle_session` is active).  Returns the events produced
    /// by the command.  Does not call `process_end_turn` and does not
    /// advance the turn counter.
    pub fn dispatch_command(&mut self, command: Command) -> Vec<Event> {
        let mut events = Vec::new();
        self.dispatch_command_inner(command, &mut events);
        for event in &events {
            self.state.event_log.push(event.to_log_message());
        }
        if self.state.event_log.len() > 50 {
            let excess = self.state.event_log.len() - 50;
            self.state.event_log.drain(0..excess);
        }
        events
    }

    /// Internal: route one command to its handler.  Shared by
    /// `apply_turn` (batch path) and `dispatch_command` (single-command
    /// path).  Does not call `process_end_turn` — that stays in
    /// `apply_turn`.
    fn dispatch_command_inner(&mut self, command: Command, events: &mut Vec<Event>) {
        match command {
            Command::EndTurn => {
                self.process_end_turn(events);
            }
            Command::SetColonyFocus {
                colony,
                prod_pct,
                research_pct,
            } => {
                self.process_set_colony_focus(colony, prod_pct, research_pct, events);
            }
            Command::MoveFleet { fleet, destination } => {
                self.process_move_fleet(fleet, destination, events);
            }
            Command::QueueBuild { colony, item } => {
                self.process_queue_build(colony, item, events);
            }
            Command::CancelBuild { colony, index } => {
                self.process_cancel_build(colony, index, events);
            }
            Command::SelectResearch { tech } => {
                self.process_select_research(tech, events);
            }
            Command::QueueResearch { tech } => {
                self.process_queue_research(tech, events);
            }
            Command::RemoveQueuedResearch { tech } => {
                self.process_remove_queued_research(tech, events);
            }
            Command::MoveQueuedResearchUp { tech } => {
                self.process_move_queued_research_up(tech, events);
            }
            Command::MoveQueuedResearchDown { tech } => {
                self.process_move_queued_research_down(tech, events);
            }
            Command::ClearResearchQueue => {
                self.process_clear_research_queue(events);
            }
            Command::SendScout { fleet, destination } => {
                self.process_send_scout(fleet, destination, events);
            }
            Command::SurveyPlanet {
                fleet,
                star,
                planet_index,
            } => {
                self.process_survey_planet(fleet, star, planet_index, events);
            }
            Command::Colonize {
                fleet,
                star,
                planet_index,
            } => {
                self.process_colonize(fleet, star, planet_index, events);
            }
            Command::Invade {
                fleet,
                star,
                planet_index,
            } => {
                self.process_invade(fleet, star, planet_index, events);
            }
            Command::SetColonyRole { colony, role } => {
                self.process_set_colony_role(colony, role, events);
            }
            Command::SetSectorDirective { sector, directive } => {
                self.process_set_sector_directive(sector, directive, events);
            }
            Command::SetColonyAutomation { colony, automation } => {
                self.process_set_colony_automation(colony, automation, events);
            }
            Command::SetRallyPoint { colony, star } => {
                self.process_set_rally_point(colony, star, events);
            }
            Command::ClearRallyPoint { colony } => {
                self.process_clear_rally_point(colony, events);
            }
            Command::SetFleetOrder { fleet, order } => {
                self.process_set_fleet_order(fleet, order, events);
            }
            Command::SetFleetRole { fleet, role } => {
                self.process_set_fleet_role(fleet, role, events);
            }
            Command::SetFleetFormation { fleet, formation } => {
                self.process_set_fleet_formation(fleet, formation, events);
            }
            Command::RenameFleet { fleet, name } => {
                self.process_rename_fleet(fleet, name, events);
            }
            Command::DeclareWar { target } => {
                self.process_declare_war(target, events);
            }
            Command::OfferPeace { target } => {
                self.process_offer_peace(target, events);
            }
            Command::AcceptPeace { target } => {
                self.process_accept_peace(target, events);
            }
            Command::RejectPeace { target } => {
                self.process_reject_peace(target, events);
            }
            Command::ProposeNonAggressionPact { target } => {
                self.process_propose_non_aggression(target, events);
            }
            Command::AcceptNonAggressionPact { target } => {
                self.process_accept_non_aggression(target, events);
            }
            Command::RejectNonAggressionPact { target } => {
                self.process_reject_non_aggression(target, events);
            }
            Command::CancelTreaty {
                target,
                treaty_type,
            } => {
                self.process_cancel_treaty(target, treaty_type, events);
            }
            Command::IssueWarning { target } => {
                self.process_issue_warning(target, events);
            }
            Command::DemandTribute { target } => {
                self.process_demand_tribute(target, events);
            }
            Command::SendGreeting { target } => {
                self.process_send_greeting(target, events);
            }
            Command::RespondToCommunication {
                communication_id,
                response,
            } => {
                self.process_respond_to_communication(communication_id, response, events);
            }
            Command::GatherIntelligence { target } => {
                self.process_gather_intelligence(target, events);
            }
            Command::SabotageProduction { target } => {
                self.process_placeholder_espionage(
                    target,
                    EspionageMission::SabotageProduction,
                    events,
                );
            }
            Command::StealResearch { target } => {
                self.process_placeholder_espionage(target, EspionageMission::StealResearch, events);
            }
            Command::CreateShipDesign {
                hull_id,
                components,
                name,
            } => {
                self.create_ship_design(hull_id, components, name, events);
            }
            Command::DeleteShipDesign { design_id } => {
                self.delete_ship_design(design_id, events);
            }
            Command::PlayBattleCard {
                session_id,
                card_index,
            } => {
                let v3_events =
                    crate::combat_v3::play_card(&mut self.state, session_id, card_index);
                events.extend(v3_events);
            }
            Command::RetreatFromBattle { session_id } => {
                let v3_events = crate::combat_v3::player_retreat(&mut self.state, session_id);
                events.extend(v3_events);
            }
        }
    }

    fn process_end_turn(&mut self, events: &mut Vec<Event>) {
        // Process colonies in deterministic order
        let colony_ids = sorted_colony_ids(&self.state.colonies);
        // Rebuilt from scratch each turn so stale colonies never linger.
        self.state.last_colony_yields.clear();
        let previous_supply = self.last_turn_colony_supply.clone();
        self.refresh_colony_supply_statuses();
        let current_turn_supply = self.state.colony_supply.clone();
        // Blockade state from last turn (persisted in GameState): use for economy penalties.
        let current_turn_blockade = self.state.colony_blockade.clone();
        // Strategic resource access snapshot for this turn's economy pass.
        self.state.empire_resource_access = self.state.recompute_empire_resource_access();
        self.refresh_empire_intelligence(events);

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
        let empire_unrest_pressure = self.empire_unrest_pressure_map();

        for colony_id in colony_ids {
            // Sector-guided automation may queue a build before the queue is read,
            // so a freshly-queued item can begin construction this same turn.
            self.run_colony_automation(colony_id, events);

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
            let context = Self::colony_unrest_yield_context(
                empire_food_shortage,
                colony_stability,
                is_connected,
                is_blockaded,
            );

            // Calculate yield via the pop/jobs model.
            let colony_yield = {
                let colony = self.state.colonies.get(&colony_id).unwrap();
                crate::yield_model::calculate_yield_with_context(colony, planet.as_ref(), context)
            };

            let bonuses = empire_tech_yield_bonus_per_colony
                .get(&owner)
                .copied()
                .unwrap_or_default();
            let resource_bonuses = self.state.strategic_resource_bonuses_for_empire(owner);

            // Apply empire identity trait modifiers on top of tech bonuses.
            let empire_def_mods = self
                .state
                .empires
                .get(&owner)
                .and_then(|e| e.empire_def)
                .and_then(crate::state::empire_definition_by_id)
                .map(|d| d.trait_modifiers)
                .unwrap_or_default();

            let mut credits = colony_yield.credits
                + bonuses.credits
                + empire_def_mods.credits_per_colony
                + resource_bonuses.credits_per_colony;
            let mut research = colony_yield.science
                + bonuses.science
                + empire_def_mods.science_per_colony
                + resource_bonuses.science_per_colony;
            let mut food = colony_yield.food
                + bonuses.food
                + empire_def_mods.food_per_colony
                + resource_bonuses.food_per_colony;
            // Industry modifier from empire def is already informational here; the
            // yield model computed the base industry.  We expose the bonus via
            // the ColonyProduced event so the UI can show "empire bonus" detail.
            let mut industry = colony_yield.industry
                + empire_def_mods.industry_per_colony
                + resource_bonuses.industry_per_colony;
            // Build queue throughput intentionally starts from colony-local industry;
            // unrest is applied later in this loop, but empire/resource flat bonuses are
            // economic modifiers and do not accelerate build-pool accumulation.
            let mut build_industry = colony_yield.industry;
            let mut colony_maintenance = colony_yield.maintenance;
            if resource_bonuses.research_percent_bonus > 0 {
                // Intentional compounding: percent applies after flat colony modifiers.
                research += research * resource_bonuses.research_percent_bonus / 100;
            }
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
            if pressure_penalty > 0
                && let Some(colony) = self.state.colonies.get_mut(&colony_id)
            {
                colony.stability = colony.stability.saturating_sub(pressure_penalty);
            }

            let unrest_eval = self
                .state
                .colonies
                .get(&colony_id)
                .map(|colony| {
                    self.evaluate_colony_unrest(
                        colony,
                        food_deficit,
                        housing_deficit,
                        is_connected,
                        is_blockaded,
                        empire_unrest_pressure
                            .get(&owner)
                            .copied()
                            .unwrap_or_default(),
                    )
                })
                .unwrap_or(UnrestEvaluation {
                    state: ColonyUnrestState::Calm,
                    causes: Vec::new(),
                    rebellion_risk_bp: 0,
                });
            let previous_unrest = self.state.colony_unrest_state(colony_id);
            if previous_unrest != unrest_eval.state {
                events.push(Event::ColonyUnrestChanged {
                    colony: colony_id,
                    from: previous_unrest,
                    to: unrest_eval.state,
                    causes: unrest_eval.causes.clone(),
                    rebellion_risk_bp: unrest_eval.rebellion_risk_bp,
                });
            }
            self.state
                .colony_unrest
                .insert(colony_id, unrest_eval.state);
            self.state
                .colony_unrest_causes
                .insert(colony_id, unrest_eval.causes.clone());
            self.state
                .colony_rebellion_risk_bp
                .insert(colony_id, unrest_eval.rebellion_risk_bp);
            if self
                .state
                .colony_recent_conquest_turn
                .get(&colony_id)
                .is_some_and(|turn| self.state.turn.saturating_sub(*turn) > CONQUEST_UNREST_TURNS)
            {
                self.state.colony_recent_conquest_turn.remove(&colony_id);
            }
            (industry, credits, research, colony_maintenance) = Self::apply_unrest_penalties(
                unrest_eval.state,
                industry,
                credits,
                research,
                colony_maintenance,
            );
            build_industry = (build_industry * unrest_eval.state.production_percent()) / 100;

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
            *empire_colony_maintenance.entry(owner).or_insert(0) += colony_maintenance;

            events.push(Event::ColonyProduced {
                colony: colony_id,
                credits,
                research,
                food,
                industry,
                maintenance: colony_maintenance,
            });

            self.state.last_colony_yields.insert(
                colony_id,
                crate::state::ColonyYieldSnapshot {
                    industry,
                    credits,
                    science: research,
                    food,
                    food_consumed: colony_yield.food_consumed,
                    maintenance: colony_maintenance,
                },
            );

            // Process production queue — one active item at a time, with deterministic
            // overflow carry into subsequent queued items in the same turn.
            if let Some(item) = build_queue_front {
                let ship_bonus = if item.is_ship() {
                    colony_role.ship_production_bonus()
                } else {
                    0
                };
                // Use the yield-model industry as effective production so that
                // output scales with population and FabricationYard buildings.
                // We take the max with the raw `colony.production` field so that
                // tests which set `production = 999` for instant builds remain
                // unaffected.
                let effective_production = (build_industry.max(1) as u64).max(production);
                let mut production_pool = accumulated + effective_production + ship_bonus;

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
                if n_completed > 0
                    && let Some(colony) = self.state.colonies.get_mut(&colony_id)
                {
                    colony.build_queue.drain(..n_completed);
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
                                self.initialize_fleet_posture(
                                    fleet_id,
                                    owner,
                                    design.fleet_kind,
                                    star_id,
                                    events,
                                );
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
                                self.initialize_fleet_posture(
                                    fleet_id,
                                    owner,
                                    design.fleet_kind,
                                    star_id,
                                    events,
                                );
                                self.maybe_route_to_rally_point(
                                    fleet_id, colony_id, star_id, events,
                                );
                            }
                        }
                        BuildItem::CustomShip(design_id) => {
                            // Clone the stats we need before mutating state
                            let (fleet_kind, strength, integrity) =
                                if let Some(design) = self.state.custom_designs.get(&design_id) {
                                    let stats = design.derived_stats();
                                    (stats.fleet_kind, stats.attack.max(1), stats.hp.max(1))
                                } else {
                                    events.push(Event::error(format!(
                                        "Custom design {} not found at completion",
                                        design_id.0
                                    )));
                                    (FleetKind::Scout, 1, 100)
                                };
                            let fleet_id = self.state.next_fleet_id();
                            self.state.fleets.insert(
                                fleet_id,
                                Fleet {
                                    id: fleet_id,
                                    owner,
                                    location: star_id,
                                    ships: 1,
                                    kind: fleet_kind,
                                    strength,
                                    integrity,
                                },
                            );
                            // Record the design association so maintenance and defense
                            // modifiers from component choices remain in effect.
                            self.state.fleet_custom_designs.insert(fleet_id, design_id);
                            events.push(Event::FleetCreated {
                                fleet: fleet_id,
                                location: star_id,
                            });
                            self.initialize_fleet_posture(
                                fleet_id, owner, fleet_kind, star_id, events,
                            );
                            events.push(Event::CustomShipConstructed {
                                empire: owner,
                                colony: colony_id,
                                design_id,
                                fleet: fleet_id,
                            });
                            self.maybe_route_to_rally_point(fleet_id, colony_id, star_id, events);
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

        let previous_trade_disrupted = &self.last_turn_trade_disrupted;
        let (trade_routes, trade_income) = self.state.recompute_empire_trade_routes();
        let current_trade_disrupted: BTreeSet<(StarId, StarId)> = trade_routes
            .values()
            .flat_map(|routes| {
                routes
                    .iter()
                    .filter(|r| r.disrupted)
                    .map(|r| (r.from, r.to))
            })
            .collect();
        for (empire_id, routes) in &trade_routes {
            let total_value: i64 = routes.iter().map(|r| r.net_value).sum();
            let disrupted_count = routes.iter().filter(|r| r.disrupted).count();
            events.push(Event::TradeRoutesComputed {
                empire: *empire_id,
                route_count: routes.len(),
                total_value,
                disrupted_count,
            });
            for route in routes {
                if route.disrupted && !previous_trade_disrupted.contains(&(route.from, route.to)) {
                    events.push(Event::TradeRouteDisrupted {
                        empire: *empire_id,
                        from: route.from,
                        to: route.to,
                        reason: route
                            .disruption_reason
                            .unwrap_or(TradeDisruptionReason::WarZone),
                    });
                } else if !route.disrupted
                    && previous_trade_disrupted.contains(&(route.from, route.to))
                {
                    events.push(Event::TradeRouteRestored {
                        empire: *empire_id,
                        from: route.from,
                        to: route.to,
                    });
                }
            }
        }
        self.state.empire_trade_routes = trade_routes;
        self.last_turn_trade_disrupted = current_trade_disrupted;

        for (empire_id, income) in &trade_income {
            if *income > 0 {
                if let Some(empire) = self.state.empires.get_mut(empire_id) {
                    empire.credits += income;
                }
                *empire_credits_income.entry(*empire_id).or_insert(0) += income;
            }
        }
        self.state.empire_trade_income = trade_income;

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
                    let owner = fleet_owner.unwrap_or(EmpireId(2));
                    self.state.mark_star_explored(owner, destination);
                    // Symmetric contact: AI scout arriving at a player colony
                    self.check_ai_contact_at_star(destination, owner, events);
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
                let surveying_empire = self.state.fleets.get(&fleet_id).map(|f| f.owner);
                self.state.survey_missions.remove(&fleet_id);
                self.complete_survey_at_star(star, planet_index, surveying_empire, events);
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
        self.process_ai_invasions(events);
        self.process_ai_diplomacy_with_events(events);
        self.process_inter_ai_relations(events);
        self.process_treaty_expirations(events);

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
                    if let Some((fleet_id, _)) = self
                        .state
                        .fleets
                        .iter()
                        .filter(|(fid, fleet)| {
                            fleet.location == star_id
                                && fleet.owner == by_empire
                                && !self.state.fleet_missions.contains_key(*fid)
                                && !self.state.scout_missions.contains_key(*fid)
                                && !self.state.survey_missions.contains_key(*fid)
                        })
                        .min_by_key(|(fid, _)| *fid)
                    {
                        events.push(Event::BlockadeFleetEstablished {
                            fleet: *fleet_id,
                            colony: colony_id,
                            star: star_id,
                            by_empire,
                        });
                    }
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
        self.state.fleet_supply = self.state.recompute_fleet_supply();
        self.refresh_unrest_statuses();
        self.state.empire_resource_access = self.state.recompute_empire_resource_access();

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

        let completed_turn = self.state.turn;
        events.extend(evaluate_victory_end_turn(&mut self.state, completed_turn));

        // Generate Galactic Dispatch if this is a cadence turn or urgent events exist
        if let Some(dispatch) =
            crate::dispatch::generate_dispatch(completed_turn, events, &self.state)
        {
            self.state.galactic_dispatches.push_back(dispatch);
            while self.state.galactic_dispatches.len() > crate::dispatch::DISPATCH_MAX_HISTORY {
                self.state.galactic_dispatches.pop_front();
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

    fn process_set_sector_directive(
        &mut self,
        sector_id: SectorId,
        directive: SectorDirective,
        events: &mut Vec<Event>,
    ) {
        // Validate sector exists
        if !self.state.sectors.contains_key(&sector_id) {
            events.push(Event::error(format!("Sector {} not found", sector_id.0)));
            return;
        }

        self.state.sector_directives.insert(sector_id, directive);
        events.push(Event::SectorDirectiveSet {
            sector: sector_id,
            directive,
        });
    }

    fn process_set_colony_automation(
        &mut self,
        colony_id: ColonyId,
        automation: ColonyAutomation,
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

        self.state.colony_automation.insert(colony_id, automation);
        events.push(Event::ColonyAutomationModeSet {
            colony: colony_id,
            automation,
        });
    }

    /// Run sector-guided automation for a single colony at end of turn.
    ///
    /// Conservative by design: only player-owned, `SectorGuided` colonies with an
    /// empty build queue are touched.  An explicit player queue is never
    /// overridden.  When a build is queued, a `ColonyAutomationQueued` event is
    /// emitted so the action is always explainable.
    fn run_colony_automation(&mut self, colony_id: ColonyId, events: &mut Vec<Event>) {
        let colony = match self.state.colonies.get(&colony_id) {
            Some(c) => c,
            None => return,
        };
        if colony.owner != self.state.player_empire {
            return;
        }
        if !colony.build_queue.is_empty() {
            return;
        }
        if self.state.colony_automation_mode(colony_id) != ColonyAutomation::SectorGuided {
            return;
        }

        let directive = self
            .state
            .colony_sector(colony_id)
            .map(|sector| self.state.sector_directive(sector))
            .unwrap_or_default();

        if let Some(item) = self.pick_automation_build(colony_id, directive) {
            if let Some(colony) = self.state.colonies.get_mut(&colony_id) {
                colony.build_queue.push(item);
            }
            events.push(Event::ColonyAutomationQueued {
                colony: colony_id,
                item,
                directive,
            });
        }
    }

    /// Deterministically choose a single build item for an automated colony,
    /// biased by its sector directive.  Returns `None` when no slot/tech allows
    /// any useful building this turn.
    ///
    /// Each directive prefers one building, then falls through to a generic
    /// infrastructure order (Fabrication Yard → Science Nexus → Aquaculture Bay)
    /// so automation keeps making progress once its preferred building exists.
    fn pick_automation_build(
        &self,
        colony_id: ColonyId,
        directive: SectorDirective,
    ) -> Option<BuildItem> {
        let colony = self.state.colonies.get(&colony_id)?;
        let planet_size = self
            .state
            .stars
            .get(&colony.star)
            .and_then(|s| s.planets.get(colony.planet_index))
            .map(|p| p.size);
        let can_surface = planet_size.is_some_and(|size| colony.can_place_surface_building(size));
        let can_orbital =
            planet_size.is_some_and(|size| colony.can_place_orbital_installation(size));
        let owner = colony.owner;
        let has_tech = |tech: TechId| -> bool {
            self.state
                .empires
                .get(&owner)
                .is_some_and(|e| e.research.completed.contains(&tech))
        };
        let surface_missing = |bt: BuildingType| can_surface && !colony.buildings.contains(&bt);
        let generic = || {
            if surface_missing(BuildingType::FabricationYard) {
                Some(BuildItem::SurfaceStructure(BuildingType::FabricationYard))
            } else if surface_missing(BuildingType::ScienceNexus) {
                Some(BuildItem::SurfaceStructure(BuildingType::ScienceNexus))
            } else if surface_missing(BuildingType::AquacultureBay) {
                Some(BuildItem::SurfaceStructure(BuildingType::AquacultureBay))
            } else {
                None
            }
        };

        match directive {
            SectorDirective::Industrial => {
                if surface_missing(BuildingType::FabricationYard) {
                    return Some(BuildItem::SurfaceStructure(BuildingType::FabricationYard));
                }
            }
            SectorDirective::Research => {
                if surface_missing(BuildingType::ScienceNexus) {
                    return Some(BuildItem::SurfaceStructure(BuildingType::ScienceNexus));
                }
            }
            SectorDirective::Agricultural | SectorDirective::Stabilization => {
                if surface_missing(BuildingType::AquacultureBay) {
                    return Some(BuildItem::SurfaceStructure(BuildingType::AquacultureBay));
                }
            }
            SectorDirective::Military => {
                if !colony.has_shipyard() && can_orbital && has_tech(TechId::ORBITAL_ENGINEERING) {
                    return Some(BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard));
                }
            }
            SectorDirective::Economic => {
                if surface_missing(BuildingType::FabricationYard) {
                    return Some(BuildItem::SurfaceStructure(BuildingType::FabricationYard));
                }
                if !colony.has_supply_hub() && can_orbital && has_tech(TechId::RAPID_TRANSIT) {
                    return Some(BuildItem::OrbitalStructure(OrbitalStructureType::SupplyHub));
                }
            }
            SectorDirective::Balanced => {}
        }

        generic()
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
                let (turns, used_lane) = self.travel_turns_for_fleet(
                    fleet_id,
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

    fn process_set_fleet_role(
        &mut self,
        fleet_id: FleetId,
        role: FleetRole,
        events: &mut Vec<Event>,
    ) {
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(fleet) => fleet,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }
        self.state.fleet_roles.insert(fleet_id, role);
        events.push(Event::FleetRoleChanged {
            fleet: fleet_id,
            role,
        });
    }

    fn process_set_fleet_formation(
        &mut self,
        fleet_id: FleetId,
        formation: FleetFormation,
        events: &mut Vec<Event>,
    ) {
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(fleet) => fleet,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }
        self.state.fleet_formations.insert(fleet_id, formation);
        events.push(Event::FleetFormationChanged {
            fleet: fleet_id,
            formation,
        });
    }

    fn process_rename_fleet(&mut self, fleet_id: FleetId, name: String, events: &mut Vec<Event>) {
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(fleet) => fleet,
            None => {
                events.push(Event::error(format!("Fleet {} not found", fleet_id.0)));
                return;
            }
        };
        if fleet.owner != self.state.player_empire {
            events.push(Event::error("Fleet not owned by player"));
            return;
        }

        let trimmed = name.trim();
        if trimmed.is_empty() {
            events.push(Event::error("Fleet name cannot be empty"));
            return;
        }
        if trimmed.len() > MAX_FLEET_NAME_LENGTH {
            events.push(Event::error(format!(
                "Fleet name too long (max {} chars)",
                MAX_FLEET_NAME_LENGTH
            )));
            return;
        }
        self.state.fleet_names.insert(fleet_id, trimmed.to_string());
        events.push(Event::FleetRenamed {
            fleet: fleet_id,
            name: trimmed.to_string(),
        });
    }

    fn default_fleet_formation_for_owner(
        &self,
        owner: EmpireId,
        role: FleetRole,
        fleet_kind: FleetKind,
    ) -> FleetFormation {
        if let Some(def) = self
            .state
            .empires
            .get(&owner)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
        {
            if role == FleetRole::InvasionFleet {
                return FleetFormation::EscortScreen;
            }
            if role == FleetRole::ExplorationFleet || role == FleetRole::RapidResponseFleet {
                return FleetFormation::FastAttack;
            }
            let aggression = def.doctrine_weight(AiDoctrine::Militarist)
                + def.doctrine_weight(AiDoctrine::Imperial);
            let caution = def.doctrine_weight(AiDoctrine::Isolationist)
                + def.doctrine_weight(AiDoctrine::Merchant);
            if aggression >= caution.saturating_add(4) {
                if matches!(fleet_kind, FleetKind::MissileFrigate) {
                    return FleetFormation::Artillery;
                }
                return FleetFormation::Aggressive;
            }
            if caution >= aggression.saturating_add(4) {
                return FleetFormation::Defensive;
            }
        }
        FleetFormation::Balanced
    }

    fn initialize_fleet_posture(
        &mut self,
        fleet_id: FleetId,
        owner: EmpireId,
        fleet_kind: FleetKind,
        at_star: StarId,
        events: &mut Vec<Event>,
    ) {
        let role = FleetRole::default_for_kind(fleet_kind);
        let formation = self.default_fleet_formation_for_owner(owner, role, fleet_kind);
        self.state.fleet_roles.insert(fleet_id, role);
        self.state.fleet_formations.insert(fleet_id, formation);
        self.state
            .fleet_names
            .entry(fleet_id)
            .or_insert_with(|| format!("{} {}", fleet_kind.label(), fleet_id.0));
        events.push(Event::FleetRoleChanged {
            fleet: fleet_id,
            role,
        });
        events.push(Event::FleetFormationChanged {
            fleet: fleet_id,
            formation,
        });

        if role == FleetRole::InvasionFleet {
            events.push(Event::InvasionFleetAssembled {
                empire: owner,
                fleet: fleet_id,
                star: at_star,
            });
        }
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

        if self
            .state
            .has_active_treaty(target, TreatyType::NonAggressionPact)
        {
            events.push(Event::error(
                "Cannot declare war while a Non-Aggression Pact is active",
            ));
            return;
        }
        if self.state.has_active_treaty(target, TreatyType::Truce) {
            events.push(Event::error("Cannot declare war while a truce is active"));
            return;
        }

        self.set_relationship_status(target, crate::state::RelationshipStatus::War, events);
        events.push(Event::WarDeclared {
            attacker: player,
            defender: target,
        });
        self.queue_diplomatic_communication(
            player,
            target,
            DiplomaticCommunicationType::WarDeclaration,
            DiplomaticTone::Hostile,
            "War Declaration".to_string(),
            "Your hostile posture is unacceptable. We are now in formal war.".to_string(),
            vec![DiplomaticResponse::Acknowledge],
            None,
            None,
            events,
        );
    }

    fn process_offer_peace(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        let status = self
            .state
            .relationship_status(self.state.player_empire, target);
        if status != RelationshipStatus::War {
            events.push(Event::error("Peace can only be offered during war"));
            return;
        }
        self.queue_diplomatic_communication(
            self.state.player_empire,
            target,
            DiplomaticCommunicationType::PeaceOffer,
            DiplomaticTone::Formal,
            "Peace Offer".to_string(),
            "We offer an immediate ceasefire and temporary truce.".to_string(),
            vec![DiplomaticResponse::Accept, DiplomaticResponse::Reject],
            Some(self.state.turn.saturating_add(4)),
            Some(TreatyType::Truce),
            events,
        );
    }

    fn validate_known_foreign_empire(
        &self,
        target: EmpireId,
        events: &mut Vec<Event>,
    ) -> Option<RelationshipStatus> {
        let player = self.state.player_empire;
        if target == player {
            events.push(Event::error("Invalid target empire"));
            return None;
        }
        if !self.state.empires.contains_key(&target) {
            events.push(Event::error(format!("Empire {} not found", target.0)));
            return None;
        }
        let status = self.state.relationship_status(player, target);
        if status == RelationshipStatus::Unknown {
            events.push(Event::error(
                "Cannot accept diplomatic terms before first contact",
            ));
            return None;
        }
        Some(status)
    }

    fn has_pending_communication(
        &self,
        sender: EmpireId,
        receiver: EmpireId,
        communication_type: DiplomaticCommunicationType,
        treaty_type: Option<TreatyType>,
    ) -> bool {
        self.state
            .diplomacy_pending_communications
            .iter()
            .any(|message| {
                message.sending_empire == sender
                    && message.receiving_empire == receiver
                    && message.communication_type == communication_type
                    && message.treaty_type == treaty_type
                    && message
                        .available_responses
                        .contains(&DiplomaticResponse::Accept)
            })
    }

    fn apply_accept_peace(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        self.set_relationship_status(target, RelationshipStatus::Neutral, events);
        self.add_treaty(target, TreatyType::Truce, TRUCE_DURATION_TURNS, events);
        events.push(Event::PeaceSigned {
            with_empire: target,
            truce_expires_turn: self.state.turn.saturating_add(TRUCE_DURATION_TURNS),
        });
    }

    fn try_accept_peace(
        &mut self,
        target: EmpireId,
        require_pending_offer: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(status) = self.validate_known_foreign_empire(target, events) else {
            return;
        };
        if status != RelationshipStatus::War {
            events.push(Event::error("Peace can only be accepted while at war"));
            return;
        }
        if require_pending_offer
            && !self.has_pending_communication(
                target,
                self.state.player_empire,
                DiplomaticCommunicationType::PeaceOffer,
                Some(TreatyType::Truce),
            )
        {
            events.push(Event::error("No pending peace offer from this empire"));
            return;
        }
        self.apply_accept_peace(target, events);
    }

    fn process_accept_peace(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        self.try_accept_peace(target, true, events);
    }

    fn process_reject_peace(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        events.push(Event::TreatyRejected {
            from_empire: target,
            to_empire: self.state.player_empire,
            treaty_type: TreatyType::Truce,
        });
    }

    fn process_propose_non_aggression(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        if self
            .state
            .relationship_status(self.state.player_empire, target)
            == RelationshipStatus::War
        {
            events.push(Event::error(
                "Cannot propose non-aggression pact while at war",
            ));
            return;
        }
        events.push(Event::TreatyProposed {
            from_empire: self.state.player_empire,
            to_empire: target,
            treaty_type: TreatyType::NonAggressionPact,
        });
        self.queue_diplomatic_communication(
            self.state.player_empire,
            target,
            DiplomaticCommunicationType::TreatyProposal,
            DiplomaticTone::Formal,
            "Proposal: Non-Aggression Pact".to_string(),
            "We propose a non-aggression pact to avoid frontier conflict for a fixed term."
                .to_string(),
            vec![DiplomaticResponse::Accept, DiplomaticResponse::Reject],
            Some(self.state.turn.saturating_add(5)),
            Some(TreatyType::NonAggressionPact),
            events,
        );
    }

    fn try_accept_non_aggression(
        &mut self,
        target: EmpireId,
        require_pending_proposal: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(status) = self.validate_known_foreign_empire(target, events) else {
            return;
        };
        if status == RelationshipStatus::War {
            events.push(Event::error(
                "Cannot accept non-aggression pact while at war",
            ));
            return;
        }
        if self
            .state
            .has_active_treaty(target, TreatyType::NonAggressionPact)
        {
            events.push(Event::error(
                "Non-aggression pact already active with this empire",
            ));
            return;
        }
        if self.state.has_active_treaty(target, TreatyType::Truce) {
            events.push(Event::error("Cannot accept pact while truce is active"));
            return;
        }
        if require_pending_proposal
            && !self.has_pending_communication(
                target,
                self.state.player_empire,
                DiplomaticCommunicationType::TreatyProposal,
                Some(TreatyType::NonAggressionPact),
            )
        {
            events.push(Event::error(
                "No pending non-aggression proposal from this empire",
            ));
            return;
        }
        events.push(Event::TreatyAccepted {
            from_empire: target,
            to_empire: self.state.player_empire,
            treaty_type: TreatyType::NonAggressionPact,
        });
        self.add_treaty(
            target,
            TreatyType::NonAggressionPact,
            NAP_DURATION_TURNS,
            events,
        );
    }

    fn process_accept_non_aggression(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        self.try_accept_non_aggression(target, true, events);
    }

    fn process_reject_non_aggression(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        events.push(Event::TreatyRejected {
            from_empire: target,
            to_empire: self.state.player_empire,
            treaty_type: TreatyType::NonAggressionPact,
        });
    }

    fn process_gather_intelligence(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        let Some(status) = self.validate_known_foreign_empire(target, events) else {
            return;
        };
        if status == RelationshipStatus::Unknown {
            events.push(Event::error(
                "Cannot gather intelligence: establish diplomatic contact first",
            ));
            return;
        }

        self.ensure_empire_intel_contact(target);
        let Some(intel) = self.state.empire_intel.get_mut(&target) else {
            return;
        };
        if intel.last_gather_turn == Some(self.state.turn) {
            events.push(Event::error(
                "Gather Intelligence already used on this empire this turn",
            ));
            return;
        }

        intel.points = intel.points.saturating_add(INTEL_GATHER_POINTS);
        intel.level = Self::intel_level_for_points(intel.points);
        intel.last_gather_turn = Some(self.state.turn);
        events.push(Event::IntelGained {
            with_empire: target,
            gained: INTEL_GATHER_POINTS,
            new_level: intel.level,
            sources: vec![IntelSource::GatherIntelligence],
        });
    }

    fn process_placeholder_espionage(
        &mut self,
        target: EmpireId,
        mission: EspionageMission,
        events: &mut Vec<Event>,
    ) {
        let Some(status) = self.validate_known_foreign_empire(target, events) else {
            return;
        };
        if status == RelationshipStatus::Unknown {
            events.push(Event::error(format!(
                "{} requires diplomatic contact first",
                mission.label()
            )));
            return;
        }
        events.push(Event::EspionageMissionUnavailable {
            with_empire: target,
            mission,
        });
    }

    fn process_cancel_treaty(
        &mut self,
        target: EmpireId,
        treaty_type: TreatyType,
        events: &mut Vec<Event>,
    ) {
        if !self.cancel_treaty(target, treaty_type, events) {
            events.push(Event::error(format!(
                "No active {} with empire {}",
                treaty_type.label(),
                target.0
            )));
        }
    }

    fn process_issue_warning(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        events.push(Event::WarningIssued {
            from_empire: self.state.player_empire,
            to_empire: target,
        });
        self.queue_diplomatic_communication(
            self.state.player_empire,
            target,
            DiplomaticCommunicationType::Warning,
            DiplomaticTone::Threatening,
            "Border Warning".to_string(),
            "Withdraw pressure near our border sectors or expect retaliation.".to_string(),
            vec![DiplomaticResponse::Acknowledge],
            Some(self.state.turn.saturating_add(4)),
            None,
            events,
        );
    }

    fn process_demand_tribute(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        events.push(Event::TributeDemanded {
            from_empire: self.state.player_empire,
            to_empire: target,
        });
        self.queue_diplomatic_communication(
            self.state.player_empire,
            target,
            DiplomaticCommunicationType::TributeDemand,
            DiplomaticTone::Threatening,
            "Tribute Demand".to_string(),
            "Submit tribute this turn cycle or relations will deteriorate further.".to_string(),
            vec![DiplomaticResponse::Comply, DiplomaticResponse::Refuse],
            Some(self.state.turn.saturating_add(3)),
            None,
            events,
        );
    }

    fn process_send_greeting(&mut self, target: EmpireId, events: &mut Vec<Event>) {
        self.queue_diplomatic_communication(
            self.state.player_empire,
            target,
            DiplomaticCommunicationType::FirstContact,
            DiplomaticTone::Formal,
            "Diplomatic Greeting".to_string(),
            "We acknowledge contact and open formal diplomatic channels.".to_string(),
            vec![DiplomaticResponse::Acknowledge],
            Some(self.state.turn.saturating_add(6)),
            None,
            events,
        );
    }

    fn process_respond_to_communication(
        &mut self,
        communication_id: u64,
        response: DiplomaticResponse,
        events: &mut Vec<Event>,
    ) {
        let Some(index) = self
            .state
            .diplomacy_pending_communications
            .iter()
            .position(|message| message.communication_id == communication_id)
        else {
            events.push(Event::error("Diplomatic communication not found"));
            return;
        };

        let communication = self
            .state
            .diplomacy_pending_communications
            .remove(index)
            .expect("communication index found via position should be valid");

        if !communication.available_responses.contains(&response) {
            events.push(Event::error(
                "Invalid response for diplomatic communication",
            ));
            return;
        }

        let other = if communication.sending_empire == self.state.player_empire {
            communication.receiving_empire
        } else {
            communication.sending_empire
        };

        match (
            communication.communication_type,
            response,
            communication.treaty_type,
        ) {
            (
                DiplomaticCommunicationType::PeaceOffer,
                DiplomaticResponse::Accept,
                Some(TreatyType::Truce),
            ) => {
                self.try_accept_peace(other, false, events);
            }
            (
                DiplomaticCommunicationType::PeaceOffer,
                DiplomaticResponse::Reject,
                Some(TreatyType::Truce),
            ) => {
                self.process_reject_peace(other, events);
            }
            (
                DiplomaticCommunicationType::TreatyProposal,
                DiplomaticResponse::Accept,
                Some(TreatyType::NonAggressionPact),
            ) => {
                self.try_accept_non_aggression(other, false, events);
            }
            (
                DiplomaticCommunicationType::TreatyProposal,
                DiplomaticResponse::Reject,
                Some(TreatyType::NonAggressionPact),
            ) => {
                self.process_reject_non_aggression(other, events);
            }
            (DiplomaticCommunicationType::TributeDemand, DiplomaticResponse::Refuse, _) => {
                events.push(Event::TributeRefused {
                    from_empire: communication.sending_empire,
                    to_empire: communication.receiving_empire,
                });
            }
            _ => {}
        }
    }

    fn create_ship_design(
        &mut self,
        hull_id: HullId,
        components: Vec<ComponentId>,
        name: Option<String>,
        events: &mut Vec<Event>,
    ) {
        let player = self.state.player_empire;
        let completed_techs: Vec<_> = self
            .state
            .empires
            .get(&player)
            .map(|e| e.research.completed.to_vec())
            .unwrap_or_default();
        let available_resources: Vec<StrategicResource> = self
            .state
            .empire_resource_access
            .get(&player)
            .map(|m| {
                m.iter()
                    .filter_map(|(resource, count)| (*count > 0).then_some(*resource))
                    .collect()
            })
            .unwrap_or_default();

        let design_name =
            name.unwrap_or_else(|| format!("Design {}", self.state.next_custom_design_id));
        let design = CustomShipDesign {
            design_id: CustomDesignId(self.state.next_custom_design_id),
            hull_id,
            components,
            owner: player,
            name: design_name.clone(),
            obsolete: false,
        };

        if let Err(reason) = design.validate_with_resources(&completed_techs, &available_resources)
        {
            events.push(Event::ShipDesignInvalid {
                empire: player,
                hull_id,
                reason: reason.to_string(),
            });
            return;
        }

        let design_id = design.design_id;
        self.state.custom_designs.insert(design_id, design);
        self.state.next_custom_design_id += 1;

        events.push(Event::ShipDesignCreated {
            empire: player,
            design_id,
            hull_id,
            name: design_name,
        });
    }

    fn delete_ship_design(&mut self, design_id: CustomDesignId, events: &mut Vec<Event>) {
        let player = self.state.player_empire;

        match self.state.custom_designs.get_mut(&design_id) {
            None => {
                events.push(Event::error(format!(
                    "Custom design {} not found",
                    design_id.0
                )));
            }
            Some(design) if design.owner != player => {
                events.push(Event::error("Cannot delete a design you do not own"));
            }
            Some(design) => {
                design.obsolete = true;
                events.push(Event::ShipDesignDeleted {
                    empire: player,
                    design_id,
                });
            }
        }
    }
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
            self.travel_turns_for_fleet(fleet_id, empire_id, from_star, rally_star);

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

    fn travel_turns_for_fleet(
        &self,
        fleet_id: FleetId,
        empire_id: EmpireId,
        from: StarId,
        to: StarId,
    ) -> (u32, bool) {
        let (base_turns, used_lane) = travel_turns_with_lanes(&self.state, empire_id, from, to);
        let mobility = self
            .state
            .fleet_evaluation(fleet_id)
            .map(|summary| summary.mobility)
            .unwrap_or(100)
            .max(1);
        let supply = self
            .state
            .fleets
            .get(&fleet_id)
            .map(|fleet| {
                if fleet_uses_logistics_penalties(fleet.kind) {
                    self.state.projected_fleet_supply(empire_id, to)
                } else {
                    FleetSupplyState::Supplied
                }
            })
            .unwrap_or(FleetSupplyState::Supplied);
        let adjusted_turns = ((base_turns as u64 * 100 * supply.movement_penalty_pct() as u64)
            .div_ceil(mobility as u64 * 100) as u32)
            .max(1);
        (adjusted_turns, used_lane)
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
            self.travel_turns_for_fleet(fleet_id, self.state.player_empire, from, destination);

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

        if let BuildItem::Ship(design_id) = item
            && design_id.record().is_none()
        {
            events.push(Event::error(format!(
                "Cannot build ship — design {} is invalid",
                design_id.0
            )));
            return;
        }

        if let BuildItem::CustomShip(design_id) = item {
            let empire = match self.state.empires.get(&self.state.player_empire) {
                Some(e) => e,
                None => {
                    events.push(Event::error("Player empire not found"));
                    return;
                }
            };
            let completed_techs: Vec<_> = empire.research.completed.to_vec();
            let available_resources: Vec<StrategicResource> = self
                .state
                .empire_resource_access
                .get(&self.state.player_empire)
                .map(|m| {
                    m.iter()
                        .filter_map(|(resource, count)| (*count > 0).then_some(*resource))
                        .collect()
                })
                .unwrap_or_default();
            match self.state.custom_designs.get(&design_id) {
                None => {
                    events.push(Event::error(format!(
                        "Custom design {} not found",
                        design_id.0
                    )));
                    return;
                }
                Some(design) => {
                    if design.obsolete {
                        events.push(Event::error(format!(
                            "Custom design {} is obsolete",
                            design_id.0
                        )));
                        return;
                    }
                    if design.owner != self.state.player_empire {
                        events.push(Event::error("Custom design not owned by player"));
                        return;
                    }
                    if let Err(reason) =
                        design.validate_with_resources(&completed_techs, &available_resources)
                    {
                        events.push(Event::error(format!(
                            "Custom design {} invalid: {}",
                            design_id.0, reason
                        )));
                        return;
                    }
                }
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
            self.travel_turns_for_fleet(fleet_id, self.state.player_empire, origin, destination);

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
        if let Some(star) = self.state.stars.get_mut(&star_id)
            && let Some(planet) = star.planets.get_mut(planet_index)
        {
            planet.colony = Some(colony_id);
        }

        // Consume the colonizer fleet.
        // The mission maps are cleared defensively so that no stale entries can
        // accumulate if the fleet was somehow in an inconsistent state on load.
        self.remove_fleet_and_assignments(fleet_id);

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

        let Some(target_colony) = self.state.colonies.get(&planet_colony).cloned() else {
            events.push(Event::error(format!(
                "Colony {} not found",
                planet_colony.0
            )));
            return;
        };
        let assessment = self.assess_invasion(&fleet, fleet_id, star_id, &target_colony);
        if assessment.supply_state == FleetSupplyState::OutOfSupply {
            events.push(Event::error(format!(
                "Fleet {} cannot invade while out of supply",
                fleet_id.0
            )));
            return;
        }

        self.execute_invasion(
            fleet.owner,
            fleet_id,
            star_id,
            planet_index,
            planet_colony,
            colony_owner,
            &assessment,
            events,
        );
    }

    /// Deterministic invasion balance for a troop transport against a colony.
    /// Shared by the player command path and the AI invasion pass.
    fn assess_invasion(
        &self,
        fleet: &Fleet,
        fleet_id: FleetId,
        star_id: StarId,
        target_colony: &Colony,
    ) -> InvasionAssessment {
        let colony_owner = target_colony.owner;
        let hostile_orbital_defenders = self.state.fleets.iter().any(|(fid, f)| {
            *fid != fleet_id
                && f.location == star_id
                && f.owner == colony_owner
                && !self.state.fleet_missions.contains_key(fid)
                && !self.state.scout_missions.contains_key(fid)
                && !self.state.survey_missions.contains_key(fid)
        });

        let defense_strength = Self::colony_defense_strength(target_colony);
        let base_invasion_strength = self.invasion_strength_for_empire(fleet.owner, fleet.ships);
        let supply_state = self.state.derived_fleet_supply_state(fleet_id);
        let escort_quality: u32 = self
            .state
            .fleets
            .iter()
            .filter(|(fid, f)| {
                **fid != fleet_id
                    && f.owner == fleet.owner
                    && f.location == star_id
                    && !self.state.fleet_missions.contains_key(*fid)
                    && !self.state.scout_missions.contains_key(*fid)
                    && !self.state.survey_missions.contains_key(*fid)
            })
            .map(|(fid, _)| {
                self.state
                    .fleet_evaluation(*fid)
                    .map(|eval| eval.escort_quality)
                    .unwrap_or(0)
            })
            .sum();
        let invasion_strength = base_invasion_strength
            .saturating_add(escort_quality / 5)
            .saturating_mul(supply_state.invasion_strength_pct())
            / 100;

        InvasionAssessment {
            invasion_strength,
            defense_strength,
            hostile_orbital_defenders,
            supply_state,
        }
    }

    /// Resolve an assessed invasion for any attacking empire: blocked by
    /// orbital defenders, conquest when strength exceeds defense, or
    /// attrition otherwise. Callers have already validated the target.
    #[allow(clippy::too_many_arguments)]
    fn execute_invasion(
        &mut self,
        attacker: EmpireId,
        fleet_id: FleetId,
        star_id: StarId,
        planet_index: usize,
        planet_colony: ColonyId,
        colony_owner: EmpireId,
        assessment: &InvasionAssessment,
        events: &mut Vec<Event>,
    ) {
        let invasion_strength = assessment.invasion_strength;
        let defense_strength = assessment.defense_strength;
        let transport_ships = self
            .state
            .fleets
            .get(&fleet_id)
            .map(|f| f.ships)
            .unwrap_or(0);

        if assessment.hostile_orbital_defenders {
            events.push(Event::InvasionFailed {
                attacker,
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
                colony.owner = attacker;
                colony.stability = CAPTURED_UNREST_STABILITY;
                colony.build_queue.clear();
                colony.accumulated_production = 0;
                colony.rally_point = None;
            }
            self.state
                .colony_recent_conquest_turn
                .insert(planet_colony, self.state.turn);
            self.state
                .colony_unrest
                .insert(planet_colony, ColonyUnrestState::RevoltRisk);
            self.state.colony_unrest_causes.insert(
                planet_colony,
                vec![UnrestCause::RecentConquest, UnrestCause::LowStability],
            );
            self.state.colony_rebellion_risk_bp.insert(
                planet_colony,
                ColonyUnrestState::RevoltRisk.base_rebellion_risk_bp(),
            );

            self.remove_fleet_and_assignments(fleet_id);
            // Ownership and fleet changes can invalidate cached blockade state.
            // Recompute now so the next end-turn economy pass does not use stale blockade entries.
            self.state.colony_blockade = self.state.recompute_colony_blockade();
            self.refresh_unrest_statuses();

            events.push(Event::InvasionSucceeded {
                attacker,
                defender: colony_owner,
                fleet: fleet_id,
                star: star_id,
                planet_index,
                colony: planet_colony,
                transports_lost: transport_ships,
            });
            return;
        }

        let transports_lost = self.reduce_transport_fleet(fleet_id, 1);
        // Fleet attrition can remove the last blockading transport; keep cache in sync.
        self.state.colony_blockade = self.state.recompute_colony_blockade();
        events.push(Event::InvasionFailed {
            attacker,
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
            self.remove_fleet_and_assignments(fleet_id);
        }
        transports_lost
    }

    fn remove_fleet_and_assignments(&mut self, fleet_id: FleetId) {
        self.state.fleets.remove(&fleet_id);
        self.state.fleet_orders.remove(&fleet_id);
        self.state.fleet_roles.remove(&fleet_id);
        self.state.fleet_formations.remove(&fleet_id);
        self.state.fleet_names.remove(&fleet_id);
        self.state.fleet_missions.remove(&fleet_id);
        self.state.scout_missions.remove(&fleet_id);
        self.state.survey_missions.remove(&fleet_id);
        self.state.fleet_custom_designs.remove(&fleet_id);
    }

    /// Mark a specific planet as surveyed and emit a `PlanetSurveyCompleted` event.
    fn complete_survey_at_star(
        &mut self,
        star_id: StarId,
        planet_index: usize,
        surveying_empire: Option<EmpireId>,
        events: &mut Vec<Event>,
    ) {
        let completed_techs: Vec<TechId> = surveying_empire
            .and_then(|empire_id| self.state.empires.get(&empire_id))
            .map(|empire| empire.research.completed.clone())
            .unwrap_or_default();

        if let Some(star) = self.state.stars.get_mut(&star_id)
            && let Some(planet) = star.planets.get_mut(planet_index)
            && !planet.surveyed
        {
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

            for special in crate::state::visible_specials_for_empire(planet, &completed_techs) {
                if special != crate::state::PlanetSpecial::AncientRuins
                    && special.is_major_discovery()
                {
                    events.push(Event::PlanetSpecialDiscovered {
                        star: star_id,
                        planet_index,
                        special,
                    });
                }
            }

            for anomaly in crate::state::visible_anomalies_for_empire(planet, &completed_techs) {
                events.push(Event::AnomalyDetected {
                    star: star_id,
                    planet_index,
                    anomaly,
                });
            }

            for resource in crate::state::visible_resources_for_empire(planet, &completed_techs) {
                events.push(Event::StrategicResourceDiscovered {
                    star: star_id,
                    planet_index,
                    resource,
                });
            }

            events.push(Event::PlanetSurveyCompleted {
                star: star_id,
                planet_index,
            });
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
                let first_status = self.first_contact_status_for_empire(empire_id);
                self.state.diplomacy.insert(empire_id, first_status);
                let mut relationship = DiplomaticRelationship::from_status(first_status);
                relationship.first_contact_turn = Some(self.state.turn);
                relationship.last_major_diplomatic_event_turn = self.state.turn;
                relationship.known_doctrine = self
                    .empire_definition(empire_id)
                    .map(|def| def.doctrine_short_summary());
                self.state
                    .diplomacy_relationships
                    .insert(empire_id, relationship);
                self.ensure_empire_intel_contact(empire_id);
                events.push(Event::FirstContact {
                    with_empire: empire_id,
                });
                let tone = self.communication_tone_for(
                    empire_id,
                    first_status,
                    DiplomaticCommunicationType::FirstContact,
                );
                let sender_name = self
                    .state
                    .empires
                    .get(&empire_id)
                    .map(|empire| empire.name.clone())
                    .unwrap_or_else(|| format!("Empire {}", empire_id.0));
                let body = match tone {
                    DiplomaticTone::Cooperative => {
                        "We welcome this first exchange and seek stable, mutually beneficial relations."
                    }
                    DiplomaticTone::Suspicious => {
                        "Your presence is noted. Respect our frontier and we may avoid conflict."
                    }
                    _ => "Formal diplomatic channels are now open between our governments.",
                };
                self.queue_diplomatic_communication(
                    empire_id,
                    self.state.player_empire,
                    DiplomaticCommunicationType::FirstContact,
                    tone,
                    format!("Transmission from {sender_name}"),
                    body.to_string(),
                    vec![DiplomaticResponse::Acknowledge],
                    Some(self.state.turn.saturating_add(8)),
                    None,
                    events,
                );
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
            let first_status = self.first_contact_status_for_empire(ai_empire_id);
            self.state.diplomacy.insert(ai_empire_id, first_status);
            let mut relationship = DiplomaticRelationship::from_status(first_status);
            relationship.first_contact_turn = Some(self.state.turn);
            relationship.last_major_diplomatic_event_turn = self.state.turn;
            relationship.known_doctrine = self
                .empire_definition(ai_empire_id)
                .map(|def| def.doctrine_short_summary());
            self.state
                .diplomacy_relationships
                .insert(ai_empire_id, relationship);
            self.ensure_empire_intel_contact(ai_empire_id);
            events.push(Event::FirstContact {
                with_empire: ai_empire_id,
            });
            let tone = self.communication_tone_for(
                ai_empire_id,
                first_status,
                DiplomaticCommunicationType::FirstContact,
            );
            let sender_name = self
                .state
                .empires
                .get(&ai_empire_id)
                .map(|empire| empire.name.clone())
                .unwrap_or_else(|| format!("Empire {}", ai_empire_id.0));
            self.queue_diplomatic_communication(
                ai_empire_id,
                self.state.player_empire,
                DiplomaticCommunicationType::FirstContact,
                tone,
                format!("Transmission from {sender_name}"),
                "Contact established. State intentions clearly to avoid strategic misunderstanding."
                    .to_string(),
                vec![DiplomaticResponse::Acknowledge],
                Some(self.state.turn.saturating_add(8)),
                None,
                events,
            );
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

        for enemy_fleet_id in enemy_fleet_ids {
            // Re-fetch arriving fleet — may have been destroyed in a prior iteration.
            let (a_str, a_int) = match self.state.fleets.get(&arrived_fleet_id) {
                Some(f) => (f.strength, f.integrity),
                None => break,
            };
            let (d_str, d_int, d_owner) = match self.state.fleets.get(&enemy_fleet_id) {
                Some(f) => (f.strength, f.integrity, f.owner),
                None => continue,
            };

            events.push(Event::FleetEngagementStarted {
                star: star_id,
                fleet_a: arrived_fleet_id,
                fleet_b: enemy_fleet_id,
            });

            if self.should_avoid_engagement(arrived_fleet_id, a_str, d_str)
                && self.start_retreat_if_possible(arrived_fleet_id, star_id, events)
            {
                break;
            }
            if self.should_avoid_engagement(enemy_fleet_id, d_str, a_str)
                && self.start_retreat_if_possible(enemy_fleet_id, star_id, events)
            {
                continue;
            }

            let (mut a_attack_pct, a_defense_pct, a_retreat_threshold) =
                self.fleet_combat_profile(arrived_fleet_id, star_id);
            let (mut d_attack_pct, d_defense_pct, d_retreat_threshold) =
                self.fleet_combat_profile(enemy_fleet_id, star_id);

            let role_a = self.state.fleet_role_for(arrived_fleet_id);
            let role_b = self.state.fleet_role_for(enemy_fleet_id);
            let formation_a = self.state.fleet_formation_for(arrived_fleet_id);
            let formation_b = self.state.fleet_formation_for(enemy_fleet_id);
            let kind_a = self
                .state
                .fleets
                .get(&arrived_fleet_id)
                .map(|fleet| fleet.kind)
                .unwrap_or(FleetKind::Scout);
            let kind_b = self
                .state
                .fleets
                .get(&enemy_fleet_id)
                .map(|fleet| fleet.kind)
                .unwrap_or(FleetKind::Scout);
            let ships_a = self
                .state
                .fleets
                .get(&arrived_fleet_id)
                .map(|fleet| fleet.ships)
                .unwrap_or(0);
            let ships_b = self
                .state
                .fleets
                .get(&enemy_fleet_id)
                .map(|fleet| fleet.ships)
                .unwrap_or(0);
            let doctrine_a = self.empire_doctrine_summary(arrived_owner);
            let doctrine_b = self.empire_doctrine_summary(d_owner);
            let supply_a = self.state.derived_fleet_supply_state(arrived_fleet_id);
            let supply_b = self.state.derived_fleet_supply_state(enemy_fleet_id);

            let mut phase_summaries: Vec<CombatPhaseSummary> = Vec::new();
            let a_detection = self.fleet_detection_score(arrived_fleet_id, kind_a, formation_a);
            let d_detection = self.fleet_detection_score(enemy_fleet_id, kind_b, formation_b);
            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::Detection,
                pressure_a: a_detection,
                pressure_b: d_detection,
                note: if a_detection == d_detection {
                    "Detection parity; both fleets commit".to_string()
                } else if a_detection > d_detection {
                    "Fleet A gains engagement initiative".to_string()
                } else {
                    "Fleet B gains engagement initiative".to_string()
                },
            });

            let initiative_lead = a_detection.saturating_sub(d_detection);
            let counter_lead = d_detection.saturating_sub(a_detection);
            a_attack_pct = a_attack_pct.saturating_add((initiative_lead / 5).min(8));
            d_attack_pct = d_attack_pct.saturating_add((counter_lead / 5).min(8));
            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::Positioning,
                pressure_a: a_attack_pct,
                pressure_b: d_attack_pct,
                note: format!(
                    "{} / {} posture applied; supply {} vs {}",
                    formation_a.label(),
                    formation_b.label(),
                    supply_a.label(),
                    supply_b.label()
                ),
            });

            let opening_bonus_a = self.fleet_opening_volley_bonus(kind_a, formation_a, role_a);
            let opening_bonus_b = self.fleet_opening_volley_bonus(kind_b, formation_b, role_b);

            // Artillery tends to counter defensive formations in auto-resolve.
            if formation_a == FleetFormation::Artillery
                && matches!(
                    formation_b,
                    FleetFormation::Defensive | FleetFormation::EscortScreen
                )
            {
                a_attack_pct = a_attack_pct.saturating_add(10);
            }
            if formation_b == FleetFormation::Artillery
                && matches!(
                    formation_a,
                    FleetFormation::Defensive | FleetFormation::EscortScreen
                )
            {
                d_attack_pct = d_attack_pct.saturating_add(10);
            }

            // Apply defense modifiers from custom designs, if any.
            // A positive defense_modifier increases the divisor, reducing incoming damage.
            let a_defense = self
                .state
                .fleet_custom_designs
                .get(&arrived_fleet_id)
                .and_then(|did| self.state.custom_designs.get(did))
                .map(|d| d.derived_stats().defense.max(1) as u64)
                .unwrap_or(a_str as u64);
            let d_defense = self
                .state
                .fleet_custom_designs
                .get(&enemy_fleet_id)
                .and_then(|did| self.state.custom_designs.get(did))
                .map(|d| d.derived_stats().defense.max(1) as u64)
                .unwrap_or(d_str as u64);

            let a_attack = ((a_str as u64 * a_attack_pct.max(10) as u64) / 100).max(1);
            let d_attack = ((d_str as u64 * d_attack_pct.max(10) as u64) / 100).max(1);
            let a_effective_defense = (a_defense * a_defense_pct.max(10) as u64 / 100).max(1);
            let d_effective_defense = (d_defense * d_defense_pct.max(10) as u64 / 100).max(1);
            let opening_damage_to_enemy = if opening_bonus_a == 0 {
                0
            } else {
                (opening_bonus_a as u64 * 100 / d_effective_defense).max(1)
            };
            let opening_damage_to_arrived = if opening_bonus_b == 0 {
                0
            } else {
                (opening_bonus_b as u64 * 100 / a_effective_defense).max(1)
            };
            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::OpeningVolley,
                pressure_a: opening_bonus_a,
                pressure_b: opening_bonus_b,
                note: format!(
                    "Opening pressure A:{} B:{}",
                    opening_damage_to_enemy, opening_damage_to_arrived
                ),
            });

            // Simultaneous damage: each fleet takes damage proportional to the
            // opponent's strength relative to its own effective defense.
            // Formula: damage = (opponent_strength * 100) / own_defense
            // Equal attack and defense → both take 100 damage (destroyed).
            // Use u64 intermediates to avoid overflow when strength is large.
            let main_damage_to_arrived: u32 =
                ((d_attack * 100) / a_effective_defense).min(u32::MAX as u64) as u32;
            let main_damage_to_enemy: u32 =
                ((a_attack * 100) / d_effective_defense).min(u32::MAX as u64) as u32;
            let mut damage_to_arrived = main_damage_to_arrived;
            let mut damage_to_enemy = main_damage_to_enemy;
            damage_to_arrived = damage_to_arrived
                .saturating_add(opening_damage_to_arrived.min(u32::MAX as u64) as u32);
            damage_to_enemy =
                damage_to_enemy.saturating_add(opening_damage_to_enemy.min(u32::MAX as u64) as u32);
            // Main engagement captures only sustained-exchange deltas.
            // Opening volley and attrition are reported in their own phases.
            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::MainEngagement,
                pressure_a: main_damage_to_enemy,
                pressure_b: main_damage_to_arrived,
                note: "Sustained engagement exchange".to_string(),
            });

            let attrition_to_arrived = if kind_b.is_combat() || kind_b == FleetKind::TroopTransport
            {
                ((damage_to_arrived as u64 * (a_retreat_threshold as u64)) / ATTRITION_DIVISOR)
                    .min(12) as u32
            } else {
                0
            };
            let attrition_to_enemy = if kind_a.is_combat() || kind_a == FleetKind::TroopTransport {
                ((damage_to_enemy as u64 * (d_retreat_threshold as u64)) / ATTRITION_DIVISOR)
                    .min(12) as u32
            } else {
                0
            };
            damage_to_arrived = damage_to_arrived.saturating_add(attrition_to_arrived);
            damage_to_enemy = damage_to_enemy.saturating_add(attrition_to_enemy);
            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::Attrition,
                pressure_a: attrition_to_enemy,
                pressure_b: attrition_to_arrived,
                note: "Cohesion attrition applied".to_string(),
            });

            let new_a_int = a_int.saturating_sub(damage_to_arrived);
            let new_d_int = d_int.saturating_sub(damage_to_enemy);

            let fleet_a_destroyed = new_a_int == 0;
            let fleet_b_destroyed = new_d_int == 0;

            events.push(Event::CombatResolved {
                star: star_id,
                fleet_a: arrived_fleet_id,
                empire_a: arrived_owner,
                fleet_b: enemy_fleet_id,
                empire_b: d_owner,
                strength_a: a_str,
                strength_b: d_str,
                integrity_a_remaining: new_a_int,
                integrity_b_remaining: new_d_int,
                fleet_a_destroyed,
                fleet_b_destroyed,
            });

            if fleet_a_destroyed {
                self.remove_fleet_and_assignments(arrived_fleet_id);
            } else if let Some(f) = self.state.fleets.get_mut(&arrived_fleet_id) {
                f.integrity = new_a_int;
            }

            if fleet_b_destroyed {
                self.remove_fleet_and_assignments(enemy_fleet_id);
            } else if let Some(f) = self.state.fleets.get_mut(&enemy_fleet_id) {
                f.integrity = new_d_int;
            }

            if !fleet_a_destroyed
                && new_a_int <= a_retreat_threshold
                && self.start_retreat_if_possible(arrived_fleet_id, star_id, events)
            {
                phase_summaries.push(CombatPhaseSummary {
                    phase: CombatPhase::RetreatOrCollapse,
                    pressure_a: a_retreat_threshold,
                    pressure_b: d_retreat_threshold,
                    note: "Fleet A disengages and retreats".to_string(),
                });
                phase_summaries.push(CombatPhaseSummary {
                    phase: CombatPhase::Resolution,
                    pressure_a: new_a_int,
                    pressure_b: new_d_int,
                    note: "Engagement resolved with withdrawal".to_string(),
                });
                self.push_battle_report(BattleReport {
                    report_id: BATTLE_REPORT_ID_PENDING,
                    turn: self.state.turn,
                    star: star_id,
                    fleet_a: arrived_fleet_id,
                    fleet_b: enemy_fleet_id,
                    empire_a: arrived_owner,
                    empire_b: d_owner,
                    role_a,
                    role_b,
                    formation_a,
                    formation_b,
                    doctrine_a,
                    doctrine_b,
                    supply_a,
                    supply_b,
                    kind_a,
                    kind_b,
                    ships_a,
                    ships_b,
                    integrity_a_start: a_int,
                    integrity_b_start: d_int,
                    integrity_a_end: new_a_int,
                    integrity_b_end: new_d_int,
                    fleet_a_destroyed,
                    fleet_b_destroyed,
                    fleet_a_retreated: true,
                    fleet_b_retreated: false,
                    phases: phase_summaries,
                    system_outcome: "Fleet A retreat; system contested".to_string(),
                });
                break;
            }
            if !fleet_b_destroyed
                && new_d_int <= d_retreat_threshold
                && self.start_retreat_if_possible(enemy_fleet_id, star_id, events)
            {
                phase_summaries.push(CombatPhaseSummary {
                    phase: CombatPhase::RetreatOrCollapse,
                    pressure_a: a_retreat_threshold,
                    pressure_b: d_retreat_threshold,
                    note: "Fleet B disengages and retreats".to_string(),
                });
                phase_summaries.push(CombatPhaseSummary {
                    phase: CombatPhase::Resolution,
                    pressure_a: new_a_int,
                    pressure_b: new_d_int,
                    note: "Engagement resolved with withdrawal".to_string(),
                });
                self.push_battle_report(BattleReport {
                    report_id: BATTLE_REPORT_ID_PENDING,
                    turn: self.state.turn,
                    star: star_id,
                    fleet_a: arrived_fleet_id,
                    fleet_b: enemy_fleet_id,
                    empire_a: arrived_owner,
                    empire_b: d_owner,
                    role_a,
                    role_b,
                    formation_a,
                    formation_b,
                    doctrine_a,
                    doctrine_b,
                    supply_a,
                    supply_b,
                    kind_a,
                    kind_b,
                    ships_a,
                    ships_b,
                    integrity_a_start: a_int,
                    integrity_b_start: d_int,
                    integrity_a_end: new_a_int,
                    integrity_b_end: new_d_int,
                    fleet_a_destroyed,
                    fleet_b_destroyed,
                    fleet_a_retreated: false,
                    fleet_b_retreated: true,
                    phases: phase_summaries,
                    system_outcome: "Fleet B retreat; system contested".to_string(),
                });
                continue;
            }

            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::RetreatOrCollapse,
                pressure_a: a_retreat_threshold,
                pressure_b: d_retreat_threshold,
                note: "No withdrawal; battle pressed to outcome".to_string(),
            });
            let system_outcome = match (fleet_a_destroyed, fleet_b_destroyed) {
                (true, true) => "Mutual destruction at contested system".to_string(),
                (true, false) => "Fleet B holds system".to_string(),
                (false, true) => "Fleet A holds system".to_string(),
                (false, false) => "Both fleets survive; control unresolved".to_string(),
            };
            phase_summaries.push(CombatPhaseSummary {
                phase: CombatPhase::Resolution,
                pressure_a: new_a_int,
                pressure_b: new_d_int,
                note: system_outcome.clone(),
            });
            self.push_battle_report(BattleReport {
                report_id: BATTLE_REPORT_ID_PENDING,
                turn: self.state.turn,
                star: star_id,
                fleet_a: arrived_fleet_id,
                fleet_b: enemy_fleet_id,
                empire_a: arrived_owner,
                empire_b: d_owner,
                role_a,
                role_b,
                formation_a,
                formation_b,
                doctrine_a,
                doctrine_b,
                supply_a,
                supply_b,
                kind_a,
                kind_b,
                ships_a,
                ships_b,
                integrity_a_start: a_int,
                integrity_b_start: d_int,
                integrity_a_end: new_a_int,
                integrity_b_end: new_d_int,
                fleet_a_destroyed,
                fleet_b_destroyed,
                fleet_a_retreated: false,
                fleet_b_retreated: false,
                phases: phase_summaries,
                system_outcome,
            });

            if fleet_a_destroyed {
                break;
            }
        }
    }

    fn should_avoid_engagement(
        &self,
        fleet_id: FleetId,
        own_strength: u32,
        enemy_strength: u32,
    ) -> bool {
        let Some(role) = self.state.fleet_roles.get(&fleet_id).copied() else {
            return false;
        };
        matches!(role, FleetRole::ExplorationFleet | FleetRole::SurveyGroup)
            && enemy_strength > own_strength.saturating_mul(EXPLORER_AVOID_ENGAGEMENT_MULTIPLIER)
    }

    fn fleet_combat_profile(&self, fleet_id: FleetId, star_id: StarId) -> (u32, u32, u32) {
        let fleet = match self.state.fleets.get(&fleet_id) {
            Some(fleet) => fleet,
            None => return (100, 100, 40),
        };
        let role = self.state.fleet_roles.get(&fleet_id).copied();
        let formation = self.state.fleet_formation_for(fleet_id);

        let mut attack_pct: u32 = 100;
        let mut defense_pct: u32 = 100;
        let mut retreat_threshold: u32 = 40;

        if let Some(role) = role {
            match role {
                FleetRole::ExplorationFleet => {
                    attack_pct = attack_pct.saturating_sub(20);
                    defense_pct = defense_pct.saturating_sub(10);
                    retreat_threshold = 75;
                }
                FleetRole::SurveyGroup => {
                    attack_pct = attack_pct.saturating_sub(15);
                    retreat_threshold = 70;
                }
                FleetRole::ColonyEscort => {
                    defense_pct = defense_pct.saturating_add(10);
                    retreat_threshold = 60;
                }
                FleetRole::PatrolFleet => {
                    defense_pct = defense_pct.saturating_add(5);
                }
                FleetRole::StrikeFleet => {
                    attack_pct = attack_pct.saturating_add(15);
                    defense_pct = defense_pct.saturating_sub(5);
                    retreat_threshold = 30;
                }
                FleetRole::DefenseFleet => {
                    defense_pct = defense_pct.saturating_add(20);
                }
                FleetRole::InvasionFleet => {
                    defense_pct = defense_pct.saturating_add(10);
                    retreat_threshold = 60;
                }
                FleetRole::BlockadeFleet => {
                    attack_pct = attack_pct.saturating_add(10);
                    retreat_threshold = 35;
                }
                FleetRole::RapidResponseFleet => {
                    attack_pct = attack_pct.saturating_add(5);
                    retreat_threshold = 55;
                }
                FleetRole::TradeProtectionFleet => {
                    defense_pct = defense_pct.saturating_add(10);
                    retreat_threshold = 60;
                }
            }
        }

        match formation {
            FleetFormation::Balanced => {}
            FleetFormation::Aggressive => {
                attack_pct = attack_pct.saturating_add(20);
                defense_pct = defense_pct.saturating_sub(10);
                retreat_threshold = retreat_threshold.saturating_sub(10);
            }
            FleetFormation::Defensive => {
                defense_pct = defense_pct.saturating_add(20);
                attack_pct = attack_pct.saturating_sub(10);
                retreat_threshold = retreat_threshold.saturating_add(10);
            }
            FleetFormation::FastAttack => {
                attack_pct = attack_pct.saturating_add(15);
                defense_pct = defense_pct.saturating_sub(15);
            }
            FleetFormation::Artillery => {
                attack_pct = attack_pct.saturating_add(20);
                defense_pct = defense_pct.saturating_sub(10);
            }
            FleetFormation::EscortScreen => {
                attack_pct = attack_pct.saturating_sub(10);
                defense_pct = defense_pct.saturating_add(25);
            }
        }

        if formation == FleetFormation::EscortScreen {
            let has_vulnerable_friendly = self.state.fleets.values().any(|other| {
                other.owner == fleet.owner
                    && other.location == star_id
                    && !other.kind.is_combat()
                    && other.id != fleet_id
            });
            if has_vulnerable_friendly {
                defense_pct = defense_pct.saturating_add(15);
            }
        }

        let supply = if fleet_uses_logistics_penalties(fleet.kind) {
            self.state.derived_fleet_supply_state(fleet_id)
        } else {
            FleetSupplyState::Supplied
        };
        attack_pct = attack_pct.saturating_mul(supply.combat_attack_pct()) / 100;
        defense_pct = defense_pct.saturating_mul(supply.combat_defense_pct()) / 100;
        if supply != FleetSupplyState::Supplied {
            retreat_threshold = retreat_threshold.saturating_add(10);
        }

        (
            attack_pct.clamp(50, 180),
            defense_pct.clamp(50, 200),
            retreat_threshold.clamp(15, 85),
        )
    }

    fn empire_doctrine_summary(&self, empire_id: EmpireId) -> String {
        self.state
            .empires
            .get(&empire_id)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
            .map(|def| def.doctrine_short_summary())
            .unwrap_or_else(|| "N/A".to_string())
    }

    fn fleet_detection_score(
        &self,
        fleet_id: FleetId,
        kind: FleetKind,
        formation: FleetFormation,
    ) -> u32 {
        let mobility = self
            .state
            .fleet_evaluation(fleet_id)
            .map(|eval| eval.mobility)
            .unwrap_or(100);
        let mut score = mobility / 2;
        score = score.saturating_add(match kind {
            FleetKind::Scout | FleetKind::FastScout => 18,
            FleetKind::Science | FleetKind::SurveyCutter => 14,
            FleetKind::MissileFrigate => 9,
            FleetKind::Destroyer => 6,
            FleetKind::EscortFrigate | FleetKind::PatrolCorvette => 4,
            FleetKind::TroopTransport | FleetKind::Colonizer | FleetKind::ColonyArk => 1,
        });
        score = score.saturating_add(match formation {
            FleetFormation::FastAttack => 10,
            FleetFormation::Artillery => 4,
            FleetFormation::Defensive | FleetFormation::EscortScreen => 2,
            FleetFormation::Balanced | FleetFormation::Aggressive => 0,
        });
        score
    }

    fn fleet_opening_volley_bonus(
        &self,
        kind: FleetKind,
        formation: FleetFormation,
        role: FleetRole,
    ) -> u32 {
        let mut bonus: u32 = match kind {
            FleetKind::MissileFrigate => 25,
            FleetKind::Destroyer => 14,
            FleetKind::EscortFrigate => 6,
            FleetKind::PatrolCorvette => 4,
            FleetKind::Scout | FleetKind::FastScout => 0,
            FleetKind::Science | FleetKind::SurveyCutter => 0,
            FleetKind::Colonizer | FleetKind::ColonyArk => 0,
            FleetKind::TroopTransport => 2,
        };
        bonus = bonus.saturating_add(match formation {
            FleetFormation::Artillery => 18,
            FleetFormation::Aggressive => 8,
            FleetFormation::FastAttack => 5,
            FleetFormation::Defensive | FleetFormation::EscortScreen => 2,
            FleetFormation::Balanced => 0,
        });
        bonus = bonus.saturating_add(match role {
            FleetRole::StrikeFleet => 10,
            FleetRole::BlockadeFleet => 6,
            FleetRole::PatrolFleet | FleetRole::RapidResponseFleet => 3,
            FleetRole::DefenseFleet | FleetRole::TradeProtectionFleet => 2,
            FleetRole::ColonyEscort | FleetRole::InvasionFleet => 1,
            FleetRole::ExplorationFleet | FleetRole::SurveyGroup => 0,
        });
        bonus
    }

    fn push_battle_report(&mut self, mut report: BattleReport) {
        report.report_id = self.state.next_battle_report_id;
        self.state.next_battle_report_id = self.state.next_battle_report_id.saturating_add(1);
        if self.state.battle_reports.len() >= BATTLE_REPORT_MAX_HISTORY {
            self.state.battle_reports.pop_front();
        }
        // Mirror into the v3 report log so the TUI can render a
        // card-driven view.  Damage still flows from the v2 formula;
        // a follow-up PR will replace the formula with per-verb
        // card-driven resolution.
        self.push_battle_report_v3_from_v2(&report);
        self.state.battle_reports.push_back(report);
    }

    /// Build a v3 report that mirrors a v2 report.  Hands are drafted
    /// deterministically from fleet composition.  The v3 round log is a
    /// synthetic per-round view: each round shows the top card from each
    /// hand and the integrity after that round.  This lets the TUI render
    /// the v3 report now; the damage formula in v3 will be filled in by
    /// a follow-up PR.
    fn push_battle_report_v3_from_v2(&mut self, v2: &BattleReport) {
        use crate::combat_v3::{BattleReportV3, BattleRoundSummary, CardId, build_hand};
        let hand_a = build_hand(&self.state, v2.fleet_a, v2.empire_a);
        let hand_b = build_hand(&self.state, v2.fleet_b, v2.empire_b);
        let total_rounds = crate::combat_v3::MAX_ROUNDS as u32;
        let start_a = v2.integrity_a_start;
        let start_b = v2.integrity_b_start;
        let end_a = v2.integrity_a_end;
        let end_b = v2.integrity_b_end;
        let rounds = (0..crate::combat_v3::MAX_ROUNDS)
            .map(|i| {
                let slice = i as u32 + 1;
                let a_int_after = if slice == total_rounds {
                    end_a
                } else {
                    start_a.saturating_sub((start_a.saturating_sub(end_a)) * slice / total_rounds)
                };
                let b_int_after = if slice == total_rounds {
                    end_b
                } else {
                    start_b.saturating_sub((start_b.saturating_sub(end_b)) * slice / total_rounds)
                };
                BattleRoundSummary {
                    round: i,
                    card_a: hand_a.get(i as usize).copied(),
                    card_b: hand_b.get(i as usize).copied(),
                    effect_a: hand_a
                        .get(i as usize)
                        .map(|c| format!("played {} (v2 damage applied)", c.0))
                        .unwrap_or_else(|| "(no card)".to_string()),
                    effect_b: hand_b
                        .get(i as usize)
                        .map(|c| format!("played {} (v2 damage applied)", c.0))
                        .unwrap_or_else(|| "(no card)".to_string()),
                    integrity_a_after: a_int_after,
                    integrity_b_after: b_int_after,
                }
            })
            .collect();
        let v3 = BattleReportV3 {
            report_id: self.state.allocate_battle_report_v3_id(),
            turn: v2.turn,
            star: v2.star,
            fleet_a: v2.fleet_a,
            fleet_b: v2.fleet_b,
            empire_a: v2.empire_a,
            empire_b: v2.empire_b,
            role_a: v2.role_a,
            role_b: v2.role_b,
            formation_a: v2.formation_a,
            formation_b: v2.formation_b,
            supply_a: v2.supply_a,
            supply_b: v2.supply_b,
            ships_a: v2.ships_a,
            ships_b: v2.ships_b,
            integrity_a_start: v2.integrity_a_start,
            integrity_b_start: v2.integrity_b_start,
            integrity_a_end: v2.integrity_a_end,
            integrity_b_end: v2.integrity_b_end,
            fleet_a_destroyed: v2.fleet_a_destroyed,
            fleet_b_destroyed: v2.fleet_b_destroyed,
            fleet_a_retreated: v2.fleet_a_retreated,
            fleet_b_retreated: v2.fleet_b_retreated,
            hand_a,
            hand_b,
            rounds,
            system_outcome: v2.system_outcome.clone(),
        };
        if self.state.battle_reports_v3.len() >= BATTLE_REPORT_MAX_HISTORY {
            self.state.battle_reports_v3.pop_front();
        }
        // The CardId import is used in the round-log effect strings above.
        let _ = std::marker::PhantomData::<CardId>;
        self.state.battle_reports_v3.push_back(v3);
    }

    fn start_retreat_if_possible(
        &mut self,
        fleet_id: FleetId,
        from_star: StarId,
        events: &mut Vec<Event>,
    ) -> bool {
        if self.state.fleet_missions.contains_key(&fleet_id)
            || self.state.scout_missions.contains_key(&fleet_id)
            || self.state.survey_missions.contains_key(&fleet_id)
        {
            return false;
        }

        let Some(fleet) = self.state.fleets.get(&fleet_id).cloned() else {
            return false;
        };

        let retreat_star = self
            .state
            .colonies
            .values()
            .filter(|colony| colony.owner == fleet.owner && colony.star != from_star)
            .map(|colony| colony.star)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|candidate| {
                let from = self.state.stars.get(&from_star)?;
                let to = self.state.stars.get(&candidate)?;
                let dx = (to.x - from.x) as i64;
                let dy = (to.y - from.y) as i64;
                Some((dx * dx + dy * dy, candidate))
            })
            .min_by_key(|(dist, sid)| (*dist, *sid))
            .map(|(_, sid)| sid);

        let Some(destination) = retreat_star else {
            return false;
        };

        let (turns, _) = self.travel_turns_for_fleet(fleet_id, fleet.owner, from_star, destination);
        self.state.fleet_missions.insert(
            fleet_id,
            FleetMission {
                fleet: fleet_id,
                destination,
                turns_remaining: turns,
                origin: from_star,
                total_duration: turns,
            },
        );
        self.state
            .fleet_orders
            .insert(fleet_id, FleetOrder::MoveToSystem(destination));
        events.push(Event::FleetRetreatTriggered {
            fleet: fleet_id,
            from_star,
            to_star: destination,
            remaining_integrity: fleet.integrity,
        });
        true
    }
}

/// Returns true if `empire_a` and `empire_b` are combat-eligible against each other.
///
/// Diplomacy in v1 is stored from the player's perspective.  If neither empire
/// is the player, the function returns `false` (AI-vs-AI not applicable).
///
/// Combat is eligible for `Contacted`, `Tense`, `Hostile`, and `War` statuses.
/// `Contacted` is kept eligible for backward compatibility with v1 saves and tests.
/// Outcome-determining numbers for a prospective invasion, computed by
/// [`Engine::assess_invasion`]. Resolution is deterministic, so these fully
/// decide success.
struct InvasionAssessment {
    invasion_strength: u32,
    defense_strength: u32,
    hostile_orbital_defenders: bool,
    supply_state: FleetSupplyState,
}

fn is_combat_eligible(state: &GameState, empire_a: EmpireId, empire_b: EmpireId) -> bool {
    state
        .relationship_status(empire_a, empire_b)
        .is_combat_eligible()
}

#[cfg(test)]
mod tests;
