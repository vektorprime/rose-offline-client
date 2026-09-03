use super::*;
use std::collections::HashMap;

pub(super) fn spawn_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone_loading_assets: &mut Vec<UntypedHandle>,
    object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    specular_texture: &SpecularTexture,
    zsc: &ZscFile,
    lightmap_path: &Path,
    lit_object: Option<&LitObject>,
    object_instance: &IfoObject,
    ifo_object_id: usize,
    zsc_object_id: usize,
    object_type: fn(ZoneObjectId) -> ZoneObject,
    part_object_type: fn(ZoneObjectPart) -> ZoneObject,
    collision_group: bevy_rapier3d::prelude::Group,
) -> Entity {
    let object = &zsc.objects[zsc_object_id];
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];
    // In-object material dedup: identical (material, lightmap) parts share one
    // ExtendedMaterial instead of one bind-group/pipeline-key per part instance.
    // (Cross-object sharing still goes through the asset-server handle cache.)
    // INVARIANT: zone statics never receive BloodOverlay (character-only), so sharing
    // one asset across parts is safe. If zone objects ever gain overlays, the blood
    // system must clone-on-write instead of mutating the shared asset.
    let mut material_cache: HashMap<
        (
            usize,
            Option<(String, u32, u32, u32)>,
        ),
        Handle<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    > = HashMap::new();

    let object_entity_commands = commands.spawn((
        EditorSelectable,
        object_type(ZoneObjectId {
            ifo_object_id,
            zsc_object_id,
        }),
        object_transform,
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
        // No Aabb on mesh-less object root: children parts carry their own bounds.
        // Previously a ±100000 box defeated frustum + occlusion culling for the whole zone.
        bevy::camera::visibility::RenderLayers::layer(0),
        RigidBody::Fixed,
    ));

    let object_entity = object_entity_commands.id();

    for (part_index, object_part) in object.parts.iter().enumerate() {
        let part_transform = Transform::default()
            .with_translation(
                Vec3::new(
                    object_part.position.x,
                    object_part.position.z,
                    -object_part.position.y,
                ) / 100.0,
            )
            .with_rotation(Quat::from_xyzw(
                object_part.rotation.x,
                object_part.rotation.z,
                -object_part.rotation.y,
                object_part.rotation.w,
            ))
            .with_scale(Vec3::new(
                object_part.scale.x,
                object_part.scale.z,
                object_part.scale.y,
            ));

        let mesh_id = object_part.mesh_id as usize;

        // VALIDATION FIX: Check mesh_id bounds before using
        if mesh_id >= zsc.meshes.len() {
            continue;
        }

        // VALIDATION FIX: Check material_id bounds
        let material_id = object_part.material_id as usize;
        if material_id >= zsc.materials.len() {
            continue;
        }

        // Index assignment (was Vec::insert, which shifts elements and breaks
        // reuse for all later parts). Cache is per-object; identical meshes across
        // objects still share the asset-server handle cache underneath.
        let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
            let mesh_path = zsc.meshes[mesh_id].path().to_string_lossy().into_owned();
            let handle: Handle<Mesh> = asset_server.load(&mesh_path);
            mesh_cache[mesh_id] = Some(handle.clone());
            handle
        });
        zone_loading_assets.push(UntypedHandle::from(mesh.clone()));
        let lit_part = lit_object.and_then(|lit_object| {
            for part in lit_object.parts.iter() {
                if part_index == part.object_part_index as usize {
                    return Some(part);
                }
            }

            lit_object.parts.get(part_index)
        });
        let lightmap_texture = lit_part.map(|lit_part| {
            let path = lightmap_path.join(&lit_part.filename);
            let path_str = path.to_string_lossy().into_owned();
            asset_server.load::<bevy::prelude::Image>(&path_str)
        });
        let (lightmap_uv_offset, lightmap_uv_scale) = lit_part
            .map(|lit_part| {
                let scale = 1.0 / lit_part.parts_per_row as f32;
                (
                    Vec2::new(
                        (lit_part.part_index % lit_part.parts_per_row) as f32,
                        (lit_part.part_index / lit_part.parts_per_row) as f32,
                    ),
                    scale,
                )
            })
            .unwrap_or((Vec2::new(0.0, 0.0), 1.0));

        // NOTE: material_id was already bounds-checked above; this fetch is for local use.
        let material_id = object_part.material_id as usize;

        let zsc_material = zsc.materials[material_id].clone();
        let material_path = zsc_material.path.path().to_string_lossy().into_owned();

        // Dedup key includes lightmap identity (path + quantized UVs) so lit parts
        // with different lightmaps still get distinct materials, while repeated
        // unlit parts share one material.
        let lightmap_key = lit_part.map(|p| {
            (
                p.filename.clone(),
                lightmap_uv_offset.x.to_bits(),
                lightmap_uv_offset.y.to_bits(),
                lightmap_uv_scale.to_bits(),
            )
        });
        let material_cache_key = (material_id, lightmap_key);
        let material = if let Some(cached) = material_cache.get(&material_cache_key) {
            cached.clone()
        } else {
            let base_texture_handle: Handle<Image> = asset_server.load(&material_path);

            // Create ExtendedMaterial with RoseObjectExtension for zone lighting support
            // This applies zone lighting ambient color to darken objects to match the original game
            let handle = object_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color_texture: if material_path.is_empty() || material_path == "" || material_path == "NULL" {
                        log::warn!("[SPAWN OBJECT DEBUG] Empty or NULL texture path for mesh_id {}, using fallback", mesh_id);
                        Some(asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"))
                    } else {
                        Some(base_texture_handle.clone())
                    },
                    unlit: false,  // Enable PBR lighting for objects/decorations
                    double_sided: zsc_material.two_sided,
                    // PBR properties for realistic lighting on vegetation and outdoor objects
                    perceptual_roughness: 0.8,  // Higher roughness for matte vegetation/buildings
                    metallic: 0.0,              // Non-metallic for organic/building materials
                    alpha_mode: if zsc_material.alpha_enabled {
                        if let Some(threshold) = zsc_material.alpha_test {
                            AlphaMode::Mask(threshold)
                        } else {
                            AlphaMode::Blend
                        }
                    } else {
                        AlphaMode::Opaque
                    },
                    ..Default::default()
                },
                extension: RoseObjectExtension {
                    lightmap_params: Vec3::new(lightmap_uv_offset.x, lightmap_uv_offset.y, lightmap_uv_scale).extend(0.0),
                    lightmap_texture: lightmap_texture.clone(),
                    specular_texture: Some(specular_texture.image.clone()),
                    blink_state: 0, // Default to eyes open
                    blood_overlay_texture: None,
                    blood_params: bevy::math::Vec4::new(0.0, 0.0, 0.0, 0.0),
                },
            });
            material_cache.insert(material_cache_key, handle.clone());
            handle
        };

        let mut collision_filter = COLLISION_FILTER_INSPECTABLE;

        if object_part.collision_shape.is_some() {
            if collision_group != COLLISION_GROUP_ZONE_EVENT_OBJECT
                && collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT
                && !object_part
                    .collision_flags
                    .contains(ZscCollisionFlags::HEIGHT_ONLY)
            {
                collision_filter |= COLLISION_FILTER_COLLIDABLE | COLLISION_GROUP_PHYSICS_TOY;
            }

            if collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT {
                if !object_part
                    .collision_flags
                    .contains(ZscCollisionFlags::NOT_PICKABLE)
                {
                    collision_filter |= COLLISION_FILTER_CLICKABLE;
                }

                if !object_part
                    .collision_flags
                    .contains(ZscCollisionFlags::NOT_MOVEABLE)
                {
                    collision_filter |= COLLISION_FILTER_MOVEABLE;
                }
            }
        }

        // Determine if this part should cast shadows based on material transparency
        // Opaque and alpha-masked materials cast shadows, alpha-blended materials don't
        let is_transparent = zsc_material.alpha_enabled && !zsc_material.z_write_enabled;

        let part_entity = commands
            .spawn((
                EditorSelectable,
                part_object_type(ZoneObjectPart {
                    ifo_object_id,
                    zsc_object_id,
                    zsc_part_id: part_index,
                    mesh_path: zsc.meshes[mesh_id].path().to_string_lossy().into(),
                    collision_shape: (&object_part.collision_shape).into(),
                    collision_not_moveable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_MOVEABLE),
                    collision_not_pickable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE),
                    collision_height_only: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::HEIGHT_ONLY),
                    collision_no_camera: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_CAMERA_COLLISION),
                }),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                part_transform,
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                // No explicit Aabb: Bevy `calculate_bounds` auto-computes tight bounds
                // from the Mesh3d once the async ZMS mesh loads. The previous ±100000
                // box made every part always-visible in main, shadow and reflection passes.
                RenderLayers::layer(0),
                ColliderParent::new(object_entity),
                AsyncCollider(ComputedColliderShape::TriMesh(
                    bevy_rapier3d::prelude::TriMeshFlags::FIX_INTERNAL_EDGES,
                )),
                CollisionGroups::new(collision_group, collision_filter),
            ))
            .id();

        // Only disable shadow casting for truly transparent (alpha-blended) materials
        // Opaque and alpha-masked materials should cast shadows
        if is_transparent {
            commands.entity(part_entity).insert(NotShadowCaster);
        }

        let active_motion = object_part.animation_path.as_ref().map(|animation_path| {
            TransformAnimation::repeat(
                asset_server.load(animation_path.path().to_string_lossy().into_owned()),
                None,
            )
        });
        if let Some(active_motion) = active_motion {
            commands.entity(part_entity).insert(active_motion);
        }

        // Add wind sway effect to grass and tree leaf models based on mesh path
        let mesh_path_lower = zsc.meshes[mesh_id].path().to_string_lossy().to_lowercase();

        // Store the part's rotation to use as base_rotation for wind sway
        let part_base_rotation = part_transform.rotation;

        // Generate a random phase offset based on object position for natural variation
        let phase_offset = (object_instance.position.x * 0.1 + object_instance.position.y * 0.13)
            .fract()
            * std::f32::consts::TAU;

        // Check for grass models (identified by "grass" in the mesh name)
        if mesh_path_lower.contains("grass") {
            commands.entity(part_entity).insert(
                WindSway::for_grass()
                    .with_base_rotation(part_base_rotation)
                    .with_phase_offset(phase_offset),
            );
        }
        // Check for tree leaf models (identified by "leaf" or "leaves" in the mesh name)
        else if mesh_path_lower.contains("leaf") || mesh_path_lower.contains("leaves") {
            commands.entity(part_entity).insert(
                WindSway::for_tree_leaves()
                    .with_base_rotation(part_base_rotation)
                    .with_phase_offset(phase_offset),
            );
        }
        // Check for tree foliage (alternative naming conventions)
        else if mesh_path_lower.contains("foliage") || mesh_path_lower.contains("canopy") {
            commands.entity(part_entity).insert(
                WindSway::for_tree_leaves()
                    .with_base_rotation(part_base_rotation)
                    .with_phase_offset(phase_offset),
            );
        }
        // Check for bush/shrub models (similar swaying behavior to grass)
        else if mesh_path_lower.contains("bush")
            || mesh_path_lower.contains("shrub")
            || mesh_path_lower.contains("plant")
        {
            commands.entity(part_entity).insert(
                WindSway::for_grass()
                    .with_base_rotation(part_base_rotation)
                    .with_phase_offset(phase_offset),
            );
        }
        // Check for tree models - apply wind sway to tree tops (leaves) but NOT trunks
        // Tree naming convention: TREE004.ZMS = top/leaves (sway), TREE004B.ZMS = trunk (no sway)
        // The "B" suffix indicates the trunk/base part which should remain static
        else if mesh_path_lower.contains("tree") {
            // Check if this is a trunk file (ends with "b.zms" or contains "b." before extension)
            let is_trunk = mesh_path_lower.ends_with("b.zms")
                || mesh_path_lower.ends_with("b")
                || mesh_path_lower
                    .rsplit_once('.')
                    .map_or(false, |(name, _ext)| name.ends_with('b'));

            if !is_trunk {
                // This is the tree top/leaves - apply wind sway
                commands.entity(part_entity).insert(
                    WindSway::for_tree_leaves()
                        .with_base_rotation(part_base_rotation)
                        .with_phase_offset(phase_offset),
                );
            }
            // If it's a trunk (ends with B), don't apply wind sway - trunk stays static
        }

        commands.entity(object_entity).add_child(part_entity);
    }

    object_entity
}

