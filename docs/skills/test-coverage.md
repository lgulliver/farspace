# Skill: Test Coverage

A playbook for writing and maintaining tests that meet the 80% coverage gate.

---

## The Gate

CI runs `cargo llvm-cov --workspace --all-targets` and fails if total line coverage is below **80%**.

Check locally before pushing:

```bash
cargo llvm-cov --workspace --all-targets --summary-only
# or open an HTML report:
cargo llvm-cov --workspace --all-targets --open
```

---

## 80% Is a Floor, Not a Target

80% is the minimum — core logic should be covered far more thoroughly. Coverage below 80% means important paths are untested. Coverage above 80% is the normal, healthy state.

---

## Positive Tests

Assert the expected output for valid, in-range inputs:

```rust
#[test]
fn end_turn_advances_turn_counter() {
    let mut engine = Engine::new(1);
    let events = engine.apply_turn(&[Command::EndTurn]);
    assert_eq!(events, vec![Event::TurnAdvanced { new_turn: 2 }]);
}
```

---

## Negative Tests

Assert that invalid inputs are rejected cleanly:

```rust
#[test]
fn set_budget_sum_over_100_emits_error() {
    let mut engine = Engine::new(1);
    let events = engine.apply_turn(&[Command::SetBudget {
        empire: EmpireId(1),
        research_pct: 60,
        industry_pct: 60,
        civics_pct: 0,
    }]);
    assert!(matches!(&events[0], Event::Error { message } if message.contains("120")));
}
```

For every validation path in `apply_turn`, there must be at least one negative test.

---

## Regression Tests

Add a test for every bug fix. Name it to describe the incorrect behaviour that was observed:

```rust
#[test]
fn budget_pct_overflow_no_longer_wraps_silently() { … }
```

---

## Property-Style Tests

For deterministic generators, run the same inputs multiple times and assert invariants:

```rust
#[test]
fn galaxy_star_count_within_bounds() {
    for seed in [0u64, 1, 42, u64::MAX] {
        let galaxy = generate_galaxy(seed, 20);
        assert!(galaxy.stars.len() >= 10 && galaxy.stars.len() <= 30);
    }
}
```

---

## TUI Tests

Avoid testing rendered string content byte-by-byte. Instead:

- Assert state transitions: `app.handle_key(...)` → check `app.active` or `app.show_help`.
- Assert dispatched commands: check that a key press pushes the correct `Command` variant.
- Use ratatui's `TestBackend` for smoke tests: render to a buffer, assert it contains expected substrings.

---

## What Not to Over-Test

- **Don't** write exhaustive character-level snapshot tests — they break on cosmetic changes.
- **Don't** test Rust std or third-party library behaviour.
- **Don't** duplicate validation tests across layers — test validation in `game_core`; `game_tui` only needs to verify the command is dispatched.
- **Don't** add tests purely to hit a coverage number without asserting anything meaningful.
