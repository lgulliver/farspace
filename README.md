# FARSPACE

<img width="1248" height="781" alt="image" src="https://github.com/user-attachments/assets/8894e8ee-4d5f-44e0-9aa4-9d37455236a2" />

Deterministic, turn-based 4X space strategy for terminal.

- **Language:** Rust
- **UI:** `ratatui` + `crossterm`
- **Simulation:** headless `game_core` with command/event model
- **Design:** keyboard-first, professional terminal UX
- **IP policy:** original IP only

## Current Status

FARSPACE is playable alpha with multi-screen TUI and deterministic core loop.

- Current implementation snapshot: [`docs/current-state.md`](docs/current-state.md)
- Architecture source of truth: [`docs/architecture.md`](docs/architecture.md)
- Full delivery plan: [`docs/roadmap.md`](docs/roadmap.md)
- Next delivery queue: [`docs/next-slices.md`](docs/next-slices.md)
- Terminal fonts and visual modes: [`docs/user/terminal-fonts.md`](docs/user/terminal-fonts.md)
- 90s-style game manual (GitHub Pages source): [`docs/manual/index.html`](docs/manual/index.html)
- Recently completed:
  - ✅ Planet Specials & Anomalies v1
  - ✅ Strategic Resources v1
  - ✅ Large-Scale Tech Tree v2
  - ✅ Research Queue v1

## Core Design Goals

- Deterministic simulation: same seed + same commands => same outcomes
- Strict crate boundaries: headless core, TUI adapter, separate save/content crates
- Command/event pipeline over direct mutation from UI
- Keyboard-first terminal interaction with contextual help and command palette
- Incremental vertical slices with tests and coverage gate

## Terminal / TUI Vision

- Fast, clear, low-noise tactical readability in text UI
- Global `?` contextual help and `:` command palette
- Resize-safe layouts with `ratatui` constraints
- State/event driven screens (menu, setup, sector, system, colony, research, diplomacy, overview)

## Architecture Overview

Core flow:

```text
UI input
→ Command
→ game_core validation
→ state mutation
→ Events
→ TUI rendering / turn report (event log)
```

Detailed architecture and boundaries: [`docs/architecture.md`](docs/architecture.md)

## Workspace / Crate Layout

| Crate | Responsibility |
|---|---|
| `game_core` | Headless simulation: state, commands, events, deterministic turn processing, AI |
| `game_tui` | ratatui/crossterm UI: key handling, screen state, rendering, command dispatch |
| `game_content` | Static content helpers and templates |
| `game_save` | Versioned save/load and migrations for `GameState` |
| `game_e2e` | Deterministic 100-turn E2E simulated-playthrough harness + reports |
| `farspace` | Binary entrypoint and terminal lifecycle |

## Installation

### Pre-built binaries (recommended)

