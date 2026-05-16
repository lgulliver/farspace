# FARSPACE Next Slices

Immediate implementation queue for focused agent sessions.

## 1) Large-Scale Technology Tree v2 (closure pass)

- **Goal:** close remaining integration gaps and confirm all intended v2 unlocks/hooks are wired.
- **Why now:** research is central dependency for economy, fleets, diplomacy, and late-game systems.
- **Dependencies:** current tech records, unlock processing, research UI filters.
- **Risk level:** Medium
- **Rough acceptance criteria:** tech data complete for planned v2 scope; unlock-bearing techs verified in simulation and UI.

## 2) Research Queue v1

- **Goal:** finalize queue UX behavior and edge-case handling as stable player workflow.
- **Why now:** reduces planning friction and supports multi-turn strategic intent.
- **Dependencies:** slice 1 validation.
- **Risk level:** Medium
- **Rough acceptance criteria:** queue operations are deterministic, discoverable in UI, and regression tested.

## 3) Pop & Jobs Lite v1

- **Goal:** introduce explicit pop/job allocation model beyond current population-only yields.
- **Why now:** economy depth and colony specialization need explicit labor model.
- **Dependencies:** stable economy and research unlock foundations.
- **Risk level:** High
- **Rough acceptance criteria:** pop/job states affect yields predictably; positive/negative tests cover assignment and deficits.

## 4) AI Doctrine v1

- **Goal:** add higher-level doctrine profiles to shape AI strategic behavior.
- **Why now:** richer economy/research/fleet systems need stronger AI direction.
- **Dependencies:** slices 1-3.
- **Risk level:** High
- **Rough acceptance criteria:** doctrine selection impacts build/research/fleet priorities deterministically and is test-covered.

## 5) Victory Conditions v1

- **Goal:** implement first shippable end-state rules.
- **Why now:** needed for v1 completion criteria and campaign closure.
- **Dependencies:** stable diplomacy/combat/economy signals.
- **Risk level:** Medium
- **Rough acceptance criteria:** game can end with clear winner and visible rule explanation.

## 6) Determinism Audit & Replay Log v1

- **Goal:** provide reproducibility audit trail and replay validation path.
- **Why now:** safeguards future complexity and release hardening.
- **Dependencies:** slices 1-5.
- **Risk level:** Medium
- **Rough acceptance criteria:** same seed+commands replay to same outcome; tooling exposes mismatches clearly.

## 7) Ship Archetype Refinement (or limited designer)

- **Goal:** improve fleet composition depth while keeping scope controlled.
- **Why now:** combat and production loops need stronger strategic differentiation.
- **Dependencies:** slices 3-6.
- **Risk level:** Medium
- **Rough acceptance criteria:** player-facing ship choices have distinct roles/costs and deterministic outcomes.

## 8) Advanced Diplomacy / Treaties v1

- **Goal:** move from relation bands to actionable treaty mechanics.
- **Why now:** supports non-combat strategic paths and faction identity depth.
- **Dependencies:** slices 4-7.
- **Risk level:** High
- **Rough acceptance criteria:** treaty states alter behavior/economy/combat eligibility and persist through save/load.

## 9) Terraforming & Planet Development v1

- **Goal:** add late-midgame planetary improvement progression.
- **Why now:** extends colony progression and map value evolution.
- **Dependencies:** slices 3, 4, 8.
- **Risk level:** High
- **Rough acceptance criteria:** terraforming actions have clear costs, deterministic effects, and migration-safe persistence.

## 10) Scenario / Modding Framework v1

- **Goal:** expose controlled scenario data-driven setup pipeline.
- **Why now:** unlocks replayable content and long-term extensibility.
- **Dependencies:** slices 1-9 stability.
- **Risk level:** High
- **Rough acceptance criteria:** scenario definitions load reliably, validate deterministically, and stay boundary-safe.
