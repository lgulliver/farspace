# Ship Designer Lite v1

FARSPACE uses a constrained, template-driven ship customisation system.  It is
**not** a freeform designer — players choose components within fixed slot layouts
per hull class, rather than freely placing modules or adjusting tonnage.

---

## Intentional Limitations

The following are deliberately out of scope for v1:

- Drag-and-drop layout editors
- Arbitrary slot placement or slot-count selection
- Reactor / power-grid simulation
- Tactical combat (no subsystem targeting)
- Refit queues
- Fighter simulation
- Ammunition, fuel, or heat mechanics

All customisation is hull-template-driven.

---

## Hull Classes

Each hull defines a fixed set of slots.  Players and the AI choose one
component per slot.

| Hull ID | Name | Role | Slots | Required Tech |
|---------|------|------|-------|---------------|
| 1 | Scout | Exploration | Engine, Utility | — |
| 2 | Colony Ship | Colonization | Engine, MissionModule | Habitat Seeding |
| 3 | Science Vessel | Survey | Engine, Utility | Survey Drones |
| 4 | Troop Transport | Invasion | Engine, Defense, MissionModule | Battle Doctrine |
| 5 | Fast Scout | Rapid Exploration | Engine, Utility | Rapid Transit Drives |
| 6 | Survey Cutter | Deep Survey | Engine, Utility, Utility | Advanced Survey Array |
| 7 | Colony Ark | Mass Colonization | Engine, MissionModule, Utility | Colonial Vanguard |
| 8 | Escort Frigate | Defensive Combat | Weapon, Defense, Engine, Utility | Perimeter Defense |
| 9 | Missile Frigate | Strike Combat | Weapon, Weapon, Defense, Engine | Long-Range Strike |
| 10 | Destroyer | Heavy Combat | Weapon, Weapon, Defense, Engine, Utility | Fleet Coordination |
| 11 | Patrol Corvette | Local Security | Weapon, Defense, Engine | Perimeter Defense |

**Future-hook hulls** (defined but not yet buildable without additional tech):
Cruiser, Carrier, Battleship.  IDs 12–14 are reserved.

### Hull Base Stats

Base stats before component modifiers:

| Hull | ATK | DEF | HP | Cost | Maint |
|------|-----|-----|----|------|-------|
| Scout | 1 | 1 | 10 | 50 | 1 |
| Colony Ship | 1 | 1 | 10 | 200 | 1 |
| Science Vessel | 1 | 1 | 10 | 100 | 1 |
| Troop Transport | 2 | 3 | 15 | 150 | 2 |
| Fast Scout | 1 | 1 | 10 | 75 | 1 |
| Survey Cutter | 1 | 1 | 10 | 150 | 2 |
| Colony Ark | 2 | 2 | 15 | 350 | 2 |
| Escort Frigate | 3 | 5 | 20 | 120 | 2 |
| Missile Frigate | 6 | 3 | 20 | 200 | 3 |
| Destroyer | 8 | 5 | 30 | 300 | 4 |
| Patrol Corvette | 2 | 3 | 15 | 80 | 1 |

---

## Slot Categories

| Category | Description |
|----------|-------------|
| `Weapon` | Offensive armament; adds attack |
| `Defense` | Armour and shields; adds defense/hp |
| `Engine` | Propulsion; affects movement |
| `Utility` | Sensors, cargo, targeting |
| `MissionModule` | Colonisation, survey, invasion payload |

Slots are ordered within each hull.  Component category must match slot
category exactly; the engine rejects mismatches.

---

## Components

### Weapon Package

| ID | Name | ATK | DEF | HP | Cost | Maint | Move | Required Tech |
|----|------|-----|-----|----|------|-------|------|---------------|
| 1 | Kinetic Battery | +2 | 0 | 0 | +20 | 0 | 0 | Kinetic Barriers |
| 2 | Missile Rack | +4 | -1 | 0 | +40 | +1 | -1 | Long-Range Strike |

### Defense Package

