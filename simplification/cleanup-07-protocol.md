# Cleanup Implementation 07 — Protocol & Audio (branch `code-simplification`)

Implementation report for `simplification/07-protocol.md`. All changes were made by the cleanup agent on the items assigned to it. Total: **~1,191 LOC removed** (net across owned files).

## Summary of changes

| Item | Change | LOC |
|---|---|---|
| Report 6 | Deleted entire `src/diagnostics/` module (mod.rs + render_diagnostics.rs) after grep-verifying only `lib.rs:74,104` reference it (all other references are commented out) | -606 |
| Report 1a | `handle_packet` in `game_client.rs`: added function-local `server_message!` macro (`($packet_type, $message { fields })` → try_from + send + `.ok()`); 40 identity-conversion arms converted to macro calls, 35 arms (ConnectReply, SelectCharacter, CharacterInventory, QuestData, JoinZone, SpawnEntityCharacter, DamageEntity, PickupItemDropResult, chat arms with `.to_string()`, UpdateInventory, UpdateAbilityValueReward*, UpdateLevel, UpdateStatusEffects, QuestResult, LearnSkill/LevelUpSkillResult, UseItem, ApplySkillDamage, MoveToggle, Party*, BankOpen, BankTransaction, LogoutResult, OpenPersonalStore, CraftItem, ClanCommand, RepairedItemUsingItem, None) left hand-written byte-identical | -321 |
| Report 1b | `handle_client_message` in all three clients: function-local `send_packet!` macro (`($packet)` → `connection.write_packet(Packet::from(&$packet)).await?;`); all 52 (game) / 8 (world) / 5 (login) arms converted | -17 |
| Report 1d | Removed commented-out `// Ok(())` in `protocol/mod.rs` macro; removed commented-out `// ServerPackets::ReturnToCharacterSelect` arm in `world_client.rs` | -2 |
| Report 4b | `SoundGain::Decibel` variant deleted; `#[allow(dead_code)]` removed; all 5 match sites simplified to ratio-only (see deviation note below) | -30 |
| Report 4a | Added `spawn_spatial_sound` helper in `src/audio/mod.rs` (exact signature from task); migrated `monster_sound_cap.rs` spawn (see deviation for `vehicle_sound_system.rs`) | +34 / -20 |
| Report 4c | `streaming_sound.rs`: `fill_mono`/`fill_stereo` deduplicated into one generic private `fill(&mut self, mut write: impl FnMut(&[f32]) -> usize, repeating)`; the `unsafe` raw-pointer cast moved verbatim into a private `stereo_frames` helper (identical cast to original :162-164) | -54 |
| Report 4e | Deleted dead `PendingMonsterSound` component + its re-export | -8 |
| Report 4f | Removed 4 stale `#[allow(dead_code)]` attributes (SpatialControlHandle, SpatialSound, ControlHandle, GlobalSound impls) | 0 |
| Report 5a | `json_format.rs`: removed 5 config fields (`span_events`, `ansi`, `display_target`, `display_filename`, `display_line_number`) + 5 builder methods; `target` now emitted unconditionally (was always true); output JSON shape unchanged | -66 |
| Report 5b | Deleted `init_bevy_logging` and `get_session_log_path` (both never called); `bevy_version` `"0.16.1"` → `"0.18.1"` (SessionInfo + test); removed now-unused `FmtSpan`/`NonBlocking`/`Registry` imports | -22 |
| Report 3a | Deleted `SpawnEffectEvent::WithTransform` variant (+`#[allow(dead_code)]`) and its match arm in `spawn_effect_system.rs` (+`Transform` import trimmed; `Transform` still used at :104 so import kept) | -25 |
| Report 3b | `blood_effect_event.rs`: deleted 4 never-called constructors (`kill_spatter`, `hit_spatter`, `update_visibility`, `cleanup` — verified via grep) and trimmed field-level doc comments on the enum variants | -73 |
| Report 3d | `ping_event.rs`: removed redundant legacy `Event` derive (kept `Message`) | 0 |
| Report 3e | `zone_event.rs`: removed stray comment | -1 |

## Macro design notes

