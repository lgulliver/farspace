# Issue: Event Log Panel

**Labels:** `tui`, `ux`, `copilot-ready`

## Goal

Implement a persistent event log panel in `game_tui`. The log records all events emitted by `Engine::apply_turn` and any UI-level notifications (errors, palette actions). It appears in the footer area across all screens.

## Behaviour

- New entries appended to the end; oldest entries scroll off when the buffer is full
- Maximum buffer size: 50 entries (configurable constant)
- Footer displays the most recent 2 entries
- Dedicated log screen (accessible via `r` / Reports or a separate binding) shows the full scrollable log
- Log entries are plain strings formatted by `game_tui` from `Event` variants — formatting is UI-only, not in `game_core`

## Acceptance Criteria

- [ ] `append_log(entry)` adds to the buffer and trims to max 50 entries
- [ ] Footer renders the last 2 entries
- [ ] Full log screen allows `j`/`k` scrolling through all entries
- [ ] Log is cleared on New Game
- [ ] `Event::TurnAdvanced` renders as `"Turn N started"`
- [ ] `Event::Error` renders as `"Error: <message>"`

## Tests Required

- `append_log_trims_to_max_capacity` (negative / boundary)
- `render_footer_shows_last_two_entries` (positive)
- `log_clear_empties_buffer` (positive)
- `turn_advanced_event_formats_correctly` (positive)
- `error_event_formats_correctly` (positive)
