---
applyTo: "crates/game_core/**"
---

# Core Simulation — Copilot Instructions

Scope: `crates/game_core/` only.

Global rules: see `.github/copilot-instructions.md`.

## Role

Headless game simulation. No terminal/UI dependencies.

## Hard Rules

- No `ratatui`, `crossterm`, or `game_tui` imports — ever.
- All randomness from seeded RNG in `GameState`. Never `SystemTime`, `Instant`, or OS entropy.
- Never iterate `HashMap` without sorting. Use `BTreeMap` or sort keys before iteration.
- Return `Vec<Event>` from `apply_turn`. Surface validation failures as `Event::Error`.
- ID types are newtypes (`StarId(u64)`, `EmpireId(u64)`, `FleetId(u64)`) — never bare integers.
- New command requires: enum variant in `commands.rs` + validation arm in `engine.rs` + event emission + tests.

## Testing

Fixed-seed determinism tests. Positive + negative path tests for every change.

```bash
rtk cargo test -p game_core
rtk cargo clippy -p game_core
```
