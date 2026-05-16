# Agent Workflow

Use this workflow for future implementation slices.

## Slice Discipline

- One prompt per slice
- Keep scope narrow and explicit
- Avoid broad refactors unrelated to slice goal

## Required Validation

After implementation run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
```

## Documentation Discipline

- Use [`docs/next-slices.md`](next-slices.md) as implementation queue
- Update [`docs/current-state.md`](current-state.md) after major slices
- Keep [`docs/roadmap.md`](roadmap.md) aligned with reality
- Keep architecture boundaries aligned with [`docs/architecture.md`](architecture.md)

## Handoff Checklist

- summarize what changed
- summarize tests/coverage results
- call out uncertainties explicitly
- identify recommended next slice