| ID | Name | ATK | DEF | HP | Cost | Maint | Move | Required Tech |
|----|------|-----|-----|----|------|-------|------|---------------|
| 10 | Reinforced Plating | 0 | +3 | +5 | +15 | 0 | 0 | Kinetic Barriers |
| 11 | Shield Matrix | 0 | +4 | +3 | +35 | +1 | 0 | Perimeter Defense |
| 12 | Point Defense Grid | -1 | +2 | 0 | +25 | 0 | 0 | Perimeter Defense |

### Engine Package

| ID | Name | ATK | DEF | HP | Cost | Maint | Move | Required Tech |
|----|------|-----|-----|----|------|-------|------|---------------|
| 20 | Chemical Thrusters | 0 | 0 | 0 | 0 | 0 | 0 | — |
| 21 | Ion Drive | 0 | 0 | 0 | +20 | 0 | +1 | Rapid Transit Drives |

### Utility Package

| ID | Name | ATK | DEF | HP | Cost | Maint | Move | Tags | Required Tech |
|----|------|-----|-----|----|------|-------|------|------|---------------|
| 30 | Targeting Suite | +1 | 0 | 0 | +15 | 0 | 0 | — | — |
| 31 | Long-Range Sensors | 0 | 0 | 0 | +10 | 0 | +1 | Sensors, LongRange | Neutrino Sensors |
| 32 | Cargo Pods | 0 | 0 | +2 | +5 | 0 | 0 | — | — |

### Mission Modules

| ID | Name | ATK | DEF | HP | Cost | Maint | Tags | Required Tech |
|----|------|-----|-----|----|------|-------|------|---------------|
| 40 | Colony Core | 0 | 0 | 0 | +50 | 0 | Colony | Habitat Seeding |
| 41 | Survey Array | 0 | 0 | 0 | +30 | 0 | Survey | Survey Drones |
| 42 | Troop Bays | +1 | 0 | 0 | +40 | +1 | Invasion | Battle Doctrine |

---

## Ship Design Model

A `CustomShipDesign` captures:

```
design_id       CustomDesignId  — stable, monotonically increasing per game
hull_id         HullId          — which hull template
components      Vec<ComponentId> — one per slot, in slot order
owner           EmpireId        — creating empire
name            String          — display name (auto or player-chosen)
obsolete        bool            — if true, cannot be queued for production
```

Designs are stored in `GameState::custom_designs: BTreeMap<CustomDesignId, CustomShipDesign>`.

---

## Stat Derivation

Derived stats are computed on demand (not stored):

```
attack   = hull.base_attack  + Σ component.attack_modifier   (min 1)
defense  = hull.base_defense + Σ component.defense_modifier  (min 1)
hp       = hull.base_hp      + Σ component.hp_modifier       (min 1)
cost     = hull.base_cost    + Σ component.cost_modifier     (min 1)
maint    = hull.base_maintenance + Σ component.maintenance_modifier (min 0)
```

All arithmetic uses `i32`/`i64` intermediates — no floating point.

Special capabilities:
- **Invasion strength** = 12 if hull is TroopTransport or any component has tag `Invasion`, else 0
- **Survey effectiveness** = 100 if hull is Science/SurveyCutter or any component has tag `Survey`, else 0

---

## Commands and Events

### Commands

| Command | Description |
|---------|-------------|
| `CreateShipDesign { hull_id, components, name }` | Create a new custom design for the player empire |
| `DeleteShipDesign { design_id }` | Remove a player-owned design |
| `QueueBuild { colony, item: BuildItem::CustomShip(design_id) }` | Queue a custom design for production |

### Events

| Event | Description |
|-------|-------------|
| `ShipDesignCreated { design_id, hull_id, owner, name }` | Design created successfully |
| `ShipDesignDeleted { design_id }` | Design removed |
| `ShipDesignInvalid { reason }` | Creation or build rejected (reason is a static string) |
| `CustomShipConstructed { colony, design_id, fleet }` | Fleet built from custom design |

---

## AI Design Philosophy

AI empires auto-generate designs for each unlocked hull at game start and after
researching each new technology.  Designs are doctrine-weighted:

