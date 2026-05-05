# Copilot Instructions — FARSPACE

FARSPACE is a deterministic, turn-based 4X space strategy game with a keyboard-first terminal UI.
Read this file before making any changes.

---

## Architecture — Non-Negotiable Boundaries

The codebase is split into strict layers. **Never cross these boundaries.**

| Package | Responsibility | Allowed dependencies |
|---|---|---|
| `internal/game` | Headless game core: commands, validation, state, events, deterministic simulation | stdlib only |
| `internal/content` | Static game content: ship templates, tech trees, planet traits | `internal/game` types only |
| `internal/save` | Serialisation/deserialisation of `GameState` | `internal/game`, stdlib, encoding packages |
| `internal/ui` | Bubble Tea TUI: input → Commands, Events → rendering | All of the above, `bubbletea`, `lipgloss`, `bubbles` |
| `cmd/farspace` | Application entrypoint | All of the above |

### Core rules

- `internal/game` must **never** import `bubbletea`, `lipgloss`, `bubbles`, or any terminal/UI package.
- `internal/game` must **never** import `internal/ui`.
- `internal/content` and `internal/save` must **never** import `internal/ui`.
- The UI sends **Commands** to the core; the core validates commands, mutates state, and emits **Events**.
- The UI renders based on **Events** and `SnapshotFor` views. It never reads internal core structs directly.

---

## Determinism — Non-Negotiable

- All randomness must come from the seeded `Rng` stored in `GameState`.
- **Never** use `time.Now()`, `rand.New(rand.NewSource(time.Now()...))`, or any non-deterministic source.
- **Never** range over `map` directly in game logic — sort keys first.
- Simulation must be fully reproducible: same seed + same commands ⇒ same output.
- Tests for deterministic systems must use fixed seeds and assert exact output.

---

## Coding Conventions

- Prefer small, focused packages and types.
- Use explicit domain structs and typed IDs (e.g. `StarID`, `EmpireID`, `FleetID`) rather than plain `int` or `string`.
- Prefer named result types and domain errors over generic `error` strings.
- Add a test for every meaningful change.
- Tests must include at least one positive path and one negative/error path.
- When adding a new command, add: struct definition, validation in `ApplyTurn`, event emission, and tests.

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
- Layout must be resize-safe: use proportional or responsive lipgloss containers.
- Contextual help (`?`) must be available on every screen.
- Command palette (`:`) should be reachable globally.
- Maintain a polished, minimal terminal feel inspired by Neovim, K9s, Lazygit.
- Do not add mouse-only affordances unless keyboard alternatives exist.

---

## Testing Policy

- Minimum 80% total test coverage (enforced by CI).
- Every feature must have positive and negative tests.
- Deterministic systems must be tested with fixed seeds and deterministic assertions.
- UI tests: focus on state transitions, input handling, and layout decisions — not fragile pixel/character snapshots.
- Regression test required for every bug fix.

---

## What Good PRs Look Like

- Smallest possible diff that satisfies the acceptance criteria.
- No unrelated refactors bundled in.
- New code has tests; coverage does not decrease.
- `go fmt`, `go vet`, and `golangci-lint` pass.
- Architecture boundaries are preserved.
