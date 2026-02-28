//! Procedural Starry Sky Material for Bevy 0.16
//!
//! This module implements a custom material that renders:
//! - Procedural stars with multiple density layers
//! - Moon with phases
//! - Night-time only visibility
//! - Integration with the zone time system
//!
//! RENDER ORDER: The starry sky uses AlphaMode::Add which places it in the
//! Transparent3d render phase. This phase runs AFTER the Bevy Atmosphere
//! (which draws between MainOpaquePass and MainTransparentPass), ensuring
//! stars appear on top of the atmospheric scattering.

use bevy::{
    asset::{load_internal_asset, weak_handle, Handle},
    math::Vec3,
    pbr::{Material, MaterialPlugin, MaterialPipeline, MaterialPipelineKey, MeshPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        alpha::AlphaMode,
        mesh::MeshVertexBufferLayoutRef,
        render_resource::*,
        view::{ViewVisibility, InheritedVisibility},
    },
};

/// Shader handle for the starry sky shader
pub const STARRY_SKY_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5e6f7a8b-9c0d-1e2f-3a4b-5c6d7e8f9a0b");

/// Plugin that registers the starry sky material
pub struct StarrySkyMaterialPlugin;

impl Plugin for StarrySkyMaterialPlugin {
    fn build(&self, app: &mut App) {
        log::info!("[STARRY SKY PLUGIN] ========== PLUGIN BUILD START ==========");
        
        load_internal_asset!(
            app,
            STARRY_SKY_SHADER_HANDLE,
            "shaders/starry_sky.wgsl",
            Shader::from_wgsl
        );
        log::info!("[STARRY SKY PLUGIN] Internal shader asset loaded: {:?}", STARRY_SKY_SHADER_HANDLE);

        // Register the material plugin for rendering
        // AlphaMode::Add will place this in Transparent3d phase which runs AFTER atmosphere
        app.add_plugins(MaterialPlugin::<StarrySkyMaterial> {
            prepass_enabled: false,
            shadows_enabled: false,
            ..Default::default()
        });
        log::info!("[STARRY SKY PLUGIN] MaterialPlugin<StarrySkyMaterial> registered");

        // Insert default starry sky settings resource
        app.init_resource::<StarrySkySettings>();
        log::info!("[STARRY SKY PLUGIN] StarrySkySettings resource initialized");

        // Add diagnostic prepare system
        app.add_systems(Update, diagnose_starry_sky_materials);
        log::info!("[STARRY SKY PLUGIN] Diagnostic system added");

        log::info!("[STARRY SKY PLUGIN] ========== PLUGIN BUILD COMPLETE ==========");
    }
}

