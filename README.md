# FARSPACE

A deterministic, turn-based 4X space strategy game for the terminal.

**Stack:** Rust · ratatui · crossterm

---

## Architecture

FARSPACE uses a strict headless-core / TUI-client separation:

| Crate | Role |
|---|---|
| `game_core` | Pure simulation — commands, validation, state, events. No terminal dependencies. |
| `game_content` | Static content — tech trees, ship templates, planet traits. |
| `game_save` | Save/load — versioned JSON serialisation of `GameState`. |
| `game_tui` | ratatui TUI — input → Commands, Events → rendering. |
| `farspace` | Binary entrypoint — terminal setup and main loop. |

The UI sends **Commands** to the core. The core validates, mutates state, and emits **Events**. The UI renders events. The UI never mutates core state directly.

---

## Install

### Pre-built binaries

Download the latest release for your platform from the [Releases page](https://github.com/lgulliver/farspace/releases):

| Platform | File |
|---|---|
| Linux x86_64 | `farspace-linux-x86_64` |
| Linux aarch64 | `farspace-linux-aarch64` |
| macOS x86_64 | `farspace-macos-x86_64` |
| macOS aarch64 (Apple Silicon) | `farspace-macos-aarch64` |
| Windows x86_64 | `farspace-windows-x86_64.exe` |

On Linux/macOS, make the binary executable and run it:

```bash
chmod +x farspace-linux-x86_64
./farspace-linux-x86_64
```

### Build from source

Requires [Rust](https://rustup.rs/) (stable toolchain).

```bash
git clone https://github.com/lgulliver/farspace.git
cd farspace
cargo build --release -p farspace
./target/release/farspace
```

---

## Development

```bash
cargo build --workspace
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

### Coverage

```bash
cargo install cargo-llvm-cov   # install once
cargo llvm-cov --workspace --all-targets --summary-only
```

Minimum required: **80%** (enforced by CI).

---

## Docs

- [`docs/roadmap.md`](docs/roadmap.md) — phased feature plan
- [`docs/testing.md`](docs/testing.md) — testing standards and coverage policy
- [`docs/skills/`](docs/skills/) — playbooks for common development tasks
- [`docs/issues/`](docs/issues/) — initial issue drafts
- [`docs/github-labels.md`](docs/github-labels.md) — label definitions
- [`.github/copilot-instructions.md`](.github/copilot-instructions.md) — Copilot/agent guidance

