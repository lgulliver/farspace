# FARSPACE Current State (Early Playable Alpha)

This document summarizes what is in the game today.

## What works now

- Deterministic game start and turn simulation from a fixed seed.
- Core 4X loop is playable:
  - explore nearby systems,
  - survey planets,
  - colonize habitable worlds,
  - queue production,
  - select and complete research,
  - end turns and react to event log output.
- Multi-screen TUI flow is available:
  - Main Menu
  - Galaxy/Sector Overview
  - Sector Map
  - System View
  - Colony View
  - Empire Overview
  - Research
  - Diplomacy
- Auto-resolved fleet combat is active when hostile contacted empires meet.
- Save/load works through menu (`L`) and command palette (`:save`, `:load`) using `farspace.sav`.
- AI empire plays turns through the same deterministic engine pipeline.
- Versioned save migrations are in place.

## Partially implemented

- Diplomacy is first-contact only (`Unknown`/`Contacted`), without stances or deals.
- Turn reporting exists as a rolling event log, not a dedicated turn-report phase screen.
- Colony focus and queue cancellation exist in core systems; advanced queue UX (reorder/cancel controls in TUI) is still limited.
- Research selection/progress works, but no breakthrough choice prompts yet.
- Ship templates exist, but no interactive ship design editor/screen yet.
- No full victory/endgame flow yet.

## Intentionally out of scope (until explicitly planned)

- Tactical hex/grid combat.
- Multiplayer/network play.
- Deep diplomacy systems (treaties, trade routes, alliances).
- Complex strategic AI beyond current baseline expansion/defense behavior.

## Known gaps

- No configurable new-game setup in UI yet (seed/galaxy options are not exposed).
- No keybinding or theme configuration UI.
- No fleet detail management layer (merge/split/stance controls).
- Balance/content depth is still alpha-level and subject to change.

## Run and play the current build

### Build and run

```bash
cargo build --release -p farspace
./target/release/farspace
```

### Basic controls

- `N`: new game
- `L`: load game (menu)
- `hjkl` or arrows: move selection
- `Enter`, `e`, or `t`: end turn (context dependent)
- `?`: contextual help
- `:`: command palette (`save`, `load`)
- `Q` / `Ctrl+C`: quit

### Suggested first session

1. Start a new game (`N`).
2. Open a sector (`Enter`) and inspect systems.
3. Send a scout from Sector Map (`S`) to unexplored systems.
4. Open a system (`Enter`) and survey planets with a science ship (`S`) when available.
5. Colonize surveyed habitable planets from System view (`C`) using a colonizer fleet in-system.
6. Open colony view (`c`/`Enter`) to queue production and set colony role.
7. Open research (`r`) to pick active technology.
8. End turns (`e`/`t`) and monitor the event log.
