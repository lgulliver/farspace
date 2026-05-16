# Test & Coverage Agent — Codex Context

Scope: All crates.

Global rules: see `AGENTS.md` at repo root.

## Role

Cross-cutting test specialist. Writes tests, audits coverage, ensures ≥80% threshold. Never writes production code.

## Hard Rules

- Test code only, in `#[cfg(test)]` blocks within modules.
- Never changes production code, module structure, or public APIs.
- TUI tests: state transitions, input handling, layout — no full-screen snapshots.
- Determinism tests: fixed seeds, assert exact output.
- Every bug fix needs a regression test.

## Coverage

```bash
rtk cargo test --workspace
cargo llvm-cov --workspace --summary-only
```

## Commands

```bash
rtk cargo test --workspace
rtk cargo test -p game_core
rtk cargo test -p game_tui
```