pub(super) fn spawn_animated_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
    stb_morph_object: &StbFile,
    object_instance: &IfoObject,
) -> Entity {
    let object_id = object_instance.object_id as usize;
    let mesh_path = stb_morph_object.get(object_id, 1).to_string();
    let motion_path = stb_morph_object.get(object_id, 2).to_string();
    let texture_path = stb_morph_object.get(object_id, 3).to_string();

    let alpha_enabled = stb_morph_object.get_int(object_id, 4) != 0;
    let two_sided = stb_morph_object.get_int(object_id, 5) != 0;
    let alpha_test_enabled = stb_morph_object.get_int(object_id, 6) != 0;

    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mesh: Handle<Mesh> = asset_server.load(&mesh_path);

    // Handle NULL texture paths for animated objects
    let texture_handle = if texture_path.is_empty() || texture_path == "NULL" {
        log::warn!("[SPAWN ANIMATED OBJECT] NULL or empty texture path, using fallback");
        asset_server.load::<Image>("ETC/SPECULAR_SPHEREMAP.DDS")
    } else {
        asset_server.load::<Image>(&texture_path)
    };

    let motion_path_buf = ZmoTextureAssetLoader::convert_path(&motion_path);
    let motion_texture_handle =
        asset_server.load(ZmoTextureAssetLoader::convert_path_texture(&motion_path));
    let motion_handle = asset_server.load(motion_path_buf.to_string_lossy().into_owned());

    let material = effect_mesh_materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(texture_handle),
            // PBR properties for realistic lighting on animated objects
            perceptual_roughness: 0.8, // Higher roughness for matte vegetation/outdoor objects
            metallic: 0.0,             // Non-metallic for organic materials
            alpha_mode: if alpha_test_enabled {
                AlphaMode::Mask(0.5)
            } else {
                AlphaMode::Opaque
            },
            double_sided: two_sided,
            ..Default::default()
        },
        extension: RoseEffectExtension {
            animation_texture: Some(motion_texture_handle.clone()),
            animation_state: crate::render::EffectMeshAnimationUniform::default(),
        },
    });

    // Determine if this animated object should cast shadows based on material transparency
    // Opaque and alpha-masked materials cast shadows, alpha-blended materials don't
    let is_transparent = alpha_enabled && !alpha_test_enabled;

    let animated_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::AnimatedObject(ZoneObjectAnimatedObject {
                mesh_path: mesh_path.to_string(),
                motion_path: motion_path.to_string(),
                texture_path: texture_path.to_string(),
            }),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            MeshAnimation::repeat(motion_handle, None),
            object_transform,
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // No explicit Aabb: auto-computed from mesh once loaded (see calculate_bounds).
            RenderLayers::layer(0),
            AsyncCollider(ComputedColliderShape::TriMesh(
                bevy_rapier3d::prelude::TriMeshFlags::empty(),
            )),
            CollisionGroups::new(COLLISION_GROUP_ZONE_OBJECT, COLLISION_FILTER_INSPECTABLE),
        ))
        .id();

    // Only disable shadow casting for truly transparent (alpha-blended) materials
    // Opaque and alpha-masked materials should cast shadows
    if is_transparent {
        commands.entity(animated_entity).insert(NotShadowCaster);
    }

    animated_entity
}

