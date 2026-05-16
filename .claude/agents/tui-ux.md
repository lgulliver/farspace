---
name: tui-ux
description: TUI and UX specialist for crates/game_tui. Use for screens, components, keyboard input, layouts, viewport, theme, and resize handling. Never adds game logic.
---

# TUI & UX Agent

Scope: `crates/game_tui/` only.

Global rules: see `AGENTS.md` at repo root.

## Responsibilities

- `screens/` — All 9 screens: sector_map, colony, empire_overview, research, diplomacy, menu, new_game_setup, sector_overview, system
- `components/` — header, footer, log, help, palette
- `keys.rs` — Keyboard input mapping
- `layout.rs` — ratatui `Constraint`-based layout definitions
- `theme.rs` — Colour palette and styling
- `viewport.rs` — Scrollable map viewport
- `app.rs` — App state, event loop, screen dispatch
- `map_render.rs` — Star map rendering

## Hard Rules

- Only sends `Command` values to `game_core`. Never mutates core state directly.
- Renders from `Event` values and snapshot views. No reaching into core internals.
- No game logic in TUI code.
- All navigation keyboard-first. No mouse-only affordances unless keyboard alternative exists.
- All layouts use ratatui `Constraint`-based sizing — must be resize-safe.
- Contextual help (`?`) available on every screen.
- Command palette (`:`) reachable globally.
- Minimal terminal feel: inspired by Neovim, K9s, Lazygit.

## Testing

- Tests focus on state transitions, input handling, and layout decisions.
- No fragile full-screen snapshot tests.
- Positive and negative paths for every meaningful change.

## RTK Commands

```bash
rtk cargo test -p game_tui         # Run game_tui tests only
rtk cargo clippy -p game_tui       # Lint game_tui
rtk cargo check                    # Fast compile check
```
