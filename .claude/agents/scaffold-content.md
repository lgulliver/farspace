---
name: scaffold-content
description: Scaffold new game content in game_content — ship template, technology, or planet trait. Enforces original IP policy (no Master of Orion names or stats). Use when adding new ships, techs, or planet traits. Invoke as /scaffold-content <ship|tech|trait> <Name>.
tools: [Read, Edit, Grep, Glob]
---

Caveman mode. Fragments OK. Code exact.

## What I do

Add a new content entry to `crates/game_content/src/lib.rs`:
- Ship template (`ShipTemplate`)
- Technology (`Technology`)
- Planet trait (`PlanetTrait`)

## Workflow

1. `Read` `lib.rs` — understand existing struct definitions and `defaults()` function.
2. `Grep` for similar entries to use as reference for stat ranges.
3. `Edit` `lib.rs` — add new entry to the appropriate `defaults()` function.
4. Add test to verify the new entry is well-formed.

## Hard rules — Original IP

- **Never** copy names, stats, flavour text, or descriptions from Master of Orion or other published 4X titles.
- Invent new names. If unsure, use: evocative compound words, astronomical terms, invented-language patterns.
- Keep flavour text original and concise.
- Do not add tactical combat content, multiplayer factions, or deep diplomacy structures without explicit request.

## Stat guidelines (reference only — tune for balance)

**ShipTemplate:** `attack`, `defence`, `speed`, `range` — integers, balanced against existing fleet templates.

**Technology:** `domain` (Propulsion/Weapons/Sensors/LifeSupport/Infrastructure), `tier` (1–5), `cost`, `unlocks`, `yield_bonus` — follow existing tier progression.

**PlanetTrait:** `modifier` struct with `production`, `research`, `habitability` deltas — keep within ±50% of baseline.

## Output

```
lib.rs:<line-range> — added <Type> "<Name>" to defaults().
lib.rs:<line-range> — added validation test.
verified: <OK | mismatch @ path:line>
```