/// Diagnostic system to log material preparation status and visibility
/// Runs every 60 frames to report material state
fn diagnose_starry_sky_materials(
    materials: Res<Assets<StarrySkyMaterial>>,
    query: Query<(&MeshMaterial3d<StarrySkyMaterial>, Entity, &Visibility, Option<&ViewVisibility>, Option<&InheritedVisibility>, &Transform), With<StarrySky>>,
    camera_query: Query<&GlobalTransform, With<Camera>>,
) {
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    
    // Log every 60 frames (~1 second at 60fps)
    if frame % 60 != 0 {
        return;
    }
    
    log::info!("[STARRY SKY PREPARE] ========== MATERIAL PREPARE DIAGNOSTIC ==========");
    log::info!("[STARRY SKY PREPARE] Frame: {}", frame);
    
    // Log camera position
    if let Ok(camera_transform) = camera_query.get_single() {
        let cam_pos = camera_transform.translation();
        let cam_distance = cam_pos.length();
        log::info!("[STARRY SKY PREPARE] Camera position: {:?}", cam_pos);
        log::info!("[STARRY SKY PREPARE] Camera distance from origin: {:.0} (sphere radius: 50000)", cam_distance);
        
        if cam_distance > 45000.0 {
            log::warn!("[STARRY SKY PREPARE] Camera may be near sphere edge!");
        }
    } else {
        log::warn!("[STARRY SKY PREPARE] No camera found!");
    }
    
    let entity_count = query.iter().count();
    log::info!("[STARRY SKY PREPARE] StarrySky entities with material: {}", entity_count);
    
    if entity_count == 0 {
        log::warn!("[STARRY SKY PREPARE] No StarrySky entities found! Spawn may have failed.");
        log::info!("[STARRY SKY PREPARE] ================================================");
        return;
    }
    
    let total_materials = materials.len();
    log::info!("[STARRY SKY PREPARE] Total StarrySkyMaterial assets: {}", total_materials);
    
    for (material_handle, entity, visibility, view_visibility, inherited_visibility, transform) in query.iter() {
        log::info!("[STARRY SKY PREPARE] Entity {:?}:", entity);
        log::info!("[STARRY SKY PREPARE]   Transform: {:?}", transform.translation);
        log::info!("[STARRY SKY PREPARE]   Visibility: {:?}", visibility);
        log::info!("[STARRY SKY PREPARE]   ViewVisibility: {:?}", view_visibility);
        log::info!("[STARRY SKY PREPARE]   InheritedVisibility: {:?}", inherited_visibility);
        
        // Check if entity is visible
        if let Some(view_vis) = view_visibility {
            log::info!("[STARRY SKY PREPARE]   view_visibility.get() = {}", view_vis.get());
        }
        if let Some(inherited_vis) = inherited_visibility {
            log::info!("[STARRY SKY PREPARE]   inherited_visibility.get() = {}", inherited_vis.get());
        }
        
        if let Some(material) = materials.get(&material_handle.0) {
            log::info!("[STARRY SKY PREPARE]   Material values (UNIFORMS SENT TO GPU):");
            log::info!("[STARRY SKY PREPARE]     binding 0 - time: {:.2}s", material.time);
            log::info!("[STARRY SKY PREPARE]     binding 1 - star_density: {:.3}", material.star_density);
            log::info!("[STARRY SKY PREPARE]     binding 2 - star_brightness: {:.3}", material.star_brightness);
            log::info!("[STARRY SKY PREPARE]     binding 3 - night_factor: {:.3} *** CRITICAL ***", material.night_factor);
            log::info!("[STARRY SKY PREPARE]       -> If night_factor <= 0.01, shader returns transparent!");
            log::info!("[STARRY SKY PREPARE]     binding 4 - moon_phase: {:.3}", material.moon_phase);
            log::info!("[STARRY SKY PREPARE]     binding 5 - moon_direction: {:?}", material.moon_direction);
            
            // Critical warnings
            if material.night_factor <= 0.0 {
                log::error!("[STARRY SKY PREPARE] !!! night_factor = 0 - STARS WILL BE INVISIBLE !!!");
                log::error!("[STARRY SKY PREPARE] !!! This means it's DAYTIME - check zone_time_system !!!");
            } else if material.night_factor < 0.5 {
                log::warn!("[STARRY SKY PREPARE] night_factor = {:.2} - stars will be dim (transition period)", material.night_factor);
            } else {
                log::info!("[STARRY SKY PREPARE] night_factor = {:.2} - stars SHOULD BE VISIBLE", material.night_factor);
            }
            
            if material.star_density <= 0.0 {
                log::error!("[STARRY SKY PREPARE] !!! star_density = 0 - NO STARS WILL BE GENERATED !!!");
            }
            
            if material.star_brightness <= 0.0 {
                log::error!("[STARRY SKY PREPARE] !!! star_brightness = 0 - STARS WILL BE BLACK !!!");
            }
        } else {
            log::error!("[STARRY SKY PREPARE] Entity {:?} material handle {:?} NOT FOUND in assets!", entity, material_handle.0);
        }
    }
    
    log::info!("[STARRY SKY PREPARE] ================================================");
}

