---
description: Full PR review — architecture boundaries, determinism, command/event flow, test coverage, scope creep, and original IP
agent: arch-guard
---

Review the current diff against all FARSPACE quality standards.

Run `!`git diff main...HEAD --stat`` to see what changed, then `!`git diff main...HEAD`` for the full diff.

Report `[PASS]` / `[FAIL]` / `[WARN]` per section.

---

## Architecture boundaries

- `game_core` has no `ratatui`, `crossterm`, or `game_tui` imports
- `game_content` depends only on `game_core`
- `game_save` depends only on `game_core`, `serde`, `serde_json`, `thiserror`
- `game_tui` sends only `Command` values — no direct `GameState` mutation
- TUI renders from `Event` values and snapshot views only
- No new cross-boundary entries in any `Cargo.toml`

## Determinism

- No `SystemTime`, `Instant`, or OS entropy used for RNG seeding
- No `HashMap` iteration without sorting in simulation paths
- All new randomness sourced from `GameState`'s seeded RNG
- New determinism-sensitive tests use fixed seeds and assert exact output

## Command / Event flow

- New commands: enum variant in `commands.rs`, validation arm in `apply_turn`, event emission
- Validation failures: `Event::Error` — not panic, not `unwrap()`
- New events: enum variant in `events.rs`

## Testing

- Every new feature has a positive path test and a negative/error path test
- Bug fixes have a regression test
- Coverage has not visibly decreased (new branches have tests)

## Scope and IP

- No unrequested tactical combat, multiplayer, deep diplomacy, or complex AI
- No names, stats, or text from Master of Orion or other published 4X titles

---

List every violation with file:line. If all pass, say so concisely.
