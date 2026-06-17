//! Card definitions and the v1 pool registry.
//!
//! 23 unique cards + 1 `Hold Fire` fallback.  All names and doctrine
//! strings are original; see `docs/design/combat-v3.md` for the catalogue.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Stable card identifier.  `0` is reserved for `HOLD_FIRE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CardId(pub u16);

/// The verb a card resolves into.  Drives presentation and (in a future
/// slice) damage formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CardVerb {
    Strike,
    Guard,
    Maneuver,
    Evasive,
    Salvo,
    Fortify,
    Disrupt,
    Probe,
    Mark,
    Overcharge,
    Withdraw,
    Bolster,
    Inspire,
    /// No-op.  Used for the `HOLD_FIRE` fallback.
    Noop,
}

impl CardVerb {
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

/// Where a card comes from.  Drives `build_hand` selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CardSource {
    Hull,
    Component,
    Tech,
    Faction,
    Fallback,
}

impl CardSource {
    pub fn label(self) -> &'static str {
        match self {
            CardSource::Hull => "Hull",
            CardSource::Component => "Component",
            CardSource::Tech => "Tech",
            CardSource::Faction => "Faction",
            CardSource::Fallback => "Fallback",
        }
    }
}

/// Static card definition.  Stored in the registry.  `name`, `effect`,
/// `target`, `synergies`, and `notes` are all original IP.
#[derive(Debug, Clone, Copy)]
pub struct CardDef {
    pub id: CardId,
    pub name: &'static str,
    pub verb: CardVerb,
    pub doctrine: &'static str,
    pub source: CardSource,
    pub effect: &'static str,
    pub target: &'static str,
    pub magnitude: Option<&'static str>,
    pub synergies: &'static str,
    pub notes: &'static str,
}

/// Alias for the card effect — currently just the static `effect` field of
/// `CardDef`.  Reserved for future expansion (multi-effect cards).
pub type CardEffect = ();

/// Last-resort filler card.  Always present in every hand as the final pad.
pub const HOLD_FIRE: CardDef = CardDef {
    id: CardId(0),
    name: "Hold Fire",
    verb: CardVerb::Noop,
    doctrine: "—",
    source: CardSource::Fallback,
    effect: "Burn the round. No damage taken or dealt.",
    target: "(none)",
    magnitude: None,
    synergies: "Filler when a hand cannot reach 5 cards.",
    notes: "Used as the universal pad for sub-5 hands.",
};

