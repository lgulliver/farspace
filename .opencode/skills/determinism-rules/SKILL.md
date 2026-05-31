---
name: determinism-rules
description: FARSPACE determinism requirements — seeded RNG, BTreeMap ordering, no wall-clock seeding. Required for reproducible simulation.
---

# Determinism Rules

FARSPACE simulation must be fully reproducible: same seed + same command sequence = identical output across all runs and platforms.

## The invariants

**1. Seeded RNG only**
- All randomness must come from the `Rng` stored in `GameState`
- The seed is set at game creation from a user-provided value or a fixed default
- Never use: `thread_rng()`, `OsRng`, `rand::random()`, `SystemTime`, `Instant`
- Pattern: `let value = state.rng.gen::<u32>();`

**2. No wall-clock seeding**
- `SystemTime::now()` and `Instant::now()` must never seed or influence simulation output
- They may be used for display purposes (e.g. showing elapsed real time in the TUI) but never in `game_core`

**3. Ordered iteration**
- `HashMap` iteration order is not defined and varies between runs
- In any simulation-critical path: use `BTreeMap` or collect and sort before iterating
- Pattern: `map.keys().collect::<Vec<_>>().into_iter().sorted()...`
- Prefer `BTreeMap<K, V>` for any collection iterated during `apply_turn` or `run_ai_turn`

**4. No external state in simulation**
- Environment variables, file modification times, process IDs, network state must never influence game outcome
- `game_core` has no I/O except the seeded RNG

## Testing for determinism

Fixed-seed tests are the canonical check:

```rust
#[test]
fn deterministic_output() {
    let seed = 42u64;
    let mut state1 = GameState::new(seed);
    let mut state2 = GameState::new(seed);
    // apply identical commands to both
    let events1 = apply_turn(&mut state1, cmd.clone());
    let events2 = apply_turn(&mut state2, cmd.clone());
    assert_eq!(events1, events2);
    assert_eq!(state1, state2);
}
```

## Red flags to audit

- `use std::time::{SystemTime, Instant}` in `crates/game_core/`
- `HashMap` in a struct stored in `GameState` that is iterated in `apply_turn`
- `thread_rng()` anywhere in `crates/game_core/`
- `rand::random::<T>()` (implicitly uses thread-local RNG)