- `server_message!`/`send_packet!` are defined **inside** the functions they serve. Module-level `#[macro_export]` was tried first but rejected by the compiler: `self`, `packet` and `connection` do not resolve hygienically from a module-level macro body ("macros cannot expand to match arms" also ruled out arm-generating macros). Function-local definitions capture `packet`/`self`/`connection` via definition-site context (verified with a standalone rustc test before applying).
- All macro expansions are byte-identical to the hand-written code they replace: same `try_from(packet)?` + `send(...).ok()` chain and same `write_packet(Packet::from(&...)).await?` wrapper. Arms the macro could not cover verbatim (renames, casts, conditional sends, enum matches) remain hand-written.

## Deviations / skipped items (documented decisions)

1. **`SoundGain` kept as enum with only `Ratio`** (report 4b suggested collapsing to `pub struct SoundGain(pub f32)`). The struct collapse is impossible while `src/audio/boat_sound.rs` is STAGING: it constructs `SoundGain::Ratio(...)` at 5 sites (lines 168, 190, 277, 284, 298) and cannot be edited. `Decibel` is deleted, the 5 match arms are simplified, `#[allow(dead_code)]` removed. When the boat agent lands, the enum → struct collapse is a trivial follow-up (5 construction sites + `sound_settings.rs:16,18`).
2. **`vehicle_sound_system.rs:80,107` NOT migrated to `spawn_spatial_sound`** (task listed them; report 4a claimed the duplicated bundle there — verified inaccurate):
   - :80 is `commands.entity(...).insert(SpatialSound::new_repeating(...))` — an **insert into an existing parented entity**, not a spawn; the helper cannot express it without despawn/respawn (changes entity identity tracked in `VehicleSound.sound_entity` and `VehicleModel.model_parts`).
   - :107 spawns a **child** of the vehicle model with `Transform::default()`; its world position derives from the parent via Bevy transform propagation (DefaultPlugins active). The helper's `Transform::from_translation(position)` would double the offset (propagation computes `parent_global × local`), moving the sound far from the vehicle. The helper also adds a `SoundGain` component the vehicle entities deliberately lack (they'd start responding to the footstep-volume sliders in `ui_settings_system.rs:433-441`).
   - `monster_sound_cap.rs` (unparented spawn, exact bundle match) WAS migrated.
3. **Report 1c (shared `build_connection_request_packet` helper) skipped**: `PacketClientConnectRequest<'a>` holds `password_md5: &'a str`, and the two types (`game_client_packets` vs `world_client_packets`) are distinct, so a shared helper needs a generic bound over `Packet: From<&T>` with a phantom lifetime — not worth it; the `send_packet!` macro already reduces both arms to one call.
4. **Report 2a skipped** (network_thread_system.rs owned by another agent).
5. **Report 4a boat parts + 4d procedural boat synthesis skipped** (boat_sound.rs STAGING, untouched — verified no edits; `SoundGain` API kept compatible so it still compiles).
6. **`RepairedItemUsingItem` unimplemented arm kept** (real packet, unimplemented behavior — report 1d flagged it but it is not dead code).
7. Dead `let player_position` + `query_player` param in `process_monster_sound_queue_system` removed (assigned, never read — pre-existing dead code in an owned file).

## Verification

`cargo check` (run once, per task) — **zero errors in owned files**. One real bug found and fixed during check: `fill`'s closure param needed `mut` for `FnMut` (streaming_sound.rs:57).

Remaining check errors are all in other agents' in-progress files: `scripting/*` (Lua4Value/macros), `ui/widgets/mod.rs`, `ui/ui_admin_menu_system.rs`, `ui/ui_inventory_system.rs`, `ui/ui_minimap_system.rs`, `bundles/ability_values.rs`, `zone_loader/systems.rs`, `systems/conversation_dialog_system.rs`, `render/*` imports in lib.rs:117, lib.rs:872. None were touched.

Pre-existing warnings in owned files left as-is: unused imports in `src/audio/ogg.rs` (Future, AsyncReadExt) and `src/audio/wav.rs` (Future, AsyncReadExt) — not covered by report items; boat_sound.rs untouched.

## LIB.RS CHANGES NEEDED

Required (compile error E0583 currently: `file not found for module 'diagnostics'`):

- `src/lib.rs:74` — delete `pub mod diagnostics;`
- `src/lib.rs:104` — delete `use diagnostics::RenderDiagnosticsPlugin;`
- Optional: `src/lib.rs:1095-1096` — the commented-out `// RenderDiagnosticsPlugin` registration block (now references a deleted plugin).

No other lib.rs changes are required by this cleanup (`RenderExtractionDiagnostics` at lib.rs:2044 is `src/resources/`, untouched).
