---
name: new-migration
description: Scaffold a new save file migration in game_save. Bumps CURRENT_VERSION, adds migration arm to migrate.rs, adds round-trip and migration tests. Use when GameState schema changes. Invoke as /new-migration <description>.
tools: [Read, Edit, Grep, Glob]
---

Caveman mode. Fragments OK. Code exact.

## What I do

Add a complete save migration to `crates/game_save/`:
1. Bump `CURRENT_VERSION` in `schema.rs`
2. Add migration arm in `migrate.rs`
3. Add tests: round-trip + migration from previous version

## Workflow

1. `Read` `schema.rs` — note current `CURRENT_VERSION`.
2. `Read` `migrate.rs` — understand existing migration pattern.
3. `Edit` `schema.rs` — increment `CURRENT_VERSION`.
4. `Edit` `migrate.rs` — add new migration arm for `old_version → new_version`.
5. Add `#[cfg(test)]` tests.

## Hard rules

- `CURRENT_VERSION` increments are permanent — never reuse a version number.
- Every migration must produce valid `GameState` — no panics, no data loss.
- Migrations must be backward-compatible: existing saves from previous versions must load.
- Allowed deps only: `game_core`, `serde`, `serde_json`, `thiserror`.

## Test requirements

- Round-trip test: `save(state)` → `load(bytes)` → assert equal to original state.
- Migration test: construct minimal JSON at `old_version`, run migration, assert output valid.
- Error test: corrupted JSON or wrong version returns `Err`, not panic.

## Output

```
schema.rs:<line> — CURRENT_VERSION bumped to <N>.
migrate.rs:<line-range> — added migration arm v<N-1>→v<N>.
migrate.rs:<line-range> — added round-trip + migration tests.
verified: <OK | mismatch @ path:line>
```
