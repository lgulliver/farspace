---
applyTo: "crates/game_save/**"
---

# Save & Persistence — Copilot Instructions

Scope: `crates/game_save/` only.

Global rules: see `.github/copilot-instructions.md`.

## Role

Save/load, schema versioning, migration. Must not break existing saves.

## Hard Rules

- Allowed deps only: `game_core`, `serde`, `serde_json`, `thiserror`. Never add `game_tui` or TUI crates.
- Every schema version bump requires a migration arm in `migrate.rs`.
- Migrations must never break existing valid save files.
- Never reuse a `CURRENT_VERSION` number.

## Testing

Round-trip (save → load = identical state). Migration (old versions load correctly). Error paths (corrupted JSON, wrong version).

```bash
rtk cargo test -p game_save
rtk cargo clippy -p game_save
```
