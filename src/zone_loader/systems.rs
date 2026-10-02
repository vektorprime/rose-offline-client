use super::loading::load_zone;
use super::spawning::spawn_zone;
use super::*;

/// `zone_loader_system` state for answering a repeated `LoadZoneEvent` from the zone
/// already displayed, and for freeing zone assets nothing uses any more.
#[derive(Default)]
pub struct ZoneReuseState {
    /// Key of the most recent successful load; `None` after a failed one.
    last_loaded: Option<ZoneLoadKey>,
    /// Zones requested again while displayed. Answered on the next run, the earliest
    /// an async load can answer, so no new event timing is introduced.
    pending: Vec<ZoneId>,
    /// Every zone asset this system added to `Assets<ZoneLoaderAsset>`.
    loaded_assets: Vec<Handle<ZoneLoaderAsset>>,
}

pub fn zone_loader_system(
    mut loading_zones: Local<Vec<LoadingZone>>,
    mut zone_reuse: Local<ZoneReuseState>,
    mut load_zone_events: MessageReader<LoadZoneEvent>,
    mut zone_loaded_from_vfs_events: MessageWriter<ZoneLoadedFromVfsEvent>,
    zone_load_receiver: ResMut<ZoneLoadChannelReceiver>,
    zone_load_sender: Res<ZoneLoadChannelSender>,
    mut spawn_zone_params: SpawnZoneParams,
    current_zone: Option<Res<CurrentZone>>,
    zones: Query<&Zone>,
) {
    let _span = info_span!("zone_loader_system").entered();
    free_unreferenced_zone_assets(
        &mut zone_reuse.loaded_assets,
        &mut spawn_zone_params.zone_loader_assets,
    );

    let has_load_events = load_zone_events.len() > 0;
    let has_loading_zones = !loading_zones.is_empty() || !zone_reuse.pending.is_empty();

    // Early return if no zones are loading and no load events to process
    // This prevents unnecessary memory allocations from logging every frame
    if !has_load_events && !has_loading_zones {
        return;
    }

    // Log periodic memory summary
    spawn_zone_params.memory_tracking.log_summary();

    // Check for loaded zones from async tasks via channel
    while let Ok((zone_id, zone_asset_result)) = zone_load_receiver.0.lock().unwrap().try_recv() {
        let zone_id: ZoneId = zone_id;
        let zone_asset_result: Result<ZoneLoaderAsset, anyhow::Error> = zone_asset_result;
        match zone_asset_result {
            Ok(zone_asset) => {
                // Remove the zone from the loading queue since it's now received from channel
                let load_key =
                    if let Some(pos) = loading_zones.iter().position(|lz| lz.zone_id == zone_id) {
                        Some(loading_zones.remove(pos).load_key)
                    } else {
                        log::warn!(
                            "[ZONE LOADER SYSTEM] Could not find zone {} in loading queue to remove",
                            zone_id.get()
                        );
                        None
                    };

                // CRITICAL FIX: Add the zone asset to the Assets collection HERE where we have ownership
                // This allows collision_player_system to access terrain height data
                let zone_handle = spawn_zone_params.zone_loader_assets.add(zone_asset);
                log::info!(
                    "[ZONE LOADER SYSTEM] Zone {} added to Assets collection with handle: {:?}",
                    zone_id.get(),
                    zone_handle
                );
                zone_reuse.loaded_assets.push(zone_handle.clone());
                // A load that timed out of the queue has no key and is never reused.
                zone_reuse.last_loaded = load_key;

                // Send event with the handle (not the Arc) to zone_loaded_from_vfs_system for spawning
                zone_loaded_from_vfs_events
                    .write(ZoneLoadedFromVfsEvent::new(zone_id, zone_handle));
            }
            Err(e) => {
                log::error!(
                    "[ZONE LOADER SYSTEM] Failed to load zone {} from async task: {:?}",
                    zone_id.get(),
                    e
                );
                zone_reuse.last_loaded = None;

                // Remove from loading queue
                if let Some(pos) = loading_zones.iter().position(|lz| lz.zone_id == zone_id) {
                    loading_zones.remove(pos);
                }
            }
        }
    }

    // Answer the zones requested again on the previous run (see below).
    for zone_id in std::mem::take(&mut zone_reuse.pending) {
        let load_key = ZoneLoadKey::new(zone_id, &spawn_zone_params);
        match reusable_zone(&zone_reuse, load_key, current_zone.as_deref(), &zones) {
            Some(zone_handle) => {
                log::info!(
                    "[ZONE LOADER SYSTEM] Zone {} is already displayed and unchanged, reusing its loaded data",
                    zone_id.get()
                );
                zone_loaded_from_vfs_events
                    .write(ZoneLoadedFromVfsEvent::new(zone_id, zone_handle));
            }
            // Replaced or changed since the request: load it like any other zone.
            None => start_zone_load(
                zone_id,
                load_key,
                &mut loading_zones,
                &zone_load_sender,
                &spawn_zone_params,
            ),
        }
    }

    for event in load_zone_events.read() {
        // DIAGNOSTIC: Track LoadZoneEvent received
        log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] LoadZoneEvent received: zone_id={}, despawn_other_zones={}",
            event.id.get(), event.despawn_other_zones);

        // CRITICAL FIX: Check for duplicate zone loading to prevent memory leaks
        // and double-spawning of the same zone
        let is_already_loading = loading_zones.iter().any(|lz| lz.zone_id == event.id)
            || zone_reuse.pending.contains(&event.id);
        if is_already_loading {
            log::warn!("[ZONE LOADER SYSTEM] Zone {} is already loading via async task, skipping duplicate request",
                event.id.get());
            continue;
        }

        let load_key = ZoneLoadKey::new(event.id, &spawn_zone_params);
        if reusable_zone(&zone_reuse, load_key, current_zone.as_deref(), &zones).is_some() {
            // Same-zone respawn/teleport, character select back to login, ...: the zone
            // is displayed, so zone_loaded_from_vfs_system would discard a new load
            // ("already exists") and only report it loaded. Skip reading it again.
            zone_reuse.pending.push(event.id);
            continue;
        }

        start_zone_load(
            event.id,
            load_key,
            &mut loading_zones,
            &zone_load_sender,
            &spawn_zone_params,
        );
    }

    // Async loads stay queued until their result arrives through the channel
    loading_zones.retain(|loading_zone| {
        // Check for timeout (30 seconds)
        let timed_out = loading_zone.loading_start_time.elapsed() > Duration::from_secs(30);
        if timed_out {
            log::error!(
                "[ZONE LOADER SYSTEM] Zone {} loading timeout after 30s, removing from queue",
                loading_zone.zone_id.get()
            );
        }
        !timed_out
    });
}

