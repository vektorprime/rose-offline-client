//! Procedural Cloud Material for Bevy 0.16
//!
//! This module implements a custom material that renders:
//! - Procedural clouds using fBm noise
//! - Time-based wind animation
//! - Time-of-day lighting integration
//! - Adjustable coverage, density, and appearance
//!
//! RENDER ORDER: The clouds use AlphaMode::Blend which places them in the
//! Transparent3d render phase. They render after opaque objects but before
//! the starry sky.

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
    },
};

/// Shader handle for the cloud shader
pub const CLOUD_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("b2c3d4e5-f6a7-8901-bcde-f23456789012");

/// Plugin that registers the cloud material
pub struct CloudMaterialPlugin;

impl Plugin for CloudMaterialPlugin {
    fn build(&self, app: &mut App) {
        log::info!("[CLOUD PLUGIN] ========== PLUGIN BUILD START ==========");
        
        load_internal_asset!(
            app,
            CLOUD_SHADER_HANDLE,
            "shaders/cloud.wgsl",
            Shader::from_wgsl
        );
        log::info!("[CLOUD PLUGIN] Internal shader asset loaded: {:?}", CLOUD_SHADER_HANDLE);

        // Register the material plugin for rendering
        app.add_plugins(MaterialPlugin::<CloudMaterial> {
            prepass_enabled: false,
            shadows_enabled: false,
            ..Default::default()
        });
        log::info!("[CLOUD PLUGIN] MaterialPlugin<CloudMaterial> registered");

        // Insert default cloud settings resource
        app.init_resource::<CloudSettings>();
        //log::info!("[CLOUD PLUGIN] CloudSettings resource initialized");

        // Add cloud update systems
        app.add_systems(Update, (
            update_cloud_material_system,
            update_cloud_lighting_system,
            diagnose_cloud_layer_system,
        ).chain());
        //log::info!("[CLOUD PLUGIN] Update systems added");

        // Add cloud layer follow camera system in PostUpdate (after transform propagation)
        app.add_systems(
            bevy::app::PostUpdate,
            cloud_layer_follow_camera_system.after(bevy::transform::TransformSystem::TransformPropagate),
        );
        //log::info!("[CLOUD PLUGIN] Cloud follow camera system added");

        //log::info!("[CLOUD PLUGIN] ========== PLUGIN BUILD COMPLETE ==========");
    }
}

/// Resource for cloud settings that can be modified at runtime.
/// These control the appearance and behavior of procedural clouds.
#[derive(Resource, Reflect, Clone, Debug)]
pub struct CloudSettings {
    // === Enable/Disable ===
    /// Master toggle for cloud rendering
    pub enabled: bool,
    
    // === Cloud Coverage ===
    /// Cloud density/coverage (0.0 = clear sky, 1.0 = overcast)
    /// Controls the threshold for cloud formation in noise function
    pub density: f32,
    
    /// Cloud coverage multiplier (affects horizontal extent)
    /// Higher values create more widespread cloud layers
    pub coverage: f32,
    
    // === Animation ===
    /// Wind speed - controls how fast clouds move (units per second)
    pub speed: f32,
    
    /// Wind direction in radians (0 = +X, PI/2 = +Z)
    pub wind_direction: f32,
    
    // === Appearance ===
    /// Cloud brightness multiplier (0.0 = dark, 1.0 = normal, 2.0 = bright)
    pub brightness: f32,
    
    /// Cloud opacity/alpha (0.0 = invisible, 1.0 = solid)
    pub opacity: f32,
    
    /// Cloud softness/feathering at edges (0.0 = hard, 1.0 = soft)
    pub softness: f32,
    
    // === Geometry ===
    /// Cloud layer altitude (world units from ground)
    pub altitude: f32,
    
    // === Quality ===
    /// Number of noise octaves (1-6, higher = more detail but slower)
    pub noise_octaves: u32,
    
    /// Noise scale multiplier (higher = smaller cloud features)
    pub noise_scale: f32,
    
