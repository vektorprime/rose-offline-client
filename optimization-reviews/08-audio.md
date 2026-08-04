# Optimization Review 08 — Audio Subsystem (WAV/OGG loading, streaming, spatial/global sound, BGM, monster caps, caches)

## 1. Title + Scope Summary

**Repo:** `rose-offline-client` (single crate, Bevy 0.18.1, custom oddio/cpal audio — `OddioPlugin` replaces Bevy audio)
**Files analyzed:** `src/audio/*` (mod, audio_source, streaming_sound, spatial_sound, global_sound, boat_sound, monster_sound_cap, wav, ogg), `src/systems/{background_music_system, animation_sound_system, npc_idle_sound_system, vehicle_sound_system}.rs`, `src/ui/{ui_sound_event_system, ui_window_sound_system}.rs`, `src/resources/{sound_cache, sound_settings}.rs`, `src/components/{sound_category, vehicle_sound}.rs`, plus call sites discovered: `src/effect_loader.rs`, `src/systems/{client_entity_event_system, use_item_event_system}.rs`, `src/zone_loader/spawning/objects.rs`, `src/lib.rs` (plugin + scheduling), `src/zone_loader.rs` (terrain lookup used by footstep logic).

**Arch docs that existed:** `system-architecture/Audio.md` (exists, covers architecture, cache strategy, BGM day/night, troubleshooting; accurate at a high level, does not analyze streaming internals, per-frame costs, or concurrency caps). `system-architecture/README.md` present. `pitfalls/index.md` exists; relevant entries: `pitfalls/boat-audio.md` (boat sound placeholder entities), `pitfalls/performance-memory.md` (lesson: no unbounded per-frame asset churn — relevant as the audio equivalent).

