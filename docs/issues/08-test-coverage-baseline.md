# Issue: Test Coverage Baseline

**Labels:** `testing`, `copilot-ready`

## Goal

Establish a test suite that brings total workspace coverage to ≥80% and ensures every existing code path has both positive and negative tests. This issue is about breadth — ensuring no significant existing code is untested.

## Scope

Audit existing tests against:

- `game_core::engine::apply_turn` — all command arms, all validation branches
- `game_core::deterministic` — sorting helpers
- `game_save::save` / `game_save::load` — round-trip, error paths
- `game_tui::app` — key handling, screen transitions, overlay toggles

For each uncovered branch, add the missing test.

## Acceptance Criteria

- [ ] `cargo llvm-cov --workspace --summary-only` reports ≥80% line coverage
- [ ] Every `Command` variant has at least one positive and one negative test
- [ ] Every `Event` variant is asserted in at least one test
- [ ] Every `SaveError` variant is produced by at least one test
- [ ] Coverage report uploaded as CI artifact

## Tests Required

This issue is entirely about tests. No production code changes unless a bug is found during the audit (in which case fix the bug and add a regression test).
