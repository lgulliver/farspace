//! Combat v3 — card-driven battle resolution.
//!
//! This module is the public surface for Combat v3.  It exposes the
//! battle session model, the hand-draft entrypoint, the round resolver,
//! the AI card picker, and the post-battle report type.  The TUI must
//! not make any combat decisions; it only renders [`BattleSession`]
//! snapshots and emits commands.
//!
//! Determinism: every entrypoint is a pure function of the inputs.  No
//! RNG is consumed during hand-draft or round resolution.  Card draft
//! iteration uses `BTreeSet`/`BTreeMap` or sorted slices; no `HashMap`
//! is iterated.

pub mod ai;
pub mod card;
pub mod deck;
pub mod report;
pub mod resolve;

#[cfg(test)]
mod tests;

pub use card::{
    CARD_REGISTRY, CardDef, CardId, CardVerb, HOLD_FIRE, card_by_id, signature_for_faction,
};
pub use deck::{
    HAND_SIZE, HandInputs, MAX_ROUNDS, build_hand, component_card_for, hull_card_for_kind,
    tech_card_for,
};
pub use report::{BattleReportV3, BattleRoundSummary, BattleSetupSummary};
pub use resolve::{BattleOutcome, apply_round};

use crate::state::{EmpireId, Fleet, FleetId, StarId};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Which side of the battle a command is acting on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BattleSide {
    Attacker,
    Defender,
}

impl BattleSide {
    /// The opposite side.
    pub fn other(self) -> BattleSide {
        match self {
            BattleSide::Attacker => BattleSide::Defender,
            BattleSide::Defender => BattleSide::Attacker,
        }
    }

    /// Short label for display.
    pub fn label(self) -> &'static str {
        match self {
            BattleSide::Attacker => "Attacker",
            BattleSide::Defender => "Defender",
        }
    }
}

/// State of a Combat v3 session.
///
/// `AwaitingPlayer` means the player must issue `PlayBattleCard` or
/// `RetreatFromBattle`.  `Resolving` is reserved for future use — in v1
/// resolution is synchronous and the session never lingers in
/// `Resolving`.  `Finished` means the session has been finalised and
/// the report has been pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BattleSessionState {
    #[default]
    AwaitingPlayer,
    Resolving,
    Finished,
}

/// Active Combat v3 battle.  Lives in `GameState::pending_battle_session`
/// while in progress; cleared on finalisation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BattleSession {
    /// Monotonic id assigned by the engine when the session is created.
    pub session_id: u64,
    /// Star system where the engagement is taking place.
    pub star: StarId,
    /// Fleet id of the attacker.
    pub attacker: FleetId,
    /// Fleet id of the defender.
    pub defender: FleetId,
    /// Empire id of the attacker.
    pub empire_a: EmpireId,
    /// Empire id of the defender.
    pub empire_b: EmpireId,
    /// Attacker hand.  Length is ≤ 5; shrinks by 1 per played card.
    pub hand_a: Vec<CardId>,
    /// Defender hand.  Length is ≤ 5.
    pub hand_b: Vec<CardId>,
    /// Current attacker integrity (0 = destroyed).
    pub integrity_a: u32,
    /// Current defender integrity (0 = destroyed).
    pub integrity_b: u32,
    /// Attacker integrity at session start.
    pub integrity_a_start: u32,
    /// Defender integrity at session start.
    pub integrity_b_start: u32,
    /// 1-based round number.  Increments after each resolved round.
    pub round: u8,
    /// Pre-battle setup summary (carries v2 fields for report compatibility).
    pub setup_summary: BattleSetupSummary,
    /// Per-round resolution log.
    pub rounds: Vec<BattleRoundSummary>,
    /// Current session state.
    pub state: BattleSessionState,
}

