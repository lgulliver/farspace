# FARSPACE Roadmap

Phases represent logical milestones, not fixed time-boxes. Each phase ships a working, tested slice of the game.

---

## Phase 0 — Repo Setup, CI, Architecture Skeleton ✦ current

- Rust workspace: `game_core`, `game_tui`, `game_content`, `game_save`, `farspace`
- CI: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo llvm-cov` with 80% gate
- Copilot instructions, issue templates, PR template, skills playbooks
- Empty but compilable workspace skeleton with documented crate boundaries

---

## Phase 1 — Playable Vertical Slice

- Deterministic galaxy generation (fixed seed → reproducible map)
- Command/event model: `EndTurn`, `SetBudget`, `MoveFleet`
- Colony production queue (build ships and infrastructure)
- Auto-resolve combat (deterministic, no tactical layer)
- Fog of war / visibility
- Main menu screen
- Galaxy map screen
- Turn report screen
- Save/load a single game slot
- Win by conquest

---

## Phase 2 — Colony and Economy Depth

- Planet traits (size, fertility, minerals — original names)
- Population growth and migration stubs
- Colony focus sliders
- Build queue reordering and cancellation
- Event log panel
- Colony detail screen
- Economy summary screen

---

## Phase 3 — Research and Ship Design

- Tech tree (original content only)
- `TechBreakthrough` event with 3-choice prompt
- Ship design editor (hull + component slots)
- Ship design screen
- Research screen

---

## Phase 4 — AI Opponent

- Basic expansion AI: colonise, build, move fleets
- AI uses the same command/event pipeline as the player
- Difficulty tuning hooks (no hardcoded magic numbers)

---

## Phase 5 — Diplomacy

- First contact events
- Stances: Neutral, Hostile, Ceasefire
- No complex treaty system yet (kept minimal)
- Diplomacy screen

---

## Phase 6 — Fleet and Combat Expansion

- Fleet merge/split commands
- Fleet stance (Engage / Avoid / Blockade)
- Richer combat summary events (losses, retreat)
- Fleet detail screen

---

## Phase 7 — Polish, Save Compatibility, Packaging

- Versioned save schema with migration support
- Keybinding configuration
- Colour theme configuration
- Help overlay polished on all screens
- Command palette wired end-to-end
- Release binary builds (GitHub Actions)

---

## Phase 8 — Multiplayer Investigation

- Evaluate hotseat (shared save, turn-swap)
- Evaluate async play-by-email model
- No implementation commitment yet

---

_Features not on this roadmap (tactical combat, diplomacy treaties, multiplayer networking) require explicit design before any code is written._
