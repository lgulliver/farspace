# Combat v3 — Card-Driven Battle Resolution

Combat v3 replaces the Combat v2 auto-resolve phase model with a deterministic,
turn-based card-driven resolution. Each side drafts a 5-card hand from its fleet
composition, ship components, and unlocked techs, then plays one card per round
for up to 5 rounds. The goal is to surface strategic depth in fleet design and
component loadout without introducing tactical maps or realtime controls.

---

## Scope and Intent

- **Replaces** Combat v2 phase auto-resolve. The v2 `BattleReport` type is
  retained as a historical record; new battles emit `BattleReportV3`.
- **No tactical map**, no realtime loop, no manual ship movement in battle,
  no subsystem targeting, no projectile simulation.
- Card play is fully turn-based and command-driven (`Command::PlayBattleCard`).
- AI plays cards via a deterministic policy in `game_core::ai`; the TUI never
  makes AI decisions.

## Determinism Rules

- All randomness in battle resolution (damage rolls, crits, retreat checks)
  comes from `GameState.rng` (seeded `ChaCha8Rng`).
- Card draft is a **pure function** of `(fleet, empire_state)`. No RNG during
  draft. Same fleet + same empire state → byte-identical hand.
- Card iteration uses `BTreeMap`/`BTreeSet` and `sort_by_id` for stable ties.
- Battle reports persist in save data; save schema version bumps to 37.

---

## High-Level Flow

```
Battle start
  → engine resolves engagement (who fights, where)
  → IF player involved: PAUSE turn, open BattleScreen overlay
  → Each side drafts a 5-card hand from fleet+tech+loadout pool
  → Alternating rounds: each side plays 1 card per round (5 rounds max)
  → Rounds resolve deterministically via seeded RNG
  → BattleScreen emits Commands (PlayBattleCard) until hand empty
  → Engine finalises BattleReportV3 (round log + v2 setup summary kept)
  → Resume main turn
```

When neither fleet belongs to the player, the engine auto-resolves the
battle without pausing. When the player is involved, the engine returns
`TurnStep::AwaitingBattleInput(BattleSession)` and waits for player commands.

---

## Card Pool (v1)

The v1 card pool contains **23 unique cards** plus a single shared fallback
`Hold Fire` (no-op) filler.

### Base Cards (15)

Sourced from hulls, components, and techs. Cards are statically registered
in a `CARD_REGISTRY` table; each `CardDef` has a stable `CardId(u16)`.

| # | Name | Source | Effect | Doctrine bias |
|---|---|---|---|---|
| 1 | Kinetic Salvo | Hull: Escort Frigate, Missile Frigate, Destroyer, Patrol Corvette | Strike (×atk) | Militarist +2 |
| 2 | Ablative Hull | Component: Reinforced Plating | Guard (×def) | Isolationist +2 |
| 3 | Phased Shield | Component: Shield Matrix | Guard + absorb 1 dmg next round | Isolationist +2 |
| 4 | CIWS Grid | Component: Point Defense | Disrupt (cancel 1 enemy card) | — |
| 5 | Burn Maneuver | Component: Ion Drive | Evasive (reduce incoming 50%) | Explorer +2 |
| 6 | Drift Burn | Hull: Scout, Fast Scout | Maneuver (initiative +1) | Explorer +2 |
| 7 | Targeting Lock | Component: Targeting Suite | Mark (next Strike +25%) | Militarist +1 |
| 8 | Sensor Sweep | Component: Long-Range Sensors | Probe (reveal enemy hand) | Explorer +1 |
| 9 | Orbital Bombardment | Hull: Destroyer | Salvo (×atk applies to all rounds) | Militarist +3 |
| 10 | Defensive Screen | Hull: Escort Frigate, Patrol Corvette | Fortify (def +50% this round) | Isolationist +1 |
| 11 | Troop Drop | Component: Troop Bays / Hull: Troop Transport | Bolster (post-battle invasion strength +) | Militarist +2 |
| 12 | Warp Retreat | Tech: Rapid Transit Drives | Withdraw (auto-retreat, preserve 50% integrity) | Explorer +2, Merchant +1 |
| 13 | Ordnance Overcharge | Tech: Long-Range Strike Doctrine | Overcharge (Strike +50% but self 1 dmg) | Militarist +2 |
| 14 | Formation Rally | Tech: Battle Doctrine | Inspire (hand refill 1 card mid-battle) | Unity +3, Militarist +1 |
| 15 | Surveyor's Gambit | Hull: Science Vessel, Survey Cutter | Probe + Evasive hybrid | Explorer +3, Merchant +1 |

