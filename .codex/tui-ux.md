# TUI & UX Agent — Codex Context

Scope: `crates/game_tui/` only.

Global rules: see `AGENTS.md` at repo root.

## Role

Specialist for terminal UI. Handles screens, components, keyboard input, layouts, viewport, and theme.

## Key Files

- `crates/game_tui/src/screens/` — 9 screens (sector_map, colony, empire_overview, research, diplomacy, menu, new_game_setup, sector_overview, system)
- `crates/game_tui/src/components/` — header, footer, log, help, palette
- `crates/game_tui/src/keys.rs` — Keyboard input
- `crates/game_tui/src/layout.rs` — ratatui layouts
- `crates/game_tui/src/theme.rs` — Colours and styles
- `crates/game_tui/src/viewport.rs` — Map viewport/scrolling
- `crates/game_tui/src/app.rs` — App state and event loop

## Hard Rules

- Only sends `Command` values to `game_core`. Never mutates `GameState` directly.
- Renders from `Event` values and snapshot views only.
- No game logic in TUI code.
- Keyboard-first. No mouse-only affordances without keyboard alternative.
- All layouts use ratatui `Constraint`-based sizing — resize-safe.
- `?` help available on every screen. `:` command palette reachable globally.

## Testing

State transitions, input handling, layout decisions. No full-screen snapshot tests.

## Commands

```bash
rtk cargo test -p game_tui
rtk cargo clippy -p game_tui
```