/// Resource for starry sky settings
/// These control the appearance of the procedural stars
#[derive(Resource, Clone, Debug)]
pub struct StarrySkySettings {
    /// Star density (0.0 to 1.0) - controls how many stars are visible
    pub star_density: f32,
    /// Overall star brightness multiplier
    pub star_brightness: f32,
    /// Moon phase (0.0 = new moon, 0.5 = full moon, 1.0 = new moon)
    pub moon_phase: f32,
    /// Moon direction (normalized) in world space
    pub moon_direction: Vec3,
    /// Night visibility factor (0.0 = day, 1.0 = night)
    /// This is typically controlled by the zone time system
    pub night_factor: f32,
}

impl Default for StarrySkySettings {
    fn default() -> Self {
        Self {
            star_density: 0.50,        // 50% of cells have stars (~3,000-5,000 stars)
            star_brightness: 1.0,      // Normal brightness
            moon_phase: 0.5,           // Full moon
            moon_direction: Vec3::new(0.3, 0.8, 0.5).normalize(),  // Upper right
            night_factor: 0.0,         // Default to daytime (stars hidden) until zone_time_system updates it
        }
    }
}

/// Custom material for procedural starry sky rendering
/// Uses AsBindGroup derive for automatic bind group creation
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct StarrySkyMaterial {
    /// Current game time for twinkling animation
    #[uniform(0)]
    pub time: f32,
    
    /// Star density setting
    #[uniform(1)]
    pub star_density: f32,
    
    /// Star brightness multiplier
    #[uniform(2)]
    pub star_brightness: f32,
    
    /// Night visibility factor (0.0 = day, 1.0 = night)
    #[uniform(3)]
    pub night_factor: f32,
    
    /// Moon phase (0.0 to 1.0)
    #[uniform(4)]
    pub moon_phase: f32,
    
    /// Moon direction in world space (padding to vec4)
    #[uniform(5)]
    pub moon_direction: Vec3,
}

impl Default for StarrySkyMaterial {
    fn default() -> Self {
        Self {
            time: 0.0,
            star_density: 0.50,  // Match StarrySkySettings default
            star_brightness: 1.0,
            night_factor: 0.0,  // Default to daytime (stars hidden)
            moon_phase: 0.5,
            moon_direction: Vec3::new(0.3, 0.8, 0.5).normalize(),
        }
    }
}

impl Material for StarrySkyMaterial {
    fn vertex_shader() -> ShaderRef {
        STARRY_SKY_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        STARRY_SKY_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Use blend blending instead of additive for better visibility
        // AlphaMode::Blend prevents the depth prepass issues
        AlphaMode::Blend
    }

    fn depth_bias(&self) -> f32 {
        // Render behind everything else
        1.0
    }

    fn reads_view_transmission_texture(&self) -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        log::info!("[STARRY SKY SPECIALIZE] Specializing pipeline for StarrySkyMaterial");
        
        // Set up vertex buffer layout - we only need positions for a sky sphere
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        log::info!("[STARRY SKY SPECIALIZE] Vertex buffer layout configured");

        // CRITICAL FIX: Disable backface culling for sky sphere
        // The camera is INSIDE the sphere, so all triangles face away from camera
        // and would be culled by default backface culling
        descriptor.primitive.cull_mode = None;  // Disable culling entirely
        log::info!("[STARRY SKY SPECIALIZE] Backface culling DISABLED (cull_mode = None)");

        // Configure blending for additive star rendering
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
            log::info!("[STARRY SKY SPECIALIZE] Blend state configured for additive rendering");
        }

        // CRITICAL: Disable depth writes AND use Always comparison for sky
        // The sky sphere is at the far plane edge, so normal depth comparison fails
        // We want the sky to always render (behind everything) but with additive blending
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = false;
            // Use Always comparison so sky renders regardless of depth
            depth_stencil.depth_compare = bevy::render::render_resource::CompareFunction::Always;
            log::info!("[STARRY SKY SPECIALIZE] Depth writes DISABLED, depth_compare = Always");
        }

        log::info!("[STARRY SKY SPECIALIZE] Pipeline specialization complete");
        Ok(())
    }
}

