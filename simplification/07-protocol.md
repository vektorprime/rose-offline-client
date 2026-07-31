# Simplification Plan 07 — Network Protocol & Audio

**PLAN ONLY — no code changes were made.** This document is a research report produced by a read-only analysis pass.

## Scope & LOC analyzed

| Module | Files | LOC |
|---|---|---|
| `src/protocol/` | mod.rs (58), irose/mod.rs (7), irose/game_client.rs (1638), irose/world_client.rs (214), irose/login_client.rs (176) | ~2093 |
| `src/systems/network_thread_system.rs` + `src/resources/network_thread.rs` | 110 + 41 | ~151 |
| `src/events/` | 32 files | ~800 |
| `src/audio/` | 9 files | ~1357 |
| `src/logging/` | mod.rs (254), json_format.rs (302) | ~556 |
| `src/diagnostics/` | mod.rs (12), render_diagnostics.rs (594) | ~606 |
| Context: `src/systems/animation_sound_system.rs`, `npc_idle_sound_system.rs`, `use_item_event_system.rs`, `client_entity_event_system.rs`, `vehicle_sound_system.rs`, `effect_loader.rs` | read for audio-spawn duplication | ~1000 |
| **Total** | | **~5,600 (plus ~1,000 context)** |

---

## 1. Protocol: `game_client.rs` — two giant functions

### 1a. `handle_packet` — 1,070-line match of ~75 arms
`src/protocol/irose/game_client.rs:93-1163`

- **What:** One giant `match FromPrimitive::from_u16(packet.command)` with ~75 arms. ~60 arms follow the identical shape:
  ```
  let message = PacketServerX::try_from(packet)?;
  self.server_message_tx.send(ServerMessage::X { ... }).ok();
  ```
  Many arms are pure field-for-field remaps with zero transformation (e.g., UpdateAmmo :372-385, SetHotbarSlot :592-600, UpdateItemLife :433-441, CastSkillSelf :677-686, StartCastingSkill :711-718, ChangeNpcId :668-676, RewardMoney :576-583, RewardItems :584-591, ClosePersonalStore :1010-1017, OpenPersonalStore :1000-1009, UpdateConsumableCooldown :746-754, UpdateCooldown :737-745, RepairedItemUsingNpc :1035-1044, PartyUpdateRules :911-919, AdjustPosition :920-928…).
- **Why over-complex:** the client re-declares every field of every `ServerMessage` by hand even when packet fields and message fields are identical (redundant copy layer between `rose_network_irose` packet types and `rose_game_common` messages).
- **Suggestion:** a macro to generate arms, e.g. `server_message!(ServerPackets::X, PacketServerX => ServerMessage::X)` expanding to the try_from + send + `.ok()` boilerplate; for the ~40 identity conversions this collapses each arm from ~9 lines to 1. Alternatively (larger refactor) implement `TryFrom<PacketServerX> for ServerMessage` in `rose_network_irose` and make the match a table. Grouped variants (MoveEntity | MoveEntityWithMoveMode :170) still need hand arms, but ~55 arms become macro calls.
- **Savings:** ~450-550 LOC.

### 1b. `handle_client_message` — 470-line match of ~55 arms
`src/protocol/irose/game_client.rs:1165-1635`

- **What:** every arm is `connection.write_packet(Packet::from(&PacketClientX { ... })).await?` — identical 4-line wrapper, ~55 times. Same wrapper repeats in `world_client.rs:135-211` (~8 arms) and `login_client.rs:130-173` (~5 arms).
- **Suggestion:** a small macro `send_packet!(connection, PacketClientX { fields... })` expanding to `connection.write_packet(Packet::from(&PacketClientX {...})).await?;`. Trivial, zero-risk change.
- **Savings:** ~150-200 LOC across the three clients.

### 1c. Duplicated `ConnectionRequest` handling across clients
- `game_client.rs:1171-1181` and `world_client.rs:136-146` are byte-identical (build `PacketClientConnectRequest` from `login_token` + `password.to_md5()`). The three ConnectReply result-to-message mappings (game_client.rs:95-109, world_client.rs:53-64, login_client.rs:50-61) are near-identical (map `Ok` → `ConnectionRequestSuccess{packet_sequence_id}`, `_` → error).
- **Suggestion:** shared helper `build_connection_request_packet(login_token, password)` (or let the `ConnectionRequest` arm live in the macro in 1b). Small.

