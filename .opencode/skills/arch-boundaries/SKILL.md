---
name: arch-boundaries
description: FARSPACE crate dependency rules — which crates may import which, and what constitutes a boundary violation
---

# Architecture Boundary Rules

FARSPACE has five crates with strict one-way dependencies. A boundary violation is any import or Cargo.toml dependency that crosses these rules.

## Legal dependency graph

```
game_core   ←── game_content
            ←── game_save
            ←── game_tui
            ←── farspace
                    ↑
         game_content, game_save, game_tui
```

## Per-crate rules

**`game_core`**
- Depends on: `std` only
- Must never import: `ratatui`, `crossterm`, `game_tui`, `game_content`, `game_save`
- Must never have `game_tui` in `Cargo.toml`

**`game_content`**
- Depends on: `game_core` types only
- Must never import: `game_tui`, `ratatui`, `crossterm`, `game_save`

**`game_save`**
- Depends on: `game_core`, `serde`, `serde_json`, `thiserror`
- Must never import: `game_tui`, `ratatui`, `crossterm`, `game_content`

**`game_tui`**
- Depends on: `game_core`, `ratatui`, `crossterm`
- Must never import: `game_save`, `game_content`
- Must not mutate `GameState` directly — all state changes go through `Command`
- Must render from `Event` values and snapshot views only

**`farspace` (binary)**
- May depend on all crates
- Wires the layers together at startup only

## What to look for

- `use game_tui::` in any file under `crates/game_core/`
- `use ratatui::` or `use crossterm::` in `game_core`, `game_content`, or `game_save`
- `game_tui = ...` in `crates/game_core/Cargo.toml` or `crates/game_save/Cargo.toml`
- Direct `GameState` field mutations in `crates/game_tui/` instead of via `Command`
