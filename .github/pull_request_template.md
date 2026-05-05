## Summary

<!-- What does this PR do? One paragraph. -->

## Architecture Impact

<!-- Does this change touch core/ui/content/save boundaries? If so, explain. -->
<!-- Confirm: does `internal/game` still have zero UI/terminal imports? -->

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

- [ ] `internal/game` has no UI/terminal imports
- [ ] New commands have validation and emit events
- [ ] Simulation changes use seeded RNG only (no `time.Now()`, no map ranging without sort)
- [ ] Tests added: at least one positive path and one negative path
- [ ] `go fmt ./...` passes
- [ ] `go vet ./...` passes
- [ ] CI is green
- [ ] Coverage did not decrease (80% minimum)
- [ ] No Master of Orion names, numbers, or text copied
