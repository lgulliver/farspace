//! Centralized balance constants for FARSPACE.
//!
//! This module is the **single source of truth** for every numeric value that
//! affects game pacing, economic output, combat resolution, and diplomatic
//! triggers.  All constants are `pub` so that other crates (e.g. the TUI) can
//! display tuning information to the player, and so integration tests can
//! assert on them directly.
//!
//! # Pacing Intent
//!
//! The values here were tuned for the following target experience:
//!
//! * **Turns 1–15** — Exploration and first research unlocks.  The player
//!   should complete at least one T1 tech and see their first scout return
//!   data within 15 turns.
//! * **Turns 15–40** — First colonisation wave.  Survey Drones gating
//!   colonisation is deliberately cheap so this window feels achievable.
//! * **Turns 40–80** — Border contact with AI, early skirmishes.  Blockades
//!   and stability pressure start to shape strategic decisions.
//! * **Turns 80+** — Mid-game wars and economic differentiation.
//!
//! # Changing Constants
//!
//! Prefer modifying this file over hard-coding values anywhere else.  Every
//! constant documents its intended effect so reviewers can evaluate trade-offs
//! without hunting through the engine code.

// ---------------------------------------------------------------------------
// Research pacing
// ---------------------------------------------------------------------------

/// Multiplier applied to T1 tech costs.  Set to 1 (no scaling) so the raw
/// `TechRecord::cost` values drive pacing directly.
pub const EARLY_TECH_COST_MULTIPLIER: i64 = 1;

/// Base science output per population unit contributed by the yield model.
///
/// Increasing this shortens the research ladder; decreasing it stretches it.
pub const BASE_SCIENCE_PER_POP: i64 = 1;

// ---------------------------------------------------------------------------
// Colony growth
// ---------------------------------------------------------------------------

/// Number of turns between automatic population growth ticks.
///
/// **Changed from 12 → 10** to give a slightly faster early-game growth feel
/// and ensure colonies feel alive within the first two dozen turns.
pub const POP_GROWTH_PERIOD_TURNS: u32 = 10;

/// Minimum colony stability required for population to grow.
///
/// **Changed from 90 → 80** to be slightly more forgiving; colonies under
/// mild pressure (housing deficit or minor food issues) can still grow.
pub const MIN_STABILITY_FOR_POP_GROWTH: u8 = 80;

// ---------------------------------------------------------------------------
// Economic penalties
// ---------------------------------------------------------------------------

/// Yield percentage retained by an isolated (disconnected) colony.
///
/// Credits and science are multiplied by this value divided by 100.
/// Food is zeroed out entirely regardless of this setting.
pub const ISOLATED_YIELD_PERCENT: i64 = 50;

/// Stability lost per turn while a colony is isolated (not connected to the
/// empire supply network).
pub const ISOLATED_STABILITY_PENALTY: u8 = 5;

/// Maximum per-turn stability penalty from a housing deficit.
///
/// The actual penalty is `min(housing_deficit, MAX_HOUSING_DEFICIT_STABILITY_PENALTY)`.
pub const MAX_HOUSING_DEFICIT_STABILITY_PENALTY: u8 = 10;

/// Maximum per-turn stability penalty from unemployed population.
///
/// The actual penalty is `min(unemployed, MAX_UNEMPLOYMENT_STABILITY_PENALTY)`.
pub const MAX_UNEMPLOYMENT_STABILITY_PENALTY: u8 = 5;

/// Maximum per-turn stability penalty from a food deficit while isolated.
///
/// Only applied to colonies that are both food-deficient AND disconnected from
/// the empire supply network.
pub const MAX_ISOLATED_FOOD_DEFICIT_STABILITY_PENALTY: u8 = 5;

/// Stability lost per turn while a colony is under active blockade.
///
/// **Changed from 5 → 8** to make blockades a meaningful strategic lever;
/// players and AI must now respond to blockades within roughly a dozen turns
/// before significant destabilisation occurs.
pub const BLOCKADED_STABILITY_PENALTY: u8 = 8;

// ---------------------------------------------------------------------------
// Combat and invasion
// ---------------------------------------------------------------------------

/// Base invasion strength contributed by a single Troop Transport ship.
///
/// Total invasion strength = ships × (TROOP_TRANSPORT_INVASION_STRENGTH +
/// empire invasion bonus per transport).
pub const TROOP_TRANSPORT_INVASION_STRENGTH: u32 = 12;

/// Colony stability after a successful capture.
///
/// **Changed from 40 → 45** so captured worlds are slightly less crippled
/// and can more quickly become productive for their new owner.
pub const CAPTURED_UNREST_STABILITY: u8 = 45;

// ---------------------------------------------------------------------------
// Diplomacy thresholds
// ---------------------------------------------------------------------------

/// Squared-distance threshold for routine border pressure.
///
/// `40_000` ≈ 200 map units.  Close enough for neighbouring home systems and
/// early border colonies to feel contested without treating the whole sector
/// as immediate pressure.
pub const BORDER_TENSION_DISTANCE_SQ: i64 = 40_000;

/// Squared-distance threshold for severe border pressure.
///
/// `12_000` ≈ 110 map units.  Very close frontier overlap; the AI is allowed
/// to escalate toward its harshest diplomacy posture.
pub const SEVERE_BORDER_TENSION_DISTANCE_SQ: i64 = 12_000;

// ---------------------------------------------------------------------------
// Fleet travel
// ---------------------------------------------------------------------------

/// Fleet travel speed in galaxy coordinate units per turn.
///
/// Stars are generated in the range `-500..=500` on each axis, giving a
/// maximum possible distance of ≈1 414 units.  A speed of 500 yields:
///
/// * dist ≤ 500 → 1 turn  (close-range, same or adjacent sectors)
/// * dist ≤ 1 000 → 2 turns (medium-range)
/// * dist > 1 000 → 3 turns (long-range, far sectors)
pub const FLEET_TRAVEL_SPEED: f64 = 500.0;

/// Hyperspace lane travel divisor.  Direct lanes halve travel duration
/// (rounded up), so a 3-turn flight becomes 2 turns.
pub const HYPERSPACE_TRAVEL_DIVISOR: u32 = 2;

// ---------------------------------------------------------------------------
// Maintenance scaling
// ---------------------------------------------------------------------------

/// Soft credit cap for total fleet maintenance per empire.
///
/// Empires whose combined fleet maintenance exceeds this threshold receive
/// doubled upkeep on every fleet above the cap, discouraging fleet spam and
/// rewarding quality over quantity.
///
/// Set to 50 credits/turn, which is roughly 8–10 mid-tier combat ships.
pub const FLEET_MAINTENANCE_CREDIT_CAP_PER_EMPIRE: i64 = 50;
