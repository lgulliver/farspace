# FARSPACE

Deterministic, turn-based 4X space strategy for terminal.

- **Language:** Rust (2021)
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
- 90s-style game manual (GitHub Pages source): [`docs/manual/index.html`](docs/manual/index.html)
- Recently completed:
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

## Testing and Coverage

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
```

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

Licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).

- Contributions are welcome under this license.
- Commercial use requires a separate commercial license from the project owner.

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
