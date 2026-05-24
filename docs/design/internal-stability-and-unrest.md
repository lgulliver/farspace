# Internal Stability and Unrest v1

## Goals

- slow conquest and overexpansion snowball
- make frontier and occupied colonies strategic liabilities
- add internal pressure without deep political simulation
- keep model deterministic and explainable

## Unrest States

Colony order now resolves into one of four deterministic states:

- `Calm`
- `Strained`
- `Unrest`
- `Revolt Risk`

State is derived from a weighted unrest score each turn and cached in `GameState.colony_unrest` for UI/reporting.

## Unrest Contributors

Per-colony contributors:

- food shortage
- housing shortage
- low stability bands
- recent conquest (decays over fixed turn window)
- blockade
- isolated / out-of-supply colony
- empire war exhaustion pressure
- empire overextension pressure (colony count above threshold)

## Empire-Level Pressure

Per-empire unrest pressure is deterministic:

- `war_exhaustion`: active war relation present
- `overextension`: colony count above `OVEREXTENSION_COLONY_THRESHOLD`

These pressures are applied uniformly to owned colonies as unrest contributors.

## Penalties

Unrest states apply deterministic colony penalties:

- reduced production throughput
- reduced credits/science output
- increased colony maintenance
- increased rebellion risk basis points (`colony_rebellion_risk_bp`) as future hook

Note: build throughput is based on colony-local industry (then unrest penalty), while empire/resource flat economic bonuses affect economy outputs only and do not directly accelerate build-pool accumulation.

No rebellion mechanics are executed in v1.

## Conquest Handling

On successful invasion:

- captured colony stability is set to `CAPTURED_UNREST_STABILITY`
- conquest turn is recorded in `colony_recent_conquest_turn`
- colony starts at least in severe unrest pressure path (`Revolt Risk` cache seed)

Recent-conquest pressure decays over a fixed window (`CONQUEST_UNREST_TURNS`).

## Recovery

Unrest is recomputed every turn from current conditions.
If shortages, blockade, war pressure, or overextension ease, colonies naturally recover toward `Calm`.

## Determinism

- no global RNG or wall-clock logic
- sorted colony iteration preserved for stable event order
- unrest transitions emitted via deterministic `ColonyUnrestChanged` events
- save/load persists unrest caches and conquest timestamps

## UI Surfaces

Unrest is surfaced in:

- Colony screen (state, causes, rebellion risk)
- System view colony status labels
- Empire Overview rows and summary (unrest highlight)
- Turn summary report (unrest worsened / revolt-risk counts)
- Galactic Dispatch (major unrest escalation items)
- Help overlay notes

## Scope Limits

Explicitly out of scope in v1:

- political factions
- ideology trees
- rebellions and civil wars
- governors and migration politics
- deep approval simulation