/// Component marker for the starry sky entity
#[derive(Component, Default)]
pub struct StarrySky;

/// Component marker for the moon light entity
#[derive(Component, Default)]
pub struct MoonLight;

/// Helper function to create a starry sky sphere mesh
/// Creates an inverted sphere that renders around the camera
pub fn create_starry_sky_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    use bevy::math::primitives::Sphere;
    
    // Create a large sphere (inverted for sky rendering)
    let sphere = Sphere::new(500.0);
    let mut mesh = Mesh::from(sphere);
    
    // Increase subdivision for better star field resolution
    // Note: Bevy 0.16 uses Sphere primitive which has default subdivisions
    // For a sky sphere we need high detail
    
    // Flip normals for inside rendering
    if let Some(normals) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(normals) = normals {
            for normal in normals.iter_mut() {
                normal[0] = -normal[0];
                normal[1] = -normal[1];
                normal[2] = -normal[2];
            }
        }
    }
    
    // CRITICAL FIX: Reverse the winding order of triangles for inside rendering
    // When viewing a sphere from inside, the triangles are front-facing if we reverse the indices
    // Without this, backface culling removes all triangles and the sky is invisible
    if let Some(indices) = mesh.indices_mut() {
        match indices {
            bevy::render::mesh::Indices::U32(indices) => {
                // Reverse each triangle (swap v1 and v2 of each triangle)
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
            bevy::render::mesh::Indices::U16(indices) => {
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
            _ => {}
        }
    }
    
    meshes.add(mesh)
}

/// System to update the starry sky material based on time and settings
pub fn update_starry_sky_system(
    time: Res<Time>,
    starry_sky_settings: Res<StarrySkySettings>,
    mut materials: ResMut<Assets<StarrySkyMaterial>>,
    query: Query<&MeshMaterial3d<StarrySkyMaterial>, With<StarrySky>>,
) {
    // Count entities with StarrySky component
    let entity_count = query.iter().count();
    
    // Log every 60 frames (~1 second at 60fps) to avoid log spam
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = frame % 60 == 0;
    
    if should_log {
        log::info!("[STARRY SKY UPDATE] ========== UPDATE SYSTEM RUNNING ==========");
        log::info!("[STARRY SKY UPDATE] Elapsed time: {}s", time.elapsed_secs());
        log::info!("[STARRY SKY UPDATE] Delta secs: {}", time.delta_secs());
        log::info!("[STARRY SKY UPDATE] Settings changed: {}", starry_sky_settings.is_changed());
        log::info!("[STARRY SKY UPDATE] StarrySky entities found: {}", entity_count);
        log::info!("[STARRY SKY UPDATE] Settings - night_factor: {}, star_density: {}, star_brightness: {}",
            starry_sky_settings.night_factor,
            starry_sky_settings.star_density,
            starry_sky_settings.star_brightness
        );
    }
    
    if entity_count == 0 {
        if should_log {
            log::warn!("[STARRY SKY UPDATE] No StarrySky entities found! Sky may not have been spawned.");
        }
        return;
    }
    
    if starry_sky_settings.is_changed() || time.delta_secs() > 0.0 {
        let mut updated_count = 0;
        for material_handle in query.iter() {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.time = time.elapsed_secs();
                material.star_density = starry_sky_settings.star_density;
                material.star_brightness = starry_sky_settings.star_brightness;
                material.night_factor = starry_sky_settings.night_factor;
                material.moon_phase = starry_sky_settings.moon_phase;
                material.moon_direction = starry_sky_settings.moon_direction;
                updated_count += 1;
            } else {
                if should_log {
                    log::warn!("[STARRY SKY UPDATE] Material handle {:?} not found in assets!", material_handle.0);
                }
            }
        }
        
        if should_log {
            log::info!("[STARRY SKY UPDATE] Updated {} material(s)", updated_count);
            log::info!("[STARRY SKY UPDATE] ================================================");
        }
    } else {
        if should_log {
            log::info!("[STARRY SKY UPDATE] No update needed (settings unchanged, delta=0)");
        }
    }
}

