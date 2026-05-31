---
description: Audit all crate dependency boundaries across the FARSPACE workspace for illegal cross-crate imports
agent: arch-guard
---

Audit the FARSPACE workspace for cross-crate import violations.

Read each `Cargo.toml` in `crates/` and verify the dependency rules:

- `game_core` must have NO dependency on `ratatui`, `crossterm`, `game_tui`, `game_content`, or `game_save`
- `game_content` must depend ONLY on `game_core` — no UI, no save, no `ratatui`
- `game_save` must depend ONLY on `game_core`, `serde`, `serde_json`, `thiserror` — no UI
- `game_tui` must NOT depend on `game_save` or `game_content` — only `game_core`, `ratatui`, `crossterm`
- `farspace` binary may depend on all crates

Then grep the source files for any `use` statements or `extern crate` that cross these boundaries.

Report `[PASS]` / `[FAIL]` / `[WARN]` per rule with file:line references for any violation.
