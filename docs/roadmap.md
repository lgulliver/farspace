# FARSPACE Full Roadmap

Phases are ordered delivery slices from current alpha to shippable v1 and expansion.

Status labels:

- **Done**
- **In Progress**
- **Planned**

---

## Phase 0 — Foundation

**Goal:** stable workspace, CI gates, deterministic command/event core, save/load base.

**Current status:** **Done**

**Key deliverables:**

- Workspace crate split (`game_core`, `game_tui`, `game_content`, `game_save`, `farspace`)
- CI gates (`fmt`, `clippy`, `test`, coverage)
- Deterministic command/event simulation baseline
- Versioned save/load with migration path

**Dependencies:** none (base layer)

**Risks:** low; drift risk if boundaries are not enforced in future slices

**Acceptance criteria:** CI green, deterministic tests present, save round-trip and migration tests present

---

## Phase 1 — Playable Core Loop

**Goal:** playable early loop through exploration, colonization, economy, research, and turn progression.

**Current status:** **In Progress**

**Key deliverables:**

- Galaxy generation
- Main menu + new game setup
- Galaxy/sector navigation and system selection
- Colony basics and end-turn pipeline
- Event log as turn feedback channel

**Dependencies:** Phase 0 foundation

**Risks:** UX discoverability, imbalance in early pacing

**Acceptance criteria:** player can run full multi-turn loop without debug tools; no deterministic regressions

---

## Phase 2 — Exploration & Spatial Galaxy

**Goal:** deepen map traversal and scouting/survey flow with spatial travel constraints.

**Current status:** **In Progress**

**Key deliverables:**

- Science ships and planet survey flow
- System view and sector navigation polish
- Distance-based travel timings
- Fleet movement visualization in map contexts
- Hyperspace lane generation and lane-aware travel behavior

**Dependencies:** Phase 1 loop stability

**Risks:** readability in dense sectors, pathing edge cases, movement clarity in TUI

**Acceptance criteria:** exploration/survey/travel loops are clear, deterministic, and test-covered

---

## Phase 3 — Colony Economy

**Goal:** evolve colonies from queue-only nodes into strategic economic systems.

**Current status:** **In Progress**

**Key deliverables:**

- Planet class/size/slot effects
- Orbital infrastructure and shipyards
- Population/food/housing/growth fundamentals
- Yield model refinement and maintenance/supply pressure
- Pop/jobs-lite baseline
- Colony roles and role-driven modifiers

**Dependencies:** Phases 1-2

**Risks:** economy snowballing, maintenance deadlocks, UI complexity

**Acceptance criteria:** economy remains legible, deterministic, and recoverable from deficit states

---

## Phase 4 — Research & Progression

**Goal:** robust progression pacing via broad tech tree and queue-driven planning.

**Current status:** **In Progress** (Tech Tree v2 and Research Queue complete)

**Key deliverables:**

- ✅ Large-Scale technology tree
- ✅ Research queue management
- Unlock integration (ships/buildings/capabilities)
- Rare/future-hook technologies with clear signaling
- AI research weighting aligned with faction profiles

**Dependencies:** Phase 3 economy maturity

**Risks:** tech bloat without gameplay payoff, AI mismatch to unlocks

**Acceptance criteria:** research choices are meaningful and unlock effects are mechanically visible

---

## Phase 5 — Fleets, Shipyards & Combat

**Goal:** complete strategic fleet warfare loop in deterministic
auto-resolve model, evolving toward card-driven battle resolution that
surfaces loadout and tech choice in combat outcomes.

**Current status:** **In Progress**

**Key deliverables:**

- Ship production from colony/shipyard systems
- Expanded ship archetype roster
- ✅ Auto-resolve combat outcomes and reporting (Combat v2)
- Blockades and orbital defense interactions
- Invasion and troop transport strategic layer
- 🆕 Card-driven battle resolution (Combat v3) — 5-card hand draft,
  alternating rounds, command-driven card play, AI card selection
- 🆕 BattleScreen TUI overlay with keyboard-first card picker
- 🆕 23-card v1 pool (15 base + 8 faction signatures) with 8-faction
  mapping to 5 play-style buckets

**Dependencies:** Phases 2-4

**Risks:** combat readability, invasion balance, maintenance pressure,
card-pool balance, AI card selection transparency, save migration safety

**Acceptance criteria:** fleet combat/invasion outcomes are deterministic,
explainable, and test-covered; card draft is a pure function of fleet and
empire state; same seed + same commands replay to byte-identical events;
save migration from v36 to v37 is lossless.

---

## Phase 6 — Empires, AI & Diplomacy

**Goal:** richer multi-empire strategy through identity, planning, and diplomacy state.

**Current status:** **In Progress**

**Key deliverables:**

- Empire selection and identity effects
- Faction identity depth (major + future minor faction hooks)
- AI planning and doctrine behavior
- Diplomacy relation bands and war state behavior
- Trade/supply strategic coupling

**Dependencies:** Phases 3-5

**Risks:** AI transparency, diplomacy complexity vs current UI affordances

**Acceptance criteria:** empire identity affects play, AI behavior is coherent, diplomacy state changes are visible and deterministic

---

## Phase 7 — Strategic Management & UX

**Goal:** reduce command friction and improve strategic observability.

**Current status:** **In Progress**

**Key deliverables:**

- Empire overview management workflows
- Turn report/notification quality improvements
- Rally points and fleet order usability
- Command palette polish
- Help/search/filtering improvements across screens

**Dependencies:** Phase 6 state richness

**Risks:** input overload, inconsistent key flows, signal-to-noise in logs

**Acceptance criteria:** common strategic tasks are fast to execute and understandable from UI feedback

---

## Phase 8 — Victory & Campaign Structure

**Goal:** define session completion and post-session outcomes.

**Current status:** **Planned**

**Key deliverables:**

- Victory conditions and trigger rules
- Scenario setup options and presets
- Scoring framework
- Campaign summary outputs
- Replay log + determinism audit tooling

**Dependencies:** stable core loops from phases 1-7

**Risks:** premature end-state rules before balance maturity

**Acceptance criteria:** players can complete a run and get clear outcome summary

---

## Phase 9 — Late Game Expansion

**Goal:** add high-depth strategic systems for long-form campaigns.

**Current status:** **Planned**

**Key deliverables:**

- Terraforming systems
- Advanced diplomacy (alliances/federations)
- Megaprojects and precursor ruins
- Dynamic events and crisis systems
- Advanced logistics and supply complexity

**Dependencies:** phase 8 end-state foundation

**Risks:** scope growth, readability cost, determinism stress in complex interactions

**Acceptance criteria:** late game remains deterministic, comprehensible, and performance-safe

---

## Phase 10 — Release Hardening

**Goal:** ship-quality reliability, compatibility, and maintainability.

**Current status:** **Planned**

**Key deliverables:**

- Save migration hardening
- Performance benchmarks and optimization pass
- Accessibility and low-motion UX pass
- Cross-platform packaging and terminal compatibility matrix
- Balance pass and playtest/manual documentation

**Dependencies:** phases 1-9 feature stability

**Risks:** regressions from hardening changes, platform variance

**Acceptance criteria:** stable release candidate with green CI, migration confidence, and validated compatibility targets