    // === Time-of-Day Response ===
    /// How much clouds respond to time-of-day lighting (0.0 = static, 1.0 = full response)
    pub tod_response: f32,
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self {
            // Master toggle
            enabled: true,
            
            // Coverage - moderate clouds by default
            density: 0.5,
            coverage: 0.6,
            
            // Animation - gentle wind
            speed: 5.0,
            wind_direction: 0.0,  // +X direction
            
            // Appearance - natural look
            brightness: 1.0,
            opacity: 0.8,
            softness: 0.3,
            
            // Geometry - high cloud layer
            altitude: 400.0,
            
            // Quality - balanced
            noise_octaves: 4,
            noise_scale: 1.0,
            
            // Time-of-day
            tod_response: 1.0,
        }
    }
}

/// Custom material for procedural cloud rendering
/// Uses AsBindGroup derive for automatic bind group creation
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CloudMaterial {
    // === Time and Animation ===
    #[uniform(0)]
    pub time: f32,
    
    #[uniform(1)]
    pub speed: f32,
    
    #[uniform(2)]
    pub wind_direction: Vec3,
    
    // === Cloud Shape ===
    #[uniform(3)]
    pub density: f32,
    
    #[uniform(4)]
    pub coverage: f32,
    
    #[uniform(5)]
    pub noise_scale: f32,
    
    #[uniform(6)]
    pub noise_octaves: f32,
    
    // === Appearance ===
    #[uniform(7)]
    pub brightness: f32,
    
    #[uniform(8)]
    pub opacity: f32,
    
    #[uniform(9)]
    pub softness: f32,
    
    // === Lighting ===
    #[uniform(10)]
    pub sun_direction: Vec3,
    
    #[uniform(11)]
    pub sun_color: Vec3,
    
    #[uniform(12)]
    pub ambient_color: Vec3,
    
    #[uniform(13)]
    pub tod_factor: f32,
}

impl Default for CloudMaterial {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 5.0,
            wind_direction: Vec3::X,
            density: 0.5,
            coverage: 0.6,
            noise_scale: 1.0,
            noise_octaves: 4.0,
            brightness: 1.0,
            opacity: 0.8,
            softness: 0.3,
            sun_direction: Vec3::new(0.5, 0.8, 0.3).normalize(),
            sun_color: Vec3::new(1.0, 0.95, 0.9),
            ambient_color: Vec3::new(0.4, 0.45, 0.5),
            tod_factor: 1.0,
        }
    }
}

impl Material for CloudMaterial {
    fn vertex_shader() -> ShaderRef {
        CLOUD_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        CLOUD_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend  // Standard alpha blending for clouds
    }

    fn depth_bias(&self) -> f32 {
        0.5  // Slight bias to render behind close objects but in front of sky
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
        log::info!("[CLOUD SPECIALIZE] Specializing pipeline for CloudMaterial");
        
        // Set up vertex buffer layout - we only need positions for a cloud plane
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        log::info!("[CLOUD SPECIALIZE] Vertex buffer layout configured");

        // Disable backface culling - camera can be under or above cloud layer
        descriptor.primitive.cull_mode = None;
        log::info!("[CLOUD SPECIALIZE] Backface culling DISABLED");

        // Configure alpha blending for soft clouds (standard alpha blending)
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                });
            }
            log::info!("[CLOUD SPECIALIZE] Blend state configured for standard alpha rendering");
        }

        // Depth settings - render clouds where no opaque objects block
        // Use GreaterEqual for reversed-Z (Bevy 0.14+) so clouds render in front of distant objects (sky)
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = false;
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
            log::info!("[CLOUD SPECIALIZE] Depth writes DISABLED, depth_compare = GreaterEqual");
        }

        log::info!("[CLOUD SPECIALIZE] Pipeline specialization complete");
        Ok(())
    }
}

/// Component marker for the cloud plane entity
#[derive(Component, Default)]
pub struct CloudLayer;

