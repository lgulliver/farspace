//! Combat v3 — card definitions and the static registry.
//!
//! The v1 pool contains 23 unique cards plus a single shared `Hold Fire`
//! fallback.  Card IDs are stable `u16` values; the registry is sorted by
//! `CardId` ascending and is shared across player and AI hands.
//!
//! Determinism: the registry is a `&'static [CardDef]` and never mutated.
//! All lookup functions use linear scans over a tiny list (≤ 24 entries) —
//! no `HashMap` iteration.
//!
//! Original IP: every card name, source string, effect text, and doctrine
//! bias below is original to FARSPACE.  No content is derived from
//! Master of Orion or any other published 4X title.

use std::fmt;

/// Stable identifier for a card in the v1 pool.
///
/// The numeric values are part of the public save schema and must not be
/// reordered or reused.  Hold Fire uses `CardId(0)` and the rest of the
/// pool occupies 1..=23.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardId(pub u16);

impl CardId {
    /// Hold Fire — the universal no-op pad.
    pub const HOLD_FIRE: CardId = CardId(0);

    /// 1 — Kinetic Salvo
    pub const KINETIC_SALVO: CardId = CardId(1);
    /// 2 — Ablative Hull
    pub const ABLATIVE_HULL: CardId = CardId(2);
    /// 3 — Phased Shield
    pub const PHASED_SHIELD: CardId = CardId(3);
    /// 4 — CIWS Grid
    pub const CIWS_GRID: CardId = CardId(4);
    /// 5 — Burn Maneuver
    pub const BURN_MANEUVER: CardId = CardId(5);
    /// 6 — Drift Burn
    pub const DRIFT_BURN: CardId = CardId(6);
    /// 7 — Targeting Lock
    pub const TARGETING_LOCK: CardId = CardId(7);
    /// 8 — Sensor Sweep
    pub const SENSOR_SWEEP: CardId = CardId(8);
    /// 9 — Orbital Bombardment
    pub const ORBITAL_BOMBARDMENT: CardId = CardId(9);
    /// 10 — Defensive Screen
    pub const DEFENSIVE_SCREEN: CardId = CardId(10);
    /// 11 — Troop Drop
    pub const TROOP_DROP: CardId = CardId(11);
    /// 12 — Warp Retreat
    pub const WARP_RETREAT: CardId = CardId(12);
    /// 13 — Ordnance Overcharge
    pub const ORDNANCE_OVERCHARGE: CardId = CardId(13);
    /// 14 — Formation Rally
    pub const FORMATION_RALLY: CardId = CardId(14);
    /// 15 — Surveyor's Gambit
    pub const SURVEYORS_GAMBIT: CardId = CardId(15);
    /// 16 — Coercive Mandate
    pub const COERCIVE_MANDATE: CardId = CardId(16);
    /// 17 — Siege Doctrine
    pub const SIEGE_DOCTRINE: CardId = CardId(17);
    /// 18 — Industrial Juggernaut
    pub const INDUSTRIAL_JUGGERNAUT: CardId = CardId(18);
    /// 19 — Algorithmic Defense
    pub const ALGORITHMIC_DEFENSE: CardId = CardId(19);
    /// 20 — Pathfinder's Wager
    pub const PATHFINDERS_WAGER: CardId = CardId(20);
    /// 21 — Council of Voices
    pub const COUNCIL_OF_VOICES: CardId = CardId(21);
    /// 22 — Trade Barge Stand
    pub const TRADE_BARGE_STAND: CardId = CardId(22);
    /// 23 — Bloom Shield
    pub const BLOOM_SHIELD: CardId = CardId(23);
}

impl fmt::Display for CardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Card#{}", self.0)
    }
}

/// Verb taxonomy.  Each card resolves into one or more verbs.  v1 implements
/// the verbs listed in `docs/design/combat-v3.md` plus the no-op `Hold Fire`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardVerb {
    /// Deal `dmg` hp to the opposing fleet.
    Strike,
    /// Reduce incoming damage by `def_value` for this round.
    Guard,
    /// +1 initiative (player's card resolves first this round).
    Maneuver,
    /// Halve incoming damage this round.
    Evasive,
    /// Deal `dmg` hp to the opposing fleet; damage recurs each remaining round.
    Salvo,
    /// Stronger guard for this round.
    Fortify,
    /// Cancel the opposing card for this round.
    Disrupt,
    /// Reveal enemy hand; no direct combat effect in v1.
    Probe,
    /// Buff the next Strike this battle: +25% damage.
    Mark,
    /// Strike +50% but self-inflict `1` damage.
    Overcharge,
    /// Auto-retreat at 50% of current integrity.
    Withdraw,
    /// Post-battle invasion strength bonus (v1: report-only).
    Bolster,
    /// Refill 1 hand slot mid-battle.
    Inspire,
    /// No-op.
    Noop,
}

