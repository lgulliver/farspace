# Testing Standards

## Coverage Requirement

Total workspace coverage must stay at or above **80%**, enforced by CI via `cargo llvm-cov`.

Coverage is checked on every push and pull request. A PR that drops coverage below 80% will not merge.

---

## What to Test

### Core logic (`game_core`)

Cover every meaningful path through:

- Command validation (valid inputs, invalid inputs, boundary values)
- Turn processing and event emission
- Budget and resource calculations
- Galaxy generation (use fixed seeds)
- Colony production queue processing
- Save/load round-trips
- Deterministic ordering helpers

### TUI (`game_tui`)

Avoid fragile full-screen character snapshots. Instead focus on:

- App state transitions (e.g., active screen changes, toggle help/palette)
- Key input handling (correct commands dispatched, correct state updates)
- Layout decisions (correct constraint splits, resize handling)
- Rendering smoke tests: assert the output is non-empty and contains expected text strings

### Save/load (`game_save`)

- Valid save → deserialises to identical `GameState`
- Corrupted or truncated save → returns a descriptive error, does not panic
- Missing fields (forwards compatibility) → handled gracefully

---

## Test Kinds

### Positive tests

Assert the happy path: given valid input, the correct output is produced.

### Negative / error tests

Assert that invalid input is rejected cleanly: correct `Event::Error` is emitted, state is unchanged.

### Regression tests

Every bug fix must include a test that would have caught the bug. Name it after the issue or behaviour.

### Property-style tests

For deterministic systems, run the same commands with the same seed multiple times and assert identical results. For generation algorithms, run with varied seeds and assert invariants (e.g., star count in expected range, no duplicate IDs).

---

## Deterministic Systems

Any code that uses the seeded RNG must be tested with a **fixed seed**. Assert exact output so the test catches any change to generation order or RNG consumption.

```rust
let mut engine = Engine::new(42); // fixed seed
let events = engine.apply_turn(&[Command::EndTurn]);
assert_eq!(events, vec![Event::TurnAdvanced { new_turn: 2 }]);
```

---

## What Not to Over-Test

- Do not write exhaustive snapshot tests for entire rendered screens — they break on cosmetic changes.
- Do not test private implementation details that are likely to be refactored.
- Do not duplicate tests across layers: if `game_core` covers validation, `game_tui` only needs to verify the command is dispatched, not re-test the validation.

---

## Running Coverage Locally

```bash
# Install once
cargo install cargo-llvm-cov

# Run with summary
cargo llvm-cov --workspace --all-targets --summary-only

# Generate HTML report
cargo llvm-cov --workspace --all-targets --open
```
