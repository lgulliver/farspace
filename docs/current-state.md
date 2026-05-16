# FARSPACE Current State

Factual snapshot of implemented systems in current repository.

Status labels:

- **Done**: implemented in code and actively wired
- **In Progress**: implemented partially or with known scope limits
- **Needs verification**: signals exist but completion level is unclear without deeper feature audit/playtest

## Core Simulation

- **Status:** **Done**
- Deterministic turn processing in `game_core::Engine::apply_turn`
- Command/event model used for player and AI actions
- Headless core with no TUI dependency

## Galaxy / Sectors

- **Status:** **Done**
- Deterministic seeded galaxy generation with sector/star/planet structure
- Deterministic hyperspace lane generation
- Galaxy size presets and scenario setup support

## System View / Survey

- **Status:** **Done**
- System screen exists in TUI
- Survey command and mission lifecycle implemented
- Survey completion events and surveyed-planet gating present

## Colonization

- **Status:** **Done**
- Colonization command and validation implemented
- Colonization mission completion creates colony records/events
- Colony role assignment and rally point hooks available

## Economy

- **Status:** **In Progress**
- Yield model includes industry/credits/science/food/maintenance/stability effects
- Supply connectivity and blockade penalties implemented
- Deficit/shortage events implemented
- Balancing and long-term economy tuning still open

## Population / Jobs

- **Status:** **In Progress**
- Population value is part of colony simulation and yield model
- Food production/consumption and shortage signaling exist
- Dedicated job assignment system is **not** implemented yet

## Research

- **Status:** **In Progress**
- Large tech record set and unlock metadata present
- Active research + queue operations implemented (queue/reorder/remove/clear)
- Research progress/completion events and queued transition events implemented
- Unlock integration and future-hook signaling are still being expanded
- AI research weighting remains in progress

## Fleets / Ships

- **Status:** **In Progress**
- Multi-archetype ship design records present (including scout/science/colonizer/combat variants)
- Fleet missions, movement timing, standing orders, and rally routing implemented
- No dedicated ship designer UI; archetypes are predefined

## Combat

- **Status:** **In Progress**
- Deterministic strategic auto-resolve combat events implemented
- Strategic invasion system with troop transports and capture/failure outcomes implemented
- No tactical battle layer (out of scope)

## Diplomacy

- **Status:** **In Progress**
- Relationship states include Unknown/Contacted/Neutral/Tense/Hostile/War
- First contact, hostility/war combat eligibility, and declare-war command implemented
- Treaty/deal/alliance negotiation systems are not implemented

## AI

- **Status:** **In Progress**
- Deterministic AI turn driver exists (research/build/scout/colonize/colony-role decisions)
- Empire identity profiles influence AI weighting
- Advanced doctrine systems beyond current deterministic heuristics are pending

## TUI Screens

- **Status:** **Done**
- Screen modules: Menu, New Game Setup, Sector Overview, Sector Map, System, Colony, Empire Overview, Research, Diplomacy
- Global help overlay and command palette implemented
- Keyboard-first navigation and resize-safe layouts in active use

## Save / Load

- **Status:** **Done**
- Versioned schema with migration support (`CURRENT_VERSION = 27`)
- Save/load file and metadata APIs implemented
- TUI menu + command palette integration for save/load

## Testing / Coverage

- **Status:** **Done**
- Extensive unit/integration-style tests in core, TUI, and save crates
- CI includes fmt/clippy/test/coverage checks with 80% minimum coverage gate

## Repo-State Uncertainties

- Fleet movement animation fidelity level in all map contexts: **Needs verification**
- Minor-faction implementation status (vs major empires): **Needs verification**
- End-to-end replay log tooling for determinism audits: **Needs verification**
