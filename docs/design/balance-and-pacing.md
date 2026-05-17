# Balance and Pacing — v1

This document describes the pacing goals, key formulas, scaling assumptions, intended game
phases, AI balance philosophy, and known limitations for FARSPACE's first balance pass.

---

## Pacing Goals

### Early Game (Turns 1–15)

- Exploration and survey should matter immediately.
- First research unlock should arrive within **10–15 turns** on a standard start.
- First scout returns survey data within 5–10 turns.
- Colony decisions (role, build priority) should feel meaningful from turn 1.
- Players should not become economically trapped under normal play.
- First diplomatic contacts occur naturally (AI explores toward player territory).
- Military conflict should not occur before turn 20 (the AI combat-fleet dispatch
  is gated behind `turn >= 20`).

### Mid Game (Turns 15–80)

- First colonisation wave completes (Survey Drones + Habitat Seeding + Colony Ship built).
- Competing borders and diplomacy emerge by turn 40–60.
- Shipyards and fleet composition begin to matter.
- AI empires diverge strategically by doctrine.
- Multiple viable strategic paths are open:
  - Military (Dominion)
  - Economic (Prosperity)
  - Scientific (Ascendancy)
  - Expansionist (Discovery)
  - Diplomatic (Unity)

### Late Game (Turns 80+)

- Victory pressure becomes visible (at least one path reaches ≥50% progress).
- Major wars and economic advantages shape the map.
- Advanced techs (T4–T6) feel impactful.
- Economy scales without exponential collapse.

---

## Major Formulas

### Colony Production (Industry)

```
industry = population + (FabricationYard_count × 2) + stability_modifier + role_modifier
stability_modifier = (stability − 100) / 10      # 0 at neutral-100; negative below 100
```

**v1 fix**: The build-queue production pool now uses `colony_yield.industry` (scaled with
population and buildings) instead of the stale `colony.production` field (which was
initialised to 10 and never updated). This makes production speed scale with colony growth.

```
effective_production = max(colony_yield.industry, colony.production)  // legacy floor
production_pool = accumulated_production + effective_production + ship_role_bonus
```

### Colony Yields per Turn

```
credits  = (industry × prod_pct / 100) + role_flat + tech_bonus_per_colony
science  = (industry × research_pct / 100) + ScienceNexus_bonus + planet_bonus + role_flat + tech_bonus
food     = population + (AquacultureBay_count × population) + planet_bonus + role_flat
food_consumed = population
```

Starting colony (pop=10, no buildings, 50/50 split):

| Resource | Per-Turn |
|----------|----------|
| Production | 10 |
| Credits | 5 |
| Science | 5 |

### Research Pacing

Science per turn scales with `colony.industry × research_pct / 100`. With a default 50/50
focus and population 10:

| Tech | Cost | Turns to complete (5 sci/turn) |
|------|------|---------------------------------|
| Void Propulsion (T1) | 40 | 8 |
| Neutrino Sensors (T1) | 50 | 10 |
| Survey Drones (T2) | 75 | 15 |
| Habitat Seeding (T1) | 65 | 13 |
| Colonial Logistics (T2) | 120 | 24 |
| Orbital Engineering (T2) | 150 | 30 |

Tech costs were reduced **15–25% for T1** and **~20% for critical T2 gates** (Survey Drones,
Drift Mapping) to ensure the first research rewards arrive within the target window.

### Population Growth

```
growth_tick = every POP_GROWTH_PERIOD_TURNS turns where (turn + colony_id) % period == 0
conditions  = not blockaded
            AND stability >= MIN_STABILITY_FOR_POP_GROWTH
            AND empire food >= 0
            AND housing_deficit == 0
            AND food >= food_consumed
```

| Constant | v0 | v1 | Reason |
|----------|----|----|--------|
| `POP_GROWTH_PERIOD_TURNS` | 12 | 10 | Faster early growth |
| `MIN_STABILITY_FOR_POP_GROWTH` | 90 | 80 | More forgiving threshold |

### Stability Penalties (per turn)

| Condition | Penalty |
|-----------|---------|
| Colony isolated (not connected) | −5 |
| Colony blockaded | **−8** (was −5) |
| Housing deficit | −`min(deficit, 10)` |
| Unemployed population | −`min(unemployed, 5)` |
| Isolated + food deficit | −`min(deficit, 5)` |

