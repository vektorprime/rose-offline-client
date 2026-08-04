# Optimization Review 03 — Gameplay Systems

## 1. Title & Scope

**Scope:** Core gameplay systems: network-message handling, player commands, combat, damage, effects, blood, cooldowns, status effects, projectiles.

**Files reviewed (all verified against source):**

| Area | Files |
|---|---|
| Network handling | `src/systems/game_connection_system.rs` (2933 lines), `src/events/network_event.rs` |
| Player commands | `src/systems/player_command_system.rs` (852), `src/systems/command_system.rs` (1237), `src/systems/game_keyboard_input_system.rs`, `src/systems/game_mouse_input_system.rs`, `src/systems/move_speed_set_system.rs`, `src/events/player_command_event.rs` |
| Combat / damage | `src/systems/hit_event_system.rs` (163), `src/systems/pending_damage_system.rs` (181), `src/systems/pending_skill_effect_system.rs` (268), `src/systems/damage_effects.rs` (88), `src/events/hit_event.rs`, `src/components/pending_damage_list.rs`, `src/components/pending_skill_effect_list.rs` |
| Status effects / cooldowns | `src/systems/status_effect_system.rs` (69), `src/systems/visible_status_effects_system.rs` (69), `src/systems/cooldown_system.rs` (47), `src/components/cooldowns.rs` (132), `src/systems/use_item_event_system.rs` (71) |
| Effects / projectiles | `src/systems/spawn_effect_system.rs` (147), `src/systems/effect_system.rs` (61), `src/systems/animation_effect_system.rs` (468), `src/systems/effect_resolution.rs` (89), `src/systems/spawn_projectile_system.rs`, `src/systems/projectile_system.rs` (129), `src/components/projectile.rs`, `src/events/spawn_effect_event.rs`, `src/events/spawn_projectile_event.rs` |
| Damage digits | `src/systems/damage_digit_render_system.rs` (180), `src/resources/damage_digits_spawner.rs` (101), `src/render/damage_digit_material.rs`, `src/render/damage_digit_render_data.rs`, `src/components/damage_digits.rs` |
| Blood effects | `src/systems/blood_spatter_system.rs` (567), `src/systems/blood_overlay_system.rs` (400), `src/systems/gash_wound_system.rs` (407), `src/systems/dirt_dash_system.rs` (254), `src/blood_effect_plugin.rs` (81), `src/events/blood_effect_event.rs`, `src/resources/blood_effect_config.rs`, `src/resources/blood_effect_runtime.rs`, `src/resources/blood_decal_atlas.rs`, `src/resources/blood_overlay_atlas.rs`, `src/components/blood_effect.rs`, `src/components/blood_overlay.rs` |
| Support | `src/components/client_entity.rs`, `client_entity_name.rs`, `dead.rs`, `effect.rs`, `facing_direction.rs`, `position.rs`, `command.rs`, `src/resources/client_entity_list.rs`, `selected_target.rs`, `src/events/use_item_event.rs`, `system_func_event.rs`, `src/lib.rs` (registration/ordering) |

**Architecture docs that existed** (checked in `system-architecture/`): `ECS.md`, `Input.md`, `Audio.md`, `Assets.md`, `Animation.md`, `Lighting.md`, `Camera.md`, `Physics.md`, `Render.md`, `Window.md`, `README.md`, `blood-effect-system.md`, `flying-system-architecture.md`, `monster-collision-system.md`, `map-editor-architecture.md`, `chat-bubble-and-name-tag-architecture.md`, `planar-water-reflection.md`, `sky_stars_architecture.md`, `SUN_DOCUMENTATION.md`, `zone_lighting.md`, `admin-menu-skill-learn-feature.md`.

**Gap:** there is **no** architecture doc for the gameplay core (network-message handling, combat/damage pipeline, cooldowns, status effects, projectiles, player commands). The only related docs are `blood-effect-system.md` (blood only) and the pitfalls entries `combat-sync.md`, `blood-effects.md`, `skill-bar-ui.md`. A `gameplay-core.md` doc would help future contributors understand the event chain (AnimationFrameEvent → HitEvent → PendingDamage/PendingSkillEffect → digits/blood/effects).

## 2. Methodology

- Read every source file in scope (full contents, including the 2933-line `game_connection_system.rs` in 3 passes).
- Read `system-architecture/ECS.md` (Bevy 0.18.1 ECS conventions used in this repo) and `pitfalls/index.md` + `combat-sync.md`, `blood-effects.md`, `skill-bar-ui.md`.
- Verified registration & ordering in `src/lib.rs` (system sets, `run_if` conditions, message registration).
- Perf analysis is static: hot loops, per-frame allocations, per-frame resource churn, log spam, and O(N×M) nested scans identified from source. No profiling data was available; severity estimates are relative.
- Bevy 0.18.1 source at `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1` used to confirm message/query semantics where relevant.

