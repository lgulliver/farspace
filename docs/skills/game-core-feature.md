# Skill: Game Core Feature

A playbook for adding a new feature to `game_core`.

---

## Principle

`game_core` is a pure, headless library. No terminal, no I/O, no UI crates.
All state changes happen through `Engine::apply_turn(&mut self, commands: &[Command]) -> Vec<Event>`.

---

## Step 1 — Add the Command variant

In `game_core/src/commands.rs`, add a new variant to the `Command` enum:

```rust
pub enum Command {
    // existing variants …
    SetColonyFocus {
        colony: ColonyId,
        prod_pct: u8,
        research_pct: u8,
    },
}
```

Commands are data only — no methods, no logic.

---

## Step 2 — Add validation and state mutation in `apply_turn`

In `game_core/src/engine.rs`, add a match arm:

```rust
Command::SetColonyFocus { colony, prod_pct, research_pct } => {
    let total = u16::from(*prod_pct) + u16::from(*research_pct);
    if total != 100 {
        events.push(Event::Error {
            message: format!("focus must sum to 100, got {total}"),
        });
        continue;
    }
    match self.state.colonies.get_mut(colony) {
        None => events.push(Event::Error { message: "colony not found".into() }),
        Some(c) => { c.focus = ColonyFocus { prod_pct: *prod_pct, research_pct: *research_pct }; }
    }
}
```

Rules:
- Validate first, mutate only on success.
- Push `Event::Error` for every failure path and `continue` — never panic.
- Do not call `apply_turn` recursively.

---

## Step 3 — Add Event variants where needed

In `game_core/src/events.rs`:

```rust
pub enum Event {
    // existing variants …
    ColonyFocusSet { colony: ColonyId },
}
```

Emit the event after a successful state change if the UI needs to know about it.

---

## Step 4 — Update state types

In `game_core/src/state.rs`, add or modify structs as needed. Keep structs small and focused.

---

## Step 5 — Deterministic ordering

If the new feature iterates a `HashMap`, sort keys before iterating:

```rust
use crate::deterministic::sorted_colony_ids;
for id in sorted_colony_ids(&self.state.colonies) { … }
```

Add a `sorted_*_ids` helper in `deterministic.rs` if it doesn't exist yet.

---

## Step 6 — Write tests

In `game_core/src/engine.rs` or a dedicated `tests/` module:

```rust
#[test]
fn set_colony_focus_valid_updates_state() { … }        // positive

#[test]
fn set_colony_focus_invalid_sum_emits_error() { … }   // negative

#[test]
fn set_colony_focus_unknown_colony_emits_error() { … } // negative
```

---

## Checklist

- [ ] `Command` variant added
- [ ] Validation and mutation in `apply_turn`
- [ ] `Event` variant added if UI needs notification
- [ ] State struct updated
- [ ] Deterministic ordering used for any map iteration
- [ ] Positive test added
- [ ] At least one negative/error test added
- [ ] No UI/terminal imports introduced
