# FARSPACE Architecture

This document is source of truth for crate boundaries and system flow.

## Crate Responsibilities

| Crate | Responsibility | Allowed Dependencies |
|---|---|---|
| `game_core` | Headless deterministic simulation (`GameState`, commands, events, turn processing, AI, galaxy generation) | `std` + core data crates only (no TUI crates) |
| `game_tui` | Terminal UI and interaction layer (screens, key handling, command dispatch, event log rendering) | `game_core`, `ratatui`, `crossterm` |
| `game_content` | Static content helpers/templates and data shaping | `game_core` types only |
| `game_save` | Save/load serialization and schema migrations for `GameState` | `game_core`, `serde`, `serde_json` |
| `farspace` | Application entrypoint, terminal setup/restore, app lifecycle | workspace crates |

## Command / Event Flow

```text
UI input
→ Command
→ game_core validation
→ state mutation
→ Events
→ TUI rendering / turn report
```

Rules:

- UI never mutates core state directly
- Core returns domain events for both success and error paths
- Event log and screen hints are derived from emitted events

## Deterministic Simulation Principles

- RNG stored in `GameState` (`ChaCha8Rng`) and advanced only through simulation
- No wall-clock or OS entropy inside simulation logic
- Ordered collections (`BTreeMap`/`BTreeSet`) for stable iteration
- Deterministic travel, supply, blockade, diplomacy drift, and AI selection logic
- Save/load must preserve deterministic replay behavior

## Save / Load Architecture

- `game_save` owns `SaveFile` schema envelope and `CURRENT_VERSION`
- Migrator upgrades old saves to current schema before exposing state
- Save metadata is readable without full load (`schema_version`, turn, seed, game version)
- Expected tests:
  - round-trip equality/invariants
  - corrupted/missing-field failure behavior
  - migration coverage across supported versions

## TUI Architecture

- `App` in `game_tui` maps keyboard input to core commands
- Overlay subsystem handles global help and command palette
- Screen modules render focused views:
  - Menu / New Game Setup
  - Sector Overview / Sector Map / System
  - Colony / Empire Overview / Research / Diplomacy
- TUI tests prefer state transitions and render smoke assertions over brittle full snapshots

## AI Architecture

- AI runs in `game_core::ai` and executes deterministic decision sequence per turn
- Current behavior includes:
  - research selection with weighted deterministic ordering
  - colony build queue decisions
  - scout dispatch and colonization actions
  - colony role assignment
- AI uses same command/event pipeline semantics as player-facing simulation

## Content / Data Architecture

- Core domain records in `game_core::state` (techs, ship designs, empire definitions)
- `game_content` provides static template-style content helpers
- Original IP only: names, lore, and labels must remain original

## Testing Strategy

Primary validation stack:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
```

Coverage floor is 80% workspace line coverage.

## Boundary Rules (Must Not Be Violated)

- `game_core` must stay headless (no `ratatui`/`crossterm`)
- `game_core` must not depend on `game_tui`
- `game_content` and `game_save` must not depend on `game_tui`
- UI sends commands only; no direct simulation mutation
- Determinism must be preserved for all simulation-affecting changes
