//! Combat v3 — deterministic hand draft.
//!
//! `build_hand` is a pure function of `(state, fleet, empire)`: same inputs
//! always produce the same 5-card hand.  No RNG is consumed during draft.
//!
//! Rules (mirrors `docs/design/combat-v3.md` §Hand Draft):
//!
//! 1. Start with a hull-granted card (if the fleet kind maps to one).
//! 2. Append one card per `Component` installed on the fleet's custom
//!    design (when the design is known), sorted by `ComponentId`.
//! 3. Append one card per completed tech that unlocks a battle card, in
//!    sorted `TechId` order.
//! 4. Sort the pool by `CardId` ascending, take the first 5.
//! 5. Pad with the empire's signature card, dedup-tracked.
//! 6. Pad to exactly 5 with `Hold Fire`.
//!
//! The function never iterates a `HashMap`; all collection iteration is
//! over sorted slices or `BTreeMap`/`BTreeSet` already in `GameState`.

use crate::combat_v3::card::{CardId, HOLD_FIRE, signature_for_faction};
use crate::state::{ComponentId, EmpireId, Fleet, FleetKind, TechId};
use std::collections::BTreeSet;

/// Hand size per side.  Matches the design doc and the existing TUI mock.
pub const HAND_SIZE: usize = 5;

/// Maximum number of rounds in a Combat v3 battle.
pub const MAX_ROUNDS: u8 = 5;

/// Map a `FleetKind` to a deterministic hull card.  Returns `None` if the
/// kind has no card mapping (fall back to padding).
///
/// The mapping mirrors the design doc table 1–15, with one card per hull
/// archetype that grants a card.  Hulls that grant no card return `None`.
pub fn hull_card_for_kind(kind: FleetKind) -> Option<CardId> {
    match kind {
        FleetKind::EscortFrigate => Some(CardId::KINETIC_SALVO),
        FleetKind::MissileFrigate => Some(CardId::KINETIC_SALVO),
        FleetKind::Destroyer => Some(CardId::ORBITAL_BOMBARDMENT),
        FleetKind::PatrolCorvette => Some(CardId::KINETIC_SALVO),
        FleetKind::Scout => Some(CardId::DRIFT_BURN),
        FleetKind::FastScout => Some(CardId::DRIFT_BURN),
        FleetKind::Science => None,
        FleetKind::SurveyCutter => Some(CardId::SURVEYORS_GAMBIT),
        FleetKind::Colonizer => None,
        FleetKind::ColonyArk => None,
        FleetKind::TroopTransport => Some(CardId::TROOP_DROP),
    }
}

/// Map a `ComponentId` to a deterministic card, when the component is
/// one of the v1 card-granting components.  Returns `None` for utility
/// components that don't grant battle cards in v1.
pub fn component_card_for(component: ComponentId) -> Option<CardId> {
    use crate::state::ComponentId as C;
    match component {
        C::REINFORCED_PLATING => Some(CardId::ABLATIVE_HULL),
        C::SHIELD_MATRIX => Some(CardId::PHASED_SHIELD),
        C::POINT_DEFENSE_GRID => Some(CardId::CIWS_GRID),
        C::ION_DRIVE => Some(CardId::BURN_MANEUVER),
        C::TARGETING_SUITE => Some(CardId::TARGETING_LOCK),
        C::LONG_RANGE_SENSORS => Some(CardId::SENSOR_SWEEP),
        C::TROOP_BAYS => Some(CardId::TROOP_DROP),
        _ => None,
    }
}

/// Map a completed `TechId` to a card.  Only techs that grant a card in
/// v1 return `Some`.
pub fn tech_card_for(tech: TechId) -> Option<CardId> {
    match tech {
        TechId::RAPID_TRANSIT => Some(CardId::WARP_RETREAT),
        TechId::STRIKE_DOCTRINE => Some(CardId::ORDNANCE_OVERCHARGE),
        TechId::BATTLE_DOCTRINE => Some(CardId::FORMATION_RALLY),
        _ => None,
    }
}

