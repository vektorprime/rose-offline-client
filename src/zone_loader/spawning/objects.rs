use super::*;
use std::collections::HashMap;

use crate::render::rose_object_material;

/// Everything a zone object part's material is built from. Parts with equal keys
/// get identical materials, so they share one asset (one bind group), which lets
/// Bevy batch their draws in every view (main, shadow cascades, reflection).
#[derive(Hash, PartialEq, Eq)]
pub(super) struct ObjectMaterialKey {
    texture_path: String,
    two_sided: bool,
    alpha_enabled: bool,
    /// `alpha_test` threshold bits.
    alpha_test: Option<u32>,
    /// ZSC specular flag (whether the specular map is bound).
    specular_enabled: bool,
    /// Lightmap page path and its parts per row. The part's cell within the page is
    /// not part of the material: it is the part entity's `MeshTag` (see below).
    lightmap: Option<(PathBuf, u32)>,
}

/// Zone-wide cache of zone object materials, shared by all `spawn_object` calls of
/// one `spawn_zone`.
///
/// INVARIANT: zone statics never receive BloodOverlay (character-only) and nothing
/// else mutates these materials, so sharing one asset across parts and objects is
/// safe. If zone objects ever gain overlays, the blood system must clone-on-write
/// instead of mutating the shared asset.
pub(super) type ObjectMaterialCache =
    HashMap<ObjectMaterialKey, Handle<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>;

