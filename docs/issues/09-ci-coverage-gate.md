# Issue: CI Coverage Gate

**Labels:** `testing`, `copilot-ready`

## Goal

Add a CI job that runs `cargo llvm-cov` and fails the build if total workspace line coverage is below 80%. The gate must run on every push and pull request.

## Implementation

- Install `cargo-llvm-cov` via `taiki-e/install-action@cargo-llvm-cov`
- Run `cargo llvm-cov --workspace --all-targets --lcov --output-path lcov.info`
- Extract the total line coverage percentage and fail if below 80%
- Upload `lcov.info` as a build artifact

## Notes

- The coverage job runs only on `ubuntu-latest` (no need to repeat on all OS)
- Coverage runs after the `test` job (depend on it or run independently — either is acceptable)
- If coverage tooling is unavailable for a specific target, document the limitation in `docs/testing.md`

## Acceptance Criteria

- [ ] `coverage` job appears in `.github/workflows/ci.yml`
- [ ] CI fails on a PR that drops coverage below 80%
- [ ] CI passes when coverage is ≥80%
- [ ] `lcov.info` uploaded as artifact on each run
- [ ] `docs/testing.md` documents how to run coverage locally

## Tests Required

None — this is a CI/tooling change. Validate by inspecting the workflow run logs.
