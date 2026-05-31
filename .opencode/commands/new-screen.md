---
description: Scaffold a new TUI screen in game_tui — creates screen module with keyboard navigation, contextual help, resize-safe layout, and wires into screen dispatcher
agent: tui-ux
---

Scaffold the `$ARGUMENTS` screen in `crates/game_tui/`.

Complete all steps:

1. **`src/screens/$ARGUMENTS_lower.rs`** — Create the screen module with:
   - A struct for screen-local state
   - `handle_key(key: KeyEvent) -> Option<Command>` — keyboard navigation; return `None` for display-only keys, `Some(Command)` to send to `game_core`
   - `render(frame: &mut Frame, area: Rect, state: &AppState)` — ratatui `Constraint`-based layout that responds correctly to any terminal size (resize-safe)
   - `?` key wired to display the help overlay component
   - `Esc` / `q` key to navigate back or exit

2. **`src/app.rs`** — Add the new screen variant to the `Screen` enum and its arm in the screen dispatcher

3. **Keyboard-first**: every action reachable by keyboard. No mouse-only affordances.

4. **No game logic**: only sends `Command` values to `game_core`. Renders from `Event` values and snapshot views — never reaches into `GameState` internals directly.

5. **Tests**:
   - State transition: pressing the expected key changes screen state correctly
   - Command dispatch: correct `Command` emitted for relevant keys
   - Layout: basic render does not panic at minimum terminal size (e.g. 80×24)

Run `rtk cargo test -p game_tui` and `rtk cargo clippy -p game_tui -- -D warnings` before finishing.
