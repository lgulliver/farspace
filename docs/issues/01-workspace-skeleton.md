# Issue: Rust Workspace Skeleton

**Labels:** `core`, `good-first-issue`, `copilot-ready`

## Goal

Create the compilable Rust workspace with all crates in place, correct dependency graph, and enforced crate boundaries. No game logic yet — just the skeleton that compiles cleanly and passes CI.

## Crates

| Crate | Type | Depends on |
|---|---|---|
| `game_core` | lib | `std` only |
| `game_content` | lib | `game_core` |
| `game_save` | lib | `game_core`, `serde`, `serde_json` |
| `game_tui` | lib | `game_core`, `ratatui`, `crossterm` |
| `farspace` | binary | `game_core`, `game_tui`, `crossterm`, `clap` |

## Acceptance Criteria

- [ ] `cargo build --workspace` succeeds with no errors or warnings
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Each crate has a `lib.rs` (or `main.rs`) with at minimum a module comment
- [ ] `game_core` has zero `ratatui`/`crossterm` imports (CI or `cargo tree` confirms)
- [ ] `farspace` binary runs and exits cleanly (`farspace --help`)

## Tests Required

- Smoke test in each lib crate (e.g. `assert!(true)` with a descriptive name to verify the crate loads)
- No coverage gate failure (trivially passing crates count toward coverage)
