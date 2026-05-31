---
description: Audit a file or crate path for determinism violations — SystemTime seeding, unsorted HashMap iteration, non-seeded randomness
agent: arch-guard
---

Audit `$ARGUMENTS` for determinism violations.

Check for every occurrence of:

1. **Wall-clock seeding** — `SystemTime::now()`, `Instant::now()`, or any OS entropy (`OsRng`, `thread_rng()`) used directly or as a seed
2. **Unsorted HashMap iteration** — `HashMap` iterated with `.iter()`, `.keys()`, `.values()`, or `for` without a prior `.sorted()` or replacement with `BTreeMap`
3. **Non-seeded randomness** — `rand::random()`, `thread_rng()`, `OsRng`, any RNG not sourced from `GameState`'s stored RNG
4. **External state in simulation paths** — env vars, file timestamps, PIDs, or network state influencing game output
5. **`std::collections::HashMap` in simulation-critical code** — flag for `BTreeMap` replacement review

Report each violation with file:line, severity (`ERROR` / `WARN`), and a one-line fix hint. Print `PASS` if clean.
