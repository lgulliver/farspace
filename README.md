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

