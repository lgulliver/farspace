# FARSPACE Engine Contract (v0)

This document defines the **core game loop**, **resolution order**, and the **contract** between the
TUI (client) and the headless game engine (core). The goal is a deterministic, testable simulation
with a premium, keyboard-first terminal UX.

---

## Design Goals

- **Headless core**: simulation runs with no terminal dependencies.
- **Deterministic**: same seed + same commands ⇒ same results.
- **Command-driven**: UI sends commands; core validates/applies; core emits events.
- **Turn-based**: player issues orders, then a single End Turn resolves everything.
- **MoO-like feel, original content**: mechanics-inspired, no copied names/text/assets/numbers.

---

## Player Turn Flow (UX)

1. **Turn Report**
   - completions, sightings, battles, alerts
2. **Set Empire Priorities**
   - global budget sliders: Research / Industry / Civics
3. **Manage Colonies**
   - local focus, build queue
4. **Fleet Orders**
   - move, merge, split, stance
5. **Research Choice**
   - only when a tier completes: choose 1 of 3 options
6. **End Turn**

---

## Resources Model (MVP)

### Empire-level
- **Credits**: upkeep and later rushing (optional later)
- **Research Points (RP)**: accumulate toward breakthroughs

### Colony-level
- **Production (Prod)**: spent on local queue
- **Population (Pop)**: grows each turn; drives outputs

No ecology/habitat constraint in MVP; planet traits provide differentiation.

---

## Travel Model

- Fleets move via **ETA turns**.
- A move order sets `InTransit{From, To, EtaTurns}`.
- Each resolution step decrements ETA.
- On ETA reaching 0, fleet arrives; visibility and conflict checks occur.

---

## Combat Model (Phase 1)

- **Auto-resolve only** (deterministic).
- Trigger: hostile fleets present at the same star (and/or orbiting colony).
- Output: a readable summary event (losses, retreat, colony impact).

Tactical combat is a future mode and must not leak into core architecture.

---

## Turn Resolution Pipeline (Engine Truth)

The engine MUST resolve in this order:

1. **Apply player commands**
   - validate and apply in deterministic order
2. **AI planning**
   - AI produces commands based on visible snapshot
3. **Apply AI commands**
4. **Economy & upkeep**
5. **Colony production**
   - advance queues, complete items
6. **Research progress**
   - if breakthrough: generate 3 options; create a pending choice
7. **Movement step**
   - decrement ETAs, process arrivals
   - update fog-of-war visibility
8. **Conflict detection**
9. **Combat resolution**
10. **Events & triggers**
    - MVP: deterministic triggers only; random events later
11. **Victory checks**
12. **Emit Turn Report**
    - ordered list of events + alerts for the UI

---

## UI ↔ Core Contract

### Core interface (conceptual)
- `ApplyTurn(playerCommands) -> (newState, events)`
- `SnapshotFor(empireID) -> viewState` (fog-of-war filtered)
- `Save(state) / Load(bytes)`

The UI must only rely on **snapshots + events**, never internal core structures.

---

## Commands (UI → Core)

> All commands must be validated by the core. Invalid commands return an error event and are ignored.

### Empire Management
- `SetBudget{ EmpireID, ResearchPct, IndustryPct, CivicsPct }`
- `ChooseTech{ EmpireID, TechID }` *(only when a choice is pending)*

### Colony Management
- `SetColonyFocus{ ColonyID, ProdPct, ResearchPct }`
- `QueueBuild{ ColonyID, ItemID }`
- `ReorderQueue{ ColonyID, FromIndex, ToIndex }`
- `CancelQueueItem{ ColonyID, Index }`

### Fleet Management
- `MoveFleet{ FleetID, ToStarID }`
- `MergeFleets{ FleetAID, FleetBID }`
- `SplitFleet{ FleetID, ShipStacks[] }`
- `SetFleetStance{ FleetID, Stance }` *(Engage / Avoid / Blockade)*

---

## Events (Core → UI)

> Events are the source of truth for the Turn Report and notifications.

### Economy / Production
- `BuildCompleted{ ColonyID, ItemID }`
- `ShipBuilt{ ColonyID, ShipDesignID, Count }`

### Research
- `TechBreakthrough{ EmpireID, Options[3] }`
- `TechChosen{ EmpireID, TechID }`

### Exploration / Visibility
- `StarSighted{ EmpireID, StarID }`
- `ColonySighted{ EmpireID, ColonyID }`
- `ContactMade{ EmpireAID, EmpireBID }`

### Movement / Conflict
- `FleetArrived{ FleetID, StarID }`
- `CombatResolved{ StarID, Summary }`

### Territory / Victory
- `ColonyCaptured{ ColonyID, ByEmpireID }`
- `Victory{ WinnerEmpireID, Type }`

---

## Minimal Core Data Model (MVP)

### GameState
- `Turn int`
- `Seed uint64`
- `Stars map[StarID]*Star`
- `Empires map[EmpireID]*Empire`
- `Fleets map[FleetID]*Fleet`

### Star / Colony
- `Star{ ID, Name, Pos(x,y), Colonies[] }`
- `Colony{ ID, Owner, Pop, Focus, Queue[], Buildings, Traits }`
  - Traits example: size, fertility, minerals (original content)

### Empire
- `Empire{ ID, Credits, Budget, ResearchState, KnownState }`
  - `KnownState` contains fog-of-war knowledge for this empire only

### Fleet
- `Fleet{ ID, Owner, Location, Ships[], Stance }`
  - `Location` is either `AtStar(StarID)` or `InTransit{From, To, EtaTurns}`

---

## Determinism Requirements (Non-Negotiable)

- All randomness comes from a seeded RNG stored in state (`Seed` + RNG state).
- Resolution order is fixed (see pipeline).
- When iterating maps, always use deterministic ordering (sort keys).
- Events are emitted in deterministic ordering.

---

## “First Playable” Definition

The game is considered playable when a user can:

- colonize multiple worlds
- build infrastructure and ships via queues
- choose tech from 3 options per breakthrough
- scout and reveal fog-of-war
- move fleets with ETAs
- fight via auto-resolve combat
- win by conquest

---

## Notes for the TUI (Non-Core)

Recommended UI affordances (not engine requirements):
- Turn Report screen with filters
- Command palette (`:`)
- Search in lists (`/`)
- Contextual help (`?`)
- Stable layout: top status bar, main pane, details pane, footer log/hints
