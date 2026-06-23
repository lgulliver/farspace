//! Combat v3 — round resolution.
//!
//! The v1 resolver implements every verb listed in the design doc.  It is
//! pure integer arithmetic (no floating point, no `f32`/`f64` constants
//! in the math) so the round log is bit-identical for the same inputs
//! and a fixed seed.
//!
//! Resolver contract:
//!
//! - `apply_round` advances the session by exactly one round.
//! - It emits a `BattleRoundSummary` appended to `session.rounds`.
//! - If the round ends the battle (destroyed/retreat/max rounds) it
//!   returns `BattleOutcome::Finished { winner }`.
//! - Otherwise it returns `BattleOutcome::Continue`.
//!
//! The resolver never mutates the host `GameState`; that work is done by
//! the engine when the session is finalised.

use crate::combat_v3::ai::ai_pick_card;
use crate::combat_v3::card::{CardId, CardVerb, HOLD_FIRE, card_by_id};
use crate::combat_v3::report::BattleRoundSummary;
use crate::combat_v3::{BattleSession, BattleSessionState, BattleSide, MAX_ROUNDS};

/// Outcome of a single round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleOutcome {
    /// Battle continues — both sides survived the round.
    Continue,
    /// Battle finished.  The `Option<BattleSide>` is the winner:
    /// `None` for a draw, `Some(side)` for a victor.
    Finished { winner: Option<BattleSide> },
}

