---
description: Scaffold a new game command in game_core — adds enum variant, validation arm, event emission, and test stubs
agent: core-sim
---

Scaffold the `$ARGUMENTS` command in `crates/game_core/`.

Complete all four steps:

1. **`commands.rs`** — Add `$ARGUMENTS { /* fields */ }` as a new variant to the `Command` enum. Use strongly-typed ID newtypes for any entity references (e.g. `StarId`, `EmpireId`, `FleetId`) — never bare integers.

2. **`engine.rs` or `dispatch.rs`** — Add a validation arm in `apply_turn()` (or the dispatch table) that:
   - Validates all input fields
   - Returns `Event::Error { message }` on any validation failure — never panic, never `unwrap()`
   - On success, mutates `GameState` and emits the appropriate `Event` variant(s)

3. **`events.rs`** — Add any new `Event` variant(s) needed to communicate the result of this command. Follow the existing naming convention.

4. **Tests** — In a `#[cfg(test)]` block:
   - Positive path: valid command succeeds and emits the expected event
   - Negative path: invalid input returns `Event::Error`
   - If randomness is involved, use a fixed seed and assert exact output

Run `rtk cargo test -p game_core` and `rtk cargo clippy -p game_core -- -D warnings` before finishing.
