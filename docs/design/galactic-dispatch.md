# Galactic Dispatch

Galactic Dispatch is a recurring in-universe news bulletin that surfaces important galaxy-wide events in an atmospheric, flavourful way. It improves strategic readability and game atmosphere without leaking hidden information or mutating game state.

---

## Feature Name

**Galactic Dispatch** — an in-universe signal intercept / intelligence briefing. Not modelled on any existing 4X franchise; all names, headlines, and presentation are original.

---

## Cadence

| Constant | Value | Meaning |
|---|---|---|
| `DISPATCH_CADENCE` | 5 | A dispatch is always generated every 5 turns |
| `DISPATCH_MAX_HISTORY` | 10 | At most 10 recent dispatches are retained in `GameState.galactic_dispatches` |

A dispatch is generated when `completed_turn % DISPATCH_CADENCE == 0`, where `completed_turn` is the value of `GameState::turn` **before** it is incremented. Because normal games start at `turn = 1`, the first in-game cadence dispatch is emitted when `state.turn` advances to a multiple of `DISPATCH_CADENCE + 1` (i.e. turns 5, 10, 15, …). The `completed_turn = 0` cadence used in unit tests reflects an initial state that starts before the first real game turn.

Off-cadence dispatches are only generated when at least one **Urgent** or **Historic** item is present (e.g. a blockade, invasion, or victory achievement).

---

## Data Model

```
GalacticDispatch
  turn: u32                  — completed turn when the dispatch was generated
  title: String              — display title ("Galactic Dispatch — Turn N")
  items: Vec<DispatchItem>   — sorted, stable-ordered items

DispatchItem
  category: DispatchCategory
  severity: DispatchSeverity
  headline: String           — concise, atmospheric, original
  body: String               — one or two sentence expansion
  related_empire_id: Option<EmpireId>
  related_star_id: Option<StarId>
  related_planet_index: Option<usize>
```

---

## Categories

| Variant | Covers |
|---|---|
| `Exploration` | System exploration, orbital surveys |
| `Colonization` | Colony founding (player and AI) |
| `Research` | Technology breakthroughs, research redirections |
| `Economy` | Credit deficits, food shortages |
| `Diplomacy` | First contact, relationship shifts |
| `War` | Fleet combat outcomes |
| `Blockades` | Blockade starts |
| `Invasions` | Planetary invasions (success and failure) |
| `Trade` | Colony isolation / supply network disruption |
| `VictoryRace` | Victory-path progress milestones, winner |
| `MinorFactions` | Reserved for future minor faction contacts |

---

## Severity Levels

| Variant | Marker | Style | When used |
|---|---|---|---|
| `Notice` | `[·]` | Muted | Routine exploration, AI research redirections |
| `Notable` | `[»]` | Title | Colonization, first contact, minor war, food shortage |
| `Urgent` | `[!!]` | Error (red) | Blockades, credit deficit, invasion attempts |
| `Historic` | `[★★]` | Accent | Ancient ruins, high-progress victory milestones (≥80 %), victories, invasion conquests |

---

## Generation Sources

The dispatch is derived from the event stream emitted during `process_end_turn`. The following `Event` variants map to dispatch items:

| Event | Category | Severity |
|---|---|---|
| `SystemExplored` | Exploration | Notice |
| `PlanetSurveyCompleted` | Exploration | Notice |
| `AncientRuinsDiscovered` | Research | Historic |
| `ColonizationCompleted` | Colonization | Notable |
| `AiColonized` | Colonization | Notable (or vague if empire unknown) |
| `ResearchCompleted` (player) | Research | Notable |
| `AiResearchSelected` | Research | Notice (if empire known) |
| `CombatResolved` | War | Urgent/Notable depending on player involvement |
| `BlockadeStarted` | Blockades | Urgent |
| `InvasionSucceeded` | Invasions | Historic |
| `InvasionFailed` | Invasions | Notable |
| `FoodShortage` (player) | Economy | Urgent |
| `EconomySummary` with deficit (player) | Economy | Notable |
| `ColonyIsolated` (player colony) | Trade | Notable |
| `FirstContact` | Diplomacy | Notable |
| `VictoryProgressMilestone` | VictoryRace | Notable (or Historic if ≥80 %) |
| `VictoryAchieved` | VictoryRace | Historic |

