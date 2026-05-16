---
applyTo: "crates/game_content/**"
---

# Content & Balance — Copilot Instructions

Scope: `crates/game_content/` only.

Global rules: see `.github/copilot-instructions.md`.

## Role

Static game content and balance. Ship templates, tech trees, planet traits.

## Hard Rules

- Only uses `game_core` types. No `game_tui`, `ratatui`, or `crossterm` dependencies.
- All content is original. Never copy names, stats, or text from Master of Orion or other 4X titles.
- Invent new names for factions, ships, techs, stars, and planet traits.
- Do not add tactical combat, multiplayer, deep diplomacy, or complex AI without explicit request.

## Testing

Validate content defaults are well-formed. Positive + negative paths for validation logic.

```bash
rtk cargo test -p game_content
rtk cargo clippy -p game_content
```
