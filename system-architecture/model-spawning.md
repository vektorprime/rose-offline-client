# Model Spawning and Colliders

How characters, NPCs/monsters, item drops, and personal stores get visible models plus Rapier colliders. Ordering is enforced by `ModelSystemSets` (`src/lib.rs:681-690,1013-1026`): `CharacterModelUpdate` → `CharacterModelAddCollider` → `PersonalStoreModel` → `PersonalStoreModelAddCollider` → `NpcModelUpdate` → `NpcModelAddCollider` → `ItemDropModel` → `ItemDropModelAddCollider`.

## Per-type pipeline

- Characters: model update system + `src/systems/character_model_add_collider_system.rs` (cuboid from combined part AABB incl. Head/Face/Hair; `COLLISION_GROUP_PLAYER` or `CHARACTER`; collider childed to root bone; sets `ColliderEntity`/`ColliderParent` pair and `ModelHeight::new(1.8 + half_extents.y * 2.0)`).
- NPCs/monsters: `npc_model_update` + `src/systems/npc_model_add_collider_system.rs` (`COLLISION_GROUP_NPC`; flying-NPC root-bone height fix). Network spawn inserts `MonsterSeparation` for `ClientEntityType::Monster` (`game_connection_system.rs`, `zone_content/monsters.rs`) — see [monster-collision-system.md](monster-collision-system.md).
- Item drops: `item_drop_model_system.rs` (clickable/inspectable collider).
- Personal stores: `personal_store_model_system.rs` + `personal_store_model_add_collider_system.rs`; UI in `ui_personal_store_system.rs` / `ui_npc_store_system.rs`; events `personal_store_event.rs` / `npc_store_event.rs`.
- Blink: `character_model_blink_system` (`PostUpdate`) toggles `CharacterBlinkTimer` (closed while dead) and swaps each character face part's `Mesh3d` between two meshes derived once per source face mesh, as the original client clipped faces: eyes open = without the first n faces (the closed eyelids), eyes closed = without the last n faces (the open eyes), n = the last entry of the face ZMS's `ZmsMaterialNumFaces`. Only the index buffer differs, so every vertex attribute and the bounds match. The face keeps `BlinkClipMeshes { source, eyes_open, eyes_closed }`; the derived meshes are cached per source `AssetId` (reused via `get_strong_handle`). A face is switched only after `skinned_mesh_fix` has processed it (`SkinningTarget` removed) and both assets are loaded; faces without a material split keep the full mesh.

## Spawn caches (`src/model_loader.rs`)

`ModelLoader` (a `Resource`, so its caches sit behind `Mutex`es) avoids per-spawn work that produced identical data:
- **NPC skeletons:** parsed `ZmdFile`s are cached per `LIST_NPC.CHR` skeleton index (`npc_skeleton`); a failed read is not cached.
- **Inverse bind poses:** one `SkinnedMeshInverseBindposes` asset per skeleton (`SkeletonKey`: Male, Female, Cart, CastleGear, Npc(index)). Bone entities are still spawned per model.
- **Part materials:** `spawn_model` shares one `ExtendedMaterial<StandardMaterial, RoseObjectExtension>` per `PartMaterialKey` (texture path after the NULL fallback, specular image, alpha mode, two-sided), i.e. everything it passes to `create_rose_object_material`. Shared materials give batchable draws instead of one bind group per part per spawn. The texture is still `asset_server.load`ed on every spawn.

The asset caches store `AssetId`s, never strong handles, and reuse them through `Assets::get_strong_handle`, which returns `None` once every model holding the asset is gone (the asset is then created again). They keep nothing alive.

**Per-entity material writes must copy first.** Every part spawned from the shared cache carries `SharedModelPartMaterial`. A system that writes per-entity values into a part material must give that part its own copy (`materials.add(material.clone())`, insert the new `MeshMaterial3d`, remove the marker) instead of writing the shared asset, or every model of that type changes. `blood_overlay_generate_system` does this (copy-on-write in `sync_material_overlay`); it is the only per-entity writer today. Zone objects use their own per-zone material cache and never carry the marker.

Collider cleanup uses the `ColliderEntity` (owner → collider) / `ColliderParent` (collider → owner) pair plus `RemoveColliderCommand` (`src/components/collision.rs`). Groups/filters reference: [Physics.md](Physics.md).
