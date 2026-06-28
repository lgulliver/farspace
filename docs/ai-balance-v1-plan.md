# AI & Balance V1 — Implementation Plan

## Executive Summary

18 targeted changes across 8 areas. Every change is purely additive or replacive within existing modules. No new crates. No new gameplay systems. All changes deterministic, headless, and save-compatible.

Estimated total: ~2500 lines changed across ~15 files.

---

## 1. Large Galaxy Support (`state.rs`, `galaxy.rs`, `engine/setup.rs`)

### 1a. New GalaxySize variants

Add `Tiny`, `Huge`, `Epic`, and a `Custom(u32)` variant to `GalaxySize` enum.

| Size | Stars | Sectors |
|------|-------|---------|
| Tiny | 40 | 3 |
| Small | 80 | 4 |
| Medium (default) | 150 | 6 |
| Large | 250 | 8 |
| Huge | 400 | 12 |
| Epic | 700 | 16 |

Update `ScenarioSetup` to accept an optional `star_count_override: Option<u32>` alongside the existing `sector_count_override`.

Update `NewGameSetupState` in TUI to expose the new sizes. Extend `effective_star_count()` to use the override when present.

### 1b. Galactic Scaling

Add `star_count_for_size(size: GalaxySize) -> usize` and `sector_count_for_size(size: GalaxySize) -> usize` to `GalaxySize`.

Update the new-game-setup screen to cycle through all 7 options (Tiny → Small → Medium → Large → Huge → Epic → Custom).

### 1c. Galaxy Generation Overhead

Review `generate_galaxy()` for O(n²) or O(n·m) loops. Replace full-star iteration in per-planet discovery generation with per-star iteration that batches planet contexts. Verify hyperspace lane generation (O(n log n) with Delaunay-like k-nearest approach).

Current lane generation: `generate_hyperspace_lanes` — determine algorithm (likely O(n²) naive). Replace with bounded k-nearest-neighbor over sector-partitioned stars so Epic (700 stars) generates in < 500ms.

**Files:** `crates/game_core/src/state.rs`, `crates/game_core/src/galaxy.rs`, `crates/game_core/src/engine/setup.rs`, `crates/game_tui/src/screens/new_game_setup.rs`

---

## 2. Better Galaxy Generation (`galaxy.rs`)

### 2a. Constellation Clusters

Add a `constellation_id: u32` field to `Star`. Generate 3–8 star clusters per sector via seeded Poisson-disc sampling. Stars within the same constellation are closer together (50–100 units); gaps between constellations are larger (200–400 units). This creates natural borders, choke points, and territorial pockets.

Algorithm:
1. For each sector, compute `constellation_count = 2 + (stars_in_sector / 8)`
2. Pick `constellation_count` centroid points via seeded random within sector bounds
3. Assign each star to the nearest centroid (weighted by distance)
4. Assign `constellation_id` from the centroid index

### 2b. Nebula Bands

Add a `fn is_in_nebula(x: i32, y: i32) -> bool` helper. Define 2–4 nebula bands per galaxy as seeded BTreeSet of star IDs. Stars inside a nebula get the `in_nebula_band` flag already used by resource weight functions (which already support it but the flag is always false).

Generate nebula bands deterministically: pick 2 anchor coordinates from the seed, define elliptical bands of width 60 units. Stars whose center falls within a band are flagged.

### 2c. Habitable Distribution

Ensure every sector gets ≥ 1 habitable planet within its first `stars_in_sector / 4` systems. Spread Terran/Oceanic/Desert classes deterministically rather than randomly.

**Files:** `crates/game_core/src/galaxy.rs`, `crates/game_core/src/state.rs`

---

## 3. Expansion AI (`ai.rs`)

### 3a. System Scoring for Scout Targets

Replace the current `nearest-unexplored` scout dispatch with a scored target list. Score each unexplored star on:

- **Planet quality** (sum of habitable planet class scores: Terran 20, Oceanic 15, Desert 12, Frozen 10, Volcanic 6, Barren 4)
- **Resources** (visible strategic resource counts × trade_value/10)
- **Specials/anomalies** (visibility-gated value × doctrine weight)
- **Supply reach** (projected fleet supply state at destination)
- **Distance** (closer = higher score, but don't filter by nearest)
- **Doctrine bias** (expansionist gets +30% colony value)
- **Frontier bonus** (systems farther from home star get a small bonus for expansionist factions)
- **Contested check** (-50 if another empire has a colony or fleet adjacent)

Sort by `(score desc, distance asc, star_id asc)`.

### 3b. Colonization Target Selection

Extend `pick_colonize_target` to consider *every* AI-explored star with an idle colonizer, not just the colonizer's current system. The AI should move the colonizer to the best target within reach.

Score each star with colonizable planets:
- Planet class score (same as above)
- Specials/resources (same weights)
- Existing empire presence (penalty for clustering: -10 per existing colony ≥ 3)

This may require the AI to issue a `MoveToSystem` order before colonizing, consuming the colony ship on arrival. For v1, keep the current "colonizer must be at star" constraint but expand which colonizer gets dispatched: find the best (fleet, destination) pair across all idle colonizers and all explored stars.

**Files:** `crates/game_core/src/ai.rs`

---

## 4. Colony Specialisation (`ai.rs`, `state/colony_roles.rs`)

### 4a. Dynamic Role Re-Assignment

The current `ai_assign_colony_roles` only assigns roles to colonies still on `Balanced`. Change this to re-evaluate roles periodically (every 15 turns, or when the empire unlocks a new strategic building/tech).

New role scoring per colony:
- **Economy** (planet class + industry focus + existing credits deficit)
- **Science** (frozen worlds, science nexus ideal, technologist doctrine)
- **Industry** (barren/volcanic worlds, industrialist doctrine, war focus)
- **Food** (oceanic/terran, food deficit, agrarian doctrine)
- **Military** (militarist doctrine, border colony, currently at war)
- **Financial** (desert worlds, merchant doctrine, trade hub presence)

Score baseline from `ai_role_for_planet_class_with_identity`, then adjust ±3 based on empire-level shortages. E.g., if food < 0 and the colony has good food output, prefer Agricultural. If the empire is at war and this colony is in a border sector, prefer Military.

### 4b. Colony-Specific Build Priority

The current `pick_build_item` uses the same priority chain for every colony. Add per-colony role awareness:
- **Scientific colonies** → ScienceNexus first, then shipyard if none, then research ships
- **Industrial colonies** → FabricationYard first, orbital shipyard, then combat ships
- **Agricultural colonies** → AquacultureBay, then housing structures
- **Military colonies** → Shipyard, then combat ships, then defense platforms
- **Financial colonies** → Supply Hubs, FabricationYard, then trade ships (future)

**Files:** `crates/game_core/src/ai.rs`

---

## 5. Research Priorities (`ai.rs`)

### 5a. Adaptive Research Weighting

The current `research_score` already has `doctrine_victory_preference`. Add situational modifiers:

- **Threat level**: If a hostile war-empire fleet is within 2 sectors of any owned colony, multiply `TechDomain::Military` scores by 1.5x (integer math: `score + score / 2`)
- **Empty queue**: If >30% of colonies have idle queues, boost `TechDomain::Engineering` (construction, orbital, production tags)
- **Colonisation pressure**: If the AI owns no idle colonizers and has unexplored habitable planets within reach, boost `TechId::HABITAT_SEEDING` and Colonial Vanguard
- **Victory path**: If the AI is leading on a victory path, boost research toward its prerequisite techs (Scientific → high-tier Society+Economy; Supremacy → Military; Ascendancy → Logistics+Production; Legacy → scouting+exploration)
- **Food crisis**: already present (`FOOD_CRISIS_SCORE_BONUS`), keep as-is

### 5b. Tech Unlock Awareness

Add a small bonus to techs that unlock new buildable items the empire doesn't have yet. E.g., if no colony has a Shipyard, boost `TechId::ORBITAL_ENGINEERING`.

**Files:** `crates/game_core/src/ai.rs`

---

## 6. Fleet & Military AI (`ai.rs`)

### 6a. Fleet Composition

Replace the simple single-type build chain with a role-aware composition:

- **Patrol Corvettes**: Build 1 per border sector (a sector where a hostile or contacted empire has a colony). Prioritize at border colonies.
- **Escort Frigates**: Build 1 per home-world or core-sector colony for local defense.
- **Missile Frigates**: Build for strike fleets when at war. Ratio: 1 per 2 colonies.
- **Destroyers**: Build as fleet anchor when the empire has sufficient industry. Ratio: 1 per 4 colonies.
- **Troop Transports**: Build when at war with a neighbor that has colonies.

AI builds the *first missing hull* from this list rather than always defaulting to Scout. The `pick_build_item` priority chain checks which ship types are under target count and builds the most-needed one first.

### 6b. Strategic Patrol

After combat fleets are dispatched in wartime, also dispatch idle combat fleets during peacetime to patrol important systems:
- Home star
- High-value colony stars (strategic resource, high population)
- Border systems with hostile neighbors

Patrol fleets are assigned to a `Hold` order at the patrol star via standing orders.

Only dispatch 1 patrol fleet per 3 combat-capable fleets owned, to avoid stripping the border.

### 6c. Civilian Escort

When a colonizer or science ship is traveling through unexplored or hostile-adjacent systems, and an idle combat fleet is nearby (≤ 2 stars), send the combat fleet as an escort to the same destination.

**Files:** `crates/game_core/src/ai.rs`

---

## 7. Victory-Aware AI (`ai.rs`, `victory.rs`)

### 7a. Victory Path Pressure

Add a new function `fn victory_pressure(state, empire) -> (VictoryPath, u8)` that returns the victory path an empire is closest to completing and a pressure score (0–100). An empire with pressure > 60 on any path should trigger strategic adjustment in other empires:

- **Scientific leader** (project points > 50% of threshold) → rivals get +15 `TechDomain::Society` research score (catch up) and +10 military score (hostile response)
- **Ascendancy leader** (hold > 5 turns) → rivals prioritise colonising in the leader's direction (border pressure)
- **Supremacy leader** (only 1 rival remains with alive status) → all other empires +25 military research, +30 ship production
- **Legacy leader** (score > 2x runner-up) → no specific adjustment; Legacy is the default timeout

These adjustments are applied as additive modifiers inside `research_score` and `pick_build_item`.

### 7b. Foreign Victory Progress Intel

Add `fn foreign_victory_progress(state, empire) -> BTreeMap<VictoryPath, u8>` that returns the highest rival progress on each path. The AIs use this to decide which victory path they should pivot toward. When an opponent is far ahead on the AI's preferred path, the AI shifts to a different path.

**Files:** `crates/game_core/src/ai.rs`, `crates/game_core/src/victory.rs`

---

## 8. Difficulty (`state.rs`, `ai.rs`)

### 8a. Make Difficulty Mechanical

Replace the placeholder `DifficultyLevel::Standard` with meaningful variants.

Add an `AiDifficulty` enum with `Easy | Standard | Hard | Brutal`.

Difficulty affects the following AI parameters (no hidden resource cheats except Brutal):

| Parameter | Easy | Standard | Hard | Brutal |
|-----------|------|----------|------|--------|
| Research score bonus | -10% | 0 | +10% | +20% |
| Build priority speed | -1 tier | 0 | +0 | +0 |
| Scout range bonus | -25% | 0 | +25% | +50% |
| Aggression threshold | +15 | 0 | -5 | -10 |
| Expansion confidence | -20% | 0 | +10% | +20% |
| Economy bonus | 0 | 0 | 0 | +15% credits |
| Fleet maintenance discount | 0 | 0 | -10% | -20% |

The economy bonus on Brutal is the only "hidden" cheat — a modest +15% credit income. All others affect decision timing, not raw yields.

Add `DifficultyLevel::from(AiDifficulty)` and thread through `ScenarioSetup`.

**Files:** `crates/game_core/src/state.rs`, `crates/game_core/src/ai.rs`, `crates/game_tui/src/screens/new_game_setup.rs`

---

## 9. Performance Optimisations (`ai.rs`, `galaxy.rs`, `engine.rs`)

### 9a. Scout Target Pre-Computation

`pick_scout_target` iterates all stars and for each checks empire fleet positions (O(n²)). Pre-filter unexplored stars into a cached `BTreeSet<StarId>` per empire, updated incrementally when a star is explored.

### 9b. Battle Report Bounds

Review the `battle_reports` and `battle_reports_v3` `VecDeque` bounds. Ensure they don't grow unbounded over a 700-turn Epic game. Already bounded by `BATTLE_REPORT_MAX_HISTORY = 40`.

### 9c. Diplomatic Empire Iteration

Ensure `process_inter_ai_relations` and `process_ai_diplomacy_with_events` don't iterate every empire pair. Current implementation is already limited to `ai_empires` vector (max 4 AIs). OK for all galaxy sizes.

### 9d. Galaxy Generation Timing

Profile `generate_galaxy` for 700 stars. The lane generation is the likely hotspot. The current `generate_hyperspace_lanes` uses a per-star pairwise distance comparison. Replace with a sector-partitioned k-NN approach: for each star, only consider the nearest `K` stars (K = 3–5) within the same or adjacent sectors.

**Files:** `crates/game_core/src/ai.rs`, `crates/game_core/src/galaxy.rs`

---

## 10. Balance Adjustments (`balance.rs`, `ai.rs`)

### 10a. Research Pacing

Reduce T1 tech costs by ~20% (from current 40/65/55/80 to 30/50/45/65). This makes the first 15 turns feel faster. Keep T2–T6 costs unchanged.

### 10b. Colonization Pacing

Reduce `TROOP_TRANSPORT_INVASION_STRENGTH` from current to `(pop * 2)` to make early invasions harder. Reduce `SURVEY_TURNS` from 2 to 1 for basic-scout surveys (already gated by Survey Drones tech).

### 10c. Economy and Growth Balance

- Increase `POP_GROWTH_PERIOD_TURNS` from 10 to 8 to speed up early colony growth.
- Pre-built colonies start with population 10 (unchanged) but production raised from 10 to 12 so early colonies have a small industrial head start.
- Reduce `FABRICATION_YARD_COST` from 80 to 60 to compress early infrastructure turns.

### 10d. Victory Pacing

- Scientific victory threshold: reduce from 1500 to 1200 project points (faster path).
- Ascendancy: change consecutive turns from 10 to 8 (slightly easier to close out).
- Turn limit: 300 stays, but Legacy scores are balanced so a focused player hits ~1500–2500 points by turn 300.

### 10e. Runaway Reduction

Add `fn runaway_penalty(state, empire) -> f64` that penalises the leading empire's research and production:
- If empire has > 2x the colonies of the next-largest empire: -10% research, -10% production
- If empire has > 3x the colonies: -20% research, -20% production
- Penalty scales over the first 100 turns (no penalty before turn 40, full penalty at turn 100+)
- Prevents snowballing without hard-caps; 4-colony empires barely notice, 20-colony empires feel it.

Return values in existing yield model (industry/research % modifiers).

**Files:** `crates/game_core/src/balance.rs`, `crates/game_core/src/engine.rs`, `crates/game_core/src/yield_model.rs`

---

## 11. AI Metrics (`ai.rs`, `state.rs`)

### 11a. EmpirePower Metric

Add a `empire_power(state, empire) -> u32` helper that computes a deterministic power score:

```
power = colonies × 10
      + ∑ population × 3
      + ∑ fleet_strength × fleet_ships × 2
      + completed_techs × 5
      + total_industry / 5
      + total_science / 5
      + credits / 10
```

This is used by the AI for threat assessment and target selection. Not stored on state; computed on demand.

### 11b. Victory Pressure Metric

Expose `fn victory_path_pressure(state, empire) -> u8` that returns 0–100 for the empire's nearest completion on any path. Reuses existing `victory_status.progress`.

**Files:** `crates/game_core/src/ai.rs`, `crates/game_core/src/state.rs`

---

## 12. Testing

Add or update tests for:

1. **Deterministic galaxy generation** — same seed + same size → byte-identical output
2. **Large galaxy generation** — `GalaxySize::Huge` and `GalaxySize::Epic` generate without panic or excessive time (time-bound test)
3. **System scoring** — `scout_resource_prospect_score` with known inputs produces deterministic output
4. **Expansion scoring** — `score_colonization_target` returns same results for same inputs
5. **Colony specialisation** — `ai_role_for_planet_class_with_identity` returns expected role for each class/identity combo
6. **Fleet role awareness** — AI builds patrol corvettes when border colonies exist
7. **Victory-aware AI** — `victory_pressure` returns correct values for each path
8. **Difficulty modifiers** — `AiDifficulty::Hard` produces different research scores than `Easy`
9. **Runaway penalty** — leading empire has penalties, trailing empires do not
10. **Deterministic tie-breaking** — all AI decision functions produce identical output for same state
11. **AI decision determinism** — same seed + same turn → same AI actions

---

## Implementation Order

### Sprint 1: Foundation (files: state.rs, galaxy.rs, balance.rs)
- GalaxySize variants (Tiny/Huge/Epic)
- k-NN hyperspace lanes
- Constellation clustering
- Nebula bands
- Habitable distribution
- Difficulty as mechanical enum
- Balance constant adjustments + runaway penalty

### Sprint 2: AI Intelligence (files: ai.rs)
- System scoring for scouts
- Multi-star colonization target selection
- Dynamic colony role reassignment
- Role-aware build priority
- Adaptive research weighting

### Sprint 3: Military & Victory (files: ai.rs, victory.rs)
- Fleet composition awareness
- Strategic patrol
- Victory path pressure
- Foreign victory progress intel
- Empire power metric

### Sprint 4: Polish (files: TUI, tests)
- New-game-setup UI for new sizes + difficulty
- Scout target pre-computation (performance)
- Determinism + large-galaxy + role + victory tests
- Final validation

---

## Non-Goals (Explicitly Out of Scope)

- No tactical combat changes
- No diplomacy rewrite
- No new victory paths or victory system changes
- No player-facing UI beyond the new-game-setup screen
- No save format changes (all new fields use `#[serde(default)]`)
- No multiplayer or networking
- No espionage or intel rework
- No minor factions

---