## 3. Findings

### A. Network-message handling (`game_connection_system.rs`)

**GC-01 — 2933-line god system: every server message handled in one sequential loop.**
- **Location:** `src/systems/game_connection_system.rs` (whole file; entry `game_connection_system`, writers declared at lines 314–323).
- **Description:** A single system reads the crossbeam channel and `match`es ~60 server message variants; 65+ handlers use `commands.queue(move |world: &mut World| { … })` deferred closures (lines 407, 530, 605, 692, 746, 787, 836, 921, 967, … through 2901).
- **Why it matters:** Every handler that mutates components runs as a deferred closure executed later by the command queue; entity lookups done twice (once for `client_entity_list.get(id)`, once inside the closure via `world.entity_mut`). The function is hard to navigate, and the closure-heavy style defeats Bevy's parallel-query safety analysis.
- **Fix sketch:** Split into per-message handler functions (one `match` arm → one `fn handle_xxx(…)`), and where a handler needs to mutate a single entity, use `commands.entity(entity).insert(…)` / `.get_mut` via `world.entity_mut` only when conditional reads are needed. Event-driven alternative: write a `ServerMessageEvent` message and let several small systems consume only the variants they care about (Bevy allows filtering a message by pattern in multiple readers).

**GC-02 — `commands.queue` closures re-query components that the outer system already has.**
- **Location:** e.g., `src/systems/game_connection_system.rs:2797` (`ClanUpdateInfo`), `2831` (`ClanMemberConnected`), `2901` (`ClanMemberList`).
- **Description:** Patterns like `commands.queue(move |world| { let mut entity_mut = world.entity_mut(player_entity); if let Some(mut clan) = entity_mut.get_mut::<Clan>() { … } })` re-fetch components that are cheap to mutate via a query in the outer system.
- **Why it matters:** Not a hot path (only on server messages), but it is the dominant pattern (~65 occurrences) and makes every handler 3–5 lines longer; compounded by GC-01.
- **Fix sketch:** Prefer direct `Query<(Entity, &mut Clan)>` in the outer system for the player entity and mutate immediately (safe because message processing is inherently sequential per frame). Reserve `commands.queue` for cases that need `world.entity_mut` on entities that could despawn this frame.

**GC-03 — All message handlers process synchronously in `Update`; a big batch of messages can extend frame time.**
- **Location:** `src/systems/game_connection_system.rs` entry (registered in `src/lib.rs:1656` with `run_if(resource_exists::<CurrentZone>)`).
- **Description:** `while … { match … }` drains the channel until `TryRecvError::Empty`; a zone join or batch of `DamageEntity`/spawn messages can deliver hundreds of messages in one frame (each spawning entities, loading assets).
- **Why it matters:** Spawn bursts cause frame hitches on zone entry / large monster pulls.
- **Fix sketch:** Cap messages processed per frame (e.g., 128) and re-run next frame (bounded latency ≤2 frames is invisible), or move purely data-driven handlers (e.g., clan/party bookkeeping) into their own schedule after the rendering-critical ones.

**GC-04 — Debug `log::info!` left in per-message paths.**
- **Location:** `src/systems/game_connection_system.rs:877` (`[ATTACK_DIAG] StopMoveEntity player`), `:815`/`:816` (`[RESPAWN_MOVE_DIAG]` warns).
- **Description:** `[ATTACK_DIAG]` info logs fire for every `StopMoveEntity` for the player; `[RESPAWN_MOVE_DIAG]` warns fire on every `Move` while no connection is present (`src/systems/player_command_system.rs:758`).
- **Why it matters:** `log::info!` formatting on the per-message path is wasted work in normal play and floods the structured session logs (`logs/<session>/structured.jsonl`).
- **Fix sketch:** Demote to `log::debug!` or gate behind a `Local<bool>` env-var check (pattern already used by `blood_overlay_force_enable_system`, `src/systems/blood_overlay_system.rs:235-248`).

### B. Player commands

**PC-01 — `PickupItem` scans every item drop linearly.**
- **Location:** `src/systems/player_command_system.rs:218-238`.
- **Description:** On each `SkillBasicCommand::PickupItem` the system iterates **all** `ItemDrop` entities (`query_dropped_items.iter()`) to find the nearest by squared distance.
- **Why it matters:** O(drops) per pickup; fine with a handful of drops, quadratic churn in busy grind spots / player-run stores.
- **Fix sketch:** Track the nearest drop incrementally in a resource updated by a `Changed<Position>`-filtered system, or pick via a coarse spatial hash of drop positions; at minimum, skip the scan when a pickup is already in flight (`Command::PickupItem` active).