Dispatches do not generate random events — they only reflect what actually happened this turn.

---

## Public Information Rules

Dispatch content is filtered to avoid leaking hidden state:

1. **Unknown empires**: if the player has not made contact with an empire (`RelationshipStatus::Unknown` or absent from `diplomacy`), that empire's name and ID are withheld. Vague wording is used instead:
   - "Unknown Forces Establish Remote Colony"
   - "Unidentified Fleet Engages Forces Near [star]"
   - "Unidentified Fleet Blockades Contested Colony"

2. **Unsurveyed planets**: planet specials and strategic resources are never referenced in dispatch text until the planet has been surveyed.

3. **Unseen stars**: system exploration events are only surfaced when the player's own fleet explored them (event is already player-scoped). AI-explored stars are not reported as exploration events.

4. **AI combat**: if the player is not involved in combat and neither fleet's empire is known, the combat event is skipped entirely.

---

## Item Ordering (Determinism)

Items are sorted by a composite key:

1. Severity descending (Historic → Urgent → Notable → Notice)
2. Category ascending (enum declaration order, mapped to u8)
3. Headline ascending (lexicographic)

This produces a stable, deterministic order regardless of event insertion order or BTreeMap iteration. Same events + same state always produce identical dispatch output.

---

## Deduplication

Multiple events of the same type in one turn are collapsed:
- Multiple `SystemExplored` events → one Exploration item ("Survey Crews Chart New Frontier Worlds")
- Multiple `PlanetSurveyCompleted` events → one Exploration item

This is achieved via a `BTreeSet<(DispatchCategory, String)>` deduplication set.

---

## Examples

```
╔══════════════════════════════════════════╗
║       ◈ GALACTIC DISPATCH ◈              ║
║  Turn 5  ·  Dispatch 1/1                 ║
║──────────────────────────────────────────║
║ [★★] [RESEARCH]                          ║
║  Ancient Ruins Discovered —              ║
║  Archaeological Teams Mobilize           ║
║   Excavations begun at Keth Prime.       ║
║                                          ║
║ [»]  [COLONIZATION]                      ║
║  Colonists Establish Foothold in         ║
║  Vela System                             ║
║   A colonial charter has been issued     ║
║   for Vela II.                           ║
║──────────────────────────────────────────║
║  ← prev / → next   Esc to close         ║
╚══════════════════════════════════════════╝
```

---

## TUI Interaction

| Key / Command | Action |
|---|---|
| `N` (global, in-game) | Open latest Galactic Dispatch |
| `:dispatch` | Open latest Galactic Dispatch |
| `:news` | Alias for `:dispatch` |
| `←` / `h` | Cycle to previous dispatch in history |
| `→` / `l` | Cycle to next dispatch in history |
| `Esc` / `N` / `n` | Close dispatch overlay |

The dispatch overlay is shown automatically when a new dispatch is generated at the end of a turn.

---

## Persistence (Save/Load)

`galactic_dispatches` is stored in `GameState` as a `VecDeque<GalacticDispatch>`. It is serialised via serde (behind the `serde` feature flag on `game_core`). Old saves (schema ≤ 28) load with an empty history via `#[serde(default)]`.

Save schema version: **29** (introduced with Galactic Dispatch v1).

---

## Determinism and Replay

- `generate_dispatch` takes only `completed_turn`, the event slice, and a shared reference to `GameState`. It uses no RNG and no wall-clock logic.
- Same seed + same command log → same events → same dispatches.
- `galactic_dispatches` is included in `GameState`'s manual `PartialEq` implementation, so equality checks (e.g. in save-load round-trip tests) compare dispatch history along with all other game state.

---

## Future Hooks

- **Minor faction sightings**: `MinorFactions` category is reserved for when minor independent factions are introduced.
- **Reduced-interruptions mode**: a future `reduced_interruptions` flag in UI settings can suppress automatic dispatch opening.
- **Dispatch filtering by category**: the TUI modal could gain a category filter.
- **Clickable context links**: headlines with `related_star_id` could eventually jump to that system.
- **Urgent off-cadence dispatch**: the off-cadence Urgent/Historic rule is already implemented; future "galactic crisis" events can piggyback on it.
