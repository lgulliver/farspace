# FARSPACE — Project Instructions

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

<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (60-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk go test             # Go test failures only (90%)
rtk jest                # Jest failures only (99.5%)
rtk vitest              # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk pytest              # Python test failures only (90%)
rtk rake test           # Ruby test failures only (90%)
rtk rspec               # RSpec test failures only (60%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%). Format flags (-c, -l, -L, -o, -Z) run raw.
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->
