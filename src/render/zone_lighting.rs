use bevy::{
    asset::{load_internal_asset, UntypedHandle, UntypedAssetId, Handle, weak_handle},
    ecs::{
        component::Component,
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::{Vec3, Vec4},
    pbr::{CascadeShadowConfig, FogVolume, VolumetricLight},
    prelude::{
        AmbientLight, App, Color, Commands, DetectChanges, DirectionalLight, EulerRot,
        FromWorld, IntoScheduleConfigs, Local, Plugin, Query, Quat, ReflectResource, Res, ResMut,
        Resource, Shader, Startup, Transform, Update, World, With,
    },
    reflect::{Reflect, TypePath},
    render::camera::Exposure,
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            encase, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, ShaderSize, ShaderStages,
            ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
        view::RenderLayers,
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
};

/// Marker component for the volumetric fog volume entity.
/// Used to query the fog volume for modifications or removal.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct VolumetricFogVolume;
use std::any::TypeId;
use std::sync::OnceLock;
use uuid::Uuid;

/// Mode for controlling how the time of day is determined.
#[derive(Reflect, Clone, Copy, PartialEq, Debug, Default)]
pub enum SkyMode {
    /// Time of day follows the game's ZoneTime resource automatically
    #[default]
    Automatic,
    /// Time of day is manually controlled by the user via SkySettings.manual_time
    Manual,
}

/// Resource for controlling sky and time-of-day settings.
/// Allows players to manually set the time or let it follow game time automatically.
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct SkySettings {
    /// Whether time is automatic (follows game time) or manual (user-controlled)
    pub mode: SkyMode,
    /// Manual time value in hours (0-24) when mode is Manual
    pub manual_time: f32,
    /// Multiplier for atmosphere scattering intensity (0.0-2.0)
    /// Values > 1.0 make the sky more dramatic, < 1.0 makes it more subtle
    pub atmosphere_intensity: f32,
}

impl Default for SkySettings {
    fn default() -> Self {
        Self {
            mode: SkyMode::Automatic,
            manual_time: 12.0, // Default to noon
            atmosphere_intensity: 1.0,
        }
    }
}

/// Global storage for the zone lighting bind group layout.
/// This allows the specialize method to access the layout without needing direct resource access.
pub static ZONE_LIGHTING_BIND_GROUP_LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

pub const ZONE_LIGHTING_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0x444949d32b35d5d9) });

pub const ZONE_LIGHTING_SHADER_HANDLE_TYPED: Handle<Shader> =
    weak_handle!("444949d3-2b35-d5d9-0000-000000000000");

fn default_light_transform() -> Transform {
    Transform::from_rotation(Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        std::f32::consts::PI * (2.0 / 3.0),
        -std::f32::consts::PI / 4.0,
    ))
}

#[derive(Default)]
pub struct ZoneLightingPlugin;

impl Plugin for ZoneLightingPlugin {
    fn build(&self, app: &mut App) {
        // bevy::log::info!("[ZONE LIGHTING] Building ZoneLightingPlugin");
        
        load_internal_asset!(
            app,
            ZONE_LIGHTING_SHADER_HANDLE_TYPED,
            "shaders/zone_lighting.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ZoneLighting>()
            .register_type::<SkySettings>()
            .register_type::<SkyMode>()
            .init_resource::<ZoneLighting>()
            .init_resource::<SkySettings>();

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            // bevy::log::info!("[ZONE LIGHTING] Initializing render app systems");
            render_app
                .add_systems(ExtractSchedule, extract_uniform_data)
                .add_systems(Render, (prepare_uniform_data,).in_set(RenderSet::Prepare));
        } else {
            bevy::log::error!("[ZONE LIGHTING] FAILED to get render app - lighting will not work!");
        }

        app.add_systems(Startup, spawn_lights)
            .add_systems(Update, (update_volumetric_fog_system, update_sun_position_system));
        // bevy::log::info!("[ZONE LIGHTING] ZoneLightingPlugin build complete");
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<ZoneLightingUniformMeta>();
    }
}

