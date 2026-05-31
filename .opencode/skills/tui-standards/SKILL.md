---
name: tui-standards
description: FARSPACE TUI quality standards — keyboard-first navigation, resize-safe ratatui layouts, theme usage, modal patterns, and visual language rules
---

# TUI Standards

FARSPACE targets a polished, minimal terminal feel inspired by Neovim, K9s, and Lazygit. Every screen must meet these standards.

## Keyboard-first

- Every action must be reachable by keyboard
- No mouse-only affordances unless a keyboard alternative also exists
- Navigation keys follow the global keymap in `keys.rs` — check there before inventing new bindings
- `?` opens contextual help on every screen
- `:` opens the command palette from any screen
- `Esc` / `q` exits or goes back; be consistent

## Resize-safe layouts

All layouts use ratatui `Constraint`-based sizing:

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),   // fixed header
        Constraint::Min(0),      // flexible body
        Constraint::Length(1),   // fixed footer
    ])
    .split(frame.area());
```

Never hardcode pixel/cell dimensions. Always test that the layout does not panic at small sizes (e.g. 40×12).

Respond to `crossterm::event::Event::Resize` — ratatui handles redraws automatically but state that depends on area size must recalculate.

## Theme usage

Use `Theme` palette roles — never raw ANSI colour codes or hardcoded `Color::Rgb(...)` values.

Key roles (verify current names in `theme.rs`):
- Primary accent, secondary accent
- Background, surface, border
- Text normal, text dim, text highlight
- Status: success, warning, error

Reuse existing spacing constants and border styles before adding new ones.

## Visual language

- **Tone**: calm, cinematic, spacious — avoid telemetry-heavy widget density
- **Composition**: centred hierarchy with breathing room; avoid stacking multiple bordered boxes
- **Modals / overlays**: `Clear` the area, rounded border, close on `Esc`, centred in the terminal
- **Status line / footer**: subtle, single-line, legible — no colour noise
- **Headers**: consistent with other screens — check `components/header.rs` for the shared pattern
- **Lists and tables**: use `ratatui::widgets::List` or `Table` with consistent highlight style from `Theme`

## Command/Event contract

TUI code:
- Sends `Command` values to `game_core` via `app.rs` dispatch
- Reads `Event` values returned by `apply_turn` to update display state
- Never reaches into `GameState` fields directly for anything other than rendering read-only snapshots

## Testing

- State transitions: key press → expected `AppState` change
- Command dispatch: key press → expected `Command` emitted (or `None`)
- Render smoke test: `render()` does not panic at minimum size
- No full-screen snapshot tests — they are fragile and maintenance-heavy