impl CardVerb {
    /// Short display label for the verb.
    pub fn label(self) -> &'static str {
        match self {
            CardVerb::Strike => "Strike",
            CardVerb::Guard => "Guard",
            CardVerb::Maneuver => "Maneuver",
            CardVerb::Evasive => "Evasive",
            CardVerb::Salvo => "Salvo",
            CardVerb::Fortify => "Fortify",
            CardVerb::Disrupt => "Disrupt",
            CardVerb::Probe => "Probe",
            CardVerb::Mark => "Mark",
            CardVerb::Overcharge => "Overcharge",
            CardVerb::Withdraw => "Withdraw",
            CardVerb::Bolster => "Bolster",
            CardVerb::Inspire => "Inspire",
            CardVerb::Noop => "(no-op)",
        }
    }
}

/// Static definition of one card.  All fields are deterministic — no
/// floating-point magnitudes in v1; damage values are integer hp.
#[derive(Debug, Clone, Copy)]
pub struct CardDef {
    /// Stable card id (`CardId`).
    pub id: CardId,
    /// Display name.
    pub name: &'static str,
    /// Verb this card resolves into.
    pub verb: CardVerb,
    /// Source label (e.g. "Hull: Destroyer") used in tooltips.
    pub source: &'static str,
    /// One-line player-facing effect description.
    pub effect_text: &'static str,
    /// Doctrine bias label (e.g. "Militarist +2").  Plain text, not parsed.
    pub doctrine_bias: &'static str,
    /// Damage dealt by Strike/Salvo/Overcharge.  `0` for non-attack cards.
    pub base_damage: u32,
    /// Defense reduction provided by Guard/Fortify.  `0` for non-guard cards.
    pub base_defense: u32,
    /// Magnitude for Fortify (+50%), Evasive (50%), Mark (+25%), etc.
    /// Stored as percent × 100 (e.g. 50% = 50, 25% = 25).  `0` if unused.
    pub magnitude_pct: u32,
    /// Self-damage taken by Overcharge.  `0` for non-Overcharge cards.
    pub self_damage: u32,
    /// Optional faction signature owner (`EmpireDefinitionId` numeric value).
    /// `None` for cards that are not faction signatures.
    pub faction_owner: Option<u8>,
}

