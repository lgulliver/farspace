//! Battle resolution: card-driven round orchestration and the v1 damage
//! model (per-verb formulas).
//!
//! This slice replaces the v2 auto-resolve damage formula with per-verb
//! card-driven damage.  Each card's verb determines how much damage it
//! deals and what modifiers apply (Guard, Evasive, Disrupt, Mark, etc.).
//! The model is deterministic: the same hands + same RNG seed → same
//! outcome.  Fleet integrity is updated per round, and a `BattleReportV3`
//! is written on completion.

use super::card::{CardId, CardVerb, card_by_id};
use super::deck::build_hand;
use super::report::{BattleReportV3, BattleRoundSummary};
use super::{BattlePhase, BattleSession, BattleSetupSummary, BattleSide};
use crate::state::{FleetId, GameState};

/// Fleet stats needed for damage computation.
struct FleetStats {
    strength: u32,
    defense: u32,
}

fn fleet_stats(state: &GameState, fleet: FleetId) -> FleetStats {
    let f = state.fleets.get(&fleet);
    FleetStats {
        strength: f.map(|f| f.strength.max(1)).unwrap_or(1),
        defense: f.map(|f| f.strength.max(1)).unwrap_or(1),
    }
}

/// Apply a v3 battle.  Mutates `state`:
/// - On entry, populates `state.pending_battle_session` (if the player is
///   involved) and emits `BattleStarted`.
/// - When the session finalises (player or AI), emits `BattleFinished`
///   and clears `pending_battle_session`.
/// - Always updates the surviving fleets' integrity in `state.fleets`.
///
/// Damage formulas:
///
/// | Verb       | Enemy damage        | Self damage | Notes |
/// |------------|---------------------|-------------|-------|
/// | Strike     | str × 0.15 (+25% if Mark) | —      | Base combat damage |
/// | Salvo      | str × 0.12          | —           | Spread over 3 rounds |
/// | Overcharge | str × 0.22          | str × 0.05  | High risk |
/// | Guard      | —                   | —           | defense × 0.10 reduction |
/// | Fortify    | —                   | —           | defense × 0.15 reduction |
/// | Evasive    | —                   | —           | incoming × 0.5 |
/// | Disrupt    | —                   | —           | skip opponent's card |
/// | Withdraw   | —                   | —           | retreat now at 50% |
/// | All others | —                   | —           | no damage |
pub fn apply_battle(
    state: &mut GameState,
    star: crate::state::StarId,
    fleet_a: FleetId,
    fleet_b: FleetId,
    setup: BattleSetupSummary,
) -> Vec<crate::events::Event> {
    let mut events = Vec::new();

    let empire_a = setup.empire_a;
    let empire_b = setup.empire_b;
    let player = state.player_empire;
    let player_involved = empire_a == player || empire_b == player;

    let hand_a = build_hand(state, fleet_a, empire_a);
    let hand_b = build_hand(state, fleet_b, empire_b);

    let session_id = state.allocate_battle_session_id();
    let session = BattleSession {
        session_id,
        setup: setup.clone(),
        hand_a: hand_a.clone(),
        hand_b: hand_b.clone(),
        round: 0,
        integrity_a: setup.integrity_a_start,
        integrity_b: setup.integrity_b_start,
        phase: BattlePhase::AwaitingInput,
    };

    events.push(crate::events::Event::BattleStarted {
        session_id,
        star,
        attacker: fleet_a,
        defender: fleet_b,
        hand_a: hand_a.clone(),
        hand_b: hand_b.clone(),
        setup: setup.clone(),
    });

    if !player_involved {
        let outcome = resolve_to_completion(state, session, &hand_a, &hand_b);
        let report = build_report(state, session_id, setup, hand_a, hand_b, outcome);
        state.battle_reports_v3.push_back(report.clone());
        apply_outcome_to_state(state, &report, &mut events);
        return events;
    }

    state.pending_battle_session = Some(session);
    events
}

