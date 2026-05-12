use super::loading::load_zone_direct;
use super::spawning::spawn_zone;
use super::*;

pub fn zone_loader_system(
    mut zone_loader_cache: Local<ZoneLoaderCache>,
    mut loading_zones: Local<Vec<LoadingZone>>,
    mut load_zone_events: MessageReader<LoadZoneEvent>,
    mut zone_events: MessageWriter<ZoneEvent>,
    mut zone_loaded_from_vfs_events: MessageWriter<ZoneLoadedFromVfsEvent>,
    mut zone_load_receiver: ResMut<ZoneLoadChannelReceiver>,
    zone_load_sender: Res<ZoneLoadChannelSender>,
    mut spawn_zone_params: SpawnZoneParams,
    mut debug_inspector_state: ResMut<DebugInspector>,
) {
    let _span = info_span!("zone_loader_system").entered();
    let use_new_terrain = spawn_zone_params.render_config.use_new_terrain;
    //log::info!("[ZONE LOADER SYSTEM] use_new_terrain = {}", use_new_terrain);
    let has_load_events = load_zone_events.len() > 0;
    let has_loading_zones = !loading_zones.is_empty();

    // Early return if no zones are loading and no load events to process
    // This prevents unnecessary memory allocations from logging every frame
    if !has_load_events && !has_loading_zones {
        return;
    }

    // log::info!("[ZONE LOADER SYSTEM] ===========================================");
    // log::info!("[ZONE LOADER SYSTEM] zone_loader_system called");
    // log::info!("[ZONE LOADER SYSTEM] Loading zones in queue: {}", loading_zones.len());
    // log::info!("[ZONE LOADER SYSTEM] ===========================================");

    // Log periodic memory summary
    spawn_zone_params.memory_tracking.log_summary();

    // Check for loaded zones from async tasks via channel
    // log::info!("[ZONE LOADER SYSTEM] Checking channel for loaded zones...");
    let mut received_count = 0;
    while let Ok((zone_id, zone_asset_result)) = zone_load_receiver.0.lock().unwrap().try_recv() {
        received_count += 1;
        // log::info!("[ZONE LOADER SYSTEM] Received {} zone(s) from channel this frame", received_count);
        let zone_id: ZoneId = zone_id;
        let zone_asset_result: Result<ZoneLoaderAsset, anyhow::Error> = zone_asset_result;
        match zone_asset_result {
            Ok(zone_asset) => {
                // log::info!("[ZONE LOADER SYSTEM] ===========================================");
                // log::info!("[ZONE LOADER SYSTEM] Zone {} loaded from async task, sending ZoneLoadedFromVfsEvent", zone_id.get());
                // log::info!("[ZONE LOADER SYSTEM] ===========================================");

                // Remove the zone from the loading queue since it's now received from channel
                if let Some(pos) = loading_zones
                    .iter()
                    .position(|lz| lz.loading_via_async_task && lz.zone_id == Some(zone_id))
                {
                    // log::info!("[ZONE LOADER SYSTEM] Removing zone {} from loading queue (received from channel)", zone_id.get());
                    loading_zones.remove(pos);
                } else {
                    log::warn!(
                        "[ZONE LOADER SYSTEM] Could not find zone {} in loading queue to remove",
                        zone_id.get()
                    );
                }

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
                // Remove the failed loading zone from cache and loading queue
                let zone_index = zone_id.get() as usize;
                zone_loader_cache.cache[zone_index] = None;

                // Remove from loading queue
                if let Some(pos) = loading_zones
                    .iter()
                    .position(|lz| lz.loading_via_async_task && lz.zone_id == Some(zone_id))
                {
                    // log::info!("[ZONE LOADER SYSTEM] Removing failed zone {} from loading queue", zone_id.get());
                    loading_zones.remove(pos);
                }
            }
        }
    }

    if received_count == 0 {
        // log::info!("[ZONE LOADER SYSTEM] No zones received from channel this frame");
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

        // log::info!("[ZONE LOADER SYSTEM] ===========================================");
        // log::info!("[ZONE LOADER SYSTEM] LoadZoneEvent received for zone_id: {}", event.id.get());
        // log::info!("[ZONE LOADER SYSTEM] Despawn other zones: {}", event.despawn_other_zones);
        // log::info!("[ZONE LOADER SYSTEM] ===========================================");

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
                if let Some(Some(cached)) = zone_loader_cache.cache.get(zone_index) {
                    if let Some(entity) = cached.spawned_entity {
                        // log::info!("[ZONE LOADER SYSTEM] Despawning existing zone {} entity {:?} as requested",
                        //event.id.get(), entity);
                        spawn_zone_params.commands.entity(entity).despawn();
                    }
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
            // log::info!("[ZONE LOADER SYSTEM] Zone not cached, loading directly from VFS");

            // WORKAROUND: Load zone directly from VFS without using AssetServer
            // This bypasses the broken asset loading pipeline in Bevy 0.13.2
            let zone_id = event.id;
            let vfs = spawn_zone_params.vfs_resource.vfs.clone();
            let base_path = spawn_zone_params.vfs_resource.base_path.clone();
            let tx = zone_load_sender.0.clone();

            // log::info!("[ZONE LOADER SYSTEM] ===========================================");
            // log::info!("[ZONE LOADER SYSTEM] Preparing to spawn async task for zone {}", zone_id.get());

            // Check if pool is initialized and get reference
            let pool = match AsyncComputeTaskPool::try_get() {
                Some(pool) => {
                    // log::info!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is available, spawning async task");
                    pool
                }
                None => {
                    log::error!("[ZONE LOADER SYSTEM] AsyncComputeTaskPool is NOT initialized! Cannot spawn async task!");
                    log::error!("[ZONE LOADER SYSTEM] This is likely why zones are not loading!");
                    // DO NOT spawn the task - skip this zone and continue to next
                    continue;
                }
            };

            // log::info!("[ZONE LOADER SYSTEM] Spawning async task to load zone {}", zone_id.get());
            // log::info!("[ZONE LOADER SYSTEM] ===========================================");

            // Spawn async task to load zone using AsyncComputeTaskPool
            // This is more appropriate for computational tasks like loading zones
            let task = pool.spawn(async move {
                // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                // log::info!("[ZONE LOADER DIRECT TASK] Async task started for zone_id: {}", zone_id.get());
                // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");

                match load_zone_direct(zone_id, &vfs, &base_path, use_new_terrain).await {
                    Ok(zone_asset) => {
                        // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");
                        // log::info!("[ZONE LOADER DIRECT TASK] Zone loaded successfully: {}", zone_id.get());
                        // log::info!("[ZONE LOADER DIRECT TASK] Sending zone through channel...");
                        // log::info!("[ZONE LOADER DIRECT TASK] ===========================================");

                        match tx.send((zone_id, Ok(zone_asset))) {
                            Ok(_) => {
                                // log::info!("[ZONE LOADER DIRECT TASK] Zone sent through channel successfully!");
                            }
                            Err(e) => {
                                log::error!("[ZONE LOADER DIRECT TASK] Failed to send zone through channel: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[ZONE LOADER DIRECT TASK] Failed to load zone {}: {:?}", zone_id.get(), e);
                        match tx.send((zone_id, Err(e))) {
                            Ok(_) => {
                                // log::info!("[ZONE LOADER DIRECT TASK] Error sent through channel successfully!");
                            }
                            Err(send_err) => {
                                log::error!("[ZONE LOADER DIRECT TASK] Failed to send error through channel: {:?}", send_err);
                            }
                        }
                    }
                }
            });

            // Detach the task so it runs in the background
            task.detach();

            // log::info!("[ZONE LOADER SYSTEM] Async task spawned and detached for zone {}", zone_id.get());
            // log::info!("[ZONE LOADER SYSTEM] ===========================================");

            // Add zone to loading queue to track that it's being loaded
            // This ensures we know the zone is in progress even though spawning is handled by zone_loaded_from_vfs_system
            // MEMORY MONITOR: Capture baseline memory before starting zone load
            let memory_snapshot =
                MemorySnapshot::capture(&format!("Zone {} loading start", zone_id.get()));

            loading_zones.push(LoadingZone {
                state: LoadingZoneState::Loading,
                handle: Handle::<ZoneLoaderAsset>::default(),
                despawn_other_zones: event.despawn_other_zones,
                zone_assets: Vec::default(),
                ready_frames: 0,
                loading_via_async_task: true,
                zone_id: Some(zone_id),
                loading_start_time: Instant::now(), // Initialize start time
                assets_cleared: false,
                memory_snapshot_start: memory_snapshot,
            });
            // log::info!("[ZONE LOADER SYSTEM] Zone queued for async loading. Total loading zones: {}", loading_zones.len());
        } else if let Some(zone_entity) = zone_loader_cache.cache[zone_index]
            .as_ref()
            .and_then(|cached_zone| cached_zone.spawned_entity)
        {
            // Zone is already spawned
            // log::info!("[ZONE LOADER SYSTEM] Zone already spawned, sending Loaded event");
            zone_events.write(ZoneEvent::Loaded(event.id));
            debug_inspector_state.entity = Some(zone_entity);
            continue;
        } else {
            // log::info!("[ZONE LOADER SYSTEM] Zone cached but not spawned, using cached handle");

            let cached_zone = zone_loader_cache.cache[zone_index].as_ref().unwrap();
            // MEMORY MONITOR: Capture baseline memory before starting zone load (cached zone path)
            let memory_snapshot =
                MemorySnapshot::capture(&format!("Zone {} loading start (cached)", event.id.get()));

            loading_zones.push(LoadingZone {
                state: LoadingZoneState::Loading,
                handle: cached_zone.data_handle.clone(),
                despawn_other_zones: event.despawn_other_zones,
                zone_assets: Vec::default(),
                ready_frames: 0,
                loading_via_async_task: false,
                zone_id: None,
                loading_start_time: Instant::now(), // Initialize start time
                assets_cleared: false,
                memory_snapshot_start: memory_snapshot,
            });
            // log::info!("[ZONE LOADER SYSTEM] LoadingZone added to queue. Total loading zones: {}", loading_zones.len());
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

                    // Check for timeout (30 seconds)
                    if loading_zone.loading_start_time.elapsed() > Duration::from_secs(30) {
                        log::error!("[ZONE LOADER SYSTEM] Zone {} loading timeout after 30s, removing from queue", zone_path);

                        // MEMORY LEAK FIX: Clear asset handles before removing timed-out zone
                        loading_zone.clear_asset_handles();

                        loading_zones.remove(index);
                        continue;
                    }

                    // log::info!("[ZONE LOADER SYSTEM] Zone {} loading via async task, keeping in queue and waiting for channel",
                    //zone_path);
                    index += 1;
                    continue;
                } else {
                    // Zone is loading via AssetServer - check LoadState
                    let zone_path = loading_zone
                        .handle
                        .path()
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    // log::info!("[ZONE LOADER SYSTEM] Checking LoadState for zone {} (AssetServer)", zone_path);

                    match spawn_zone_params
                        .asset_server
                        .get_load_state(&loading_zone.handle)
                    {
                        Some(LoadState::NotLoaded) | Some(LoadState::Loading) => {
                            // log::info!("[ZONE LOADER SYSTEM] Zone {} still loading (LoadState: {:?}), keeping in queue",
                            //zone_path, spawn_zone_params.asset_server.get_load_state(&loading_zone.handle));
                            index += 1;
                        }
                        Some(LoadState::Loaded) => {
                            // log::info!("[ZONE LOADER SYSTEM] Zone {} loaded, transitioning to Spawned state", zone_path);
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
                let zone_id = if let Some(zone_index) =
                    zone_loader_cache.cache.iter().position(|z| {
                        z.as_ref()
                            .map(|z| z.data_handle == zone_handle)
                            .unwrap_or(false)
                    }) {
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
                    // Clear the VFS file cache when switching zones to free memory
                    // This removes all cached DDS textures and model files from memory
                    clear_vfs_file_cache();

                    // log::info!("[ZONE LOADER SYSTEM] Despawning other zones");
                    for cached_zone in zone_loader_cache
                        .cache
                        .iter_mut()
                        .filter_map(|x| x.as_mut())
                    {
                        if let Some(spawned_entity) = cached_zone.spawned_entity.take() {
                            // info!("[ASSET LIFECYCLE] Despawning zone entity: {:?}", spawned_entity);
                            log::warn!("[ZONE LOADER SYSTEM DIAGNOSTIC] ✗ Despawning existing zone entity: entity={:?}", spawned_entity);
                            spawn_zone_params.commands.entity(spawned_entity).despawn();
                            spawn_zone_params.memory_tracking.log_entity_despawned();
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
                let zone_handle_clone = zone_handle.clone();
                let spawn_result = {
                    let zone_data_opt =
                        spawn_zone_params.zone_loader_assets.get(&zone_handle_clone);

                    if let Some(zone_data) = zone_data_opt {
                        // log::info!("[ZONE LOADER SYSTEM] Zone data retrieved, starting spawn process");
                        // log::info!("[ZONE LOADER SYSTEM] Calling spawn_zone()");
                        // Extract the data we need before the borrow ends
                        let zone_id = zone_data.zone_id;
                        let zone_path = zone_data.zone_path.clone();
                        let blocks_len = zone_data.blocks.len();
                        let npcs_len = zone_data.npcs.len();

                        // log::info!("[ZONE LOADER SYSTEM] Spawning zone: id={}, path={}, blocks={}, npcs={}",
                        //zone_id.get(), zone_path.display(), blocks_len, npcs_len);

                        // Use raw pointer to work around borrow checker
                        // This is safe because:
                        // 1. spawn_zone doesn't actually use zone_loader_assets (it ignores it with `zone_loader_assets: _`)
                        // 2. spawn_zone_params is not modified through zone_loader_assets during the call
                        // 3. The reference is only used for the duration of spawn_zone call
                        let zone_data_ptr: *const ZoneLoaderAsset = zone_data;
                        let spawn_zone_params_ptr: *mut SpawnZoneParams = &mut spawn_zone_params;

                        unsafe {
                            let zone_data_ref: &ZoneLoaderAsset = &*zone_data_ptr;
                            let spawn_zone_params_ref: &mut SpawnZoneParams =
                                &mut *spawn_zone_params_ptr;
                            Some(spawn_zone(spawn_zone_params_ref, zone_data_ref))
                        }
                    } else {
                        log::warn!("[ZONE LOADER SYSTEM] Zone data not available!");
                        None::<Result<(Entity, Vec<UntypedHandle>), anyhow::Error>>
                    }
                };

                if let Some(result) = spawn_result {
                    match result {
                        Ok((zone_entity, zone_loading_assets)) => {
                            // log::info!("[ZONE LOADER SYSTEM] Zone spawned successfully");

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
                                // log::info!("[ZONE LOADER SYSTEM] No additional assets to load, sending Loaded event");

                                // MEMORY LEAK FIX: Clear asset handles before removing zone
                                loading_zone.clear_asset_handles();

                                zone_events.write(ZoneEvent::Loaded(zone_id));
                                loading_zones.remove(index);
                            } else {
                                // log::info!("[ZONE LOADER SYSTEM] Waiting for additional assets to load");
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

            LoadingZoneState::Spawned => {
                let is_loading = loading_zone.zone_assets.iter().any(|handle| {
                    matches!(
                        spawn_zone_params.asset_server.get_load_state(handle),
                        Some(LoadState::NotLoaded) | Some(LoadState::Loading)
                    )
                });

                if is_loading {
                    index += 1;
                } else if let Some(zone_data) = spawn_zone_params
                    .zone_loader_assets
                    .get(&loading_zone.handle)
                {
                    // The physics system will take 2 frames to initialise colliders properly
                    loading_zone.ready_frames += 1;

                    if loading_zone.ready_frames == 2 {
                        // log::info!("[ZONE LOADER SYSTEM] Zone ready after 2 frames, sending Loaded event");

                        // MEMORY LEAK FIX: Clear asset handles before removing zone
                        loading_zone.clear_asset_handles();

                        zone_events.write(ZoneEvent::Loaded(zone_data.zone_id));
                        loading_zones.remove(index);
                    } else {
                        index += 1;
                    }
                } else {
                    index += 1;
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

    if !already_loaded.is_empty() {
        // log::info!("[ZONE LOADED FROM VFS] Currently loaded zones: {:?}",
        //    already_loaded.iter().collect::<Vec<_>>());
    }

    // log::info!("[ZONE LOADED FROM VFS] Processing {} zone events this frame", event_count);
    spawn_zone_params.memory_tracking.log_summary();

    let mut processed_count = 0;
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;

    // CRITICAL FIX: Deduplicate events - prevent duplicate zone IDs in same batch
    let mut seen_zone_ids: std::collections::HashSet<u16> = std::collections::HashSet::new();

    // Process events directly, skipping duplicates
    // DIAGNOSTIC: Track ZoneLoadedFromVfsEvent processing
    // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] Processing {} ZoneLoadedFromVfsEvent(s)", events.len());

    for event in events.read() {
        // DIAGNOSTIC: Individual event details
        // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] Processing event: zone_id={}", event.zone_id.get());

        // Deduplicate: Skip duplicate zone IDs in the same batch
        if !seen_zone_ids.insert(event.zone_id.get()) {
            log::warn!(
                "[ZONE LOADED FROM VFS] DUPLICATE EVENT for zone {} ignored in batch",
                event.zone_id.get()
            );
            skipped_count += 1;
            continue;
        }

        // CRITICAL FIX: Skip zones that are already loaded
        if already_loaded.contains(&event.zone_id.get()) {
            log::warn!("[ZONE LOADED FROM VFS] Zone {} already exists, skipping spawn to prevent memory leak",
                event.zone_id.get());
            skipped_count += 1;
            // FIX: Still send ZoneEvent::Loaded so JoinZoneRequest is sent to server
            // This is critical for respawn scenarios where the player needs to re-join the zone
            zone_events.write(ZoneEvent::Loaded(event.zone_id));
            continue;
        }

        processed_count += 1;
        // log::info!("[ZONE LOADED FROM VFS] ===========================================");
        // log::info!("[ZONE LOADED FROM VFS] Spawning zone {} from VFS (event {}/{})"
        //    , event.zone_id.get(), processed_count, event_count);
        // log::info!("[ZONE LOADED FROM VFS] ===========================================");

        let zone_index = event.zone_id.get() as usize;

        // CRITICAL FIX: Handle despawn_other_zones flag (matching AssetServer path behavior)
        // Default to true to match the typical behavior when loading a new zone
        let despawn_other_zones = true;

        if despawn_other_zones {
            // Clear the VFS file cache when switching zones to free memory
            // This removes all cached DDS textures and model files from memory
            clear_vfs_file_cache();

            // log::info!("[ZONE LOADED FROM VFS] Despawning other zones");
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

            spawn_zone_params.commands.remove_resource::<CurrentZone>();
        }

        // CRITICAL FIX: The zone asset was already added to the Assets collection in zone_loader_system
        // We just need to use the handle from the event to spawn the zone
        let zone_handle = event.zone_handle.clone();
        // log::info!("[ZONE LOADED FROM VFS] Using zone handle from event: {:?}", zone_handle);

        // Spawn the zone using the asset from the collection (via handle)
        // DIAGNOSTIC: About to call spawn_zone
        // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] About to call spawn_zone for zone_id={}",
        //    event.zone_id.get());

        // Use raw pointer to work around borrow checker (same pattern as zone_loader_system line 1510-1516)
        // This is safe because spawn_zone doesn't modify zone_loader_assets
        let zone_asset_ref = match spawn_zone_params.zone_loader_assets.get(&zone_handle) {
            Some(asset) => asset,
            None => {
                log::error!(
                    "[ZONE LOADED FROM VFS] Zone asset not found in collection for handle: {:?}",
                    zone_handle
                );
                failed_count += 1;
                continue;
            }
        };
        let zone_data_ptr: *const ZoneLoaderAsset = zone_asset_ref;
        let spawn_zone_params_ptr: *mut SpawnZoneParams = &mut spawn_zone_params;

        let spawn_result = unsafe {
            let zone_data_ref: &ZoneLoaderAsset = &*zone_data_ptr;
            let spawn_zone_params_ref: &mut SpawnZoneParams = &mut *spawn_zone_params_ptr;
            spawn_zone(spawn_zone_params_ref, zone_data_ref)
        };

        match spawn_result {
            Ok((entity, _zone_assets)) => {
                success_count += 1;
                // log::info!("[ZONE LOADED FROM VFS] Zone {} spawned successfully! entity={:?}",
                //    event.zone_id.get(), entity);

                // DIAGNOSTIC: Zone entity successfully created and returned
                // log::info!("[ZONE LOADED FROM VFS DIAGNOSTIC] ✓ Zone entity created in zone_loaded_from_vfs_system: entity={:?}, zone_id={}",
                //    entity, event.zone_id.get());

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

                // Log memory summary after zone spawn
                // MEMORY MONITOR: Log memory status after zone spawn completes
                log_memory_status(&format!(
                    "Zone {} spawned successfully",
                    event.zone_id.get()
                ));
            }
            Err(e) => {
                failed_count += 1;
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

    // log::info!("[ZONE LOADED FROM VFS] ===========================================");
    // log::info!("[ZONE LOADED FROM VFS] Processing complete: {} success, {} failed, {} skipped (duplicates) out of {}",
    //    success_count, failed_count, skipped_count, processed_count);
    spawn_zone_params.memory_tracking.log_summary();

    // MEMORY MONITOR: Log final memory status after all VFS zone processing
    if processed_count > 0 {
        log_memory_status("Zone loading batch complete");
    }

    // log::info!("[ZONE LOADED FROM VFS] ===========================================");
}

pub fn force_zone_visibility_system(mut zone_query: Query<&mut Visibility, With<Zone>>) {
    for mut visibility in zone_query.iter_mut() {
        if *visibility != Visibility::Visible {
            log::info!("[FORCE VISIBILITY] Forcing Zone to Visibility::Visible");
            *visibility = Visibility::Visible;
        }
    }
}
