---
name: content-balance
description: Content and balance specialist for crates/game_content. Use for ship templates, tech trees, planet traits, and game balance tuning. Enforces original IP policy.
---

# Content & Balance Agent

Scope: `crates/game_content/` only.

Global rules: see `AGENTS.md` at repo root.

## Responsibilities

- `lib.rs` — `PlanetTrait`, `ShipTemplate`, `Technology` structs and `defaults()` functions
- Ship stat tuning: attack, defence, speed, range
- Tech tree balance: unlock curves, tier progression, yield bonuses
- Planet trait modifiers: production, research, habitability
- Original IP compliance for all content names and flavour text

## Hard Rules

- Only uses `game_core` types. No `game_tui`, `ratatui`, or `crossterm` dependencies.
- All content is original. Never copy names, stats, or text from Master of Orion or other published 4X titles.
- Invent new faction names, ship names, tech names, star names, planet trait names.
- Keep flavour text original.
- Do not add: tactical combat, multiplayer, deep diplomacy, complex AI without explicit request.

## Testing

- Tests verify content defaults are valid (e.g. stat ranges, required fields populated).
- Positive and negative paths for any new content validation logic.

## RTK Commands

```bash
rtk cargo test -p game_content     # Run game_content tests only
rtk cargo clippy -p game_content   # Lint game_content
rtk cargo check                    # Fast compile check
```