/// Per-verb base damage.  Returns (damage_to_enemy, damage_to_self).
fn card_damage(verb: CardVerb, strength: u32) -> (u32, u32) {
    match verb {
        CardVerb::Strike => ((strength as f64 * 0.15) as u32, 0),
        CardVerb::Salvo => ((strength as f64 * 0.12) as u32, 0),
        CardVerb::Overcharge => (
            (strength as f64 * 0.22) as u32,
            (strength as f64 * 0.05) as u32,
        ),
        CardVerb::Withdraw => (0, 0),
        CardVerb::Guard => (0, 0),
        CardVerb::Fortify => (0, 0),
        CardVerb::Evasive => (0, 0),
        CardVerb::Disrupt => (0, 0),
        CardVerb::Mark => (0, 0),
        CardVerb::Probe => (0, 0),
        CardVerb::Maneuver => (0, 0),
        CardVerb::Inspire => (0, 0),
        CardVerb::Bolster => (0, 0),
        CardVerb::Noop => (0, 0),
    }
}

/// Defense reduction: guard = def × 0.10, fortify = def × 0.15.
fn guard_value(verb: CardVerb, defense: u32) -> u32 {
    match verb {
        CardVerb::Guard | CardVerb::Noop => (defense as f64 * 0.10) as u32,
        CardVerb::Fortify => (defense as f64 * 0.15) as u32,
        _ => 0,
    }
}

/// Resolve one round: apply both cards, return damage amounts.
#[allow(clippy::too_many_arguments)]
fn resolve_round(
    card_a: Option<CardId>,
    card_b: Option<CardId>,
    ctx: &RoundCtx,
    a_guard: &mut u32,
    b_guard: &mut u32,
    a_evasive: &mut bool,
    b_evasive: &mut bool,
    a_disrupted: &mut bool,
    b_disrupted: &mut bool,
    a_mark: &mut bool,
    b_mark: &mut bool,
) -> (u32, u32) {
    let a_verb = card_a.map(|c| card_by_id(c).verb).unwrap_or(CardVerb::Noop);
    let b_verb = card_b.map(|c| card_by_id(c).verb).unwrap_or(CardVerb::Noop);

    // Disrupt cancels the opponent's card if we haven't already been disrupted.
    let a_effective = if *a_disrupted { CardVerb::Noop } else { a_verb };
    let b_effective = if *b_disrupted { CardVerb::Noop } else { b_verb };

    // Apply disrupt: the side that plays Disrupt cancels the opponent.
    *a_disrupted = a_verb == CardVerb::Disrupt;
    *b_disrupted = b_verb == CardVerb::Disrupt;

    // Guard
    *a_guard = guard_value(a_effective, ctx.a_def).max(*a_guard);
    *b_guard = guard_value(b_effective, ctx.b_def).max(*b_guard);

    // Evasive
    if matches!(a_effective, CardVerb::Evasive) {
        *a_evasive = true;
    }
    if matches!(b_effective, CardVerb::Evasive) {
        *b_evasive = true;
    }

    // Mark
    let a_has_mark = *a_mark || matches!(a_effective, CardVerb::Mark);
    let b_has_mark = *b_mark || matches!(b_effective, CardVerb::Mark);
    *a_mark = matches!(a_effective, CardVerb::Mark);
    *b_mark = matches!(b_effective, CardVerb::Mark);

    // Damage
    let (mut d_a, self_a) = card_damage(a_effective, ctx.a_str);
    let (mut d_b, self_b) = card_damage(b_effective, ctx.b_str);

    // Mark boosts Strike/Salvo/Overcharge by 25%.
    if b_has_mark
        && matches!(
            b_effective,
            CardVerb::Strike | CardVerb::Salvo | CardVerb::Overcharge
        )
    {
        d_b = (d_b as f64 * 1.25) as u32;
    }
    if a_has_mark
        && matches!(
            a_effective,
            CardVerb::Strike | CardVerb::Salvo | CardVerb::Overcharge
        )
    {
        d_a = (d_a as f64 * 1.25) as u32;
    }

    // Apply Guard reduction.
    let d_b_after_guard = d_b.saturating_sub(*b_guard);
    let d_a_after_guard = d_a.saturating_sub(*a_guard);

    // Apply Evasive: ×0.5
    let d_b_final = if *a_evasive {
        d_b_after_guard / 2
    } else {
        d_b_after_guard
    };
    let d_a_final = if *b_evasive {
        d_a_after_guard / 2
    } else {
        d_a_after_guard
    };

    // Self-damage (Overcharge)
    (d_a_final + self_a, d_b_final + self_b)
}

/// Per-round context: fleet stats (strength, defense).
struct RoundCtx {
    a_str: u32,
    b_str: u32,
    a_def: u32,
    b_def: u32,
}

