# Copilot Instructions — FARSPACE

FARSPACE is a deterministic, turn-based 4X space strategy game with a keyboard-first terminal UI.
Read this file before making any changes.

---

## Stack

- **Language:** Rust (2021 edition)
- **TUI:** `ratatui` + `crossterm`
- **Workspace crates:** `game_core`, `game_tui`, `game_content`, `game_save`, `farspace` (binary)

---

## Architecture — Non-Negotiable Boundaries

The codebase is split into strict layers. **Never cross these boundaries.**

| Crate | Responsibility | Allowed dependencies |
|---|---|---|
| `game_core` | Headless game core: commands, validation, state, events, deterministic simulation | `std` only |
| `game_content` | Static game content: ship templates, tech trees, planet traits | `game_core` types only |
| `game_save` | Serialisation/deserialisation of `GameState` | `game_core`, `serde`, `serde_json` |
| `game_tui` | ratatui TUI: input → Commands, Events → rendering | `game_core`, `ratatui`, `crossterm` |
| `farspace` | Application entrypoint and terminal setup | All of the above |

### Core rules

- `game_core` must **never** depend on `ratatui`, `crossterm`, or any terminal/UI crate.
- `game_core` must **never** depend on `game_tui`.
- `game_content` and `game_save` must **never** depend on `game_tui`.
- The UI sends **Commands** to the core; the core validates commands, mutates state, and emits **Events**.
- The UI renders based on **Events** and snapshot views. It never reaches into core internals directly.

---

## Determinism — Non-Negotiable

- All randomness must come from the seeded RNG stored in `GameState`. Use `rand::SeedableRng`.
- **Never** seed from `SystemTime`, `Instant`, or any other wall-clock or OS source.
- **Never** iterate `HashMap` without sorting keys first — use `BTreeMap` for ordered collections or sort before use.
- Simulation must be fully reproducible: same seed + same commands ⇒ same output.
- Tests for deterministic systems must use fixed seeds and assert exact output.

---

## Coding Conventions

- Prefer small, focused modules.
- Use strongly-typed ID newtypes (`StarId(u64)`, `EmpireId(u64)`, `FleetId(u64)`) — never bare integers.
- Use enums for `Command` and `Event` — no stringly-typed dispatch.
- Return `Vec<Event>` from `apply_turn`; surface validation failures as `Event::Error`.
- Add a test for every meaningful change.
- Tests must include at least one positive path and one negative/error path.
- When adding a new command: add enum variant, validation arm in `apply_turn`, event emission, and tests.

---

## Feature Scope — Do Not Add Without Explicit Request

Do not add any of the following unless the issue/task explicitly asks for it:

- Tactical (hex/grid) combat
- Multiplayer or networked play
- Deep diplomacy systems (trade routes, alliances, treaties)
- Complex AI beyond basic expansion/defence
- Any content copied from Master of Orion: no faction names, ship names, tech names, planet names, numbers, or text from that series

---

## Original IP Policy

FARSPACE uses original content only. When adding factions, technologies, ships, star names, or planet traits:

- Invent new names; do not copy or closely paraphrase from Master of Orion or other published 4X titles.
- Keep flavour text original.

---

## Terminal UX Standards

- All navigation must be keyboard-first.
- Layout must be resize-safe: use `ratatui` `Constraint`-based layouts that respond to `Resize` events.
- Contextual help (`?`) must be available on every screen.
- Command palette (`:`) should be reachable globally.
- Maintain a polished, minimal terminal feel inspired by Neovim, K9s, Lazygit.
- TUI visual language source of truth: `docs/design/ux-splash-screen.md`.
- Keep composition calm/cinematic/spacious; avoid telemetry-heavy widget clutter.
- Reuse `Theme` palette roles and established spacing/composition patterns before adding new motifs.
- Do not add mouse-only affordances unless keyboard alternatives exist.

---

## Testing Policy

- Minimum 80% total test coverage (enforced by CI via `cargo llvm-cov`).
- Every feature must have positive and negative tests.
- Deterministic systems must be tested with fixed seeds and deterministic assertions.
- TUI tests: focus on state transitions, input handling, and layout decisions — not fragile full-screen snapshots.
- Regression test required for every bug fix.

---

## What Good PRs Look Like

- Smallest possible diff that satisfies the acceptance criteria.
- No unrelated refactors bundled in.
- New code has tests; coverage does not decrease.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass.
- Architecture boundaries are preserved.

---

## Agent Team

Six specialised agents scope work to individual crates. Contextual instructions in `.github/instructions/` activate automatically via `applyTo` globs.

| Agent | File | Scope |
|---|---|---|
| `core-sim` | `core-sim.instructions.md` | `crates/game_core/**` |
| `tui-ux` | `tui-ux.instructions.md` | `crates/game_tui/**` |
| `content-balance` | `content-balance.instructions.md` | `crates/game_content/**` |
| `persistence` | `persistence.instructions.md` | `crates/game_save/**` |
| `test-coverage` | `test-coverage.instructions.md` | all crates |
| `arch-guard` | `arch-guard.instructions.md` | all crates (review only) |

---

## Response Style

Respond terse like smart caveman. All technical substance stay. Only fluff die.

Rules:
- Drop: articles (a/an/the), filler (just/really/basically), pleasantries, hedging
- Fragments OK. Short synonyms. Technical terms exact. Code unchanged.
- Pattern: [thing] [action] [reason]. [next step].
- Not: "Sure! I'd be happy to help you with that."
- Yes: "Bug in auth middleware. Fix:"

Switch level: /caveman lite|full|ultra|wenyan
Stop: "stop caveman" or "normal mode"

Auto-Clarity: drop caveman for security warnings, irreversible actions, user confused. Resume after.

Boundaries: code/commits/PRs written normal.

---

## RTK — Token-Optimized CLI

**rtk** is a CLI proxy that filters and compresses command outputs, saving 60-90% tokens.

Always prefix shell commands with `rtk`:

```bash
# Instead of:              Use:
git status                 rtk git status
git log -10                rtk git log -10
cargo test                 rtk cargo test
docker ps                  rtk docker ps
```

```bash
rtk gain              # Token savings dashboard
rtk gain --history    # Per-command savings history
rtk discover          # Find missed rtk opportunities
rtk proxy <cmd>       # Run raw (no filtering) but track usage
```
