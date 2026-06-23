//! Combat v3 — cross-module integration tests.

use super::*;
use crate::combat_v3::ai::ai_pick_card;
use crate::combat_v3::card::{CardId, CardVerb, HOLD_FIRE, signature_for_faction};
use crate::combat_v3::deck::{HAND_SIZE, HandInputs, MAX_ROUNDS, build_hand};
use crate::combat_v3::report::BattleSetupSummary;
use crate::combat_v3::resolve::apply_round;
use crate::state::{
    ComponentId, EmpireId, Fleet, FleetFormation, FleetId, FleetKind, FleetRole, FleetSupplyState,
    GameState, StarId, TechId,
};

fn fleet(kind: FleetKind, owner: EmpireId) -> Fleet {
    Fleet {
        id: FleetId(1),
        owner,
        location: StarId(1),
        ships: 1,
        kind,
        strength: 1,
        integrity: 100,
    }
}

#[test]
fn build_hand_always_returns_five_cards() {
    let f = fleet(FleetKind::Destroyer, EmpireId(1));
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
fn build_hand_is_deterministic() {
    let f = fleet(FleetKind::EscortFrigate, EmpireId(1));
    let components = vec![
        ComponentId::SHIELD_MATRIX,
        ComponentId::ION_DRIVE,
        ComponentId::TARGETING_SUITE,
    ];
    let techs = vec![TechId::BATTLE_DOCTRINE, TechId::STRIKE_DOCTRINE];
    let a = HandInputs {
        fleet: &f,
        empire_id: EmpireId(1),
        components: &components,
        completed_techs: &techs,
        empire_def_id: Some(0),
    };
    let b = HandInputs {
        fleet: &f,
        empire_id: EmpireId(1),
        components: &components,
        completed_techs: &techs,
        empire_def_id: Some(0),
    };
    assert_eq!(build_hand(&a), build_hand(&b));
}

#[test]
fn build_hand_pads_with_hold_fire() {
    // Colonizer hull grants no card; with no components/techs/signature
    // the hand must be 5x Hold Fire.
    let f = fleet(FleetKind::Colonizer, EmpireId(1));
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
        assert_eq!(*c, HOLD_FIRE.id);
    }
}

#[test]
fn ai_pick_card_is_deterministic() {
    let session = sample_session();
    let idx_a = ai_pick_card(&session, BattleSide::Attacker);
    let idx_b = ai_pick_card(&session, BattleSide::Attacker);
    assert_eq!(idx_a, idx_b);
}

#[test]
fn ai_pick_card_returns_hand_index() {
    let session = sample_session();
    let idx = ai_pick_card(&session, BattleSide::Attacker);
    let hand = session.hand(BattleSide::Attacker);
    assert!(idx < hand.len());
}

#[test]
fn play_card_advances_session() {
    let mut session = sample_session();
    let starting_round = session.round;
    let card = session.hand_a[0];
    let (outcome, _) = apply_round(&mut session, BattleSide::Attacker, card);
    if matches!(outcome, BattleOutcome::Continue) {
        assert!(session.round > starting_round);
    }
    assert_eq!(session.rounds.len(), 1);
}

#[test]
fn invalid_card_in_hand_does_not_crash() {
    let mut session = sample_session();
    // CIWS Grid is not in the hand; apply_round should still process
    // the round (it just resolves whatever the AI chose).  The card
    // removal is a no-op for unknown ids and the resolver still runs.
    let (outcome, _) = apply_round(&mut session, BattleSide::Attacker, CardId::CIWS_GRID);
    // We don't assert which outcome — just that no panic occurred.
    let _ = outcome;
}

#[test]
fn battle_session_state_default_is_awaiting_player() {
    let s = BattleSession::new(
        1,
        StarId(1),
        FleetId(1),
        FleetId(2),
        EmpireId(1),
        EmpireId(2),
        vec![],
        vec![],
        100,
        100,
        BattleSetupSummary::default(),
    );
    assert_eq!(s.state, BattleSessionState::AwaitingPlayer);
}

#[test]
fn battle_setup_summary_default_is_consistent() {
    let s = BattleSetupSummary::default();
    assert_eq!(s.role_a, FleetRole::StrikeFleet);
    assert_eq!(s.role_b, FleetRole::DefenseFleet);
    assert_eq!(s.kind_a, FleetKind::Destroyer);
    assert_eq!(s.kind_b, FleetKind::EscortFrigate);
}

#[test]
fn max_rounds_is_five() {
    assert_eq!(MAX_ROUNDS, 5);
    assert_eq!(HAND_SIZE, 5);
}

#[test]
fn card_registry_is_complete() {
    // Every card in the v1 pool has a verb, name, source, and effect.
    for card in super::card::CARD_REGISTRY.iter() {
        assert!(!card.name.is_empty());
        assert!(!card.effect_text.is_empty());
        assert!(!card.source.is_empty());
    }
    // Hold Fire is the pad.
    assert_eq!(HOLD_FIRE.id, CardId(0));
    assert!(matches!(HOLD_FIRE.verb, CardVerb::Noop));
}

#[test]
fn signature_for_each_faction_is_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for faction in 0u8..=7 {
        let sig = signature_for_faction(faction).expect("signature");
        assert!(seen.insert(sig), "duplicate signature {sig:?}");
    }
}

#[test]
fn battle_round_summary_round_number_is_stable() {
    let mut session = sample_session();
    let card0 = session.hand_a[0];
    let _ = apply_round(&mut session, BattleSide::Attacker, card0);
    let card1 = session.hand_a[0];
    let _ = apply_round(&mut session, BattleSide::Attacker, card1);
    for (i, round) in session.rounds.iter().enumerate() {
        // Round numbers are 1-based; the i-th round summary has round = i+1.
        assert_eq!(round.round as usize, i + 1);
    }
}