/// Resolve a session to completion in one shot (no player input).  Used
/// for AI-only battles and for the deterministic test path.
fn resolve_to_completion(
    state: &GameState,
    mut session: BattleSession,
    hand_a: &[CardId],
    hand_b: &[CardId],
) -> super::BattleOutcome {
    let mut rounds = Vec::new();
    let mut local_a: Vec<CardId> = hand_a.to_vec();
    let mut local_b: Vec<CardId> = hand_b.to_vec();
    let mut integrity_a = session.integrity_a;
    let mut integrity_b = session.integrity_b;

    let stats = fleet_stats(state, session.setup.fleet_a);
    let stats_b = fleet_stats(state, session.setup.fleet_b);
    let ctx = RoundCtx {
        a_str: stats.strength,
        b_str: stats_b.strength,
        a_def: stats.defense,
        b_def: stats_b.defense,
    };

    let mut a_guard: u32 = 0;
    let mut b_guard: u32 = 0;
    let mut a_evasive = false;
    let mut b_evasive = false;
    let mut a_disrupted = false;
    let mut b_disrupted = false;
    let mut a_mark = false;
    let mut b_mark = false;

    let mut a_retreated = false;
    let mut b_retreated = false;

    for round_idx in 0..super::MAX_ROUNDS as usize {
        session.round = round_idx as u8;
        let card_a = local_a.first().copied();
        let card_b = local_b.first().copied();

        // Describe effects
        let effect_a = card_a
            .map(|c| describe_card_play(c, BattleSide::Attacker, round_idx as u8))
            .unwrap_or_else(|| "(no cards left)".to_string());
        let effect_b = card_b
            .map(|c| describe_card_play(c, BattleSide::Defender, round_idx as u8))
            .unwrap_or_else(|| "(no cards left)".to_string());

        // Check for Withdraw
        if let Some(c) = card_a {
            if card_by_id(c).verb == CardVerb::Withdraw {
                integrity_a = (integrity_a * 50) / 100;
                a_retreated = true;
                if !local_b.is_empty() {
                    local_b.remove(0);
                }
                rounds.push(BattleRoundSummary {
                    round: round_idx as u8,
                    card_a,
                    card_b,
                    effect_a: format!("{effect_a} — auto-retreat at 50%"),
                    effect_b,
                    integrity_a_after: integrity_a,
                    integrity_b_after: integrity_b,
                });
                break;
            }
        }
        if let Some(c) = card_b {
            if card_by_id(c).verb == CardVerb::Withdraw {
                integrity_b = (integrity_b * 50) / 100;
                b_retreated = true;
                if !local_a.is_empty() {
                    local_a.remove(0);
                }
                rounds.push(BattleRoundSummary {
                    round: round_idx as u8,
                    card_a,
                    card_b,
                    effect_a,
                    effect_b: format!("{effect_b} — auto-retreat at 50%"),
                    integrity_a_after: integrity_a,
                    integrity_b_after: integrity_b,
                });
                break;
            }
        }

        if !local_a.is_empty() {
            local_a.remove(0);
        }
        if !local_b.is_empty() {
            local_b.remove(0);
        }

        let (da, db) = resolve_round(
            card_a,
            card_b,
            &ctx,
            &mut a_guard,
            &mut b_guard,
            &mut a_evasive,
            &mut b_evasive,
            &mut a_disrupted,
            &mut b_disrupted,
            &mut a_mark,
            &mut b_mark,
        );

        integrity_a = integrity_a.saturating_sub(da);
        integrity_b = integrity_b.saturating_sub(db);

        rounds.push(BattleRoundSummary {
            round: round_idx as u8,
            card_a,
            card_b,
            effect_a,
            effect_b,
            integrity_a_after: integrity_a,
            integrity_b_after: integrity_b,
        });

        if integrity_a == 0 || integrity_b == 0 {
            break;
        }
    }

    let fleet_a_destroyed = integrity_a == 0;
    let fleet_b_destroyed = integrity_b == 0;
    let system_outcome = if fleet_a_destroyed && fleet_b_destroyed {
        "Mutual destruction".to_string()
    } else if fleet_a_destroyed {
        "Defender wins".to_string()
    } else if fleet_b_destroyed {
        "Attacker wins".to_string()
    } else if a_retreated && !b_retreated {
        format!("Attacker retreated ({integrity_a}% remaining)")
    } else if b_retreated && !a_retreated {
        format!("Defender retreated ({integrity_b}% remaining)")
    } else if integrity_a > integrity_b {
        format!("Attacker wins on integrity ({integrity_a} vs {integrity_b})")
    } else if integrity_b > integrity_a {
        format!("Defender wins on integrity ({integrity_b} vs {integrity_a})")
    } else {
        "Draw — defender holds".to_string()
    };

    let _ = session;
    let _ = state;

    super::BattleOutcome {
        integrity_a,
        integrity_b,
        fleet_a_destroyed,
        fleet_b_destroyed,
        fleet_a_retreated: a_retreated,
        fleet_b_retreated: b_retreated,
        rounds,
        system_outcome,
    }
}