pub(super) fn spawn_effect_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    vfs_resource: &VfsResource,
    effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
    particle_materials: &mut Assets<ParticleMaterial>,
    meshes: &mut Assets<bevy::prelude::Mesh>,
    storage_buffers: &mut Assets<bevy::render::storage::ShaderStorageBuffer>,
    effect_object: &IfoEffectObject,
    ifo_object_id: usize,
    effect_cache: &EffectCache,
) -> Entity {
    let object = &effect_object.object;
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            object.rotation.x,
            object.rotation.z,
            -object.rotation.y,
            object.rotation.w,
        ))
        .with_scale(Vec3::new(object.scale.x, object.scale.z, object.scale.y));

    let effect_object_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::EffectObject {
                ifo_object_id,
                effect_path: effect_object
                    .effect_path
                    .path()
                    .to_string_lossy()
                    .to_string(),
            },
            object_transform,
            GlobalTransform::from(object_transform),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // No Aabb on mesh-less effect root (children self-cull via calculate_bounds).
            RenderLayers::layer(0),
        ))
        .id();

    spawn_effect(
        &vfs_resource.vfs,
        commands,
        asset_server,
        particle_materials,
        effect_mesh_materials,
        storage_buffers,
        meshes,
        (&effect_object.effect_path).into(),
        false,
        Some(effect_object_entity),
        Some(effect_cache),
        Some(Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0),
    );

    effect_object_entity
}

