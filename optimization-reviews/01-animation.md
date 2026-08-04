# Optimization Review 01 — Animation & Character Model Rendering Subsystem

**Repo:** `rose-offline-client` (single crate, Bevy 0.18.1, bevy_rapier3d 0.33, bevy_egui 0.39, egui 0.33, wgpu v27)
**Date:** 2026-08-03
**Reviewer role:** research/writing only — no game code was modified, no build was run.

## 1. Scope

### Files covered

- `src/animation/` — `animation_state.rs`, `camera_animation.rs`, `mesh_animation.rs`, `mod.rs`, `skeletal_animation.rs`, `transform_animation.rs`, `zmo_asset_loader.rs`
- `src/model_loader.rs` (~1443 lines) — character/NPC/vehicle/item model spawning & caching
- `src/zms_asset_loader.rs` — ZMS → Bevy `Mesh` conversion (incl. load-time normal smoothing)
- `src/render/skinned_mesh_fix.rs` — deferred `SkinnedMesh` insertion
- `src/systems/` — `character_model_system.rs`, `character_model_add_collider_system.rs`, `character_model_blink_system.rs`, `npc_model_system.rs`, `npc_model_add_collider_system.rs`, `vehicle_model_system.rs`, `wing_spawn_system.rs`, `sail_animation_system.rs`, `animation_effect_system.rs`, `animation_sound_system.rs`, plus animation-consuming systems (`command_system.rs`, `npc_idle_sound_system.rs`, `pending_damage_system.rs`, `character_select_system.rs`, `model_viewer_system.rs`)
- `src/components/` — `character_model.rs`, `character_model_blink_timer.rs`, `npc_model.rs`, `skinned_mesh_target_bone.rs`, `skinning_target.rs`, `blink_clip.rs`, `dummy_bone_offset.rs`, `model_height.rs`, `vehicle_model.rs`
- `src/bundles/ability_values.rs` (checked; only feeds attack-animation speed via `get_attack_animation_speed` in `command_system.rs:298`, no direct animation interaction)
- Supporting: `src/render/effect_mesh_extension.rs`, `src/render/object_material_extension.rs`, `src/render/trail_effect.rs` (rendering disabled — component only), `src/effect_loader.rs`, `src/zone_loader/spawning/objects.rs`, `src/vfs_asset_io.rs`

### Architecture docs that existed

- `system-architecture/Animation.md` — **exists and is detailed** (ZMO format, sampling, event system, blending, command-based state machine). Read in full.
- `system-architecture/ECS.md` — skimmed (no animation-specific content beyond generic Bevy ECS references).
- `system-architecture/Assets.md` — read (VFS cache, asset loaders, EffectCache).
- Pitfalls read: `pitfalls/index.md`, `pitfalls/model-viewer.md`, `pitfalls/performance-memory.md` (the last one is directly relevant: **"Every-frame asset creation leaks"** — pattern found again here in a variant: *every-frame asset mutation* → per-frame GPU re-preparation).

## 2. Methodology

