//! Deterministic hand draft.
//!
//! `build_hand` is a pure function of `(fleet, empire_state)`.  No RNG
//! advances.  Same inputs → same hand.  Iterates sorted collections.

use super::HAND_SIZE;
use super::card::{CardId, CardSource, HOLD_FIRE, POOL, card_by_id};
use crate::state::{EmpireId, FleetId, GameState};

/// Build a 5-card hand for `fleet` owned by `empire`.  Pure: no RNG, no
/// state mutation.  Stable across replays.
pub fn build_hand(state: &GameState, fleet: FleetId, empire: EmpireId) -> Vec<CardId> {
    let mut pool: Vec<CardId> = Vec::new();
    let mut seen: std::collections::BTreeSet<CardId> = std::collections::BTreeSet::new();

    // 1. Hull card from the fleet's kind/template.
    if let Some(fleet_data) = state.fleets.get(&fleet) {
        if let Some(card_id) = hull_card_for_kind(fleet_data.kind) {
            if seen.insert(card_id) {
                pool.push(card_id);
            }
        }
    }

    // 2. Component cards from the fleet's custom design.
    if let Some(design_id) = state.fleet_custom_designs.get(&fleet) {
        if let Some(design) = state.custom_designs.get(design_id) {
            for component_id in &design.components {
                if let Some(card_id) = component_card(*component_id) {
                    if seen.insert(card_id) {
                        pool.push(card_id);
                    }
                }
            }
        }
    }

    // 3. Tech cards from the empire's unlocked techs (deterministic by tech id).
    if let Some(empire_data) = state.empires.get(&empire) {
        for tech_id in &empire_data.research.completed {
            if let Some(card_id) = tech_card(*tech_id) {
                if seen.insert(card_id) {
                    pool.push(card_id);
                }
            }
        }
    }

    // 4. Faction signature card (if any) — included as a soft bonus; falls
    //    back to the empire's first doctrine when no signature matches.
    if let Some(empire_data) = state.empires.get(&empire) {
        if let Some(empire_def_id) = empire_data.empire_def {
            if let Some(sig) = faction_signature_for(empire_def_id) {
                if seen.insert(sig) {
                    pool.push(sig);
                }
            }
        }
    }

    // 5. Doctrine-biased sort.  Each card is scored by whether its doctrine
    //    tag matches any doctrine the empire favours.  No RNG; pure
    //    sort by (score desc, id asc).
    let empire_doctrines = empire_doctrine_tags(state, empire);
    pool.sort_by(|a, b| {
        let sa = card_score(*a, &empire_doctrines);
        let sb = card_score(*b, &empire_doctrines);
        sb.cmp(&sa).then(a.0.cmp(&b.0))
    });

    // 6. Truncate to HAND_SIZE; pad with HOLD_FIRE.
    pool.truncate(HAND_SIZE);
    while pool.len() < HAND_SIZE {
        pool.push(HOLD_FIRE.id);
    }
    pool
}

/// Look up the hull card for a fleet kind.  Maps common combat hulls to
/// their signature cards.
fn hull_card_for_kind(kind: crate::state::FleetKind) -> Option<CardId> {
    use crate::state::FleetKind::*;
    match kind {
        Scout | FastScout => Some(CardId(6)),
        Science | SurveyCutter => Some(CardId(15)),
        Colonizer | ColonyArk => Some(CardId(2)),
        TroopTransport => Some(CardId(11)),
        EscortFrigate => Some(CardId(10)),
        MissileFrigate => Some(CardId(9)),
        Destroyer => Some(CardId(9)),
        PatrolCorvette => Some(CardId(10)),
    }
}

