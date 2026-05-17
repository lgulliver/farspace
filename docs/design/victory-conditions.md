# FARSPACE Victory Conditions v1

Victory checks run at end of each turn in `game_core`, using only authoritative deterministic state.

## Enabled defaults

- Enabled by default: Dominion, Ascendancy, Prosperity, Discovery
- Disabled by default: Unity
- Configurable in `ScenarioSetup.victory_settings`

## Tie-break order (deterministic)

If multiple paths complete on same turn, winner uses fixed order:

1. Dominion
2. Ascendancy
3. Prosperity
4. Discovery
5. Unity

If multiple empires satisfy same winning path on same turn, lowest `EmpireId` wins.

## Paths

### Dominion

- Goal: territorial supremacy
- v1 checks:
  - controlled colonized-system percentage threshold, or
  - elimination condition (single major empire with colonies remaining) when enabled

### Ascendancy

- Goal: late-game scientific supremacy
- v1 checks:
  - complete configured count of explicitly listed victory tech IDs
  - future-hook techs count only if listed explicitly in condition config

### Prosperity

- Goal: population + economic + internal stability dominance
- v1 checks all configured thresholds:
  - total population
  - credits
  - connected colonies
  - average stability
  - optional food surplus

### Discovery

- Goal: exploration and survey dominance
- v1 checks:
  - explored-systems percentage
  - surveyed-planets percentage
  - optional required exploration tech completion

### Unity (v1 placeholder path)

- Disabled by default
- Optional progress path only
- v1 checks when enabled:
  - contacted empires
  - non-war relations
  - connected colonies
- No federation mechanics in v1

## AI doctrine relationship (lightweight)

AI uses deterministic doctrine-weighted path preference for scoring:

- Militarist / Imperial → Dominion
- Technologist → Ascendancy
- Explorer / Expansionist → Discovery
- Merchant / Industrialist / Biologist → Prosperity
- Isolationist + diplomatic-economy weighting → Unity/Prosperity

No hidden AI bonuses added. AI still uses normal command/event pipeline.

## Save/load

- `VictorySettings` persisted under scenario setup
- `VictoryStatus` persisted in game state
- Migration path bumps schema to v28 with default-backed passthrough

## TUI exposure

- Empire Overview now includes victory panel:
  - enabled state
  - progress %
  - requirements/threshold details
  - leading empire
  - completion/winner summary
- `V` key opens Empire Overview victory section (same destination as `O`)
- End-turn report includes victory milestone and victory completion counters

## Future hooks (out of scope in v1)

- Full diplomacy/federation Unity mechanics
- Megastructure/campaign crisis paths
- Extended score/endgame presentation
