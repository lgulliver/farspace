# Planet Specials and Anomalies v1

Planet Specials and Anomalies v1 expands exploration with rare, deterministic discoveries that make worlds and frontier systems feel distinct without turning each turn into narrative event spam.

## Planet special categories

- **Resource** — direct extraction or economy identity
- **Scientific** — research-rich sites and archives
- **Biological** — food, ecology, and life-science worlds
- **Industrial** — construction, logistics, and fabrication assets
- **Environmental** — unusual terrain and physics signatures
- **Precursor** — ancient signal, beacon, and relic sites
- **Hazard** — valuable but risky planetary conditions
- **Strategic** — worlds worth contesting or defending
- **Cultural** — salvage fields, memorial belts, historic remains

## Anomaly categories

- **Stellar**
- **Precursor**
- **Biological**
- **Temporal**
- **Gravitational**
- **Military**
- **Archaeological**
- **Exotic Physics**

Anomalies stay rarer than specials. They represent persistent orbital or planetary mysteries rather than disposable popup events.

## Placement philosophy

Generation is deterministic from galaxy seed plus local context:

- planet class
- star spectral class
- sector identity
- frontier distance from galactic center
- hazardous signature
- precursor signature
- nebula-band signature

Frontier and dangerous worlds get higher odds for rare findings. Precursor signatures spike precursor specials and anomalies. Not every planet gets a discovery; worlds that do should feel memorable.

## Rarity philosophy

- **Common** — useful, readable, appears regularly
- **Uncommon** — notable, shapes colonization choice
- **Rare** — strategic discovery, worth contesting
- **Legendary** — genuine hotspot, low-frequency map-defining find

Special weighting strongly favors Common and Uncommon. Anomalies start at Uncommon and above so they stay rare and important.

## Discovery flow

1. System explored
2. Science ship surveys world
3. Survey reveals special/resource data
4. Advanced science techs reveal deeper anomaly intel
5. Discovery emits concise report, may surface in Dispatch, and becomes persistent world modifier

Visibility rules:

- unsurveyed worlds reveal nothing
- advanced anomalies require stronger survey/sensor techs
- Dispatch only reports discoveries already visible to player

## Strategic effects

v1 effects stay low-noise and mostly persistent:

- colony yield bonuses or penalties
- research-rich precursor worlds
- frontier chokepoint and salvage value
- hazard-for-reward development choices
- stronger colonization and military target evaluation

Effects are intentionally compact. No sprawling modifier stacks, no per-turn procedural event chains.

## AI valuation rules

AI remains deterministic and does not read hidden discovery state.

- **Explorer** values frontier reach, temporal/gravitic anomalies, survey-rich targets
- **Technologist** values scientific, precursor, archaeological, and exotic-physics finds
- **Militarist** values hazardous and military discoveries plus contested strategic worlds
- **Industrialist** values industrial/resource-heavy specials
- **Expansionist** values biological and habitable discovery clusters

Known discoveries raise colonization and wartime target scores. Scout prospecting still uses visible heuristics rather than hidden anomaly cheating.

## Precursor hooks

v1 stores structured IDs, categories, rarity, tags, and optional `future_hook` metadata for later expansion:

- archaeology projects
- precursor translation paths
- signal chains
- defense-grid restoration
- vault decryption
- salvage operations

## Future archaeology hooks

Not in v1:

- branching narrative chains
- dialogue trees
- click-through archaeology minigames
- realtime anomaly handling
- full precursor campaign scripting

v1 only lays deterministic metadata and persistent modifiers for those later systems.

## Intentional limitations

- no giant narrative event chains
- no random click interruptions
- no empire-specific hidden cheating for AI
- no full anomaly resolution gameplay loop
- no per-empire survey intel model yet

v1 focuses on replayable strategic identity, terminal readability, deterministic generation, and migration-safe persistence.
