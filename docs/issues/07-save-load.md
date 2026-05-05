# Issue: Save / Load

**Labels:** `save-load`, `determinism`, `copilot-ready`

## Goal

Implement `game_save::save()` and `game_save::load()` to serialise and deserialise a full `GameState` to/from JSON bytes. Integrate save/load into the `farspace` binary (single auto-save slot). The loaded game must resume identically to a game that was never saved.

## API

```rust
// game_save/src/lib.rs
pub fn save(state: &GameState) -> Result<Vec<u8>, SaveError>;
pub fn load(bytes: &[u8]) -> Result<GameState, SaveError>;
```

## Save Format

- JSON via `serde_json`
- Wrapped in `SaveFile { version: u32, state: GameState }`
- `CURRENT_VERSION = 1`

## Integration

- Auto-save on End Turn to `~/.local/share/farspace/save.json` (Linux/macOS) or `%APPDATA%\farspace\save.json` (Windows)
- "Continue" on the main menu loads this file if it exists
- Save errors shown in the event log; do not crash the game

## Acceptance Criteria

- [ ] `save()` → `load()` round-trip produces an identical `GameState`
- [ ] Loading a saved game and replaying the same commands produces identical events (determinism)
- [ ] Corrupted/truncated JSON returns `SaveError::Malformed`, does not panic
- [ ] Missing file returns `SaveError::NotFound`, does not panic
- [ ] Old version number returns `SaveError::UnsupportedVersion` or migrates successfully
- [ ] `game_save` has no `game_tui` dependency

## Tests Required

- `save_load_round_trip_preserves_state` (positive)
- `save_load_determinism_replay` (determinism)
- `load_empty_bytes_returns_error` (negative)
- `load_truncated_json_returns_error` (negative)
- `load_wrong_version_returns_unsupported_or_migrates` (negative)