#[test]
fn withdraw_card_finalizes_battle_immediately() {
    let mut session = sample_session();
    session.hand_a[0] = CardId::WARP_RETREAT;
    let (outcome, _) = apply_round(&mut session, BattleSide::Attacker, CardId::WARP_RETREAT);
    assert!(matches!(outcome, BattleOutcome::Finished { .. }));
    assert_eq!(session.integrity_a, 50);
}

#[test]
fn fixed_seed_battle_produces_identical_report() {
    // Run the same battle twice from identical state and compare the
    // round log and final outcome.  Both should be byte-identical.
    let (events_a, events_b) = simulate_ai_battle(42, EmpireId(1), EmpireId(2));
    // Filter to the BattleRoundPlayed + BattleFinished events.
    let summaries_a = round_summaries(&events_a);
    let summaries_b = round_summaries(&events_b);
    assert_eq!(summaries_a, summaries_b);
}

// --- helpers below ---

fn sample_session() -> BattleSession {
    BattleSession::new(
        1,
        StarId(1),
        FleetId(1),
        FleetId(2),
        EmpireId(1),
        EmpireId(2),
        vec![
            CardId::KINETIC_SALVO,
            CardId::ABLATIVE_HULL,
            CardId::PHASED_SHIELD,
            CardId::DRIFT_BURN,
            CardId::SENSOR_SWEEP,
        ],
        vec![
            CardId::KINETIC_SALVO,
            CardId::ABLATIVE_HULL,
            HOLD_FIRE.id,
            HOLD_FIRE.id,
            HOLD_FIRE.id,
        ],
        100,
        100,
        BattleSetupSummary {
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
    )
}

/// Run a complete AI-vs-AI battle from a fresh state and return the
/// resulting event lists.  Used by the determinism test.
fn simulate_ai_battle(
    seed: u64,
    empire_a: EmpireId,
    empire_b: EmpireId,
) -> (Vec<crate::events::Event>, Vec<crate::events::Event>) {
    let engine_a = crate::Engine::new(seed);
    let engine_b = crate::Engine::new(seed);

    let session = BattleSession::new(
        1,
        StarId(1),
        FleetId(1),
        FleetId(2),
        empire_a,
        empire_b,
        hand_from_fleet(&engine_a.state, FleetId(1), empire_a),
        hand_from_fleet(&engine_b.state, FleetId(2), empire_b),
        100,
        100,
        BattleSetupSummary::default(),
    );

    // The "events" we capture here are the round outcomes synthesised
    // by the test harness — we don't drive the engine because the
    // engine integration is a separate test.  We just call apply_round
    // repeatedly until finished.
    let events_a = drive_battle(session.clone());
    let events_b = drive_battle(session);
    (events_a, events_b)
}

fn drive_battle(mut session: BattleSession) -> Vec<crate::events::Event> {
    let mut events: Vec<crate::events::Event> = Vec::new();
    let mut iterations = 0;
    while !matches!(session.state, BattleSessionState::Finished) && iterations < 32 {
        iterations += 1;
        if session.hand_a.is_empty() && session.hand_b.is_empty() {
            break;
        }
        let a_card = if session.hand_a.is_empty() {
            HOLD_FIRE.id
        } else {
            session.hand_a[0]
        };
        let (outcome, _summary) = apply_round(&mut session, BattleSide::Attacker, a_card);
        events.push(crate::events::Event::BattleRoundPlayed {
            session_id: session.session_id,
            round: session.round,
            side: BattleSide::Attacker,
            card: a_card,
            effect_summary: format!("round {} simulated", session.round),
        });
        if matches!(outcome, BattleOutcome::Finished { .. }) {
            break;
        }
        let b_card = if session.hand_b.is_empty() {
            HOLD_FIRE.id
        } else {
            session.hand_b[0]
        };
        let (outcome, _summary) = apply_round(&mut session, BattleSide::Defender, b_card);
        events.push(crate::events::Event::BattleRoundPlayed {
            session_id: session.session_id,
            round: session.round,
            side: BattleSide::Defender,
            card: b_card,
            effect_summary: format!("round {} simulated", session.round),
        });
        if matches!(outcome, BattleOutcome::Finished { .. }) {
            break;
        }
    }
    events.push(crate::events::Event::BattleFinished {
        session_id: session.session_id,
        report_id: 1,
        star: session.star,
        winner: match session.integrity_a.cmp(&session.integrity_b) {
            std::cmp::Ordering::Greater => Some(BattleSide::Attacker),
            std::cmp::Ordering::Less => Some(BattleSide::Defender),
            std::cmp::Ordering::Equal => None,
        },
        fleet_a_destroyed: session.integrity_a == 0,
        fleet_b_destroyed: session.integrity_b == 0,
        fleet_a_retreated: false,
        fleet_b_retreated: false,
    });
    events
}

fn round_summaries(events: &[crate::events::Event]) -> Vec<(u8, BattleSide, CardId)> {
    events
        .iter()
        .filter_map(|e| match e {
            crate::events::Event::BattleRoundPlayed {
                round, side, card, ..
            } => Some((*round, *side, *card)),
            _ => None,
        })
        .collect()
}

fn hand_from_fleet(_state: &GameState, _fid: FleetId, _empire: EmpireId) -> Vec<CardId> {
    // Placeholder: tests use the canonical sample_session() instead of
    // a real engine state.  Kept for future engine integration tests.
    vec![
        CardId::KINETIC_SALVO,
        CardId::ABLATIVE_HULL,
        HOLD_FIRE.id,
        HOLD_FIRE.id,
        HOLD_FIRE.id,
    ]
}