Blockade penalty was raised from 5 → 8 so blockades require a strategic response within
roughly 12 turns before severe destabilisation.

### Isolation / Blockade Yield Penalty

Isolated and blockaded colonies both receive:
```
credits' = credits × 50 / 100
science' = science × 50 / 100
food'    = 0   (no food export while cut off)
```

### Fleet Maintenance

```
maintenance_per_fleet = kind.maintenance_cost() + empire_modifier
total_maintenance = Σ max(0, maintenance_per_fleet) for all empire fleets
```

Fleet maintenance is deducted from empire credits each turn after colony production.
A soft-cap constant `FLEET_MAINTENANCE_CREDIT_CAP_PER_EMPIRE = 50` is defined in
`balance.rs` for future use (doubled upkeep above the cap to deter fleet spam).

### Invasion

```
invasion_strength = troop_transports × (TROOP_TRANSPORT_INVASION_STRENGTH + empire_bonus)
                  = troop_transports × (12 + empire_bonus)
```

| Constant | v0 | v1 | Reason |
|----------|----|----|--------|
| `CAPTURED_UNREST_STABILITY` | 40 | 45 | Captured worlds slightly less crippled |

### Fleet Travel

```
travel_turns = max(1, ceil(distance / FLEET_TRAVEL_SPEED))
             = max(1, ceil(distance / 500.0))

lane_turns   = max(1, ceil(base_turns / HYPERSPACE_TRAVEL_DIVISOR))
             = max(1, ceil(base_turns / 2))
```

Stars are generated in `−500..=500` on each axis; max distance ≈ 1414 units:

| Distance | Base turns | Lane turns |
|----------|-----------|------------|
| ≤ 500 | 1 | 1 |
| ≤ 1000 | 2 | 1 |
| > 1000 | 3 | 2 |

---

## Scaling Assumptions

1. **Production scales with population.** After the v1 fix, a colony at pop 20 produces
   roughly twice the production output of a pop-10 colony (before buildings).

2. **FabricationYard adds +2 industry.** Three yards on a pop-10 colony raise industry from
   10 to 16 (+60%), making early building investment highly rewarding.

3. **Science/credits scale linearly** with industry until late-game tech bonuses compound.
   No exponential growth is intended; per-colony tech yield bonuses are small flat amounts.

4. **Fleet maintenance acts as a soft fleet-count cap.** Each fleet costs 1–4 credits/turn;
   an empire earning 15–20 credits/turn can support roughly 4–10 fleets before maintenance
   becomes strategically significant.

5. **Victory progress scales with empire breadth.** Dominion requires controlling 60% of
   colonised systems; Discovery requires 80% of stars explored + 70% of planets surveyed.
   Neither is achievable purely by turtling.

---

## Intended Game Phases

| Phase | Turns | Key events |
|-------|-------|-----------|
| Exploration | 1–15 | First tech, scouts explore nearby stars |
| Early Expansion | 15–40 | Survey Drones → coloniser built → first new colony |
| Border Contact | 40–60 | AI contact, diplomacy established |
| Economic Divergence | 60–80 | Shipyards built, fleet doctrine matters |
| Late-Game Pressure | 80+ | War, victory paths visible, advanced techs |

---

## AI Balance Philosophy

### Doctrine-Driven Priorities

AI empires choose research and builds based on weighted doctrine axes:

| Doctrine | Research bias | Build bias |
|----------|--------------|-----------|
| Explorer | Exploration, Survey | Scout, Science Ship |
| Expansionist | Colonisation, Biology | Colony Ship, Colony Ark |
| Industrialist | Engineering | Fabrication Yard, Shipyard |
| Militarist | Military | Combat ships |
| Imperial | Military + Society | Combat ships, troop transports |
| Merchant | Economy, Logistics | Fabrication Yard |
| Technologist | Engineering, Society | Science Nexus |
| Biologist | Biology | Aquaculture Bay |
| Isolationist | Society, Stability | Stability buildings |

### AI Combat (v1)

AI empires now dispatch idle combat fleets toward enemy colonies when at war:

- Activates only at `turn >= 20` (prevents immediate early aggression).
- Only combat fleet kinds (EscortFrigate, MissileFrigate, Destroyer, PatrolCorvette) are
  dispatched; scouts and colonisers are not used for war.