### 1d. Commented-out / dead protocol code
- `protocol/mod.rs:52` — commented-out `// Ok(())` inside the `implement_protocol_client!` macro.
- `world_client.rs:123` — commented-out arm `// ServerPackets::ReturnToCharacterSelect -> ...`.
- `game_client.rs:1153-1158` — `RepairedItemUsingItem` arm only logs "Unimplemented" (real packet, unimplemented behavior).
- `game_client.rs:1630-1632`, `world_client.rs:203-208`, `login_client.rs:165-170` — `unimplemented => log::info!(...)` fallback arms (fine as safety net, but note the same log line is duplicated 3x; could be a macro).
- **Savings:** ~20 LOC (plus risk reduction).

### 1e. Good pattern (keep)
`implement_protocol_client!` (protocol/mod.rs:15-56) already removes the connection-loop duplication across the three clients — do not refactor.

---

## 2. Network thread plumbing

### 2a. `network_thread_system.rs` — three arms of identical channel plumbing
`src/systems/network_thread_system.rs:21-108`

- **What:** `ConnectLogin`/`ConnectWorld`/`ConnectGame` arms each repeat: create `crossbeam_channel::unbounded`, create `tokio::sync::mpsc::unbounded_channel`, `format!("{}:{}", ip, port).parse().unwrap()`, `control_tx.send(NetworkThreadMessage::RunProtocolClient(Box::new(...)))`, `commands.insert_resource(...)`. ~25 lines each, only the client type and resource differ.
- **Suggestion:** one helper `connect<T: ProtocolClient + 'static>(...)` or a `ClientConnection` builder taking the constructor closure. Also the `unwrap()` on address parse could be a clean error path.
- **Savings:** ~35-40 LOC.

### 2b. `resources/network_thread.rs` — fine
41 lines, `Exit` variant is genuinely used (lib.rs:2104). No action.

---

## 3. Events module — 32 tiny files

### 3a. Dead `SpawnEffectEvent::WithTransform`
- `src/events/spawn_effect_event.rs:51-52` — variant flagged `#[allow(dead_code)]`; matched in `spawn_effect_system.rs:145-153` but **never emitted anywhere** (grep: only the system match arm).
- **Suggestion:** delete variant + match arm + the `Transform` import.
- **Savings:** ~10 LOC + one `#[allow]`.

### 3b. `blood_effect_event.rs` over-engineering
`src/events/blood_effect_event.rs` (166 LOC): 7 constructor functions with heavy doc comments (lines 76-166, ~90 LOC) of which the ones actually called are `kill_spatter_with_profile` (blood_spatter_system.rs:128, hit_event_system.rs:165, pending_damage_system.rs:161), `hit_spatter_with_profile` (hit_event_system.rs:173, pending_damage_system.rs:169), `show_wound` (gash_wound_system.rs:128,146, hit_event_system.rs:187, pending_damage_system.rs:183). Never called: `kill_spatter`, `hit_spatter`, `update_visibility` (gash_wound_system matches the enum directly :318), `cleanup` (matched directly :328).
- **Suggestion:** delete the 4 unused constructors; trim the doc-comment verbosity on the rest. Optional — enum variants are all used, so this is polish, not deletion.
- **Savings:** ~60 LOC.

### 3c. Near-identical single-entity struct events
`boat_event.rs` (`BoardBoatEvent`/`DisembarkBoatEvent`), `flight_event.rs` (`FlightToggleEvent`), `move_speed_event.rs` (`MoveSpeedSetEvent`), `game_connection_event.rs` — all `{ entity }`-style carrier structs with identical doc-comment style.
- **Suggestion (optional, low priority):** one `EntityActionEvent { entity, action }` or keep as-is. The file-per-event layout (32 files for ~800 LOC) is consistent and idiomatic for Bevy; consolidating saves nothing but friction. **Recommend keeping**, except 3a/3b.

### 3d. `ping_event.rs:5,12` — redundant double derive
`#[derive(Event, Message, …)]` — `Event` is the legacy alias of `Message` in Bevy 0.18; the `Event` derive is redundant. Minor cleanup.

### 3e. `zone_event.rs:5` — stray comment
`// Import ZoneLoaderAsset for use in event` (line 5) is noise; also the import is only needed for the `Handle<ZoneLoaderAsset>` field. Minor.

---

## 4. Audio — duplicated sound setup patterns

### 4a. Duplicate spawn bundle repeated across ≥8 call sites
The exact bundle `(SoundCategory, SoundGain, SpatialSound::new(_)/new_repeating(_), Transform, GlobalTransform, Option<SoundRadius>)` is hand-built in:
- `boat_sound.rs:158-200` (`spawn_loop_sound` / `spawn_one_shot_sound` — two 20-line functions differing **only** in `new` vs `new_repeating`)
- `monster_sound_cap.rs:70-80`
- `animation_sound_system.rs:48-80` (`spawn_sound` helper duplicating the queue/direct split)
- `client_entity_event_system.rs:98`, `use_item_event_system.rs:79`, `vehicle_sound_system.rs:80,107`, `effect_loader.rs:179-202`, `zone_loader/spawning/objects.rs:682`
- **Suggestion:** one `pub fn spawn_spatial_sound(commands, handle, position, gain, radius, category, repeating)` in `audio/mod.rs`; boat functions collapse to calls. This also fixes the copy-paste bug where boat sounds are tagged `SoundCategory::PlayerFootstep` (boat_sound.rs:170,192) instead of a boat category.
- **Savings:** ~80-120 LOC across the codebase.

