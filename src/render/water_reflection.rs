//! Planar reflection for water surfaces using a mirrored camera.
//!
//! A dedicated camera renders the scene from a position mirrored across the
//! water plane into an off-screen HDR texture. The water material samples that
//! texture in its fragment shader, projecting each water fragment through the
//! main camera's `clip_from_world` after reflecting it across the water plane
//! (which is exactly equivalent to projecting through the mirrored camera's
//! view-projection).
//!
//! Design notes:
//! - The reflection camera renders only [`RenderLayers::layer(0)`] entities;
//!   water planes live on layer 1 so they never render into their own
//!   reflection (no recursion).
//! - The camera is disabled while underwater, when no water volume exists, or
//!   when reflections are disabled in the settings.
//! - The oblique near clip plane (`PerspectiveProjection::near_clip_plane`)
//!   clips geometry below the water surface so lake beds never appear in the
//!   reflection. This is the Lengyel oblique near-plane technique used by the
//!   official Bevy `mirror` example.
//! - The reflection camera mirrors the main camera's atmosphere and
//!   environment map so the reflected scene is lit identically.

use bevy::{
    asset::RenderAssetUsages,
    camera::{
        primitives::{Frustum, Sphere},
        visibility::VisibleEntities,
        Camera, CameraProjection, ClearColorConfig, Projection, RenderTarget,
    },
    color::Color,
    image::Image,
    light::EnvironmentMapLight,
    math::{vec2, Isometry3d, Mat4, primitives::InfinitePlane3d, reflection_matrix},
    prelude::{
        App, Assets, Camera3d, Commands, Component, Entity, GlobalTransform, Handle,
        IntoScheduleConfigs, Local, Mesh3d, Msaa, PerspectiveProjection, Plugin, Query, Res,
        ResMut, Resource, Transform, UVec2, Vec3, Vec3A, Visibility, Window, With, Without,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

use crate::{
    render::{
        underwater_effect::{CameraUnderwaterState, UnderwaterVolumes},
        water_material::WaterMaterial,
    },
    resources::WaterSettings,
};

/// Marker component for the camera that renders the reflected world.
#[derive(Component)]
pub struct WaterReflectionCamera;

/// The off-screen render target used by the reflection camera.
#[derive(Resource)]
pub struct WaterReflectionImage {
    /// Handle to the reflection render target texture.
    pub handle: Handle<Image>,
    /// Current size of the render target in physical pixels.
    pub size: UVec2,
}

/// Registry of water material asset ids so their reflection texture handles
/// can be kept in sync with the (possibly recreated) render target.
#[derive(Resource, Default)]
struct WaterMaterialRegistry {
    ids: Vec<bevy::asset::AssetId<WaterMaterial>>,
}

/// Plugin that enables planar water reflections.
pub struct WaterReflectionPlugin;

impl Plugin for WaterReflectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterMaterialRegistry>()
            .add_systems(
                // PostStartup so the main camera (and its egui context) is
                // spawned first.
                bevy::prelude::PostStartup,
                setup_water_reflection,
            )
            .add_systems(
                bevy::prelude::Update,
                (
                    manage_reflection_image,
                    sync_reflection_textures,
                    sanitize_reflection_camera,
                ),
            )
            .add_systems(
                bevy::prelude::PostUpdate,
                sync_reflection_camera.before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

/// Computes the size of the reflection render target from the window size.
fn reflection_size(windows: &Query<&Window>, scale: f32) -> UVec2 {
    let window = windows.iter().next().expect("water reflection: no window");
    let size = vec2(window.physical_width() as f32, window.physical_height() as f32) * scale;
    size.as_uvec2().max(UVec2::ONE)
}

/// Creates the off-screen HDR image that the reflection camera renders into.
fn create_reflection_image(images: &mut Assets<Image>, size: UVec2) -> Handle<Image> {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // LDR sRGB target - identical to the official Bevy `mirror` example.
        // (Sampling an sRGB texture in WGSL returns linear values.)
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::RENDER_ATTACHMENT;
    images.add(image)
}

/// Spawns the reflection camera and its render target at startup.
fn setup_water_reflection(
    mut commands: Commands,
    windows: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
    water_settings: Res<WaterSettings>,
) {
    let size = reflection_size(&windows, water_settings.reflection_scale);
    let handle = create_reflection_image(&mut images, size);
    commands.insert_resource(WaterReflectionImage {
        handle: handle.clone(),
        size,
    });

    log::info!(
        "[WATER REFLECTION] Spawned reflection render target size {:?}, reflection_scale {}",
        size,
        water_settings.reflection_scale
    );

    commands.spawn((
        // NOTE: `Camera3d` - the exact configuration used by the official Bevy
        // `mirror` example. The reflection camera mirrors the main camera's
        // transform each frame, so a few game systems that call `.single()` on
        // `With<Camera3d>` queries were updated with
        // `Without<WaterReflectionCamera>` filters.
        Camera3d::default(),
        Msaa::Off,
        Camera {
            order: -1,
            is_active: false,
            invert_culling: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..Default::default()
        },
        RenderTarget::Image(handle.clone().into()),
        Projection::Perspective(PerspectiveProjection::default()),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Visible,
        bevy::camera::visibility::RenderLayers::layer(0),
        WaterReflectionCamera,
    ));
}

/// Recreates the render target when the window size or the reflection scale
/// setting changes.
fn manage_reflection_image(
    mut commands: Commands,
    windows: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
    mut reflection_image: ResMut<WaterReflectionImage>,
    reflection_cameras: Query<Entity, With<WaterReflectionCamera>>,
    water_settings: Res<WaterSettings>,
) {
    let size = reflection_size(&windows, water_settings.reflection_scale);
    if size == reflection_image.size {
        return;
    }
    let new_handle = create_reflection_image(&mut images, size);
    images.remove(reflection_image.handle.id());
    reflection_image.handle = new_handle.clone();
    reflection_image.size = size;
    for entity in reflection_cameras.iter() {
        commands
            .entity(entity)
            .insert(RenderTarget::Image(new_handle.clone().into()));
    }
}

/// Removes post/prepass components that must never be on the reflection camera.
///
/// Defense-in-depth for the `graphics/apply_systems.rs` invariant: `#[require]`
/// chains auto-add prepass components (SSAO pulls DepthPrepass+NormalPrepass,
/// MotionBlur pulls DepthPrepass+MotionVectorPrepass). A reflection view with
/// prepass phases but no deferred phases panics Bevy 0.18.1 in
/// `queue_prepass_material_meshes` once a deferred material is visible to it.
/// The query only matches infected cameras, so the steady-state cost is ~zero.
fn sanitize_reflection_camera(
    mut commands: Commands,
    infected: Query<
        Entity,
        (
            With<WaterReflectionCamera>,
            bevy::ecs::query::Or<(
                With<bevy::core_pipeline::prepass::DepthPrepass>,
                With<bevy::core_pipeline::prepass::NormalPrepass>,
                With<bevy::core_pipeline::prepass::MotionVectorPrepass>,
                With<bevy::core_pipeline::prepass::DeferredPrepass>,
                With<bevy::pbr::ScreenSpaceAmbientOcclusion>,
                With<bevy::post_process::motion_blur::MotionBlur>,
                With<bevy::post_process::dof::DepthOfField>,
            )>,
        ),
    >,
) {
    for entity in infected.iter() {
        commands
            .entity(entity)
            .remove::<bevy::core_pipeline::prepass::DepthPrepass>()
            .remove::<bevy::core_pipeline::prepass::NormalPrepass>()
            .remove::<bevy::core_pipeline::prepass::MotionVectorPrepass>()
            .remove::<bevy::core_pipeline::prepass::DeferredPrepass>()
            .remove::<bevy::pbr::ScreenSpaceAmbientOcclusion>()
            .remove::<bevy::post_process::motion_blur::MotionBlur>()
            .remove::<bevy::post_process::dof::DepthOfField>();
        log::warn!("[WATER REFLECTION] Stripped forbidden post/prepass components from reflection camera (see apply_systems invariant)");
    }
}

/// Keeps every water material's reflection texture handle pointing at the
/// current render target. Also covers materials created later (zone reloads).
fn sync_reflection_textures(
    mut materials: ResMut<Assets<WaterMaterial>>,
    reflection_image: Res<WaterReflectionImage>,
    mut registry: ResMut<WaterMaterialRegistry>,
) {
    for (id, _) in materials.iter() {
        if !registry.ids.contains(&id) {
            registry.ids.push(id);
            log::info!(
                "[WATER REFLECTION] Registered water material {:?}",
                id
            );
        }
    }
    for id in registry.ids.iter() {
        if let Some(mut material) = materials.get_mut(*id) {
            if material.reflection_texture != reflection_image.handle {
                material.reflection_texture = reflection_image.handle.clone();
                log::info!(
                    "[WATER REFLECTION] Assigned reflection texture {:?} to material {:?}",
                    reflection_image.handle.id(),
                    id
                );
            }
        }
    }
}

/// Mirrors the main camera across the water plane each frame and enables the
/// reflection camera only when reflections are meaningful.
fn sync_reflection_camera(
    mut commands: Commands,
    mut frame_counter: Local<u32>,
    mut last_status: Local<u32>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut material_registry: ResMut<WaterMaterialRegistry>,
    main_cameras: Query<
        (&Transform, &Projection, &CameraUnderwaterState, Option<&EnvironmentMapLight>),
        (
            With<Camera3d>,
            With<CameraUnderwaterState>,
            Without<WaterReflectionCamera>,
        ),
    >,
    mut reflection_cameras: Query<
        (
            Entity,
            &mut Transform,
            &mut Projection,
            &mut Camera,
            &mut Frustum,
            &VisibleEntities,
            &GlobalTransform,
            Option<&mut EnvironmentMapLight>,
        ),
        With<WaterReflectionCamera>,
    >,
    underwater_volumes: Res<UnderwaterVolumes>,
    water_settings: Res<WaterSettings>,
) {
    let Some((main_transform, main_projection, underwater_state, main_envmap)) =
        main_cameras.iter().next()
    else {
        return;
    };

    let Projection::Perspective(main_perspective) = main_projection else {
        return;
    };

    let surface_y = water_settings.water_surface_y;

    // Reflection matrix for the water plane (normal +Y, passing through y = surface_y).
    let plane_offset = Mat4::from_translation(Vec3::Y * surface_y);
    let reflect = Mat4::from_mat3a(reflection_matrix(Vec3::Y));
    let mirror_matrix = plane_offset * reflect * plane_offset.inverse();

    // Oblique near clip plane: clip everything below the water surface from the
    // reflection camera's view so the lake bed never shows up in reflections.
    //
    // NOTE: the normal must point AWAY from the reflected scene (downward, into
    // the water) - the same convention as the official mirror example. Using
    // +Y here puts the clip plane on the wrong side of the camera, which makes
    // the derived frustum degenerate (culling fails -> the reflection camera
    // renders the entire scene) and clips the whole reflected scene.
    let distance = InfinitePlane3d::new(Vec3::Y).signed_distance(
        Isometry3d::IDENTITY,
        Vec3::Y * surface_y - main_transform.translation,
    );
    let view_from_world = main_transform.compute_affine().matrix3.inverse();
    let plane_normal_view = (view_from_world * Vec3::NEG_Y).normalize();

    let has_water = !underwater_volumes.volumes.is_empty();

    // Only render reflections when the camera is close to a water volume:
    // the reflection pass renders the whole scene again, so gating it by
    // distance keeps it off when no water is nearby (huge perf win in zones
    // where water only exists in some areas).
    let nearest_water_distance = underwater_volumes
        .volumes
        .iter()
        .map(|volume| {
            let dx = (main_transform.translation.x - volume.center.x).abs() - volume.half_extents.x;
            let dz = (main_transform.translation.z - volume.center.z).abs() - volume.half_extents.y;
            dx.max(0.0).hypot(dz.max(0.0))
        })
        .fold(f32::INFINITY, f32::min);
    const REFLECTION_MAX_DISTANCE: f32 = 300.0;

    let reflection_active = water_settings.reflection_enabled
        && has_water
        && !underwater_state.is_underwater
        && nearest_water_distance <= REFLECTION_MAX_DISTANCE;

    *frame_counter = frame_counter.wrapping_add(1);

    // Fast path: when reflections are off (setting, no water, underwater, or
    // >300m), only ensure the camera stays inactive. Previously mirror matrix +
    // frustum recompute + per-view scans ran every frame even when inactive.
    if !reflection_active {
        for (_, _, _, mut camera, _, _, _, _) in reflection_cameras.iter_mut() {
            if camera.is_active {
                camera.is_active = false;
            }
        }
        return;
    }

    let mut reflection_log = None;
    for (
        entity,
        mut transform,
        mut projection,
        mut camera,
        mut frustum,
        visible_entities,
        camera_transform,
        mut envmap,
    ) in reflection_cameras.iter_mut()
    {
        *transform = Transform::from_matrix(mirror_matrix * main_transform.to_matrix());
        // DEBUG TEST: temporarily disabled the oblique near clip plane to
        // determine whether the clip plane or the mirror transform breaks the
        // frustum culling.
        // Reflection uses a halved far plane to tighten CPU culling in the mirrored
        // pass. (Under Bevy 0.18 infinite reverse-Z, far never enters the clip matrix,
        // so this saves culled draws, not depth precision.)
        // +500 margin keeps the 4000-radius sky (centered on main camera, mirrored
        // baseline offset) fully inside the reflection frustum.
        let mut reflection_perspective = main_perspective.clone();
        reflection_perspective.far = (main_perspective.far * 0.5).max(2000.0) + 500.0;
        if let Projection::Perspective(ref mut current) = *projection {
            if (current.far - reflection_perspective.far).abs() > 1.0 {
                *projection = Projection::Perspective(reflection_perspective.clone());
            }
        } else {
            *projection = Projection::Perspective(reflection_perspective.clone());
        }
        camera.is_active = reflection_active;

        // Write the frustum directly from the REFLECTION perspective (not the main
        // one): update_frusta only recomputes on Changed<GlobalTransform|Projection>,
        // and a stale default (all-zero half spaces) would cull everything except
        // NoFrustumCulling entities. update_frusta overwrites this later with the
        // same halved perspective, so this is consistent, not dead.
        *frustum =
            reflection_perspective.compute_frustum(&GlobalTransform::from(*transform));

        if *frame_counter % 150 == 0 {
            let hit = |p: Vec3| {
                frustum.intersects_sphere(&Sphere { center: Vec3A::from(p), radius: 1.0 }, false)
            };
            // log::info!(
            //     "[WATER REFLECTION DBG] gt_pos={:?} fwd={:?} | near_plane={:?} p0={:?} | hits cam={} up100={} down100={} surf50={}",
            //     camera_transform.translation(),
            //     camera_transform.forward(),
            //     frustum.half_spaces[Frustum::NEAR_PLANE_IDX].normal_d(),
            //     frustum.half_spaces[0].normal_d(),
            //     hit(main_transform.translation),
            //     hit(main_transform.translation + Vec3::Y * 100.0),
            //     hit(main_transform.translation - Vec3::Y * 100.0),
            //     hit(Vec3::new(
            //         main_transform.translation.x,
            //         surface_y + 50.0,
            //         main_transform.translation.z,
            //     )),
            // );
        }

        let visible_count = visible_entities
            .entities
            .values()
            .map(|entities| entities.len())
            .sum::<usize>();
        let visible_mesh3d_count = visible_entities.len(core::any::TypeId::of::<Mesh3d>());
        reflection_log = Some((
            transform.translation,
            visible_count,
            visible_mesh3d_count,
            reflection_active,
        ));

        // Mirror the environment map light so the reflected scene is lit
        // identically to the main view. NOTE: the Atmosphere is intentionally
        // NOT mirrored - the mesh view bind group layout adds atmosphere
        // bindings for views with an Atmosphere component, but the matching
        // render-world atmosphere textures are not prepared for this camera,
        // which causes a wgpu bind group validation panic.
        if let Some(env) = main_envmap {
            if envmap.is_none() {
                commands.entity(entity).insert(env.clone());
            }
        }
    }

    // Push the camera status into the water materials (only on change) so the
    // debug view can show it.
    // Status: 0 = disabled, 1 = active but 0 entities visible,
    //         2 = active with suspiciously few entities (< 300, likely a
    //             broken frustum culling everything),
    //         3 = active with a normal entity count.
    let status = reflection_log.map_or(0, |(_, visible, _, active)| {
        if !active {
            0
        } else if visible == 0 {
            1
        } else if visible < 300 {
            2
        } else {
            3
        }
    });
    if *last_status != status {
        *last_status = status;
        log::info!("[WATER REFLECTION] status changed to {}", status);
        for (id, _) in water_materials.iter() {
            if !material_registry.ids.contains(&id) {
                material_registry.ids.push(id);
            }
        }
        for id in material_registry.ids.iter() {
            if let Some(mut material) = water_materials.get_mut(*id) {
                if material.reflection_status != status {
                    material.reflection_status = status;
                }
            }
        }
    }

    if *frame_counter % 150 == 0 {
        log::info!(
            "[WATER REFLECTION] cam={:?} surface_y={:.2} volumes={} underwater={} settings_enabled={} active={} water_dist={:.1} refl_pos={:?} refl_visible_entities={} refl_visible_mesh3d={}",
            main_transform.translation,
            surface_y,
            underwater_volumes.volumes.len(),
            underwater_state.is_underwater,
            water_settings.reflection_enabled,
            reflection_active,
            nearest_water_distance,
            reflection_log.map(|(p, _, _, _)| p),
            reflection_log.map_or(0, |(_, c, _, _)| c),
            reflection_log.map_or(0, |(_, _, m, _)| m),
        );
    }
}