Download the latest release from [GitHub Releases](https://github.com/lgulliver/farspace/releases).

**Linux (x86_64 / aarch64)**

```bash
# Download the binary for your architecture, e.g.:
curl -Lo farspace https://github.com/lgulliver/farspace/releases/latest/download/farspace-linux-x86_64
chmod +x farspace
./farspace
```

**macOS (Apple Silicon / Intel)**

```bash
# Apple Silicon (M-series):
curl -Lo farspace https://github.com/lgulliver/farspace/releases/latest/download/farspace-macos-aarch64
# Intel Mac:
# curl -Lo farspace https://github.com/lgulliver/farspace/releases/latest/download/farspace-macos-x86_64

chmod +x farspace
# Remove macOS quarantine flag (required on first run):
xattr -d com.apple.quarantine farspace 2>/dev/null || true
./farspace
```

**Windows (x86_64)**

1. Download `farspace-windows-x86_64.exe` from [GitHub Releases](https://github.com/lgulliver/farspace/releases).
2. Rename or keep the file as `farspace.exe`.
3. Open a terminal (PowerShell or Command Prompt) in the download folder and run:
   ```powershell
   .\farspace.exe
   ```

> FARSPACE is a terminal application. Run it inside a terminal emulator (Windows Terminal, iTerm2, Alacritty, kitty, etc.) for the best experience.

### Build from source

Requires [Rust](https://rustup.rs/) stable (2021 edition).

```bash
git clone https://github.com/lgulliver/farspace.git
cd farspace
cargo run --release -p farspace
```

### Common Controls

- `N`: new game
- `L`: load game from menu
- `hjkl` / arrows: navigation
- `Enter` / `e` / `t`: confirm or end turn (context dependent)
- `?`: contextual help
- `:`: command palette (`save`, `load`)
- `Q` / `Ctrl+C`: quit

## Alpha Player Guide

### How to start a game

1. From menu press `N`.
2. Pick faction with `j/k`, confirm with `Enter`.
3. In setup choose galaxy size / AI count / seed, then press `S` to start.

### First 20 turns (recommended flow)

1. Press `Enter` to open sector map.
2. Use `S` on unexplored stars to dispatch scout fleets.
3. Open systems with `Enter`; survey planets with `S`.
4. Colonize surveyed habitable worlds with `C`.
5. Open research with `r`; set active tech with `Enter`; queue follow-ups with `a`.
6. Open colony with `c`; queue production with `Enter`.
7. End turns with `E` or `T`; review turn summaries and dispatch bulletins (`N`).
8. Check empire progress with `O`/`V`; monitor diplomacy with `D`.

### Major systems overview

- **Exploration:** sector/system navigation, scout missions, survey actions.
- **Colonization & economy:** colony creation, role assignment, production queue, supply/food/credits.
- **Research:** active tech, queue management, unlock-driven progression.
- **Diplomacy & war:** first contact, relationship drift, empire intel levels, war state, invasion and strategic auto-resolve combat.
- **Victory:** progress tracked in empire overview (`O`/`V`) across enabled victory paths.
- **Save/load:** menu (`L`) or command palette (`:` then `save` / `load`).

### Known alpha limitations

- Diplomacy supports stance/state progression and war, but no treaty negotiation UI.
- Combat is strategic auto-resolve only (no tactical battle layer).
- Economy balance and late-game pacing are still being tuned.
- Some advanced roadmap systems remain future work (jobs depth, advanced diplomacy, late-game expansion).

## Testing and Coverage

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
```

Run the E2E simulated playthrough locally:

```bash
cargo test -p game_e2e --test e2e_100_turn_playthrough -- --nocapture
```

Optional E2E runner command:

```bash
cargo run -p game_e2e --bin e2e_runner -- --seed 12345 --turns 100 --report target/e2e/playthrough-report.json
```

For intel / diplomacy slices, use the report to verify rival info stays redacted until the expected intel level and that deterministic runs produce the same intel gains for the same seed.

Testing policy details: [`docs/testing.md`](docs/testing.md)

## Current Gameplay Loop

Current playable loop:

1. Start game from menu and setup
2. Explore sectors and systems
3. Survey planets with science fleets
4. Colonize surveyed habitable worlds
5. Manage colony queues/economy/research
6. End turn, process events, react to AI and diplomacy state

More detail: [`docs/gameplay-loop.md`](docs/gameplay-loop.md)

## Current Limitations

- No final victory/campaign end-state yet
- No tactical battle layer (combat is auto-resolve)
- Diplomacy has relationship states and war declaration, but no treaty/deal system
- Pop/jobs simulation layer is not implemented yet
- Advanced late-game systems are roadmap items


## License

Licensed under the [GNU General Public License v3.0](LICENSE) (GPL-3.0-only).

## Contribution and Agent Workflow (Summary)

- Read first: [`AGENTS.md`](AGENTS.md)
- Follow docs source-of-truth set:
  - [`docs/architecture.md`](docs/architecture.md)
  - [`docs/roadmap.md`](docs/roadmap.md)
  - [`docs/current-state.md`](docs/current-state.md)
  - [`docs/next-slices.md`](docs/next-slices.md)
  - [`docs/testing.md`](docs/testing.md)
  - [`docs/design/index.md`](docs/design/index.md)
- Keep changes small, deterministic, and boundary-safe
- Add tests for meaningful behavior changes; keep coverage >= 80%
