use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Deterministic victory-path ordering. Earlier variants win ties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VictoryPath {
    /// Last surviving major empire wins.
    Supremacy,
    /// Wide-empire control victory: hold ≥ threshold of colonized systems for N consecutive turns.
    Ascendancy,
    /// Late-game science / project victory: complete the late-game tech and accumulate a
    /// deterministic project threshold.
    Scientific,
    /// Turn-limit prestige victory: highest Legacy score when the turn limit is reached.
    Legacy,
}

impl VictoryPath {
    /// Display label for the path (e.g. `"Supremacy"`).
    pub fn label(self) -> &'static str {
        match self {
            VictoryPath::Supremacy => "Supremacy",
            VictoryPath::Ascendancy => "Ascendancy",
            VictoryPath::Scientific => "Scientific",
            VictoryPath::Legacy => "Legacy",
        }
    }

    /// Compact three-letter abbreviation used in tight UI rows.
    pub fn short(self) -> &'static str {
        match self {
            VictoryPath::Supremacy => "Sup",
            VictoryPath::Ascendancy => "Asc",
            VictoryPath::Scientific => "Sci",
            VictoryPath::Legacy => "Leg",
        }
    }

    /// Order used to break ties when multiple paths can fire in the same turn.
    pub fn tie_break_order() -> &'static [VictoryPath] {
        &[
            VictoryPath::Supremacy,
            VictoryPath::Ascendancy,
            VictoryPath::Scientific,
            VictoryPath::Legacy,
        ]
    }
}

/// Coarse status of a single victory path for an empire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VictoryPathStatus {
    /// Path is in play; the empire has not yet satisfied the requirement.
    #[default]
    InProgress,
    /// An empire has satisfied the path this turn (or earlier) and won.
    Achieved,
    /// Path is turned off in the scenario settings.
    Disabled,
}

/// Configurable thresholds for each victory path. Stored inside `VictorySettings`
/// on the scenario. The default `v1` settings enable all four paths and a 300-turn limit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VictoryCondition {
    /// Supremacy is gated only by the liveness rule; no numeric threshold.
    Supremacy,
    /// Ascendancy: hold ≥ `control_percent` of unique colonized systems for
    /// `consecutive_turns_required` consecutive turns. `control_percent` is 1–100.
    Ascendancy {
        control_percent: u8,
        consecutive_turns_required: u32,
    },
    /// Scientific: complete the late-game tech `eligibility_tech` and then accumulate
    /// `project_points_required` science/industry points (per empire, summed across turns).
    Scientific {
        eligibility_tech: TechId,
        project_points_required: i64,
    },
    /// Legacy: scored at the turn limit (when no other path has fired). Thresholds
    /// only affect early-warning / leader reporting, not the actual score.
    Legacy { early_warning_percent: u8 },
}

impl VictoryCondition {
    /// Returns the path this condition configures.
    pub fn path(&self) -> VictoryPath {
        match self {
            VictoryCondition::Supremacy => VictoryPath::Supremacy,
            VictoryCondition::Ascendancy { .. } => VictoryPath::Ascendancy,
            VictoryCondition::Scientific { .. } => VictoryPath::Scientific,
            VictoryCondition::Legacy { .. } => VictoryPath::Legacy,
        }
    }
}

/// Player / scenario configured victory settings.
///
/// All fields use `#[serde(default)]` so older saves (pre-v42) deserialise cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VictorySettings {
    /// Which paths are currently enabled. Mirrors the conditions list, but is the
    /// authoritative enable flag for evaluation. `#[serde(default)]` so old saves
    /// without this field get a sensible default.
    #[cfg_attr(feature = "serde", serde(default))]
    pub enabled_paths: BTreeSet<VictoryPath>,
    /// Threshold configuration for each path (in `VictoryPath::tie_break_order()` order).
    #[cfg_attr(feature = "serde", serde(default))]
    pub conditions: Vec<VictoryCondition>,
    /// Whether the campaign enforces a turn limit. When true, `Legacy` becomes the
    /// fallback winner at `turn_limit` if no other path has fired.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub turn_limit_enabled: bool,
    /// Number of turns after which the turn-limit `Legacy` evaluation fires.
    /// Default is 300.
    #[cfg_attr(feature = "serde", serde(default = "default_turn_limit"))]
    pub turn_limit: u32,
}