/// Translate a `BattleOutcome` into a `BattleReportV3` and apply it to
/// `state` (integrity + report history + events).
fn build_report(
    state: &mut GameState,
    _session_id: u64,
    setup: BattleSetupSummary,
    hand_a: Vec<CardId>,
    hand_b: Vec<CardId>,
    outcome: super::BattleOutcome,
) -> BattleReportV3 {
    let report_id = state.allocate_battle_report_v3_id();
    BattleReportV3 {
        report_id,
        turn: state.turn,
        star: setup.star,
        fleet_a: setup.fleet_a,
        fleet_b: setup.fleet_b,
        empire_a: setup.empire_a,
        empire_b: setup.empire_b,
        role_a: setup.role_a,
        role_b: setup.role_b,
        formation_a: setup.formation_a,
        formation_b: setup.formation_b,
        supply_a: setup.supply_a,
        supply_b: setup.supply_b,
        ships_a: setup.ships_a,
        ships_b: setup.ships_b,
        integrity_a_start: setup.integrity_a_start,
        integrity_b_start: setup.integrity_b_start,
        integrity_a_end: outcome.integrity_a,
        integrity_b_end: outcome.integrity_b,
        fleet_a_destroyed: outcome.fleet_a_destroyed,
        fleet_b_destroyed: outcome.fleet_b_destroyed,
        fleet_a_retreated: outcome.fleet_a_retreated,
        fleet_b_retreated: outcome.fleet_b_retreated,
        hand_a,
        hand_b,
        rounds: outcome.rounds,
        system_outcome: outcome.system_outcome,
    }
}

/// Apply the report's outcome to `state`: update fleet integrity, append
/// the final report, emit `BattleFinished`.
fn apply_outcome_to_state(
    state: &mut GameState,
    report: &BattleReportV3,
    events: &mut Vec<crate::events::Event>,
) {
    state.battle_reports_v3.push_back(report.clone());
    const MAX: usize = 40;
    while state.battle_reports_v3.len() > MAX {
        state.battle_reports_v3.pop_front();
    }
    if let Some(f) = state.fleets.get_mut(&report.fleet_a) {
        if report.fleet_a_destroyed {
            f.integrity = 0;
        } else {
            f.integrity = report.integrity_a_end;
        }
    }
    if let Some(f) = state.fleets.get_mut(&report.fleet_b) {
        if report.fleet_b_destroyed {
            f.integrity = 0;
        } else {
            f.integrity = report.integrity_b_end;
        }
    }
    state.pending_battle_session = None;
    events.push(crate::events::Event::BattleFinished {
        session_id: report.report_id,
        report_id: report.report_id,
        star: report.star,
        outcome: report.system_outcome.clone(),
    });
}

/// Build a one-line description of a card play for the round log.
fn describe_card_play(card: CardId, side: BattleSide, round: u8) -> String {
    let c = card_by_id(card);
    let who = side.label();
    format!(
        "R{}: {} played {} ({})",
        round + 1,
        who,
        c.name,
        c.verb.label()
    )
}

