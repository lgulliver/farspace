# FARSPACE AI Doctrines v1

AI Doctrines v1 adds deterministic strategic weighting for empire AI behavior.

## Doctrine definitions

- **Explorer**: scouting, survey intel, sensors, hyperspace reach.
- **Expansionist**: colony tempo, growth support, frontier settlement.
- **Industrialist**: production capacity, shipyards, infrastructure depth.
- **Militarist**: warship readiness, defenses, combat pressure.
- **Technologist**: science velocity, advanced capability timing.
- **Merchant**: credits, trade throughput, supply/logistics efficiency.
- **Isolationist**: internal stability, defensive posture, low-risk expansion.
- **Imperial**: coercive diplomacy, conquest tools, escalation posture.
- **Biologist**: food, housing, growth, adaptation priorities.

## Faction-to-doctrine mapping

- **Ashveran Compact**: Industrialist + Isolationist
- **Luminal Traverse**: Explorer + Merchant + Expansionist
- **Sylvaran Accord**: Expansionist + Biologist
- **Thalori Exchange**: Merchant + Expansionist
- **Vorath Dominion**: Militarist + Imperial
- **Elarith Confluence**: Industrialist + Technologist + Isolationist
- **Terran Concord**: Explorer + Technologist + Merchant (lower aggression)
- **Terran Dominion**: Imperial + Militarist + Industrialist (higher aggression)

## Decision areas affected

Doctrine weights bias deterministic scoring for:

- research plan and short research queue construction
- colony role and build queue priorities
- crisis reactions (food, housing, stability, blockades, local defense)
- shipyard and ship archetype production choices
- scouting/survey/colonization emphasis
- diplomacy drift and escalation pace

All actions remain rule-valid and use normal command/event paths.

## Deterministic tie-breaking rules

AI decisions use stable ordering and explicit score ladders.

- Candidates are filtered first (valid/unlocked/not completed as applicable).
- Candidates are sorted deterministically with fixed keys (score + stable IDs/order fields).
- No stochastic behavior is used for doctrine choices.
- Same seed + same state + same faction identity yields same AI decisions.

## Current limitations

- Doctrine reasoning is weighted heuristics, not long-horizon planning/search.
- No machine learning or hidden AI bonus resources.
- Diplomacy remains state-band drift, not full treaty negotiation.
- Tactical combat AI remains out of scope.
- Biologist is represented as doctrine axis for growth-oriented factions.

## Future improvements

- richer doctrine-aware threat models for multi-front wars
- deeper doctrine hooks for future population/jobs systems
- broader doctrine-aware victory-path logic when victory framework expands
- optional developer-facing AI reasoning inspection tooling