### Faction Signature Cards (8)

Each faction has one signature card. Used as a fallback pad when a faction's
generic pool produces fewer than 5 cards.

| Faction | Signature card | Effect | Bias |
|---|---|---|---|
| Vorath Dominion | Coercive Mandate | Strike + bleed (next round enemy cards cost 1 hp) | Militarist |
| Terran Dominion | Siege Doctrine | Strike + Bolster (pressure + post-battle invasion +) | Militarist+Imperial |
| Ashveran Compact | Industrial Juggernaut | Guard + sustain (heal 2 hp at end of round) | Industrialist |
| Elarith Confluence | Algorithmic Defense | Guard + Probe (def + reveal enemy hand) | Technologist |
| Luminal Traverse | Pathfinder's Wager | Probe + Evasive (reveal + reduce incoming 50%) | Explorer |
| Terran Concord | Council of Voices | Inspire (refill 1 hand slot) | Unity/Explorer |
| Thalori Exchange | Trade Barge Stand | Guard (cheap, +25% def) | Merchant |
| Sylvaran Accord | Bloom Shield | Guard + regen (heal 1 hp at round start next 2 rounds) | Biologist |

### Fallback

- `Hold Fire` (single shared card): no effect, burns the round. Last-resort
  pad when the generic pool + signature card still produce fewer than 5 cards.

---

## Hand Draft (`build_hand`)

```rust
fn build_hand(fleet, empire) -> [CardId; 5]:
  pool = []
  pool += hull_card(fleet.hull)
  for component in sorted_by_component_id(fleet.components):
    pool += component_card(component)            // if defined
  for tech in sorted_by_tech_id(empire.unlocked):
    pool += tech_card(tech)                       // if defined
  pool = filter_eligible(pool, doctrine)          // doctrine-locked cards filtered
  pool = weight_and_sort(pool, doctrine)          // stable sort, no RNG
  hand = take(pool, 5)                            // first 5
  if hand.len() < 5:
    hand += faction_signature_card(empire)        // dedup-tracked, may be repeated
  if hand.len() < 5:
    hand += Hold Fire                              // last-resort pad
  hand.truncate(5)
```

Rules:

- `BTreeMap`/`BTreeSet` for any iteration. Sort by `CardId` ascending for ties.
- No RNG pulled during draft.
- Same fleet + same empire state + same hand-slot list → byte-identical hand.
- Replay stability preserved: reloading a save mid-battle reproduces the same
  hand and round log.

---

## Card Effects (Verbs)

Each card resolves into one or more of these verbs. All verbs are deterministic
functions over `(session, side, card, target, rng_state)`.