impl VictorySettings {
    /// Default v1 victory configuration: all four paths enabled, 300-turn limit.
    ///
    /// The Scientific eligibility tech is `Transcendent Gate Theory`
    /// (`TechId(61)`); a player must research it before the project
    /// point counter starts ticking.
    pub fn default_v1() -> Self {
        let enabled_paths = [
            VictoryPath::Supremacy,
            VictoryPath::Ascendancy,
            VictoryPath::Scientific,
            VictoryPath::Legacy,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let conditions = vec![
            VictoryCondition::Supremacy,
            VictoryCondition::Ascendancy {
                control_percent: 50,
                consecutive_turns_required: 10,
            },
            VictoryCondition::Scientific {
                // Transcendent Gate Theory — the late-game unlock that
                // gates the Scientific project.  The point counter only
                // advances once the player has researched this tech.
                eligibility_tech: TechId(61),
                project_points_required: 1_500,
            },
            VictoryCondition::Legacy {
                early_warning_percent: 75,
            },
        ];
        Self {
            enabled_paths,
            conditions,
            turn_limit_enabled: true,
            turn_limit: 300,
        }
    }

    /// Returns the configured condition for `path`, if any.  The
    /// conditions list is stored in insertion order; later entries
    /// for the same path are ignored.
    pub fn condition_for(&self, path: VictoryPath) -> Option<&VictoryCondition> {
        self.conditions
            .iter()
            .find(|condition| condition.path() == path)
    }

    /// True when `path` is in the `enabled_paths` set.
    pub fn is_enabled(&self, path: VictoryPath) -> bool {
        self.enabled_paths.contains(&path)
    }
}

impl Default for VictorySettings {
    fn default() -> Self {
        Self::default_v1()
    }
}

/// Deterministic breakdown of an empire's Legacy score. Each component is
/// computed from existing game state so the score is fully explainable to the
/// player (and to the TUI). Components are summed in the order they appear.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LegacyScoreBreakdown {
    /// +1 per colony owned by the empire.
    pub colonies: i64,
    /// +1 per unit of total population.
    pub population: i64,
    /// +5 per completed technology.
    pub completed_technologies: i64,
    /// +1 per system explored by the empire.
    pub explored_systems: i64,
    /// +1 per surveyed planet in the empire's explored footprint.
    pub surveyed_planets: i64,
    /// +3 per discovered special, +5 per detected anomaly, +4 per strategic
    /// resource under extraction.
    pub discoveries_and_resources: i64,
    /// +20 per recorded battle win in `state.battle_reports` involving the empire.
    pub battle_victories: i64,
    /// +1 per 10 credits of liquid treasury, floored.
    pub credits: i64,
    /// Total of the components above.
    pub total: i64,
}

/// Per-empire victory progress snapshot. Stored on the `GameState` so the TUI
/// can render it without re-deriving the whole tree, and so it round-trips
/// through save/load.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmpireVictoryProgress {
    /// Per-path status (mirrors `VictorySettings::enabled_paths` for the
    /// 4-path v1 set).
    #[cfg_attr(feature = "serde", serde(default))]
    pub path_status: BTreeMap<VictoryPath, VictoryPathStatus>,
    /// Ascendancy hold counter (consecutive turns ≥ threshold).
    #[cfg_attr(feature = "serde", serde(default))]
    pub ascendancy_hold_turns: u32,
    /// Whether the empire currently meets the Ascendancy threshold.
    #[cfg_attr(feature = "serde", serde(default))]
    pub ascendancy_meets_threshold: bool,
    /// Cumulative Scientific project points accumulated toward the threshold.
    /// Only advances after the empire has researched the eligibility tech.
    #[cfg_attr(feature = "serde", serde(default))]
    pub scientific_project_points: i64,
    /// Whether the empire has researched the Scientific eligibility tech.
    #[cfg_attr(feature = "serde", serde(default))]
    pub scientific_eligible: bool,
    /// Legacy score snapshot + transparent breakdown for the current turn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub legacy_breakdown: LegacyScoreBreakdown,
    /// Last warning milestone already emitted for this empire/path (0 = none).
    /// 25 / 50 / 75 / 90 are the standard Scientific warning bands.
    #[cfg_attr(feature = "serde", serde(default))]
    pub warning_milestones: BTreeMap<VictoryPath, u8>,
}

/// Outcome of a fully resolved campaign. `None` while the campaign is still
/// in progress. Stored on `GameState::final_victory`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FinalVictory {
    /// Empire that won the campaign.
    pub winner: EmpireId,
    /// Path that fired.
    pub path: VictoryPath,
    /// Turn number on which the path was satisfied.
    pub turn: u32,
    /// Human-readable reason string used by the TUI Victory screen and event log.
    pub reason: String,
}

/// Per-empire aggregate victory view. Mirrors the four `VictoryCondition`
/// variants and exposes a single `progress_percent` so the TUI can display a
/// concise bar without switching on each `VictoryCondition` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VictoryProgress {
    pub path: VictoryPath,
    pub status: VictoryPathStatus,
    pub progress_percent: u8,
    pub leading_empire: Option<EmpireId>,
}