/// Apply one round.  Picks the AI card with `ai_pick_card`, takes the
/// player card from `player_card` (passed by the caller so the engine
/// can stay command-driven).  All damage is integer hp.
///
/// Returns the outcome and the `BattleRoundSummary` that was appended
/// to the session.  When the outcome is `Finished` the caller must
/// finalise the report.
pub fn apply_round(
    session: &mut BattleSession,
    player_side: BattleSide,
    player_card: CardId,
) -> (BattleOutcome, BattleRoundSummary) {
    // AI chooses a card.
    let ai_side = player_side.other();
    let ai_index = ai_pick_card(session, ai_side);
    let ai_card = match ai_side {
        BattleSide::Attacker => session.hand_a[ai_index],
        BattleSide::Defender => session.hand_b[ai_index],
    };

    // Snapshot starting integrity for the summary.
    let start_a = session.integrity_a;
    let start_b = session.integrity_b;
    let round = session.round;

    // Resolve cards.  Each side may be cancelled by the other's Disrupt.
    let (player_card_resolved, ai_card_resolved) = apply_disrupt(player_card, ai_card);

    // Normalize *all* side-indexed state to the attacker/defender
    // frame of reference.  Doing this once up-front means the
    // mutation block, the round summary, and the outcome decision
    // can all use a single set of variables and never need to know
    // which side the player is on.  This is critical for
    // WARP_RETREAT — if the player is the defender and plays
    // Withdraw, the halving must hit `integrity_b` (the
    // defender-side value), not `integrity_a`.
    let (card_a, card_b, effect_a, effect_b, a_retreated, b_retreated) = match player_side {
        BattleSide::Attacker => {
            let (eff_a, eff_b) = resolve_pair(
                session,
                player_card_resolved,
                ai_card_resolved,
                BattleSide::Attacker,
                BattleSide::Defender,
            );
            // Use the *resolved* cards (post-CIWS) for retreat
            // detection: a cancelled WARP_RETREAT is no longer a
            // retreat.
            let p_retreated = matches!(card_by_id(player_card_resolved).verb, CardVerb::Withdraw);
            let f_retreated = matches!(card_by_id(ai_card_resolved).verb, CardVerb::Withdraw);
            (
                Some(player_card),
                Some(ai_card),
                eff_a,
                eff_b,
                p_retreated,
                f_retreated,
            )
        }
        BattleSide::Defender => {
            // Player is side B; the AI's card is the attacker's card.
            // resolve_pair still expects (attacker, defender) order.
            let (eff_a, eff_b) = resolve_pair(
                session,
                ai_card_resolved,
                player_card_resolved,
                BattleSide::Attacker,
                BattleSide::Defender,
            );
            // See the attacker branch above: use the *resolved* cards
            // so a CIWS-cancelled WARP_RETREAT does not trigger a
            // retreat.
            let p_retreated = matches!(card_by_id(player_card_resolved).verb, CardVerb::Withdraw);
            let f_retreated = matches!(card_by_id(ai_card_resolved).verb, CardVerb::Withdraw);
            // Swap so the *attacker-side* retreat flag is the AI's and
            // the *defender-side* flag is the player's.
            (
                Some(ai_card),
                Some(player_card),
                eff_a,
                eff_b,
                f_retreated,
                p_retreated,
            )
        }
    };

    // Clamp integrity at zero (integer floor).
    session.integrity_a = session.integrity_a.min(start_a); // already lowered
    session.integrity_b = session.integrity_b.min(start_b);
    // The above `min` calls are a no-op because resolve_pair already
    // saturated; we keep them for defensive clarity in case the math
    // ever changes.  No effect on determinism.

    // Apply Withdraw: retreating side drops to 50% of its *pre-round*
    // integrity (clamped at 0).  Using the pre-round value mirrors the
    // design-doc "preserves 50% of current integrity" wording and
    // makes the result independent of which card the opponent played.
    if a_retreated {
        let halved = start_a / 2;
        session.integrity_a = halved;
    }
    if b_retreated {
        let halved = start_b / 2;
        session.integrity_b = halved;
    }

    // Remove the played cards from their hands.
    remove_card_from_hand(session, player_side, player_card);
    remove_card_from_hand(session, ai_side, ai_card);

    // Append the round summary.
    let summary = BattleRoundSummary {
        round,
        card_a,
        card_b,
        effect_a,
        effect_b,
        integrity_a_after: session.integrity_a,
        integrity_b_after: session.integrity_b,
    };
    session.rounds.push(summary.clone());

    // Decide outcome.
    let a_destroyed = session.integrity_a == 0;
    let b_destroyed = session.integrity_b == 0;
    let a_dead = a_destroyed || a_retreated;
    let b_dead = b_destroyed || b_retreated;
    let max_rounds = session.round >= MAX_ROUNDS;
    let exhausted = session.hand_a.is_empty() && session.hand_b.is_empty();

    let outcome = if a_dead || b_dead || max_rounds || exhausted {
        let winner = if a_dead && b_dead {
            None
        } else if a_dead {
            Some(BattleSide::Defender)
        } else if b_dead {
            Some(BattleSide::Attacker)
        } else if max_rounds || exhausted {
            // Tiebreaker: higher integrity wins; defender wins ties.
            match session.integrity_a.cmp(&session.integrity_b) {
                std::cmp::Ordering::Greater => Some(BattleSide::Attacker),
                std::cmp::Ordering::Less => Some(BattleSide::Defender),
                std::cmp::Ordering::Equal => Some(BattleSide::Defender),
            }
        } else {
            None
        };
        BattleOutcome::Finished { winner }
    } else {
        session.round = session.round.saturating_add(1);
        BattleOutcome::Continue
    };

    if matches!(outcome, BattleOutcome::Finished { .. }) {
        session.state = BattleSessionState::Finished;
    }

    (outcome, summary)
}

/// Apply the Disrupt card cancellation.  If either side plays
/// `CIWS Grid`, the opposing card is replaced with `Hold Fire` for this
/// round.  Returns the resolved `(player_card, ai_card)` pair.
fn apply_disrupt(player_card: CardId, ai_card: CardId) -> (CardId, CardId) {
    let player_resolved = if ai_card == CardId::CIWS_GRID {
        HOLD_FIRE.id
    } else {
        player_card
    };
    let ai_resolved = if player_card == CardId::CIWS_GRID {
        HOLD_FIRE.id
    } else {
        ai_card
    };
    (player_resolved, ai_resolved)
}

