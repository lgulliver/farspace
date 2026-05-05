# GitHub Labels

Suggested label set for the FARSPACE repository. Create these labels in GitHub → Issues → Labels.

| Label | Colour | Purpose |
|---|---|---|
| `core` | `#0075ca` | Touches `game_core` crate (commands, events, engine, state) |
| `tui` | `#e4e669` | Touches `game_tui` crate (screens, input, layout, rendering) |
| `content` | `#d93f0b` | Touches `game_content` crate (tech trees, ship templates, traits) |
| `save-load` | `#0052cc` | Touches `game_save` crate (serialisation, versioning, migration) |
| `testing` | `#bfd4f2` | Adds or fixes tests, coverage tooling, or CI |
| `determinism` | `#5319e7` | Affects reproducibility — seeded RNG, iteration order, save replay |
| `ux` | `#fbca04` | Keyboard bindings, help overlays, command palette, layout polish |
| `good-first-issue` | `#7057ff` | Self-contained, well-scoped, good entry point for a new contributor |
| `copilot-ready` | `#0e8a16` | Well-specified issue with clear acceptance criteria; safe for AI agent |
| `needs-design` | `#e99695` | Not ready to implement — needs design discussion first |
| `blocked` | `#b60205` | Waiting on another issue or external decision |

---

## Usage Notes

- Add `copilot-ready` only when the issue has acceptance criteria, affected crate, and test requirements filled in.
- Use `needs-design` freely to prevent premature implementation.
- `determinism` should be applied to any issue that touches RNG, turn resolution order, or save/load replay correctness.
