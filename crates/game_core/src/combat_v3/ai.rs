//! Combat v3 — deterministic AI card picker.
//!
//! `ai_pick_card` returns a hand *index* (not a `CardId`) for the AI to
//! play this round.  No RNG is used; no lookahead is performed.  The
//! policy uses a single scoring function over the current hand and
//! resolves ties by `CardId` ascending then hand index ascending.
//!
//! When the playing side has an `EmpireDefinition` available, the
//! score is augmented by the card's `doctrine_bias` parsed against
//! the empire's `doctrine_weights` and `playstyle` tags.  This makes
//! the AI's behaviour vary by faction while remaining fully
//! deterministic.

#[cfg(test)]
use crate::combat_v3::BattleSessionState;
use crate::combat_v3::card::{CardId, CardVerb, card_by_id};
use crate::combat_v3::{BattleSession, BattleSide, HAND_SIZE};
use crate::state::{AiDoctrine, EmpireDefinition, PlaystyleTag};

/// Score for a given card on a given side, given the current session
/// state.  Higher scores are preferred.  Deterministic.
fn card_score(session: &BattleSession, side: BattleSide, card: CardId) -> i32 {
    let (own_int, enemy_int) = match side {
        BattleSide::Attacker => (session.integrity_a, session.integrity_b),
        BattleSide::Defender => (session.integrity_b, session.integrity_a),
    };

    let def = card_by_id(card);
    let mut score: i32 = 0;

    // 1. Pure-pressure bias: prefer damage cards when enemy is weak.
    if enemy_int <= 30 {
        match def.verb {
            CardVerb::Strike | CardVerb::Salvo | CardVerb::Overcharge => {
                score = score.saturating_add(20);
            }
            _ => {}
        }
    }

    // 2. Survival bias: prefer defense/evasive when own integrity is low.
    if own_int <= 40 {
        match def.verb {
            CardVerb::Guard | CardVerb::Fortify | CardVerb::Evasive => {
                score = score.saturating_add(15);
            }
            _ => {}
        }
    }

    // 3. Verb-shape priors (mirrors the doctrine buckets in the design doc).
    match def.verb {
        CardVerb::Strike => score = score.saturating_add(6),
        CardVerb::Salvo => score = score.saturating_add(8),
        CardVerb::Overcharge => score = score.saturating_add(7),
        CardVerb::Mark => score = score.saturating_add(4),
        CardVerb::Bolster => score = score.saturating_add(2),
        CardVerb::Guard | CardVerb::Fortify => score = score.saturating_add(3),
        CardVerb::Evasive => score = score.saturating_add(4),
        CardVerb::Maneuver => score = score.saturating_add(2),
        CardVerb::Disrupt => score = score.saturating_add(5),
        CardVerb::Probe => score = score.saturating_add(2),
        CardVerb::Inspire => score = score.saturating_add(3),
        CardVerb::Withdraw => {
            // Withdraw only if losing badly — never as a first play.
            if own_int <= 25 {
                score = score.saturating_add(10);
            }
        }
        CardVerb::Noop => {
            // Hold Fire should never be picked first if anything else is
            // available; score it last.
            score = score.saturating_sub(50);
        }
    }

    score
}

/// Parse a `doctrine_bias` string of the form
/// `"Militarist +2, Merchant +1"` into `(label, weight)` pairs.
///
/// Unrecognised labels (e.g. the "Unity" placeholder) are silently
/// dropped.  Returns an empty `Vec` for the neutral bias `"—"` or
/// any other non-matching input.  The function never panics.
fn parse_doctrine_bias(s: &str) -> Vec<(&str, i32)> {
    let mut out = Vec::new();
    for chunk in s.split(',') {
        let chunk = chunk.trim();
        if chunk.is_empty() || chunk == "—" {
            continue;
        }
        // Expect "<Label> +<n>" or "<Label> -<n>".
        let mut parts = chunk.split_whitespace();
        let label = parts.next().unwrap_or("");
        let sign = parts.next().unwrap_or("");
        let n: i32 = match sign {
            "+1" => 1,
            "+2" => 2,
            "+3" => 3,
            "+4" => 4,
            "+5" => 5,
            "-1" => -1,
            "-2" => -2,
            "-3" => -3,
            _ => continue,
        };
        out.push((label, n));
    }
    out
}

