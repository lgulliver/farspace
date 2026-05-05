# Issue: Main Menu Screen

**Labels:** `tui`, `ux`, `copilot-ready`

## Goal

Implement the main menu screen in `game_tui`. This is the first screen the player sees. It should offer New Game, Continue (disabled until save/load is implemented), and Quit.

## UI Behaviour

- Keyboard navigation: `j`/`↓` and `k`/`↑` to move selection; `Enter` to confirm
- `q` or `Ctrl-C` quits from the main menu
- "Continue" is rendered as dimmed/disabled until a save file exists
- `?` shows a help overlay with keybindings

## Layout

- Centred vertically and horizontally in the terminal
- Game title "FARSPACE" rendered prominently above the menu
- No header/footer chrome (main menu is full-screen)

## Acceptance Criteria

- [ ] Selecting "New Game" transitions to the galaxy screen (with a default or entered seed)
- [ ] Selecting "Quit" exits the application
- [ ] "Continue" is visually disabled when no save exists
- [ ] Resize events do not break the layout
- [ ] `?` toggles a help overlay listing menu keybindings

## Tests Required

- `main_menu_initial_selection_is_new_game` (positive)
- `down_key_moves_selection` (positive)
- `enter_on_quit_returns_exit_signal` (positive)
- `help_toggle_shows_and_hides_overlay` (positive)
- `render_produces_non_empty_buffer_containing_farspace_title` (smoke test)
