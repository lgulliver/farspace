//! Combat v3 — battle report structures.
//!
//! `BattleReportV3` is the canonical post-battle record emitted when a
//! Combat v3 session finalises.  It supersedes the v2 `BattleReport` for
//! new battles but does not delete it; legacy reports remain in
//! `GameState::battle_reports` for history and save compatibility.

use crate::combat_v3::card::CardId;
use crate::state::{
    EmpireId, FleetFormation, FleetId, FleetKind, FleetRole, FleetSupplyState, StarId,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Pre-battle fleet setup summary embedded in the report.  Mirrors the v2
/// `BattleReport` setup fields; retained for downstream tools that read
/// the old schema.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BattleSetupSummary {
    pub role_a: FleetRole,
    pub role_b: FleetRole,
    pub formation_a: FleetFormation,
    pub formation_b: FleetFormation,
    pub doctrine_a: String,
    pub doctrine_b: String,
    pub supply_a: FleetSupplyState,
    pub supply_b: FleetSupplyState,
    pub kind_a: FleetKind,
    pub kind_b: FleetKind,
    pub ships_a: u32,
    pub ships_b: u32,
}

impl Default for BattleSetupSummary {
    fn default() -> Self {
        Self {
            role_a: FleetRole::StrikeFleet,
            role_b: FleetRole::DefenseFleet,
            formation_a: FleetFormation::Balanced,
            formation_b: FleetFormation::Balanced,
            doctrine_a: String::new(),
            doctrine_b: String::new(),
            supply_a: FleetSupplyState::Supplied,
            supply_b: FleetSupplyState::Supplied,
            kind_a: FleetKind::Destroyer,
            kind_b: FleetKind::EscortFrigate,
            ships_a: 1,
            ships_b: 1,
        }
    }
}

/// Per-round summary appended to a `BattleReportV3`.  The `Option` on the
/// card fields lets the round record reflect that one side declined to
/// play (e.g. out of cards or retreated early).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BattleRoundSummary {
    /// 1-based round number.
    pub round: u8,
    /// Card played by side A.  `None` if side A did not play this round.
    pub card_a: Option<CardId>,
    /// Card played by side B.  `None` if side B did not play this round.
    pub card_b: Option<CardId>,
    /// Short effect text shown in the report (e.g. "A Strike -18 hp").
    pub effect_a: String,
    /// Short effect text shown in the report.
    pub effect_b: String,
    /// Side A integrity after the round resolved.
    pub integrity_a_after: u32,
    /// Side B integrity after the round resolved.
    pub integrity_b_after: u32,
}

impl BattleRoundSummary {
    /// Build an empty round summary at the given round number with the
    /// current integrity values.  Used as a placeholder by tests.
    pub fn empty(round: u8, integrity_a: u32, integrity_b: u32) -> Self {
        Self {
            round,
            card_a: None,
            card_b: None,
            effect_a: String::new(),
            effect_b: String::new(),
            integrity_a_after: integrity_a,
            integrity_b_after: integrity_b,
        }
    }
}

/// Post-battle report emitted when a Combat v3 session finalises.
///
/// `report_id` is allocated by the engine on insertion into
/// `GameState::battle_reports_v3`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BattleReportV3 {
    pub report_id: u64,
    pub turn: u32,
    pub star: StarId,
    pub fleet_a: FleetId,
    pub fleet_b: FleetId,
    pub empire_a: EmpireId,
    pub empire_b: EmpireId,
    pub setup_summary: BattleSetupSummary,
    pub hand_a: Vec<CardId>,
    pub hand_b: Vec<CardId>,
    pub rounds: Vec<BattleRoundSummary>,
    pub integrity_a_start: u32,
    pub integrity_b_start: u32,
    pub integrity_a_end: u32,
    pub integrity_b_end: u32,
    pub fleet_a_destroyed: bool,
    pub fleet_b_destroyed: bool,
    pub fleet_a_retreated: bool,
    pub fleet_b_retreated: bool,
    pub system_outcome: String,
}

impl BattleReportV3 {
    /// Construct a report with the given setup and round log.  The
    /// `system_outcome` is left empty — the caller is expected to fill
    /// it in after computing the win/loss summary.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        report_id: u64,
        turn: u32,
        star: StarId,
        fleet_a: FleetId,
        fleet_b: FleetId,
        empire_a: EmpireId,
        empire_b: EmpireId,
        setup_summary: BattleSetupSummary,
        hand_a: Vec<CardId>,
        hand_b: Vec<CardId>,
        rounds: Vec<BattleRoundSummary>,
        integrity_a_start: u32,
        integrity_b_start: u32,
        integrity_a_end: u32,
        integrity_b_end: u32,
    ) -> Self {
        let fleet_a_destroyed = integrity_a_end == 0;
        let fleet_b_destroyed = integrity_b_end == 0;
        Self {
            report_id,
            turn,
            star,
            fleet_a,
            fleet_b,
            empire_a,
            empire_b,
            setup_summary,
            hand_a,
            hand_b,
            rounds,
            integrity_a_start,
            integrity_b_start,
            integrity_a_end,
            integrity_b_end,
            fleet_a_destroyed,
            fleet_b_destroyed,
            fleet_a_retreated: false,
            fleet_b_retreated: false,
            system_outcome: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_report_marks_destroyed_flags() {
        let report = BattleReportV3::new(
            1,
            5,
            StarId(1),
            FleetId(10),
            FleetId(11),
            EmpireId(1),
            EmpireId(2),
            BattleSetupSummary::default(),
            vec![],
            vec![],
            vec![],
            100,
            100,
            0,
            40,
        );
        assert!(report.fleet_a_destroyed);
        assert!(!report.fleet_b_destroyed);
    }

    #[test]
    fn empty_round_summary_records_integrity() {
        let r = BattleRoundSummary::empty(1, 80, 80);
        assert_eq!(r.round, 1);
        assert_eq!(r.integrity_a_after, 80);
        assert_eq!(r.integrity_b_after, 80);
        assert!(r.card_a.is_none());
        assert!(r.card_b.is_none());
    }
}
