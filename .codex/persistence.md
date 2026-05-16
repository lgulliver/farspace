# Save & Persistence Agent — Codex Context

Scope: `crates/game_save/` only.

Global rules: see `AGENTS.md` at repo root.

## Role

Specialist for save/load, schema versioning, and migration. Must not break existing saves.

## Key Files

- `crates/game_save/src/lib.rs` — `save()`, `load()`, `save_to_file()`, `load_from_file()`
- `crates/game_save/src/schema.rs` — `SaveFile`, `SaveMetadata`, `CURRENT_VERSION`
- `crates/game_save/src/migrate.rs` — Migration logic

## Hard Rules

- Allowed deps only: `game_core`, `serde`, `serde_json`, `thiserror`. Never add `game_tui` or TUI deps.
- Every version bump requires a migration arm in `migrate.rs`.
- Migrations must never break existing valid save files.
- Never reuse a version number.

## Testing

Round-trip tests (save → load = identical state). Migration tests (old versions load correctly). Error paths (corrupted JSON, wrong version).

## Commands

```bash
rtk cargo test -p game_save
rtk cargo clippy -p game_save
```