/// All v1 cards in stable `CardId`-ascending order.
///
/// The list is built deterministically and never mutated.  Adding new cards
/// requires both bumping the save schema and updating `docs/design/combat-v3.md`.
pub static CARD_REGISTRY: [CardDef; 23] = [
    CardDef {
        id: CardId::KINETIC_SALVO,
        name: "Kinetic Salvo",
        verb: CardVerb::Strike,
        source: "Hull: Escort Frigate / Missile Frigate / Destroyer / Patrol Corvette",
        effect_text: "Deal direct damage to the enemy fleet.",
        doctrine_bias: "Militarist +2",
        base_damage: 18,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::ABLATIVE_HULL,
        name: "Ablative Hull",
        verb: CardVerb::Guard,
        source: "Component: Reinforced Plating",
        effect_text: "Reduce incoming damage this round by your defense value.",
        doctrine_bias: "Isolationist +2",
        base_damage: 0,
        base_defense: 12,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::PHASED_SHIELD,
        name: "Phased Shield",
        verb: CardVerb::Guard,
        source: "Component: Shield Matrix",
        effect_text: "Guard this round plus absorb 1 hp on the next round.",
        doctrine_bias: "Isolationist +2",
        base_damage: 0,
        base_defense: 14,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::CIWS_GRID,
        name: "CIWS Grid",
        verb: CardVerb::Disrupt,
        source: "Component: Point Defense Grid",
        effect_text: "Cancel one enemy card queued for this round.",
        doctrine_bias: "—",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::BURN_MANEUVER,
        name: "Burn Maneuver",
        verb: CardVerb::Evasive,
        source: "Component: Ion Drive",
        effect_text: "Reduce incoming damage this round by 50%.",
        doctrine_bias: "Explorer +2",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 50,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::DRIFT_BURN,
        name: "Drift Burn",
        verb: CardVerb::Maneuver,
        source: "Hull: Scout / Fast Scout",
        effect_text: "Gain +1 initiative this round; your card resolves first.",
        doctrine_bias: "Explorer +2",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::TARGETING_LOCK,
        name: "Targeting Lock",
        verb: CardVerb::Mark,
        source: "Component: Targeting Suite",
        effect_text: "Your next Strike this battle deals +25% damage.",
        doctrine_bias: "Militarist +1",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 25,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::SENSOR_SWEEP,
        name: "Sensor Sweep",
        verb: CardVerb::Probe,
        source: "Component: Long-Range Sensors",
        effect_text: "Reveal the enemy hand (names, verbs, doctrine).",
        doctrine_bias: "Explorer +1",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::ORBITAL_BOMBARDMENT,
        name: "Orbital Bombardment",
        verb: CardVerb::Salvo,
        source: "Hull: Destroyer",
        effect_text: "Deal heavy damage that persists across remaining rounds.",
        doctrine_bias: "Militarist +3",
        base_damage: 12,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::DEFENSIVE_SCREEN,
        name: "Defensive Screen",
        verb: CardVerb::Fortify,
        source: "Hull: Escort Frigate / Patrol Corvette",
        effect_text: "Add 50% to your defense multiplier this round.",
        doctrine_bias: "Isolationist +1",
        base_damage: 0,
        base_defense: 8,
        magnitude_pct: 50,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::TROOP_DROP,
        name: "Troop Drop",
        verb: CardVerb::Bolster,
        source: "Component: Troop Bays / Hull: Troop Transport",
        effect_text: "Add invasion strength for the post-battle colony capture.",
        doctrine_bias: "Militarist +2",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::WARP_RETREAT,
        name: "Warp Retreat",
        verb: CardVerb::Withdraw,
        source: "Tech: Rapid Transit Drives",
        effect_text: "Auto-retreat, preserving 50% of current integrity.",
        doctrine_bias: "Explorer +2, Merchant +1",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 50,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::ORDNANCE_OVERCHARGE,
        name: "Ordnance Overcharge",
        verb: CardVerb::Overcharge,
        source: "Tech: Long-Range Strike Doctrine",
        effect_text: "Deal 28 hp damage, but take 1 hp self-damage.",
        doctrine_bias: "Militarist +2",
        base_damage: 28,
        base_defense: 0,
        magnitude_pct: 50,
        self_damage: 1,
        faction_owner: None,
    },
    CardDef {
        id: CardId::FORMATION_RALLY,
        name: "Formation Rally",
        verb: CardVerb::Inspire,
        source: "Tech: Battle Doctrine",
        effect_text: "Refill 1 hand slot with the top of your deck mid-battle.",
        doctrine_bias: "Unity +3, Militarist +1",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::SURVEYORS_GAMBIT,
        name: "Surveyor's Gambit",
        verb: CardVerb::Probe,
        source: "Hull: Science Vessel / Survey Cutter",
        effect_text: "Probe (reveal enemy hand) combined with Evasive (50% dmg cut).",
        doctrine_bias: "Explorer +3, Merchant +1",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 50,
        self_damage: 0,
        faction_owner: None,
    },
    CardDef {
        id: CardId::COERCIVE_MANDATE,
        name: "Coercive Mandate",
        verb: CardVerb::Strike,
        source: "Faction signature: Vorath Dominion",
        effect_text: "Strike plus bleed: enemy cards next round cost 1 hp each.",
        doctrine_bias: "Militarist",
        base_damage: 14,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: Some(4),
    },
    CardDef {
        id: CardId::SIEGE_DOCTRINE,
        name: "Siege Doctrine",
        verb: CardVerb::Strike,
        source: "Faction signature: Terran Dominion",
        effect_text: "Strike plus Bolster: damage plus +3 post-battle invasion.",
        doctrine_bias: "Militarist, Imperial",
        base_damage: 16,
        base_defense: 0,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: Some(7),
    },
    CardDef {
        id: CardId::INDUSTRIAL_JUGGERNAUT,
        name: "Industrial Juggernaut",
        verb: CardVerb::Guard,
        source: "Faction signature: Ashveran Compact",
        effect_text: "Guard plus end-of-round heal of 2 hp.",
        doctrine_bias: "Industrialist",
        base_damage: 0,
        base_defense: 14,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: Some(0),
    },
    CardDef {
        id: CardId::ALGORITHMIC_DEFENSE,
        name: "Algorithmic Defense",
        verb: CardVerb::Guard,
        source: "Faction signature: Elarith Confluence",
        effect_text: "Guard plus Probe: defend and reveal the enemy hand.",
        doctrine_bias: "Technologist",
        base_damage: 0,
        base_defense: 12,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: Some(5),
    },
    CardDef {
        id: CardId::PATHFINDERS_WAGER,
        name: "Pathfinder's Wager",
        verb: CardVerb::Evasive,
        source: "Faction signature: Luminal Traverse",
        effect_text: "Probe plus Evasive: reveal enemy and cut incoming 50%.",
        doctrine_bias: "Explorer",
        base_damage: 0,
        base_defense: 0,
        magnitude_pct: 50,
        self_damage: 0,
        faction_owner: Some(1),
    },
    CardDef {
        id: CardId::COUNCIL_OF_VOICES,
        name: "Council of Voices",
        verb: CardVerb::Inspire,
        source: "Faction signature: Terran Concord",
        effect_text: "Inspire plus Guard: refill 1 hand and reduce incoming damage.",
        doctrine_bias: "Unity, Explorer",
        base_damage: 0,
        base_defense: 6,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: Some(6),
    },
    CardDef {
        id: CardId::TRADE_BARGE_STAND,
        name: "Trade Barge Stand",
        verb: CardVerb::Guard,
        source: "Faction signature: Thalori Exchange",
        effect_text: "Cheap Guard with +25% defense bonus.",
        doctrine_bias: "Merchant",
        base_damage: 0,
        base_defense: 8,
        magnitude_pct: 25,
        self_damage: 0,
        faction_owner: Some(3),
    },
    CardDef {
        id: CardId::BLOOM_SHIELD,
        name: "Bloom Shield",
        verb: CardVerb::Guard,
        source: "Faction signature: Sylvaran Accord",
        effect_text: "Guard plus regen: heal 1 hp at round start for 2 rounds.",
        doctrine_bias: "Biologist",
        base_damage: 0,
        base_defense: 10,
        magnitude_pct: 0,
        self_damage: 0,
        faction_owner: Some(2),
    },
];