/// Compute the doctrine-derived score contribution for a card given
/// an empire definition.  Sums the bias weights for each label that
/// matches an `AiDoctrine` the empire weights, plus `1` per matching
/// `PlaystyleTag` occurrence.  `0` if the empire has no definition.
fn doctrine_bonus(def: &EmpireDefinition, card: CardId) -> i32 {
    let bias = parse_doctrine_bias(card_by_id(card).doctrine_bias);
    if bias.is_empty() {
        return 0;
    }
    let mut bonus: i32 = 0;
    for (label, w) in bias {
        // Doctrine and playstyle are independent signals.  A label
        // like "Militarist" matches BOTH an AiDoctrine variant and
        // a PlaystyleTag variant — both contributions are folded
        // into the score.  This matters for empires whose
        // `playstyle` includes a tag that doubles as an
        // `AiDoctrine::label()` (Militarist, Expansionist).
        if let Some(d) = doctrine_from_label(label) {
            let weight = def.doctrine_weight(d) as i32;
            // Bias "Militarist +2" against a Militarist-3 empire
            // contributes 2 × 3 = 6.
            bonus = bonus.saturating_add(w.saturating_mul(weight));
        }
        if playstyle_from_label(label).is_some() && def.playstyle.iter().any(|t| t.label() == label)
        {
            // Playstyle match: a flat contribution equal to the
            // bias weight.  Stacks on top of any doctrine match.
            bonus = bonus.saturating_add(w);
        }
        // Unrecognised labels (e.g. "Unity") silently dropped.
    }
    bonus
}

/// Map a label string back to an `AiDoctrine` variant.  Returns
/// `None` for any label that does not match.
fn doctrine_from_label(label: &str) -> Option<AiDoctrine> {
    Some(match label {
        "Explorer" => AiDoctrine::Explorer,
        "Technologist" => AiDoctrine::Technologist,
        "Merchant" => AiDoctrine::Merchant,
        "Imperial" => AiDoctrine::Imperial,
        "Militarist" => AiDoctrine::Militarist,
        "Industrialist" => AiDoctrine::Industrialist,
        "Expansionist" => AiDoctrine::Expansionist,
        "Isolationist" => AiDoctrine::Isolationist,
        "Biologist" => AiDoctrine::Biologist,
        _ => return None,
    })
}

/// Map a label string back to a `PlaystyleTag` variant.  Returns
/// `None` for any label that does not match.
fn playstyle_from_label(label: &str) -> Option<PlaystyleTag> {
    Some(match label {
        "Industrial" => PlaystyleTag::Industrial,
        "Scientific" => PlaystyleTag::Scientific,
        "Expansionist" => PlaystyleTag::Expansionist,
        "Militarist" => PlaystyleTag::Militarist,
        "Agrarian" => PlaystyleTag::Agrarian,
        "Diplomatic" => PlaystyleTag::Diplomatic,
        _ => return None,
    })
}

