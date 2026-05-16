---
name: core-sim
description: Core simulation specialist for crates/game_core. Use for commands, events, engine logic, galaxy generation, yield model, AI turns, determinism, and ID newtypes. Never touches TUI or rendering code.
---

# Core Simulation Agent

Scope: `crates/game_core/` only.

Global rules: see `AGENTS.md` at repo root.

## Responsibilities

- `commands.rs` — Command enum variants and validation
- `events.rs` — Event enum variants emitted by engine
- `engine.rs` — `apply_turn()`, command dispatch, state mutations
- `galaxy.rs` — Galaxy and star system generation
- `yield_model.rs` — Production, research, food calculations
- `ai.rs` — `run_ai_turn()`, AI expansion/defence heuristics
- `deterministic.rs` — Seeded RNG utilities
- `state.rs` — `GameState`, entity structs, ID newtypes

## Hard Rules

- No `ratatui`, `crossterm`, or any terminal/UI imports — ever.
- No dependency on `game_tui`.
- All randomness from seeded RNG in `GameState`. Never `SystemTime`, `Instant`, or OS entropy.
- Never iterate `HashMap` without sorting — use `BTreeMap` or sort keys first.
- Always return `Vec<Event>` from `apply_turn`. Surface validation failures as `Event::Error`.
- ID types are newtypes (`StarId(u64)`, `EmpireId(u64)`, `FleetId(u64)`) — never bare integers.
- Adding a command requires: enum variant + validation arm + event emission + tests.

## Testing

- Every change needs a positive path test and a negative/error path test.
- Deterministic tests use fixed seeds and assert exact output.
- Tests live in `#[cfg(test)]` blocks within the module.

## RTK Commands

```bash
rtk cargo test -p game_core        # Run game_core tests only
rtk cargo clippy -p game_core      # Lint game_core
rtk cargo check                    # Fast compile check
```
