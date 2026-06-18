//! Deterministic AI card selection.
//!
//! AI picks the card from its hand with the highest weighted score:
//! `score = sum(card.doctrine_weight[d] * empire.doctrine_weights[d])`.
//! Ties broken by `CardId` ascending.  No RNG.

use super::card::{CardId, card_by_id};
use crate::state::{EmpireDefinition, EmpireId, empire_definition_by_id};
use std::collections::BTreeMap;

/// Pick a card for the AI to play this round.  Returns `None` if the
/// hand is empty.  Deterministic; no RNG.
pub fn ai_pick_card(hand: &[CardId], empire_def: Option<&EmpireDefinition>) -> Option<CardId> {
    if hand.is_empty() {
        return None;
    }
    let best = hand.iter().copied().max_by(|a, b| {
        let sa = score(*a, empire_def);
        let sb = score(*b, empire_def);
        sa.cmp(&sb).then(b.0.cmp(&a.0))
    });
    best
}

/// Score one card against the empire's doctrine weights.  Higher is
/// better.  Includes a small verb-bonus for the empire's preferred verbs
/// (computed from doctrine labels).
pub fn score(card: CardId, empire_def: Option<&EmpireDefinition>) -> i64 {
    let Some(def) = empire_def else {
        // No doctrine info — fall back to a flat tiebreak by id.
        return card.0 as i64;
    };
    let c = card_by_id(card);
    let mut s: i64 = 0;
    for entry in def.doctrine_weights {
        let w = entry.weight as i64;
        if w == 0 {
            continue;
        }
        let doctrine_label = entry.doctrine.label();
        if c.doctrine.contains(doctrine_label) {
            s += w * 10;
        }
    }
    // Bias by card id to keep ordering stable across equal scores.
    s += card.0 as i64;
    s
}

/// Find the empire definition for an empire id from a state slice.
pub fn empire_def_for(
    empires: &BTreeMap<EmpireId, crate::state::Empire>,
    empire_id: EmpireId,
) -> Option<&EmpireDefinition> {
    let empire = empires.get(&empire_id)?;
    let def_id = empire.empire_def?;
    empire_definition_by_id(def_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EmpireDefinitionId;

    #[test]
    fn empty_hand_returns_none() {
        let hand: Vec<CardId> = vec![];
        assert!(ai_pick_card(&hand, None).is_none());
    }

    #[test]
    fn single_card_returns_that_card() {
        let hand = vec![CardId(1)];
        assert_eq!(ai_pick_card(&hand, None), Some(CardId(1)));
    }

    #[test]
    fn militarist_prefers_strike_cards() {
        let def = empire_definition_by_id(EmpireDefinitionId(4)).unwrap(); // Vorath
        let hand = vec![CardId(2), CardId(9), CardId(12), CardId(5)];
        let pick = ai_pick_card(&hand, Some(def)).unwrap();
        // Vorath: Militarist (10) + Imperial (8). Orbital Bombardment (9)
        // is Militarist +3 → strongest score among these.
        assert_eq!(pick, CardId(9));
    }

    #[test]
    fn isolationist_prefers_guard_cards() {
        let def = empire_definition_by_id(EmpireDefinitionId(0)).unwrap(); // Ashveran
        let hand = vec![CardId(1), CardId(2), CardId(5), CardId(14)];
        let pick = ai_pick_card(&hand, Some(def)).unwrap();
        // Ashveran: Industrialist (9) + Isolationist (7). Ablative Hull (2)
        // is Isolationist +2 → strongest score.
        assert_eq!(pick, CardId(2));
    }

    #[test]
    fn ties_broken_by_card_id_ascending() {
        let hand = vec![CardId(5), CardId(1), CardId(3)];
        let pick = ai_pick_card(&hand, None).unwrap();
        // Without a doctrine profile all scores fall back to card id.
        // The max is CardId(5).
        assert_eq!(pick, CardId(5));
    }

    #[test]
    fn signature_card_picked_when_empire_matches() {
        let def = empire_definition_by_id(EmpireDefinitionId(4)).unwrap(); // Vorath
        let hand = vec![CardId(2), CardId(16), CardId(1)];
        let pick = ai_pick_card(&hand, Some(def)).unwrap();
        // Coercive Mandate (16) is Vorath signature, doctrine "Militarist".
        assert_eq!(pick, CardId(16));
    }
}