/// System to make the sky sphere follow the camera
///
/// IMPORTANT: This system is now DISABLED because it causes star rendering issues.
/// The shader calculates `dir = normalize(world_position)` which expects the sphere
/// to be at world origin. Moving the sphere causes incorrect star directions.
///
/// The sphere has radius 50000 and the camera is at ~5120, 100, -5120 (~7242 units
/// from origin), so the camera is well inside the sphere and this system is not needed.
///
/// NOTE: If the game world expands beyond radius 50000, re-enable this system and
/// fix the shader to calculate direction relative to sphere center.
pub fn sky_sphere_follow_camera_system(
    camera_query: Query<&GlobalTransform, With<Camera>>,
    sky_query: Query<&Transform, With<StarrySky>>,
) {
    // DIAGNOSTIC: Log once per second to verify sphere is at origin
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    
    if frame % 60 == 0 {
        if let Ok(camera_transform) = camera_query.get_single() {
            let camera_pos = camera_transform.translation();
            let camera_distance = camera_pos.length();
            
            for sky_transform in sky_query.iter() {
                let sphere_pos = sky_transform.translation;
                let sphere_radius = 50000.0;
                
                log::info!(
                    "[SKY SPHERE] Camera at {:?} (distance: {:.0} from origin), Sphere at {:?}, radius: {}",
                    camera_pos, camera_distance, sphere_pos, sphere_radius
                );
                
                if camera_distance > sphere_radius * 0.9 {
                    log::warn!(
                        "[SKY SPHERE] Camera is near sphere edge! Distance: {:.0}, Radius: {}",
                        camera_distance, sphere_radius
                    );
                }
            }
        }
    }
    
    // DISABLED: Moving the sphere breaks star rendering because the shader
    // uses normalize(world_position) which expects sphere at origin.
    // The sphere radius (50000) is large enough to contain the entire game world.
    //
    // for mut sky_transform in sky_query.iter_mut() {
    //     sky_transform.translation = camera_pos;
    // }
}

/// System to make the moon light follow the camera and point in the moon direction
pub fn moon_light_follow_camera_system(
    camera_query: Query<&GlobalTransform, With<Camera>>,
    mut moon_query: Query<&mut Transform, With<MoonLight>>,
    starry_sky_settings: Res<StarrySkySettings>,
) {
    // Get camera position
    if let Ok(camera_transform) = camera_query.get_single() {
        let camera_pos = camera_transform.translation();
        
        // Update moon light position to follow camera
        for mut moon_transform in moon_query.iter_mut() {
            // Position the moon light above and in the direction specified by settings
            let moon_dir = starry_sky_settings.moon_direction.normalize();
            let moon_distance = 500.0; // Distance from camera
            
            // Position moon light relative to camera
            let moon_pos = camera_pos + moon_dir * moon_distance;
            moon_transform.translation = moon_pos;
            
            // Make the light point toward the camera (down toward the scene)
            moon_transform.look_at(camera_pos, Vec3::Y);
        }
    }
}