fn spawn_lights(mut commands: Commands, zone_lighting: Res<ZoneLighting>) {
    // bevy::log::info!("[ZONE LIGHTING] Spawning directional and ambient lights");

    // Bevy 0.14: Use individual components instead of DirectionalLightBundle
    // IMPORTANT: shadows_enabled MUST be true for VolumetricLight to work
    let light_entity = commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,  // Reduced for balanced PBR lighting (was 50000.0 too bright)
            shadows_enabled: true,  // REQUIRED for volumetric lighting
            ..Default::default()
        },
        default_light_transform(),
        CascadeShadowConfig {
            bounds: vec![50.0, 150.0, 500.0, 2000.0],  // Tighter bounds for better shadow quality at game scale
            overlap_proportion: 0.3,  // More overlap for smoother cascade transitions
            minimum_distance: 0.1,
        },
        RenderLayers::default(),
        VolumetricLight,  // Enable volumetric light shafts for this directional light
    )).id();

    // bevy::log::info!("[ZONE LIGHTING] Directional light spawned: entity={:?}, illuminance=15000.0, shadows_enabled=true, VolumetricLight component added", light_entity);

    // Bevy 0.14: AmbientLight is now a component that can be spawned as an entity
    // or kept as a resource. Using as resource for global ambient light.
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0),  // Slightly cool ambient for better atmosphere
        brightness: 350.0,  // Increased to provide fill light on shadowed character faces (was 150.0, originally 500.0)
        affects_lightmapped_meshes: true,
    });
    
    //bevy::log::info!("[ZONE LIGHTING] Ambient light inserted: brightness=1.0");

    // Spawn the volumetric fog volume that covers the entire world
    // This enables light shafts/god rays from the directional light
    // Use initial values from ZoneLighting resource
    let density_factor = if zone_lighting.volumetric_fog_enabled {
        zone_lighting.volumetric_density_factor
    } else {
        0.0
    };
    
    // Use volumetric_fog_color from ZoneLighting for time-of-day integration
    let fog_color = Color::srgb(
        zone_lighting.volumetric_fog_color.x,
        zone_lighting.volumetric_fog_color.y,
        zone_lighting.volumetric_fog_color.z,
    );
    
    // CRITICAL FIX: Position the fog volume at the center of the game world (5120, 0, -5120)
    // The game world is centered around these coordinates, NOT at origin (0,0,0).
    // If the fog volume is at origin, the camera is ~7200 units away and sees a black box.
    // Scale of 2000.0 means the volume spans from (4120, -1000, -6120) to (6120, 1000, -4120)
    let fog_volume_center = Vec3::new(5120.0, 0.0, -5120.0);
    let fog_volume_scale = 2000.0;
    
    commands.spawn((
        FogVolume {
            fog_color,
            density_factor,
            absorption: zone_lighting.volumetric_absorption,
            scattering: zone_lighting.volumetric_scattering,
            scattering_asymmetry: zone_lighting.volumetric_scattering_asymmetry,
            ..Default::default()
        },
        Transform::from_translation(fog_volume_center).with_scale(Vec3::splat(fog_volume_scale)),
        VolumetricFogVolume,  // Marker component for querying
    ));
    
    // bevy::log::info!(
    //     "[ZONE LIGHTING] Volumetric fog volume spawned at center=({}), scale={}, density_factor={}",
    //     fog_volume_center, fog_volume_scale, density_factor
    // );
    // bevy::log::info!(
    //     "[ZONE LIGHTING] Fog volume bounds: ({}) to ({})",
    //     fog_volume_center - Vec3::splat(fog_volume_scale / 2.0),
    //     fog_volume_center + Vec3::splat(fog_volume_scale / 2.0)
    // );
}

