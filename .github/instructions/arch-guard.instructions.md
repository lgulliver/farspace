---
applyTo: "**"
---

# Architecture Guard — Copilot Instructions

Scope: All crates (cross-cutting boundary enforcement).

Global rules: see `.github/copilot-instructions.md`.

## Boundary Rules (Always Enforce)

- `game_core`: never import `ratatui`, `crossterm`, or `game_tui`
- `game_content`: only uses `game_core` types
- `game_save`: only `game_core`, `serde`, `serde_json`, `thiserror`
- `game_tui`: sends `Command` to core; renders from `Event` + snapshots; never mutates `GameState` directly
- New crate dependencies must go in the correct crate's `Cargo.toml` only

## Determinism Rules (Always Enforce)

- No `SystemTime`, `Instant`, or OS entropy for RNG seeding
- No `HashMap` iteration without sorting — use `BTreeMap` or sort keys first
- New randomness routes through `GameState`'s seeded RNG

## Command/Event Flow (Always Enforce)

- New commands: enum variant in `commands.rs`, validation arm in `engine.rs`, event emission, tests
- Validation failures: `Event::Error`, not panics or bare `unwrap()`

## Scope (Flag Without Explicit Request)

- Tactical combat, multiplayer, deep diplomacy, complex AI, Master of Orion content
