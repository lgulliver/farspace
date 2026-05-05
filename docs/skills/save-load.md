# Skill: Save / Load

A playbook for implementing and maintaining save/load in `game_save`.

---

## Crate Responsibility

`game_save` owns serialisation and deserialisation of `GameState`. It must not depend on `game_tui`.

```
game_save/src/
  lib.rs        # public API: save(), load()
  schema.rs     # versioned SaveFile wrapper
  migrate.rs    # migration logic between schema versions
```

---

## Versioned Save Schema

Wrap `GameState` in a versioned envelope so old saves can be detected and migrated:

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub state: GameState,
}

pub const CURRENT_VERSION: u32 = 1;
```

Increment `CURRENT_VERSION` whenever the schema changes in a breaking way.

---

## Serde Expectations

- `GameState` and all types it transitively contains must derive `Serialize` and `Deserialize`.
- Use `#[serde(deny_unknown_fields)]` on stable structs to catch schema drift early during development.
- Use `#[serde(default)]` on optional new fields to allow older saves to load without them.

---

## Backwards Compatibility Strategy

1. Old save version detected → run migration function for each version step.
2. Migration functions are pure: `migrate_v1_to_v2(old: SaveFileV1) -> SaveFileV2`.
3. Keep old version structs in `migrate.rs` until they are no longer needed.
4. Document breaking changes in a comment above `CURRENT_VERSION`.

---

## Corrupted Save Handling

`load()` must return `Result<GameState, SaveError>` — never panic on bad input.

```rust
pub fn load(bytes: &[u8]) -> Result<GameState, SaveError> {
    let file: SaveFile = serde_json::from_slice(bytes)
        .map_err(|e| SaveError::Malformed(e.to_string()))?;
    migrate(file)
}
```

Callers (in `game_tui` or `farspace`) handle `SaveError` by showing an error message, not by crashing.

---

## Public API

```rust
/// Serialise a GameState to bytes.
pub fn save(state: &GameState) -> Result<Vec<u8>, SaveError>;

/// Deserialise bytes to a GameState, running any needed migrations.
pub fn load(bytes: &[u8]) -> Result<GameState, SaveError>;
```

---

## Tests for Valid Saves

```rust
#[test]
fn save_load_round_trip_preserves_state() {
    let engine = Engine::new(42);
    let bytes = save(&engine.state).unwrap();
    let loaded = load(&bytes).unwrap();
    assert_eq!(loaded.turn, engine.state.turn);
    assert_eq!(loaded.seed, engine.state.seed);
}
```

## Tests for Invalid Saves

```rust
#[test]
fn load_empty_bytes_returns_error() {
    assert!(load(&[]).is_err());
}

#[test]
fn load_truncated_json_returns_error() {
    assert!(load(b"{\"version\":1").is_err());
}

#[test]
fn load_wrong_version_is_handled() {
    let old = br#"{"version":0,"state":{}}"#;
    // either migrates successfully or returns a descriptive error
    let result = load(old);
    // assert no panic and either Ok or a known error variant
    match result {
        Ok(_) | Err(SaveError::UnsupportedVersion(_)) => {}
        Err(e) => panic!("unexpected error: {e}"),
    }
}
```

---

## Determinism After Load

After loading, the game must resume as if it was never saved. The RNG state is part of `GameState` and must be serialised and deserialised faithfully. See `docs/skills/deterministic-simulation.md` for the replay test pattern.