**PC-02 — `PlayerCommandEvent` is cloned for every event.**
- **Location:** `src/systems/player_command_system.rs:165` (`let mut event = event.clone();`).
- **Description:** Every event is cloned before the `match` solely to rewrite `UseHotbar` into `UseSkill`/`UseItem` (lines 167–186).
- **Why it matters:** `PlayerCommandEvent` contains `ItemReference`, positions, and hotbar slots; per-keypress clone is wasteful (typing WASD = `Move` clones continuously).
- **Fix sketch:** Match on a reference first for `UseHotbar` (translate without moving), then `match event` by value for the rest, or handle `UseHotbar` in a separate small system that writes a new `PlayerCommandEvent`.

**PC-03 — Duplicated target-validation logic between skills and consumables.**
- **Location:** `src/systems/player_command_system.rs:375-394` vs `:504-522`.
- **Description:** The `query_skill_target` + `is_valid_skill_target` dance is repeated for `CastSkill` and for MagicItem consumables.
- **Why it matters:** Maintenance risk (validators drift apart — one already returns "Invalid target" via chatbox while the other silently drops).
- **Fix sketch:** Extract `fn resolve_skill_target(…) -> Option<ClientEntityId>` used by both paths.

**PC-04 — Move command spam: WASD move sends one message per keypress frame.**
- **Location:** `src/systems/game_keyboard_input_system.rs` (`WASD_MOVE_COMMAND_INTERVAL_SECS = 0.10`, `WASD_MOVE_COMMAND_LEAD_TIME_SECS = 0.25`).
- **Description:** Movement re-issues `PlayerCommandEvent::Move` at 10 Hz; each becomes a `ClientMessage::Move` over the channel.
- **Why it matters:** 10 msgs/s baseline + chatbox `"[RESPAWN_MOVE_DIAG]"` warns when disconnected (PC-04/GC-04); mostly a bandwidth/log concern, not CPU.
- **Fix sketch:** Throttle to server tick rate (5 Hz is enough for linear movement with interpolation); suppress sends while disconnected.

### C. Combat / damage pipeline

**CM-01 — `hit_frame_expected` scans all projectiles per pending-damage entry (O(N×M)).**
- **Location:** `src/systems/pending_damage_system.rs:34-74` (called from `:117-125` inside the per-entity, per-pending-damage loop `:108-179`).
- **Description:** For every pending damage with `is_kill`, the system iterates **all** `Projectile` entities (`query_projectiles.iter().any(…)`) to check whether a projectile from the attacker is still in flight toward the defender. With P pending damages and Q projectiles this is P×Q scans per frame, repeated until each kill resolves.
- **Why it matters:** Multi-target fights (AoE skills, volleys of arrows) make both P and Q grow; this is the most algorithmic hot spot in the damage pipeline.
- **Fix sketch:** Maintain a per-attacker map of in-flight projectiles (resource or `HashMap<Entity, Vec<ProjectileTarget>>` updated by `spawn_projectile_system`/`projectile_system`), or give each pending-damage entry a "checked" flag so the scan happens once per entry rather than every frame.

**CM-02 — `PendingDamageList` linear removal scan per hit event.**
- **Location:** `src/systems/hit_event_system.rs:67-85`; component cap in `src/components/pending_damage_list.rs` (cap 32).
- **Description:** Each `HitEvent` linearly scans the defender's pending list matching `attacker` + `skill_id`, removing matches. Cap of 32 bounds it, but `remove(i)` shifts.
- **Why it matters:** Bounded (≤32), so severity is low; the same attacker+skill match could be indexed.
- **Fix sketch:** Keep the cap; swap-remove instead of shift-remove (order does not matter here — entries are independent).

**CM-03 — Duplicated kill/death handling between `pending_damage_system` and `hit_event_system`.**
- **Location:** `src/systems/pending_damage_system.rs:145-153` vs `src/systems/hit_event_system.rs:98-109`.
- **Description:** Both systems insert `Dead`/`DeathBloodHandled`/`NextCommand::with_die()` and remove `ClientEntity` — but the **kill-damage digits** are spawned by `pending_damage_system` while the **kill spatter event** is emitted by `hit_event_system` (`emit_blood_and_wounds`), and both call `spawn_damage_digits` (the `hit_event_system` path only when a matching pending entry exists). The two code paths must stay exactly in sync — a subtle source of the desync class documented in `pitfalls/combat-sync.md`.
- **Why it matters:** Any drift reintroduces the delayed-death bug. Single-responsibility split would prevent it.
- **Fix sketch:** Consolidate: `pending_damage_system` decides *when* damage/kill is applied; `hit_event_system` only forwards visuals (digits, effects, blood) driven by the pending entries, not by re-evaluating state itself. Add one debug assert that both agree.