### 4b. Dead `SoundGain::Decibel` variant
- `audio/mod.rs:21-32` — enum has `#[allow(dead_code)]`; grep shows `Decibel` is **never constructed anywhere**; only match arms exist (spatial_sound.rs:77,184; global_sound.rs:67,128,147).
- **Suggestion:** collapse to `pub struct SoundGain(pub f32)` amplitude ratio (or keep enum with only Ratio). Delete the 5 match branches and the `Default` stays `1.0`.
- **Savings:** ~25 LOC + removed dead API.

### 4c. `streaming_sound.rs` — `fill_mono`/`fill_stereo` near-verbatim duplication
- `streaming_sound.rs:45-105` vs `:107-175` — the `Streaming` branch (lines 47-83 vs 113-149) is duplicated with only `frame_stereo` wrapping differing; the `Buffered` branch similarly. Also contains an `unsafe` raw-pointer slice cast (`:162-164`).
- **Suggestion:** one generic `fn fill(&mut self, write: impl FnMut(&[f32]) -> usize, repeating: bool)` parameterized by the write closure; callers pass `|samples| stream.write(samples)` or `|samples| stream.write(frame_stereo(samples))*2`.
- **Savings:** ~50-60 LOC and removes one `unsafe`.

### 4d. `boat_sound.rs` — 300 LOC of procedural sound synthesis
- `boat_sound.rs:12,55-103,109-156` — hand-synthesized placeholder sounds (`generated_loop`, `generated_one_shot`, `pseudo_noise`, `envelope`, 7 synthesis closures, `make_audio_source`). This is significant over-engineering: synthetic audio generated at runtime in code.
- **Suggestion:** replace with 7 small real `.ogg` files loaded via `asset_server.load()` like every other game sound (removes `make_audio_source`, `envelope`, `generated_loop`, `generated_one_shot`, `pseudo_noise`, the `BOAT_SOUND_SAMPLE_RATE` const, and the 7 closures).
- **Savings:** ~100 LOC + removes the highest-maintenance audio code. (Needs 7 asset files — the only non-code work item.)

### 4e. Dead `PendingMonsterSound` component
- `monster_sound_cap.rs:19-26` — Component type never spawned (queue uses `PendingMonsterSoundData`, lines 34-42); re-exported in `audio/mod.rs:56`.
- **Suggestion:** delete component + its `pub use`.
- **Savings:** ~8 LOC.

### 4f. Stale `#[allow(dead_code)]` attributes in audio
- `spatial_sound.rs:20,48` and `global_sound.rs:15,40` — `#[allow(dead_code)]` on impls whose every method IS used (gain_control/stop_control/stream_control/spatial_control; `new`/`new_repeating`). Remove the allows (or they hide future dead code).
- **Savings:** 0 LOC, hygiene only.

### 4g. `animation_sound_system.rs` (565 LOC) — repetitive flag blocks
- `animation_sound_system.rs:82-565` — 7 near-identical `if event.flags.contains(AnimationEventFlags::X)` blocks, each ending in the same 8-line `spawn_sound(...)` call (9 call sites: :158, :204, :230, :276, :340, :418, :452, :483, :517) with identical `sound_category`/`is_player_sound` derivation repeated 6x.
- **Suggestion:** fold the category/gain derivation into the `spawn_sound` helper (it already receives `is_player_sound`) so each call site shrinks to 2-3 args; consider a `play_sound(game_data, …)` closure for the common "sound_id → spawn" tail.
- **Savings:** ~60-80 LOC.

---

## 5. Logging

### 5a. `json_format.rs` (302 LOC) — dead builder API
- Only `TagExtractingJsonFormat::new()` (all defaults) is ever used (mod.rs:174). The 5 builder methods (`with_ansi` :82, `with_target` :88, `with_file` :94, `with_line_number` :100, `with_span_events` :106) and their backing fields (`ansi`, `display_target`, `display_filename`, `display_line_number`, `span_events` — lines 56-61, 63-73) are dead. `format_event` always writes `"span": null` when no span exists (:196) and `"kvs": {}` (:203) — constant boilerplate.
- **Suggestion:** strip the 5 config fields + builder methods; the format becomes a pure function of event metadata. (Replacing the whole formatter with `tracing_subscriber::fmt::format::Json` is possible but loses `[TAG]` extraction, so a slimmed custom formatter is the right target.)
- **Savings:** ~90-100 LOC.

