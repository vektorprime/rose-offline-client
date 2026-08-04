use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    terrain_material: &Handle<TerrainMaterial>,
    tile_textures: &Vec<Handle<Image>>,
    zone_data: &ZoneLoaderAsset,
    block_data: &ZoneLoaderBlock,
    terrain_noise: &crate::terrain::GlobalTerrainNoise,
) -> Entity {
    let _span = info_span!(
        "spawn_terrain",
        block_x = block_data.block_x,
        block_y = block_data.block_y
    )
    .entered();
    log::debug!(
        "[SPAWN TERRAIN] Spawning terrain block {}_{}",
        block_data.block_x,
        block_data.block_y
    );
    let offset_x = 160.0 * block_data.block_x as f32;
    let offset_y = 160.0 * (65.0 - block_data.block_y as f32);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs_lightmap = Vec::new();
    let mut uvs_tile = Vec::new();
    let mut indices = Vec::new();
    let mut tile_ids = Vec::new();

    let tilemap = block_data.til.as_ref();
    let heightmap = &block_data.him;

    // Build tile_texture_map for UV lookup
    let mut tile_texture_map = vec![0u32; tile_textures.len().max(1)];

    // First pass: build the texture mapping
    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile_idx = tilemap
                .map(|tm| tm.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0);

            if tile_idx >= zone_data.zon.tiles.len() {
                continue;
            }

            let tile = &zone_data.zon.tiles[tile_idx];
            let tile_array_index1 = (tile.layer1 + tile.offset1) as usize;
            let tile_array_index2 = (tile.layer2 + tile.offset2) as usize;

            if tile_array_index1 < tile_texture_map.len() {
                tile_texture_map[tile_array_index1] = tile_array_index1 as u32;
            } else {
                warn!(
                    "[SPAWN TERRAIN] Invalid tile layer1 id {} (max: {}), clamping",
                    tile_array_index1,
                    tile_texture_map.len().saturating_sub(1)
                );
            }

            if tile_array_index2 < tile_texture_map.len() {
                tile_texture_map[tile_array_index2] = tile_array_index2 as u32;
            } else {
                warn!(
                    "[SPAWN TERRAIN] Invalid tile layer2 id {} (max: {}), clamping",
                    tile_array_index2,
                    tile_texture_map.len().saturating_sub(1)
                );
            }
        }
    }

    // Second pass: build mesh vertices with tile info
    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile_idx = tilemap
                .map(|tm| tm.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0);

            let tile = if tile_idx < zone_data.zon.tiles.len() {
                &zone_data.zon.tiles[tile_idx]
            } else {
                continue;
            };

            // Get tile texture indices with bounds checking
            let tile_array_index1 =
                if ((tile.layer1 + tile.offset1) as usize) < tile_texture_map.len() {
                    tile_texture_map[(tile.layer1 + tile.offset1) as usize]
                } else {
                    0
                };
            let tile_array_index2 =
                if ((tile.layer2 + tile.offset2) as usize) < tile_texture_map.len() {
                    tile_texture_map[(tile.layer2 + tile.offset2) as usize]
                } else {
                    0
                };

            let tile_rotation = match tile.rotation {
                ZonTileRotation::FlipHorizontal => 2,
                ZonTileRotation::FlipVertical => 3,
                ZonTileRotation::Flip => 4,
                ZonTileRotation::Clockwise90 => 5,
                ZonTileRotation::CounterClockwise90 => 6,
                _ => 0,
            };
            let tile_indices_base = positions.len() as u16;
            let tile_offset_x = tile_x as f32 * 4.0 * 2.5;
            let tile_offset_y = tile_y as f32 * 4.0 * 2.5;

            for y in 0..5 {
                for x in 0..5 {
                    let heightmap_x = x + tile_x as i32 * 4;
                    let heightmap_y = y + tile_y as i32 * 4;
                    let base_height = heightmap.get_clamped(heightmap_x, heightmap_y) / 100.0;

                    // Calculate world coordinates for noise sampling
                    // Local position within block
                    let local_x = tile_offset_x + x as f32 * 2.5;
                    let local_z = tile_offset_y + y as f32 * 2.5;
                    // World position (matching the transform applied at spawn)
                    let world_x = offset_x - 5200.0 + local_x;
                    let world_z = -offset_y + 5200.0 + local_z;

                    // Apply procedural noise to height
                    let noise_offset = terrain_noise.get_noise(world_x, world_z);
                    let height = base_height + noise_offset;

                    // Calculate normals using noise-adjusted heights
                    let base_height_l = heightmap.get_clamped(heightmap_x - 1, heightmap_y) / 100.0;
                    let base_height_r = heightmap.get_clamped(heightmap_x + 1, heightmap_y) / 100.0;
                    let base_height_t = heightmap.get_clamped(heightmap_x, heightmap_y - 1) / 100.0;
                    let base_height_b = heightmap.get_clamped(heightmap_x, heightmap_y + 1) / 100.0;

                    // Apply noise to neighboring heights for smooth normals
                    let world_x_l = world_x - 2.5;
                    let world_x_r = world_x + 2.5;
                    let world_z_t = world_z - 2.5;
                    let world_z_b = world_z + 2.5;

                    let height_l = base_height_l + terrain_noise.get_noise(world_x_l, world_z);
                    let height_r = base_height_r + terrain_noise.get_noise(world_x_r, world_z);
                    let height_t = base_height_t + terrain_noise.get_noise(world_x, world_z_t);
                    let height_b = base_height_b + terrain_noise.get_noise(world_x, world_z_b);

                    let normal = Vec3::new(
                        (height_l - height_r) / 2.0,
                        1.0,
                        (height_t - height_b) / 2.0,
                    )
                    .normalize();

                    positions.push([local_x, height, local_z]);
                    normals.push([normal.x, normal.y, normal.z]);
                    uvs_tile.push([x as f32 / 4.0, y as f32 / 4.0]);
                    uvs_lightmap.push([
                        (tile_x as f32 * 4.0 + x as f32) / 64.0,
                        (tile_y as f32 * 4.0 + y as f32) / 64.0,
                    ]);

                    // Pack tile info: layer1_id | layer2_id << 8 | rotation << 16
                    tile_ids.push(
                        tile_array_index1
                            | (tile_array_index2 << 8)
                            | ((tile_rotation as u32) << 16),
                    );
                }
            }

            for y in 0..(5 - 1) {
                for x in 0..(5 - 1) {
                    let start = tile_indices_base + y * 5 + x;
                    indices.push(start);
                    indices.push(start + 5);
                    indices.push(start + 1);

                    indices.push(start + 1);
                    indices.push(start + 5);
                    indices.push(start + 1 + 5);
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let vertex_count = positions.len();
    let triangle_count = indices.len() / 3;
    mesh.insert_indices(Indices::U16(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
    mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, uvs_tile);

    // CRITICAL FIX: Insert tile_ids as custom vertex attribute for terrain texture mapping
    // This was the missing piece - tile_ids was computed but never added to the mesh!
    mesh.insert_attribute(crate::render::TERRAIN_MESH_ATTRIBUTE_TILE_INFO, tile_ids);

    log::debug!("[SPAWN TERRAIN] Block {}_{}: Mesh created with {} vertices, {} triangles (with tile_info attribute)",
        block_data.block_x, block_data.block_y, vertex_count, triangle_count);
    log::debug!(
        "[MEMORY] Terrain mesh created for block {}_{}",
        block_data.block_x,
        block_data.block_y
    );

    let mut collider_verts = Vec::new();
    let mut collider_indices = Vec::new();

    for y in 0..heightmap.height as i32 {
        for x in 0..heightmap.width as i32 {
            // Calculate world coordinates for noise sampling (same as mesh vertices)
            let local_x = x as f32 * 2.5;
            let local_z = y as f32 * 2.5;
            let world_x = offset_x - 5200.0 + local_x;
            let world_z = -offset_y + 5200.0 + local_z;

            // Apply same noise to collider for physics consistency
            let base_height = heightmap.get_clamped(x, y) / 100.0;
            let noise_offset = terrain_noise.get_noise(world_x, world_z);
            let height = base_height + noise_offset;

            collider_verts.push([local_x, height, local_z].into());
        }
    }

    for y in 0..(heightmap.height - 1) {
        for x in 0..(heightmap.width - 1) {
            let start = y * heightmap.width + x;
            collider_indices.push([start, start + heightmap.width, start + 1]);
            collider_indices.push([
                start + 1,
                start + heightmap.width,
                start + 1 + heightmap.width,
            ]);
        }
    }

    // The shader uses binding_array to sample from up to 100 textures based on per-vertex tile_info
    // The TerrainMaterial is created once per zone and shared by all blocks.

    // Split spawn to avoid Bundle tuple limit (15+ components not supported)
    let terrain_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::Terrain(ZoneObjectTerrain {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
            }),
            MapEditorTerrainBlock {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
                him_width: heightmap.width,
                him_height: heightmap.height,
                him_heights_cm: heightmap.heights.clone(),
                til_width: tilemap.map(|t| t.width).unwrap_or(0),
                til_height: tilemap.map(|t| t.height).unwrap_or(0),
                til_tiles: tilemap.map(|t| t.tiles.clone()).unwrap_or_default(),
                height_offset_cm: 0.0,
                fill_tile_id: None,
                dirty: false,
            },
            TerrainMeshForGrass,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(terrain_material.clone()),
            Transform::from_xyz(offset_x - 5200.0, 0.0, -offset_y + 5200.0),
            GlobalTransform::default(),
            Visibility::Visible,
            ViewVisibility::default(),
            InheritedVisibility::default(),
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
            NotShadowCaster,
        ))
        .insert((
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices)
                .expect("Failed to create terrain collider"),
            CollisionGroups::new(
                COLLISION_GROUP_ZONE_TERRAIN,
                COLLISION_FILTER_INSPECTABLE
                    | COLLISION_FILTER_COLLIDABLE
                    | COLLISION_GROUP_PHYSICS_TOY
                    | COLLISION_FILTER_MOVEABLE
                    | COLLISION_FILTER_CLICKABLE,
            ),
        ))
        .id();
    log::debug!(
        "[SPAWN TERRAIN] Terrain entity created: {:?} at position ({}, 0, {})",
        terrain_entity,
        offset_x,
        offset_y
    );
    terrain_entity
}