impl BattleSession {
    /// Build a fresh session from the given inputs.  Used by
    /// `start_battle_v3` in the engine; tests construct sessions
    /// directly.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: u64,
        star: StarId,
        attacker: FleetId,
        defender: FleetId,
        empire_a: EmpireId,
        empire_b: EmpireId,
        hand_a: Vec<CardId>,
        hand_b: Vec<CardId>,
        integrity_a: u32,
        integrity_b: u32,
        setup_summary: BattleSetupSummary,
    ) -> Self {
        Self {
            session_id,
            star,
            attacker,
            defender,
            empire_a,
            empire_b,
            hand_a,
            hand_b,
            integrity_a,
            integrity_b,
            integrity_a_start: integrity_a,
            integrity_b_start: integrity_b,
            round: 1,
            setup_summary,
            rounds: Vec::new(),
            state: BattleSessionState::AwaitingPlayer,
        }
    }

    /// The hand of the given side.
    pub fn hand(&self, side: BattleSide) -> &[CardId] {
        match side {
            BattleSide::Attacker => &self.hand_a,
            BattleSide::Defender => &self.hand_b,
        }
    }

    /// Mutate the hand of the given side.
    pub fn hand_mut(&mut self, side: BattleSide) -> &mut Vec<CardId> {
        match side {
            BattleSide::Attacker => &mut self.hand_a,
            BattleSide::Defender => &mut self.hand_b,
        }
    }

    /// The integrity of the given side.
    pub fn integrity(&self, side: BattleSide) -> u32 {
        match side {
            BattleSide::Attacker => self.integrity_a,
            BattleSide::Defender => self.integrity_b,
        }
    }

    /// The empire id of the given side.
    pub fn empire(&self, side: BattleSide) -> EmpireId {
        match side {
            BattleSide::Attacker => self.empire_a,
            BattleSide::Defender => self.empire_b,
        }
    }

    /// The fleet id of the given side.
    pub fn fleet(&self, side: BattleSide) -> FleetId {
        match side {
            BattleSide::Attacker => self.attacker,
            BattleSide::Defender => self.defender,
        }
    }
}

/// Apply a player card.  Returns the outcome and the new round summary
/// for the engine to emit `BattleRoundPlayed` and (if the round ended
/// the battle) `BattleFinished`.
pub fn play_player_card(
    session: &mut BattleSession,
    player_side: BattleSide,
    card: CardId,
) -> (BattleOutcome, BattleRoundSummary) {
    apply_round(session, player_side, card)
}

/// Apply a free retreat from the player side.  Burns the current round,
/// reduces the player's integrity to 50% (clamped at 0), and finalises
/// the session.
pub fn apply_retreat(session: &mut BattleSession, side: BattleSide) {
    // Snapshot the pre-retreat integrity for the retreating side, then
    // halve it directly in a single match.  No unused-variable dance.
    let pre = match side {
        BattleSide::Attacker => session.integrity_a,
        BattleSide::Defender => session.integrity_b,
    };
    let halved = pre / 2;
    match side {
        BattleSide::Attacker => session.integrity_a = halved,
        BattleSide::Defender => session.integrity_b = halved,
    }
    session.state = BattleSessionState::Finished;
}

/// Build a `BattleSetupSummary` for the given fleets and empires using
/// the same data the engine has on hand.  Used by `start_battle_v3`.
#[allow(clippy::too_many_arguments)]
pub fn build_setup_summary(
    fleet_a: &Fleet,
    fleet_b: &Fleet,
    role_a: crate::state::FleetRole,
    role_b: crate::state::FleetRole,
    formation_a: crate::state::FleetFormation,
    formation_b: crate::state::FleetFormation,
    supply_a: crate::state::FleetSupplyState,
    supply_b: crate::state::FleetSupplyState,
    doctrine_a: String,
    doctrine_b: String,
) -> BattleSetupSummary {
    BattleSetupSummary {
        role_a,
        role_b,
        formation_a,
        formation_b,
        doctrine_a,
        doctrine_b,
        supply_a,
        supply_b,
        kind_a: fleet_a.kind,
        kind_b: fleet_b.kind,
        ships_a: fleet_a.ships,
        ships_b: fleet_b.ships,
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn battle_side_other_flips() {
        assert_eq!(BattleSide::Attacker.other(), BattleSide::Defender);
        assert_eq!(BattleSide::Defender.other(), BattleSide::Attacker);
    }

    #[test]
    fn battle_session_default_state_is_awaiting_player() {
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
        assert_eq!(s.round, 1);
        assert!(s.rounds.is_empty());
    }

    #[test]
    fn hand_and_integrity_helpers_index_by_side() {
        let s = BattleSession::new(
            1,
            StarId(1),
            FleetId(1),
            FleetId(2),
            EmpireId(1),
            EmpireId(2),
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
            80,
            60,
            BattleSetupSummary::default(),
        );
        assert_eq!(s.hand(BattleSide::Attacker).len(), 5);
        assert_eq!(s.hand(BattleSide::Defender).len(), 5);
        assert_eq!(s.integrity(BattleSide::Attacker), 80);
        assert_eq!(s.integrity(BattleSide::Defender), 60);
        assert_eq!(s.empire(BattleSide::Attacker), EmpireId(1));
        assert_eq!(s.empire(BattleSide::Defender), EmpireId(2));
        assert_eq!(s.fleet(BattleSide::Attacker), FleetId(1));
        assert_eq!(s.fleet(BattleSide::Defender), FleetId(2));
    }
}