/// System to update cloud material from settings
pub fn update_cloud_material_system(
    time: Res<Time>,
    cloud_settings: Res<CloudSettings>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    query: Query<(&MeshMaterial3d<CloudMaterial>, &ViewVisibility, Option<&InheritedVisibility>, Entity), With<CloudLayer>>,
) {
    // Frame counter for throttling logs
    //static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    //let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = false; // Disabled: frame % 60 == 0; // Log every 60 frames
    
    if should_log {
        log::info!("[CLOUD UPDATE] ========== UPDATE SYSTEM RUNNING ==========");
        log::info!("[CLOUD UPDATE] CloudSettings.enabled: {}", cloud_settings.enabled);
        log::info!("[CLOUD UPDATE] CloudLayer entities: {}", query.iter().count());
    }
    
    if !cloud_settings.enabled {
        return;
    }
    
    let mut updated_count = 0;
    for (material_handle, view_visibility, inherited_visibility, entity) in query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            // Update time
            material.time = time.elapsed_secs();
            
            // Update from settings
            material.speed = cloud_settings.speed;
            material.density = cloud_settings.density;
            material.coverage = cloud_settings.coverage;
            material.noise_scale = cloud_settings.noise_scale;
            material.noise_octaves = cloud_settings.noise_octaves as f32;
            material.brightness = cloud_settings.brightness;
            material.opacity = cloud_settings.opacity;
            material.softness = cloud_settings.softness;
            
            // Calculate wind direction vector
            let wind_rad = cloud_settings.wind_direction;
            material.wind_direction = Vec3::new(
                wind_rad.cos(),
                0.0,
                wind_rad.sin(),
            ).normalize();
            
            updated_count += 1;
        } else {
            if should_log {
                log::warn!("[CLOUD UPDATE] Material handle {:?} not found in assets!", material_handle.0);
            }
        }
    }
    
    if should_log {
        log::info!("[CLOUD UPDATE] Updated {} material(s)", updated_count);
        
        // Log visibility status
        for (_, view_visibility, inherited_visibility, entity) in query.iter() {
            let view_vis = view_visibility.get();
            let inherited_vis = inherited_visibility.map(|i| i.get());
            log::info!("[CLOUD UPDATE] Entity {:?} view_visibility={}, inherited_visibility={:?}", entity, view_vis, inherited_vis);
        }
        
        log::info!("[CLOUD UPDATE] ================================================");
    }
}

/// System to update cloud lighting based on time of day
pub fn update_cloud_lighting_system(
    zone_time: Option<Res<crate::resources::ZoneTime>>,
    zone_lighting: Res<crate::render::ZoneLighting>,
    cloud_settings: Res<CloudSettings>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    query: Query<&MeshMaterial3d<CloudMaterial>, With<CloudLayer>>,
) {
    let Some(zone_time) = zone_time else {
        return;
    };
    
    if !cloud_settings.enabled || cloud_settings.tod_response <= 0.0 {
        return;
    }
    
    // Frame counter for throttling logs
    //static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    //let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = false; // Disabled: frame % 60 == 0; // Log every 60 frames
    
    // Calculate sun direction and color based on time of day
    let (sun_direction, sun_color, ambient_color, tod_factor) = 
        calculate_cloud_lighting(&zone_time, &zone_lighting);
    
    if should_log {
        log::info!("[CLOUD LIGHTING] ========== LIGHTING UPDATE ==========");
        log::info!("[CLOUD LIGHTING] ZoneTime state: {:?}", zone_time.state);
        log::info!("[CLOUD LIGHTING] Sun direction: {:?}", sun_direction);
        log::info!("[CLOUD LIGHTING] Sun color: {:?}", sun_color);
        log::info!("[CLOUD LIGHTING] TOD factor: {}", tod_factor);
    }
    
    let mut updated_count = 0;
    for material_handle in query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.sun_direction = sun_direction;
            material.sun_color = sun_color * cloud_settings.tod_response;
            material.ambient_color = ambient_color;
            material.tod_factor = tod_factor;
            updated_count += 1;
        }
    }
    
    if should_log {
        log::info!("[CLOUD LIGHTING] Updated {} material(s)", updated_count);
        log::info!("[CLOUD LIGHTING] ================================================");
    }
}