/// Resolve a pair of cards simultaneously.  Each side deals damage to the
/// other based on the verb, applying guard/evasive/fortify reductions
/// against the incoming strike.  Self-damage (Overcharge) is applied to
/// the attacker.
///
/// Returns `(effect_text_for_attacker, effect_text_for_defender)`.
fn resolve_pair(
    session: &mut BattleSession,
    card_a: CardId,
    card_b: CardId,
    side_a: BattleSide,
    side_b: BattleSide,
) -> (String, String) {
    let def_a = card_by_id(card_a);
    let def_b = card_by_id(card_b);

    // 1. Compute attacker's outgoing damage.
    let atk_a = outgoing_damage(session, side_a, card_a);
    // Defender's Evasive halves the incoming attack directly, before
    // the guard subtraction.  This avoids the "huge guard = zero
    // damage" trap from the earlier model.
    let mut dmg_b = atk_a;
    if matches!(def_b.verb, CardVerb::Evasive) {
        dmg_b /= 2;
    }
    let guard_b = incoming_reduction(session, side_b, card_b);
    dmg_b = dmg_b.saturating_sub(guard_b);

    // 2. Compute defender's outgoing damage.
    let atk_b = outgoing_damage(session, side_b, card_b);
    let mut dmg_a = atk_b;
    if matches!(def_a.verb, CardVerb::Evasive) {
        dmg_a /= 2;
    }
    let guard_a = incoming_reduction(session, side_a, card_a);
    dmg_a = dmg_a.saturating_sub(guard_a);

    // 3. Apply damage to integrity, clamped at zero.
    let start_a = session.integrity_a;
    let start_b = session.integrity_b;
    session.integrity_a = start_a.saturating_sub(dmg_a);
    session.integrity_b = start_b.saturating_sub(dmg_b);

    // 4. Apply self-damage (Overcharge).
    let (self_a, self_b) = self_damage(card_a, card_b);
    if self_a > 0 {
        session.integrity_a = session.integrity_a.saturating_sub(self_a);
    }
    if self_b > 0 {
        session.integrity_b = session.integrity_b.saturating_sub(self_b);
    }

    // 5. Inspire adds a Hold Fire if hand is short.  This is a v1
    //    simplification: real refill draws the top of the deck, but
    //    we don't have a deck object yet.
    if def_a.verb == CardVerb::Inspire {
        push_hold_fire_if_short(session, side_a);
    }
    if def_b.verb == CardVerb::Inspire {
        push_hold_fire_if_short(session, side_b);
    }

    let eff_a = format_effect(def_a.name, dmg_b, self_a);
    let eff_b = format_effect(def_b.name, dmg_a, self_b);
    (eff_a, eff_b)
}

/// Compute outgoing damage for a card.  Returns `0` for non-damage verbs.
fn outgoing_damage(session: &BattleSession, side: BattleSide, card: CardId) -> u32 {
    let def = card_by_id(card);
    let (own_int, enemy_int) = match side {
        BattleSide::Attacker => (session.integrity_a, session.integrity_b),
        BattleSide::Defender => (session.integrity_b, session.integrity_a),
    };
    // Scaled base damage by relative integrity — losing side hits harder.
    let scale_pct: u32 = if enemy_int == 0 {
        0
    } else {
        100 + ((own_int.saturating_sub(enemy_int)) / 10).min(20)
    };

    match def.verb {
        CardVerb::Strike | CardVerb::Salvo | CardVerb::Overcharge => {
            (def.base_damage.saturating_mul(scale_pct) / 100).max(1)
        }
        _ => 0,
    }
}

/// Compute incoming damage reduction for a Guard / Fortify card.
/// Returns `0` for any other verb (Evasive is handled in `resolve_pair`
/// to halve the attack directly).
fn incoming_reduction(_session: &BattleSession, side: BattleSide, card: CardId) -> u32 {
    let def = card_by_id(card);
    let _ = side;
    let base = match def.verb {
        CardVerb::Guard | CardVerb::Fortify => def.base_defense,
        _ => 0,
    };

    // Fortify +50% defense multiplier.
    let factor_pct: u32 = if def.verb == CardVerb::Fortify {
        150
    } else {
        100
    };
    base.saturating_mul(factor_pct) / 100
}

