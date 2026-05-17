# Pop & Jobs Lite v1

## Scope

Pop & Jobs Lite v1 adds deterministic automatic workforce assignment without manual per-pop micromanagement.

Included in this slice:
- Discrete population as Pops (existing `colony.population`)
- Deterministic job slot generation from buildings and colony baseline
- Deterministic automatic assignment based on colony role + shortage pressure
- Housing pressure (unhoused Pops) and unemployment pressure surfaced in economy and UI
- Colony-level workforce and yield breakdowns for explainability

Deferred (explicitly out of scope):
- Manual job locking/dragging
- Migration simulation
- Species and genetics
- Political factions and ethics
- Promotion/demotion strata systems
- Advanced consumer/trade-goods economy

## Job Types

Lite v1 job taxonomy:
- Farmer
- Miner
- Technician
- Researcher
- Administrator
- Security
- Worker
- Unemployed

## Assignment Rules (Deterministic)

Assignment is automatic and deterministic:
1. Compute total slots by job type from colony baseline and buildings.
2. Build deterministic priority list from:
   - colony role
   - food shortage pressure
   - stability pressure
3. Fill slots in priority order with available Pops.
4. Pops beyond housing capacity are marked unhoused (housing deficit).
5. Remaining housed Pops without a filled slot become unemployed.

Tie-break behavior is stable and deterministic by fixed enum order.

## Buildings and Jobs

Current v1 mapping:
- Aquaculture Bay: expands housing and farmer capacity
- Fabrication Yard: adds miner/technician capacity
- Science Nexus: adds researcher capacity

Buildings no longer need to be understood as only flat yield bonuses; they primarily shape workforce composition.

## Housing, Food, Stability

- Pops consume food each turn.
- Pops require housing capacity.
- Unhoused Pops count toward housing deficit pressure, not unemployment.
- Housing deficit and unemployment create stability pressure.
- Isolated/blockaded colonies are more vulnerable to food pressure.
- Stability still modifies effective economic output.

## Yield Composition

Colony yield is decomposed into:
- job-derived output
- focus conversion output
- planet modifiers
- special/resource modifiers
- stability modifier

This breakdown is exposed to TUI views for explainability.

## AI Assumptions

AI remains deterministic and explainable.

In this slice AI economy planning:
- prioritizes food/housing relief when deficits appear
- avoids persistent unemployment where practical
- continues role-aware deterministic build decisions

No advanced governor subsystem is introduced in v1.
