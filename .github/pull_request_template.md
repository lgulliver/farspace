## Summary

<!-- What does this PR do? One paragraph. -->

## Architecture Impact

<!-- Does this change touch core/tui/content/save crate boundaries? If so, explain. -->
<!-- Confirm: does `game_core` still have zero UI/terminal imports? -->

## Test Coverage

### Positive test cases added

<!-- List new happy-path tests -->

### Negative / error test cases added

<!-- List new error-path or invalid-input tests -->

### Deterministic behaviour considered

<!-- Does this touch simulation logic? If yes, confirm fixed-seed tests are included. -->

## Screenshots / Terminal Recordings

<!-- For TUI changes: attach a screenshot or asciinema recording. Delete if not applicable. -->

## Checklist

- [ ] `game_core` has no `ratatui`/`crossterm`/UI imports
- [ ] New commands have validation and emit events
- [ ] Simulation changes use seeded RNG only (no `SystemTime`, no unsorted `HashMap` iteration)
- [ ] Tests added: at least one positive path and one negative path
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] CI is green
- [ ] Coverage did not decrease (80% minimum)
- [ ] No Master of Orion names, numbers, or text copied