| Doctrine | Component Preference |
|----------|----------------------|
| Militarist / Imperial | High attack; Kinetic Battery, Missile Rack |
| Isolationist | High defense/hp; Reinforced Plating, Shield Matrix |
| Explorer / Expansionist | Movement, sensors; Ion Drive, Long-Range Sensors |
| Merchant | Low maintenance, long range; Chemical Thrusters |
| Default | Balanced attack + defense |

AI doctrines map to `AiDoctrine` variants (see `docs/design/ai-doctrines.md`).

At game setup, `ai_generate_designs()` is called for each AI empire.  Whenever
a tech is completed, `ai_generate_designs()` is called again so new hull
unlocks are immediately equipped with doctrine-appropriate designs.

Component selection is deterministic: for each slot, all unlocked components of
the correct category are scored and the highest-scoring one is chosen.  Tie-
breaking uses the component's numeric ID (ascending) to preserve determinism
even if scoring produces equal scores.

---

## Tech Tree Integration

Components are gated behind techs already in the tree:

| Component | Unlocking Tech (ID) |
|-----------|---------------------|
| Kinetic Battery | Kinetic Barriers (4) |
| Reinforced Plating | Kinetic Barriers (4) |
| Missile Rack | Long-Range Strike Doctrine (17) |
| Shield Matrix | Perimeter Defense Doctrine (16) |
| Point Defense Grid | Perimeter Defense Doctrine (16) |
| Ion Drive | Rapid Transit Drives (13) |
| Long-Range Sensors | Neutrino Sensors (3) |
| Colony Core | Habitat Seeding (2) |
| Survey Array | Survey Drones (12) |
| Troop Bays | Battle Doctrine (11) |

Components with `required_tech = None` (Chemical Thrusters, Targeting Suite,
Cargo Pods) are available from the start of the game.

---

## TUI — Ship Designer Screen

Opened with `W` from any in-game screen.

### Layout

```
┌ Ship Designs ──────────┬ Slot Configuration ───────────┬ Design Stats ─────────┐
│ [New Design]           │ Hull: Escort Frigate           │ Hull: Escort Frigate  │
│ Scout [EXP]            │                                │ Role: Defensive Combat│
│ Escort Frigate [MIL]   │ Slot 1: Weapon Package         │                       │
│                        │  ● Kinetic Battery  ATK +2     │ ATK:  5               │
│                        │  ○ Missile Rack     ATK +4     │ DEF:  8               │
│                        │                                │ HP:   25              │
│                        │ Slot 2: Defense Package        │ Cost: 155             │
│                        │  ● Reinforced Plating DEF +3   │ Maint: 3/turn         │
│                        │                                │                       │
│                        │ Slot 3: Engine Package         │ [s] Save              │
│                        │  ● Chemical Thrusters          │ [d] Delete            │
├────────────────────────┴────────────────────────────────┴───────────────────────┤
│ [n]New  [Enter]Select  [Tab]Panel  [s]Save  [d]Delete  [Esc]Back  [?]Help       │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Keyboard bindings

| Key | Action |
|-----|--------|
| `W` | Open Ship Designer (global) |
| `n` | New design |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `l` | Cycle component within slot |
| `Enter` | Confirm selection |
| `Tab` | Cycle panel focus |
| `s` | Save design |
| `d` | Delete design |
| `Esc` | Cancel / back |
| `?` | Help overlay |

Locked components (tech not yet researched) are shown greyed out and cannot
be selected.

---

## Future Expansion Path

The following can be added in later releases without breaking v1 designs:

1. **More components** — Plasma Emitter, Railgun, Phase Drive, ECM Suite,
   Siege Pods: add to `all_components()` with tech gates and numeric IDs above
   existing ones.
2. **More hulls** — Cruiser, Carrier, Battleship: add to `all_hull_templates()`
   with IDs 12–14 and appropriate slot layouts.
3. **Refit queues** — mark existing fleets as "refit pending" on design change.
4. **Obsolete-on-update** — when a player edits a design, mark old version
   obsolete automatically.
5. **Faction-specific components** — add empire-gated components checked in
   `validate()`.
6. **Player design naming** — already supported via `name` field and
   `CreateShipDesign { name: Some(...) }`.