/// Inputs to `build_hand` derived from `GameState` for a single side.
#[derive(Debug, Clone)]
pub struct HandInputs<'a> {
    /// The fleet this hand belongs to.
    pub fleet: &'a Fleet,
    /// Empire that owns the fleet.
    pub empire_id: EmpireId,
    /// Components installed on the fleet's custom design (empty for
    /// stock `ShipDesignId`-built fleets).  Already deduplicated by
    /// the caller; this function does not re-dedupe.
    pub components: &'a [ComponentId],
    /// Techs completed by the empire.  Already deduplicated by the caller.
    pub completed_techs: &'a [TechId],
    /// Empire definition numeric id (`u8`) for the signature-card fallback.
    /// `None` if the empire has no definition.
    pub empire_def_id: Option<u8>,
}

/// Build a deterministic 5-card hand from fleet/empire state.
///
/// The hand is filled in **bucket priority** order so that hull
/// always lands in slot 0, components fill subsequent slots in
/// sorted `ComponentId` order, techs fill following slots in sorted
/// `TechId` order, the faction signature is slotted next (and may
/// repeat), and `Hold Fire` is the final pad.  Duplicates are
/// filtered (a hull card that also matches a component / tech is
/// only placed once).
///
/// Always returns exactly `HAND_SIZE` cards.  Never consumes RNG.
/// All input slices are expected to be already deduplicated by the
/// caller; this function never iterates a `HashMap`.
pub fn build_hand(inputs: &HandInputs<'_>) -> Vec<CardId> {
    let mut hand: Vec<CardId> = Vec::with_capacity(HAND_SIZE);
    let mut used: BTreeSet<CardId> = BTreeSet::new();

    // 1. Hull-granted card (slot 0).
    if let Some(card) = hull_card_for_kind(inputs.fleet.kind) {
        hand.push(card);
        used.insert(card);
    }

    // 2. Component-granted cards in sorted `ComponentId` order.
    // Caller passes a slice; sort a local copy so the caller's
    // order is not required to be pre-sorted.
    let mut components_sorted: Vec<ComponentId> = inputs.components.to_vec();
    components_sorted.sort();
    for comp in components_sorted {
        if hand.len() >= HAND_SIZE {
            break;
        }
        if let Some(card) = component_card_for(comp)
            && used.insert(card)
        {
            hand.push(card);
        }
    }

    // 3. Tech-granted cards in sorted `TechId` order.
    let mut techs_sorted: Vec<TechId> = inputs.completed_techs.to_vec();
    techs_sorted.sort();
    for tech in techs_sorted {
        if hand.len() >= HAND_SIZE {
            break;
        }
        if let Some(card) = tech_card_for(tech)
            && used.insert(card)
        {
            hand.push(card);
        }
    }

    // 4. Faction signature card (may repeat to fill).
    if hand.len() < HAND_SIZE
        && let Some(faction) = inputs.empire_def_id
        && let Some(sig) = signature_for_faction(faction)
    {
        // Signature is *not* added to `used`; it may repeat, per
        // design (the signature is the "identity" of the empire and
        // is allowed to fill multiple slots).
        while hand.len() < HAND_SIZE {
            hand.push(sig);
        }
    }

    // 5. Hold Fire pad.
    while hand.len() < HAND_SIZE {
        hand.push(HOLD_FIRE.id);
    }

    hand.truncate(HAND_SIZE);
    hand
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Fleet, FleetId, FleetKind, StarId};

    fn fleet(kind: FleetKind) -> Fleet {
        Fleet {
            id: FleetId(1),
            owner: EmpireId(1),
            location: StarId(1),
            ships: 1,
            kind,
            strength: 1,
            integrity: 100,
        }
    }

    #[test]
    fn hull_card_for_kind_uses_design_table() {
        assert_eq!(
            hull_card_for_kind(FleetKind::EscortFrigate),
            Some(CardId::KINETIC_SALVO)
        );
        assert_eq!(
            hull_card_for_kind(FleetKind::Destroyer),
            Some(CardId::ORBITAL_BOMBARDMENT)
        );
        assert_eq!(
            hull_card_for_kind(FleetKind::Scout),
            Some(CardId::DRIFT_BURN)
        );
        assert_eq!(hull_card_for_kind(FleetKind::Colonizer), None);
    }

    #[test]
    fn build_hand_returns_exactly_five_cards() {
        let f = fleet(FleetKind::Destroyer);
        let inputs = HandInputs {
            fleet: &f,
            empire_id: EmpireId(1),
            components: &[],
            completed_techs: &[],
            empire_def_id: None,
        };
        let hand = build_hand(&inputs);
        assert_eq!(hand.len(), HAND_SIZE);
    }

    #[test]
    fn build_hand_pads_with_hold_fire_when_empty() {
        let f = fleet(FleetKind::Colonizer);
        let inputs = HandInputs {
            fleet: &f,
            empire_id: EmpireId(1),
            components: &[],
            completed_techs: &[],
            empire_def_id: None,
        };
        let hand = build_hand(&inputs);
        assert_eq!(hand.len(), HAND_SIZE);
        for c in &hand {
            assert_eq!(*c, HOLD_FIRE.id, "expected Hold Fire pad");
        }
    }

    #[test]
    fn build_hand_uses_signature_for_faction() {
        // Vorath Dominion is faction id 4 → Coercive Mandate.
        let f = fleet(FleetKind::Colonizer);
        let inputs = HandInputs {
            fleet: &f,
            empire_id: EmpireId(1),
            components: &[],
            completed_techs: &[],
            empire_def_id: Some(4),
        };
        let hand = build_hand(&inputs);
        assert_eq!(hand.len(), HAND_SIZE);
        for c in &hand {
            assert_eq!(*c, CardId::COERCIVE_MANDATE);
        }
    }

    #[test]
    fn build_hand_is_deterministic_for_same_inputs() {
        let f = fleet(FleetKind::Destroyer);
        let components = vec![
            ComponentId::SHIELD_MATRIX,
            ComponentId::ION_DRIVE,
            ComponentId::TARGETING_SUITE,
        ];
        let techs = vec![TechId::BATTLE_DOCTRINE, TechId::STRIKE_DOCTRINE];
        let inputs_a = HandInputs {
            fleet: &f,
            empire_id: EmpireId(1),
            components: &components,
            completed_techs: &techs,
            empire_def_id: Some(0),
        };
        let inputs_b = HandInputs {
            fleet: &f,
            empire_id: EmpireId(1),
            components: &components,
            completed_techs: &techs,
            empire_def_id: Some(0),
        };
        assert_eq!(build_hand(&inputs_a), build_hand(&inputs_b));
    }

    #[test]
    fn build_hand_buckets_hull_components_techs_signature_hold_fire() {
        // Bucket-priority hand draft: slot 0 is the hull card; slots
        // 1..k are component cards in ascending `ComponentId` order
        // (duplicates dropped); subsequent slots are tech cards in
        // ascending `TechId` order; the signature card fills the
        // remaining slots; `Hold Fire` is the final pad.
        let f = fleet(FleetKind::EscortFrigate);
        // Components deliberately given in a non-sorted order to
        // confirm the function sorts internally.
        let components = vec![
            ComponentId::TARGETING_SUITE,    // id 30 → TARGETING_LOCK
            ComponentId::ION_DRIVE,          // id 21 → BURN_MANEUVER
            ComponentId::REINFORCED_PLATING, // id 10 → ABLATIVE_HULL
        ];
        let techs = vec![TechId::STRIKE_DOCTRINE, TechId::RAPID_TRANSIT];
        let inputs = HandInputs {
            fleet: &f,
            empire_id: EmpireId(1),
            components: &components,
            completed_techs: &techs,
            empire_def_id: None,
        };
        let hand = build_hand(&inputs);
        assert_eq!(hand.len(), HAND_SIZE);
        // Slot 0: hull card for EscortFrigate.
        assert_eq!(
            hand[0],
            hull_card_for_kind(FleetKind::EscortFrigate).unwrap()
        );
        // Slot 1..3: component cards in ascending ComponentId order.
        assert_eq!(hand[1], CardId::ABLATIVE_HULL); // REINFORCED_PLATING id 10
        assert_eq!(hand[2], CardId::BURN_MANEUVER); // ION_DRIVE id 21
        assert_eq!(hand[3], CardId::TARGETING_LOCK); // TARGETING_SUITE id 30
        // Slot 4: tech card in ascending `TechId` order.  TechIds
        // are RAPID_TRANSIT (13) and STRIKE_DOCTRINE (17), so the
        // lower-id one (RAPID_TRANSIT) wins and grants
        // WARP_RETREAT.  Exact equality — anything else is a
        // regression in the bucket-priority sort.
        assert_eq!(hand[4], CardId::WARP_RETREAT);
    }
}