/// System to update starry sky night_factor based on zone time state
/// This connects the ZoneTimeState to star visibility
///
/// Night factor values:
/// - Night = 1.0 (stars fully visible)
/// - Evening/Morning = 0.5 (transition, stars partially visible)
/// - Day = 0.0 (stars invisible)
pub fn update_starry_sky_night_factor(
    zone_time: Option<Res<crate::resources::ZoneTime>>,
    mut starry_sky_settings: ResMut<StarrySkySettings>,
) {
    use crate::resources::ZoneTimeState;

    // DEBUG OVERRIDE: Set to true to force night mode for testing stars
    // TODO: Set back to false after testing
    const FORCE_NIGHT_MODE: bool = false;

    // Frame counter for throttling logs
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = frame % 60 == 0; // Log every 60 frames

    // DEBUG: Force night mode for testing
    if FORCE_NIGHT_MODE {
        if should_log {
            log::warn!("[NIGHT_FACTOR_UPDATE] ========== DEBUG MODE: FORCING NIGHT ==========");
            log::warn!("[NIGHT_FACTOR_UPDATE] FORCE_NIGHT_MODE is enabled - stars should be visible!");
        }
        starry_sky_settings.night_factor = 1.0;
        return;
    }
    
    if should_log {
        log::info!("[NIGHT_FACTOR_UPDATE] ========== SYSTEM RUNNING ==========");
        log::info!("[NIGHT_FACTOR_UPDATE] Frame: {}", frame);
    }
    
    // Check if ZoneTime resource exists
    let Some(zone_time) = zone_time else {
        if should_log {
            log::error!("[NIGHT_FACTOR_UPDATE] ZoneTime resource DOES NOT EXIST!");
            log::error!("[NIGHT_FACTOR_UPDATE] This means zone_time_system hasn't run or hasn't inserted the resource.");
            log::error!("[NIGHT_FACTOR_UPDATE] Current night_factor in settings: {}", starry_sky_settings.night_factor);
        }
        return;
    };
    
    if should_log {
        log::info!("[NIGHT_FACTOR_UPDATE] ZoneTime resource EXISTS");
        log::info!("[NIGHT_FACTOR_UPDATE] ZoneTime details:");
        log::info!("[NIGHT_FACTOR_UPDATE]   state: {:?}", zone_time.state);
        log::info!("[NIGHT_FACTOR_UPDATE]   state_percent_complete: {:.2}", zone_time.state_percent_complete);
        log::info!("[NIGHT_FACTOR_UPDATE]   time: {:.2}", zone_time.time);
        log::info!("[NIGHT_FACTOR_UPDATE]   debug_overwrite_time: {:?}", zone_time.debug_overwrite_time);
        log::info!("[NIGHT_FACTOR_UPDATE]   is_changed: {}", zone_time.is_changed());
    }
    
    // Store old value for comparison
    let old_night_factor = starry_sky_settings.night_factor;
    
    // Calculate new night factor based on time state
    let new_night_factor = match zone_time.state {
        ZoneTimeState::Night => {
            if should_log {
                log::info!("[NIGHT_FACTOR_UPDATE] State is NIGHT -> night_factor = 1.0");
            }
            1.0
        }
        ZoneTimeState::Evening => {
            // Fade in during second half of evening
            if zone_time.state_percent_complete > 0.5 {
                let factor = (zone_time.state_percent_complete - 0.5) * 2.0;
                if should_log {
                    log::info!("[NIGHT_FACTOR_UPDATE] State is EVENING (2nd half, {:.2}%) -> night_factor = {:.2}",
                        zone_time.state_percent_complete * 100.0, factor);
                }
                factor
            } else {
                if should_log {
                    log::info!("[NIGHT_FACTOR_UPDATE] State is EVENING (1st half, {:.2}%) -> night_factor = 0.0",
                        zone_time.state_percent_complete * 100.0);
                }
                0.0
            }
        }
        ZoneTimeState::Morning => {
            // Fade out during first half of morning
            if zone_time.state_percent_complete < 0.5 {
                let factor = 1.0 - zone_time.state_percent_complete * 2.0;
                if should_log {
                    log::info!("[NIGHT_FACTOR_UPDATE] State is MORNING (1st half, {:.2}%) -> night_factor = {:.2}",
                        zone_time.state_percent_complete * 100.0, factor);
                }
                factor
            } else {
                if should_log {
                    log::info!("[NIGHT_FACTOR_UPDATE] State is MORNING (2nd half, {:.2}%) -> night_factor = 0.0",
                        zone_time.state_percent_complete * 100.0);
                }
                0.0
            }
        }
        ZoneTimeState::Day => {
            if should_log {
                log::info!("[NIGHT_FACTOR_UPDATE] State is DAY -> night_factor = 0.0");
            }
            0.0
        }
    };
    
    if should_log {
        log::info!("[NIGHT_FACTOR_UPDATE] Calculation result:");
        log::info!("[NIGHT_FACTOR_UPDATE]   old_night_factor: {:.2}", old_night_factor);
        log::info!("[NIGHT_FACTOR_UPDATE]   new_night_factor: {:.2}", new_night_factor);
        log::info!("[NIGHT_FACTOR_UPDATE]   values_different: {}", old_night_factor != new_night_factor);
    }
    
    // Only update if changed (avoids unnecessary change detection)
    if starry_sky_settings.night_factor != new_night_factor {
        starry_sky_settings.night_factor = new_night_factor;
        
        // Always log when value actually changes
        log::info!(
            "[NIGHT_FACTOR_UPDATE] UPDATED: night_factor {:.2} -> {:.2} (state: {:?}, progress: {:.2})",
            old_night_factor,
            new_night_factor,
            zone_time.state,
            zone_time.state_percent_complete
        );
    } else {
        if should_log {
            log::info!("[NIGHT_FACTOR_UPDATE] No change needed (value already {:.2})", new_night_factor);
        }
    }
    
    if should_log {
        log::info!("[NIGHT_FACTOR_UPDATE] ================================================");
    }
}

