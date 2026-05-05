# Skill: Deterministic Simulation

A playbook for writing and maintaining deterministic game logic in `game_core`.

---

## Core Rule

> Same seed + same commands ⇒ same output, always, on all platforms.

Any deviation from this rule is a bug.

---

## Seeded RNG

Use `rand::rngs::StdRng` seeded from the `GameState.seed` field. Store the RNG state in `GameState` so it advances deterministically with each turn.

```rust
use rand::{rngs::StdRng, SeedableRng, Rng};

// Initialise once in Engine::new
let rng = StdRng::seed_from_u64(seed);

// Use in turn resolution
let roll: u32 = self.state.rng.gen_range(1..=100);
```

**Never** seed from `SystemTime`, `Instant`, thread-local state, or any OS entropy source after initialisation.

---

## No Wall-Clock Dependencies

Simulation logic must not call:
- `std::time::SystemTime::now()`
- `std::time::Instant::now()`
- `std::thread::sleep()`

These are allowed only in `game_tui` (for frame timing) and `farspace` (for initial seed from entropy).

---

## Stable Iteration Ordering

`HashMap` iteration order is unspecified in Rust. Never iterate a `HashMap` directly in simulation logic.

**Preferred:** Use `BTreeMap` for collections that must iterate in sorted order.

**Alternative:** Sort keys before iterating using helpers in `deterministic.rs`:

```rust
for id in sorted_empire_ids(&self.state.empires) {
    let empire = &self.state.empires[&id];
    // …
}
```

Apply this rule to:
- Turn resolution loops
- Event emission order
- Galaxy generation iteration
- Any place where order affects RNG consumption

---

## Reproducible Tests

Fix the seed in every test that involves RNG or generation:

```rust
#[test]
fn galaxy_generation_is_reproducible() {
    let result_a = generate_galaxy(42, 20);
    let result_b = generate_galaxy(42, 20);
    assert_eq!(result_a, result_b);
}

#[test]
fn different_seeds_produce_different_galaxies() {
    let a = generate_galaxy(1, 20);
    let b = generate_galaxy(2, 20);
    assert_ne!(a.stars, b.stars);
}
```

---

## Save/Load Determinism

A loaded game must resume as if it was never saved:

1. Save `GameState` including full RNG state.
2. Load into a fresh `Engine`.
3. Apply the same sequence of commands.
4. Assert identical events and final state.

Test this explicitly — see `docs/skills/save-load.md`.

---

## Checklist

- [ ] New random draws use `self.state.rng`, not a fresh/local RNG
- [ ] No `SystemTime` or `Instant` in `game_core`
- [ ] Map iteration uses sorted keys or `BTreeMap`
- [ ] Events emitted in deterministic order
- [ ] Fixed-seed tests assert exact output
- [ ] Save/load preserves RNG state