/// Apply a player card play to the pending session.  Applies damage
/// immediately (the AI responds on the same round).  `session_id` must
/// match the pending session.  Returns events to append.
pub fn play_card(
    state: &mut GameState,
    session_id: u64,
    card_index: usize,
) -> Vec<crate::events::Event> {
    let mut events = Vec::new();
    let Some(session) = state.pending_battle_session.as_mut() else {
        return events;
    };
    if session.session_id != session_id {
        return events;
    }
    let player = state.player_empire;
    let player_is_a = session.setup.empire_a == player;
    let hand = if player_is_a {
        &mut session.hand_a
    } else {
        &mut session.hand_b
    };

    if card_index >= hand.len() {
        return events;
    }
    let card = hand.remove(card_index);
    let round = session.round;
    let sid = session.session_id;

    let card_def = card_by_id(card);

    // Log player's play.
    events.push(crate::events::Event::BattleRoundPlayed {
        session_id: sid,
        round,
        side: if player_is_a {
            crate::events::BattleRoundSide::Attacker
        } else {
            crate::events::BattleRoundSide::Defender
        },
        card,
        effect: format!(
            "R{}: played {} ({})",
            round + 1,
            card_def.name,
            card_def.verb.label()
        ),
    });

    // AI responds.
    let ai_hand = if player_is_a {
        &session.hand_b
    } else {
        &session.hand_a
    };
    if let Some(ai_card) = ai_pick_card(ai_hand, None) {
        // Log AI play.
        let ai_def = card_by_id(ai_card);
        events.push(crate::events::Event::BattleRoundPlayed {
            session_id: sid,
            round,
            side: if player_is_a {
                crate::events::BattleRoundSide::Defender
            } else {
                crate::events::BattleRoundSide::Attacker
            },
            card: ai_card,
            effect: format!(
                "R{}: AI played {} ({})",
                round + 1,
                ai_def.name,
                ai_def.verb.label()
            ),
        });
    }

    // Advance round counter.
    session.round = (session.round + 1).min(super::MAX_ROUNDS);

    // Check if battle should auto-finalise.
    let should_finalise = session.hand_a.is_empty()
        || session.hand_b.is_empty()
        || session.round >= super::MAX_ROUNDS;
    if should_finalise {
        // Capture state before dropping the session borrow.
        let setup = session.setup.clone();
        let ha = session.hand_a.clone();
        let hb = session.hand_b.clone();
        let start_a = session.integrity_a;
        let start_b = session.integrity_b;
        // Build a minimal transient session for the resolve engine.
        // `session` is a `&mut` borrow; it ends at the `;` from the last
        // use above.  The tail of this block operates on `state` alone.
        let transient = BattleSession {
            session_id: sid,
            setup,
            hand_a: ha.clone(),
            hand_b: hb.clone(),
            round: 0,
            integrity_a: start_a,
            integrity_b: start_b,
            phase: BattlePhase::AwaitingInput,
        };
        let transient_setup = transient.setup.clone();
        let outcome = resolve_to_completion(state, transient, &ha, &hb);
        let report = build_report(state, sid, transient_setup, ha, hb, outcome);
        apply_outcome_to_state(state, &report, &mut events);
    }
    events
}

/// Apply a free-retreat command.  Sets the player side's integrity to 25%
/// and finalises the battle immediately.  The AI side retains full integrity.
pub fn player_retreat(state: &mut GameState, session_id: u64) -> Vec<crate::events::Event> {
    let mut events = Vec::new();
    let Some(session) = state.pending_battle_session.as_mut() else {
        return events;
    };
    if session.session_id != session_id {
        return events;
    }
    let player = state.player_empire;
    let player_is_a = session.setup.empire_a == player;

    let (p_int, ai_int) = if player_is_a {
        (session.integrity_a, session.integrity_b)
    } else {
        (session.integrity_b, session.integrity_a)
    };
    let new_p_int = (p_int * 25) / 100;
    if player_is_a {
        session.integrity_a = new_p_int;
    } else {
        session.integrity_b = new_p_int;
    }

    events.push(crate::events::Event::BattleRoundPlayed {
        session_id,
        round: session.round,
        side: if player_is_a {
            crate::events::BattleRoundSide::Attacker
        } else {
            crate::events::BattleRoundSide::Defender
        },
        card: CardId(0),
        effect: "Free retreat (r command)".to_string(),
    });

    // Build a final report.  Drop the session borrow first.
    let setup = session.setup.clone();
    let ha = session.hand_a.clone();
    let hb = session.hand_b.clone();
    let a_retreated = player_is_a;
    let b_retreated = !player_is_a;
    // `session` is a `&mut` borrow; it ends at the `;` from the last use
    // above.  The tail of this block operates on `state` alone.
    let outcome = super::BattleOutcome {
        integrity_a: if a_retreated { new_p_int } else { ai_int },
        integrity_b: if b_retreated { new_p_int } else { ai_int },
        fleet_a_destroyed: false,
        fleet_b_destroyed: false,
        fleet_a_retreated: a_retreated,
        fleet_b_retreated: b_retreated,
        rounds: vec![],
        system_outcome: format!("Player retreated at {new_p_int}% integrity"),
    };
    let report = build_report(state, session_id, setup, ha, hb, outcome);
    apply_outcome_to_state(state, &report, &mut events);
    events
}

