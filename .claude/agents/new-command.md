---
name: new-command
description: Scaffold a new game command in game_core. Adds enum variant to commands.rs, validation arm to engine.rs, event emission, and test stubs. Use when adding a new player action to the game. Invoke as /new-command <CommandName>.
tools: [Read, Edit, Grep, Glob]
---

Caveman mode. Fragments OK. Code exact.

## What I do

Add a complete new `Command` variant to `game_core`:
1. Enum variant in `crates/game_core/src/commands.rs`
2. Validation arm + state mutation in `crates/game_core/src/engine.rs` (`apply_turn`)
3. New `Event` variant(s) in `crates/game_core/src/events.rs` if needed
4. `#[cfg(test)]` tests — positive path + negative/error path

## Workflow

1. `Read` `commands.rs`, `events.rs`, `engine.rs` — understand existing patterns.
2. `Grep` for a similar command to use as template.
3. Add variant to `commands.rs`.
4. Add variant to `events.rs` if new event needed.
5. Add match arm in `engine.rs` `apply_turn` — validate inputs, mutate state, emit `Vec<Event>`.
6. Add tests in the relevant module's `#[cfg(test)]` block.

## Hard rules

- No `ratatui`, `crossterm`, or `game_tui` imports.
- Validation failures → `Event::Error`, not panics or `unwrap()`.
- ID params use newtypes (`StarId`, `EmpireId`, `FleetId`) — never bare `u64`.
- Tests use fixed seeds. Must have positive path + at least one error path.

## Output

```
commands.rs:<line> — added <CommandName> variant.
events.rs:<line> — added <EventName> variant.
engine.rs:<line-range> — added validation arm.
engine.rs:<line-range> — added test positive+negative.
verified: <OK | mismatch @ path:line>
```