/// System that updates the FogVolume component from ZoneLighting resource settings.
/// This allows runtime control of volumetric fog parameters through the ZoneLighting resource.
/// Uses change detection to only update when ZoneLighting has been modified.
fn update_volumetric_fog_system(
    zone_lighting: Res<ZoneLighting>,
    mut fog_volume_query: Query<&mut FogVolume, With<VolumetricFogVolume>>,
) {
    // Only proceed if ZoneLighting has changed (change detection)
    if !zone_lighting.is_changed() {
        return;
    }
    
    // bevy::log::debug!(
    //     "[ZONE LIGHTING] ZoneLighting changed - updating fog volumes (enabled={}, density={}, absorption={}, scattering={})",
    //     zone_lighting.volumetric_fog_enabled,
    //     zone_lighting.volumetric_density_factor,
    //     zone_lighting.volumetric_absorption,
    //     zone_lighting.volumetric_scattering
    // );
    
    // Update all fog volumes marked with VolumetricFogVolume
    for mut fog_volume in fog_volume_query.iter_mut() {
        if zone_lighting.volumetric_fog_enabled {
            fog_volume.fog_color = Color::srgb(
                zone_lighting.volumetric_fog_color.x,
                zone_lighting.volumetric_fog_color.y,
                zone_lighting.volumetric_fog_color.z,
            );
            fog_volume.density_factor = zone_lighting.volumetric_density_factor;
            fog_volume.absorption = zone_lighting.volumetric_absorption;
            fog_volume.scattering = zone_lighting.volumetric_scattering;
            fog_volume.scattering_asymmetry = zone_lighting.volumetric_scattering_asymmetry;
            
            // bevy::log::debug!(
            //     "[ZONE LIGHTING] FogVolume updated: density_factor={}, absorption={}, scattering={}",
            //     fog_volume.density_factor, fog_volume.absorption, fog_volume.scattering
            // );
        } else {
            // When disabled, set density to 0 to effectively disable the fog
            fog_volume.density_factor = 0.0;
            // bevy::log::debug!("[ZONE LIGHTING] Volumetric fog disabled - set density_factor to 0");
        }
    }
}