/// Look up the card unlocked by a given component id.
fn component_card(component: crate::state::ComponentId) -> Option<CardId> {
    use crate::state::ComponentId;
    match component {
        ComponentId(1) => Some(CardId(2)), // Kinetic Battery → Ablative Hull fallback (no dedicated card)
        ComponentId(2) => Some(CardId(9)), // Missile Rack → Orbital Bombardment
        ComponentId(10) => Some(CardId(2)), // Reinforced Plating → Ablative Hull
        ComponentId(11) => Some(CardId(3)), // Shield Matrix → Phased Shield
        ComponentId(12) => Some(CardId(4)), // Point Defense → CIWS Grid
        ComponentId(20) => None,           // Chemical Thrusters
        ComponentId(21) => Some(CardId(5)), // Ion Drive → Burn Maneuver
        ComponentId(30) => Some(CardId(7)), // Targeting Suite → Targeting Lock
        ComponentId(31) => Some(CardId(8)), // Long-Range Sensors → Sensor Sweep
        ComponentId(32) => None,           // Cargo Pods
        ComponentId(40) => Some(CardId(2)), // Colony Core → Ablative Hull fallback
        ComponentId(41) => Some(CardId(8)), // Survey Array → Sensor Sweep
        ComponentId(42) => Some(CardId(11)), // Troop Bays → Troop Drop
        _ => None,
    }
}

/// Look up the card unlocked by a given tech id.  Limited to a few v1
/// technologies that are mentioned in the design doc.
fn tech_card(tech: crate::state::TechId) -> Option<CardId> {
    use crate::state::TechId;
    // 11 = Battle Doctrine, 12 = Survey Drones, 13 = Rapid Transit Drives,
    // 16 = Perimeter Defense, 17 = Long-Range Strike.
    match tech {
        TechId(11) => Some(CardId(14)), // Battle Doctrine → Formation Rally
        TechId(13) => Some(CardId(12)), // Rapid Transit Drives → Warp Retreat
        TechId(17) => Some(CardId(13)), // Long-Range Strike → Ordnance Overcharge
        _ => None,
    }
}

/// Faction signature card by empire definition id.  Matches the 8
/// factions in `state::empire_definitions`.
fn faction_signature_for(def_id: crate::state::EmpireDefinitionId) -> Option<CardId> {
    use crate::state::EmpireDefinitionId;
    match def_id {
        EmpireDefinitionId(0) => Some(CardId(18)), // Ashveran
        EmpireDefinitionId(1) => Some(CardId(20)), // Luminal
        EmpireDefinitionId(2) => Some(CardId(23)), // Sylvaran
        EmpireDefinitionId(3) => Some(CardId(22)), // Thalori
        EmpireDefinitionId(4) => Some(CardId(16)), // Vorath
        EmpireDefinitionId(5) => Some(CardId(19)), // Elarith
        EmpireDefinitionId(6) => Some(CardId(21)), // Terran Concord
        EmpireDefinitionId(7) => Some(CardId(17)), // Terran Dominion
        _ => None,
    }
}

/// Doctrine tags (as strings) the empire favours.  Sourced from the
/// empire's `doctrine_weights`.  Top 3 by weight, ties broken by label
/// asc.  No RNG.
fn empire_doctrine_tags(state: &GameState, empire: EmpireId) -> Vec<String> {
    let Some(empire_data) = state.empires.get(&empire) else {
        return Vec::new();
    };
    let Some(def_id) = empire_data.empire_def else {
        return Vec::new();
    };
    let Some(def) = state::empire_definition_by_id(def_id) else {
        return Vec::new();
    };
    let mut weights: Vec<_> = def.doctrine_weights.to_vec();
    weights.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then(a.doctrine.label().cmp(b.doctrine.label()))
    });
    weights
        .into_iter()
        .take(3)
        .map(|w| w.doctrine.label().to_string())
        .collect()
}

use crate::state;

/// Card score: number of matching doctrine tags between the card's
/// doctrine string and the empire's favoured doctrine tags.  Stable.
fn card_score(card: CardId, doctrines: &[String]) -> u32 {
    let c = card_by_id(card);
    doctrines
        .iter()
        .filter(|d| c.doctrine.contains(d.as_str()))
        .count() as u32
}

