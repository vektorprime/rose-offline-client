# Model Spawning and Colliders

How characters, NPCs/monsters, item drops, and personal stores get visible models plus Rapier colliders. Ordering is enforced by `ModelSystemSets` (`src/lib.rs:681-690,1013-1026`): `CharacterModelUpdate` → `CharacterModelAddCollider` → `PersonalStoreModel` → `PersonalStoreModelAddCollider` → `NpcModelUpdate` → `NpcModelAddCollider` → `ItemDropModel` → `ItemDropModelAddCollider`.

## Per-type pipeline

- Characters: model update system + `src/systems/character_model_add_collider_system.rs` (cuboid from combined part AABB incl. Head/Face/Hair; `COLLISION_GROUP_PLAYER` or `CHARACTER`; collider childed to root bone; sets `ColliderEntity`/`ColliderParent` pair and `ModelHeight::new(1.8 + half_extents.y * 2.0)`).
- NPCs/monsters: `npc_model_update` + `src/systems/npc_model_add_collider_system.rs` (`COLLISION_GROUP_NPC`; flying-NPC root-bone height fix). Network spawn inserts `MonsterSeparation` for `ClientEntityType::Monster` (`game_connection_system.rs`, `zone_content/monsters.rs`) — see [monster-collision-system.md](monster-collision-system.md).
- Item drops: `item_drop_model_system.rs` (clickable/inspectable collider).
- Personal stores: `personal_store_model_system.rs` + `personal_store_model_add_collider_system.rs`; UI in `ui_personal_store_system.rs` / `ui_npc_store_system.rs`; events `personal_store_event.rs` / `npc_store_event.rs`.
- Blink state for characters flows through `RoseObjectExtension` (`src/render/object_material_extension.rs`) and `character_model_blink_system` (`PostUpdate`).

Collider cleanup uses the `ColliderEntity` (owner → collider) / `ColliderParent` (collider → owner) pair plus `RemoveColliderCommand` (`src/components/collision.rs`). Groups/filters reference: [Physics.md](Physics.md).
