//! Combat v3 — card-driven battle resolution
//!
//! Each side drafts a 5-card hand from fleet composition, ship components,
//! and unlocked techs. Each round, each side plays one card. Cards drive
//! presentation and targeting; the v1 damage model is the same deterministic
//! auto-resolve formula as Combat v2 (future slices will replace the
//! formula with per-verb damage rules).
//!
//! Determinism rules:
//!
//! - Card draft is a pure function of `(fleet, empire_state)`. No RNG during
//!   draft.
//! - RNG inside resolve (if any) pulls from `GameState.rng`.
//! - AI card selection is deterministic; ties broken by `CardId` ascending.

#![forbid(unsafe_code)]
// The v3 module is alpha.  Stylistic clippy lints (collapsible ifs,
// needless lifetimes, let-binding returns, etc.) are deferred to a
// follow-up cleanup PR; the priority here is functional correctness.
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::let_and_return)]
#![allow(clippy::unnecessary_filter_map)]

pub mod ai;
pub mod card;
pub mod deck;
pub mod report;
pub mod resolve;
pub mod withdraw;

pub use card::{CardDef, CardEffect, CardId, CardSource, CardVerb};
pub use deck::build_hand;
pub use report::{BattleReportV3, BattleRoundSummary};
pub use resolve::{apply_battle, finalise_pending, play_card, player_retreat};
pub use withdraw::{apply_withdraw_card, free_retreat};

use crate::state::{
    EmpireId, FleetFormation, FleetId, FleetRole, FleetSupplyState, GameState, StarId,
};

/// Hand size for v1.  Padded with `card::HOLD_FIRE` (id 0) when the
/// deterministic pool yields fewer than 5 cards.
pub const HAND_SIZE: usize = 5;

/// Maximum rounds before tiebreaker.
pub const MAX_ROUNDS: u8 = 5;

/// Which side of the engagement this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BattleSide {
    /// The side that initiated the engagement (arriving fleet at a star).
    Attacker,
    /// The side that was present at the engagement location first.
    Defender,
}

impl BattleSide {
    pub fn other(self) -> Self {
        match self {
            BattleSide::Attacker => BattleSide::Defender,
            BattleSide::Defender => BattleSide::Attacker,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BattleSide::Attacker => "Attacker",
            BattleSide::Defender => "Defender",
        }
    }
}

/// Setup summary carried into the v3 report and into the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BattleSetupSummary {
    pub star: StarId,
    pub fleet_a: FleetId,
    pub fleet_b: FleetId,
    pub empire_a: EmpireId,
    pub empire_b: EmpireId,
    pub role_a: FleetRole,
    pub role_b: FleetRole,
    pub formation_a: FleetFormation,
    pub formation_b: FleetFormation,
    pub supply_a: FleetSupplyState,
    pub supply_b: FleetSupplyState,
    pub ships_a: u32,
    pub ships_b: u32,
    pub integrity_a_start: u32,
    pub integrity_b_start: u32,
    pub doctrine_a: String,
    pub doctrine_b: String,
}

/// Live state of a v3 battle.  Lives in `GameState.pending_battle_session`
/// while the player is playing cards.  Removed once the battle finalises;
/// the result is appended to `GameState.battle_reports_v3`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BattleSession {
    pub session_id: u64,
    pub setup: BattleSetupSummary,
    /// Hand for side A (attacker).  Length ≤ HAND_SIZE.
    pub hand_a: Vec<CardId>,
    /// Hand for side B (defender).  Length ≤ HAND_SIZE.
    pub hand_b: Vec<CardId>,
    /// 0-based round index; `0` means round 1.
    pub round: u8,
    pub integrity_a: u32,
    pub integrity_b: u32,
    pub phase: BattlePhase,
}

/// Phase of a v3 session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BattlePhase {
    /// Awaiting the next player or AI card play.
    AwaitingInput,
    /// Battle finished; `report_id` is pending write into the report log.
    Finished,
}

impl BattleSession {
    pub fn is_finished(&self) -> bool {
        matches!(self.phase, BattlePhase::Finished) || self.round >= MAX_ROUNDS
    }
}

/// Result of one battle resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleOutcome {
    pub integrity_a: u32,
    pub integrity_b: u32,
    pub fleet_a_destroyed: bool,
    pub fleet_b_destroyed: bool,
    pub fleet_a_retreated: bool,
    pub fleet_b_retreated: bool,
    pub rounds: Vec<BattleRoundSummary>,
    pub system_outcome: String,
}

/// Look up the pending battle session by id.  Returns `None` if the session
/// has already finalised.
pub fn find_session(state: &GameState, session_id: u64) -> Option<&BattleSession> {
    state
        .pending_battle_session
        .as_ref()
        .filter(|s| s.session_id == session_id)
}
