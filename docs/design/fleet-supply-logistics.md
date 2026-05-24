# Fleet Supply & Logistics

Fleet Supply & Logistics v1 adds deterministic strategic reach pressure for combat fleets and troop transports without introducing tactical fuel accounting.

## Goals

- make forward fleets weaker when they outrun empire support
- keep outcomes deterministic and easy to read in TUI
- preserve simple command flow: no new player command required

## Supply States

Each fleet resolves into one of three states:

- **Supplied** — operating inside full logistics reach
- **Extended** — beyond ideal reach but still partially supported
- **Out of Supply** — unsupported deep strike posture

Current state is derived from `GameState`, not player input.

## What Projects Supply

Supply originates from owned colonies and expands based on:

- colony connectivity
- shipyards
- supply hubs
- direct usable hyperspace lanes
- empire logistics technology bonuses
- blockade state

Blocked colonies do not project fleet supply. Isolated colonies project weaker reach unless they have orbital support.

## Penalties

Fleet supply penalties apply to combat fleets and troop transports:

| State | Attack | Defense | Travel Time | Invasion |
|---|---:|---:|---:|---:|
| Supplied | 100% | 100% | 100% | 100% |
| Extended | 90% | 95% | 125% | 75% |
| Out of Supply | 75% | 80% | 160% | 0% |

Out-of-supply troop transports cannot invade.

## UI Surfaces

Fleet supply is shown in:

- **Sector Map** system rows as projected supply for movement planning
- **System View** fleet roster and focused fleet status
- **Battle Reports** for both sides
- **Turn/log messages** when fleets arrive and in end-turn fleet supply summary

Labels stay literal for readability:

- `Supplied`
- `Extended`
- `Out of Supply`

## Determinism Notes

- no wall-clock or OS randomness
- derived from seeded map/state only
- ordered collections or stable iteration required for any aggregate recalculation
- save migration re-derives fleet supply rather than trusting stale persisted values

## Scope Limits

v1 does **not** add:

- per-ship fuel tracking
- manual supply convoy commands
- local stockpile simulation
- tactical combat fuel/ammo phases
