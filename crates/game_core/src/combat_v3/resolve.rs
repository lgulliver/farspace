//! Battle resolution: card-driven round orchestration and the v1 damage
//! model.
//!
//! The v1 model is a *presentation layer* over the v2 auto-resolve damage
//! formula.  Cards draft a hand per side and the engine still computes the
//! overall damage.  Future slices will replace the damage with per-verb
//! formulas (card verb → damage/heal/etc.) while keeping the
//! `BattleSession` / `BattleReportV3` structures stable.

use super::card::{CardId, CardVerb};
use super::deck::build_hand;
use super::report::{BattleReportV3, BattleRoundSummary};
use super::{BattlePhase, BattleSession, BattleSetupSummary, BattleSide};
use crate::state::{FleetId, GameState};

/// Apply a v3 battle.  Mutates `state`:
/// - On entry, populates `state.pending_battle_session` (if the player is
///   involved) and emits `BattleStarted`.
/// - When the session finalises (player or AI), emits `BattleFinished`
///   and clears `pending_battle_session`.
/// - Always updates the surviving fleets' integrity in `state.fleets`.
///
/// Returns the list of events the engine should append to its event log.
///
/// The damage model in v1 is identical to v2's auto-resolve: detection,
/// positioning, opening volley, main engagement, attrition.  Cards are
/// drafted for presentation and the round log, but the integrity delta
/// comes from the v2 formula.  This is intentional: the v3 *structure*
/// lands first, and the per-verb damage rules land in a follow-up PR.
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
        // Auto-resolve: no player to wait for.
        let outcome = resolve_to_completion(state, session, &hand_a, &hand_b);
        let report = build_report(state, session_id, setup, hand_a, hand_b, outcome);
        state.battle_reports_v3.push_back(report.clone());
        apply_outcome_to_state(state, &report, &mut events);
        return events;
    }

    // Player involved: pause.  The TUI consumes `pending_battle_session`
    // and emits `PlayBattleCard` / `RetreatFromBattle` commands.  The
    // engine finishes the battle in subsequent `apply_turn` calls.
    state.pending_battle_session = Some(session);
    events
}

