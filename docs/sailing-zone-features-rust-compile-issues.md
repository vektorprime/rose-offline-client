# Sailing Zone Features - Rust Compile Issues

Scope: `sailing-zone-features` branch compared with `origin/main`.

Validation performed:
- Reviewed `pitfalls` and `system-architecture` notes for zone, water, ECS, transform, and physics context.
- Ran `cargo check`; it fails before deeper system validation with 12 reported errors.
- Checked `C:\Users\vicha\RustroverProjects\rust-errors\all-rust-errors.md` entries for E0106, E0405, E0425, and E0603.

## Current Errors

### 1. C-style ternary operators

Files:
- `src/sailing/mod_sea_creatures.rs:310`
- `src/sailing/mod_sea_creatures.rs:345`
- `src/sailing/mod_sea_creatures.rs:350`

Problem: Rust has no `condition ? a : b` operator.

Fix: Replace each expression with Rust `if { } else { }`, for example:

```rust
position.position.z = if (-100.0..0.0).contains(&position.position.z) {
    position.position.z
} else {
    -50.0
};
```

### 2. Missing return lifetime

File: `src/sailing/mod_sailing_zone.rs:49`

Problem: `get_sailing_config(config: &Res<SailingZoneConfig>) -> &SailingZoneConfig` returns a borrow without stating which input lifetime it is tied to.

Fix: Prefer removing the helper and using `config.as_ref()` at call sites. If kept, give the function an explicit lifetime tied to the input borrow.

### 3. Invalid random range API

File: `src/sailing/mod_storms.rs:91-92`

Problem: `(2000.0..8000.0).gen()` is not a Rust or `rand` API.

Fix: Use `rand::Rng::gen_range` on an RNG:

```rust
let mut rng = rand::thread_rng();
let x = rng.gen_range(2000.0..8000.0);
```

### 4. Field used on the wrong struct

File: `src/sailing/mod_storms.rs:123`

Problem: `storm.flash_timer.reset()` tries to access `flash_timer` on `StormState`, but that field exists on `StormVisual`.

Fix: Either add the timer to `StormState`, or query/update a `StormVisual` component/resource instead.

### 5. `Projectile::default()` does not exist

File: `src/sailing/mod_cannons.rs:116`

Problem: `crate::components::Projectile` has no `Default` implementation and requires fields such as `target`, `source`, `effect_id`, `skill_id`, `move_type`, and `move_speed`.

Fix: Construct a full `Projectile` value, add a valid `Default` impl if a generic projectile makes sense, or remove this component from cannonballs and use only `CannonProjectile`.

## Masked Or Latent Rust Errors

### 6. Orphaned nested module tree has missing files

File: `src/sailing/pirates/mod.rs:30-31`

Problem: The nested `src/sailing/pirates` module declares `combat_system` and `cleanup_system`, but the branch does not add `combat_system.rs` or `cleanup_system.rs`. This is not currently emitted because `src/sailing/mod.rs` does not declare `mod pirates;`.

Fix: Add the missing files, remove the declarations, or choose one sailing implementation path and delete the orphaned tree.

### 7. `.copied()` called on `Option<Entity>`

Files:
- `src/sailing/pirates/spawn_system.rs:72`
- `src/sailing/sea_creatures/spawn_system.rs:92`

Problem: `Query<Entity>::iter().next()` returns `Option<Entity>`, not `Option<&Entity>`, so `copied()` is invalid.

Fix:

```rust
let zone_entity = zone_query.iter().next().unwrap_or(Entity::PLACEHOLDER);
```

