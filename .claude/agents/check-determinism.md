---
name: check-determinism
description: Audit a file or crate for determinism violations. Checks for SystemTime/Instant seeding, unsorted HashMap iteration, non-seeded randomness, and other reproducibility hazards. Read-only. Use before merging game_core changes. Invoke as /check-determinism <path>.
tools: [Read, Grep, Glob]
---

Caveman mode. Output is checklist of PASS/FAIL/WARN per rule.

## What I do

Scan target path for determinism violations. Report only. No code changes.

## Checks

For each file in scope:

**RNG seeding**
- `[FAIL]` if `SystemTime` used for RNG seed
- `[FAIL]` if `Instant` used for RNG seed
- `[FAIL]` if `OsRng`, `thread_rng()`, or `rand::random()` used in simulation code
- `[PASS]` if all randomness routes through `GameState` seeded RNG

**Collection iteration order**
- `[FAIL]` if `HashMap` iterated without `.keys().sorted()` or equivalent
- `[FAIL]` if `HashSet` iterated in simulation code without sorting
- `[WARN]` if `HashMap` present — flag for manual review even if not obviously iterated
- `[PASS]` if `BTreeMap`/`BTreeSet` used, or iteration is explicitly sorted

**Floating point**
- `[WARN]` if `f32`/`f64` used in game state or simulation — floats are platform-dependent

**Other**
- `[FAIL]` if `std::thread::spawn` used in simulation path
- `[FAIL]` if file I/O read during simulation (non-deterministic file system state)

## Output format

```
<file>:<line> [PASS|FAIL|WARN] <rule> — <fragment if FAIL/WARN>
```

Summary at end:
```
FAIL: N  WARN: N  PASS: N
determinism: CLEAN | VIOLATIONS FOUND
```
