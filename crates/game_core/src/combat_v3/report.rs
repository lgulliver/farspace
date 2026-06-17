//! BattleReportV3 — round-logged summary written after each battle.

use super::card::CardId;
use crate::state::{EmpireId, FleetFormation, FleetId, FleetRole, FleetSupplyState, StarId};

/// Per-round summary in a v3 battle report.  `card_a` and `card_b` are
/// `None` if the side had no cards left when the round resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BattleRoundSummary {
    pub round: u8,
    pub card_a: Option<CardId>,
    pub card_b: Option<CardId>,
    pub effect_a: String,
    pub effect_b: String,
    pub integrity_a_after: u32,
    pub integrity_b_after: u32,
}

/// Structured card-driven battle report.  Replaces the v2 `BattleReport`
/// for new battles; v2 reports are kept for legacy save compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BattleReportV3 {
    pub report_id: u64,
    pub turn: u32,
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
    pub integrity_a_end: u32,
    pub integrity_b_end: u32,
    pub fleet_a_destroyed: bool,
    pub fleet_b_destroyed: bool,
    pub fleet_a_retreated: bool,
    pub fleet_b_retreated: bool,
    pub hand_a: Vec<CardId>,
    pub hand_b: Vec<CardId>,
    pub rounds: Vec<BattleRoundSummary>,
    pub system_outcome: String,
}
