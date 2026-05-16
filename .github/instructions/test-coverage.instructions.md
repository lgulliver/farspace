---
applyTo: "**"
---

# Test & Coverage — Copilot Instructions

Scope: All crates (cross-cutting).

Global rules: see `.github/copilot-instructions.md`.

## Role

Test writing and coverage. Applies everywhere.

## Hard Rules

- Minimum 80% line coverage enforced by CI (`cargo llvm-cov`). Coverage must not decrease.
- Every meaningful change needs a positive path test and a negative/error path test.
- Every bug fix needs a regression test.
- Determinism tests: use fixed seeds, assert exact output.
- TUI tests: state transitions, input handling, layout decisions — no full-screen snapshot tests.
- Tests live in `#[cfg(test)]` blocks within the module, not in separate test files.

```bash
rtk cargo test --workspace
cargo llvm-cov --workspace --summary-only
```
