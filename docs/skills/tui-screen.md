# Skill: TUI Screen

A playbook for adding or modifying a screen in `game_tui`.

---

## Where screen code lives

```
game_tui/src/
  app.rs            # AppState, top-level event routing, screen dispatch
  keys.rs           # KeyMap and key-matching helpers
  theme.rs          # Colour palette and shared styles
  layout.rs         # compose_layout() — header / main / footer split
  screens/
    mod.rs          # Screen enum and render dispatch
    galaxy.rs       # Galaxy map screen
    planets.rs      # Colony management screen
    fleets.rs       # Fleet overview screen
    tech.rs         # Research screen
    diplo.rs        # Diplomacy screen
    reports.rs      # Strategic reports screen
    turn_report.rs  # Turn report (events from last resolution)
  components/
    mod.rs
    header.rs       # render_header(area, turn, screen_name)
    footer.rs       # render_footer(area, hints, log)
    log.rs          # append_log / render_log
```

---

## How input flows

1. The `farspace` binary reads `crossterm::event::Event` in the main loop.
2. `crossterm::event::Event::Key(key_event)` is passed to `App::handle_key(&mut self, key: KeyEvent) -> bool`.
3. `handle_key` checks global bindings first (quit, help, palette, end turn, screen navigation).
4. If unhandled, it delegates to the active screen's `handle_key` method.
5. Screen handlers may push `Command`s to a pending buffer, which is flushed to `Engine::apply_turn` on End Turn.
6. `handle_key` returns `true` to signal the main loop to exit.

---

## Adding a new screen

1. Create `game_tui/src/screens/my_screen.rs`.
2. Implement a `render(frame: &mut Frame, area: Rect, state: &AppState)` function.
3. Optionally implement `handle_key(app: &mut AppState, key: KeyEvent)`.
4. Add a variant to the `Screen` enum in `screens/mod.rs`.
5. Add the render arm to the `Screen::render` dispatch match.
6. Add a keybinding in `KeyMap` and handle it in `App::handle_key`.

---

## Focus management

- The `AppState.active: Screen` field determines which screen receives key events.
- Overlays (help, palette) shadow the active screen: check `show_help` / `show_palette` before dispatching to the screen.
- Within a screen, use a local `focus: Focus` enum if there are multiple focusable panes.

---

## Resize handling

- `crossterm::event::Event::Resize` is handled in the main loop — just re-draw; ratatui recomputes layout automatically.
- Never store absolute pixel/character positions. Always derive dimensions from `frame.area()` or the `Rect` passed to render.
- Use `Layout::vertical` / `Layout::horizontal` with `Constraint::Min`, `Constraint::Length`, or `Constraint::Percentage`.

---

## Help overlay integration

- `AppState.show_help: bool` toggles the help overlay.
- In `App::draw`, if `show_help` is true, render the help panel over the main area.
- Each screen should provide a `help_text() -> &'static str` function listing its keybindings.

---

## Command palette integration

- `AppState.show_palette: bool` toggles the palette overlay.
- The palette is rendered by `components::palette::render` over the main area.
- Palette commands dispatch the same `Command`s as key bindings — no special path.

---

## Rendering expectations

- Every screen render function is pure: given the same `AppState`, it produces the same `Buffer`.
- Do not mutate state inside render.
- Tests: call `render` with a fixed `AppState`, assert the resulting `Buffer` contains expected strings.
