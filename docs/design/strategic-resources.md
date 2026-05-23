# Strategic Resources v1

Strategic Resources v1 adds rare, deterministic map assets that create pressure to expand, survey, defend chokepoints, trade, and fight over contested systems.

## Resource catalog

Ten resources:

- Quantum Crystals
- Reactive Isotopes
- Dark Matter
- Living Alloy
- Hyperfiber Organics
- Helium-3
- Psionic Spores
- Neutronium Deposits
- Antimatter Residue
- Precursor Datacores

Each resource defines:

- `resource_id`
- `name`
- `description`
- `rarity` (`Common`, `Uncommon`, `Rare`, `Legendary`)
- `category` (`Industrial`, `Energy`, `Military`, `Exotic`, `Biological`, `Precursor`)
- `discovery_requirements`
- `extraction_requirements`
- `tech_requirements`
- `strategic_effects`
- `trade_value`
- `future_hook_megaproject`

## Placement philosophy

Generation is deterministic per galaxy seed and context, with asymmetry by:

- planet class
- star spectral class
- sector identity
- frontier distance from galactic center
- hazardous system signatures
- precursor signatures
- nebula-band signatures

Sectors can roll resource-poor or hotspot behavior. Rare and legendary resources have lower base weight and context-driven spikes.

## Discovery rules

- Resources are hidden until survey completes.
- Discovery is also gated by resource-specific discovery techs.
- UI and Dispatch only surface discovered resources (no hidden-info leaks).

## Extraction and control

Extraction requires:

- colony ownership/control
- required buildings/orbitals (resource-specific)
- required extraction tech (resource-specific)
- connected supply network
- no active blockade (for blockable resources)

`GameState.empire_resource_access` persists deterministic per-empire extracted counts and is recomputed from state each turn.

## Strategic effects (v1)

v1 effects focus on clear, low-noise levers:

- empire-wide colony yield adjustments (industry/credits/science/food)
- limited research acceleration for precursor artifacts
- Helium-3 fleet maintenance relief
- advanced ship component gating by resource access

Effects are intentionally capped and non-explosive.

## Tech integration

Resources use existing tech tree gates for:

- discovery visibility (survey/sensor-tier techs)
- extraction enablement (industry/orbital/precursor lanes)
- advanced ship component requirements

Early game remains playable with baseline designs and no rare resources.

## AI valuation rules

AI uses deterministic resource-weighted scoring for:

- scout prospect targeting
- colonization target ranking
- wartime target selection (resource-rich enemy colony stars)

Doctrine weighting biases priorities:

- Industrialist: industrial assets
- Militarist: military assets
- Technologist: exotic/precursor assets
- Merchant: high trade-value assets
- Expansionist: energy/frontier assets

## Galactic Dispatch integration

Resource discoveries emit dedicated events and can generate Dispatch headlines with rarity-scaled severity:

- Common: Notice
- Uncommon: Notable
- Rare: Urgent
- Legendary: Historic

## UI/TUI integration

System and colony screens now show discovered resources with:

- rarity
- category
- trade value (system detail)
- extraction status (`active` / `offline`)

Diplomacy summary includes known empires’ strategic asset counts when contact exists.

Help overlay includes resource legend notes.

## Save/load and determinism

- Save schema bumped to v34.
- Migration derives `empire_resource_access` deterministically.
- Placement, discovery visibility, extraction access, and AI targeting remain deterministic under fixed seed + command stream.

## Intentional limitations (v1)

Not included in v1:

- manufacturing chains
- dynamic market pricing
- stock-market simulation
- crafting pipelines
- fuel logistics
- tactical harvesting minigames
- high-micromanagement resource spreadsheets

v1 prioritizes strategic clarity over simulation depth.