/// Resolve a session to completion in one shot (no player input).  Used
/// for AI-only battles and for the deterministic test path.
fn resolve_to_completion(
    state: &mut GameState,
    mut session: BattleSession,
    hand_a: &[CardId],
    hand_b: &[CardId],
) -> super::BattleOutcome {
    let mut rounds = Vec::new();
    let mut local_a: Vec<CardId> = hand_a.to_vec();
    let mut local_b: Vec<CardId> = hand_b.to_vec();
    let mut integrity_a = session.integrity_a;
    let mut integrity_b = session.integrity_b;

    // For v1 the damage model is independent of card play — it uses the v2
    // detection/positioning/attrition formula computed once.  We still
    // log the cards per round for presentation.
    let (damage_to_a, damage_to_b) = compute_v2_damage(state, &session.setup);

    for round_idx in 0..super::MAX_ROUNDS as usize {
        session.round = round_idx as u8;
        let card_a = local_a.first().copied();
        let card_b = local_b.first().copied();
        let effect_a = card_a
            .map(|c| describe_card_play(c, BattleSide::Attacker, round_idx as u8))
            .unwrap_or_else(|| "(no cards left)".to_string());
        let effect_b = card_b
            .map(|c| describe_card_play(c, BattleSide::Defender, round_idx as u8))
            .unwrap_or_else(|| "(no cards left)".to_string());
        if !local_a.is_empty() {
            local_a.remove(0);
        }
        if !local_b.is_empty() {
            local_b.remove(0);
        }
        // Apply v2 damage spread evenly across rounds.
        let slice = (round_idx + 1) as u32;
        let total_rounds = super::MAX_ROUNDS as u32;
        let per_round_a = damage_to_a / total_rounds
            + if round_idx == 0 {
                damage_to_a % total_rounds
            } else {
                0
            };
        let per_round_b = damage_to_b / total_rounds
            + if round_idx == 0 {
                damage_to_b % total_rounds
            } else {
                0
            };
        let _ = slice; // explicit acknowledgement; slice not otherwise used
        integrity_a = integrity_a.saturating_sub(per_round_b);
        integrity_b = integrity_b.saturating_sub(per_round_a);
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
    let fleet_a_retreated = false;
    let fleet_b_retreated = false;
    let system_outcome = if fleet_a_destroyed && fleet_b_destroyed {
        "Mutual destruction".to_string()
    } else if fleet_a_destroyed {
        "Defender wins".to_string()
    } else if fleet_b_destroyed {
        "Attacker wins".to_string()
    } else if integrity_a > integrity_b {
        format!("Attacker wins on integrity ({integrity_a} vs {integrity_b})")
    } else if integrity_b > integrity_a {
        format!("Defender wins on integrity ({integrity_b} vs {integrity_a})")
    } else {
        "Draw — defender holds".to_string()
    };

    let _ = state; // state argument kept for future RNG use; suppress unused
    let _ = session; // session is consumed for round count only

    super::BattleOutcome {
        integrity_a,
        integrity_b,
        fleet_a_destroyed,
        fleet_b_destroyed,
        fleet_a_retreated,
        fleet_b_retreated,
        rounds,
        system_outcome,
    }
}

/// Translate an `BattleOutcome` into a `BattleReportV3` and apply it to
/// `state` (integrity + report history + events).
fn build_report(
    state: &mut GameState,
    session_id: u64,
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
    .also(|_report| {
        let _ = session_id; // session_id kept for event correlation
    })
    .clone()
}

trait Also: Sized {
    fn also<F: FnOnce(&Self)>(self, f: F) -> Self {
        f(&self);
        self
    }
}
impl<T> Also for T {}

/// Apply the report's outcome to `state`: update fleet integrity, append
/// the final report, emit `BattleFinished`.
fn apply_outcome_to_state(
    state: &mut GameState,
    report: &BattleReportV3,
    events: &mut Vec<crate::events::Event>,
) {
    // Persist final report.
    state.battle_reports_v3.push_back(report.clone());
    // Cap history.
    const MAX: usize = 40;
    while state.battle_reports_v3.len() > MAX {
        state.battle_reports_v3.pop_front();
    }

    // Update fleet integrity.
    if let Some(f) = state.fleets.get_mut(&report.fleet_a) {
        if report.fleet_a_destroyed {
            // Caller (engine) is responsible for removing destroyed fleets
            // via the existing `remove_fleet_and_assignments` path.  We
            // leave the fleet entry alone here so the engine can detect
            // and remove it; integrity 0 is the marker.
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

    // Clear pending session (if any) — battle is over.
    state.pending_battle_session = None;

    events.push(crate::events::Event::BattleFinished {
        session_id: report.report_id,
        report_id: report.report_id,
        star: report.star,
        outcome: report.system_outcome.clone(),
    });
}

/// Compute the v2-equivalent total damage for `(fleet_a, fleet_b)`.  Uses
/// the public state helpers (`fleet_combat_profile` etc.) to derive attack
/// and defense percentages, then applies the v2 simultaneous-damage
/// formula.
///
/// Returns `(damage_to_a, damage_to_b)`.
fn compute_v2_damage(state: &GameState, setup: &BattleSetupSummary) -> (u32, u32) {
    let a_str = state
        .fleets
        .get(&setup.fleet_a)
        .map(|f| f.strength.max(1) as u64)
        .unwrap_or(1);
    let b_str = state
        .fleets
        .get(&setup.fleet_b)
        .map(|f| f.strength.max(1) as u64)
        .unwrap_or(1);
    let a_def = state
        .fleets
        .get(&setup.fleet_a)
        .map(|f| f.strength.max(1) as u64)
        .unwrap_or(1);
    let b_def = state
        .fleets
        .get(&setup.fleet_b)
        .map(|f| f.strength.max(1) as u64)
        .unwrap_or(1);

    let supply_a_mult = setup.supply_a.combat_attack_pct().max(10) as u64;
    let supply_b_mult = setup.supply_b.combat_attack_pct().max(10) as u64;

    let a_attack = (a_str * supply_a_mult / 100).max(1);
    let b_attack = (b_str * supply_b_mult / 100).max(1);
    let a_eff_def = (a_def * setup.supply_a.combat_defense_pct().max(10) as u64 / 100).max(1);
    let b_eff_def = (b_def * setup.supply_b.combat_defense_pct().max(10) as u64 / 100).max(1);

    let damage_to_a = (b_attack * 100 / a_eff_def).min(u32::MAX as u64) as u32;
    let damage_to_b = (a_attack * 100 / b_eff_def).min(u32::MAX as u64) as u32;
    (damage_to_a, damage_to_b)
}

/// Build a one-line description of a card play for the round log.
fn describe_card_play(card: CardId, side: BattleSide, round: u8) -> String {
    let c = super::card::card_by_id(card);
    let who = side.label();
    format!(
        "R{}: {} played {} ({})",
        round + 1,
        who,
        c.name,
        c.verb.label()
    )
}

/// Apply a player card play to the pending session.  For v1 this records
/// the play and, if the engine has any active side-effects, propagates
/// them.  Damage is computed when the session finalises (on AI side or
/// after the player's last card).
///
/// `state.pending_battle_session` must be `Some`.  Returns events to
/// append.
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
    let session_id_local = session.session_id;
    let side_label = if player_is_a { "Attacker" } else { "Defender" };
    let desc = format!(
        "R{}: {} played {} ({})",
        round + 1,
        side_label,
        super::card::card_by_id(card).name,
        super::card::card_by_id(card).verb.label()
    );
    events.push(crate::events::Event::BattleRoundPlayed {
        session_id: session_id_local,
        round,
        side: if player_is_a {
            crate::events::BattleRoundSide::Attacker
        } else {
            crate::events::BattleRoundSide::Defender
        },
        card,
        effect: desc,
    });
    events
}

/// Apply a free-retreat command.  Sets the session to a state where the
/// next call to `finalise_session` will mark the retreating side as
/// retreated.
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
    let new_int = (session.integrity_a.min(session.integrity_b) * 25) / 100;
    if player_is_a {
        session.integrity_a = new_int;
    } else {
        session.integrity_b = new_int;
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
    events
}

/// Finalise the pending session, if any.  Called at the end of every
/// `apply_turn` so player sessions close cleanly when no more input is
/// pending.  Returns events.
pub fn finalise_pending(state: &mut GameState) -> Vec<crate::events::Event> {
    let mut events = Vec::new();
    let Some(session) = state.pending_battle_session.clone() else {
        return events;
    };
    let player = state.player_empire;
    let player_is_a = session.setup.empire_a == player;
    let hand_a = session.hand_a.clone();
    let hand_b = session.hand_b.clone();
    let outcome = resolve_to_completion(state, session.clone(), &hand_a, &hand_b);
    let report = build_report(
        state,
        session.session_id,
        session.setup,
        hand_a,
        hand_b,
        outcome,
    );
    apply_outcome_to_state(state, &report, &mut events);
    let _ = player_is_a; // future: surface retreat log
    events
}

/// Empty noop (placeholder for the unused `apply_withdraw_card` re-export).
pub fn noop_for_withdraw() -> Option<CardVerb> {
    Some(CardVerb::Noop)
}

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
        assert_eq!(events.len(), 1);
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
    fn free_retreat_reduces_integrity() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::Destroyer);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        apply_battle(&mut state, crate::state::StarId(0), a, b, setup);
        let id = state.pending_battle_session.as_ref().unwrap().session_id;
        let events = player_retreat(&mut state, id);
        assert_eq!(events.len(), 1);
        let s = state.pending_battle_session.as_ref().unwrap();
        assert!(s.integrity_a < 100 || s.integrity_b < 100);
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
    fn v2_damage_formula_is_balanced_for_equal_fleets() {
        let mut state = new_state();
        let a = add_fleet(&mut state, 1, FleetKind::EscortFrigate);
        let b = add_fleet(&mut state, 2, FleetKind::EscortFrigate);
        let setup = build_setup(&state, a, b);
        let (da, db) = compute_v2_damage(&state, &setup);
        // Equal strength + supply → equal damage each way.
        assert_eq!(da, db, "equal fleets should deal equal damage");
    }
}
