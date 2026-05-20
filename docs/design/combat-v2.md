# Combat v2 — Tactical Auto-Resolve Depth

Combat v2 deepens strategic auto-resolve without adding tactical maps or realtime controls.

## Combat phases

Each fleet engagement resolves in deterministic ordered phases:

1. Detection
2. Positioning
3. Opening Volley
4. Main Engagement
5. Attrition
6. Retreat/Collapse
7. Resolution

Phase summaries are persisted in structured `BattleReport` records.

## Formation effects

- **Balanced**: neutral baseline.
- **Aggressive**: higher pressure, lower defense, lower retreat threshold.
- **Defensive**: stronger survivability, slower kills, higher retreat threshold.
- **Fast Attack**: initiative/mobility edge, lower durability.
- **Artillery**: stronger opening pressure, weak in prolonged exchanges.
- **Escort Screen**: stronger support protection, lower offense.

## Doctrine effects

Doctrine influences combat via deterministic fleet evaluation, role assignment, and posture:

- Concord-like profiles trend survivability/organized withdrawal.
- Dominion-like profiles trend offensive pressure/escalation posture.
- Merchant/isolationist profiles favor protection and controlled engagements.
- Explorer profiles bias mobility and avoidance when outmatched.

## Role integration

Fleet role contributes to engagement profile and opening behavior:

- escorts and defense roles increase protection posture.
- strike/blockade roles increase opening and sustained pressure.
- exploration/survey roles prefer disengagement when outmatched.
- invasion/colony-escort roles preserve force continuity via higher retreat bias.

## Retreat logic

Retreat remains deterministic and can fail when no valid fallback colony exists.
Retreat decisions use role, formation, doctrine-derived profile, and current integrity.
Not all engagements end in annihilation; surviving fleets keep post-battle integrity.

## Invasion and blockade

Invasion and blockade remain strategic systems with deterministic outcomes.
Combat v2 preserves existing requirements:

- invasion requires troop transport capability.
- escort quality improves invasion strength.
- orbital defenses reduce invasion success chance by strength comparison.
- blockade requires hostile fleet presence and no defending idle friendly fleet.

## Battle reports

`BattleReport` records include:

- participants (fleets/empires)
- fleet roles, formations, doctrine summaries
- per-phase pressure summary and turning-point notes
- integrity start/end, destruction/retreat flags
- system outcome summary

Reports are stored in deterministic bounded history (`GameState.battle_reports`).

## Galactic Dispatch integration

Major combats are highlighted with higher dispatch severity when destruction or high combined strength is detected, while still respecting fog-of-war and unknown-empire redaction rules.

## TUI integration

- Global `B` opens Battle Reports modal.
- `Enter` toggles inspect mode for selected report.
- `↑/↓` selects report.
- `Esc`/`B` closes modal.
- System view includes quick battle-report hint in fleet controls.

## Save/load + replay determinism

- Battle reports are persisted in save data (`next_battle_report_id`, `battle_reports`).
- Save schema version bumped to 33.
- Replay stability preserved by deterministic phase ordering, deterministic notes, and stable report insertion order.

## Intentional limitations

Still out of scope:

- tactical maps
- manual ship movement in battle
- projectile simulation
- subsystem targeting
- realtime combat controls
- ammo/fuel/morale/admiral layers
