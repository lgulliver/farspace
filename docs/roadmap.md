# FARSPACE Roadmap

Phases represent logical milestones, not fixed time-boxes. Each phase ships a working, tested slice of the game.

Status key:
- ✅ implemented in the current build
- 🟡 partially implemented / simplified
- ⬜ not implemented yet

---

## Phase 0 — Repo Setup, CI, Architecture Skeleton ✅ complete

- ✅ Rust workspace: `game_core`, `game_tui`, `game_content`, `game_save`, `farspace`
- ✅ CI: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo llvm-cov` with 80% gate
- ✅ Copilot instructions, issue templates, PR template, skills playbooks
- ✅ Workspace and crate boundaries are established and enforced

---

## Phase 1 — Playable Vertical Slice 🟡 mostly complete

- ✅ Deterministic galaxy generation (fixed seed → reproducible map)
- ✅ Command/event model (`EndTurn`, `MoveFleet`, colony management and exploration commands)
- ✅ Colony production queue (ships and structures)
- ✅ Auto-resolve combat (deterministic, no tactical layer)
- ✅ Fog of war / exploration visibility
- ✅ Main menu screen
- ✅ Galaxy overview + sector map + system screen
- 🟡 Turn report exists as rolling event log (no separate report screen)
- ✅ Save/load a single local save file (`farspace.sav`)
- ⬜ Win-by-conquest / endgame victory flow

---

## Phase 2 — Colony and Economy Depth 🟡 in progress

- ✅ Planet classes, size, specials/resources, and per-turn yield model
- ⬜ Population growth and migration systems
- 🟡 Colony focus values exist in simulation; no dedicated interactive slider UI yet
- 🟡 Build queue cancellation exists in core API; reordering/cancel is not exposed in TUI
- ✅ Event log panel
- ✅ Colony detail screen
- ✅ Empire/economy overview screen

---

## Phase 3 — Research and Ship Design 🟡 in progress

- ✅ Tech tree with prerequisites and unlocks (original content)
- ⬜ `TechBreakthrough` 3-choice prompt flow
- ⬜ Ship design editor (hull + component slots)
- ⬜ Ship design screen
- ✅ Research screen (selection, progress, completion)

---

## Phase 4 — AI Opponent 🟡 in progress

- ✅ Basic expansion AI (explore, colonize, build, pick research)
- ✅ AI uses the same deterministic turn pipeline
- ⬜ Difficulty tuning hooks / configurable difficulty levels

---

## Phase 5 — Diplomacy 🟡 minimal baseline

- ✅ First-contact detection and eventing
- 🟡 Relationship model is currently `Unknown` / `Contacted` only
- ✅ Diplomacy screen (contact visibility)
- ⬜ Stance/treaty mechanics (Neutral/Hostile/Ceasefire, deals, negotiations)

---

## Phase 6 — Fleet and Combat Expansion ⬜ not started

- ⬜ Fleet merge/split commands
- ⬜ Fleet stance (Engage / Avoid / Blockade)
- ⬜ Richer combat summary events (losses, retreat)
- ⬜ Fleet detail screen

---

## Phase 7 — Polish, Save Compatibility, Packaging 🟡 in progress

- ✅ Versioned save schema with migration support
- ⬜ User keybinding configuration
- ⬜ User theme configuration
- ✅ Help overlay on all screens
- 🟡 Command palette available for save/load commands
- ✅ Release binary builds (GitHub Actions)

---

## Phase 8 — Multiplayer Investigation ⬜ not started

- Evaluate hotseat (shared save, turn-swap)
- Evaluate async play-by-email model
- No implementation commitment yet

---

_Features not on this roadmap (tactical combat, diplomacy treaties, multiplayer networking) require explicit design before any code is written._