/// Resource to track whether the atmosphere should be enabled
/// This is used to toggle the Atmosphere component on the camera based on time of day
/// NOTE: Default is `enabled: true` because the camera is spawned WITH Atmosphere components.
/// This ensures the initial state matches the actual camera state.
#[derive(Resource, Debug)]
pub struct AtmosphereState {
    pub enabled: bool,
}

impl Default for AtmosphereState {
    fn default() -> Self {
        Self {
            enabled: true,  // Camera is spawned WITH Atmosphere, so default is true
        }
    }
}

/// System to toggle the Atmosphere component based on time of day
///
/// During Night time, the Atmosphere is removed to allow stars to be visible.
/// During Day/Evening/Morning, the Atmosphere is re-added for realistic sky rendering.
///
/// This system must run after zone_time_system to get the current time state.
    pub fn toggle_atmosphere_based_on_time(
    zone_time: Option<Res<crate::resources::ZoneTime>>,
    mut atmosphere_state: ResMut<AtmosphereState>,
        camera_query: Query<Entity, With<bevy::prelude::Camera3d>>,
    mut commands: Commands,
) {
    use crate::resources::ZoneTimeState;
    use bevy::pbr::{Atmosphere, AtmosphereSettings};

    // DEBUG OVERRIDE: Must match FORCE_NIGHT_MODE in update_starry_sky_night_factor
    // When forcing night mode, also force atmosphere OFF
    const FORCE_NIGHT_MODE: bool = false;

    // Frame counter for throttling diagnostic logs
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = frame % 60 == 0; // Log every 60 frames

    // DEBUG: Force atmosphere OFF when forcing night mode
    if FORCE_NIGHT_MODE {
        if atmosphere_state.enabled {
            if let Ok(camera_entity) = camera_query.get_single() {
                atmosphere_state.enabled = false;
                commands.entity(camera_entity).remove::<Atmosphere>();
                commands.entity(camera_entity).remove::<AtmosphereSettings>();
                log::warn!("[ATMOSPHERE] DEBUG: Forcing atmosphere OFF for night mode testing");
            }
        }
        return;
    }
    
    // Check if ZoneTime resource exists
    let Some(zone_time) = zone_time else {
        // ZoneTime doesn't exist yet - keep atmosphere ENABLED (default daytime sky)
        // This happens during loading screen before zone is fully loaded
        if should_log {
            log::warn!("[ATMOSPHERE] ZoneTime resource DOES NOT EXIST - keeping atmosphere ENABLED");
        }
        
        // Ensure atmosphere is enabled if it was disabled
        if !atmosphere_state.enabled {
            if let Ok(camera_entity) = camera_query.get_single() {
                atmosphere_state.enabled = true;
                commands.entity(camera_entity).insert((
                    Atmosphere::EARTH,
                    AtmosphereSettings::default(),
                ));
                log::info!("[ATMOSPHERE] Re-enabled atmosphere (ZoneTime was missing)");
            }
        }
        return;
    };
    
    // Determine if atmosphere should be enabled based on time state
    let should_enable_atmosphere = match zone_time.state {
        ZoneTimeState::Night => false,  // Disable atmosphere at night to show stars
        ZoneTimeState::Evening => true, // Enable atmosphere during evening transition
        ZoneTimeState::Morning => true, // Enable atmosphere during morning transition
        ZoneTimeState::Day => true,     // Enable atmosphere during day
    };
    
    // Diagnostic logging
    if should_log {
        log::info!("[ATMOSPHERE] ========== TOGGLE SYSTEM RUNNING ==========");
        log::info!("[ATMOSPHERE] Frame: {}", frame);
        log::info!("[ATMOSPHERE] ZoneTime state: {:?}", zone_time.state);
        log::info!("[ATMOSPHERE] ZoneTime progress: {:.2}%", zone_time.state_percent_complete * 100.0);
        log::info!("[ATMOSPHERE] Current atmosphere_state.enabled: {}", atmosphere_state.enabled);
        log::info!("[ATMOSPHERE] should_enable_atmosphere: {}", should_enable_atmosphere);
        log::info!("[ATMOSPHERE] Change needed: {}", atmosphere_state.enabled != should_enable_atmosphere);
        
        if let Ok(_camera_entity) = camera_query.get_single() {
            log::info!("[ATMOSPHERE] Camera entity found");
        } else {
            log::warn!("[ATMOSPHERE] No Camera3d entity found!");
        }
    }
    
    // Only make changes if state has changed
    if atmosphere_state.enabled != should_enable_atmosphere {
        let old_state = atmosphere_state.enabled;
        atmosphere_state.enabled = should_enable_atmosphere;
        
        // Find the camera entity and toggle atmosphere components
        if let Ok(camera_entity) = camera_query.get_single() {
            if should_enable_atmosphere {
                // Re-add atmosphere components
                commands.entity(camera_entity).insert((
                    Atmosphere::EARTH,
                    AtmosphereSettings::default(),
                ));
                log::info!(
                    "[ATMOSPHERE] ✓ ENABLED atmosphere: {:?} -> true (state: {:?}, progress: {:.2}%)",
                    old_state,
                    zone_time.state,
                    zone_time.state_percent_complete * 100.0
                );
            } else {
                // Remove atmosphere components to show stars
                commands.entity(camera_entity).remove::<Atmosphere>();
                commands.entity(camera_entity).remove::<AtmosphereSettings>();
                log::info!(
                    "[ATMOSPHERE] ✗ DISABLED atmosphere: {:?} -> false (state: {:?}, progress: {:.2}%)",
                    old_state,
                    zone_time.state,
                    zone_time.state_percent_complete * 100.0
                );
            }
        } else {
            log::warn!("[ATMOSPHERE] Cannot toggle - no Camera3d entity found!");
        }
    } else if should_log {
        log::info!("[ATMOSPHERE] No change needed (atmosphere already {})",
            if should_enable_atmosphere { "ENABLED" } else { "DISABLED" });
    }
    
    if should_log {
        log::info!("[ATMOSPHERE] ================================================");
    }
}
