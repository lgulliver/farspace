# Fleet Roles and Formations v1

Fleet Roles and Formations v1 adds strategic posture to fleets without tactical combat.

## Goals

- Give fleets strategic identity beyond raw strength.
- Make composition and doctrine visible in AI and UI.
- Keep combat auto-resolve deterministic, fast, and inspectable.

## Fleet Roles

Roles are authoritative per fleet and persisted in save data.

- Exploration Fleet
- Survey Group
- Colony Escort
- Patrol Fleet
- Strike Fleet
- Defense Fleet
- Invasion Fleet
- Blockade Fleet
- Rapid Response Fleet
- Trade Protection Fleet

Role defaults come from `FleetKind`. AI can reassign roles by doctrine and war state.

Role effects influence:

- attack/defense posture
- retreat thresholds
- movement profile (via mobility summary)
- invasion escort value
- blockade suitability

## Fleet Formations

Formations are abstract strategic stances and persisted per fleet.

- Balanced
- Aggressive
- Defensive
- Fast Attack
- Artillery
- Escort Screen

Formation effects influence:

- attack/defense multipliers in auto-resolve
- retreat behavior
- mobility summary
- artillery-vs-defensive interaction
- escort protection behavior

## Fleet Evaluation Summary

Each fleet derives deterministic summary values:

- offensive
- defensive
- invasion capability
- survey capability
- mobility
- blockade strength
- escort quality

Summary uses:

- fleet kind and ship count
- strength/integrity
- custom design derived stats (if present)
- role modifiers
- formation modifiers
- doctrine modifiers from empire identity

## Combat Integration

Auto-resolve now uses role/formation/doctrine posture with deterministic calculations:

- engagement start event emitted before each clash
- role/formation attack and defense modifiers applied
- artillery gets practical edge vs defensive/escort posture
- exploration/survey fleets try to avoid overwhelming enemies
- retreat triggers when integrity crosses role/formation/doctrine threshold and fallback route exists

No tactical layer added. No positioning map, no subsystem targeting, no real-time controls.

## Strategic Behavior Integration

- Movement timing now scales with derived mobility.
- Invasions gain strength from escort quality at target system.
- Blockade derivation prefers fleets with blockade-capable posture.
- Blockade establishment emits both colony event and fleet establishment event.

## AI Doctrine Integration

AI now deterministically assigns role + formation per fleet each turn based on:

- fleet kind
- doctrine weights
- war status
- target context (blockade intent)

Examples:

- Concord-like explorer/merchant profiles trend toward defensive, escort, and rapid-response scouting.
- Dominion-like imperial/militarist profiles trend toward strike, invasion, and blockade posture.

AI scout target selection also avoids high-threat hostile systems for fragile scouting groups.

## TUI Integration

System fleet list now shows:

- fleet role
- formation
- doctrine summary marker
- composition
- offensive/defensive/invasion/mobility summary

System screen actions:

- `f` cycle focused fleet
- `R` assign next role
- `F` assign next formation

Help/footer bindings updated accordingly.

## Save/Load

Save schema bumped to v31.

Persisted:

- `fleet_roles`
- `fleet_formations`
- `fleet_names`

Migration v30 → v31 is passthrough with serde defaults.

## Future Hooks

- richer multi-fleet coordination
- doctrine-specific retreat corridors
- blockade pressure escalation
- optional deeper fleet command screen

## Intentional Limitations

- no tactical combat
- no fuel, morale, leaders, admirals, or XP systems
- no manual formation map editing
- no real-time combat controls