| Verb | Description |
|---|---|
| **Strike** | Deal `atk × multiplier` damage to enemy fleet integrity. |
| **Guard** | Reduce incoming damage by `def × multiplier` this round. |
| **Maneuver** | +1 initiative (player's card resolves first next round). |
| **Snipe** | Strike against a specific ship slot (future expansion). |
| **Bolster** | Post-battle invasion strength bonus. |
| **Disrupt** | Cancel one enemy card queued for this round. |
| **Rally** | No-op in v1 (reserved for future team-play hooks). |
| **Evasive** | Reduce incoming damage by 50% this round. |
| **Salvo** | Strike applies to all subsequent rounds in this battle. |
| **Fortify** | Guard applies for this round with +50% def multiplier. |
| **Probe** | Reveal enemy hand (one card per Probe played). |
| **Mark** | Next Strike card deals +25% damage. |
| **Overcharge** | Strike +50% but self-inflict 1 damage. |
| **Withdraw** | Auto-retreat, preserve 50% of current integrity. |
| **Inspire** | Refill 1 hand slot mid-battle (deck top, deterministic). |

---

## Battle Session and Resolution

### `BattleSession` (lives in `GameState.pending_battle_session`)

```rust
pub struct BattleSession {
  pub session_id: u64,
  pub star: StarId,
  pub attacker: FleetId,
  pub defender: FleetId,
  pub empire_a: EmpireId,
  pub empire_b: EmpireId,
  pub hand_a: Vec<CardId>,
  pub hand_b: Vec<CardId>,
  pub integrity_a: u32,
  pub integrity_b: u32,
  pub round: u8,             // 0..=4
  pub setup_summary: BattleSetupSummary,  // v2 fields kept for report
  pub state: SessionState,   // AwaitingInput | Resolving | Finished
}
```

### Round Resolution

For each round (0..=4):

1. Both sides play one card from their hand (player via `Command::PlayBattleCard`,
   AI via `ai_pick_card`).
2. `apply_round` resolves the side with higher initiative first. Initiative
   defaults to 0; Maneuver grants +1.
3. Cards resolve in initiative order. Disrupt cancels the first enemy card
   queued for the round. Mark and Overcharge apply to their follow-up Strike.
4. Integrity updates are applied; if either side reaches 0, session finalises.
5. After all 5 rounds (or earlier if one side is destroyed/retreats), the
   engine emits `Event::BattleFinished` with a `BattleReportV3`.

### Win Conditions

- **Annihilation**: enemy fleet integrity reaches 0.
- **Retreat**: `Withdraw` card played (auto-success) or `RetreatFromBattle`
  command (free, burns current turn; fleet retains 25% integrity).
- **Tiebreaker**: after 5 rounds, side with higher integrity wins. Ties go to
  the defender (or attacker if defender was player).

### Doctrine Interactions

- Doctrine influences card selection (see AI section) and a small subset of
  card biases (e.g. Militarist gets +2 weight on Strike cards). It does not
  change effect magnitudes.
- Formation (Balanced/Aggressive/Defensive/Fast Attack/Artillery/Escort Screen)
  applies a global modifier to integrity and damage calculations, identical
  to v2 behavior. Formations are picked fleet-side before battle start.

---

## AI Card Play (`ai_pick_card`)

```rust
fn ai_pick_card(session, side, doctrine_weights, round, integrity_diff) -> CardId:
  cards = session.hand(side)
  best = max by (sum(card.doctrine_weight[d] * doctrine_weights[d] for d in doctrines))
  ties broken by CardId ascending
  return best
```

The AI does not look ahead multiple rounds; it picks the locally optimal card
based on the current round, hand state, and integrity delta. Doctrine weights
are sourced from the empire's `doctrine_weights` field (already in
`EmpireDefinition`).

### Faction-to-Play-Style Mapping

Eight factions map onto five play-style buckets. Each bucket has a per-round
card priority used to seed the AI when the generic scoring is ambiguous.

| Bucket | Faction | id | Top doctrines | Card priority (R1..R5) |
|---|---|---|---|---|
| Militarist | Vorath Dominion | 4 | Militarist (10), Imperial (8) | Strike → Strike → Salvo/Overcharge → Mark+Strike → Bolster |
| Militarist | Terran Dominion | 7 | Imperial (10), Militarist (9), Industrialist (8) | Strike → Bolster → Salvo → Strike → Bolster |
| Isolationist | Ashveran Compact | 0 | Industrialist (9), Isolationist (7) | Guard → Fortify → Guard → Disrupt → Withdraw if losing |
| Isolationist | Elarith Confluence | 5 | Technologist (10), Industrialist (8), Isolationist (8) | Guard → Probe → Guard → Disrupt → Withdraw if losing |
| Explorer | Luminal Traverse | 1 | Explorer (9), Merchant (7), Expansionist (6) | Probe → Maneuver → Evasive → Strike (snipe) → Withdraw if losing |
| Unity | Terran Concord | 6 | Explorer (10), Technologist (9), Merchant (7) | Inspire → Guard → Strike → Inspire → Bolster |
| Merchant | Thalori Exchange | 3 | Merchant (10), Expansionist (8) | Guard → Probe → Guard → Withdraw early → Bolster |
| Merchant | Sylvaran Accord | 2 | Expansionist (9), Biologist (9) | Guard → Bloom Shield → Guard → Withdraw early → Bolster |

