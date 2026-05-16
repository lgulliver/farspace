# Content & Balance Agent — Codex Context

Scope: `crates/game_content/` only.

Global rules: see `AGENTS.md` at repo root.

## Role

Specialist for static game content and balance. Handles ship templates, tech trees, planet traits, and stat tuning.

## Key Files

- `crates/game_content/src/lib.rs` — `PlanetTrait`, `ShipTemplate`, `Technology`, `defaults()`

## Hard Rules

- Only uses `game_core` types. No `game_tui`, `ratatui`, or `crossterm`.
- All content is original. Never copy from Master of Orion or other 4X titles.
- Invent new names for factions, ships, techs, stars, planet traits.
- Do not add tactical combat, multiplayer, deep diplomacy, or complex AI without explicit request.

## Testing

Validate content defaults are well-formed. Positive + negative paths for validation logic.

## Commands

```bash
rtk cargo test -p game_content
rtk cargo clippy -p game_content
```
