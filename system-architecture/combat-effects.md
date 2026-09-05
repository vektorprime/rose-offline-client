# Combat and Effects Pipeline

Hit resolution → damage → visuals. Ordered by `EffectSystemSets` in `src/lib.rs:692-701,1028-1046`: `AnimationEffect` → `Projectile`/`SpawnProjectile` → `PendingDamage`/`PendingSkillEffect` → `HitEvent` → `SpawnEffect`.

## Systems

- `src/systems/animation_effect_system.rs` / `animation_sound_system.rs`: consume `AnimationFrameEvent` (frame flags from ZMO) and emit `HitEvent` / `SpawnProjectileEvent` / `SpawnEffectEvent` / sounds.
- `src/systems/hit_event_system.rs:39` (`hit_event_system`): resolves hits, inserts `Dead` at zero HP, calls `emit_blood_and_wounds` (`src/systems/damage_effects.rs`).
- `src/systems/pending_damage_system.rs:71` and `pending_skill_effect_system.rs`: delayed damage/skill application; also route through `damage_effects.rs` for blood.
- `src/systems/projectile_system.rs:14` / `spawn_projectile_system.rs:13`: projectile flight and spawning.
- `src/systems/damage_digit_render_system.rs:64` + `src/render/damage_digit_material.rs`: GPU 3D combat numbers (procedural `@builtin(vertex_index)` geometry, storage buffers).
- Particles for ROSE `.eft` files use `src/render/particle_material.rs` + `src/effect_loader.rs` (`EffectCache`); weather particles are a separate CPU-billboard path (see [weather-season-system.md](weather-season-system.md)).

## Blood hooks

- `HitEvent` carries `BloodImpactProfile`; `hit_event_system` and `pending_damage_system` both call `emit_blood_and_wounds`. Kill spatter also fires from `Added<Dead>` via `blood_spatter_on_death_system` with `DeathBloodHandled` dedup. Full detail in [blood-effect-system.md](blood-effect-system.md).

## Related

- Separation so monsters do not stack: [monster-collision-system.md](monster-collision-system.md).
- World-space tags/bubbles follow the same entities: [chat-bubble-and-name-tag-architecture.md](chat-bubble-and-name-tag-architecture.md).