### 5b. `logging/mod.rs` dead code + stale data
- `init_bevy_logging` (mod.rs:215-221) — 1-line wrapper around `init_session_logging`, never called (main.rs:296 uses `init_session_logging` directly). Dead.
- `get_session_log_path()` (mod.rs:224-228) — body is a comment saying "This is a placeholder", always returns `None`, never called. Dead.
- `SessionInfo.bevy_version` hardcoded `"0.16.1"` (mod.rs:75, asserted in test :239) — **stale**: project is Bevy 0.18. Either remove the field or use `bevy::utils::...`/env var.
- **Suggestion:** delete both functions; fix/remove the version field.
- **Savings:** ~20 LOC + correct metadata.

---

## 6. Diagnostics — entire module is dead code

`src/diagnostics/render_diagnostics.rs` (594) + `mod.rs` (12)

- **Evidence:** `RenderDiagnosticsPlugin` is imported at `lib.rs:104` but its registration is **commented out** (`lib.rs:1095-1096`). Every `log_*` call site in the rest of the codebase is commented out (`model_loader.rs:34,1369`; `render/world_ui.rs:55-56,358,526,567`). Three functions already carry `#[allow(dead_code)]` (`diagnostic_mesh_material_system` :499, `diagnostic_gpu_image_system` :532, `check_pipeline_cache_health` :563). The state vectors (`RenderDiagnosticsState`) grow and get cleared every 300 frames — pointless overhead if ever re-enabled.
- **Suggestion:** delete the module and its re-exports, or move the file to a `debug/` branch. If kept, at least delete the 3 `#[allow(dead_code)]` functions and the duplicate pair `log_alpha_blend_mesh_setup` (:274-327) / `log_alpha_blend_mesh_setup_simple` (:333-365) which differ only in the state-recording part (~40 LOC duplicated).
- **Savings:** **~594 LOC** (or ~90 if kept trimmed).

---

## 7. Long-function summary

| Function | Location | LOC |
|---|---|---|
| `GameClient::handle_packet` | game_client.rs:93-1163 | ~1070 |
| `GameClient::handle_client_message` | game_client.rs:1165-1635 | ~470 |
| `animation_sound_system` | animation_sound_system.rs:82-565 | ~484 |
| `spatial_sound_system` | spatial_sound.rs:84-228 | ~145 |
| `global_sound_system` | global_sound.rs:74-173 | ~100 |
| `log_render_state` / `log_pipeline_cache_access` | render_diagnostics.rs:419-493 / 174-232 | ~75 / ~60 |

---

## Prioritized summary (quick wins first)

1. **Delete `src/diagnostics/` (~600 LOC)** — plugin never registered, all call sites commented out. Pure dead code; fastest, safest win. *(Also deletes duplicated alpha-blend logging pair.)*
2. **Macro-ize packet boilerplate in `game_client.rs` (~600-700 LOC)** — `server_message!(...)` for the ~55 identity-conversion arms of `handle_packet`, `send_packet!(...)` for the ~55 arms of `handle_client_message` (also covers world/login clients' ~13 arms).
3. **Audio: collapse `SoundGain` to ratio-only + shared `spawn_spatial_sound` helper (~150 LOC)** — kills dead `Decibel` variant/5 match branches, unifies 8+ copy-pasted spawn bundles, collapses `spawn_loop_sound`/`spawn_one_shot_sound`, and fixes the boat `SoundCategory::PlayerFootstep` copy-paste bug.
4. **Trim logging module (~110 LOC)** — delete unused `TagExtractingJsonFormat` builder API + `init_bevy_logging` + `get_session_log_path`, fix stale `bevy_version: "0.16.1"`.
5. **Dead-event cleanup (~70 LOC)** — delete `SpawnEffectEvent::WithTransform` (+match arm), `PendingMonsterSound` component, unused `BloodEffectEvent` constructors; drop redundant `Event` derive in ping_event.rs.
6. (Nice-to-have) `streaming_sound.rs` fill_mono/fill_stereo dedup (~55 LOC, removes one `unsafe`).
7. (Nice-to-have) `network_thread_system.rs` channel-setup helper (~35 LOC).
8. (Bigger, needs assets) Replace procedural `boat_sound.rs` synthesis with real sound files (~100 LOC, adds 7 asset files).

**Estimated total savings: ~1,700-2,000 LOC out of ~5,600 analyzed** (roughly 30-35%), with items 1-5 being low-risk mechanical changes.

---

*This document is a PLAN ONLY. No files were modified during this analysis.*
