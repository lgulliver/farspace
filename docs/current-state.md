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

## Planet Specials & Anomalies

- **Status:** **Done**
- 17 planet specials across 9 categories (Resource, Scientific, Biological, Industrial, Environmental, Precursor, Hazard, Strategic, Cultural)
- 10 planet anomalies across 8 categories with rarity from Uncommon to Legendary
- Deterministic generation from galaxy seed and planet context (class, star spectral class, sector, frontier distance)
- Visibility gated by survey and tech tiers (basic survey reveals specials; advanced tech reveals anomalies)
- Discovery events (`PlanetSpecialDiscovered`, `AnomalyDetected`) emitted on survey completion
- Colony yield modifiers and AI valuation weights implemented
- Empire-aware visibility helpers (`visible_specials_for_empire`, `visible_anomalies_for_empire`)
- TUI renders specials with rarity/category/effect; anomalies with rarity/category/risk level
- Precursor hooks stored for future archaeology/translation/restoration systems

## Strategic Resources

- **Status:** **Done**
- 10 strategic resources with discovery, extraction, and tech requirements
- Empire resource access tracking (`empire_resource_access` field on `GameState`)
- Extraction requires colony control, buildings, supply connectivity, and tech
- Strategic resource discovery events on survey completion
- AI resource-weighted scoring for scout, colonize, and military targeting
- TUI rendering on system, colony, and diplomacy screens
- Resource glyphs in all visual modes (ASCII, Unicode, Nerd Font)

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
- Unlock integration for ships, buildings, resources, and components is active
- AI research weighting is implemented with doctrine-aligned profiles

## Fleets / Ships

- **Status:** **In Progress**
- Multi-archetype ship design records present (including scout/science/colonizer/combat variants)
- Fleet missions, movement timing, standing orders, and rally routing implemented
- Fleet supply/logistics v1 implemented with `Supplied` / `Extended` / `Out of Supply` states
- Sector/system views now surface projected/current fleet supply for movement and posture planning
- Custom ship designer with hull selection, component allocation, and validation present in TUI

## Combat

- **Status:** **In Progress**
- Deterministic strategic auto-resolve combat events implemented
- Strategic invasion system with troop transports and capture/failure outcomes implemented
- Battle reports record both fleets' supply states and TUI exposes logistics posture in report details
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
- AI doctrine weighting is implemented for research, production, expansion, and diplomacy drift

## Galactic Dispatch

- **Status:** **Done**
- Turn-based news system with periodic cadence (every 5 turns)
- Categories: Exploration, Economy, Military, Diplomacy, Research, Discovery
- Severity levels: Notice, Notable, Urgent, Historic
- Major discoveries (Rare+ specials, Precursor/Strategic categories) flagged for Dispatch
- Dispatch items generated deterministically from game events and state

## TUI Screens

- **Status:** **Done**
- Screen modules: Menu, New Game Setup, Sector Overview, Sector Map, System, Colony, Empire Overview, Research, Diplomacy, Ship Designer
- Global help overlay and command palette implemented
- Keyboard-first navigation and resize-safe layouts in active use
- Settings, Galactic Dispatch, Battle Reports, Update Confirm all use the overlay modal pattern (see `docs/design/ux-splash-screen.md`)
- Splash screen is the canonical UX reference: full-viewport starfield, palette-driven title/tagline/menu/footer, update notification banner

## Save / Load

- **Status:** **Done**
- Versioned schema with migration support (`CURRENT_VERSION = 36`)
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