/// Starts loading `zone_id` on the AsyncComputeTaskPool. The result arrives through
/// the zone load channel.
fn start_zone_load(
    zone_id: ZoneId,
    load_key: ZoneLoadKey,
    loading_zones: &mut Vec<LoadingZone>,
    zone_load_sender: &ZoneLoadChannelSender,
    spawn_zone_params: &SpawnZoneParams<'_, '_>,
) {
    // WORKAROUND: Load zone directly from VFS without using AssetServer
    let vfs = spawn_zone_params.vfs_resource.vfs.clone();
    let base_path = spawn_zone_params.vfs_resource.base_path.clone();
    let use_new_terrain = load_key.use_new_terrain;
    // The load task builds terrain meshes with it (see load_zone).
    let terrain_noise = Arc::new((*spawn_zone_params.terrain_noise).clone());
    let tx = zone_load_sender.0.clone();

    // Check if pool is initialized and get reference
    let pool = match AsyncComputeTaskPool::try_get() {
        Some(pool) => pool,
        None => {
            log::error!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is NOT initialized! Cannot spawn async task!");
            log::error!("[ZONE LOADER SYSTEM] This is likely why zones are not loading!");
            // DO NOT spawn the task - skip this zone
            return;
        }
    };

    // Spawn async task to load zone using AsyncComputeTaskPool
    let task = pool.spawn(async move {
        match load_zone(zone_id, vfs, base_path, use_new_terrain, terrain_noise).await {
            Ok(zone_asset) => {
                if let Err(e) = tx.send((zone_id, Ok(zone_asset))) {
                    log::error!("[ZONE LOADER DIRECT TASK] Failed to send zone through channel: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("[ZONE LOADER DIRECT TASK] Failed to load zone {}: {:?}", zone_id.get(), e);
                if let Err(send_err) = tx.send((zone_id, Err(e))) {
                    log::error!("[ZONE LOADER DIRECT TASK] Failed to send error through channel: {:?}", send_err);
                }
            }
        }
    });

    // Detach the task so it runs in the background
    task.detach();

    // Add zone to loading queue to track that it's being loaded
    // This ensures we know the zone is in progress even though spawning is handled by zone_loaded_from_vfs_system
    loading_zones.push(LoadingZone {
        zone_id,
        loading_start_time: Instant::now(),
        load_key,
    });
}

/// The displayed zone's data handle if a new load of `load_key` can be skipped:
/// a Zone entity with that id exists (zone_loaded_from_vfs_system then only reports
/// it loaded, whatever data it gets), and the most recent load was of this zone with
/// nothing it depends on changed since.
fn reusable_zone(
    zone_reuse: &ZoneReuseState,
    load_key: ZoneLoadKey,
    current_zone: Option<&CurrentZone>,
    zones: &Query<&Zone>,
) -> Option<Handle<ZoneLoaderAsset>> {
    if zone_reuse.last_loaded != Some(load_key) {
        return None;
    }
    let current_zone = current_zone.filter(|current_zone| current_zone.id == load_key.zone_id)?;
    zones
        .iter()
        .any(|zone| zone.id == load_key.zone_id)
        .then(|| current_zone.handle.clone())
}

/// `Assets<ZoneLoaderAsset>` is initialised as a plain resource (no asset tracking
/// systems run for it), so a zone asset is not freed when its last handle drops.
/// Free each zone loaded here once `loaded_assets` holds its only handle: every user
/// (CurrentZone, events, spawning) reaches zone data through a strong handle.
fn free_unreferenced_zone_assets(
    loaded_assets: &mut Vec<Handle<ZoneLoaderAsset>>,
    zone_loader_assets: &mut ResMut<Assets<ZoneLoaderAsset>>,
) {
    loaded_assets.retain(|handle| {
        let Handle::Strong(strong_handle) = handle else {
            return false;
        };
        if Arc::strong_count(strong_handle) > 1 {
            return true;
        }
        if let Some(zone_asset) = zone_loader_assets.remove_untracked(handle) {
            // Dropping a zone's block data takes a moment; keep it off the main thread.
            match AsyncComputeTaskPool::try_get() {
                Some(pool) => pool.spawn(async move { drop(zone_asset) }).detach(),
                None => drop(zone_asset),
            }
        }
        false
    });
}

/// System to handle spawning zones that were loaded from VFS via async tasks
/// This separate system avoids borrow checker conflicts by handling spawning independently
/// CRITICAL FIX: Process ALL events, not just one, to prevent event queue buildup
/// CRITICAL FIX: Deduplicate events and prevent spawning already-loaded zones
pub fn zone_loaded_from_vfs_system(
    mut events: MessageReader<ZoneLoadedFromVfsEvent>,
    // Zone entities spawned here and not despawned yet. Holds no zone data handle,
    // so a replaced zone's data can be freed (see zone_loader_system).
    mut spawned_zones: Local<Vec<Entity>>,
    mut zone_events: MessageWriter<ZoneEvent>,
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut spawn_zone_params: SpawnZoneParams,
    // CRITICAL FIX: Query existing zones to prevent duplicate spawning
    existing_zones: Query<(Entity, &Zone)>,
) {
    let _span = info_span!("zone_loaded_from_vfs_system").entered();
    let event_count = events.len();
    if event_count == 0 {
        return;
    }

    // CRITICAL FIX: Check for already-loaded zones to prevent duplicates
    let already_loaded: std::collections::HashSet<u16> = existing_zones
        .iter()
        .map(|(_, zone)| zone.id.get())
        .collect();

    spawn_zone_params.memory_tracking.log_summary();

    let mut processed_count = 0;

    // CRITICAL FIX: Deduplicate events - prevent duplicate zone IDs in same batch
    let mut seen_zone_ids: std::collections::HashSet<u16> = std::collections::HashSet::new();

    for event in events.read() {
        // Deduplicate: Skip duplicate zone IDs in the same batch
        if !seen_zone_ids.insert(event.zone_id.get()) {
            log::warn!(
                "[ZONE LOADED FROM VFS] DUPLICATE EVENT for zone {} ignored in batch",
                event.zone_id.get()
            );
            continue;
        }

        // CRITICAL FIX: Skip zones that are already loaded
        if already_loaded.contains(&event.zone_id.get()) {
            log::warn!("[ZONE LOADED FROM VFS] Zone {} already exists, skipping spawn to prevent memory leak",
                event.zone_id.get());
            // FIX: Still send ZoneEvent::Loaded so JoinZoneRequest is sent to server
            // This is critical for respawn scenarios where the player needs to re-join the zone
            zone_events.write(ZoneEvent::Loaded(event.zone_id));
            continue;
        }

        processed_count += 1;

        // CRITICAL FIX: Handle despawn_other_zones flag (matching AssetServer path behavior)
        // Default to true to match the typical behavior when loading a new zone
        let despawn_other_zones = true;

        if despawn_other_zones {
            for spawned_entity in spawned_zones.drain(..) {
                log::warn!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✗ Despawning existing zone entity: entity={:?}", spawned_entity);
                spawn_zone_params.commands.entity(spawned_entity).despawn();
                spawn_zone_params.memory_tracking.log_entity_despawned();
            }

            spawn_zone_params.commands.remove_resource::<CurrentZone>();
        }

        // CRITICAL FIX: The zone asset was already added to the Assets collection in zone_loader_system
        // We just need to use the handle from the event to spawn the zone
        let zone_handle = event.zone_handle.clone();

        // Spawn the zone using the asset from the collection (via handle)
        // The asset is temporarily removed from the Assets collection so spawn_zone
        // can be called without borrow conflicts, then re-inserted immediately after.
        let mut zone_data = match spawn_zone_params.zone_loader_assets.remove(&zone_handle) {
            Some(asset) => asset,
            None => {
                log::error!(
                    "[ZONE LOADED FROM VFS] Zone asset not found in collection for handle: {:?}",
                    zone_handle
                );
                continue;
            }
        };
        let spawn_result = spawn_zone(&mut spawn_zone_params, &mut zone_data);
        spawn_zone_params
            .zone_loader_assets
            .insert(zone_handle.id(), zone_data);

        match spawn_result {
            Ok((entity, _zone_assets)) => {
                // The spawned_entity is what matters for despawning; the handle is kept by CurrentZone
                spawned_zones.push(entity);

                // CRITICAL FIX: Set CurrentZone resource with the REAL handle
                // This allows collision_player_system to access zone data via zone_loader_assets.get(&current_zone.handle)
                spawn_zone_params.commands.insert_resource(CurrentZone {
                    id: event.zone_id,
                    handle: zone_handle,
                });

                // Send loaded event
                zone_events.write(ZoneEvent::Loaded(event.zone_id));

                // Update debug inspector
                debug_inspector_state.entity = Some(entity);

                // MEMORY MONITOR: Log memory status after zone spawn completes
                log_memory_status(&format!(
                    "Zone {} spawned successfully",
                    event.zone_id.get()
                ));
            }
            Err(e) => {
                log::error!(
                    "[ZONE LOADED FROM VFS] Failed to spawn zone {}: {:?}",
                    event.zone_id.get(),
                    e
                );

                // DIAGNOSTIC: Zone entity spawn failed
                log::error!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✗ spawn_zone FAILED for zone_id={}: error={:?}",
                    event.zone_id.get(), e);
            }
        }
    }

    spawn_zone_params.memory_tracking.log_summary();

    // MEMORY MONITOR: Log final memory status after all VFS zone processing
    if processed_count > 0 {
        log_memory_status("Zone loading batch complete");
    }
}

pub fn force_zone_visibility_system(mut zone_query: Query<&mut Visibility, With<Zone>>) {
    for mut visibility in zone_query.iter_mut() {
        if *visibility != Visibility::Visible {
            log::info!("[FORCE VISIBILITY] Forcing Zone to Visibility::Visible");
            *visibility = Visibility::Visible;
        }
    }
}
