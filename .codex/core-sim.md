# Core Simulation Agent — Codex Context

Scope: `crates/game_core/` only.

Global rules: see `AGENTS.md` at repo root.

## Role

Specialist for headless game simulation. Handles commands, events, engine, galaxy generation, yield model, AI turns, and determinism.

## Key Files

- `crates/game_core/src/commands.rs` — Command enum
- `crates/game_core/src/events.rs` — Event enum
- `crates/game_core/src/engine.rs` — `apply_turn()`, validation, state mutation
- `crates/game_core/src/galaxy.rs` — Galaxy and star generation
- `crates/game_core/src/yield_model.rs` — Production/research/food calculations
- `crates/game_core/src/ai.rs` — `run_ai_turn()`
- `crates/game_core/src/deterministic.rs` — Seeded RNG utilities
- `crates/game_core/src/state.rs` — `GameState`, entities, ID newtypes

## Hard Rules

- No `ratatui`, `crossterm`, or `game_tui` dependencies.
- All randomness from seeded RNG in `GameState`. Never `SystemTime` or `Instant`.
- Never iterate `HashMap` without sorting. Use `BTreeMap` or sort keys first.
- Return `Vec<Event>` from `apply_turn`. Validation failures as `Event::Error`.
- ID types are newtypes (`StarId(u64)`, etc.) — never bare integers.
- New command = enum variant + validation arm + event emission + tests.

## Testing

Fixed-seed determinism tests. Positive + negative path per change.

## Commands

```bash
rtk cargo test -p game_core
rtk cargo clippy -p game_core
```