pub(super) fn spawn_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone_loading_assets: &mut Vec<UntypedHandle>,
    object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    material_cache: &mut ObjectMaterialCache,
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
        let lit_part = lit_object
            .and_then(|lit_object| {
                for part in lit_object.parts.iter() {
                    if part_index == part.object_part_index as usize {
                        return Some(part);
                    }
                }

                lit_object.parts.get(part_index)
            })
            // A page without a grid has no cells (the old offset math divided by it).
            .filter(|lit_part| lit_part.parts_per_row > 0);
        let lightmap_page_path = lit_part.map(|lit_part| lightmap_path.join(&lit_part.filename));
        let lightmap_texture = lightmap_page_path.as_ref().map(|path| {
            let path_str = path.to_string_lossy().into_owned();
            asset_server.load::<bevy::prelude::Image>(&path_str)
        });
        // Lightmap UV = uv_b * scale + (column, row) of the part's cell in the page.
        // The material holds the page layout (x, y = 0, z = scale, w = parts per row);
        // the shader rebuilds (column, row) = (cell % per_row, cell / per_row) from the
        // part's MeshTag = cell index. Same integers and the same float ops as when the
        // offset was stored per material, so materials can be shared per page.
        let lightmap_params = lit_part.map_or(Vec4::new(0.0, 0.0, 1.0, 0.0), |lit_part| {
            Vec4::new(
                0.0,
                0.0,
                1.0 / lit_part.parts_per_row as f32,
                lit_part.parts_per_row as f32,
            )
        });

        // NOTE: material_id was already bounds-checked above; this fetch is for local use.
        let material_id = object_part.material_id as usize;

        let zsc_material = zsc.materials[material_id].clone();
        let material_path = zsc_material.path.path().to_string_lossy().into_owned();

        let material_cache_key = ObjectMaterialKey {
            texture_path: material_path.clone(),
            two_sided: zsc_material.two_sided,
            alpha_enabled: zsc_material.alpha_enabled,
            alpha_test: zsc_material.alpha_test.map(f32::to_bits),
            specular_enabled: zsc_material.specular_enabled,
            lightmap: lightmap_page_path
                .clone()
                .zip(lit_part.map(|lit_part| lit_part.parts_per_row)),
        };
        let material = if let Some(cached) = material_cache.get(&material_cache_key) {
            cached.clone()
        } else {
            let base_texture_handle: Handle<Image> = asset_server.load(&material_path);

            // Forward-rendered RoseObjectExtension material (lightmap, specular); see
            // `rose_object_material`.
            let handle = object_materials.add(rose_object_material(
                StandardMaterial {
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
                RoseObjectExtension {
                    lightmap_params,
                    lightmap_texture: lightmap_texture.clone(),
                    // Only for ZSC materials flagged specular (the original's
                    // sphere-map specular); others keep the standard reflectance.
                    specular_texture: zsc_material
                        .specular_enabled
                        .then(|| specular_texture.image.clone()),
                    blood_overlay_texture: None,
                    blood_params: bevy::math::Vec4::new(0.0, 0.0, 0.0, 0.0),
                },
            ));
            material_cache.insert(material_cache_key, handle.clone());
            handle
        };

        // Wind sway kind from the mesh path (grass, leaves, tree tops, ...).
        let wind_sway = WindSway::for_mesh_path(&zsc.meshes[mesh_id].path().to_string_lossy());

        // Grass-kind vegetation (grass, bushes, plants) is walk-through: characters
        // wade through it instead of bumping into it or stepping up onto it, even
        // where the ZSC gives it a collision shape (e.g. Junon GRASS002/GRASS003).
        // Its collider keeps only INSPECTABLE (map editor selection, the debug
        // inspector, name-tag occlusion): no movement, ground-height, camera,
        // click-to-move or physics-toy query accepts that bit alone.
        let walk_through = wind_sway
            .as_ref()
            .is_some_and(|wind_sway| wind_sway.is_grass);

        let mut collision_filter = COLLISION_FILTER_INSPECTABLE;

        if object_part.collision_shape.is_some() && !walk_through {
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
                // Shape built once per mesh and shared (see SharedMeshCollider).
                SharedMeshCollider(bevy_rapier3d::prelude::TriMeshFlags::FIX_INTERNAL_EDGES),
                CollisionGroups::new(collision_group, collision_filter),
            ))
            .id();

        // Only disable shadow casting for truly transparent (alpha-blended) materials
        // Opaque and alpha-masked materials should cast shadows
        if is_transparent {
            commands.entity(part_entity).insert(NotShadowCaster);
        }

        // The part's lightmap cell (see lightmap_params above).
        if let Some(lit_part) = lit_part {
            commands
                .entity(part_entity)
                .insert(bevy_mesh::MeshTag(lit_part.part_index));
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

        // Wind sway rotates the part around its own spawn rotation. It only drives
        // the part's Transform, so it works with or without movement collision.
        if let Some(wind_sway) = wind_sway {
            // Phase offset from the object position, so neighbours do not sway in sync
            let phase_offset =
                (object_instance.position.x * 0.1 + object_instance.position.y * 0.13).fract()
                    * std::f32::consts::TAU;

            commands.entity(part_entity).insert(
                wind_sway
                    .with_base_rotation(part_transform.rotation)
                    .with_phase_offset(phase_offset),
            );
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
    let render_states = crate::render::EffectMeshRenderStates {
        alpha_enabled,
        alpha_test_enabled,
        two_sided,
        depth_test_enabled: stb_morph_object.get_int(object_id, 7) != 0,
        depth_write_enabled: stb_morph_object.get_int(object_id, 8) != 0,
        src_blend_factor: crate::effect_loader::decode_blend_factor(
            stb_morph_object.get_int(object_id, 9) as u32,
        ),
        dst_blend_factor: crate::effect_loader::decode_blend_factor(
            stb_morph_object.get_int(object_id, 10) as u32,
        ),
        blend_op: crate::effect_loader::decode_blend_op(
            stb_morph_object.get_int(object_id, 11) as u32,
        ),
    };

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

    // The STB's blend equation and depth states (blended morph objects such as
    // glows and waterfalls were drawn opaque). rose_effect_material renders
    // forward: the water reflection camera has no deferred prepass, so deferred
    // animated objects were missing from reflections.
    let material = effect_mesh_materials.add(crate::render::rose_effect_material(
        StandardMaterial {
            base_color_texture: Some(texture_handle),
            // PBR properties for realistic lighting on animated objects
            perceptual_roughness: 0.8, // Higher roughness for matte vegetation/outdoor objects
            metallic: 0.0,             // Non-metallic for organic materials
            // Additive glows keep their texture color instead of sun shading.
            unlit: alpha_enabled
                && render_states.dst_blend_factor
                    == bevy::render::render_resource::BlendFactor::One,
            ..Default::default()
        },
        Some(motion_texture_handle.clone()),
        render_states,
    ));

    // Determine if this animated object should cast shadows based on material transparency
    // Opaque and alpha-masked materials cast shadows, blended ones (transparent pass) don't
    let is_transparent = alpha_enabled
        || !render_states.depth_write_enabled
        || !render_states.depth_test_enabled;

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
            // The morph animation moves vertices outside the base mesh bounds:
            // update_mesh_animation_aabb_system (render/culling_bounds.rs) gives the
            // entity an Aabb covering the mesh and every animation frame instead, so
            // it is frustum, shadow-cascade and occlusion culled without popping.
            bevy::camera::visibility::NoAutoAabb,
            RenderLayers::layer(0),
            SharedMeshCollider(bevy_rapier3d::prelude::TriMeshFlags::empty()),
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
    storage_buffers: &mut Assets<bevy::render::storage::ShaderBuffer>,
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