pub(super) fn spawn_sound_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    sound_object: &IfoSoundObject,
    ifo_object_id: usize,
) -> Entity {
    let object = &sound_object.object;
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(object.position.x, object.position.z, -object.position.y) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            object.rotation.x,
            object.rotation.z,
            -object.rotation.y,
            object.rotation.w,
        ))
        .with_scale(Vec3::new(object.scale.x, object.scale.z, object.scale.y));

    let sound_path_str = sound_object.sound_path.path().to_string_lossy().to_string();

    // Handle NULL sound paths - skip loading if path is NULL or empty
    if sound_path_str.is_empty() || sound_path_str == "NULL" {
        log::warn!("[SPAWN SOUND OBJECT] NULL or empty sound path, skipping sound loading");
        let effect_object_entity = commands
            .spawn((
                EditorSelectable,
                ZoneObject::SoundObject {
                    ifo_object_id,
                    sound_path: sound_path_str.clone(),
                },
            object_transform,
            GlobalTransform::from(object_transform),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // No Aabb on mesh-less effect root.
            RenderLayers::layer(0),
            ))
            .id();
        return effect_object_entity;
    }

    let effect_object_entity = commands
        .spawn((
            EditorSelectable,
            ZoneObject::SoundObject {
                ifo_object_id,
                sound_path: sound_path_str.clone(),
            },
            SpatialSound::new_repeating(asset_server.load(&sound_path_str)),
            SoundRadius::new(sound_object.range as f32 / 10.0),
            object_transform,
            GlobalTransform::from(object_transform),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // No Aabb on mesh-less sound object.
            RenderLayers::layer(0),
        ))
        .id();

    effect_object_entity
}
