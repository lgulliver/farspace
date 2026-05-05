# Issue: Galaxy Map Screen

**Labels:** `tui`, `ux`, `copilot-ready`

## Goal

Implement the galaxy map screen in `game_tui`. Display all known stars as positioned dots/symbols on a 2D canvas. Support cursor navigation to select a star and show a detail sidebar.

## UI Behaviour

- `h`/`j`/`k`/`l` or arrow keys to move the star cursor
- `Enter` to select the star under the cursor (shows detail panel)
- `Esc` to deselect
- Star positions derived from `GameState` snapshot (fog-of-war aware)
- Known stars shown with name; unknown positions shown as `?`
- Selected star highlighted; detail panel shows name, colonies, fleets present

## Layout

```
┌──────────────────────────┬──────────────┐
│  Galaxy Map              │  Star Detail │
│  (star canvas)           │  (sidebar)   │
└──────────────────────────┴──────────────┘
```

- Canvas occupies ~75% width; sidebar ~25%
- Resize-safe: recalculate character positions from terminal size

## Acceptance Criteria

- [ ] Stars rendered at correct relative positions on the canvas
- [ ] Cursor movement wraps or clamps at canvas edges (define and document the choice)
- [ ] Selected star detail shown in sidebar
- [ ] Layout survives resize to small terminals (minimum 80×24) without panic
- [ ] `game_core` state read via snapshot only — no internal struct access

## Tests Required

- `cursor_moves_right_on_l_key` (positive)
- `selecting_star_populates_detail_panel` (positive)
- `esc_clears_selection` (positive)
- `render_smoke_test_contains_star_name` (smoke test)
