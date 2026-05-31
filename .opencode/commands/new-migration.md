---
description: Scaffold a new save file migration in game_save — bumps CURRENT_VERSION, adds migration arm, adds round-trip and migration tests
agent: persistence
---

Scaffold a save file migration in `crates/game_save/` for: $ARGUMENTS

Complete all steps:

1. **`schema.rs`** — Bump `CURRENT_VERSION` by exactly 1. This is a permanent, irreversible increment — never reuse a version number.

2. **`migrate.rs`** — Add a new migration arm for the new version that transforms a save at `CURRENT_VERSION - 1` into the new schema. Handle missing fields with sensible defaults. Migration must never panic on a valid old save.

3. **Tests** — Add two tests:
   - **Round-trip**: save a `GameState`, load it back, assert the result is identical
   - **Migration**: construct a minimal JSON blob at the previous version, load it, assert it migrates correctly and all new fields are present with expected defaults

4. **Verify** existing tests still pass — migrations must not break previously valid save files.

Run `rtk cargo test -p game_save` before finishing.