**CM-04 — `pending_skill_effect_system` iterates all combat entities every frame.**
- **Location:** `src/systems/pending_skill_effect_system.rs:219-267`.
- **Description:** The "apply expired skill effects" loop iterates every entity with `(AbilityValues, HealthPoints, Option<ManaPoints>, MoveSpeed, PendingSkillEffectList, StatusEffects)` every frame to age the pending list, even when the list is empty for nearly all entities.
- **Why it matters:** With many monsters in view this is a full per-frame pass over combat entities just to add `delta_time` to (mostly empty) lists.
- **Fix sketch:** Move aging into the list types (`PendingSkillEffectList::tick(delta)`) with a `Changed`-style dirty flag; only entities that had effects the previous frame need re-checking, and only the first list element's age matters for the 2 s expiry.

**CM-05 — Kill-wait interplay: `hit_frame_expected` defers kills during any attacker animation.**
- **Location:** `src/systems/pending_damage_system.rs:49-62`.
- **Description:** If the attacker's `Command` is `Attack(_) | CastSkill(_)` and its `SkeletalAnimation` is not completed, the kill waits (bounded by `KILL_MAX_DAMAGE_AGE = 1.5`).
- **Why it matters:** Correctness trade-off (documented in `pitfalls/combat-sync.md`); not a perf issue. Flagged so future changes keep the `1.5 s` cap — increasing it reopens the delayed-death bug.
- **Fix sketch:** No change; note in code comment.

### D. Status effects & cooldowns

