---
name: new-screen
description: Scaffold a new TUI screen in game_tui. Creates a screen module with keyboard navigation, contextual help (?), resize-safe ratatui layout, and wires it into the screen dispatcher. Use when adding a new game view. Invoke as /new-screen <ScreenName>.
tools: [Read, Edit, Write, Grep, Glob]
---

Caveman mode. Fragments OK. Code exact.

## What I do

Add a complete new screen to `crates/game_tui/src/screens/`:
1. New `<screen_name>.rs` module with render + input handling
2. Wire into `screens/mod.rs` dispatcher
3. Add `Screen` enum variant (or equivalent routing)
4. Tests for state transitions and key input

## Workflow

1. `Read` an existing screen (e.g. `colony.rs` or `research.rs`) as template.
2. `Read` `screens/mod.rs` to understand dispatch pattern.
3. `Read` `keys.rs` for input handling conventions.
4. `Write` new screen module.
5. `Edit` `screens/mod.rs` to register new screen.
6. Add `#[cfg(test)]` tests for key transitions.

## Screen requirements (non-negotiable)

- `?` key shows contextual help — wire to `HelpComponent` or equivalent.
- `:` opens command palette — must be passable to global handler.
- Layout uses ratatui `Constraint`-based sizing only — no hardcoded pixel/char sizes.
- Responds to `crossterm::event::Event::Resize` — layout recalculates on terminal resize.
- Only sends `Command` values to `game_core`. Never mutates `GameState` directly.
- Keyboard-first. No mouse-only affordances.
- Follows `docs/design/ux-splash-screen.md` visual language: calm, cinematic, spacious, readable.
- Reuses existing `Theme` palette roles and spacing/composition patterns before inventing new ones.

## Hard rules

- No game logic in screen code.
- No direct `GameState` field access — use snapshot views or `Event` data.
- Minimal terminal feel: clean lines, no ASCII art borders unless consistent with existing screens.
- Keep hierarchy centered and breathable; avoid dashboard-like clutter.

## Output

```
screens/<name>.rs — created, <N> lines.
screens/mod.rs:<line> — registered new screen.
screens/<name>.rs:<line-range> — added tests.
verified: <OK | mismatch @ path:line>
```
