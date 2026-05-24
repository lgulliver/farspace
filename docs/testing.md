# Testing Standards

This file defines mandatory testing expectations for FARSPACE.

## Coverage Gate

- Workspace line coverage must stay at or above **80%**.
- CI enforces this with `cargo llvm-cov`.
- A PR that drops total coverage below 80% is not mergeable.

## Required Test Types

### Positive and Negative Paths

Every meaningful feature change must include:

- positive-path tests (valid flow)
- negative/error-path tests (invalid input, rejected command, or failure behavior)

### Deterministic Tests

For simulation-affecting logic:

- use fixed seeds
- assert reproducible outcomes/events/state
- avoid non-deterministic dependencies in assertions

### Save/Load Round-Trip and Migration

- save/load round-trip tests are required for schema-affecting changes
- migration behavior must be tested for changed schema versions
- corrupted/missing-field cases must return errors, not panic

### Regression Tests

Every bug fix must include a regression test covering the failed behavior.

## TUI Testing Guidance

- Prefer state-transition and render-smoke tests
- Avoid brittle full-screen snapshot assertions
- Validate key handling and command dispatch behavior

## Local Validation Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
```

## E2E Guide

Use the deterministic E2E harness when a slice changes multi-turn game flow or information visibility.

```bash
cargo test -p game_e2e --test e2e_100_turn_playthrough -- --nocapture
cargo run -p game_e2e --bin e2e_runner -- --seed 12345 --turns 100 --report target/e2e/playthrough-report.json
```

When reviewing E2E output for diplomacy / intel slices, verify:

- intel gains appear deterministically at the same turns for the same seed
- dispatch and log summaries do not reveal rival research or colony details too early
- save/load or replay continuation preserves intel level and redaction behavior