/// Key for the per-(empire, path) milestone map. Tuple keys are not
/// supported by serde's JSON map encoder, so we wrap the pair in a small
/// named struct with deterministic `Ord` for `BTreeMap` storage. The
/// serde representation is a deterministic string of the form
/// `"<empire>:<path_label>"`, which serde_json accepts as a map key.
///
/// The empire id is stored as a full `u64` (no truncation) so collisions
/// like `EmpireId(1)` vs `EmpireId(65_537)` cannot occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MilestoneKey {
    empire: EmpireId,
    path: VictoryPath,
}

impl MilestoneKey {
    /// Build a milestone key from an empire id and a path.  Both
    /// components are stored at full fidelity, so the key
    /// round-trips losslessly through equality checks and serde.
    pub const fn new(empire: EmpireId, path: VictoryPath) -> Self {
        Self { empire, path }
    }

    /// Empire half of the pair.
    pub const fn empire(self) -> EmpireId {
        self.empire
    }

    /// Path half of the pair.
    pub const fn path(self) -> VictoryPath {
        self.path
    }
}

#[cfg(feature = "serde")]
impl Serialize for MilestoneKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}:{}", self.empire.0, self.path.label()))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MilestoneKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let (emp, path) = s
            .split_once(':')
            .ok_or_else(|| serde::de::Error::custom("MilestoneKey must be empire:path"))?;
        let empire_id: u64 = emp
            .parse()
            .map_err(|_| serde::de::Error::custom("invalid empire id in MilestoneKey"))?;
        let path = match path {
            "Supremacy" => VictoryPath::Supremacy,
            "Ascendancy" => VictoryPath::Ascendancy,
            "Scientific" => VictoryPath::Scientific,
            "Legacy" => VictoryPath::Legacy,
            _ => return Err(serde::de::Error::custom("unknown VictoryPath")),
        };
        Ok(MilestoneKey::new(EmpireId(empire_id), path))
    }
}

/// Top-level victory state stored on `GameState::victory_status`.
///
/// `progress` always lists all four `VictoryPath` variants in tie-break order so
/// the TUI never needs to special-case missing rows. `final_victory` is `None`
/// while the campaign is still in play; once set, no further `VictoryAchieved`
/// events fire.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VictoryStatus {
    /// Per-path progress snapshot, ordered for deterministic rendering.
    #[cfg_attr(feature = "serde", serde(default))]
    pub progress: Vec<VictoryProgress>,
    /// Per-empire progress (status, Ascendancy hold counter, Scientific points,
    /// Legacy breakdown, warning milestones). Keyed by `EmpireId`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub per_empire: BTreeMap<EmpireId, EmpireVictoryProgress>,
    /// Final, terminal victory outcome. `None` while the campaign continues.
    #[cfg_attr(feature = "serde", serde(default))]
    pub final_victory: Option<FinalVictory>,
    /// Per-empire recorded highest-path-warning milestone (25/50/75/90). Kept
    /// here as well as inside `per_empire` so dispatch logic can scan it
    /// without a per-empire lookup. For v1 these two are written together.
    #[cfg_attr(feature = "serde", serde(default))]
    pub milestone_levels: BTreeMap<MilestoneKey, u8>,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

#[cfg(feature = "serde")]
fn default_turn_limit() -> u32 {
    300
}

#[cfg(test)]
mod milestone_key_tests {
    use super::*;

    /// Regression guard for the legacy truncation bug: `MilestoneKey`
    /// previously stored only the low 16 bits of `EmpireId`, which
    /// meant `EmpireId(1)` and `EmpireId(65_537)` collided.  Two
    /// distinct empires should always map to distinct keys.
    #[test]
    fn milestone_keys_with_distant_empire_ids_do_not_collide() {
        let a = MilestoneKey::new(EmpireId(1), VictoryPath::Supremacy);
        let b = MilestoneKey::new(EmpireId(65_537), VictoryPath::Supremacy);
        let c = MilestoneKey::new(EmpireId(1), VictoryPath::Ascendancy);
        assert_ne!(a, b, "distant empire ids must not collide on the same path");
        assert_ne!(a, c, "different paths must produce distinct keys");
        assert_eq!(a.empire(), EmpireId(1));
        assert_eq!(b.empire(), EmpireId(65_537));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn milestone_key_round_trips_full_empire_id() {
        let key = MilestoneKey::new(EmpireId(65_537), VictoryPath::Ascendancy);
        let json = serde_json::to_string(&key).expect("serialize");
        let restored: MilestoneKey = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, key);
        assert_eq!(restored.empire(), EmpireId(65_537));
        assert_eq!(restored.path(), VictoryPath::Ascendancy);
    }
}