/// System that updates the directional light rotation based on SkySettings and ZoneTime.
/// This creates a dynamic day/night cycle where the sun position changes with time.
/// The sun rotates around the scene based on the time of day (0-24 hours).
///
/// When SkySettings.mode is Automatic, the sun follows the game's ZoneTime.
/// When SkySettings.mode is Manual, the sun position is controlled by SkySettings.manual_time.
///
/// Sun path:
/// - Sunrise (~6:00): Sun at horizon in the East
/// - Noon (~12:00): Sun directly overhead
/// - Sunset (~18:00): Sun at horizon in the West
/// - Night (~21:00-5:00): Sun below horizon
fn update_sun_position_system(
    zone_time: Res<crate::resources::ZoneTime>,
    sky_settings: Res<SkySettings>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    // Determine if we should update based on mode and what changed
    let should_update = match sky_settings.mode {
        SkyMode::Automatic => zone_time.is_changed() || sky_settings.is_changed(),
        SkyMode::Manual => sky_settings.is_changed(),
    };
    
    if !should_update {
        return;
    }
    
    // Get the time value based on mode
    let time_hours = match sky_settings.mode {
        SkyMode::Automatic => {
            // Use game time from ZoneTime
            zone_time.time as f32
        }
        SkyMode::Manual => {
            // Use manual time from SkySettings
            sky_settings.manual_time
        }
    };
    
    for mut transform in query.iter_mut() {
        // Normalize time to 0-24 hours range
        let normalized_time = time_hours % 24.0;
        
        // Convert time to a fraction of the day (0.0 to 1.0)
        let day_fract = (normalized_time / 24.0).clamp(0.0, 1.0);
        
        // Earth's axial tilt - this creates the arc path of the sun
        // Higher values make the sun rise higher at noon
        let earth_tilt_rad = std::f32::consts::PI / 3.0; // 60 degrees
        
        // Create rotation that moves the sun in an arc from east to west
        // Using ZYX euler angles:
        // - Z (earth_tilt_rad): Tilts the rotation axis to create the arc path
        // - Y (0.0): No Y rotation needed
        // - X (-day_fract * TAU): Rotates the sun around the tilted axis over the day
        //
        // At day_fract = 0.0 (midnight): sun is at lowest point (below horizon)
        // At day_fract = 0.25 (6am): sun is at horizon (sunrise in east)
        // At day_fract = 0.5 (noon): sun is at highest point (overhead)
        // At day_fract = 0.75 (6pm): sun is at horizon (sunset in west)
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            earth_tilt_rad,
            0.0,
            -day_fract * std::f32::consts::TAU,
        );
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ZoneLighting {
    pub map_ambient_color: Vec3,
    pub character_ambient_color: Vec3,
    pub character_diffuse_color: Vec3,
    pub light_direction: Vec3,

    pub color_fog_enabled: bool,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    pub alpha_fog_enabled: bool,
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
    // Height-based fog parameters
    pub fog_min_height: f32,
    pub fog_max_height: f32,
    pub fog_height_density: f32,
    // Time of day parameters
    pub time_of_day: f32,
    pub day_color: Vec3,
    pub night_color: Vec3,
    // Volumetric fog settings
    pub volumetric_fog_enabled: bool,
    pub volumetric_fog_color: Vec3,
    pub volumetric_density_factor: f32,
    pub volumetric_absorption: f32,
    pub volumetric_scattering: f32,
    pub volumetric_scattering_asymmetry: f32,
}

impl Default for ZoneLighting {
    fn default() -> Self {
        Self {
            map_ambient_color: Vec3::ONE,
            character_ambient_color: Vec3::ONE,
            character_diffuse_color: Vec3::ONE,
            light_direction: default_light_transform().back().normalize(),
            fog_color: Vec3::new(0.2, 0.2, 0.2),
            color_fog_enabled: true,
            fog_density: 0.0018,
            fog_min_density: 0.0,
            fog_max_density: 0.75,
            alpha_fog_enabled: true,
            fog_alpha_weight_start: 0.85,
            fog_alpha_weight_end: 0.98,
            // Height-based fog parameters
            fog_min_height: -10.0,
            fog_max_height: 50.0,
            fog_height_density: 0.5,
            // Time of day parameters
            time_of_day: 0.5, // 0.0 = night, 1.0 = day
            day_color: Vec3::new(0.7, 0.8, 1.0), // Day fog color (blueish)
            night_color: Vec3::new(0.1, 0.1, 0.3), // Night fog color (dark blue)
            // Volumetric fog settings - tuned for atmospheric depth and light shafts
            volumetric_fog_enabled: false,  // Disabled by default - can be enabled in settings
            volumetric_fog_color: Vec3::new(0.85, 0.9, 1.0), // Soft blue-white for atmospheric haze
            volumetric_density_factor: 0.05,  // Balanced density for visible light shafts without obscuring gameplay
            volumetric_absorption: 0.1,  // Moderate absorption for depth perception
            volumetric_scattering: 0.11,  // Scattering coefficient for balanced light shafts (was 0.5 too high)
            volumetric_scattering_asymmetry: 0.7,  // Higher asymmetry for forward-scattering (Mie scattering)
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct ZoneLightingUniformData {
    // Group 0: 64 bytes (4 vec4)
    pub map_ambient_color: Vec4,
    pub character_ambient_color: Vec4,
    pub character_diffuse_color: Vec4,
    pub light_direction: Vec4,

    // Group 1: 64 bytes (4 vec4)
    pub fog_color: Vec4,
    pub day_color: Vec4,
    pub night_color: Vec4,
    // Pack 4 f32 values into vec4 for alignment: fog_density, fog_min_density, fog_max_density, fog_height_density
    pub fog_params: Vec4,
    
    // Group 2: 48 bytes (3 vec4)
    // Pack 4 f32 values into vec4 for alignment: fog_min_height, fog_max_height, time_of_day, unused
    pub fog_height_params: Vec4,
    // Pack 2 f32 values with padding: fog_alpha_range_start, fog_alpha_range_end, unused, unused
    pub fog_alpha_params: Vec4,
    pub _padding: Vec4, // Padding to ensure total size is multiple of 16
}

#[derive(Resource)]
pub struct ZoneLightingUniformMeta {
    buffer: Buffer,
    bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ZoneLightingUniformMeta {
    fn from_world(world: &mut World) -> Self {
        //bevy::log::info!("[ZONE LIGHTING] Creating ZoneLightingUniformMeta render resources");
        
        let render_device = world.resource::<RenderDevice>();

        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: ZoneLightingUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("zone_lighting_uniform_buffer"),
        });
        //bevy::log::info!("[ZONE LIGHTING] Uniform buffer created: size={} bytes",
            //ZoneLightingUniformData::min_size().get());

        let bind_group_layout =
            render_device.create_bind_group_layout(
                Some("zone_lighting_uniform_layout"),
                &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ZoneLightingUniformData::min_size()),
                    },
                    count: None,
                }],
            );
        //bevy::log::info!("[ZONE LIGHTING] Bind group layout created");

        let bind_group = render_device.create_bind_group(
            "zone_lighting_uniform_bind_group",
            &bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        );
        //bevy::log::info!("[ZONE LIGHTING] Bind group created - ZoneLightingUniformMeta ready");

        // Store the bind group layout in the global static for access during pipeline specialization
        let _ = ZONE_LIGHTING_BIND_GROUP_LAYOUT.set(bind_group_layout.clone());

        ZoneLightingUniformMeta {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

fn extract_uniform_data(
    mut commands: Commands,
    zone_lighting: Extract<Res<ZoneLighting>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // // Log every 60 frames to avoid spam
    // if *frame_count % 60 == 1 {
    //     bevy::log::info!("[ZONE LIGHTING] Extracting uniform data (frame {})", *frame_count);
    //     bevy::log::info!("[ZONE LIGHTING]   Map ambient: {:?}", zone_lighting.map_ambient_color);
    //     bevy::log::info!("[ZONE LIGHTING]   Light direction: {:?}", zone_lighting.light_direction);
    //     bevy::log::info!("[ZONE LIGHTING]   Fog enabled: {}, density: {}",
    //         zone_lighting.color_fog_enabled, zone_lighting.fog_density);
    // }
    
    commands.insert_resource(ZoneLightingUniformData {
        map_ambient_color: zone_lighting.map_ambient_color.extend(1.0),
        character_ambient_color: zone_lighting.character_ambient_color.extend(1.0),
        character_diffuse_color: zone_lighting.character_diffuse_color.extend(1.0),
        light_direction: zone_lighting.light_direction.extend(1.0),
        fog_color: zone_lighting.fog_color.extend(1.0),
        day_color: zone_lighting.day_color.extend(1.0),
        night_color: zone_lighting.night_color.extend(1.0),
        // Pack fog params: fog_density, fog_min_density, fog_max_density, fog_height_density
        fog_params: Vec4::new(
            if zone_lighting.color_fog_enabled { zone_lighting.fog_density } else { 0.0 },
            if zone_lighting.color_fog_enabled { zone_lighting.fog_min_density } else { 0.0 },
            if zone_lighting.color_fog_enabled { zone_lighting.fog_max_density } else { 0.0 },
            zone_lighting.fog_height_density,
        ),
        // Pack fog height params: fog_min_height, fog_max_height, time_of_day, unused
        fog_height_params: Vec4::new(
            zone_lighting.fog_min_height,
            zone_lighting.fog_max_height,
            zone_lighting.time_of_day,
            0.0, // unused
        ),
        // Pack fog alpha params: fog_alpha_range_start, fog_alpha_range_end, unused, unused
        fog_alpha_params: Vec4::new(
            if zone_lighting.alpha_fog_enabled { zone_lighting.fog_alpha_weight_start } else { 99999999999.0 },
            if zone_lighting.alpha_fog_enabled { zone_lighting.fog_alpha_weight_end } else { 999999999.0 },
            0.0, // unused
            0.0, // unused
        ),
        _padding: Vec4::ZERO,
    });
}

fn prepare_uniform_data(
    uniform_data: Res<ZoneLightingUniformData>,
    uniform_meta: ResMut<ZoneLightingUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; ZoneLightingUniformData::SHADER_SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&uniform_meta.buffer, 0, buffer.as_ref());
}

pub struct SetZoneLightingBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetZoneLightingBindGroup<I> {
    type Param = SRes<ZoneLightingUniformMeta>;
    type ItemQuery = ();
    type ViewQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &meta.into_inner().bind_group, &[]);

        RenderCommandResult::Success
    }
}