/// Calculate cloud lighting parameters based on time of day
fn calculate_cloud_lighting(
    zone_time: &crate::resources::ZoneTime,
    zone_lighting: &crate::render::ZoneLighting,
) -> (Vec3, Vec3, Vec3, f32) {
    use crate::resources::ZoneTimeState;
    
    // Sun direction varies with time of day
    // Morning: East (low angle), Noon: Up, Evening: West (low angle), Night: Below horizon
    let time_of_day = match zone_time.state {
        ZoneTimeState::Morning => {
            // Sun rises in the east, moves upward
            let t = zone_time.state_percent_complete;
            0.0 + t * 0.5  // 0.0 to 0.5 (sunrise to noon approach)
        }
        ZoneTimeState::Day => {
            // Sun at highest point, slowly descending
            let t = zone_time.state_percent_complete;
            0.5 + t * 0.25  // 0.5 to 0.75 (noon to afternoon)
        }
        ZoneTimeState::Evening => {
            // Sun sets in the west
            let t = zone_time.state_percent_complete;
            0.75 + t * 0.25  // 0.75 to 1.0 (sunset)
        }
        ZoneTimeState::Night => {
            // Sun below horizon
            0.0
        }
    };
    
    // Calculate sun direction from time
    let sun_angle = time_of_day * std::f32::consts::PI;
    let sun_direction = Vec3::new(
        -sun_angle.cos(),  // X: east-west
        sun_angle.sin(),   // Y: up-down
        0.3,               // Z: slight northward tilt
    ).normalize();
    
    // Sun color varies with time of day
    let sun_color = match zone_time.state {
        ZoneTimeState::Morning => {
            // Warm orange/pink sunrise
            let t = zone_time.state_percent_complete;
            Vec3::new(1.0, 0.7 + t * 0.2, 0.5 + t * 0.4)  // Orange -> whiter
        }
        ZoneTimeState::Day => {
            // Bright white/yellow daylight
            Vec3::new(1.0, 0.98, 0.95)
        }
        ZoneTimeState::Evening => {
            // Warm orange/red sunset
            let t = zone_time.state_percent_complete;
            Vec3::new(1.0, 0.9 - t * 0.4, 0.8 - t * 0.5)  // White -> orange/red
        }
        ZoneTimeState::Night => {
            // Dim moonlight
            Vec3::new(0.2, 0.25, 0.4)
        }
    };
    
    // Ambient color from zone lighting
    let ambient_color = zone_lighting.map_ambient_color;
    
    // Time-of-day factor for cloud visibility
    let tod_factor = match zone_time.state {
        ZoneTimeState::Morning => 0.5 + zone_time.state_percent_complete * 0.5,
        ZoneTimeState::Day => 1.0,
        ZoneTimeState::Evening => 1.0 - zone_time.state_percent_complete * 0.5,
        ZoneTimeState::Night => 0.3,  // Clouds still slightly visible at night
    };
    
    (sun_direction, sun_color, ambient_color, tod_factor)
}

/// Create a cloud plane mesh
pub fn create_cloud_plane_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    // Create a large flat plane for the cloud layer
    use bevy::math::primitives::Plane3d;
    
    // Create plane with default UP normal. With backface culling disabled in specialize(),
    // the plane will be visible from both sides (above and below).
    // Camera looks UP at clouds from below, so this works correctly.
    let plane = Plane3d::new(Vec3::Y, bevy::math::Vec2::splat(50000.0));
    let mesh = Mesh::from(plane);
    
    log::info!("[CLOUD MESH] Created cloud plane mesh with size 100000x100000");
    
    meshes.add(mesh)
}