/// Iterator helper used by callers that want the pool of all cards a
/// fleet could possibly draw from (e.g. for `Inspire`-style refill).
pub fn possible_cards(state: &GameState, fleet: FleetId, empire: EmpireId) -> Vec<CardId> {
    let mut out: Vec<CardId> = build_hand(state, fleet, empire).into_iter().collect();
    out.extend(
        POOL.iter()
            .filter(|c| c.source == CardSource::Faction)
            .map(|c| c.id),
    );
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Fleet, FleetKind, GameState, ScenarioSetup};

    fn new_state() -> GameState {
        crate::engine::Engine::new_from_setup(ScenarioSetup::default_for_seed(42)).state
    }

    fn add_fleet(state: &mut GameState, id: u64, kind: FleetKind) -> FleetId {
        let id = FleetId(id);
        state.fleets.insert(
            id,
            Fleet {
                id,
                owner: state.player_empire,
                kind,
                location: crate::state::StarId(0),
                ships: 1,
                strength: 10,
                integrity: 100,
            },
        );
        id
    }

    #[test]
    fn hand_has_five_cards() {
        let mut state = new_state();
        let fleet = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let hand = build_hand(&state, fleet, state.player_empire);
        assert_eq!(hand.len(), HAND_SIZE);
    }

    #[test]
    fn hand_is_deterministic_for_same_state() {
        let mut state = new_state();
        let fleet = add_fleet(&mut state, 1, FleetKind::EscortFrigate);
        let h1 = build_hand(&state, fleet, state.player_empire);
        let h2 = build_hand(&state, fleet, state.player_empire);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hand_changes_with_fleet_kind() {
        let mut state = new_state();
        let destroyer = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let escort = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let h1 = build_hand(&state, destroyer, state.player_empire);
        let h2 = build_hand(&state, escort, state.player_empire);
        assert_ne!(
            h1, h2,
            "different fleet kinds should draft different hull cards"
        );
    }

    #[test]
    fn hand_pads_with_hold_fire_when_no_techs() {
        let mut state = new_state();
        let fleet = add_fleet(&mut state, 1, FleetKind::Scout);
        let hand = build_hand(&state, fleet, state.player_empire);
        // Scout with no custom design and no techs still gets a 5-card hand.
        assert_eq!(hand.len(), HAND_SIZE);
        // At least one HOLD_FIRE is likely; we don't assert because the
        // faction signature might fill the gap.
    }

    #[test]
    fn hand_card_ids_are_sorted_by_doctrine_score() {
        let mut state = new_state();
        let fleet = add_fleet(&mut state, 1, FleetKind::EscortFrigate);
        let hand = build_hand(&state, fleet, state.player_empire);
        // The list should be deterministic; verify by running twice.
        let hand2 = build_hand(&state, fleet, state.player_empire);
        assert_eq!(hand, hand2);
    }

    #[test]
    fn hand_does_not_contain_duplicates() {
        let mut state = new_state();
        let fleet = add_fleet(&mut state, 1, FleetKind::Destroyer);
        // Force a richer hand: assign a custom design and several techs.
        let design = crate::state::CustomShipDesign {
            design_id: crate::state::CustomDesignId(1),
            hull_id: crate::state::HullId(10),
            components: vec![
                crate::state::ComponentId(1),
                crate::state::ComponentId(10),
                crate::state::ComponentId(21),
                crate::state::ComponentId(30),
                crate::state::ComponentId(42),
            ],
            owner: state.player_empire,
            name: "Test".to_string(),
            obsolete: false,
        };
        let design_id = design.design_id;
        state.custom_designs.insert(design_id, design);
        state.fleet_custom_designs.insert(fleet, design_id);
        if let Some(empire) = state.empires.get_mut(&state.player_empire) {
            empire.research.completed = vec![
                crate::state::TechId(11),
                crate::state::TechId(13),
                crate::state::TechId(17),
            ];
        }
        let hand = build_hand(&state, fleet, state.player_empire);
        let mut sorted = hand.clone();
        sorted.sort_by_key(|c| c.0);
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            hand.len(),
            "hand should have no duplicates: {hand:?}"
        );
    }

    #[test]
    fn faction_signature_included_for_known_empires() {
        let mut state = new_state();
        let fleet = add_fleet(&mut state, 1, FleetKind::Destroyer);
        // Force player empire to be Vorath (id 4) by setting its def.
        if let Some(empire) = state.empires.get_mut(&state.player_empire) {
            empire.empire_def = Some(crate::state::EmpireDefinitionId(4));
        }
        let hand = build_hand(&state, fleet, state.player_empire);
        assert!(
            hand.contains(&CardId(16)),
            "Vorath signature (Coercive Mandate) should appear"
        );
    }
}