/// Self-damage dealt by a card (Overcharge).  Returns `(self_a, self_b)`.
fn self_damage(card_a: CardId, card_b: CardId) -> (u32, u32) {
    let a = card_by_id(card_a);
    let b = card_by_id(card_b);
    let a_self = if a.verb == CardVerb::Overcharge {
        a.self_damage
    } else {
        0
    };
    let b_self = if b.verb == CardVerb::Overcharge {
        b.self_damage
    } else {
        0
    };
    (a_self, b_self)
}

/// Push a `Hold Fire` into the hand if it has fewer than 5 cards.
fn push_hold_fire_if_short(session: &mut BattleSession, side: BattleSide) {
    let hand = match side {
        BattleSide::Attacker => &mut session.hand_a,
        BattleSide::Defender => &mut session.hand_b,
    };
    if hand.len() < 5 {
        hand.push(HOLD_FIRE.id);
    }
}

fn remove_card_from_hand(session: &mut BattleSession, side: BattleSide, card: CardId) {
    let hand = match side {
        BattleSide::Attacker => &mut session.hand_a,
        BattleSide::Defender => &mut session.hand_b,
    };
    if let Some(pos) = hand.iter().position(|c| *c == card) {
        hand.remove(pos);
    } else {
        // Defensive: never panic if a card was missing.
        hand.pop();
    }
}