/// System to spawn cloud layer when entering a zone
pub fn spawn_cloud_layer(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    cloud_settings: Res<CloudSettings>,
    existing_clouds: Query<Entity, With<CloudLayer>>,
) {
    // Despawn existing cloud layer
    for entity in existing_clouds.iter() {
        commands.entity(entity).despawn();
    }
    
    if !cloud_settings.enabled {
        log::info!("[CLOUD] Clouds disabled, not spawning layer");
        return;
    }
    
    // Create cloud material
    let cloud_material = CloudMaterial::default();
    let material_handle = materials.add(cloud_material);
    
    // Create cloud plane mesh
    let mesh_handle = create_cloud_plane_mesh(&mut meshes);
    
    // Spawn cloud layer entity
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, cloud_settings.altitude, 0.0),
        CloudLayer,
        Name::new("CloudLayer"),
        Visibility::Visible,
        bevy::render::view::NoFrustumCulling,  // Prevent frustum culling of large cloud plane
    ));
    
    log::info!("[CLOUD] Spawned cloud layer at altitude {}", cloud_settings.altitude);
}

/// System to make cloud layer follow camera horizontally
/// This ensures clouds are always visible regardless of camera position
pub fn cloud_layer_follow_camera_system(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    cloud_settings: Res<CloudSettings>,
    mut cloud_query: Query<&mut Transform, With<CloudLayer>>,
) {
    // Get camera position
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    
    let camera_pos = camera_transform.translation();
    
    // Frame counter for throttling logs
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = frame % 60 == 0; // Log every 60 frames
    
    // Update cloud layer position to follow camera horizontally
    for mut transform in cloud_query.iter_mut() {
        transform.translation.x = camera_pos.x;
        transform.translation.z = camera_pos.z;
        transform.translation.y = cloud_settings.altitude;
        
        if should_log {
            log::info!("[CLOUD FOLLOW] Camera pos: {:?}, Cloud pos: {:?}, Altitude: {}",
                camera_pos, transform.translation, cloud_settings.altitude);
        }
    }
}

/// Diagnostic system to check cloud layer visibility and render queue status
pub fn diagnose_cloud_layer_system(
    materials: Res<Assets<CloudMaterial>>,
    query: Query<(&MeshMaterial3d<CloudMaterial>, &ViewVisibility, Option<&InheritedVisibility>, Entity), With<CloudLayer>>,
) {
    // Frame counter for throttling logs - DISABLED
    //static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    //let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    //let should_log = frame % 60 == 0; // Log every 60 frames
    
    // Always return early to disable all CLOUD DIAGNOSE logs
    return;
    /*
    
    log::info!("[CLOUD DIAGNOSE] ========== CLOUD LAYER DIAGNOSTIC ==========");
    
    let entity_count = query.iter().count();
    log::info!("[CLOUD DIAGNOSE] CloudLayer entities: {}", entity_count);
    
    if entity_count == 0 {
        log::warn!("[CLOUD DIAGNOSE] No CloudLayer entities found!");
        return;
    }
    
    for (material_handle, view_visibility, inherited_visibility, entity) in query.iter() {
        log::info!("[CLOUD DIAGNOSE] Entity {:?}:", entity);
        log::info!("[CLOUD DIAGNOSE]   ViewVisibility: {}", view_visibility.get());
        
        if let Some(inherited) = inherited_visibility {
            log::info!("[CLOUD DIAGNOSE]   InheritedVisibility: {}", inherited.get());
        }
        
        if let Some(material) = materials.get(&material_handle.0) {
            log::info!("[CLOUD DIAGNOSE]   Material values:");
            log::info!("[CLOUD DIAGNOSE]     time: {:.2}", material.time);
            log::info!("[CLOUD DIAGNOSE]     opacity: {:.2}", material.opacity);
            log::info!("[CLOUD DIAGNOSE]     tod_factor: {:.2}", material.tod_factor);
        } else {
            log::warn!("[CLOUD DIAGNOSE]   Material NOT FOUND in assets!");
        }
    }
    
    log::info!("[CLOUD DIAGNOSE] ================================================");
    */
}