**Library versions (from Cargo.lock):** oddio 0.6.2, cpal 0.15, lewton 0.10, hound 3.4. oddio is *not* in `bevy-collection`; its behavior was verified against the vendored source at `%USERPROFILE%\.cargo\registry\src\index.crates.io-*\oddio-0.6.2\`.

## 2. Methodology

- Read every file listed above in full (all audio code paths and every call site of `SpatialSound`/`GlobalSound`/`SoundCache`/`queue_monster_sound` found via grep).
- Verified oddio 0.6.2 semantics from vendored source:
  - `StreamControl::write` (`stream.rs:120`) → `spsc::send_from_slice` (`spsc.rs:29`) is **non-blocking, partial-write** (fills available ring space, returns count). No main-thread blocking. The client's fill loop is therefore safe, but runs *every frame* per sound.
  - `Mixer::sample` (`mixer.rs:85-111`): the mixer's `Set` holds a strong `Arc` per signal; when the client handle drops, `handle_dropped()` is called but the signal **keeps playing until its ring drains** (`is_finished` → `stop` → `remove`). Conclusion: despawn does not truncate one-shots; it only stops *future* fills. The per-instance ring memory is freed only after the audio thread drains it.
  - `SpatialBuffered::new` (`spatial.rs:32-52`): allocates a `Ring` of `(max_distance / SPEED_OF_SOUND + buffer_duration) * rate` frames, **per spatial sound instance, on the calling (main) thread**.
  - Ear attenuation (`spatial.rs:548-551`): `distance_gain = radius / distance.max(radius)` — 1/r falloff with **no distance cutoff**; a radius-4 sound at 400 m is still -40 dB and still decoded/filled/mixed.
- Verified `get_terrain_height`/`get_tile_index` (`src/zone_loader.rs:358-419`) — bilinear heightmap + optional 4-octave f64 Perlin (via `terrain/noise_overlay.rs:98-132`; disabled by default, `noise_enabled=false`).
- Confirmed `SoundData.max_mix_count` (`rose-data/src/sound_database.rs:14`) is **never used anywhere in the client** (grep over `src/` → 0 hits).
- Confirmed `SoundCache::clear()` has **no callers**.
- No code was modified; no builds were run.

## 3. Findings

### F1 — All Vorbis decoding runs on the main thread, in bursts, in the `Last` schedule
- **Where:** `src/audio/ogg.rs:73-92` (`read_packet` → `lewton` `read_dec_packet_generic`), driven by the fill loop `src/audio/streaming_sound.rs:57-95`, called from `src/audio/spatial_sound.rs:159-163` and `src/audio/global_sound.rs:88-105`. Both systems run in `Last` (`src/audio/mod.rs:143-153`).
- **Detail:** OGG files are *not* decoded at load time (`ogg.rs:30-38`, `decoded: None`); each *playing instance* decodes packets on the main thread. Ring buffers: spatial = `sample_rate/8` (~125 ms), global/BGM = `sample_rate/2` (~500 ms). Refill bursts: when a ring drains, one frame decodes ~5 packets (spatial) or ~20-25 packets (BGM) back-to-back. lewton is a slow pure-Rust Vorbis decoder (~0.2-1 ms/packet depending on rate/channels).
- **Impact:** every active OGG stream costs single-digit % of one core, but the burst structure produces frame spikes (BGM: one ~5-20 ms spike every ~0.5 s; each spatial OGG: ~1-5 ms spike every ~125 ms). With several concurrent OGG sources (BGM + ambient sound objects + effect OGGs) this is visible as micro-stutter and is pure main-thread load.
- **Fix sketch:** (a) for short/repeating OGGs, decode once on the asset-load task into `AudioSourceDecoded` (same as `WavLoader`) — loops then become instant `Buffered` playback with zero per-frame decode; (b) for BGM, cap decode per frame (`decode only N ms per frame`, refill over several frames) or move decode to a background thread feeding a channel.
```rust
// (b) chunked refill inside fill(): instead of looping read_packet until ring full,
// stop after ~50ms of audio per call and return true (refill again next frame).
```

### F2 — No distance culling anywhere: distant sounds are decoded, filled, and mixed for nothing
- **Where:** `src/audio/spatial_sound.rs:125-169` (every `SpatialSound` gets fill + `set_motion` every frame, regardless of distance), `src/audio/monster_sound_cap.rs:78-87` (`queue_monster_sound` computes `distance_to_player` but only uses it to *sort*, never to reject), oddio `spatial.rs:551` (no attenuation cutoff).
- **Detail:** the spatial system is the only gate; with `SoundRadius` default 4.0, a monster sound at 150 m is at ~-31 dB yet still: spawns, allocates its ring, fills every frame, and is mixed on the audio thread. Zone ambient sound objects (`src/zone_loader/spawning/objects.rs:558`) are repeating and are filled every frame forever.
- **Impact:** cost scales with *all* active sounds, not audible ones. In a zone with 100+ ambient objects + combat noise, the majority of fill/mixing work is inaudible. This is the single largest scaling problem.
- **Fix sketch:**
```rust
// In queue_monster_sound: reject outright
if distance_to_player > AUDIBLE_CUTOFF { return; }
// In spatial_sound_system: pause + stop filling distant sounds
let dist = (sound_global_translation - listener_position).length();
if dist > cutoff { if let Some(h) = handle { h.stop_control().pause(); } continue; }
else { if let Some(h) = handle { h.stop_control().resume(); } /* fill */ }
```
Use a soft range (e.g. 60-100 m) — beyond oddio's attenuation is inaudible anyway (see F4 for `max_distance` interplay).

### F3 — "Monster sound cap" limits *spawns per frame*, not *concurrent sounds*
- **Where:** `src/audio/monster_sound_cap.rs:14` (`MAX_CONCURRENT_MONSTER_SOUNDS = 3`), `:46-61` (drain + take(3) per frame).
- **Detail:** 3 spawns/frame × 60 fps = up to 180 one-shots spawned/s; with 0.3-1 s durations that sustains ~90-150 *simultaneously active* spatial sounds (each: ~275 KB ring, per-frame fill, per-callback mixing — see F4). The name promises concurrency limiting but nothing counts currently-playing sounds. Additionally `SoundData.max_mix_count` (present in the database, `rose-data/src/sound_database.rs:14`) is completely unused — it was the original game's per-sound concurrency limit.
- **Impact:** heavy fights inflate audio-thread mixing and main-thread fill work far beyond the intended budget; cap is effectively 180/s.
- **Fix sketch:** keep a `Resource { active_sounds: usize }` — increment on spawn, decrement when a sound finishes (already have despawn points in `spatial_sound_system`/`global_sound_system`); refuse spawns when `active_sounds >= MAX_ACTIVE`. Optionally respect `sound_data.max_mix_count` per id (cheap: one counter per SoundId).
```rust
fn spawn_sound(...) {
    if active.0 >= MAX_ACTIVE { return; }
    active.0 += 1; // decremented in spatial/global sound system on despawn
    ...
}
```

### F4 — Every spatial sound allocates a ~275 KB delay ring on the main thread
- **Where:** `src/audio/spatial_sound.rs:186-196` — `play_buffered(..., max_distance: 500.0, rate, buffer_duration: 0.1)`.
- **Detail:** oddio `SpatialBuffered::new` allocates `(500/343 + 0.1) × 44100 ≈ 68,700` mono f32 frames ≈ **275 KB per instance**, allocated at spawn time on the main thread, freed only after the audio thread drains it. For one-shots (footsteps, monster hits — F3 churn), this is pure allocator traffic: e.g. 90 concurrent sounds ≈ 25 MB resident + continuous churn. 500 m is also far beyond any practical audibility, so the buffer is sized for delay that can never be heard.
- **Impact:** main-thread allocation stalls during combat bursts; tens of MB resident on the audio side.
- **Fix sketch:** (a) shrink `max_distance` to match the F2 cutoff (e.g. 100 → ~55 KB/sound, factor 5 reduction); (b) bigger win — for `Buffered` (decoded WAV) one-shots, bypass `Stream`+`play_buffered` entirely and use oddio's seekable `Frames` signal with `play()` (`oddio spatial.rs:307`, `frames.rs`), which needs no delay ring and no per-frame filling.
```rust
// if decoded && !repeating && !spatial_velocity_needed:
let frames = oddio::Frames::from_slice(rate, &decoded.samples); // mono interleave
player.control().play(oddio::Gain::new(frames), SpatialOptions { radius, .. });
// handle: Stop<Gain<FramesSignal>> — no per-frame fill required
```

### F5 — Every active spatial sound gets position/velocity math + a `set_motion` control write every frame
- **Where:** `src/audio/spatial_sound.rs:136-152` (velocity guess + `relative_velocity / time.delta_secs()`), `:155-156` (normalize + length), `:165-169` (`set_motion`), `:121-123` (listener rotation every frame).
- **Detail:** two sqrts, a division, and an atomically-swapped control message per sound per frame — done even for static zone ambience at 100 m that hasn't moved. Listener rotation is also rewritten with identical values every frame.
- **Impact:** minor per-sound, but it multiplies with F2/F3 (100+ sounds → 100+ sqrts + 100+ atomic swaps + 100+ RefCell control borrows per frame in `Last`).
- **Fix sketch:** throttle to 10-20 Hz per entity (store `last_update: f32` in `SpatialSound`), skip `set_motion` when position delta < epsilon; only call `set_listener_rotation` when the camera rotation actually changed.

### F6 — Footstep sounds run full terrain-height + tile lookups for every entity's every step, with no distance check first
- **Where:** `src/systems/animation_sound_system.rs:119-167` — per `SOUND_FOOTSTEP` event (any entity, any distance) it calls `current_zone_data.get_terrain_height(...)` and `get_tile_index(...)` (`src/zone_loader.rs:358-419`), then `get_step_sound(...)`, then queues the sound.
- **Detail:** terrain height does 4 clamped heightmap reads + bilinear interp, and `get_terrain_height` additionally evaluates the thread-local Perlin noise overlay (`src/terrain/noise_overlay.rs:152-160`; 4-octave f64 when enabled — currently default off, so mostly a thread-local borrow). All of this runs for footsteps of NPCs 500 m away whose sounds will be capped/dropped by the monster queue anyway (F3) — the lookup happens *before* any distance filter.
- **Impact:** wasted per-event CPU proportional to total NPC count walking; the only "cost proportional to what you hear" guarantee in this system is currently missing.
- **Fix sketch:** cheap distance gate first:
```rust
let step_pos = event_entity_full.global_transform.translation();
if step_pos.distance_squared(player_position) > (80.0 * 80.0) { continue; }
```
Also hoist `SoundId::new(653).unwrap()` (`:120`) to a `const` — it's currently re-created per footstep event.

### F7 — BGM "crossfade" is a fake: 2-second silence pause, frame-counted, no gain ramp
- **Where:** `src/systems/background_music_system.rs:10, 69-100` — `FadingOut` merely waits `timer_ms += 16` (fixed per frame, not `time.delta()`), old track keeps playing at full volume, new track spawns only *after* the old is despawned. No overlap, no gain fade.
- **Detail:** `*timer_ms += 16` makes the "2000 ms" duration frame-rate dependent (4 s at 120 fps, 1 s at 30 fps). During the fade window the game plays music that's about to be cut, then abruptly switches.
- **Impact:** gameplay quality, not CPU; trivial fix. (No extra mixing cost today — only one track ever plays — so the fix must keep that property or budget two tracks during overlap.)
- **Fix sketch:** on transition, spawn the new track immediately and fade the old one with its `GainControl` (`set_amplitude_ratio` ramped over `CROSSFADE_DURATION` via a small `Local` timer), despawn when it reaches 0. Use `time.delta()`.

### F8 — `SoundCache` uses a `std::sync::RwLock` + per-event `Handle` clone for main-thread-only access
- **Where:** `src/resources/sound_cache.rs:10-47` — every `load()` does a read-lock, `Vec` index, and `Handle` clone (two atomic refcount ops); `set()` takes a write lock. All call sites are the main thread.
- **Detail:** cache size = `sounds.len()` (`src/lib.rs:1813`); out-of-range `SoundId`s are silently ignored by both `get` and `set` (`:24-25, :41-44`) → permanent cache misses that redo `asset_server.load` + a `String` alloc (`to_string_lossy().into_owned()`) on every event.
- **Impact:** small constant overhead per sound event (footstep/attack/UI) — worth fixing only as hygiene; the out-of-range miss path is the real (though rare) waste.
- **Fix sketch:** replace with a plain `Vec<Option<Handle<AudioSource>>>` in a `Resource` (no `RwLock`), `debug_assert!(id < len)` in `set`, and keep the OOB fallback returning a fresh load.

### F9 — OGG streaming allocates a new `Vec` per packet and memmoves its staging buffer
- **Where:** `src/audio/ogg.rs:73-92` (`read_packet` returns a fresh `packet.samples` Vec each call — ~40 calls/s per active stream), `src/audio/streaming_sound.rs:60-64, 88` (`buffer.drain(0..samples_read)` shifts the tail; `extend_from_slice` may reallocate).
- **Impact:** thousands of small allocations + memmoves per second across all active OGG streams (scales with F2/F3). Low per-alloc, but trivially avoidable.
- **Fix sketch:** keep a reusable scratch `Vec<f32>` in `StreamingSound::Streaming` and `mem::swap` with the packet samples instead of `extend_from_slice`/`drain`; replace the drain-based buffer with a read cursor (`start: usize`).

### F10 — Repeating OGG sources re-parse headers on every loop (main thread)
- **Where:** `src/audio/ogg.rs:68-71` — `rewind()` calls `seek_absgp_pg(0)` which re-reads the header chain; invoked by `streaming_sound.rs:71-75` at every loop boundary (BGM: every ~2-4 min; ambient loops: every few seconds).
- **Impact:** small periodic main-thread cost + risk of stutter at loop points; unnecessary for short loops.
- **Fix sketch:** pre-decode repeating OGGs into `AudioSourceDecoded` (set `decoded: Some(...)` at load for short files) — `StreamingSound::new` then picks the `Buffered` path (`streaming_sound.rs:18-22`), which loops with an index reset and zero decode/alloc.

### F11 — All played WAVs stay fully decoded in RAM for the session; cache is never evicted or cleared
- **Where:** `src/audio/wav.rs:39-57` (full decode to `Vec<f32>` at load; `bytes: Arc::new([])` — the raw file is *not* retained, so re-decode requires re-reading from the VFS), `src/resources/sound_cache.rs:49-51` (`clear()` has zero callers).
- **Detail:** `SoundCache` holds strong handles forever, and `Assets<AudioSource>` keeps decoded PCM alive. Session memory grows to roughly the sum of every sound ever played (footsteps + hit/attack variants + UI blips + NPC chatter). At ~176 KB/s (mono 44.1 kHz f32) × 0.3-3 s each, hundreds of sounds can reach tens-to-hundreds of MB.
- **Impact:** slow, unbounded main-RAM growth — the audio analogue of the `performance-memory.md` pitfall (unbounded asset retention).
- **Fix sketch:** LRU the cache (`clear()` on zone change loses nothing — handles re-load from VFS and Bevy caches assets; only decode cost is re-incurred) or cap resident decoded assets (drop handles beyond N; bevy `Assets::remove`).

### F12 — Effect sounds and zone sound objects bypass `SoundCategory`/`SoundSettings` entirely
- **Where:** `src/effect_loader.rs:174-199` (spawns `SpatialSound`/`GlobalSound` with `SoundGain::default()` = 1.0 and *no* `SoundCategory`), `src/zone_loader/spawning/objects.rs:558-567` (ambient objects: no `SoundGain`, no `SoundCategory`).
- **Detail:** these sounds cannot be volume-controlled (settings sliders, mute) and the `*_gain_changed_system`s (`spatial_sound.rs:69-78`, `global_sound.rs:59-68`) can never adjust them. `ui_sound_event_system` (`src/ui/ui_sound_event_system.rs:32-36`) and BGM do it right.
- **Impact:** correctness/UX (uncontrollable loudness), no CPU. Cheap fix: thread `sound_settings.gain(category)` + `SoundCategory` through `effect_loader`/`objects.rs`.

### F13 — Boat loop system rewrites `SoundGain` every frame even when unchanged
- **Where:** `src/audio/boat_sound.rs:275-300` — unconditional `*gain = SoundGain::Ratio(...)` per loop sound per frame.
- **Detail:** every write flips change detection, so `spatial_sound_gain_changed_system` (`spatial_sound.rs:69-78`) fires and calls `set_amplitude_ratio` for all 3 boat loop sounds every frame. Also note `SoundCategory::PlayerFootstep` is used for boat sounds (`:170, :192`) — correct only by accident (combat/other volumes can't reach boat sounds).
- **Impact:** trivial CPU; churn is unnecessary.
- **Fix sketch:** compare before writing (`if (new - old).abs() > 1e-4 { *gain = ...; }`).

### F14 — Dead `&mut GlobalSound` query parameter in `background_music_system`
- **Where:** `src/systems/background_music_system.rs:42` — `mut query_global_sounds: Query<&mut GlobalSound>` is never used in the body.
- **Detail:** an unused `&mut` query still registers exclusive write access to `GlobalSound`'s storage in the `Update` schedule, serializing the system against any future `Update` system touching `GlobalSound`.
- **Impact:** none today, latent conflict + scheduler pessimism.
- **Fix sketch:** delete the parameter.

### F15 — NaN risk in spatial position/velocity math
- **Where:** `src/audio/spatial_sound.rs:155-156` — `(sound_global_translation - camera_position).normalize()` → NaN when the sound is exactly at the camera (0-length vector; e.g., sounds spawned at the player position with a camera near/inside the player); `:151` — `relative_velocity / time.delta_secs()` → Inf if `delta_secs == 0.0`.
- **Impact:** NaN/Inf motion fed into oddio's spatial mixer → potential audio glitches or NaN propagation; intermittent.
- **Fix sketch:** `let dir = (pos - cam).try_normalize().unwrap_or(Vec3::Z);` and guard `delta.max(1e-6)`.

### F16 — Stereo OGG played through the spatial path is mis-sampled (plays at double rate)
- **Where:** `src/audio/spatial_sound.rs:159-160` — `fill_mono` writes interleaved stereo samples into a mono `Stream` at the source rate; `src/audio/ogg.rs:59-66` reports `channel_count = 2` for stereo files but `spatial_sound_system` always uses `fill_mono` (`:160`).
- **Detail:** a stereo OGG as a spatial sound consumes `2×rate` samples/sec into a `rate`-tick stream → ~2× pitch/speed. `global_sound_system` handles channels correctly (`global_sound.rs:115-148`); spatial doesn't. (If all irose SFX are mono this never triggers, but effect files can be stereo.)
- **Fix:** downmix stereo → mono at decode (or per-packet) before spatial filling; assert/downmix in `StreamingSound::fill_mono` when `channel_count == 2`.

### F17 — Vehicle sounds bypass `SoundCache` and rebuild path strings on every toggle
- **Where:** `src/systems/vehicle_sound_system.rs:82, 107` — `asset_server.load(sound_data.path.path().to_string_lossy().into_owned())` per idle↔move switch; `:30` — `query_vehicle_model.get_mut(...).unwrap()` can panic if the model entity is missing.
- **Impact:** trivial (Bevy dedups handles); hygiene + robustness.
- **Fix sketch:** route through `SoundCache` (it's keyed by `SoundId`, which `sound_data` carries), replace `unwrap` with `let Ok(...) = ... else { continue }`.

### F18 — One-shot despawn releases the `Stream` handle before the ring drains — acceptable but worth knowing
- **Where:** `src/audio/spatial_sound.rs:171-175` / `src/audio/global_sound.rs:107-109` — despawn on `!has_more_audio`.
- **Detail:** verified in oddio `mixer.rs:85-111` that the mixer keeps a strong ref and plays the ring out to `is_finished` — so no audible truncation (up to 125 ms spatial / 500 ms global tail plays out). The cost: a despawned one-shot still occupies its ring (~22-275 KB, F4) and mixes on the audio thread until drained. No fix needed; noted to avoid a future "optimization" that kills the tail (e.g., calling `stop()` on despawn would truncate audio).

## 4. Priority-Ranked Summary

| # | Finding | Impact | Effort | Priority |
|---|---------|--------|--------|----------|
| F2 | No distance culling (fill/mix everything) | High — scales with all active sounds | Low-Med | **High** |
| F3 | Monster cap = spawns/frame, not concurrency | High — 90-150 concurrent one-shots in combat | Low-Med | **High** |
| F4 | ~275 KB ring per spatial sound (main-thread alloc) | Med-High — alloc churn, memory | Low (max_distance) / Med (Frames path) | **High** |
| F1 | OGG decode on main thread, bursty refills | Med — frame spikes, BGM stutter | Med | Med-High |
| F6 | Footstep terrain lookups before distance check | Med — wasted work × NPC count | Low | Med |
| F7 | Fake crossfade, frame-counted timer | Med (quality) | Low | Med |
| F10 | Repeating OGGs re-parse headers per loop | Low-Med | Low | Med |
| F11 | WAVs decoded forever, no eviction | Med (memory) | Low-Med | Med |
| F5 | Per-frame motion update for all sounds | Low-Med | Low | Med |
| F8 | RwLock + handle-clone cache | Low | Low | Low |
| F9 | Packet Vec alloc + drain memmove | Low | Low | Low |
| F12 | Effect/zone sounds bypass volume settings | Low (UX) | Low | Low |
| F13 | Boat gain rewrite every frame | Low | Trivial | Low |
| F14 | Dead `&mut GlobalSound` query | Low (latent) | Trivial | Low |
| F15 | NaN in spatial math | Low | Trivial | Low |
| F16 | Stereo OGG as spatial sound = 2× pitch | Low (if any stereo SFX) | Low | Low |
| F17 | Vehicle sounds bypass cache / unwrap | Low | Trivial | Low |
| F18 | Despawn/ring-drain semantics (informational) | — | — | — |

## 5. Quick Wins

1. **Distance cutoff in `queue_monster_sound`** (F2): one `if distance > cutoff { return; }` — kills the majority of wasted spawn/fill/mix work in busy zones.
2. **`max_distance: 500.0 → 100.0`** in `spatial_sound.rs:193` (F4): 5× smaller per-sound ring (~275 KB → ~55 KB), aligns with F2's cutoff.
3. **Delete the unused `query_global_sounds` parameter** in `background_music_system.rs:42` (F14).
4. **Boat gain write-on-change** (F13): 3-line change, stops per-frame gain-control churn.
5. **Distance gate before footstep terrain lookup** (F6): one `distance_squared` check at `animation_sound_system.rs:150`.
6. **Epsilon-guard the normalize + delta** (F15): `try_normalize` + `delta.max(1e-6)`.
7. **Drop the `RwLock`** in `SoundCache` (F8) — main-thread-only access.
8. **`const DEFAULT_STEP_SOUND: SoundId = SoundId::new(653)`** — hoist per-event unwrap (F6/F18 area).

## 6. Risks / Considerations

- **Culling cutoffs must not cause audible pops.** Pausing a `Stop` control mid-playback clicks. When implementing F2, ramp via `GainControl` to 0 before `pause()`, and make the resume threshold (e.g. 50 m) tighter than the pause threshold (80 m) to avoid hysteresis churn.
- **F4's `Frames`-based path changes the control graph** (`SpatialBuffered<Stop<Gain<Stream>>>` → `Spatial<Stop<Gain<FramesSignal>>>`): gain/stop/stream controls differ (`stream_control` disappears; `Frames` handles its own position). Requires reworking `SpatialControlHandle` (`spatial_sound.rs:16-36`) and keeping the repeating/one-shot + rewind behavior identical. Do it behind the `Buffered` variant only, and keep the streaming path for OGG.
- **Pre-decoding OGGs (F1/F10) trades CPU for memory**: a 3-min stereo BGM at 44.1 kHz f32 ≈ 63 MB. Only pre-decode short/repeating SFX and ambient loops; keep BGM streaming (chunked refill or background thread instead).
- **`max_distance` < real listener distance causes delay-line clamps** (oddio `spatial.rs:418` debug_assert). If F2's cutoff is 100 m, set `max_distance ≈ 150-200` as a safety margin; don't shrink below audibility + margin.
- **F3's active-count bookkeeping must be exact**: a lost increment (spawn path that never despawns — e.g., the failed-load repeating-sound path at `spatial_sound.rs:209-219`) would silently starve all monster sounds. Count only on successful control-handle creation, decrement where the handle is released (including failed-load despawn).
- **BGM overlap during a true crossfade (F7)** doubles mixing cost briefly; keep the overlap ≤ 2 s and ensure the old track's `GainControl` ramp is the only per-frame write.
- **`SoundCache::clear()`/LRU (F11)** must not drop handles that are still playing (playing `GlobalSound`/`SpatialSound` components hold their own `Handle<AudioSource>` clones — safe), but bevy's `Assets` will unload the decode while a *queued-but-not-yet-started* entity still references it — it will simply reload on the next `StreamingSound::new` (already the behavior today for OOB ids).
- **Scheduling**: `spatial_sound_system`/`global_sound_system` run in `Last` (`mod.rs:143-153`) — all audio work is serialized at frame end. Moving fill work to a mid-`Update` slot with ordering constraints would parallelize it with render work; keep the `process_monster_sound_queue_system.before(spatial_sound_system)` invariant.
- **Verified non-issues** (do not "optimize" these): `StreamControl::write` is non-blocking SPSC (no main-thread blocking); despawn does not truncate ring tails (mixer keeps strong refs — F18); WAV decode happens on Bevy's asset task pool, not the main thread; sample-rate conversion is handled inside oddio (no 22.05 kHz boat sounds / device-rate mismatch problems).