/// The v1 card pool.  Exactly 23 unique cards.
pub static POOL: &[CardDef] = &[
    CardDef {
        id: CardId(1),
        name: "Kinetic Salvo",
        verb: CardVerb::Strike,
        doctrine: "Militarist (+2)",
        source: CardSource::Hull,
        effect: "Deal direct damage to the enemy fleet.",
        target: "Enemy fleet",
        magnitude: Some("18 hp"),
        synergies: "Stacks with Mark (+25% dmg) and Salvo.",
        notes: "Baseline kinetic armament. Reliable, no drawbacks.",
    },
    CardDef {
        id: CardId(2),
        name: "Ablative Hull",
        verb: CardVerb::Guard,
        doctrine: "Isolationist (+2)",
        source: CardSource::Component,
        effect: "Reduce damage taken this round by your defense value.",
        target: "Self",
        magnitude: Some("def × 1.0"),
        synergies: "Stacks additively with Fortify and Evasive.",
        notes: "Passive defensive layer; always useful.",
    },
    CardDef {
        id: CardId(3),
        name: "Phased Shield",
        verb: CardVerb::Guard,
        doctrine: "Isolationist (+2)",
        source: CardSource::Component,
        effect: "Guard this round plus absorb up to 1 hp on the next round.",
        target: "Self",
        magnitude: Some("def × 1.0 + 1 hp buffer"),
        synergies: "Absorb persists into the next round if unused.",
        notes: "Trade plating for a delayed-damage buffer.",
    },
    CardDef {
        id: CardId(4),
        name: "CIWS Grid",
        verb: CardVerb::Disrupt,
        doctrine: "—",
        source: CardSource::Component,
        effect: "Cancel one enemy card queued for this round.",
        target: "Enemy queued card",
        magnitude: Some("1 card cancelled"),
        synergies: "Best used to nullify a Strike or Salvo.",
        notes: "Resolves before the enemy card's effect fires.",
    },
    CardDef {
        id: CardId(5),
        name: "Burn Maneuver",
        verb: CardVerb::Evasive,
        doctrine: "Explorer (+2)",
        source: CardSource::Component,
        effect: "Reduce incoming damage this round by 50%.",
        target: "Self",
        magnitude: Some("incoming × 0.5"),
        synergies: "Stacks with Guard and Fortify (multiplicative).",
        notes: "Best when enemy has a high-damage round queued.",
    },
    CardDef {
        id: CardId(6),
        name: "Drift Burn",
        verb: CardVerb::Maneuver,
        doctrine: "Explorer (+2)",
        source: CardSource::Hull,
        effect: "Gain +1 initiative this round; your card resolves first.",
        target: "Self",
        magnitude: Some("+1 initiative"),
        synergies: "Pairs with Disrupt or pre-emptive Guard.",
        notes: "Tempo card. Useful when initiative matters.",
    },
    CardDef {
        id: CardId(7),
        name: "Targeting Lock",
        verb: CardVerb::Mark,
        doctrine: "Militarist (+1)",
        source: CardSource::Component,
        effect: "Buff: your next Strike card this battle deals +25% damage.",
        target: "Self (queued buff)",
        magnitude: Some("+25% next Strike"),
        synergies: "Apply before Strike, Salvo, or Overcharge.",
        notes: "Buff persists until consumed or battle ends.",
    },
    CardDef {
        id: CardId(8),
        name: "Sensor Sweep",
        verb: CardVerb::Probe,
        doctrine: "Explorer (+1)",
        source: CardSource::Component,
        effect: "Reveal the enemy hand (names, verbs, doctrine).",
        target: "Enemy info",
        magnitude: Some("1 reveal"),
        synergies: "Stacks — repeat Probes reveal nothing new but log it.",
        notes: "Reveal persists for the rest of the battle.",
    },
    CardDef {
        id: CardId(9),
        name: "Orbital Bombardment",
        verb: CardVerb::Salvo,
        doctrine: "Militarist (+3)",
        source: CardSource::Hull,
        effect: "Deal heavy damage that persists across remaining rounds.",
        target: "Enemy fleet",
        magnitude: Some("24 hp (×atk all rounds)"),
        synergies: "Pairs with Mark (+25%) and Overcharge (×1.5).",
        notes: "Best opener against high-HP enemy fleets.",
    },
    CardDef {
        id: CardId(10),
        name: "Defensive Screen",
        verb: CardVerb::Fortify,
        doctrine: "Isolationist (+1)",
        source: CardSource::Hull,
        effect: "Add 50% to your defense multiplier this round.",
        target: "Self",
        magnitude: Some("def × 1.5"),
        synergies: "Stacks additively with Guard.",
        notes: "Bigger bump than Ablative Hull but limited to one round.",
    },
    CardDef {
        id: CardId(11),
        name: "Troop Drop",
        verb: CardVerb::Bolster,
        doctrine: "Militarist (+2)",
        source: CardSource::Component,
        effect: "Add invasion strength for the post-battle colony capture.",
        target: "Enemy colony (post-battle)",
        magnitude: Some("+5 invasion"),
        synergies: "Stacks with other Bolster cards in a multi-card hand.",
        notes: "Only applies if you win the engagement.",
    },
    CardDef {
        id: CardId(12),
        name: "Warp Retreat",
        verb: CardVerb::Withdraw,
        doctrine: "Explorer (+2), Merchant (+1)",
        source: CardSource::Tech,
        effect: "Auto-retreat, preserving 50% of current integrity.",
        target: "Self",
        magnitude: Some("retreat at 50% integrity"),
        synergies: "Counts as a card play; preserves your turn slot.",
        notes: "Use when outmatched; losing is worse than retreating.",
    },
    CardDef {
        id: CardId(13),
        name: "Ordnance Overcharge",
        verb: CardVerb::Overcharge,
        doctrine: "Militarist (+2)",
        source: CardSource::Tech,
        effect: "Deal 28 hp damage, but take 1 hp self-damage.",
        target: "Enemy fleet (self-damage)",
        magnitude: Some("28 hp enemy / 1 hp self"),
        synergies: "Combines with Mark for +25% (self-damage unchanged).",
        notes: "Ignores Evasive. High risk, high reward.",
    },
    CardDef {
        id: CardId(14),
        name: "Formation Rally",
        verb: CardVerb::Inspire,
        doctrine: "Unity (+3), Militarist (+1)",
        source: CardSource::Tech,
        effect: "Refill 1 hand slot with the top of your deck mid-battle.",
        target: "Self (hand)",
        magnitude: Some("+1 card"),
        synergies: "Refill is deterministic — no RNG.",
        notes: "Breaks the strict 5-round cap when used late.",
    },
    CardDef {
        id: CardId(15),
        name: "Surveyor's Gambit",
        verb: CardVerb::Probe,
        doctrine: "Explorer (+3), Merchant (+1)",
        source: CardSource::Hull,
        effect: "Probe (reveal enemy hand) combined with Evasive (50% dmg cut).",
        target: "Self + enemy info",
        magnitude: Some("Probe + Evasive"),
        synergies: "Best opener against unknown enemy loadouts.",
        notes: "Rare dual-effect card from survey hulls.",
    },
    CardDef {
        id: CardId(16),
        name: "Coercive Mandate",
        verb: CardVerb::Strike,
        doctrine: "Militarist",
        source: CardSource::Faction,
        effect: "Strike plus bleed: enemy cards next round cost 1 hp each.",
        target: "Enemy fleet",
        magnitude: Some("14 hp + 1 hp/round bleed"),
        synergies: "Pairs with multi-round enemy hands.",
        notes: "Vorath Dominion signature. Pressure over precision.",
    },
    CardDef {
        id: CardId(17),
        name: "Siege Doctrine",
        verb: CardVerb::Strike,
        doctrine: "Militarist + Imperial",
        source: CardSource::Faction,
        effect: "Strike plus Bolster: damage plus +3 post-battle invasion.",
        target: "Enemy fleet + post-battle colony",
        magnitude: Some("16 hp + +3 invasion"),
        synergies: "Combines damage and capture in one card.",
        notes: "Terran Dominion signature. Siege-oriented.",
    },
    CardDef {
        id: CardId(18),
        name: "Industrial Juggernaut",
        verb: CardVerb::Guard,
        doctrine: "Industrialist",
        source: CardSource::Faction,
        effect: "Guard plus end-of-round heal of 2 hp.",
        target: "Self",
        magnitude: Some("def × 1.0 + 2 hp heal"),
        synergies: "Heal persists across rounds until you stop guarding.",
        notes: "Ashveran Compact signature. Sustainable defense.",
    },
    CardDef {
        id: CardId(19),
        name: "Algorithmic Defense",
        verb: CardVerb::Guard,
        doctrine: "Technologist",
        source: CardSource::Faction,
        effect: "Guard plus Probe: defend and reveal the enemy hand.",
        target: "Self + enemy info",
        magnitude: Some("def × 1.0 + reveal"),
        synergies: "Defensive Probe. Information while you turtle.",
        notes: "Elarith Confluence signature. Intel + armor in one card.",
    },
    CardDef {
        id: CardId(20),
        name: "Pathfinder's Wager",
        verb: CardVerb::Evasive,
        doctrine: "Explorer",
        source: CardSource::Faction,
        effect: "Probe plus Evasive: reveal enemy and cut incoming 50%.",
        target: "Self + enemy info",
        magnitude: Some("Probe + incoming × 0.5"),
        synergies: "Offensive intel: see the threat and dodge half of it.",
        notes: "Luminal Traverse signature. Risky but rewarding opener.",
    },
    CardDef {
        id: CardId(21),
        name: "Council of Voices",
        verb: CardVerb::Inspire,
        doctrine: "Unity + Explorer",
        source: CardSource::Faction,
        effect: "Inspire plus Guard: refill 1 hand and reduce incoming dmg.",
        target: "Self (hand) + Self (defense)",
        magnitude: Some("+1 card + def × 0.5"),
        synergies: "Plays the rally and defense in one turn.",
        notes: "Terran Concord signature. Cooperative, low-aggression.",
    },
    CardDef {
        id: CardId(22),
        name: "Trade Barge Stand",
        verb: CardVerb::Guard,
        doctrine: "Merchant",
        source: CardSource::Faction,
        effect: "Cheap Guard with +25% defense bonus.",
        target: "Self",
        magnitude: Some("def × 1.25"),
        synergies: "Pure defense. Frees other cards for offense.",
        notes: "Thalori Exchange signature. Trade-route protection.",
    },
    CardDef {
        id: CardId(23),
        name: "Bloom Shield",
        verb: CardVerb::Guard,
        doctrine: "Biologist",
        source: CardSource::Faction,
        effect: "Guard plus regen: heal 1 hp at round start for 2 rounds.",
        target: "Self",
        magnitude: Some("def × 1.0 + 1 hp/round × 2"),
        synergies: "Sustained healing pairs with longer battles.",
        notes: "Sylvaran Accord signature. Slow but persistent recovery.",
    },
];

