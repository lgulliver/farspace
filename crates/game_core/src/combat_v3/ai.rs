//! Combat v3 — deterministic AI card picker.
//!
//! `ai_pick_card` returns a hand *index* (not a `CardId`) for the AI to
//! play this round.  No RNG is used; no lookahead is performed.  The
//! policy uses a single scoring function over the current hand and
//! resolves ties by `CardId` ascending then hand index ascending.

#[cfg(test)]
use crate::combat_v3::BattleSessionState;
use crate::combat_v3::card::{CardId, CardVerb, card_by_id};
use crate::combat_v3::{BattleSession, BattleSide, HAND_SIZE};

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

/// Pick a hand index for the AI to play this round.
///
/// Returns the smallest hand index whose card has the highest score; ties
/// are broken by `CardId` ascending.  The function panics only if the
/// hand is empty — that condition is unreachable at the call sites
/// (resolvers only call the picker when the hand has cards).
pub fn ai_pick_card(session: &BattleSession, side: BattleSide) -> usize {
    let hand: &[CardId] = match side {
        BattleSide::Attacker => &session.hand_a,
        BattleSide::Defender => &session.hand_b,
    };

    assert!(!hand.is_empty(), "ai_pick_card called on empty hand");

    let mut best_idx: usize = 0;
    let mut best_card: CardId = hand[0];
    let mut best_score: i32 = card_score(session, side, best_card);

    for (i, &card) in hand.iter().enumerate().skip(1) {
        let score = card_score(session, side, card);
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
    use crate::combat_v3::{BattleSession, BattleSetupSummary, BattleSide, HOLD_FIRE};
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
        }
    }

    #[test]
    fn ai_prefers_strike_when_enemy_is_weak() {
        let mut s = empty_session();
        s.integrity_b = 20;
        s.hand_a = vec![HOLD_FIRE.id, CardId::ABLATIVE_HULL, CardId::KINETIC_SALVO];
        let idx = ai_pick_card(&s, BattleSide::Attacker);
        assert_eq!(s.hand_a[idx], CardId::KINETIC_SALVO);
    }

    #[test]
    fn ai_prefers_guard_when_own_is_low() {
        let mut s = empty_session();
        s.integrity_a = 10;
        s.hand_a = vec![HOLD_FIRE.id, CardId::KINETIC_SALVO, CardId::ABLATIVE_HULL];
        let idx = ai_pick_card(&s, BattleSide::Attacker);
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
        let idx = ai_pick_card(&s, BattleSide::Attacker);
        assert_eq!(s.hand_a[idx], CardId::KINETIC_SALVO);
    }

    #[test]
    fn ai_does_not_pick_withdraw_unless_losing() {
        let mut s = empty_session();
        s.integrity_a = 90;
        s.hand_a = vec![CardId::WARP_RETREAT, CardId::KINETIC_SALVO];
        let idx = ai_pick_card(&s, BattleSide::Attacker);
        assert_eq!(s.hand_a[idx], CardId::KINETIC_SALVO);

        let mut s = empty_session();
        s.integrity_a = 20;
        s.hand_a = vec![CardId::WARP_RETREAT, CardId::KINETIC_SALVO];
        let idx = ai_pick_card(&s, BattleSide::Attacker);
        assert_eq!(s.hand_a[idx], CardId::WARP_RETREAT);
    }
}