**SE-01 — `status_effect_system` iterates every combat entity every frame.**
- **Location:** `src/systems/status_effect_system.rs:11-69` (registered `src/lib.rs:1558` inside `run_if(in_state(AppState::Game))`).
- **Description:** Every frame iterates all entities with `(AbilityValues, HealthPoints, Option<ManaPoints>, StatusEffects, StatusEffectsRegen)`; the per-second tick only gates the HP subtraction, not the iteration or the `get_status_effect` lookups (lines 45-46, 55-56).
- **Why it matters:** Full per-frame pass over all monsters/players when usually nobody is poisoned; `game_data.status_effects.get_status_effect(id)` HashMap lookups only on ticks, so the main cost is iteration.
- **Fix sketch:** Filter with `Changed<StatusEffects>` for the poison-processing system (a poison tick only matters when the list changed… it doesn't change on tick — so instead maintain a `Local<f32>` accumulator and only iterate when a second has elapsed). At minimum: only `StatusEffectsRegen`/`StatusEffects` are needed in the query; drop the other components.

**SE-02 — `visible_status_effects_system` uses `Changed<StatusEffects>` — good, but re-spawns effects on every change.**
- **Location:** `src/systems/visible_status_effects_system.rs:15-68`.
- **Description:** Uses `Changed<StatusEffects>` (efficient); on each change despawns and re-spawns the effect entity per active effect (lines 34, 43-61, 64-66).
- **Why it matters:** Applying/re-applying status effects (e.g., repeated poison refreshes) churns effect spawn/despawn entities every message; also `StatusEffects` changes on server HP messages? (No — `StatusEffects` only changes on effect apply/expire, so this is acceptable.) Severity low.
- **Fix sketch:** Keep as is; optionally reuse the effect entity and only replace the `SpawnEffect` data when the file id changed.

**CD-01 — `cooldown_system` walks HashMaps with per-frame allocation-free decrements (fine).**
- **Location:** `src/systems/cooldown_system.rs:5-47`.
- **Description:** Four loops decrementing `(Duration, Duration)` tuples; no allocations; typically one player entity.
- **Why it matters:** Negligible; **not a finding to act on**. Listed to document review coverage.

**CD-02 — Cooldown decrement is frame-rate dependent.**
- **Location:** `src/systems/cooldown_system.rs:9-44`.
- **Description:** `delta = time.delta()` decrements in real time; at 30 fps vs 144 fps the perceived cooldown duration is identical (real seconds) — correct. No action needed.

### E. Effects & projectiles

**EF-01 — `spawn_effect_system` logs `info!` for every effect spawn (log spam + formatting cost).**
- **Location:** `src/systems/spawn_effect_system.rs:50` (`log::info!("[SPAWN EFFECT SYSTEM] Processing event: {:?}", event)`), plus `:56`, `:83` inside each branch.
- **Description:** Every `SpawnEffectEvent` — one per hit, skill cast stage, status effect, consumable use, vehicle effect — is formatted with `Debug` and written to the log at info level. During combat this is dozens of log lines/second.
- **Why it matters:** Debug formatting of events allocates per hit; session logs balloon; this is the single highest-frequency log statement in the gameplay systems.
- **Fix sketch:** Remove or demote to `log::trace!`/`log::debug!` (consistent with `animation_effect_system.rs:49,197,210` which use `debug`). Keep only the `warn!` branches (`:75`).

**EF-02 — `effect_system` does nested query lookups per child per frame.**
- **Location:** `src/systems/effect_system.rs:19-60`.
- **Description:** Per effect entity: iterate `Children`, then for each child query `query_children` again, then `query_particle_sequence.get(child)` and `query_effect_mesh.get(child)` (4 query gets per grandchild). Effects with many particles (explosions, AoE) iterate the whole subtree every frame until finished.
- **Why it matters:** The dominant per-frame effect cost; several concurrent effects × dozens of children × 4 lookups/frame.
- **Fix sketch:** Query the whole tree in one pass via `Query<&Children>` iteration with an explicit work stack instead of recursive `get`, or track finish state directly on the effect entity via events (`ParticleSequence` finished → `EffectFinishedEvent`) rather than polling children each frame.

**EF-03 — `animation_effect_system` is event-driven (good), but resolves motion/effect data via DB lookups per event.**
- **Location:** `src/systems/animation_effect_system.rs:33-448`.
- **Description:** Reads `AnimationFrameEvent` (event-driven, excellent); each flagged frame does `game_data.skills.get_skill(id)` / `items.get_weapon_item(...)` / `npcs.get_npc_motion(...)` lookups.
- **Why it matters:** Frequency = only on animation hit frames — acceptable. No action.

**PR-01 — `projectile_system` re-inserts the transform via commands every frame instead of querying `&mut Transform`.**
- **Location:** `src/systems/projectile_system.rs:17-28` (query `(Entity, &mut Projectile, &Transform)`), `:124-127` (`commands.entity(entity).insert(transform)`).
- **Description:** The system reads `&Transform`, computes the new transform, then issues a deferred `insert` per projectile per frame. Also does 2 extra queries per projectile per frame (`query_skeleton.get` + `query_global_transform.get`, lines 26-37).
- **Why it matters:** Deferred command allocation per frame per projectile; forces sequential apply; double query lookup for the same target.
- **Fix sketch:** Query `(Entity, &mut Projectile, &mut Transform)` and mutate in place; cache target translation in the `Projectile` component (invalidated only when target entity changes) to avoid per-frame skeleton/transform lookups.

**PR-02 — Parabola normalization edge case: zero-distance target.**
- **Location:** `src/systems/projectile_system.rs:56-79`.
- **Description:** If target ≈ source (point-blank bow shot), `distance` is ~0; `move_vec` normalization is degenerate; `velocity_y = travel_time * 98 / 2` with `travel_time ≈ 0` — the projectile completes instantly in practice, but NaN risk exists if `distance == 0.0` exactly (`normalize` of zero vector).
- **Why it matters:** Rare NaN could corrupt a transform (invisible projectile) — correctness, not perf.
- **Fix sketch:** Early-out `if distance < f32::EPSILON { complete = true }` before computing the parabola.

### F. Damage digits

**DD-01 — Per-digit-entity per-frame GPU resource churn: 3 storage buffers recreated and removed every frame.**
- **Location:** `src/systems/damage_digit_render_system.rs:145-171`.
- **Description:** For **every active damage-digit entity, every frame**: `storage_buffers.add(ShaderStorageBuffer::from(Vec…))` ×3 (positions/sizes/uvs), material fields swapped, then `storage_buffers.remove(&old_*)` ×3. Each `add` allocates a new GPU buffer and uploads data; each `remove` frees it. With N simultaneous digit popups (multi-hit monsters, AoE), that's 3N allocations + 3N frees + 3N uploads per frame.
- **Why it matters:** Highest-frequency GPU-resource churn in the gameplay systems; completely avoidable — the buffers exist solely to hold ≤10 quads of digit data.
- **Fix sketch:** (a) Use `storage_buffers.get_mut(handle)` and write into the existing buffer in place (Bevy re-uploads the buffer); (b) or use a single shared static-capacity buffer (e.g., 32 digits × 4 quads) with per-entity offsets — one buffer for all digit entities, upload only when digit data changes (`Changed<DamageDigitRenderData>`).

**DD-02 — Each digit entity gets its own unique 60-vertex mesh.**
- **Location:** `src/systems/damage_digit_render_system.rs:39-53`.
- **Description:** Every digit popup creates a new `Mesh` (60 zero-position vertices, `PrimitiveTopology::TriangleList`) via `meshes.add(mesh)`.
- **Why it matters:** Mesh asset churn per popup; all digit meshes are identical (vertex positions are procedural via `@builtin(vertex_index)`).
- **Fix sketch:** Share a single mesh handle from `DamageDigitsSpawner` (which already owns `mesh: Handle<Mesh>` — it's currently unused for this path since `damage_digits_spawner.rs:31` adds a 1×1 rectangle, and `create_damage_digit_material_system` ignores it).

**DD-03 — Damage digits spawn two entities + `PendingDamageDigitMaterial` round-trip.**
- **Location:** `src/resources/damage_digits_spawner.rs:71-93`; `src/systems/damage_digit_render_system.rs:24-61`.
- **Description:** `spawn()` creates a child (with `PendingDamageDigitMaterial`) and a parent; a separate system then converts pending→real material+mesh next frame (one-frame delay before digits show).
- **Why it matters:** Minor one-frame latency + 2-entity overhead per popup. Acceptable, but the pending indirection exists only because `Spawner` avoids creating materials directly.
- **Fix sketch:** Have `DamageDigitsSpawner` create the material + shared mesh directly at spawn (resources already hold handles) and drop the `PendingDamageDigitMaterial` stage.

### G. Blood effects

**BL-01 — `blood_spatter_spawn_system` counts active spatters by scanning all of them, per event (O(events×spatters)).**
- **Location:** `src/systems/blood_spatter_system.rs:159-162` (per-frame full `query_spatters.iter().filter(active).count()`) and `:221-241` (eviction `min_by` scan of all active spatters for the oldest, inside the per-event loop).
- **Description:** For every `SpawnSpatter` event in a frame (AoE kill = many), the system re-counts all active spatters and, when at cap, re-scans all to evict the oldest.
- **Why it matters:** With `max_spatters = 100` and burst events this is O(events × 100) + the `min_by` comparisons — the blood system's hot spot.
- **Fix sketch:** Track `active_count` in `BloodEffectRuntime` (increment on spawn, decrement on return) — the runtime resource already exists; use a `Vec<Entity>` insertion-ordered or heap-ordered pool so "oldest" is O(1) instead of O(N) min-scan.

**BL-02 — `blood_overlay_generate_system` recursively walks the model subtree every frame for every bloodied entity.**
- **Location:** `src/systems/blood_overlay_system.rs:101-147` — `collect_material_entities_recursive` (lines 27-62) runs **before** the clean/dirty check (line 128), so even clean overlays pay the recursive material collection each frame (query_children + query_materials gets per node).
- **Why it matters:** Bloodied characters stay bloodied for the whole session; this is a per-frame tree walk over every model part per bloodied entity, including the `HashMap::clone()` of per-material textures at line 200 on dirty frames.
- **Fix sketch:** Move the recursive collection inside the dirty branch; only re-bind overlay handles when `Changed<BloodOverlayTextures>` or when material handles change (compare handle ids, cheap). Only re-paint textures when `texture_dirty`.

**BL-03 — `wound_visibility_system` iterates every HP-bearing entity every frame.**
- **Location:** `src/systems/gash_wound_system.rs:84-150` (query: `(Entity, &HealthPoints, &AbilityValues, …)` `Without<Dead>`).
- **Description:** Every frame computes `health_percent` for all alive monsters/players to toggle wound visibility.
- **Why it matters:** Full combat-entity pass per frame; `HealthPoints` changes only on damage/heal.
- **Fix sketch:** Use `Changed<HealthPoints>` filter; toggling visibility only matters when HP crosses the threshold.

**BL-04 — `wound_spawn_system` runs `project_world_to_uv` (triangle-ray vs skinned mesh) per wound.**
- **Location:** `src/systems/gash_wound_system.rs:228-251` → `uv_projection::project_world_to_uv` (in `src/systems/uv_projection.rs`), invoked for every `ShowWound` event on `CharacterModel` entities (2–3 wounds per hit, `damage_effects.rs:77-86`).
- **Description:** Accurate-but-expensive: transforms skinned vertices and ray-intersects triangles per wound.
- **Why it matters:** Per-hit cost on player/character hits; wounds are capped at `max_wounds_per_entity = 4` so repeated hits converge to 0 new wounds, but every hit still attempts the projection.
- **Fix sketch:** Skip the projection when the entity is at the wound cap (`overlay.stain_count() >= max` check exists at line 254 but the expensive projection at 231-251 runs before it); move the cap check before the projection.

**BL-05 — `blood_spatter_fade_system` writes to decal materials every frame per spatter.**
- **Location:** `src/systems/blood_spatter_system.rs:392-430` (`decal_materials.get_mut(&material_handle.0)` per active spatter per frame).
- **Description:** Each active spatter's material `base_color` is recomputed and written each frame (alpha + wet→dry blend).
- **Why it matters:** 100 material writes/frame with `max_spatters=100`; materials are shared per decal so this is 100 distinct materials — GPU-side update cost each frame.
- **Fix sketch:** Update material only when alpha actually changes (e.g., every 0.1 s, or on a per-spatter dirty flag); the wet→dry color is smooth but 10 Hz would be visually identical.

**BL-06 — Dirt-dash particle count scanned twice per frame + `thread_rng` per frame.**
- **Location:** `src/systems/dirt_dash_system.rs:77` (`particle_count.iter().count()`), `:110` (again inside the burst loop), `:74` (`rand::thread_rng()`).
- **Description:** Full particle query count twice per frame (once up front, once per burst), plus thread-local RNG acquisition per frame.
- **Why it matters:** O(particles) ×2 per frame; minor but trivially avoidable.
- **Fix sketch:** Count once per frame; decrement a `Local` counter as particles are spawned/despawned (despawn count needs tracking — use `Changed`/`RemovedComponents` or a shared resource counter). Reuse a `Local<ThreadRng>`.

**BL-07 — `BloodEffectConfig::low_intensity()` is dead-inconsistent with defaults (`intensity: 0.3`).**
- **Location:** `src/resources/blood_effect_config.rs:150-162`.
- **Description:** `low_intensity` sets `intensity: 0.3` but `default()` is `1.5`; `high_intensity` sets `1.0`. The intensity scale is inverted/confusing (default 1.5 > "high" 1.0) and `low_intensity` isn't wired anywhere.
- **Why it matters:** Config maintenance hazard; someone enabling `low_intensity` on low-end GPUs would get *fewer* spatters but a different color scale than intended.
- **Fix sketch:** Normalize scale (default 1.0, low 0.5, high 1.5) or document intent; remove unused presets or wire them to a CLI flag/quality setting.

**BL-08 — `rand::random::<usize>() % len` modulo bias in texture pick.**
- **Location:** `src/systems/blood_spatter_system.rs:44` (`pick_spatter_texture`).
- **Description:** `rand::random::<usize>() % atlas.spatter_textures.len()` — modulo bias; visually irrelevant (8 variants).
- **Why it matters:** Negligible; listed for completeness. Fix with `rand::Rng::gen_range` if touched.

### H. Registration & scheduling (`src/lib.rs`)

**RG-01 — Combat/effect systems run in `Update` without `in_state(AppState::Game)` gating.**
- **Location:** `src/lib.rs:1098-1111` (`animation_effect_system`, `projectile_system`, `spawn_projectile_system`), `:1113-1151` (`pending_damage_system`, `pending_skill_effect_system`, `hit_event_system`, `spawn_effect_system`, `visible_status_effects_system`, `damage_digit_*`, `name_tag_*`).
- **Description:** Unlike "Game systems - part 1/2" (lines 1391-1428, 1554-1563, both `run_if(in_state(AppState::Game))`), these systems run in menu/login/model-viewer states too (queries mostly empty, so cost is small, but message events could leak: e.g., `AnimationFrameEvent` produced in menus).
- **Why it matters:** Wasted schedule slots outside Game + potential cross-state event leakage; minor.
- **Fix sketch:** Add `.run_if(in_state(AppState::Game))` to the effect-set registration, matching part 1/2.

**RG-02 — Blood systems run in `PostUpdate` without state gating.**
- **Location:** `src/blood_effect_plugin.rs` (`BloodSpatterPlugin`, `GashWoundPlugin`, `BloodOverlayPlugin` register in `PostUpdate`; also `DirtDashPlugin` in `Update`, `src/systems/dirt_dash_system.rs:26-28`).
- **Description:** `blood_spatter_on_death_system` / `wound_visibility_system` / `blood_overlay_generate_system` iterate every frame regardless of app state.
- **Why it matters:** In menus these query mostly-empty world — trivial; the bigger cost is in-game and covered by BL-02/BL-03.
- **Fix sketch:** Gate with `run_if(in_state(AppState::Game))` for consistency.

## 4. Priority Summary

| # | Finding | Priority | Effort | Category |
|---|---|---|---|---|
| DD-01 | Per-frame storage-buffer churn per digit entity (3 add + 3 remove per frame) | **High** | Medium | GPU/CPU churn |
| EF-01 | `log::info!` per effect spawn (Debug format + log flood) | **High** | Trivial | Log spam |
| CM-01 | `hit_frame_expected` scans all projectiles per pending kill (O(N×M)) | **High** | Medium | Algorithmic |
| GC-01 | 2933-line god system with 65 deferred closures | **High** (maintainability) | Large | Structure |
| BL-02 | Per-frame recursive material walk for every bloodied entity | **High** | Medium | Algorithmic |
| BL-04 | Per-hit triangle-ray UV projection runs even at wound cap | Medium | Trivial | Algorithmic |
| PR-01 | Per-frame `commands.insert(transform)` + 2 extra queries per projectile | Medium | Trivial | Per-frame alloc |
| SE-01 | Per-frame full combat-entity iteration in `status_effect_system` | Medium | Trivial | Algorithmic |
| CM-04 | Per-frame full combat-entity iteration in `pending_skill_effect_system` | Medium | Trivial | Algorithmic |
| BL-03 | Per-frame full HP-entity iteration in `wound_visibility_system` | Medium | Trivial | Algorithmic |
| BL-01 | O(events×spatters) counting + O(N) eviction scan | Medium | Medium | Algorithmic |
| BL-05 | Per-frame material write per active spatter | Medium | Trivial | GPU updates |
| BL-06 | Dirt-dash particle counted twice/frame + thread_rng | Medium | Trivial | Per-frame alloc |
| DD-02 | Unique 60-vertex mesh per digit entity | Low-Medium | Trivial | Asset churn |
| DD-03 | Two-entity spawn + one-frame pending material round-trip | Low | Trivial | Latency |
| PC-01 | Linear scan of all item drops per pickup | Low-Medium | Small | Algorithmic |
| PC-02 | Clone of every `PlayerCommandEvent` | Low | Trivial | Per-input alloc |
| EF-02 | Nested 4-query lookups per effect child per frame | Low-Medium | Medium | Algorithmic |
| GC-04 / PC-04 | `[ATTACK_DIAG]`/`[RESPAWN_MOVE_DIAG]` info/warn spam | Low | Trivial | Log spam |
| RG-01 | Effect systems ungated in Update | Low | Trivial | Schedule |
| BL-07 | Confusing blood intensity presets | Low | Trivial | Config |
| CM-02 | Shift-remove in pending list | Low | Trivial | Micro |
| CM-03 | Duplicated kill/death logic in two systems | Medium (correctness) | Medium | Structure |

## 5. Quick Wins (do these first)

1. **Demote `spawn_effect_system` info logs** (`src/systems/spawn_effect_system.rs:50,56,83`) to `debug!` — instant log/perf win during combat.
2. **Demote `[ATTACK_DIAG]`/`[RESPAWN_MOVE_DIAG]` logs** (`command_system.rs:894,1128`, `game_connection_system.rs:877`, `player_command_system.rs:758`).
3. **Digit buffers in place** — use `storage_buffers.get_mut` instead of `add`/`remove` each frame (`damage_digit_render_system.rs:145-171`).
4. **`Changed<HealthPoints>`** on `wound_visibility_system` (`gash_wound_system.rs:103`).
5. **Move the wound-cap check before UV projection** (`gash_wound_system.rs:231` vs `:254`).
6. **Count dirt particles once per frame** (`dirt_dash_system.rs:77,110`).
7. **Mutate projectile transform in place** (`projectile_system.rs:124-127`).
8. **Gate effect systems with `in_state(AppState::Game)`** (`src/lib.rs:1098-1151`).

## 6. Risks & Considerations

- **Combat-sync regression risk (highest):** any refactor of `pending_damage_system` / `hit_event_system` / `pending_skill_effect_system` can reintroduce the delayed-death desync documented in `pitfalls/combat-sync.md`. The 0.25 s grace, 1.5 s kill cap, and `distance <= attack_range` (inclusive, `command_system.rs:910`) are load-bearing invariants — preserve them and re-test one-hit kills, projectile kills, and AoE multi-kills.
- **`commands.queue` closures are load-bearing** for despawn safety in `game_connection_system.rs`: converting to direct queries (GC-02) is safe only where the target entity cannot be despawned in the same frame; do it incrementally with tests.
- **Bevy 0.18.1 specifics:** messages (`MessageReader`/`MessageWriter`) replaced the old `EventReader`/`EventWriter` API — any new event-writer code must follow the repo pattern (`derive(Message)`, `#[derive(Message)]` in `src/events/*.rs`). Check `system-architecture/ECS.md` for the canonical examples before editing.
- **Storage buffer in-place update (DD-01)** — verify with the digit shader (`src/render/damage_digit_material.rs`, weak shader handle `6a4b5c6d-…`): it binds `positions`/`sizes`/`uvs` buffers by handle; `get_mut` on the same handle re-uploads data without rebinding, which is compatible — but confirm `ShaderStorageBuffer` dirty-flag behavior in Bevy 0.18.1 before committing to it.
- **Layered blood diagnostics** (`enable_diagnostics`) logs every 5 s (`blood_spatter_system.rs:432-445`) — keep disabled in release.
- **Scope note:** `move_speed_set_system.rs:19-22` logs at `info!` per event — same log-spam class as GC-04, fix alongside.
- **No profiling data** was used; findings are from static analysis. Priority assumes typical gameplay (1 player + tens of monsters + occasional AoE). Measure after applying high-priority items.