/// Pick a hand index for the AI to play this round, with optional
/// doctrine awareness.
///
/// Returns the smallest hand index whose card has the highest score;
/// ties are broken by `CardId` ascending.  The function panics only if
/// the hand is empty — that condition is unreachable at the call
/// sites (resolvers only call the picker when the hand has cards).
///
/// `empire_def` is the empire definition of the playing side.  When
/// `Some`, the card's `doctrine_bias` is folded into the score; when
/// `None`, scoring is the legacy "verb-only" baseline.
pub fn ai_pick_card(
    session: &BattleSession,
    side: BattleSide,
    empire_def: Option<&EmpireDefinition>,
) -> usize {
    let hand: &[CardId] = match side {
        BattleSide::Attacker => &session.hand_a,
        BattleSide::Defender => &session.hand_b,
    };

    assert!(!hand.is_empty(), "ai_pick_card called on empty hand");

    let mut best_idx: usize = 0;
    let mut best_card: CardId = hand[0];
    let mut best_score: i32 = match empire_def {
        Some(def) => {
            card_score(session, side, best_card).saturating_add(doctrine_bonus(def, best_card))
        }
        None => card_score(session, side, best_card),
    };

    for (i, &card) in hand.iter().enumerate().skip(1) {
        let score = match empire_def {
            Some(def) => card_score(session, side, card).saturating_add(doctrine_bonus(def, card)),
            None => card_score(session, side, card),
        };
        let better = score > best_score
            || (score == best_score
                && (card.0 < best_card.0 || (card.0 == best_card.0 && i < best_idx)));
        if better {
            best_idx = i;
            best_card = card;
            best_score = score;
        }
    }

    // Defensive: clamp into [0, HAND_SIZE).
    best_idx.min(HAND_SIZE - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_v3::{BattleSession, BattleSetupSummary, HOLD_FIRE};
    use crate::state::{
        EmpireId, FleetFormation, FleetId, FleetKind, FleetRole, FleetSupplyState, StarId,
    };

    fn empty_session() -> BattleSession {
        BattleSession {
            session_id: 1,
            star: StarId(1),
            attacker: FleetId(1),
            defender: FleetId(2),
            empire_a: EmpireId(1),
            empire_b: EmpireId(2),
            hand_a: vec![],
            hand_b: vec![],
            integrity_a: 100,
            integrity_b: 100,
            integrity_a_start: 100,
            integrity_b_start: 100,
            round: 1,
            rounds: Vec::new(),
            setup_summary: BattleSetupSummary {
                role_a: FleetRole::StrikeFleet,
                role_b: FleetRole::DefenseFleet,
                formation_a: FleetFormation::Balanced,
                formation_b: FleetFormation::Defensive,
                doctrine_a: String::new(),
                doctrine_b: String::new(),
                supply_a: FleetSupplyState::Supplied,
                supply_b: FleetSupplyState::Supplied,
                kind_a: FleetKind::Destroyer,
                kind_b: FleetKind::EscortFrigate,
                ships_a: 1,
                ships_b: 1,
            },
            state: BattleSessionState::AwaitingPlayer,
            mark_a_pending: false,
            mark_b_pending: false,
            salvo_a_recurring: 0,
            salvo_b_recurring: 0,
        }
    }

    #[test]
    fn ai_prefers_strike_when_enemy_is_weak() {
        let mut s = empty_session();
        s.integrity_b = 20;
        s.hand_a = vec![HOLD_FIRE.id, CardId::ABLATIVE_HULL, CardId::KINETIC_SALVO];
        let idx = ai_pick_card(&s, BattleSide::Attacker, None);
        assert_eq!(s.hand_a[idx], CardId::KINETIC_SALVO);
    }

    #[test]
    fn ai_prefers_guard_when_own_is_low() {
        let mut s = empty_session();
        s.integrity_a = 10;
        s.hand_a = vec![HOLD_FIRE.id, CardId::KINETIC_SALVO, CardId::ABLATIVE_HULL];
        let idx = ai_pick_card(&s, BattleSide::Attacker, None);
        assert_eq!(s.hand_a[idx], CardId::ABLATIVE_HULL);
    }

    #[test]
    fn ai_picks_lowest_card_id_on_tie() {
        let mut s = empty_session();
        s.integrity_a = 100;
        s.integrity_b = 100;
        // Two Strike cards with no weak-side boost and no other
        // distinguishing feature: the lower CardId must win.
        s.hand_a = vec![CardId::KINETIC_SALVO, CardId::COERCIVE_MANDATE];
        let idx = ai_pick_card(&s, BattleSide::Attacker, None);
        assert_eq!(s.hand_a[idx], CardId::KINETIC_SALVO);
    }

    #[test]
    fn ai_does_not_pick_withdraw_unless_losing() {
        let mut s = empty_session();
        s.integrity_a = 90;
        s.hand_a = vec![CardId::WARP_RETREAT, CardId::KINETIC_SALVO];
        let idx = ai_pick_card(&s, BattleSide::Attacker, None);
        assert_eq!(s.hand_a[idx], CardId::KINETIC_SALVO);

        let mut s = empty_session();
        s.integrity_a = 20;
        s.hand_a = vec![CardId::WARP_RETREAT, CardId::KINETIC_SALVO];
        let idx = ai_pick_card(&s, BattleSide::Attacker, None);
        assert_eq!(s.hand_a[idx], CardId::WARP_RETREAT);
    }

    #[test]
    fn parse_doctrine_bias_handles_compound_strings() {
        assert!(parse_doctrine_bias("—").is_empty());
        assert!(parse_doctrine_bias("").is_empty());
        assert_eq!(
            parse_doctrine_bias("Militarist +2"),
            vec![("Militarist", 2)]
        );
        assert_eq!(
            parse_doctrine_bias("Explorer +2, Merchant +1"),
            vec![("Explorer", 2), ("Merchant", 1)]
        );
        assert_eq!(
            parse_doctrine_bias("Unity +3, Militarist +1"),
            vec![("Unity", 3), ("Militarist", 1)]
        );
    }

    #[test]
    fn doctrine_and_playstyle_both_contribute_for_overlapping_labels() {
        // Regression: "Militarist" matches BOTH an AiDoctrine
        // variant and a PlaystyleTag variant.  An empire that
        // weights Militarist doctrine AND lists Militarist in its
        // playstyle should receive BOTH contributions stacked.
        // The previous `else if` formulation dropped the playstyle
        // contribution when the doctrine branch matched.
        //
        // We exercise the public `ai_pick_card` with two hands:
        // the first contains only an Isolationist-biased card
        // (HandIsolation = Ablative Hull), the second contains
        // only a Militarist-biased card (Orbital Bombardment).
        // For an empire that is both Militarist and Isolationist,
        // the relative scores between the two hands must reflect
        // the doctrine bias, not just the verb score.

        // Terran Dominion (faction 7) is the empire with both
        // Militarist doctrine and Militarist playstyle.  Confirm
        // that picking a Militarist-biased card from a hand that
        // is otherwise tied with an Isolationist-biased card
        // yields the Militarist card.
        let terran = crate::empire_definition_by_id(crate::EmpireDefinitionId(7))
            .expect("Terran Dominion definition");
        let hand = vec![
            CardId::ORBITAL_BOMBARDMENT, // Militarist +3
            CardId::ABLATIVE_HULL,       // Isolationist +2
            HOLD_FIRE.id,
            HOLD_FIRE.id,
            HOLD_FIRE.id,
        ];
        let s = {
            let mut s = empty_session();
            s.hand_a = hand;
            s
        };
        let pick = ai_pick_card(&s, BattleSide::Attacker, Some(terran));
        assert_eq!(
            s.hand_a[pick],
            CardId::ORBITAL_BOMBARDMENT,
            "Terran Dominion AI must prefer the Militarist-biased card (both doctrine and playstyle match)"
        );
    }
}