/// Hold Fire — the universal pad.  Not part of `CARD_REGISTRY` because
/// it is used as a filler, not as a discoverable card.
pub const HOLD_FIRE: CardDef = CardDef {
    id: CardId::HOLD_FIRE,
    name: "Hold Fire",
    verb: CardVerb::Noop,
    source: "Fallback pad",
    effect_text: "Burn the round. No damage taken or dealt.",
    doctrine_bias: "—",
    base_damage: 0,
    base_defense: 0,
    magnitude_pct: 0,
    self_damage: 0,
    faction_owner: None,
};

/// Look up a card by `CardId`.  Returns `Hold Fire` for unknown ids.
///
/// The function is linear in `CARD_REGISTRY.len()` (≤ 23).  Avoid
/// `HashMap` to keep the data flow deterministic.
pub fn card_by_id(id: CardId) -> &'static CardDef {
    if id == HOLD_FIRE.id {
        return &HOLD_FIRE;
    }
    for card in CARD_REGISTRY.iter() {
        if card.id == id {
            return card;
        }
    }
    &HOLD_FIRE
}

/// Signature card for a faction.  Returns `None` for unknown factions.
pub fn signature_for_faction(faction: u8) -> Option<CardId> {
    for card in CARD_REGISTRY.iter() {
        if card.faction_owner == Some(faction) {
            return Some(card.id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_twenty_three_unique_cards() {
        assert_eq!(CARD_REGISTRY.len(), 23);
    }

    #[test]
    fn registry_ids_are_unique_and_ascending() {
        let mut prev: u16 = 0;
        for (i, card) in CARD_REGISTRY.iter().enumerate() {
            assert_eq!(card.id.0 as usize, i + 1, "registry must start at 1");
            assert!(card.id.0 > prev);
            prev = card.id.0;
        }
    }

    #[test]
    fn hold_fire_is_distinct_from_pool() {
        assert_eq!(HOLD_FIRE.id.0, 0);
        for card in CARD_REGISTRY.iter() {
            assert_ne!(card.id, HOLD_FIRE.id);
        }
    }

    #[test]
    fn card_by_id_finds_pool_and_fallback() {
        assert_eq!(card_by_id(CardId::KINETIC_SALVO).name, "Kinetic Salvo");
        assert_eq!(card_by_id(CardId::BLOOM_SHIELD).name, "Bloom Shield");
        assert_eq!(card_by_id(CardId::HOLD_FIRE).name, "Hold Fire");
        assert_eq!(card_by_id(CardId(9999)).name, "Hold Fire");
    }

    #[test]
    fn all_cards_have_effect_and_doctrine_text() {
        for card in CARD_REGISTRY.iter() {
            assert!(!card.effect_text.is_empty(), "{} empty", card.name);
            assert!(!card.doctrine_bias.is_empty(), "{} empty", card.name);
            assert!(!card.source.is_empty(), "{} empty", card.name);
        }
    }

    #[test]
    fn signature_for_faction_resolves_each_faction() {
        // 8 factions, one signature card each.
        for faction in 0u8..=7 {
            let card_id = signature_for_faction(faction)
                .unwrap_or_else(|| panic!("missing signature for faction {faction}"));
            let card = card_by_id(card_id);
            assert_eq!(card.faction_owner, Some(faction));
        }
    }

    #[test]
    fn signature_for_unknown_faction_is_none() {
        assert!(signature_for_faction(99).is_none());
    }

    #[test]
    fn card_verb_labels_are_stable() {
        assert_eq!(CardVerb::Strike.label(), "Strike");
        assert_eq!(CardVerb::Noop.label(), "(no-op)");
        assert_eq!(CardVerb::Withdraw.label(), "Withdraw");
    }
}
