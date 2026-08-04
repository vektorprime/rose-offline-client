use super::loading::load_zone;
use super::spawning::spawn_zone;
use super::*;

pub fn zone_loader_system(
    mut zone_loader_cache: Local<ZoneLoaderCache>,
    mut loading_zones: Local<Vec<LoadingZone>>,
    mut load_zone_events: MessageReader<LoadZoneEvent>,
    mut zone_events: MessageWriter<ZoneEvent>,
    mut zone_loaded_from_vfs_events: MessageWriter<ZoneLoadedFromVfsEvent>,
    zone_load_receiver: ResMut<ZoneLoadChannelReceiver>,
    zone_load_sender: Res<ZoneLoadChannelSender>,
    mut spawn_zone_params: SpawnZoneParams,
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut last_requested_zone: ResMut<LastRequestedZone>,
) {
    let _span = info_span!("zone_loader_system").entered();
    let use_new_terrain = spawn_zone_params.render_config.use_new_terrain;
    let has_load_events = load_zone_events.len() > 0;
    let has_loading_zones = !loading_zones.is_empty();

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

        // Remove the zone from the loading queue; if it is not in the queue the
        // load is stale (timed out or superseded) and the result is dropped so a
        // stale zone can never spawn over the current one.
        let Some(loading_pos) = loading_zones
            .iter()
            .position(|lz| lz.loading_via_async_task && lz.zone_id == Some(zone_id))
        else {
            log::warn!(
                "[ZONE LOADER SYSTEM] Dropping stale load result for zone {} (no longer in loading queue)",
                zone_id.get()
            );
            continue;
        };
        loading_zones.remove(loading_pos);

        match zone_asset_result {
            Ok(zone_asset) => {
                // CRITICAL FIX: Add the zone asset to the Assets collection HERE where we have ownership
                // This allows collision_player_system to access terrain height data
                let zone_handle = spawn_zone_params.zone_loader_assets.add(zone_asset);
                log::info!(
                    "[ZONE LOADER SYSTEM] Zone {} added to Assets collection with handle: {:?}",
                    zone_id.get(),
                    zone_handle
                );

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
                // Remove the failed loading zone from the cache
                let zone_index = zone_id.get() as usize;
                zone_loader_cache.cache[zone_index] = None;
            }
        }
    }

    if zone_loader_cache.cache.is_empty() {
        zone_loader_cache
            .cache
            .resize_with(spawn_zone_params.game_data.zone_list.len(), || None);
    }

    for event in load_zone_events.read() {
        // DIAGNOSTIC: Track LoadZoneEvent received
        log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] LoadZoneEvent received: zone_id={}, despawn_other_zones={}",
            event.id.get(), event.despawn_other_zones);

        // Record the most recent request so stale async completions (e.g. the
        // login screen's background zone finishing after a newer request) can
        // be dropped instead of replacing the current zone.
        last_requested_zone.0 = Some(event.id);

        let zone_index = event.id.get() as usize;

        // Memory tracking: Log cache state
        let cached_zones = zone_loader_cache
            .cache
            .iter()
            .filter(|z| z.is_some())
            .count();
        let spawned_zones = zone_loader_cache
            .cache
            .iter()
            .filter(|z| z.is_some() && z.as_ref().unwrap().spawned_entity.is_some())
            .count();
        log::info!(
            "[MEMORY] Cache state: {} zones cached, {} spawned",
            cached_zones,
            spawned_zones
        );

        // CRITICAL FIX: Check for duplicate zone loading to prevent memory leaks
        // and double-spawning of the same zone
        let is_already_loading = loading_zones.iter().any(|lz| lz.zone_id == Some(event.id));
        let is_already_loaded = zone_loader_cache
            .cache
            .get(zone_index)
            .map(|c| {
                c.as_ref()
                    .map(|cz| cz.spawned_entity.is_some())
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        if is_already_loading {
            log::warn!("[ZONE LOADER SYSTEM] Zone {} is already loading via async task, skipping duplicate request",
                event.id.get());
            continue;
        }

        if is_already_loaded {
            log::warn!("[ZONE LOADER SYSTEM] Zone {} is already loaded. Consider despawning old instance first.",
                event.id.get());
            // Optionally: Despawn the old zone here if event.despawn_other_zones is true
            if event.despawn_other_zones {
                if let Some(Some(cached)) = zone_loader_cache.cache.get_mut(zone_index) {
                    if let Some(entity) = cached.spawned_entity {
                        spawn_zone_params.commands.entity(entity).despawn();
                    }
                    // Release the parsed zone data; the reload below creates a fresh asset
                    spawn_zone_params
                        .zone_loader_assets
                        .remove(&cached.data_handle);
                }
                zone_loader_cache.cache[zone_index] = None;
            } else {
                // Skip if already loaded and not despawning
                continue;
            }
        }

        if zone_loader_cache
            .cache
            .get(zone_index)
            .map(|c| c.is_none())
            .unwrap_or(true)
        {
            // WORKAROUND: Load zone directly from VFS without using AssetServer
            let zone_id = event.id;
            let vfs = spawn_zone_params.vfs_resource.vfs.clone();
            let base_path = spawn_zone_params.vfs_resource.base_path.clone();
            let tx = zone_load_sender.0.clone();

            // Check if pool is initialized and get reference
            let pool = match AsyncComputeTaskPool::try_get() {
                Some(pool) => pool,
                None => {
                    log::error!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is NOT initialized! Cannot spawn async task!");
                    log::error!("[ZONE LOADER SYSTEM] This is likely why zones are not loading!");
                    // DO NOT spawn the task - skip this zone and continue to next
                    continue;
                }
            };

            // Spawn async task to load zone using AsyncComputeTaskPool
            let task = pool.spawn(async move {
                match load_zone(zone_id, &vfs, &base_path, use_new_terrain).await {
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
                state: LoadingZoneState::Loading,
                handle: Handle::<ZoneLoaderAsset>::default(),
                despawn_other_zones: event.despawn_other_zones,
                zone_assets: Vec::default(),
                loading_via_async_task: true,
                zone_id: Some(zone_id),
                loading_start_time: Instant::now(), // Initialize start time
                assets_cleared: false,
            });
        } else if let Some(zone_entity) = zone_loader_cache.cache[zone_index]
            .as_ref()
            .and_then(|cached_zone| cached_zone.spawned_entity)
        {
            // Zone is already spawned
            zone_events.write(ZoneEvent::Loaded(event.id));
            debug_inspector_state.entity = Some(zone_entity);
            continue;
        } else {
            // Zone cached but not spawned: spawn directly from the cached data
            // (no re-parse, no re-decompress on revisit)
            let cached_zone = zone_loader_cache.cache[zone_index].as_ref().unwrap();
            zone_loaded_from_vfs_events.write(ZoneLoadedFromVfsEvent::new(
                event.id,
                cached_zone.data_handle.clone(),
            ));
        }
    }

    let mut index = 0;
    while index < loading_zones.len() {
        let loading_zone = &mut loading_zones[index];

        match loading_zone.state {
            LoadingZoneState::Loading => {
                // Zones loaded via async task should stay in queue and wait for channel
                if loading_zone.loading_via_async_task {
                    let zone_path = loading_zone
                        .handle
                        .path()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    // Check for timeout (120 seconds, generous for large zones)
                    if loading_zone.loading_start_time.elapsed() > Duration::from_secs(120) {
                        log::error!("[ZONE LOADER SYSTEM] Zone {} loading timeout after 120s, removing from queue", zone_path);

                        // MEMORY LEAK FIX: Clear asset handles before removing timed-out zone
                        loading_zone.clear_asset_handles();

                        loading_zones.remove(index);
                        continue;
                    }

                    index += 1;
                    continue;
                } else {
                    // Zone is loading via AssetServer - check LoadState
                    let zone_path = loading_zone
                        .handle
                        .path()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    match spawn_zone_params
                        .asset_server
                        .get_load_state(&loading_zone.handle)
                    {
                        Some(LoadState::NotLoaded) | Some(LoadState::Loading) => {
                            index += 1;
                        }
                        Some(LoadState::Loaded) => {
                            loading_zone.state = LoadingZoneState::Spawned;
                            index += 1;
                        }
                        None | Some(LoadState::Failed(_)) => {
                            log::warn!("[ZONE LOADER SYSTEM] Zone {} failed to load (LoadState: {:?}), removing from queue",
                                zone_path, spawn_zone_params.asset_server.get_load_state(&loading_zone.handle));

                            // MEMORY LEAK FIX: Clear asset handles before removing failed zone
                            loading_zone.clear_asset_handles();

                            loading_zones.remove(index);
                        }
                    }
                }
            }

            LoadingZoneState::Spawned => {
                // DIAGNOSTIC: Zone transitioning to Spawned state
                log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] LoadingZone transitioning to Spawned state for zone");

                let zone_handle = loading_zone.handle.clone();

                // Get zone_id from handle by looking up in cache
                let zone_id = if zone_loader_cache
                    .cache
                    .iter()
                    .any(|z| {
                        z.as_ref()
                            .map(|z| z.data_handle == zone_handle)
                            .unwrap_or(false)
                    })
                {
                    zone_loader_cache
                        .cache
                        .iter()
                        .enumerate()
                        .find_map(|(idx, z)| {
                            z.as_ref().and_then(|cached| {
                                if cached.data_handle == zone_handle {
                                    Some(ZoneId::new(idx as u16).unwrap())
                                } else {
                                    None
                                }
                            })
                        })
                        .unwrap()
                } else {
                    log::error!("[ZONE LOADER SYSTEM] Cannot find zone_id for handle");

                    // MEMORY LEAK FIX: Clear asset handles before removing zone with error
                    loading_zone.clear_asset_handles();

                    loading_zones.remove(index);
                    continue;
                };

                // Despawn other zones first
                if loading_zone.despawn_other_zones {
                    // Evict zone-scoped byte cache entries, keeping the incoming and
                    // current zones' block data (shared assets such as UI textures stay cached)
                    let mut keep_zones = vec![zone_id.get()];
                    if let Some(current_zone) = spawn_zone_params.current_zone.as_ref() {
                        keep_zones.push(current_zone.id.get());
                    }
                    evict_zone_tagged_files(&keep_zones);

                    for cached_zone in zone_loader_cache
                        .cache
                        .iter_mut()
                        .filter_map(|x| x.as_mut())
                    {
                        if let Some(spawned_entity) = cached_zone.spawned_entity.take() {
                            log::warn!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✗ Despawning existing zone entity: entity={:?}", spawned_entity);
                            spawn_zone_params.commands.entity(spawned_entity).despawn();
                            spawn_zone_params.memory_tracking.log_entity_despawned();
                        }
                    }

                    // Evict parsed zone data for other zones; the incoming zone's
                    // data is still needed below (its handle came from the cache) and
                    // the current zone's data stays cached for instant revisits
                    let current_idx = spawn_zone_params
                        .current_zone
                        .as_ref()
                        .map(|z| z.id.get() as usize);
                    for (idx, cached_zone) in zone_loader_cache.cache.iter_mut().enumerate() {
                        if idx == zone_id.get() as usize || Some(idx) == current_idx {
                            continue;
                        }
                        if let Some(cached_zone) = cached_zone.take() {
                            spawn_zone_params
                                .zone_loader_assets
                                .remove(&cached_zone.data_handle);
                        }
                    }

                    spawn_zone_params.commands.remove_resource::<CurrentZone>();
                }

                // DIAGNOSTIC: About to spawn zone from zone_loader_system
                log::info!(
                    "[ZONE LOADER SYSTEM DIAGNOSTIC] About to call spawn_zone for zone_id={}",
                    zone_id.get()
                );

                // Get zone_data and spawn
                // The asset is temporarily removed from the Assets collection so spawn_zone
                // can be called without borrow conflicts, then re-inserted immediately after.
                let zone_handle_clone = zone_handle.clone();
                let spawn_result = match spawn_zone_params
                    .zone_loader_assets
                    .remove(&zone_handle_clone)
                {
                    Some(zone_data) => {
                        let result = spawn_zone(&mut spawn_zone_params, &zone_data);
                        spawn_zone_params
                            .zone_loader_assets
                            .insert(zone_handle_clone.id(), zone_data);
                        Some(result)
                    }
                    None => {
                        log::warn!("[ZONE LOADER SYSTEM] Zone data not available!");
                        None::<Result<(Entity, Vec<UntypedHandle>), anyhow::Error>>
                    }
                };

                if let Some(result) = spawn_result {
                    match result {
                        Ok((zone_entity, zone_loading_assets)) => {
                            // DIAGNOSTIC: Zone entity successfully spawned from zone_loader_system
                            log::info!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✓ Zone entity created in zone_loader_system: entity={:?}, zone_id={}",
                                zone_entity, zone_id.get());

                            // Check if assets are empty before moving
                            let assets_empty = zone_loading_assets.is_empty();

                            // Update cache with spawned entity
                            let zone_index = zone_id.get() as usize;
                            if let Some(cached_zone) = zone_loader_cache.cache[zone_index].as_mut()
                            {
                                cached_zone.spawned_entity = Some(zone_entity);
                            }

                            loading_zone.zone_assets = zone_loading_assets;
                            loading_zone.state = LoadingZoneState::Spawned;

                            // CRITICAL FIX: Set CurrentZone resource (was missing in Bevy 0.13 implementation)
                            // This matches Bevy 0.11 behavior (lines 482-485)
                            spawn_zone_params.commands.insert_resource(CurrentZone {
                                id: zone_id,
                                handle: zone_handle_clone,
                            });

                            if assets_empty {
                                // MEMORY LEAK FIX: Clear asset handles before removing zone
                                loading_zone.clear_asset_handles();

                                zone_events.write(ZoneEvent::Loaded(zone_id));
                                loading_zones.remove(index);
                            } else {
                                index += 1;
                            }
                        }
                        Err(e) => {
                            log::error!("[ZONE LOADER SYSTEM] Failed to spawn zone: {:?}", e);

                            // DIAGNOSTIC: Zone entity spawn failed in zone_loader_system
                            log::error!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✗ spawn_zone FAILED in zone_loader_system for zone_id={}: error={:?}",
                                zone_id.get(), e);

                            // MEMORY LEAK FIX: Clear asset handles before removing zone on failure
                            loading_zone.clear_asset_handles();

                            loading_zones.remove(index);
                        }
                    }
                }
            }
        }
    }
}