- Target is the nearest player colony by squared distance.
- All dispatch decisions are deterministic: fleets sorted by `FleetId`, targets sorted by
  `StarId`.

### AI Limitations in v1

- The AI does not declare war proactively (only the player can declare war).
- The AI does not evaluate peace opportunities or sue for peace.
- The AI does not use blockades strategically (fleet movement to blockade positions is
  reactive, not planned).
- The AI does not coordinate multiple fleets for combined arms.
- Research plan length is short (2–4 techs); it does not plan long tech chains.

---

## Victory Conditions (Default)

| Path | Conditions |
|------|------------|
| Dominion | Control ≥60% of all colonised systems |
| Ascendancy | Complete ≥4 of 6 designated victory technologies |
| Prosperity | Pop ≥40, Credits ≥300, ≥4 connected colonies, avg stability ≥95, food surplus ≥0 |
| Discovery | 80% stars explored, 70% planets surveyed, Hyperspace Cartography + Sector Cartography |
| Unity | Contact ≥2 empires, ≥2 non-war relations, ≥3 connected colonies |

All paths are enabled by default except Unity (which requires non-war diplomatic investment).

---

## Known Balance Risks and Future Work

### Short-term

- **Prosperity victory** may be easier than intended because the stability threshold (≥95)
  can be maintained by a turtling player who avoids expansion. Consider adding a minimum
  empire-size requirement.
- **Discovery victory** requires 80% star exploration; on small maps this is achievable
  early, on large maps it may be too hard. Consider making the threshold map-size-aware.
- **Fleet maintenance** soft cap (`FLEET_MAINTENANCE_CREDIT_CAP_PER_EMPIRE = 50`) is
  defined but **not yet wired into the engine**. Without it, wealthy empires can spam cheap
  scouts to maintain map presence at low cost.
- **AI never declares war**: players who prefer defence can survive indefinitely without
  meaningful military threat beyond blocking AI expansion.

### Mid-term

- Colony growth gating on `empire.food >= 0` creates a hard cliff: any food deficit halts
  growth empire-wide regardless of which colony is deficient. Consider per-colony food
  accounting.
- Pop growth at period 10 with pop=10 means a colony reaches pop=15 after ~50 turns if
  conditions are met continuously. This may still feel slow for wide-expansion strategies.
- Research scaling is flat (no diminishing returns); a player or AI that focuses heavily on
  science buildings can compress mid-game research significantly.

### Long-term (requires new features)

- **Megastructures** (future): tech hooks defined (`future_hook: true`); balance TBD.
- **Crises** (future): no crisis system yet; late-game may feel unchallenging once dominant.
- **Advanced diplomacy** (future): treaties, trade routes, and alliance mechanics would give
  Unity and Prosperity paths more strategic depth.
- **Custom ship design** (future): current ship archetypes provide basic fleet diversity but
  do not allow mid-game design adaptation.
- **Espionage** (future): `TechTag::EspionageFuture` is defined; no implementation yet.

---

## Centralized Balance Constants

All numeric tuning values live in `crates/game_core/src/balance.rs`. See that file for
inline documentation of each constant's purpose and intended effect.

Key constants:

```rust
POP_GROWTH_PERIOD_TURNS             = 10   // turns between growth ticks
MIN_STABILITY_FOR_POP_GROWTH        = 80   // stability floor for growth
BLOCKADED_STABILITY_PENALTY         = 8    // per-turn stability loss under blockade
ISOLATED_YIELD_PERCENT              = 50   // % yield retained while isolated
TROOP_TRANSPORT_INVASION_STRENGTH   = 12   // strength per troop transport
CAPTURED_UNREST_STABILITY           = 45   // stability after colony capture
BORDER_TENSION_DISTANCE_SQ          = 40_000 // ~200 map units: routine tension
SEVERE_BORDER_TENSION_DISTANCE_SQ   = 12_000 // ~110 map units: severe tension
FLEET_TRAVEL_SPEED                  = 500.0  // map units per turn
HYPERSPACE_TRAVEL_DIVISOR           = 2      // lane speed multiplier (÷2)
FLEET_MAINTENANCE_CREDIT_CAP_PER_EMPIRE = 50 // reserved, not yet active
```