pub(super) fn spawn_new_terrain(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
    zone_data: &ZoneLoaderAsset,
    block_data: &ZoneLoaderBlock,
) -> Entity {
    let _span = info_span!(
        "spawn_new_terrain",
        block_x = block_data.block_x,
        block_y = block_data.block_y
    )
    .entered();
    log::debug!(
        "[SPAWN NEW TERRAIN] Spawning new terrain block {}_{}",
        block_data.block_x,
        block_data.block_y
    );

    let offset_x = 160.0 * block_data.block_x as f32;
    let offset_y = 160.0 * (65.0 - block_data.block_y as f32);

    let mesh_data = block_data
        .new_terrain_mesh
        .as_ref()
        .expect("New terrain mesh data missing");
    let mut cursor = 0;

    let read_u32 = |cursor: &mut usize, data: &[u8]| {
        let val = u32::from_le_bytes(data[*cursor..*cursor + 4].try_into().unwrap());
        *cursor += 4;
        val
    };

    let read_f32 = |cursor: &mut usize, data: &[u8]| {
        let val = f32::from_le_bytes(data[*cursor..*cursor + 4].try_into().unwrap());
        *cursor += 4;
        val
    };

    let vertex_count = read_u32(&mut cursor, mesh_data) as usize;
    let mut positions = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        positions.push([
            read_f32(&mut cursor, mesh_data),
            read_f32(&mut cursor, mesh_data),
            read_f32(&mut cursor, mesh_data),
        ]);
    }

    let mut normals = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        normals.push([
            read_f32(&mut cursor, mesh_data),
            read_f32(&mut cursor, mesh_data),
            read_f32(&mut cursor, mesh_data),
        ]);
    }

    let mut uvs = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        uvs.push([
            read_f32(&mut cursor, mesh_data),
            read_f32(&mut cursor, mesh_data),
        ]);
    }

    let has_tangents = read_u32(&mut cursor, mesh_data) == 1;
    let tangents = if has_tangents {
        let mut t = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            t.push([
                read_f32(&mut cursor, mesh_data),
                read_f32(&mut cursor, mesh_data),
                read_f32(&mut cursor, mesh_data),
                read_f32(&mut cursor, mesh_data),
            ]);
        }
        Some(t)
    } else {
        None
    };

    let index_count = read_u32(&mut cursor, mesh_data) as usize;
    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        indices.push(read_u32(&mut cursor, mesh_data));
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_indices(Indices::U32(indices.clone()));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if let Some(tangents) = tangents {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }

    let albedo_path = zone_data.zone_path.join(format!(
        "block_{}_{}_albedo.png",
        block_data.block_x, block_data.block_y
    ));
    let normal_path = zone_data.zone_path.join(format!(
        "block_{}_{}_normal.png",
        block_data.block_x, block_data.block_y
    ));

    let albedo = asset_server.load(albedo_path.to_string_lossy().into_owned());
    let normal = asset_server.load_with_settings(
        normal_path.to_string_lossy().into_owned(),
        |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
        },
    );

    let material = standard_materials.add(StandardMaterial {
        base_color_texture: Some(albedo),
        normal_map_texture: Some(normal),
        // Reduce directional glossy highlights on terrain in --new-terrain mode.
        // Keep terrain strongly diffuse/non-metallic to avoid metallic-looking sheen.
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        depth_bias: 0.1, // Small bias to mitigate Z-fighting in overlapping terrain
        ..Default::default()
    });

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut collider_verts = Vec::with_capacity(vertex_count);
    for p in &positions {
        let pos = Vec3::new(p[0], p[1], p[2]);
        collider_verts.push(pos);
        min = min.min(pos);
        max = max.max(pos);
    }

    let mut collider_indices = Vec::with_capacity(indices.len() / 3);
    for i in 0..indices.len() / 3 {
        collider_indices.push([indices[i * 3], indices[i * 3 + 1], indices[i * 3 + 2]]);
    }

    let terrain_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::Terrain(ZoneObjectTerrain {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
            }),
            MapEditorTerrainBlock {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
                him_width: block_data.him.width,
                him_height: block_data.him.height,
                him_heights_cm: block_data.him.heights.clone(),
                til_width: block_data.til.as_ref().map(|t| t.width).unwrap_or(0),
                til_height: block_data.til.as_ref().map(|t| t.height).unwrap_or(0),
                til_tiles: block_data
                    .til
                    .as_ref()
                    .map(|t| t.tiles.clone())
                    .unwrap_or_default(),
                height_offset_cm: 0.0,
                fill_tile_id: None,
                dirty: false,
            },
            TerrainMeshForGrass,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            Transform::from_xyz(offset_x - 5200.0, 0.0, -offset_y + 5200.0),
            GlobalTransform::default(),
            Visibility::Visible,
            ViewVisibility::default(),
            InheritedVisibility::default(),
            Aabb::from_min_max(min, max),
            RenderLayers::layer(0),
            NotShadowCaster,
        ))
        .insert((
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices)
                .expect("Failed to create terrain collider"),
            CollisionGroups::new(
                COLLISION_GROUP_ZONE_TERRAIN,
                COLLISION_FILTER_INSPECTABLE
                    | COLLISION_FILTER_COLLIDABLE
                    | COLLISION_GROUP_PHYSICS_TOY
                    | COLLISION_FILTER_MOVEABLE
                    | COLLISION_FILTER_CLICKABLE,
            ),
        ))
        .id();

    terrain_entity
}