AI behaviour resolution order:

1. Identify primary bucket from `faction.id` (table above).
2. Look up bucket's per-round card priority (5-round matrix).
3. Apply faction signature card as round-1 or round-5 anchor.
4. Apply doctrine-weight modifier: cards whose doctrine tag matches the
   faction's top-2 doctrines get +1 weight per match.
5. Pick max-weighted eligible card from current hand.
6. Ties broken by `CardId` ascending.

Deterministic, no RNG in selection. Replay-stable.

---

## Player Flow

The engine returns `TurnStep::AwaitingBattleInput(BattleSession)` when a
player-involved battle starts. The TUI opens the `BattleScreen` overlay.
The player plays cards via:

| Key | Action |
|---|---|
| `1`–`5` | Play card N from hand |
| `Tab` | Toggle side view (your hand / enemy hand) |
| `Enter` | Confirm card target (where required) |
| `r` | Free retreat command (burns current turn; fleet retains 25% integrity) |
| `?` | Help overlay |
| `Esc` | Close (only allowed if hand is empty or integrity is 0) |

Each `PlayBattleCard` command emits `Event::BattleRoundPlayed`. The hand
shrinks by 1. When both hands are empty (or one side is destroyed/retreating),
the engine finalises the battle and emits `Event::BattleFinished`.

---

## Commands

| Command | Description |
|---|---|
| `Command::PlayBattleCard { session_id, card_index, target }` | Player plays a card from their hand. |
| `Command::RetreatFromBattle { session_id }` | Free retreat. Burns current turn. |
| `Command::ResolveAiBattle { session_id }` | Engine-internal. Used during AI-only battles. |

`Command::PlayBattleCard` is rejected with `Event::Error` when:
- The session is not awaiting input.
- The card index is out of range.
- The card is not in the player's hand (dedup mismatch).
- The target is invalid for the card's verb.

---

## Events

| Event | Description |
|---|---|
| `Event::BattleStarted { session_id, attacker, defender, setup_summary }` | New battle session. |
| `Event::BattleRoundPlayed { session_id, round, side, card, effect_summary }` | One card played and resolved. |
| `Event::BattleFinished { session_id, report_id, outcome }` | Battle complete. |
| `Event::CombatResolved { ... }` | **Deprecated.** Retained for legacy save compatibility. |

`Event::CombatResolved` is no longer emitted by the engine. It is preserved in
the event enum for legacy save migrations.

---

## Battle Report v3

```rust
pub struct BattleReportV3 {
  pub report_id: u64,
  pub turn: u32,
  pub star: StarId,
  pub fleet_a: FleetId,
  pub fleet_b: FleetId,
  pub empire_a: EmpireId,
  pub empire_b: EmpireId,
  pub setup_summary: BattleSetupSummary,  // v2 fields
  pub hand_a: Vec<CardId>,
  pub hand_b: Vec<CardId>,
  pub rounds: Vec<BattleRoundSummary>,    // per-round log
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

pub struct BattleRoundSummary {
  pub round: u8,
  pub card_a: Option<CardId>,
  pub card_b: Option<CardId>,
  pub effect_a: String,
  pub effect_b: String,
  pub integrity_a_after: u32,
  pub integrity_b_after: u32,
}
```

