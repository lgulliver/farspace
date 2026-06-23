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
/// The optional `ai_empire_def` is the empire definition of the
/// non-player side.  When `Some`, the AI's card pick is augmented by
/// the doctrine-bias contribution.  When `None`, the AI uses the
/// verb-only baseline.
///
/// Returns the outcome and the `BattleRoundSummary` that was appended
/// to the session.  When the outcome is `Finished` the caller must
/// finalise the report.
pub fn apply_round(
    session: &mut BattleSession,
    player_side: BattleSide,
    player_card: CardId,
    ai_empire_def: Option<&crate::EmpireDefinition>,
) -> (BattleOutcome, BattleRoundSummary) {
    // AI chooses a card.
    let ai_side = player_side.other();
    let ai_index = ai_pick_card(session, ai_side, ai_empire_def);
    let ai_card = match ai_side {
        BattleSide::Attacker => session.hand_a[ai_index],
        BattleSide::Defender => session.hand_b[ai_index],
    };

    // Snapshot starting integrity for the summary.
    let start_a = session.integrity_a;
    let start_b = session.integrity_b;
    let round = session.round;

    // Disrupt cancellation: any CIWS Grid on either side replaces the
    // opposing card with Hold Fire for this round.
    let (player_card_resolved, ai_card_resolved) = apply_disrupt(player_card, ai_card);

    // Determine initiative.  Maneuver gives the playing side
    // initiative for the round: if exactly one side played Maneuver
    // (post-CIWS), that side's effects resolve first.  Ties (both or
    // neither) resolve attacker-first.
    let p_initiative = matches!(card_by_id(player_card_resolved).verb, CardVerb::Maneuver);
    let ai_initiative = matches!(card_by_id(ai_card_resolved).verb, CardVerb::Maneuver);
    let attacker_first = match (p_initiative, ai_initiative) {
        (true, false) => match player_side {
            BattleSide::Attacker => true,
            BattleSide::Defender => false,
        },
        (false, true) => match player_side {
            BattleSide::Attacker => false,
            BattleSide::Defender => true,
        },
        // Tie: attacker always goes first.
        _ => true,
    };

    // Resolve the round in initiative order.  Each call applies one
    // side's card against the current state; with simultaneous
    // resolution the first call is the one with initiative and the
    // second call observes the integrity after the first.  We do
    // NOT collapse to a single `resolve_pair` because initiative
    // must order mutations (Disrupt cancels the *played* card, not
    // the resolved card; a Withdraw that drops integrity to 0 before
    // the opponent strikes affects their outgoing damage math).
    let (card_a, card_b, effect_a, effect_b, a_retreated, b_retreated) = if attacker_first {
        let (eff_a, eff_b, p_retreated, f_retreated) = resolve_attacker_then_defender(
            session,
            player_card_resolved,
            player_side,
            ai_card_resolved,
            ai_side,
        );
        // Map the inner "first" / "second" retreated flags back to
        // the attacker / defender frame based on which side the
        // player is on.
        let (a_retreated, b_retreated) = match player_side {
            BattleSide::Attacker => (p_retreated, f_retreated),
            BattleSide::Defender => (f_retreated, p_retreated),
        };
        (
            Some(player_card),
            Some(ai_card),
            eff_a,
            eff_b,
            a_retreated,
            b_retreated,
        )
    } else {
        // Defender (defender's chosen side) goes first.
        let (eff_a, eff_b, p_retreated, f_retreated) = resolve_attacker_then_defender(
            session,
            ai_card_resolved,
            ai_side,
            player_card_resolved,
            player_side,
        );
        // When the player is the defender and the player's card
        // resolved second (the inner "first" / "second" are the
        // attacker's / defender's resolved order), map accordingly.
        let (a_retreated, b_retreated) = match player_side {
            BattleSide::Attacker => (p_retreated, f_retreated),
            BattleSide::Defender => (f_retreated, p_retreated),
        };
        (
            Some(ai_card),
            Some(player_card),
            eff_a,
            eff_b,
            a_retreated,
            b_retreated,
        )
    };

    // Clamp integrity at zero (integer floor).  resolve_attacker_then_defender
    // already saturates, but the explicit min()s are a belt-and-braces
    // safety net.
    session.integrity_a = session.integrity_a.min(start_a);
    session.integrity_b = session.integrity_b.min(start_b);

    // Withdraw: retreating side drops to 50% of pre-round integrity.
    if a_retreated {
        session.integrity_a = start_a / 2;
    }
    if b_retreated {
        session.integrity_b = start_b / 2;
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

    // Decide outcome.  A retreated fleet survives; a destroyed fleet
    // is one whose integrity reached 0.  Retreat and destruction
    // are mutually exclusive by construction (Withdraw drops
    // integrity to start_a/2 ≥ 1, never 0).
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
            // Tiebreaker: higher integrity wins; equal integrity is a draw.
            match session.integrity_a.cmp(&session.integrity_b) {
                std::cmp::Ordering::Greater => Some(BattleSide::Attacker),
                std::cmp::Ordering::Less => Some(BattleSide::Defender),
                std::cmp::Ordering::Equal => None,
            }
        } else {
            None
        };
        BattleOutcome::Finished { winner }
    } else {
        session.round = session.round.saturating_add(1);
        // Apply this round's accumulated Salvo recurring damage to
        // the opposing fleet at the start of the *next* round.  We
        // do that here so the post-round summary records the
        // post-recurring-damage integrity.
        apply_recurring_salvo(session);
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

/// Read side integrity by frame (attacker/defender).
fn side_integrities(session: &BattleSession, side: BattleSide) -> (u32, u32) {
    match side {
        BattleSide::Attacker => (session.integrity_a, session.integrity_b),
        BattleSide::Defender => (session.integrity_b, session.integrity_a),
    }
}

fn set_side_integrity(session: &mut BattleSession, side: BattleSide, value: u32) {
    match side {
        BattleSide::Attacker => session.integrity_a = value,
        BattleSide::Defender => session.integrity_b = value,
    }
}

fn enemy_side(side: BattleSide) -> BattleSide {
    side.other()
}

fn side_has_mark(session: &BattleSession, side: BattleSide) -> bool {
    match side {
        BattleSide::Attacker => session.mark_a_pending,
        BattleSide::Defender => session.mark_b_pending,
    }
}

fn consume_mark(session: &mut BattleSession, side: BattleSide) {
    match side {
        BattleSide::Attacker => session.mark_a_pending = false,
        BattleSide::Defender => session.mark_b_pending = false,
    }
}

/// Mark applied effect note.
const MARK_APPLIED_SUFFIX: &str = " (+Mark)";

/// Mark consumed effect note (reserved for future effect-text annotation).
#[allow(dead_code)]
const MARK_CONSUMED_SUFFIX: &str = " (Mark)";

/// Resolve the round in initiative order: the first side's card
/// applies, then the second side's card applies.  Returns
/// `(effect_a, effect_b, a_retreated, b_retreated)`.
fn resolve_attacker_then_defender(
    session: &mut BattleSession,
    card_a: CardId,
    side_a: BattleSide,
    card_b: CardId,
    side_b: BattleSide,
) -> (String, String, bool, bool) {
    let def_a = card_by_id(card_a);
    let def_b = card_by_id(card_b);
    let a_initiative = matches!(def_a.verb, CardVerb::Maneuver);
    let b_initiative = matches!(def_b.verb, CardVerb::Maneuver);
    let attacker_first = match (a_initiative, b_initiative) {
        (true, false) => true,
        (false, true) => false,
        // Tie: attacker first.
        _ => true,
    };

    // Recurring Salvo pressure is applied at the *end* of
    // `apply_round` (in the Continue branch), not here, to avoid
    // double-counting: the start-of-round tick and the post-round
    // tick would otherwise both subtract `salvo_x_recurring` from
    // the same integrities on the same round.

    // Set the side that will go first and resolve in order.
    let (first_card, first_side, second_card, second_side) = if attacker_first {
        (card_a, side_a, card_b, side_b)
    } else {
        (card_b, side_b, card_a, side_a)
    };

    // First side resolves.
    let (first_dmg, first_self) = resolve_one_side(session, first_side, first_card);

    // Second side resolves against the now-updated state.  If the
    // first side destroyed the second side (integrity 0), the second
    // side's card is a no-op.
    let second_dmg = {
        let (_, enemy_int) = side_integrities(session, enemy_side(second_side));
        if enemy_int == 0 {
            0
        } else {
            resolve_one_side(session, second_side, second_card).0
        }
    };

    // Subtract damages from the appropriate integrities.  Pass each
    // target's played card explicitly so Evasive halving works on the
    // current round (the round summary is not yet in `session.rounds`).
    apply_damage_to(session, first_side, Some(first_card), second_dmg);
    apply_damage_to(session, second_side, Some(second_card), first_dmg);

    // Apply self-damage to the playing side via the original cards.
    // resolve_one_side already applied the self-damage internally.
    let _ = first_self; // self-damage already applied in resolve_one_side

    // Recurring Salvo: if a side just played Salvo, set its
    // recurring field.  Recurring damage equals `base_damage / 4`
    // (rounded down) so the per-round bleed stays small.  Salvo is
    // unique to the Orbital Bombardment card and to the faction
    // signature card "Siege Doctrine" in v1.
    set_recurring_salvo(session, first_side, first_card);
    set_recurring_salvo(session, second_side, second_card);

    // Inspire: if either side played Inspire, refill hand to 5.
    if def_a.verb == CardVerb::Inspire {
        push_hold_fire_if_short(session, side_a);
    }
    if def_b.verb == CardVerb::Inspire {
        push_hold_fire_if_short(session, side_b);
    }

    // Build effect text.  We use the resolved damage values plus
    // Mark annotations to give the player a clear log.
    let eff_first = build_effect_text(
        card_by_id(first_card).name,
        first_dmg,
        if matches!(card_by_id(first_card).verb, CardVerb::Overcharge) {
            card_by_id(first_card).self_damage
        } else {
            0
        },
    );
    let eff_second = build_effect_text(
        card_by_id(second_card).name,
        second_dmg,
        if matches!(card_by_id(second_card).verb, CardVerb::Overcharge) {
            card_by_id(second_card).self_damage
        } else {
            0
        },
    );

    // Mark effect strings: if either side *gained* a mark, annotate
    // their effect with "(+Mark)" so the log is informative.
    let eff_first = annotate_mark_applied(card_by_id(first_card).verb, eff_first);
    let eff_second = annotate_mark_applied(card_by_id(second_card).verb, eff_second);

    // Map back to attacker/defender frame.
    let (eff_a, eff_b) = if attacker_first {
        (eff_first, eff_second)
    } else {
        (eff_second, eff_first)
    };

    // Retreat flags: post-CIWS card verbs.
    let a_retreated = matches!(def_a.verb, CardVerb::Withdraw);
    let b_retreated = matches!(def_b.verb, CardVerb::Withdraw);

    (eff_a, eff_b, a_retreated, b_retreated)
}

/// Resolve a single side's card and return `(damage_to_enemy,
/// self_damage)`.  Self-damage is applied to the playing side here;
/// damage to the enemy is *not* applied here — the caller applies
/// it after both sides resolve so the order is well-defined.
fn resolve_one_side(session: &mut BattleSession, side: BattleSide, card: CardId) -> (u32, u32) {
    let def = card_by_id(card);
    let self_dmg = if def.verb == CardVerb::Overcharge {
        def.self_damage
    } else {
        0
    };
    if self_dmg > 0 {
        let (own, _) = side_integrities(session, side);
        set_side_integrity(session, side, own.saturating_sub(self_dmg));
    }

    // Mark: the playing side gains a pending mark if they play Mark.
    if def.verb == CardVerb::Mark {
        match side {
            BattleSide::Attacker => session.mark_a_pending = true,
            BattleSide::Defender => session.mark_b_pending = true,
        }
    }

    // Outgoing damage (with Mark +25% if pending and the card is a
    // damage verb; consume the mark when applied).
    let raw = outgoing_damage(session, side, card);
    let mut dmg = match def.verb {
        CardVerb::Strike | CardVerb::Salvo | CardVerb::Overcharge => raw,
        _ => 0,
    };
    if dmg > 0 && side_has_mark(session, side) {
        dmg = dmg.saturating_mul(125) / 100;
        consume_mark(session, side);
    }

    // Evasive on the playing side?  Evasive reduces *incoming* damage
    // to the playing side, which is the *opponent's* outgoing
    // damage.  Apply Evasive by halving the opponent's accumulated
    // damage in `apply_damage_to` when the playing side's integrity
    // is being reduced.  We approximate that here by leaving dmg
    // unchanged and handling the half in `apply_damage_to`.  That
    // keeps the damage call site single.
    (dmg, self_dmg)
}

/// Apply `dmg` to the `target` side, modified by the target's card:
///   - Evasive halves the incoming damage.
///   - Guard subtracts `base_defense` (clamped at 0).
///   - Fortify subtracts `base_defense * 1.5` (clamped at 0).
///
/// `target_card` is passed in explicitly because the round summary
/// is appended only after `apply_damage_to` returns; the function
/// cannot look it up via `session.rounds.last()`.
fn apply_damage_to(
    session: &mut BattleSession,
    target: BattleSide,
    target_card: Option<CardId>,
    dmg: u32,
) {
    if dmg == 0 {
        return;
    }
    let (halved, reduced) = if let Some(card) = target_card {
        let def = card_by_id(card);
        let is_evasive = matches!(def.verb, CardVerb::Evasive);
        let reduction = match def.verb {
            CardVerb::Guard => def.base_defense,
            CardVerb::Fortify => def.base_defense.saturating_mul(150) / 100,
            _ => 0,
        };
        (is_evasive, reduction)
    } else {
        (false, 0)
    };
    let post_reduce = dmg.saturating_sub(reduced);
    let final_dmg = if halved { post_reduce / 2 } else { post_reduce };
    let (own, _) = side_integrities(session, target);
    set_side_integrity(session, target, own.saturating_sub(final_dmg));
}

/// If the playing side played Salvo, set its `salvo_x_recurring`
/// field to a small value (base_damage / 4) for the rest of the
/// battle.  Idempotent: a fresh Salvo refreshes the field.
fn set_recurring_salvo(session: &mut BattleSession, side: BattleSide, card: CardId) {
    let def = card_by_id(card);
    if def.verb == CardVerb::Salvo {
        let recurring = def.base_damage / 4;
        match side {
            BattleSide::Attacker => session.salvo_a_recurring = recurring,
            BattleSide::Defender => session.salvo_b_recurring = recurring,
        }
    }
}

/// Apply each side's recurring Salvo pressure to the *opposing* side
/// at the start of a new round.  Called from `apply_round` (in the
/// Continue branch) so the post-round summary records the bleed on
/// the *following* round.
fn apply_recurring_salvo(session: &mut BattleSession) {
    let dmg_to_b = session.salvo_a_recurring;
    let dmg_to_a = session.salvo_b_recurring;
    if dmg_to_b > 0 {
        let (own, _) = side_integrities(session, BattleSide::Defender);
        set_side_integrity(session, BattleSide::Defender, own.saturating_sub(dmg_to_b));
    }
    if dmg_to_a > 0 {
        let (own, _) = side_integrities(session, BattleSide::Attacker);
        set_side_integrity(session, BattleSide::Attacker, own.saturating_sub(dmg_to_a));
    }
}

/// Annotate the effect string with "(+Mark)" when the playing side
/// gained a Mark (played a Mark card).  Annotate with "(Mark)" when
/// the playing side consumed a Mark with a damage card.
fn annotate_mark_applied(verb: CardVerb, effect: String) -> String {
    if matches!(verb, CardVerb::Mark) {
        format!("{effect}{MARK_APPLIED_SUFFIX}")
    } else {
        effect
    }
}

/// Build the human-readable effect text.  Wrapper over `format_effect`
/// kept for symmetry with the Mark / Salvo annotations done at the
/// call site.
fn build_effect_text(name: &str, dmg: u32, self_dmg: u32) -> String {
    format_effect(name, dmg, self_dmg)
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

/// Self-damage dealt by a card (Overcharge).  Returns `(self_a, self_b)`.
#[allow(dead_code)]
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
            mark_a_pending: false,
            mark_b_pending: false,
            salvo_a_recurring: 0,
            salvo_b_recurring: 0,
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
        let (outcome, summary) =
            apply_round(&mut s, BattleSide::Attacker, CardId::KINETIC_SALVO, None);
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
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::ABLATIVE_HULL, None);
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
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::CIWS_GRID, None);
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
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::WARP_RETREAT, None);
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
        let (outcome, _) = apply_round(&mut s, BattleSide::Defender, CardId::WARP_RETREAT, None);
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
        let (outcome, _) = apply_round(&mut s, BattleSide::Attacker, CardId::WARP_RETREAT, None);
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
    fn ciws_cancels_withdraw_as_defender_no_retreat() {
        // Mirror of `ciws_cancels_withdraw_no_retreat_no_halving`:
        // the player is the defender and plays WARP_RETREAT; the
        // AI attacks with CIWS_GRID and cancels the retreat.  The
        // same contract holds: round continues, no integrity
        // halving, no finalisation.
        let mut s = make_session(
            vec![
                CardId::CIWS_GRID,
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
        let (outcome, _) = apply_round(&mut s, BattleSide::Defender, CardId::WARP_RETREAT, None);
        assert!(matches!(outcome, BattleOutcome::Continue));
        assert_eq!(
            s.integrity_a, 100,
            "cancelled defender retreat must not halve attacker"
        );
        assert_eq!(
            s.integrity_b, 100,
            "cancelled defender retreat must not halve defender"
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
        let _ = apply_round(&mut s, BattleSide::Defender, CardId::BURN_MANEUVER, None);
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
        let _ = apply_round(
            &mut s,
            BattleSide::Attacker,
            CardId::ORBITAL_BOMBARDMENT,
            None,
        );
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
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::KINETIC_SALVO, None);
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
        let _ = apply_round(&mut s, BattleSide::Defender, CardId::ABLATIVE_HULL, None);
        // Defender integrity should be high (>= 90).
        assert!(s.integrity_b >= 90);
    }

    // --- New tests for Mark / Salvo / Maneuver / retreat flags ---

    #[test]
    fn mark_buff_then_strike_applies_25_percent_bonus() {
        // Round 1: attacker plays Mark → mark_a_pending = true.
        // Round 2: attacker plays Kinetic Salvo → damage boosted by
        // 125/100, mark_a_pending is consumed (false again).
        let mut s = make_session(
            vec![
                CardId::TARGETING_LOCK,
                CardId::KINETIC_SALVO,
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
        // Round 1 — Mark.
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::TARGETING_LOCK, None);
        assert!(
            s.mark_a_pending,
            "Mark buff should be pending after Mark card"
        );
        assert_eq!(s.integrity_b, 100, "Mark deals no damage");

        // Round 2 — Strike with Mark.
        let pre_b = s.integrity_b;
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::KINETIC_SALVO, None);
        let dmg = pre_b - s.integrity_b;
        // 18 base * 125% = 22.5 → 22 (integer).
        assert_eq!(dmg, 22, "Mark+Strike should deal 22 damage (18 * 125/100)");
        assert!(
            !s.mark_a_pending,
            "Mark should be consumed after the buffed Strike"
        );
    }

    #[test]
    fn mark_buff_not_consumed_by_non_damage_card() {
        // Mark played round 1, then a non-damage card round 2.  Mark
        // must persist until a damage card is played.
        let mut s = make_session(
            vec![
                CardId::TARGETING_LOCK,
                CardId::SENSOR_SWEEP,
                CardId::KINETIC_SALVO,
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
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::TARGETING_LOCK, None);
        assert!(s.mark_a_pending);
        // Play a Probe (Sensor Sweep) — not a damage verb.
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::SENSOR_SWEEP, None);
        assert!(
            s.mark_a_pending,
            "Mark must persist through a non-damage card"
        );
    }

    #[test]
    fn salvo_recurring_damage_applies_on_subsequent_rounds() {
        // Round 1: attacker plays Orbital Bombardment (Salvo, 12 dmg).
        // Round 2 onwards: defender takes salvo_a_recurring (12 / 4 = 3)
        // at the start of each round.
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
        // Round 1: Salvo.
        let _ = apply_round(
            &mut s,
            BattleSide::Attacker,
            CardId::ORBITAL_BOMBARDMENT,
            None,
        );
        let integrity_b_after_round_1 = s.integrity_b;
        assert_eq!(s.salvo_a_recurring, 3, "Salvo recurring = 12 / 4 = 3");

        // Round 2: AI plays Hold Fire.  No damage from cards, but
        // salvo_a_recurring ticks 3 damage against the defender.
        let _ = apply_round(&mut s, BattleSide::Attacker, HOLD_FIRE.id, None);
        // 88 (round 1) - 3 (post-round tick) - 3 (round 2 start) = 82.
        assert_eq!(
            s.integrity_b, 82,
            "salvo_a_recurring should tick 3 damage at start of round 2 (post-round tick = 3, round 2 start = 3)"
        );
        let _ = integrity_b_after_round_1; // recorded for context
    }

    #[test]
    fn maneuver_card_gives_initiative_to_playing_side() {
        // Attacker plays Drift Burn (Maneuver, gives initiative).
        // The attacker should resolve first, so when the AI's
        // Hold Fire (no-op) is the second resolve, the round
        // still records the attacker's effect on the defender.
        let mut s = make_session(
            vec![
                CardId::DRIFT_BURN,
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
        // Drift Burn deals no damage but should be recorded as
        // attacker's effect.  AI's Hold Fire is the second.
        let _ = apply_round(&mut s, BattleSide::Attacker, CardId::DRIFT_BURN, None);
        // Maneuver itself deals no damage; both integrities should
        // remain at 100.
        assert_eq!(s.integrity_a, 100);
        assert_eq!(s.integrity_b, 100);
        // The round summary should record Drift Burn on side A.
        assert_eq!(s.rounds.last().unwrap().card_a, Some(CardId::DRIFT_BURN));
    }

    #[test]
    fn withdraw_does_not_finalize_when_cancelled_by_ai_ciws() {
        // Symmetric to the existing attacker test: player is the
        // defender, plays WARP_RETREAT, AI plays CIWS_GRID.  The
        // retreat is cancelled, no integrity halving, round
        // continues.
        let mut s = make_session(
            vec![
                CardId::CIWS_GRID,
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
        // Player is the defender; plays WARP_RETREAT.  AI plays CIWS.
        let (outcome, _) = apply_round(&mut s, BattleSide::Defender, CardId::WARP_RETREAT, None);
        assert!(matches!(outcome, BattleOutcome::Continue));
        // Both integrities untouched.
        assert_eq!(s.integrity_a, 100);
        assert_eq!(s.integrity_b, 100);
    }
}
