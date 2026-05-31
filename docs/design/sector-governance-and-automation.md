# Sector Governance and Automation (v1)

## Goal

Make larger empires manageable without removing player control. Sectors gain a
gameplay purpose beyond map navigation, and colonies can optionally automate
their production queue using an explainable, deterministic rule set.

This is a **conservative** automation system. It never overrides explicit
player decisions and always reports what it did.

## Non-goals (explicitly out of scope for v1)

- Opaque "governor" agents or leader characters.
- Complex budget sliders or resource reallocation.
- Autonomous diplomacy or autonomous fleet warfare.
- Full sector AI that plays the game for the player.

## Model

### Sectors

Sectors already exist as a spatial partition of the galaxy. Each `Star` carries
a `sector: SectorId`. A colony's sector is therefore derived from its star — no
new per-colony assignment storage is needed. A colony "belongs to" the sector of
the star it orbits.

Helper: `GameState::colony_sector(colony_id) -> Option<SectorId>` and
`GameState::colonies_in_sector(sector_id) -> Vec<ColonyId>` (sorted).

### Sector directives

Each sector has a `SectorDirective` (default `Balanced`). The directive biases
automation suggestions toward a focus. It is **advisory** — it never hard-forces
production and never modifies yields directly.

| Directive      | Focus                                            |
|----------------|--------------------------------------------------|
| Balanced       | No bias; general-purpose infrastructure order.   |
| Industrial     | Prefer production buildings (Fabrication Yard).  |
| Research       | Prefer science buildings (Science Nexus).        |
| Agricultural   | Prefer food buildings (Aquaculture Bay).         |
| Military       | Prefer Shipyard (orbital), then production.      |
| Economic       | Prefer production, then logistics (Supply Hub).  |
| Stabilization  | Address food/housing/unrest (Aquaculture Bay).   |

Directives are stored in `GameState::sector_directives: BTreeMap<SectorId,
SectorDirective>`. Absent entries are implicitly `Balanced`.

### Colony automation

Each colony has a `ColonyAutomation` mode (default `Manual`):

- `Manual` — the player controls the queue; the engine never touches it.
- `SectorGuided` — when the colony's build queue is **empty**, the engine
  deterministically picks one build item biased by the colony's sector
  directive, and emits an event describing the choice.

Modes are stored in `GameState::colony_automation: BTreeMap<ColonyId,
ColonyAutomation>`. Absent entries are implicitly `Manual`.

## Automation rules

Automation runs at the start of each colony's production step in
`process_end_turn`, before the queue is processed, so a freshly-queued item can
begin building the same turn.

Preconditions for a colony to be automated this turn:

1. Colony is owned by the player empire.
2. `colony_automation_mode(colony) == SectorGuided`.
3. `colony.build_queue` is **empty**.

If any precondition fails, the colony is left untouched. In particular, a
colony with an explicit player queue is **never** overridden.

When automated, the engine selects exactly **one** item via a deterministic
pick keyed by the sector directive. Selection only ever queues buildings the
colony lacks and only when a surface/orbital slot (and any required tech) is
available. Each directive falls through to a sensible generic order
(Fabrication Yard → Science Nexus → Aquaculture Bay) so automation keeps doing
something useful once its preferred building exists.

Pick priority by directive (first satisfiable option wins):

- **Industrial / Economic**: Fabrication Yard → (Economic: Supply Hub) → generic.
- **Research**: Science Nexus → generic.
- **Agricultural**: Aquaculture Bay → generic.
- **Stabilization**: Aquaculture Bay (food/housing relief) → generic.
- **Military**: Shipyard (needs Orbital Engineering + orbital slot) → generic.
- **Balanced**: generic order only.

If no slot is free for any building, automation queues nothing that turn (no
event). When it does queue, it emits `Event::ColonyAutomationQueued { colony,
item, directive }`.

## Determinism

- Colonies are processed in `sorted_colony_ids` order (already the case).
- The pick is a pure function of `GameState` — no RNG, no wall-clock.
- Same seed + same commands ⇒ same automation choices ⇒ stable replay and
  fingerprint.

## Commands and events

New commands:

- `Command::SetSectorDirective { sector, directive }`
- `Command::SetColonyAutomation { colony, automation }`

New events:

- `Event::SectorDirectiveSet { sector, directive }`
- `Event::ColonyAutomationModeSet { colony, automation }`
- `Event::ColonyAutomationQueued { colony, item, directive }`

Validation mirrors `SetColonyRole`: the targeted colony/sector must exist, and
colony ownership must be the player empire; failures surface as `Event::error`.

## Persistence

`sector_directives` and `colony_automation` are added to `GameState` with
`#[serde(default)]`, so existing saves load unchanged (empty maps ⇒ all
`Balanced` / `Manual`). No save-version migration is required.

## TUI

- **Sector Governance** screen (`Screen::SectorGovernance`, opened globally with
  `G`): lists sectors with name, member colonies, directive, automation summary,
  warnings, and an aggregate output summary; `D` cycles the selected sector's
  directive and `A` toggles sector-guided automation across the sector's
  colonies. Keyboard-first, resize-safe, `?` help.
- **Empire Overview**: a Governance line in the strategic dashboard showing the
  colonized-sector count and automated-colony count, pointing to `G` to manage.
- **Colony** screen: a per-colony automation toggle bound to `A`, shown as an
  Automation status line alongside the role.
- **Turn Report**: an "automation queued N" count of automation actions taken
  this turn.
- **Help overlay**: updated with the Sector Governance screen and the new
  `G`/`D`/`A` keys.

## Limitations

- Automation only queues buildings (and Military Shipyards); it never
  auto-builds warships, colony ships, or scouts.
- One item per colony per turn; it does not plan multi-item queues.
- Economic directive has no dedicated economic building, so it proxies through
  industry (Fabrication Yard) and logistics (Supply Hub).
