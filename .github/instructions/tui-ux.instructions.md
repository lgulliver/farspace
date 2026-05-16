---
applyTo: "crates/game_tui/**"
---

# TUI & UX — Copilot Instructions

Scope: `crates/game_tui/` only.

Global rules: see `.github/copilot-instructions.md`.

## Role

Terminal UI rendering and input handling. No game logic.

## Hard Rules

- Only sends `Command` values to `game_core`. Never mutates `GameState` directly.
- Renders from `Event` values and snapshot views only. Never reaches into core internals.
- All layouts use ratatui `Constraint`-based sizing — must be resize-safe.
- Keyboard-first navigation. No mouse-only affordances without keyboard alternative.
- `?` contextual help available on every screen.
- `:` command palette reachable globally.
- No game logic in TUI code.

## Testing

State transitions, input handling, layout decisions. No fragile full-screen snapshot tests.

```bash
rtk cargo test -p game_tui
rtk cargo clippy -p game_tui
```