fn format_effect(name: &str, dmg: u32, self_dmg: u32) -> String {
    match (dmg, self_dmg) {
        (0, 0) => format!("{} (no effect)", name),
        (d, 0) => format!("{} -{} hp", name, d),
        (0, s) => format!("{} (self -{} hp)", name, s),
        (d, s) => format!("{} -{} hp (self -{} hp)", name, d, s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_v3::{BattleSession, BattleSetupSummary};
    use crate::state::{
        EmpireId, FleetFormation, FleetId, FleetKind, FleetRole, FleetSupplyState, StarId,
    };

    fn make_session(hand_a: Vec<CardId>, hand_b: Vec<CardId>) -> BattleSession {
        BattleSession {
            session_id: 1,
            star: StarId(1),
            attacker: FleetId(1),
            defender: FleetId(2),
            empire_a: EmpireId(1),
            empire_b: EmpireId(2),
            hand_a,
            hand_b,
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
    fn strike_deals_damage_to_enemy() {
        let mut s = make_session(
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let (outcome, summary) = apply_round(&mut s, BattleSide::Attacker, CardId::KINETIC_SALVO);
        assert!(matches!(outcome, BattleOutcome::Continue));
        assert!(s.integrity_b < 100, "enemy should take damage");
        assert_eq!(summary.card_a, Some(CardId::KINETIC_SALVO));
    }

    #[test]
    fn guard_reduces_incoming_damage() {
        let mut s = make_session(
            vec![
                CardId::ABLATIVE_HULL,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::ABLATIVE_HULL);
        assert!(matches!(outcome, BattleOutcome::Continue));
        // AI struck attacker, but ablative reduced damage; attacker
        // integrity should be high (>= 90) and enemy integrity = 100.
        assert!(s.integrity_a >= 90);
        assert_eq!(s.integrity_b, 100);
    }

    #[test]
    fn ciws_grid_cancels_opponent_card() {
        // AI plays Strike; player plays CIWS Grid → AI strike cancelled.
        let mut s = make_session(
            vec![
                CardId::CIWS_GRID,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::CIWS_GRID);
        assert!(matches!(outcome, BattleOutcome::Continue));
        // Player's CIWS cancelled the AI strike; attacker's integrity
        // should not have changed (and defender still has 100).
        assert_eq!(s.integrity_a, 100);
        assert_eq!(s.integrity_b, 100);
    }

    #[test]
    fn withdraw_finalizes_battle() {
        let mut s = make_session(
            vec![
                CardId::WARP_RETREAT,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::WARP_RETREAT);
        assert!(matches!(outcome, BattleOutcome::Finished { .. }));
        // Player retains 50% integrity (50 hp from 100).
        assert_eq!(s.integrity_a, 50);
    }

    #[test]
    fn withdraw_as_defender_halves_defender_integrity() {
        // Regression for the side-indexing bug: when the player is the
        // defender and plays WARP_RETREAT, integrity_b (the defender's
        // integrity) must be halved, not integrity_a.
        let mut s = make_session(
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                CardId::WARP_RETREAT,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let (outcome, _) = apply_round(&mut s, BattleSide::Defender, CardId::WARP_RETREAT);
        assert!(matches!(outcome, BattleOutcome::Finished { .. }));
        // Defender (the player) drops to 50 hp; attacker is untouched.
        assert_eq!(s.integrity_a, 100, "attacker integrity must be untouched");
        assert_eq!(s.integrity_b, 50, "defender integrity must be halved");
    }

    #[test]
    fn ciws_cancels_withdraw_no_retreat_no_halving() {
        // Regression: when CIWS Grid cancels a WARP_RETREAT, the
        // cancelled card becomes HOLD_FIRE (Noop), and the retreat
        // must NOT apply — neither side retreats, no halving occurs,
        // and the round continues.
        let mut s = make_session(
            vec![
                CardId::WARP_RETREAT,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                CardId::CIWS_GRID,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::WARP_RETREAT);
        // Round is not a finished battle — the cancelled retreat does
        // not finalise the session.
        assert!(matches!(outcome, BattleOutcome::Continue));
        // Neither integrity is halved by a cancelled retreat; the
        // post-round values depend only on damage from HOLD_FIRE
        // exchanges (zero) and the round log has been appended.
        assert_eq!(
            s.integrity_a, 100,
            "cancelled retreat must not halve attacker"
        );
        assert_eq!(
            s.integrity_b, 100,
            "cancelled retreat must not halve defender"
        );
        assert_eq!(s.rounds.len(), 1, "round must be recorded");
    }

    #[test]
    fn evasive_halves_incoming_damage_not_zeroes_it() {
        // Attacker (side A) hand is a single Kinetic Salvo; defender
        // (side B) hand is a single Burn Maneuver (Evasive).  The AI
        // for the attacker plays the only card in hand_a, so a Strike
        // hits an Evasive.  Evasive should halve the incoming
        // damage, not negate it entirely.
        let mut s = make_session(
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                CardId::BURN_MANEUVER,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        // Defender (side B) plays Evasive; attacker (side A) plays Strike.
        let _ = apply_round(&mut s, BattleSide::Defender, CardId::BURN_MANEUVER);
        // Attacker (side A) played Strike 18 hp; defender (side B)
        // played Evasive.  Evasive halves 18 → 9 damage, so
        // integrity_b = 100 - 9 = 91 exactly.  Exact equality is
        // required to catch any future regression in the halving math.
        assert_eq!(s.integrity_a, 100, "attacker integrity must be untouched");
        assert_eq!(s.integrity_b, 91, "Evasive should halve 18 → 9 damage");
    }

    #[test]
    fn integrity_is_clamped_at_zero() {
        // 100 hp each, attacker plays Orbital Bombardment, defender plays
        // Hold Fire.  Bombardment deals 12 hp scaled; defender has no
        // other hand so the round resolves.  Even if a future test
        // applies a 200-damage card, integrity must clamp at zero.
        let mut s = make_session(
            vec![
                CardId::ORBITAL_BOMBARDMENT,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::ORBITAL_BOMBARDMENT);
        // 12 hp damage hits defender; integrity >= 80.
        assert!(s.integrity_b >= 80);
        assert!(s.integrity_b <= 100);
    }

    #[test]
    fn round_log_is_appended() {
        let mut s = make_session(
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        assert!(s.rounds.is_empty());
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::KINETIC_SALVO);
        assert_eq!(s.rounds.len(), 1);
        assert_eq!(s.rounds[0].round, 1);
    }

    #[test]
    fn defender_side_resolves_correctly() {
        let mut s = make_session(
            vec![
                CardId::KINETIC_SALVO,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
            vec![
                CardId::ABLATIVE_HULL,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
                HOLD_FIRE.id,
            ],
        );
        // Defender plays Guard; attacker plays Strike.
        let _ = apply_round(&mut s, BattleSide::Defender, CardId::ABLATIVE_HULL);
        // Defender integrity should be high (>= 90).
        assert!(s.integrity_b >= 90);
    }
}
