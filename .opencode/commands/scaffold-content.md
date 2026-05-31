---
description: Scaffold new game content in game_content — ship template, technology, or planet trait. Enforces original IP policy.
agent: content-balance
---

Scaffold new game content in `crates/game_content/`: $ARGUMENTS

Parse the type and name from the arguments, then:

**`ship <Name>`**
Add a `ShipTemplate` to `lib.rs` with:
- An original name (not from Master of Orion or other 4X titles)
- Balanced stats: `attack`, `defence`, `speed`, `range` — use existing templates as reference range
- A one-line original flavour description

**`tech <Name>`**
Add a `Technology` entry to the tech tree in `lib.rs` with:
- An original name and original flavour text
- A logical tier placement and balanced unlock cost
- Yield bonuses (production/research/food) consistent with that tier

**`trait <Name>`**
Add a `PlanetTrait` to `lib.rs` with:
- An original name
- Balanced modifiers: `production_bonus`, `research_bonus`, `habitability` — positive and negative traits welcome
- A one-line original flavour description

All names and flavour text must be original. Never copy or closely paraphrase from Master of Orion, Stellaris, GalCiv, or other published 4X titles.

Add validation tests for the new content (stat ranges in bounds, required fields populated).

Run `rtk cargo test -p game_content` before finishing.
