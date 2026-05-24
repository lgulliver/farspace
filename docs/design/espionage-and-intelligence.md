# Espionage and Intelligence Lite v1

Espionage and Intelligence Lite v1 adds deterministic rival-knowledge progression without spy units, random mission chains, or per-agent micromanagement.

## Goals

- make diplomacy, war, and scouting carry information value
- keep intel deterministic and explainable
- reveal rival data in layers instead of all-at-once omniscience
- leave hooks for later sabotage and research theft slices

## Intel levels

Each foreign empire resolves to one of five player-facing intel levels:

- **Unknown** — no contact, no verified rival detail
- **Contacted** — diplomatic stance known, richer data still hidden
- **Basic** — colony count and coarse fleet strength visible
- **Informed** — tech tier and economy snapshot visible, fleet detail improves
- **Deep** — strategic-resource access visible

## Passive intel gain

Passive intel progresses every end turn for contacted empires.

Deterministic sources:

- contact itself
- nearby fleet or colony presence
- survey / sensor technology
- active treaties

No wall-clock logic. No global RNG. Stable empire ordering preserved with ordered collections.

## Active intel action

v1 adds one direct action:

- **Gather Intelligence** — deterministic, immediate point gain, once per target per turn

Future hooks are surfaced as placeholders only:

- **Sabotage Production**
- **Steal Research**

They intentionally do not resolve real effects in Lite v1.

## Visibility rules

Intel level controls rival visibility in diplomacy, dispatch, logs, and system views:

- colony count: Basic+
- fleet strength: Basic+
- tech level: Informed+
- economy summary: Informed+
- diplomatic stance: Contacted+
- strategic resources: Deep

Foreign colony and fleet panels avoid leaking unrevealed values before the required intel level is reached.

## UI surfaces

- Diplomacy screen shows intel level and gated empire detail
- System screen hides foreign colony and fleet detail until intel improves
- Turn Report counts intel gains
- Help overlay documents intel controls
- Galactic Dispatch redacts rival colonization / research items until the required intel level is reached

## Advisor

Advisor now warns when contacted empires remain below informed intel and points player toward the diplomacy screen.

## Save/load

- `GameState.empire_intel` persists current intel progress
- save schema bumped to v38
- v37 → v38 seeds contacted empires with baseline intel on load

## Scope limits

Not included in v1:

- spy agents
- mission chains
- counterintelligence systems
- assassinations, coups, or procedural espionage event spam
- random mission outcomes
