# Issue: Deterministic Galaxy Generation

**Labels:** `core`, `determinism`, `copilot-ready`

## Goal

Implement `generate_galaxy(seed: u64, star_count: u32) -> Galaxy` in `game_core`. Given the same seed and star count, the function must always produce identical output. Galaxy data must be original — no names from published 4X titles.

## Scope

- `Galaxy` struct containing a `BTreeMap<StarId, Star>` (ordered for determinism)
- Each `Star` has: `id: StarId`, `name: String`, `position: (i32, i32)`
- Names generated from a simple original syllable list (no MoO content)
- Positions distributed without overlap (minimum distance enforced)
- `StarId`s assigned sequentially from 1

## Acceptance Criteria

- [ ] `generate_galaxy(42, 20)` called twice returns identical `Galaxy` values
- [ ] `generate_galaxy(1, 20)` and `generate_galaxy(2, 20)` return different galaxies
- [ ] Star count in output equals the requested `star_count`
- [ ] No two stars share the same position
- [ ] No star names copied from Master of Orion or other 4X titles
- [ ] `game_core` still has no UI/terminal imports

## Tests Required

- `galaxy_is_reproducible_with_same_seed` (positive, determinism)
- `different_seeds_produce_different_galaxies` (property)
- `star_count_matches_requested_count` (positive)
- `no_duplicate_positions` (invariant)