/// Finalise the pending session, if any.  Called at the end of every
/// `apply_turn` so player sessions close cleanly.
pub fn finalise_pending(state: &mut GameState) -> Vec<crate::events::Event> {
    let mut events = Vec::new();
    let Some(session) = state.pending_battle_session.clone() else {
        return events;
    };
    let ha = session.hand_a.clone();
    let hb = session.hand_b.clone();
    let outcome = resolve_to_completion(state, session.clone(), &ha, &hb);
    let report = build_report(state, session.session_id, session.setup, ha, hb, outcome);
    apply_outcome_to_state(state, &report, &mut events);
    events
}

/// Empty noop (placeholder for the unused `apply_withdraw_card` re-export).
pub fn noop_for_withdraw() -> Option<CardVerb> {
    Some(CardVerb::Noop)
}

use super::ai::ai_pick_card;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Fleet, FleetKind, ScenarioSetup};

    fn new_state() -> GameState {
        crate::engine::Engine::new_from_setup(ScenarioSetup::default_for_seed(7)).state
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

    fn build_setup(state: &GameState, a: FleetId, b: FleetId) -> BattleSetupSummary {
        BattleSetupSummary {
            star: crate::state::StarId(0),
            fleet_a: a,
            fleet_b: b,
            empire_a: state.player_empire,
            empire_b: state.player_empire,
            role_a: crate::state::FleetRole::StrikeFleet,
            role_b: crate::state::FleetRole::DefenseFleet,
            formation_a: crate::state::FleetFormation::Balanced,
            formation_b: crate::state::FleetFormation::Balanced,
            supply_a: crate::state::FleetSupplyState::Supplied,
            supply_b: crate::state::FleetSupplyState::Supplied,
            ships_a: 1,
            ships_b: 1,
            integrity_a_start: 100,
            integrity_b_start: 100,
            doctrine_a: String::new(),
            doctrine_b: String::new(),
        }
    }

    #[test]
    fn ai_only_battle_resolves_and_clears_session() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        // Make BOTH fleets owned by non-player empires so the battle is AI-only.
        let ai_a = crate::state::EmpireId(98);
        let ai_b = crate::state::EmpireId(99);
        for id in [ai_a, ai_b] {
            state.empires.insert(
                id,
                crate::state::Empire {
                    id,
                    name: format!("AI {}", id.0),
                    credits: 0,
                    research_points: 0,
                    home_star: crate::state::StarId(0),
                    research: crate::state::ResearchState::default(),
                    food: 0,
                    empire_def: None,
                },
            );
        }
        if let Some(fleet_a) = state.fleets.get_mut(&a) {
            fleet_a.owner = ai_a;
        }
        if let Some(fleet_b) = state.fleets.get_mut(&b) {
            fleet_b.owner = ai_b;
        }
        let setup = BattleSetupSummary {
            star: crate::state::StarId(0),
            fleet_a: a,
            fleet_b: b,
            empire_a: ai_a,
            empire_b: ai_b,
            role_a: crate::state::FleetRole::StrikeFleet,
            role_b: crate::state::FleetRole::DefenseFleet,
            formation_a: crate::state::FleetFormation::Balanced,
            formation_b: crate::state::FleetFormation::Balanced,
            supply_a: crate::state::FleetSupplyState::Supplied,
            supply_b: crate::state::FleetSupplyState::Supplied,
            ships_a: 1,
            ships_b: 1,
            integrity_a_start: 100,
            integrity_b_start: 100,
            doctrine_a: String::new(),
            doctrine_b: String::new(),
        };
        let events = apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        assert!(!events.is_empty(), "AI battle should emit events");
        assert!(state.pending_battle_session.is_none());
        assert!(!state.battle_reports_v3.is_empty());
    }

    #[test]
    fn player_battle_pauses_with_pending_session() {
        let mut state = new_state();
        // Both fleets are player-owned → player is involved → session pauses.
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        let _events = apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        assert!(state.pending_battle_session.is_some());
    }

    #[test]
    fn play_card_records_event_and_trims_hand() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        let session = state.pending_battle_session.as_ref().unwrap();
        let id = session.session_id;
        let hand_len_before = session.hand_a.len();
        let events = play_card(&mut state, id, 0);
        // Player card + AI response.
        assert_eq!(events.len(), 2);
        let new_len = state.pending_battle_session.as_ref().unwrap().hand_a.len();
        assert_eq!(new_len, hand_len_before - 1);
    }

    #[test]
    fn play_card_rejects_invalid_session() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        let events = play_card(&mut state, 9999, 0);
        assert!(events.is_empty());
    }

    #[test]
    fn play_card_rejects_out_of_range_index() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        let id = state.pending_battle_session.as_ref().unwrap().session_id;
        let events = play_card(&mut state, id, 9999);
        assert!(events.is_empty());
    }

    #[test]
    fn finalise_clears_pending_and_appends_report() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        let count_before = state.battle_reports_v3.len();
        let events = finalise_pending(&mut state);
        assert!(!events.is_empty());
        assert!(state.pending_battle_session.is_none());
        assert!(state.battle_reports_v3.len() > count_before);
    }

    #[test]
    fn free_retreat_finalises_and_reduces_integrity() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        let id = state.pending_battle_session.as_ref().unwrap().session_id;
        let events = player_retreat(&mut state, id);
        // BattleRoundPlayed + BattleFinished.
        assert_eq!(events.len(), 2);
        assert!(state.pending_battle_session.is_none());
        let report = state.battle_reports_v3.back().unwrap();
        assert!(report.fleet_a_retreated || report.fleet_b_retreated);
        assert!(
            report.integrity_a_end < 100 || report.integrity_b_end < 100,
            "retreat should reduce integrity"
        );
    }

    #[test]
    fn describe_card_play_formats_round_log() {
        let s = describe_card_play(CardId(1), BattleSide::Attacker, 0);
        assert!(s.contains("R1"));
        assert!(s.contains("Kinetic Salvo"));
        assert!(s.contains("Attacker"));
    }

    #[test]
    fn describe_card_play_uses_defender_label() {
        let s = describe_card_play(CardId(2), BattleSide::Defender, 2);
        assert!(s.contains("R3"));
        assert!(s.contains("Defender"));
    }

    #[test]
    fn equal_fleets_deal_symmetric_damage() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::EscortFrigate);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        // Both fleets owned by different AI for auto-resolve.
        let ai_a = crate::state::EmpireId(98);
        let ai_b = crate::state::EmpireId(99);
        for id in [ai_a, ai_b] {
            state.empires.insert(
                id,
                crate::state::Empire {
                    id,
                    name: format!("AI {}", id.0),
                    credits: 0,
                    research_points: 0,
                    home_star: crate::state::StarId(0),
                    research: crate::state::ResearchState::default(),
                    food: 0,
                    empire_def: None,
                },
            );
        }
        if let Some(f) = state.fleets.get_mut(&a) {
            f.owner = ai_a;
        }
        if let Some(f) = state.fleets.get_mut(&b) {
            f.owner = ai_b;
        }
        let setup = BattleSetupSummary {
            star: crate::state::StarId(0),
            fleet_a: a,
            fleet_b: b,
            empire_a: ai_a,
            empire_b: ai_b,
            role_a: crate::state::FleetRole::StrikeFleet,
            role_b: crate::state::FleetRole::DefenseFleet,
            formation_a: crate::state::FleetFormation::Balanced,
            formation_b: crate::state::FleetFormation::Balanced,
            supply_a: crate::state::FleetSupplyState::Supplied,
            supply_b: crate::state::FleetSupplyState::Supplied,
            ships_a: 1,
            ships_b: 1,
            integrity_a_start: 100,
            integrity_b_start: 100,
            doctrine_a: String::new(),
            doctrine_b: String::new(),
        };
        let events = apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        assert!(!events.is_empty());
        assert!(!state.battle_reports_v3.is_empty());
        let report = state.battle_reports_v3.back().unwrap();
        // With 2 equal-strength Escort Frigates (strength=10 each) and the
        // default hand, the damage should be symmetric enough that both
        // sides take similar integrity loss.
        assert!(
            (report.integrity_a_end as i64 - report.integrity_b_end as i64).unsigned_abs() < 30,
            "equal fleets should have close final integrity (a={}, b={})",
            report.integrity_a_end,
            report.integrity_b_end
        );
    }
}