1. Read the two arch docs + pitfalls index and the three relevant pitfall entries first (per instructions).
2. Read every listed source file end-to-end (multiple passes for `model_loader.rs`, `command_system.rs`, `game_connection_system.rs`).
3. Traced callers/consumers via grep (e.g., all `SkeletalAnimation` usages, all `blink_state` writes, all `BlinkClipState` writes, `Npc` mutation sites) to distinguish event-driven from per-frame work.
4. Validated Bevy 0.18.1 APIs against `bevy-collection\bevy-0.18.1\crates\`:
   - `bevy_asset/src/assets.rs:440-454` — `Assets::get_mut` emits `AssetEvent::Modified` on drop (confirmed: mutating a material/mesh asset every frame re-syncs it to the render world).
   - `bevy_render/src/extract_component.rs:86-122` — `ExtractComponentPlugin` extraction iterates **all** matching entities every frame (no `Changed` filter), with `try_insert_batch` (value-equal inserts are no-ops in the render world, but the main-world write still happens).
   - `bevy_ecs/src/system/query.rs:891,936` — `Query::iter_many` / `iter_many_mut` exist in 0.18 (used in suggested fix F2).
   - `bevy_time` — `Time::delta_secs()`, `Timer` semantics confirmed (used by blink/timer systems).
5. No code was changed; no `cargo build` was run; no game run.

---

## 3. Findings

> Naming: each finding lists `file:line` refs, impact estimate, and a concrete fix sketch.

---

### F1. [HIGH] All animated characters/NPCs are fully updated every frame — no visibility or distance culling

**Files:** `src/animation/skeletal_animation.rs:37-152` (bone writes), `src/animation/mesh_animation.rs:39-92`, `src/animation/transform_animation.rs:25-88`, `src/animation/camera_animation.rs:26-105`

**Description:** All four animation systems run in `PostUpdate` (registered in `mod.rs:50-63`) and iterate **every** entity with an animation component every frame. There is no `ViewVisibility`/frustum check and no distance check. Idle/stop animations are infinite loops (`SkeletalAnimation::repeat(motion, None)` from `command_system.rs:290`), so `completed()` is almost never true and the per-bone work runs unconditionally. Off-screen NPCs, characters behind the camera, and entities in entirely different zones (character-select screen, map editor) keep animating.

**Impact:** A busy town/field with ~150–300 animated entities (static NPCs + monsters + players) × ~40–50 bones each ⇒ ~7k–13k bone `Transform` writes **plus** the resulting hierarchy propagation (see F2) every single frame, regardless of what is on screen. This is the dominant CPU cost of the animation subsystem. Extrapolated: 400k–800k transform writes/sec at 60 fps.

**Fix sketch (per system, e.g. skeletal):**
```rust
fn skeletal_animation_system(
    mut query_animations: Query<(Entity, &mut SkeletalAnimation, Option<&SkinnedMesh>, Option<&ViewVisibility>)>,
    ...
) {
    for (entity, mut skeletal_animation, skinned_mesh, view_visibility) in query_animations.iter_mut() {
        if view_visibility.is_some_and(|vv| !vv.get()) { continue; } // off-screen: skip entirely
        ...
    }
}
```
The character/NPC root entities carry `ViewVisibility` (spawn bundles in `game_connection_system.rs:115,424` and `model_viewer_system.rs:301-304`). Optionally add a squared-distance check against the camera for very large zones. The same filter can be applied to `mesh_animation_system` (effect meshes) and `transform_animation_system` (zone animated objects).

---

### F2. [HIGH] Per-bone `Query::get_mut` + unconditional bone writes force full transform propagation for thousands of bone entities

**Files:** `src/animation/skeletal_animation.rs:104-147`

**Description:** For each animated entity, the system loops `skinned_mesh.joints` and issues a separate `query_transform.get_mut(*bone_entity)` (line 105) — one archetype lookup per bone, scattered across many archetypes. Every bone `Transform` is then *written unconditionally* every frame (translation + rotation), which flags every bone entity `Changed<Transform>` → `TransformSystems::Propagate` walks the whole skeleton hierarchy (bones + mesh-part children) each frame. Additionally each bone entity carries `Visibility/InheritedVisibility/ViewVisibility` (`model_loader.rs:1109-1119`), so per-frame visibility propagation also re-walks them.

**Impact:** With N animated entities: N×~45 archetype lookups + N×~45 component writes + a full transform-propagation tree walk of N×~55 entities per frame. The propagation cost scales quadratically with the number of animated bones in the scene and is pure waste whenever the skeleton pose did not change (it always changes here because of F1).

**Fix sketch:**
- Batch with `iter_many_mut` (verified available, `bevy_ecs/src/system/query.rs:936`):
```rust
for (bone_id, (mut bone_transform,)) in
    query_transform.iter_many_mut(skinned_mesh.joints.iter().copied())
{
    // sample + lerp, write
}
```
- Skip the write when the sampled value equals the current value (frame hasn't advanced; at high fps several frames share one animation frame): cheap `abs_diff_eq` guard before assignment. This alone stops the `Changed<Transform>` storm on ~fps/animation_fps of the frames.
- Long-term (big refactor): stop animating through the ECS hierarchy — write skinning matrices directly to a per-entity storage buffer (Bevy's skinning pipeline currently reads joint `GlobalTransform`s; a custom render path with a `ShaderStorageBuffer` pose upload would remove ~55 entities × N of propagation work entirely). This is a render-pipeline change, high effort, high reward.

---

### F3. [HIGH] `command_system` writes `SkeletalAnimation` every frame for every idle/moving entity (change-detection churn)

**Files:** `src/systems/command_system.rs:273-296` (`update_active_motion`), called per frame from `update_stop_motion` (lines 632-644, 667-681)

**Description:** For every entity in the `Command` query, when the command is `Stop` (the common idle case), `update_stop_motion` → `update_active_motion` runs every frame. In `update_active_motion`, when the same motion is already playing:
```rust
if active_motion.motion().id() == motion.id() && !active_motion.completed() {
    active_motion.set_animation_speed(animation_speed);  // unconditional &mut write
    return;
}
```
`set_animation_speed` (`animation_state.rs:110-112`) writes `self.animation_speed = ...` **every frame even when the value is unchanged**, marking the component `Changed`. Worse, for entities whose desired motion is `Handle::default()` (missing animation data), `update_active_motion` **re-inserts a brand-new `SkeletalAnimation` every frame** (line 288-295).

**Impact:** Every idle character/NPC/vehicle gets a pointless component write per frame ⇒ `SkeletalAnimation` change ticks fire for the whole population each frame. This defeats any future `Changed<SkeletalAnimation>`-filtered system, adds cache pressure, and the default-handle path performs a component replace per frame (archetype write + change event).

**Fix sketch:**
```rust
impl AnimationState {
    pub fn animation_speed(&self) -> f32 { self.animation_speed }
}
// in update_active_motion:
if active_motion.motion().id() == motion.id() && !active_motion.completed() {
    if active_motion.animation_speed() != animation_speed {
        active_motion.set_animation_speed(animation_speed);
    }
    return;
}
if motion.is_strong() {  // skip insert for Handle::default()
    entity_commands.insert(...);
}
```

---

### F4. [HIGH] `mesh_animation_system` mutates material assets every frame → per-frame render-world re-prepare (bind group + uniform re-upload) per animated effect

**Files:** `src/animation/mesh_animation.rs:79-90` (`effect_mesh_materials.get_mut`), `95-133` (`update_effect_mesh_animation_material`)

**Description:** Every frame, for every `EffectMesh` with a material, the system does `effect_mesh_materials.get_mut(&material_handle)` and rewrites `extension.animation_state`. In Bevy 0.18, `Assets::get_mut` marks the asset modified (`bevy_asset/src/assets.rs:440-454`), which re-syncs the material to the render world → `prepare_assets::<Material>` re-runs → bind-group re-creation + uniform-buffer re-upload **per animated effect mesh per frame**. The function also recomputes the constant `flags` field every frame (`flags |= (num_frames as u32) << 4`, line 122, plus channel booleans) even though it never changes for a given ZMO.

**Impact:** In combat with 10–100 animated effect meshes (skills, hits, zone morph objects), that is 10–100 GPU bind-group recreations + uniform uploads per frame, plus CPU-side `Assets`/event churn. Zone morph objects (`zone_loader/spawning/objects.rs:386-404`) run forever (repeat, no limit) and are always animating — including many that may be off-screen (see F1).

**Fix sketches (composable):**
1. Precompute `flags` once at load time and store on `ZmoAsset` (or on the material at spawn); skip recompute per frame.
2. Early-out when nothing changed: `if uniform.current_next_frame == new && uniform.next_weight == new_weight { return; }` — many entities share the same frame across high-fps frames.
3. Structural (medium effort): move the 4-field `EffectMeshAnimationUniform` out of the material into a per-entity uniform (custom `ExtractComponent` + `MeshUniform`-style buffer or a `ShaderStorageBuffer`), so the material asset is never mutated per frame and only a 16-byte per-entity upload happens on actual changes.

---

### F5. [HIGH] `sail_animation_system` rewrites the whole sail `Mesh` vertex buffer on CPU every frame

**Files:** `src/systems/sail_animation_system.rs:81-119`

**Description:** For every active sail, each frame the system mutates `Mesh::ATTRIBUTE_POSITION` in-place through `meshes.get_mut(&mesh_3d.0)` (line 81) and rewrites every vertex position (lines 91-119). Because `Assets<Mesh>::get_mut` fires `AssetEvent::Modified`, the render world re-extracts and **re-uploads the entire vertex buffer to the GPU every frame per sail** — even though only the Z offset changes and the base mesh is static.

**Impact:** Each sail = one full VBO upload per frame (e.g., 2–8k vertices × 12 bytes = 24–96 KB GPU upload/frame/sail). With a boat + several remote boats, this is sustained per-frame transfer and CPU memcpy work; it also re-runs mesh `prepare` (possibly including BVH/tangent re-preparation) on every sail mesh.

**Fix sketch:** Move the deformation to the vertex shader:
- Keep the static base mesh (positions = `base_positions`).
- Pass per-sail uniforms (billow, luff, side, time) — e.g., a tiny custom material or `ExtractComponent` + storage buffer.
- Compute the parabola/triangle/sin offsets in the shader; zero per-frame CPU mesh mutation.
- Fallback if shader change is undesired: quantize updates to e.g. 10–15 Hz (`Timer`/frame counter) to cut uploads 4–6×.

---

### F6. [MED-HIGH] `sync_blink_clip_to_state` unconditionally re-inserts `BlinkClipState` every frame for every face mesh

**Files:** `src/components/blink_clip.rs:50-69` (plugin + sync system), `72-80` (`update_blink_clip_state` — dead, never registered)

**Description:** `BlinkClipPlugin` registers `sync_blink_clip_to_state` (line 55), which for **every** entity with `BlinkClip + Mesh3d` executes `commands.entity(entity).insert(state)` **every frame** — even though `BlinkClip` changes only when the character blinks (a few times a minute). The insert is value-identical but still: a `Commands` write, a component-storage write, a change tick, render-world sync, and `ExtractComponentPlugin` extraction of the component (which iterates all such entities every frame — `bevy_render/src/extract_component.rs:86-122`). Additionally `update_blink_clip_state` (a `Changed`-sensitive variant) exists but is never added to the app — dead code.

**Impact:** 1 pointless insert + change tick + extract pass per character face mesh per frame (~1 write × N faces × 60 fps). Small in absolute terms, but it is 100% waste and pollutes change detection.

**Fix sketch:**
```rust
fn sync_blink_clip_to_state(
    mut commands: Commands,
    query: Query<(Entity, &BlinkClip), (With<Mesh3d>, Changed<BlinkClip>)>,
) { ... } // unchanged body
```
(`Changed<BlinkClip>` filter; the insert only fires on real blink transitions. Optionally remove the dead `update_blink_clip_state`.)

---

### F7. [MED] `SkinnedMeshInverseBindposes` asset duplicated per character/NPC/vehicle spawn

**Files:** `src/model_loader.rs:1083-1203` (`spawn_skeleton`, esp. 1151, 1183-1184)

**Description:** `spawn_skeleton` recomputes the bind pose, inverts ~45 matrices, and calls `skinned_mesh_inverse_bindposes_assets.add(...)` for **every** character, NPC, and vehicle spawn. The skeletons are drawn from a small set of shared files (`MALE.ZMD`, `FEMALE.ZMD`, `CART01.ZMD`, `CASTLEGEAR02.ZMD`, and per-NPC skeletons). Every character with the same gender gets a byte-identical inverse-bind-pose asset; every NPC of the same type too.

**Impact:** Zone load with 100+ characters/NPCs ⇒ 100+ duplicate `SkinnedMeshInverseBindposes` assets (~2.5 KB each + asset-slot churn) and redundant matrix-inverse CPU work per spawn. Memory grows with population, not with unique skeletons.

**Fix sketch:** Cache by skeleton identity in `ModelLoader`:
```rust
inverse_bindposes_cache: HashMap<u32 /*skeleton file hash or gender id*/, Handle<SkinnedMeshInverseBindposes>>,
```
Return the cached `Handle` (cloned) on subsequent spawns; only compute + `add` on cache miss. Since the handles are strong handles, the asset stays alive for the whole session — correct, because bones are per-entity but the bind-pose asset is shared data.

---

### F8. [MED] Per-instance material duplication in `spawn_model` (+ dead `blink_state` uniform)

**Files:** `src/model_loader.rs:1249-1266` (`create_rose_object_material` call per part); `src/render/object_material_extension.rs:49` (`blink_state`), `model_loader.rs:76` (init 0)

**Description:** Every model part of every character/NPC/vehicle creates a brand-new `ExtendedMaterial<StandardMaterial, RoseObjectExtension>` via `object_materials.add(...)` with no caching keyed by (texture, alpha mode, two-sided). The same texture+settings combos are re-created per instance (hundreds of characters wearing the same armor). Furthermore `blink_state` (a uniform field in every character material) is **never written after creation** (grep: only initialization sites) and **never read by the shader** (`rose_object_extension.wgsl` contains no `blink` references) — it is dead payload in every material, and it is not what makes per-instance materials necessary (blinking is handled via the separate `BlinkClipState` component path).

**Impact:** ~5000 material assets for 500 characters vs ~300 unique combos. Each material = own uniform buffer + bind-group slot and asset-slot churn at spawn/despawn. Memory/GPU-uniform overhead scales with population.

**Fix sketches:**
1. Add a material cache in `ModelLoader`: `HashMap<(AssetId<Image>, AlphaMode, bool /*two_sided*/), Handle<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>` and clone the cached handle instead of `add()`-ing a new one.
2. Remove the `blink_state` field from `RoseObjectExtension` (dead) — or, if blinking via material is intended later, wire it to the `BlinkClipState` value; keeping it as-is is fine too, but note the uniform can then be shared.
3. Note: the same duplication exists for effect-mesh materials (`effect_loader.rs:334-353`, `zone_loader/spawning/objects.rs:386-404`) — every effect spawn re-creates its material; an `EffectCache` exists for parsed EFT files (`effect_loader.rs` `EffectCache`) but not for materials.

---

### F9. [MED] NPC skeleton `.ZMD` file re-read + re-parsed on every NPC spawn of the same type

**Files:** `src/model_loader.rs:253-258` (`spawn_npc_model` → `self.vfs.read_file::<ZmdFile, _>(p)`)

**Description:** For each spawned NPC, `spawn_npc_model` resolves the NPC's skeleton path and **reads + fully parses** the `ZmdFile` from the VFS. NPCs of the same `skeleton_index` (very common — dozens of NPCs share one skeleton file) repeat this file read + parse per spawn. The raw bytes are VFS-cached (so I/O is cheap), but the `ZmdFile` parse (bone tree construction) runs per spawn.

**Impact:** Zone load with 50 static NPCs across ~8 skeleton files ⇒ 50 parses instead of ~8. Each parse is small (tens of bones) but the pattern is exactly the "repeated file read/parse" anti-pattern; it also runs on every NPC respawn during gameplay.

**Fix sketch:** Cache parsed skeletons in `ModelLoader`:
```rust
npc_skeleton_cache: HashMap<usize /*skeleton_index*/, Option<Arc<ZmdFile>>>,
```
populated on first use. (Option so missing skeletons are not re-probed.)

---

### F10. [MED] `ZmoAsset` keeps per-bone `scale` channel data that skeletal animation never uses

**Files:** `src/animation/zmo_asset_loader.rs:192-194` (scale parse), `20-25` (`ZmoAssetBone`), `src/animation/skeletal_animation.rs:104-147` (only translation+rotation sampled)

**Description:** `ZmoAssetBone` stores `translation`, `rotation`, **and** `scale` per bone; `skeletal_animation_system` samples only translation and rotation. `sample_scale` is called only by `transform_animation_system` (channel 0). For character/NPC skeleton ZMOs, the scale vectors are retained in memory and never read.

**Impact:** Memory only, but non-trivial across the whole animation set: e.g., 45 bones × 60 frames × 4 bytes = ~10 KB per animated skeleton ZMO × dozens of motion files × character variants = low hundreds of KB to a few MB retained. Also, per-frame `sample_translation`/`sample_rotation` do two bounds-checked `.get()` chains each (`zmo_asset_loader.rs:48-64`) — fine, but the storage layout (AoS `Vec<Vec3>` per bone) is cache-unfriendly when sampling all bones of one frame.

**Fix sketches:**
1. Load skeleton-ZMO scale data lazily or skip entirely (parse only when a consumer needs it; e.g., `transform_animation` variants).
2. Optional: add a `skeleton: bool` flag in `ZmoAsset` (set by `ZmoAssetLoader` vs `ZmoTextureAssetLoader`) so consumers can skip scale.
3. Optional micro-opt: switch to `Vec<[Vec3; 4]>`/SoA frame-major storage so a per-frame pass over all bones touches contiguous memory. Low priority.

---

### F11. [MED] `character_model_add_collider_system` / `npc_model_add_collider_system` poll every frame (repeated AABB queries) until colliders exist

**Files:** `src/systems/character_model_add_collider_system.rs:20-143`, `src/systems/npc_model_add_collider_system.rs:24-112`

**Description:** Both systems run every frame over entities without `ColliderEntity`. While a model's meshes are still loading (AABB absent), they re-run the per-part `query_aabb.get(...)` loop and `inverse_bindposes.get(...)`/ZMO lookups every frame (`npc` variant also does `zmo_assets.get(&action_motions[Stop])` per frame, lines 53-70). This window is short (a few frames per spawn), but each frame of the window performs N-parts AABB queries + `log::info!` spam (lines 36, 92, 141; the NPC variant has no info spam).

**Impact:** Low per-frame cost; the main cost is wasted query work and log noise during spawn windows in heavily-populated zones. Event-driven via `AssetEvent<Mesh>`/`Added<Mesh3d>` completion would remove the polling entirely.

**Fix sketch:** Drive the "wait until all parts' meshes are loaded" condition from `AssetServer::get_load_state`/`AssetEvent<Mesh>` instead of re-querying AABBs each frame; or keep polling but only every K frames via a `Timer`/`Local<u32>` frame counter. Also remove the per-frame `info!` logs (lines 36, 92-95, 141-142).

---

### F12. [LOW-MED] `mesh_animation_system` material access pattern creates a global mutex-point + Handle clones

**Files:** `src/animation/mesh_animation.rs:39-56, 62, 79-80`

**Description:** The system takes `ResMut<Assets<ExtendedMaterial<...>>>` (line 50-52), which serializes it against every other system that adds/mutates effect-mesh materials (e.g., `effect_loader` spawns during combat). It also clones the ZMO handle per entity per frame (line 62) and clones the material handle via `.map(|m| m.0.clone())` (line 79) — small per-frame allocations (Handle clones are cheap, but unnecessary: can borrow).

**Impact:** Contention window is small but real (effect spawning happens in bursts during combat); the clones are negligible. The `ResMut` is only needed because the material is mutated (F4). If F4's fix #3 lands, the system becomes read-only (`Res<Assets<...>>` + `Query<&MeshMaterial3d<...>>`).

---

### F13. [LOW] `blink_state` uniform + `BlinkUniform`/`BlinkUniformBuffer` plumbing is dead code on every character material

**Files:** `src/render/object_material_extension.rs:45-49`, `src/components/blink_clip.rs:36-45`

**Description:** Confirmed by grep: `blink_state` is initialized to 0 (`model_loader.rs:76`, `object_material_extension.rs:70`, map editor + zone loader copies) and never updated; `rose_object_extension.wgsl` never references it. `BlinkUniform` in `blink_clip.rs` is likewise unbound to any shader.

**Impact:** Dead uniform data on ~N materials; no runtime cost beyond a few bytes per material, but it misleads future work on blinking. Removing it also enables material sharing (F8) without behavior change.

---

### F14. [LOW] `VfsAssetIo::read` clones the entire cached file bytes on every asset read

**Files:** `src/vfs_asset_io.rs:140-155`

**Description:** On every cache hit, `VecReader::new((*cached_data).clone())` clones the full byte buffer (e.g., a 1–4 MB DDS or 100–500 KB ZMS) into a fresh `Vec`. Asset loads are cached by `AssetServer` (one read per unique path), so this is load-time only — but during a zone load that reads hundreds of models/textures, this is hundreds of multi-MB allocations + memcpys, and it repeats every time a character variant re-spawns a unique path.

**Impact:** Load-time memory churn; minor per-frame impact (some effect textures load mid-combat). A `Bytes`/`Arc<[u8]>`-backed reader would avoid the clone (Bevy's `VecReader` takes `Vec<u8>`; one can pass a `Vec` built from `cached_data.to_vec()` — the clone is inherent to the current reader design; wrapping the Arc in a custom reader or using `bevy_asset::io::Bytes` would avoid it).

---

### F15. [LOW] `AnimationState::advance` writes all frame fields every frame even when the frame index did not advance

**Files:** `src/animation/animation_state.rs:190-212`

**Description:** At high fps (or with slow animations), several rendered frames share the same animation frame. `advance()` recomputes and writes `current_frame_fract/index/next/loop_count` unconditionally, marking `AnimationState` changed every frame. The write cost itself is trivial, but combined with F1/F2 the bone-write guard (F2 fix) makes the *only* remaining per-frame work in `skeletal_animation_system` this component write.

**Fix sketch:** Compute the new values into locals first; only assign (and thus mark changed) if they differ from current. This makes `Changed<SkeletalAnimation>` meaningful and lets F2's guard skip bone writes on identical frames.

---

### F16. [LOW-MED] `npc_idle_sound_system` iterates all NPCs every frame (read-only, but part of the per-frame animation-touching set)

**Files:** `src/systems/npc_idle_sound_system.rs:25-95`

**Description:** Runs every frame for every NPC with `&SkeletalAnimation`/`&Command`, reads loop counts, and only rarely plays a sound (20% per loop). Read-only; cost is the query iteration itself.

**Impact:** Minor (it shares the iteration cost with F1's query). If F1 adds culling, consider adding the same visibility filter here; alternatively gate on a `Timer` tick of ~0.5 s (loop checks don't need 60 Hz). Low priority.

---

### F17. [LOW] Dead debug scaffolding in hot paths

**Files:** `src/animation/skeletal_animation.rs:96-102, 115-119, 149-151` (`should_log` computed per entity per frame), `src/animation/zmo_asset_loader.rs:250-289` (info-level logging inside the ZMO-texture loader — load-time only)

**Description:** `should_log` is computed every frame per entity (two comparisons) and wraps fully commented-out logs. The ZMO texture loader emits 5 `log::info!` lines per loaded animation texture (load-time, harmless).

**Impact:** Negligible CPU; housekeeping.

---

### F18. [LOW] `wing_spawn_system` is a stub with a per-event `log::info!`

**Files:** `src/systems/wing_spawn_system.rs:26-37`

**Description:** Wing spawning is intentionally disabled; the system only logs. No cost. Flagged for completeness (flying characters currently animate without wings — if wings return, see F8 for material/mesh reuse).

---

## 4. Priority-ranked summary

| # | Finding | Impact | Effort |
|---|---------|--------|--------|
| F1 | No visibility/distance culling on any animation system | High | Low–Med |
| F2 | Per-bone `Query::get_mut` + unconditional bone writes ⇒ propagation storm | High | Med |
| F3 | `update_active_motion` writes speed (or re-inserts) every frame | High | Low |
| F4 | Per-frame material asset mutation ⇒ per-frame render re-prepare | High | Med |
| F5 | Sail mesh vertex buffer re-uploaded every frame | High | Med |
| F6 | `sync_blink_clip_to_state` re-inserts component every frame (+ dead twin) | Med-High | Very Low |
| F7 | Inverse-bind-pose asset duplicated per spawn | Med | Low |
| F8 | Per-instance material duplication (+ dead `blink_state`) | Med | Low–Med |
| F9 | NPC skeleton ZMD re-read/re-parsed per spawn | Med | Low |
| F10 | Unused per-bone scale channel data in skeleton ZMOs | Med (memory) | Low |
| F11 | Collider systems poll + log every frame while meshes load | Med | Low |
| F12 | `ResMut` material assets in `mesh_animation_system` (contention + clones) | Low-Med | Low |
| F13 | Dead `blink_state`/`BlinkUniform` on all character materials | Low | Very Low |
| F14 | VFS cache read clones full file bytes | Low (load-time) | Med |
| F15 | `advance()` writes frame fields even when unchanged | Low | Low |
| F16 | `npc_idle_sound_system` per-frame iteration of all NPCs | Low | Low |
| F17 | Dead `should_log` scaffolding in hot loop | Very Low | Very Low |
| F18 | `wing_spawn_system` stub logging | None | — |

## 5. Quick wins (small change, big effect)

1. **F6:** add `Changed<BlinkClip>` to `sync_blink_clip_to_state` (one-line filter; removes per-frame inserts/sync/extract churn). Delete the unregistered `update_blink_clip_state`.
2. **F3:** compare `animation_speed` before writing and skip `insert` for `Handle::default()` motions — kills per-frame component writes for the entire animated population.
3. **F1 (subset):** add `Option<&ViewVisibility>` + skip to `skeletal_animation_system` and `mesh_animation_system` — immediately halves+ animation cost in typical scenes with off-screen NPCs.
4. **F4 (subset):** precompute `flags` once (store on `ZmoAsset` at load time) and add an early-out when `current_next_frame`/`next_weight` are unchanged.
5. **F7:** cache inverse-bind-pose handles per skeleton/gender in `ModelLoader`.
6. **F9:** cache parsed NPC skeletons by `skeleton_index`.
7. **F2 (subset):** guard bone writes with `abs_diff_eq` (skip identical poses on repeated frames at high fps).
8. **F17:** delete `should_log` scaffolding; **F11:** drop per-frame `info!` logs in the collider systems.

## 6. Risks & considerations per fix

- **F1 (culling):** Correctness — a character whose *skeleton* is off-screen but whose weapon trail/effect crosses the screen would freeze its pose. Recommended approach: use the model root's `ViewVisibility` (already frustum-computed) and keep a safety margin (e.g., also animate entities within X meters of the camera regardless of visibility). Beware: `ViewVisibility` is computed in `MainPass`/visibility propagation; `skeletal_animation_system` runs in `PostUpdate` *after* visibility propagation in the same schedule — in Bevy 0.18, PostUpdate ordering vs `VisibilityPropagate` is fine (both PostUpdate; add `.after(VisibilityPropagate)` if needed — verify actual ordering, `system-architecture/Animation.md` documents the current `before(TransformSystems::Propagate)` constraint).
- **F2 (batching/guards):** `iter_many_mut` is available in 0.18 (`bevy_ecs/src/system/query.rs:936`) but requires `&mut` only (fine here) and loses per-entity pairing with bone_id — you must zip with the joints iterator; keep the `get_mut` fallback for despawned joints (bones are despawned during gender/model changes, see `character_model_system.rs:83-87` — a frame gap could despawn bones between systems; `iter_many_mut` silently skips missing entities, which is safe but check counts). `abs_diff_eq` on Quat/Vec3: use an epsilon consistent with visual quality (e.g., 1e-5 translation, 1e-4 quat) — too large eps cause visible pop at loop boundaries; the `next_frame_index` guard in `advance` (last-frame-of-loop) must still write.
- **F3 (speed guard):** `animation_speed()` getter must be added; behavior identical. For `Handle::default()` motions — verify that inserting a default-handle animation is never used to *stop* an existing animation (grep: `update_active_motion` is the only inserter; `command_system.rs:1002` inserts `SkeletalAnimation::once` for death directly, not via default handle). Keep the insert if any code relies on default-handle replacing a running animation.
- **F4 (uniform refactor):** Precomputed flags: store as a `u32` on `ZmoAsset` set in both loaders. Early-out: `current_next_frame`/`next_weight` only change when the animation advances — at 30 fps animation vs 60+ fps render, ~half the frames can be skipped; the `alpha` field also depends on frame. The structural fix (per-entity uniform) requires a custom material/pipeline change and shader edits (`rose_effect_extension.wgsl`); do it behind the existing `EffectMeshAnimationUniform` layout to avoid shader churn (uniform layout must stay 16 bytes to match).
- **F5 (shader deformation):** Medium risk — sail visual quality is tuned in `sail_animation_system` (billow/luff formulas, `SailQuality` setting in `graphics_settings`). Preserve the `SailQuality::Low` early-out. If moving to a shader, the base positions must be passed as an extra vertex attribute or uniform-packed buffer; mesh must be marked `RenderAssetUsages::RENDER_WORLD`-only to avoid re-upload. Fallback: 10–15 Hz quantized CPU updates (still exact visuals, 4–6× less upload).
- **F6 (Changed filter):** Safe — `BlinkClip` only changes in `character_model_blink_system` on blink transitions; `sync_blink_clip_to_state` will fire exactly then. Note: `BlinkClip` is inserted fresh on face entities at spawn (`character_model_blink_system.rs:44-59`), which counts as `Added` and is covered by `Changed` (Added ⊆ Changed). Keep the `With<Mesh3d>` filter.
- **F7 (IBP cache):** Safe — `SkinnedMeshInverseBindposes` is immutable per skeleton; handles are strong so the asset lives. Must key on the exact skeleton (gender for characters; skeleton_index/path for NPCs; cart vs castle gear for vehicles) — a wrong-key collision would visibly break skinning. Clear the cache on... nothing (assets are session-scoped; zone transitions keep characters). Memory: cache holds one asset per unique skeleton (~10) instead of per spawn — strictly better.
- **F8 (material cache):** Key must include texture handle id, `AlphaMode` (Opaque/Mask threshold/Blend), `two_sided`/`cull_mode`, specular handle. Do **not** share materials across entities that need per-entity material state — currently none do (blink is component-based, `blink_state` dead). The effect-mesh material (`RoseEffectExtension`) is mutated per frame by F4 — do not share *those* until F4 fix #3 lands (otherwise per-frame mutation of a shared material would update all instances).
- **F9 (skeleton cache):** Safe; `ZmdFile` is immutable. Watch memory: cache only NPC skeleton files (a handful), not per-NPC variants.
- **F11 (event-driven colliders):** Must keep the "wait for all part meshes + inverse bind poses + (NPC) idle ZMO" completion logic — moving to `AssetEvent<Mesh>` is fiddly because a model part has multiple meshes; a frame-counter throttle is the low-risk option.
- **F14 (VFS clone):** Bevy's `VecReader` owns a `Vec`; wrapping the `Arc<Vec<u8>>` in a custom `Reader` avoids the copy but must uphold `AssetReader`'s `Send + Sync + 'static` requirements. Keep `use_cache` semantics (the raw cache must not be mutated by readers).
- **F15 (advance writes):** `AnimationState` fields are public-read via getters; assigning only on change is behavior-preserving. Must still write when `completed` flips (death-pose handling, lines 196-212).
- **F13 (remove blink_state):** Verify nothing reads it in any shader first (`rose_object_extension.wgsl` — grep found none; re-check map-editor and zone-loader material creations which set it at construction). Removing a uniform field changes the material bind group layout — bump the `#[uniform]`/AsBindGroup indices only if fields before it change order (they don't).

## 7. Notes for future reviews

- The `skeletal_animation_system` + `command_system` interplay is the single largest CPU consumer in the animation domain; revisit F1–F3 as a combined change (culling + batching + write guards) and measure with `tracy`/frame-time diagnostics before/after.
- `system-architecture/Animation.md` was accurate against the code on every point checked (advance semantics, event system, command-driven state machine). No doc gap found for the animation domain.
- Pitfall `performance-memory.md` ("every-frame asset creation") has a sibling here: **every-frame asset mutation** (F4, F5) — worth a new pitfall entry if the fixes land.