/// System to handle spawning zones that were loaded from VFS via async tasks
/// This separate system avoids borrow checker conflicts by handling spawning independently
/// CRITICAL FIX: Process ALL events, not just one, to prevent event queue buildup
/// CRITICAL FIX: Deduplicate events and prevent spawning already-loaded zones
pub fn zone_loaded_from_vfs_system(
    mut events: MessageReader<ZoneLoadedFromVfsEvent>,
    mut zone_loader_cache: Local<ZoneLoaderCache>,
    mut zone_events: MessageWriter<ZoneEvent>,
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut spawn_zone_params: SpawnZoneParams,
    last_requested_zone: Res<LastRequestedZone>,
    // CRITICAL FIX: Query existing zones to prevent duplicate spawning
    existing_zones: Query<(Entity, &Zone)>,
) {
    let _span = info_span!("zone_loaded_from_vfs_system").entered();
    let event_count = events.len();
    if event_count == 0 {
        return;
    }

    // Initialize cache if empty
    if zone_loader_cache.cache.is_empty() {
        zone_loader_cache
            .cache
            .resize_with(spawn_zone_params.game_data.zone_list.len(), || None);
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
        // Drop stale completions: a zone whose async load finished after a
        // newer zone was requested (e.g. the login screen's background zone
        // finishing after the game zone request on first login) must not
        // replace the zone the game is actually waiting for.
        if let Some(last_requested) = last_requested_zone.0 {
            if last_requested != event.zone_id {
                log::warn!(
                    "[ZONE LOADED FROM VFS] Ignoring stale event for zone {} (last requested zone {})",
                    event.zone_id.get(),
                    last_requested.get()
                );
                continue;
            }
        }

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

        let zone_index = event.zone_id.get() as usize;

        // CRITICAL FIX: Handle despawn_other_zones flag (matching AssetServer path behavior)
        // Default to true to match the typical behavior when loading a new zone
        let despawn_other_zones = true;

        if despawn_other_zones {
            // Evict zone-tagged byte cache entries for zones other than the current
            // and incoming ones, so revisits stay cheap while block data stays bounded.
            // Shared assets (UI textures, skybox, models) remain cached.
            let mut keep_zones = vec![event.zone_id.get()];
            if let Some(current_zone) = spawn_zone_params.current_zone.as_ref() {
                keep_zones.push(current_zone.id.get());
            }
            evict_zone_tagged_files(&keep_zones);

            for cached_zone in zone_loader_cache
                .cache
                .iter_mut()
                .filter_map(|x| x.as_mut())
            {
                if let Some(spawned_entity) = cached_zone.spawned_entity.take() {
                    log::warn!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✗ Despawning existing zone entity: entity={:?}", spawned_entity);
                    spawn_zone_params.commands.entity(spawned_entity).despawn();
                    spawn_zone_params.memory_tracking.log_entity_despawned();
                }
            }

            // Release parsed zone data for zones that are no longer current.
            // The incoming zone's data is held under the fresh event handle and
            // is re-cached below after spawning; the current zone's data is kept
            // (it is still referenced by CurrentZone until the flush, and keeping
            // it means revisits of the previous zone stay instant).
            let incoming_idx = event.zone_id.get() as usize;
            let current_idx = spawn_zone_params
                .current_zone
                .as_ref()
                .map(|z| z.id.get() as usize);
            for (idx, cached_zone) in zone_loader_cache.cache.iter_mut().enumerate() {
                if idx == incoming_idx || Some(idx) == current_idx {
                    continue;
                }
                if let Some(cached_zone) = cached_zone.take() {
                    spawn_zone_params
                        .zone_loader_assets
                        .remove(&cached_zone.data_handle);
                }
            }

            // Release cached parsed effects from the old zone (live effects hold
            // their own Arc references); the new zone's effects populate the
            // cache during spawn below
            spawn_zone_params.effect_cache.clear();

            spawn_zone_params.commands.remove_resource::<CurrentZone>();
        }

        // CRITICAL FIX: The zone asset was already added to the Assets collection in zone_loader_system
        // We just need to use the handle from the event to spawn the zone
        let zone_handle = event.zone_handle.clone();

        // Spawn the zone using the asset from the collection (via handle)
        // The asset is temporarily removed from the Assets collection so spawn_zone
        // can be called without borrow conflicts, then re-inserted immediately after.
        let zone_data = match spawn_zone_params.zone_loader_assets.remove(&zone_handle) {
            Some(asset) => asset,
            None => {
                log::error!(
                    "[ZONE LOADED FROM VFS] Zone asset not found in collection for handle: {:?}",
                    zone_handle
                );
                continue;
            }
        };
        let spawn_result = spawn_zone(&mut spawn_zone_params, &zone_data);
        spawn_zone_params
            .zone_loader_assets
            .insert(zone_handle.id(), zone_data);

        match spawn_result {
            Ok((entity, _zone_assets)) => {
                // CRITICAL FIX: Cache VFS-loaded zones with the REAL handle (not placeholder)
                // The spawned_entity is what matters for despawning; handle is used for terrain height lookups
                zone_loader_cache.cache[zone_index] = Some(CachedZone {
                    data_handle: zone_handle.clone(),
                    spawned_entity: Some(entity),
                });

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
