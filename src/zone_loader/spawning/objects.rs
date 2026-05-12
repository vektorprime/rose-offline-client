use super::*;

pub(super) fn spawn_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone_loading_assets: &mut Vec<UntypedHandle>,
    vfs_resource: &VfsResource,
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
    // log::info!("[SPAWN OBJECT] Spawning object: IFO id={}, ZSC id={}, parts={}",
    //     ifo_object_id, zsc_object_id, zsc.objects[zsc_object_id].parts.len());
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

    let mut part_entities: ArrayVec<Entity, 256> = ArrayVec::new();
    let mut object_entity_commands = commands.spawn((
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
        Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
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
            // log::warn!("[SPAWN OBJECT] Object {} part {} has invalid mesh_id {} (max: {}), skipping part",
            //     zsc_object_id, part_index, mesh_id, zsc.meshes.len().saturating_sub(1));
            continue;
        }

        // VALIDATION FIX: Check material_id bounds
        let material_id = object_part.material_id as usize;
        if material_id >= zsc.materials.len() {
            // log::warn!("[SPAWN OBJECT] Object {} part {} has invalid material_id {} (max: {}), skipping part",
            //     zsc_object_id, part_index, material_id, zsc.materials.len().saturating_sub(1));
            continue;
        }

        let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
            let mesh_path = zsc.meshes[mesh_id].path().to_string_lossy().into_owned();
            let mesh_path_log = mesh_path.clone();
            // log::info!("[SPAWN OBJECT] Loading mesh: {}", mesh_path_log);
            let handle = asset_server.load(&mesh_path);
            mesh_cache.insert(mesh_id, Some(handle.clone()));
            //info!("[MEMORY TRACKING] Mesh handle created: {}", mesh_path_log);
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
            let handle = asset_server.load::<bevy::prelude::Image>(&path_str);
            //info!("[MEMORY TRACKING] Lightmap texture handle created: {}", path_str);
            handle
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

        // NOTE: material_id was already validated at lines 2437-2443 above
        // This second fetch is just for local use
        let material_id = object_part.material_id as usize;

        let zsc_material = zsc.materials[material_id].clone();
        let material_path = zsc_material.path.path().to_string_lossy().into_owned();
        let material_path_log = material_path.clone();

        //log::info!("[SPAWN OBJECT] Creating material: {}", material_path_log);
        let base_texture_handle = asset_server.load(&material_path);
        //info!("[MEMORY TRACKING] Object material base texture handle created: {}", material_path_log);

        let lightmap_count = lightmap_texture.as_ref().is_some() as usize;

        // Create ExtendedMaterial with RoseObjectExtension for zone lighting support
        // This applies zone lighting ambient color to darken objects to match the original game
        let material = object_materials.add(ExtendedMaterial {
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

        // CRITICAL FIX: Validate material handle before spawning
        // Note: is_weak() was removed in Bevy 0.17, removing this check
        // let material_id = material.id();
        // let is_material_weak = material.is_weak();

        // Verify material is strong
        // if material.is_weak() {
        //     log::error!("[SPAWN OBJECT] CRITICAL: Material is weak! Object {} part {} will not render!",
        //         zsc_object_id, part_index);
        // }

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
                Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
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
        part_entities.push(part_entity);
    }

    // log::info!("[SPAWN OBJECT] Object entity created: {:?} with {} parts",
    //     object_entity, part_entities.len());
    let mesh_count = mesh_cache.iter().filter(|m| m.is_some()).count();
    //info!("[MEMORY TRACKING] Object entity created with {} mesh handles",
    //mesh_count);
    // log::info!("[MEMORY] Object entity created with {} mesh handles",
    //mesh_count);

    for object_effect in object.effects.iter() {
        let effect_transform = Transform::default()
            .with_translation(
                Vec3::new(
                    object_effect.position.x,
                    object_effect.position.z,
                    -object_effect.position.y,
                ) / 100.0,
            )
            .with_rotation(Quat::from_xyzw(
                object_effect.rotation.x,
                object_effect.rotation.z,
                -object_effect.rotation.y,
                object_effect.rotation.w,
            ))
            .with_scale(Vec3::new(
                object_effect.scale.x,
                object_effect.scale.z,
                object_effect.scale.y,
            ));

        // Effect spawning temporarily disabled (use custom materials)
        /*
        if let Some(effect_path) = zsc.effects.get(object_effect.effect_id as usize) {
            if let Some(effect_entity) = spawn_effect(
                &vfs_resource.vfs,
                commands,
                asset_server,
                particle_materials,
                effect_mesh_materials,
                effect_path.into(),
                false,
                None,
            ) {
                if let Some(parent_part_entity) = object_effect
                    .parent
                    .and_then(|parent_part_index| part_entities.get(parent_part_index as usize))
                {
                    commands
                        .entity(*parent_part_entity)
                        .add_child(effect_entity);
                } else {
                    commands.entity(object_entity).add_child(effect_entity);
                }

                commands.entity(effect_entity).insert(effect_transform);

                if matches!(object_effect.effect_type, ZscEffectType::DayNight) {
                    commands.entity(effect_entity).insert(NightTimeEffect);
                }
            }
        }
        */
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
    let z_test_enabled = stb_morph_object.get_int(object_id, 7) != 0;
    let z_write_enabled = stb_morph_object.get_int(object_id, 8) != 0;

    let src_blend_factor = stb_morph_object.get_int(object_id, 9) as u32;
    let dst_blend_factor = stb_morph_object.get_int(object_id, 10) as u32;
    let blend_op = stb_morph_object.get_int(object_id, 11) as u32;

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

    let mesh_path_str = mesh_path.clone();
    let texture_path_str = texture_path.clone();
    let motion_path_str = motion_path.clone();

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

    // Log asset creation
    //info!("[MEMORY TRACKING] Animated object mesh handle created: {}", mesh_path_str);
    //info!("[MEMORY TRACKING] Animated object texture handle created: {}", texture_path_str);
    //info!("[MEMORY TRACKING] Animated object motion texture handle created: {}",
    // ZmoTextureAssetLoader::convert_path_texture(&motion_path));
    //info!("[MEMORY TRACKING] Animated object motion handle created: {}",
    //motion_path_buf.display());

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

    //info!("[MEMORY TRACKING] Animated object material created with 3 textures (base, motion texture, motion)");

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
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
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

    // info!("[ASSET LIFECYCLE] Animated object entity spawned: {:?}", animated_entity);
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

    let effect_path_str = effect_object
        .effect_path
        .path()
        .to_string_lossy()
        .to_string();
    // info!("[ASSET LIFECYCLE] Spawning effect object: {}", effect_path_str);

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
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
        ))
        .id();

    // info!("[ASSET LIFECYCLE] Effect object entity spawned: {:?}", effect_object_entity);

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
    // info!("[ASSET LIFECYCLE] Spawning sound object: {}", sound_path_str);

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
                Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
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
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
        ))
        .id();

    // info!("[ASSET LIFECYCLE] Sound object entity spawned: {:?}", effect_object_entity);
    effect_object_entity
}