Reports are stored in `GameState.battle_reports_v3: VecDeque<BattleReportV3>`.
The legacy `battle_reports: VecDeque<BattleReport>` field is kept for history
and is never written to post-migration.

---

## Module Layout

```
crates/game_core/src/combat_v3/
  mod.rs            # BattleSession, TurnStep, public API
  card.rs           # CardDef, CardId, CardEffect, CARD_REGISTRY (23)
  deck.rs           # build_hand(fleet, empire, doctrine) -> [CardId; 5]
  resolve.rs        # apply_round(session, side, card, target, rng) -> EffectOutput
  ai.rs             # ai_pick_card(session, side, doctrine_weights, round, integrity_diff)
  report.rs         # BattleReportV3, BattleRoundSummary
  withdraw.rs       # Warp Retreat card + free retreat command logic
  tests.rs          # ~34 tests
```

`crates/game_tui/src/screens/battle.rs` — overlay-style modal. Reuses the
existing `Theme` palette, spacing, and overlay patterns from
`docs/design/ux-splash-screen.md`.

---

## Save / Load

- Schema version bumps from 36 to 37.
- New `GameState` fields (all `serde(default)`):
  - `next_battle_session_id: u64 = 1`
  - `pending_battle_session: Option<BattleSession>`
  - `battle_reports_v3: VecDeque<BattleReportV3>`
- Migration v36 → v37: passthrough with defaults populated. Existing
  `battle_reports: VecDeque<BattleReport>` entries remain.
- Replay stability preserved: deterministic card draft, deterministic round
  resolution, stable report insertion order.

---

## Determinism Audit Checklist

- [x] No `SystemTime` / `Instant` in any combat code path.
- [x] No `HashMap` iteration; all collections are `BTreeMap`/`BTreeSet` or
      sorted vectors.
- [x] RNG pulled from `GameState.rng` only inside `apply_round`.
- [x] Card draft is a pure function of `(fleet, empire_state)`.
- [x] AI card selection is deterministic (no stochastic sampling).
- [x] Tie-breaks use stable `CardId`/`ComponentId`/`TechId` ordering.
- [x] Replay tests with fixed seed produce byte-identical events.

---

## TUI Integration

`crates/game_tui/src/screens/battle.rs` (new) — overlay modal:

```
┌ Battle: Escort Frigate vs Missile Frigate ──── Round 3/5 ─┐
│ Your Hand (Ashveran Compact)       Enemy Hand (unknown)   │
│  1 Ablative Hull        Guard      1  ?                   │
│  2 Phased Shield        Guard+abs  2  ?                   │
│  3 Industrial Juggernaut Guard+heal 3  ?                   │
│  4 CIWS Grid            Disrupt    4  ?                   │
│  5 Hold Fire            (no-op)    5  ?                   │
│                                                            │
│ Integrity: YOU ████████░░ 80%   ENEMY █████░░░░░ 50%      │
│ Last round:  Enemy played Strike -8  You played Guard -5 │
│                                                            │
│ [1-5]Play  [Tab]Side  [Enter]Confirm  [r]Retreat  [?]Help │
└────────────────────────────────────────────────────────────┘
```

Keyboard-first. Reuses `Theme` palette + spacing. Matches overlay pattern
from `docs/design/ux-splash-screen.md`.

---

## Out of Scope (Deliberate)

- Tactical maps, hex/grid combat, realtime controls.
- Multiplayer, networked play.
- New diplomacy mechanics beyond existing war/contacted/hostile states.
- AI lookahead, MCTS, search-based planning.
- Subsystem targeting, projectile simulation.
- Manual ship movement in battle.
- Ammo, fuel, heat, morale, admiral, XP layers.

---

## Future Expansion Path

- Additional cards unlocked by T4–T6 techs.
- Multi-fleet battles (N-vs-N with one hand per fleet).
- Snipe targeting (Strike against a specific ship slot).
- Rally effects (cross-fleet buffs in multi-fleet battles).
- Card upgrades via tech or doctrine.
- Optional developer-facing AI reasoning inspection tooling.