/// Look up a card by id.  Returns `HOLD_FIRE` for unknown ids.
pub fn card_by_id(id: CardId) -> &'static CardDef {
    if id == HOLD_FIRE.id {
        return &HOLD_FIRE;
    }
    POOL.iter().find(|c| c.id == id).unwrap_or(&HOLD_FIRE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_has_expected_count() {
        assert_eq!(POOL.len(), 23, "v1 pool must hold 23 unique cards");
    }

    #[test]
    fn pool_ids_are_unique() {
        let mut ids: Vec<u16> = POOL.iter().map(|c| c.id.0).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), POOL.len(), "card IDs must be unique");
    }

    #[test]
    fn hold_fire_is_distinct_from_pool() {
        assert_eq!(HOLD_FIRE.id, CardId(0));
        assert!(POOL.iter().all(|c| c.id != HOLD_FIRE.id));
    }

    #[test]
    fn card_by_id_finds_pool_and_fallback() {
        assert_eq!(card_by_id(CardId(1)).name, "Kinetic Salvo");
        assert_eq!(card_by_id(CardId(23)).name, "Bloom Shield");
        assert_eq!(card_by_id(CardId(0)).name, "Hold Fire");
        assert_eq!(card_by_id(CardId(999)).name, "Hold Fire");
    }

    #[test]
    fn damage_cards_have_magnitudes() {
        for card in POOL {
            if matches!(
                card.verb,
                CardVerb::Strike
                    | CardVerb::Salvo
                    | CardVerb::Overcharge
                    | CardVerb::Bolster
                    | CardVerb::Withdraw
            ) {
                assert!(
                    card.magnitude.is_some(),
                    "{} (verb {}) should have a magnitude",
                    card.name,
                    card.verb.label()
                );
            }
        }
    }

    #[test]
    fn all_pool_cards_have_text() {
        for card in POOL {
            assert!(!card.effect.is_empty(), "{} missing effect", card.name);
            assert!(!card.target.is_empty(), "{} missing target", card.name);
            assert!(
                !card.synergies.is_empty(),
                "{} missing synergies",
                card.name
            );
            assert!(!card.notes.is_empty(), "{} missing notes", card.name);
        }
    }
}
