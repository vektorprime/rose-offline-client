//! 3D Volumetric Cloud System for Bevy 0.18.1
//!
//! Creates fluffy cumulus-style 3D clouds that:
//! - Float at fixed world positions (not skybox attached)
//! - Have full 3D dimensions visible from all angles
//! - Use volumetric rendering with noise-based density
//! - Support time-of-day lighting integration

use bevy::{
    asset::{load_internal_asset, weak_handle, Handle},
    math::{Quat, Vec3},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{alpha::AlphaMode, render_resource::*, renderer::RenderDevice},
};
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_shader::{Shader, ShaderRef};

pub const VOLUMETRIC_CLOUD_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c3d4e5f6-a7b8-9012-cdef-345678901234");

// ROSE world/map center and full-map half extent in world units.
// Used to ensure volumetric clouds spawn from map center and spread across the map.
const MAP_CENTER_X: f32 = 5120.0;
const MAP_CENTER_Z: f32 = -5120.0;
const MAP_HALF_EXTENT: f32 = 5120.0;

pub struct VolumetricCloudPlugin;

impl Plugin for VolumetricCloudPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            VOLUMETRIC_CLOUD_SHADER_HANDLE,
            "shaders/volumetric_cloud.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(MaterialPlugin::<VolumetricCloudMaterial>::default());
        app.init_resource::<VolumetricCloudSettings>();

        app.add_systems(
            Update,
            (
                sync_volumetric_cloud_structure_system,
                update_volumetric_cloud_material_system,
                update_volumetric_cloud_lighting_system,
            )
                .chain(),
        );

        app.add_systems(OnEnter(crate::AppState::Game), spawn_volumetric_clouds);
        app.add_systems(OnExit(crate::AppState::Game), despawn_volumetric_clouds);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VolumetricCloudStructuralSettings {
    enabled: bool,
    cloud_count: usize,
    cluster_size_min: usize,
    cluster_size_max: usize,
    cloud_radius_min: f32,
    cloud_radius_max: f32,
    cloud_height_min: f32,
    cloud_height_max: f32,
    cloud_spawn_radius: f32,
}

impl From<&VolumetricCloudSettings> for VolumetricCloudStructuralSettings {
    fn from(settings: &VolumetricCloudSettings) -> Self {
        Self {
            enabled: settings.enabled,
            cloud_count: settings.cloud_count,
            cluster_size_min: settings.cluster_size_min,
            cluster_size_max: settings.cluster_size_max,
            cloud_radius_min: settings.cloud_radius_min,
            cloud_radius_max: settings.cloud_radius_max,
            cloud_height_min: settings.cloud_height_min,
            cloud_height_max: settings.cloud_height_max,
            cloud_spawn_radius: settings.cloud_spawn_radius,
        }
    }
}

#[derive(Resource, Reflect, Clone, Debug)]
pub struct VolumetricCloudSettings {
    pub enabled: bool,
    pub cloud_count: usize,
    pub cluster_size_min: usize,
    pub cluster_size_max: usize,
    pub cloud_radius_min: f32,
    pub cloud_radius_max: f32,
    pub cloud_height_min: f32,
    pub cloud_height_max: f32,
    pub cloud_spawn_radius: f32,
    pub density: f32,
    pub opacity: f32,
    pub brightness: f32,
    pub drift_speed: Vec3,
    pub noise_scale: f32,
    pub noise_octaves: u32,
    pub tod_response: f32,
}

impl Default for VolumetricCloudSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            cloud_count: 120,
            cluster_size_min: 2,
            cluster_size_max: 5,
            cloud_radius_min: 180.0,
            cloud_radius_max: 380.0,
            cloud_height_min: 300.0,
            cloud_height_max: 700.0,
            // Default to full-map coverage from center.
            cloud_spawn_radius: MAP_HALF_EXTENT,
            density: 0.95,
            opacity: 1.0,
            brightness: 2.9,
            drift_speed: Vec3::new(15.0, 0.0, 8.0),
            noise_scale: 0.01,
            noise_octaves: 4,
            tod_response: 0.2,
        }
    }
}

#[derive(Asset, TypePath, Clone, Debug)]
pub struct VolumetricCloudMaterial {
    pub time: f32,
    pub density: f32,
    pub opacity: f32,
    pub brightness: f32,
    pub noise_scale: f32,
    pub noise_octaves: f32,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub ambient_color: Vec3,
    pub tod_factor: f32,
    pub drift_speed: Vec3,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VolumetricCloudMaterialKey;

impl AsBindGroup for VolumetricCloudMaterial {
    type Data = VolumetricCloudMaterialKey;
    type Param = ();

    fn label() -> &'static str {
        "volumetric_cloud_material"
    }

    fn bind_group_data(&self) -> Self::Data {
        VolumetricCloudMaterialKey
    }

    fn as_bind_group(
        &self,
        layout_descriptor: &BindGroupLayoutDescriptor,
        render_device: &RenderDevice,
        _pipeline_cache: &PipelineCache,
        _param: &mut (),
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("volumetric_cloud_uniforms"),
            contents: bytemuck::cast_slice(&[
                self.time,
                self.density,
                self.opacity,
                self.brightness,
                self.noise_scale,
                self.noise_octaves,
                0.0,
                0.0,
                self.sun_direction.x,
                self.sun_direction.y,
                self.sun_direction.z,
                0.0,
                self.sun_color.x,
                self.sun_color.y,
                self.sun_color.z,
                0.0,
                self.ambient_color.x,
                self.ambient_color.y,
                self.ambient_color.z,
                self.tod_factor,
                self.drift_speed.x,
                self.drift_speed.y,
                self.drift_speed.z,
                0.0,
            ]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let layout = _pipeline_cache.get_bind_group_layout(layout_descriptor);

        let entries = vec![BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }];

        let bind_group =
            render_device.create_bind_group("volumetric_cloud_material", &layout, &entries);

        Ok(PreparedBindGroup {
            bindings: BindingResources(vec![]),
            bind_group,
        })
    }

    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _param: &mut (),
        _bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
        _force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        vec![BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }]
    }
}

impl Default for VolumetricCloudMaterial {
    fn default() -> Self {
        Self {
            time: 0.0,
            density: 0.5,
            opacity: 0.7,
            brightness: 1.0,
            noise_scale: 0.02,
            noise_octaves: 4.0,
            sun_direction: Vec3::new(0.5, 0.8, 0.3).normalize(),
            sun_color: Vec3::new(1.0, 0.95, 0.9),
            ambient_color: Vec3::new(0.4, 0.45, 0.5),
            tod_factor: 1.0,
            drift_speed: Vec3::ZERO,
        }
    }
}

impl Material for VolumetricCloudMaterial {
    fn vertex_shader() -> ShaderRef {
        VOLUMETRIC_CLOUD_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        VOLUMETRIC_CLOUD_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Keep cloud pixels solid (non-transparent) while still allowing shader discard
        // to carve out fluffy volume silhouettes.
        AlphaMode::Opaque
    }

    fn depth_bias(&self) -> f32 {
        0.0
    }

    fn reads_view_transmission_texture(&self) -> bool {
        false
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout
            .0
            .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = None;
            }
        }

        descriptor.primitive.cull_mode = None;

        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = true;
            // Bevy 0.18 uses reverse-Z in 3D, so depth compare must be GreaterEqual.
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
        }

        Ok(())
    }
}

#[derive(Component, Default)]
pub struct VolumetricCloud;

pub fn spawn_volumetric_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VolumetricCloudMaterial>>,
    cloud_settings: Res<VolumetricCloudSettings>,
    existing_clouds: Query<Entity, With<VolumetricCloud>>,
) {
    for entity in existing_clouds.iter() {
        commands.entity(entity).despawn();
    }

    if !cloud_settings.enabled {
        log::warn!("[VOLUMETRIC CLOUDS] Clouds are disabled in settings");
        return;
    }

    log::info!("[VOLUMETRIC CLOUDS] Starting to spawn clouds...");
    log::info!(
        "[VOLUMETRIC CLOUDS] Settings: count={}, cluster_size={}-{}, radius={}-{}, height={}-{}, spawn_radius={}",
        cloud_settings.cloud_count,
        cloud_settings.cluster_size_min,
        cloud_settings.cluster_size_max,
        cloud_settings.cloud_radius_min,
        cloud_settings.cloud_radius_max,
        cloud_settings.cloud_height_min,
        cloud_settings.cloud_height_max,
        cloud_settings.cloud_spawn_radius
    );

    // Spawn around map center and ensure spread covers the whole map area.
    let center_x = MAP_CENTER_X;
    let center_z = MAP_CENTER_Z;
    let spawn_radius = cloud_settings.cloud_spawn_radius.max(MAP_HALF_EXTENT);
    // Keep cloud band at midpoint of configured min/max range.
    let spawn_height = cloud_settings.cloud_height_min
        + (cloud_settings.cloud_height_max - cloud_settings.cloud_height_min) * 0.5;

    log::info!(
        "[VOLUMETRIC CLOUDS] Spawn distribution center=({}, {}), radius={}, height={}",
        center_x,
        center_z,
        spawn_radius,
        spawn_height
    );

    let cloud_material = VolumetricCloudMaterial {
        density: cloud_settings.density,
        opacity: cloud_settings.opacity,
        brightness: cloud_settings.brightness,
        noise_scale: cloud_settings.noise_scale,
        noise_octaves: cloud_settings.noise_octaves as f32,
        drift_speed: cloud_settings.drift_speed,
        ..Default::default()
    };
    let material_handle = materials.add(cloud_material);
    log::info!(
        "[VOLUMETRIC CLOUDS] Created material handle: {:?}",
        material_handle
    );

    let sphere_mesh = meshes.add(Sphere::new(1.0).mesh());
    log::info!("[VOLUMETRIC CLOUDS] Created sphere mesh");

    let cluster_size_min = cloud_settings.cluster_size_min.max(2);
    let cluster_size_max = cloud_settings.cluster_size_max.max(cluster_size_min);

    let mut spawned_count = 0usize;
    let mut cluster_index = 0usize;
    while spawned_count < cloud_settings.cloud_count {
        let remaining = cloud_settings.cloud_count - spawned_count;
        let cluster_span = cluster_size_max - cluster_size_min + 1;
        let mut cluster_size = cluster_size_min + (rand::random::<usize>() % cluster_span);

        if remaining <= cluster_size_min {
            // Keep the final cluster a group (>=2). This may overshoot by at most 1
            // when only one blob remains, but guarantees no isolated single-cloud blob.
            cluster_size = remaining.max(2);
        } else if cluster_size > remaining {
            cluster_size = remaining;
        }

        let cluster_center_x = center_x - spawn_radius + rand::random::<f32>() * (2.0 * spawn_radius);
        let cluster_center_z = center_z - spawn_radius + rand::random::<f32>() * (2.0 * spawn_radius);
        let cluster_center_y = spawn_height + (rand::random::<f32>() - 0.5) * 120.0;
        let cluster_spread = cloud_settings.cloud_radius_max * (0.55 + rand::random::<f32>() * 0.95);

        for _ in 0..cluster_size {
            let allow_overshoot = remaining == 1;
            if spawned_count >= cloud_settings.cloud_count && !allow_overshoot {
                break;
            }

            let radius = cloud_settings.cloud_radius_min
                + rand::random::<f32>()
                    * (cloud_settings.cloud_radius_max - cloud_settings.cloud_radius_min);

            // Per-blob non-uniform scaling yields less uniform silhouettes.
            let width_scale = 1.1 + rand::random::<f32>() * 1.4;
            let height_scale = 0.45 + rand::random::<f32>() * 0.55;
            let depth_scale = 1.0 + rand::random::<f32>() * 1.3;

            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let distance = rand::random::<f32>() * cluster_spread;
            let x = cluster_center_x + angle.cos() * distance;
            let z = cluster_center_z + angle.sin() * distance;
            let y = cluster_center_y + (rand::random::<f32>() - 0.5) * radius * 0.35;

            let scale = Vec3::new(
                radius * width_scale,
                radius * height_scale,
                radius * depth_scale,
            );

            // Subtle orientation variation further breaks up repeated silhouettes.
            let yaw = rand::random::<f32>() * std::f32::consts::TAU;
            let pitch = (rand::random::<f32>() - 0.5) * 0.25;
            let roll = (rand::random::<f32>() - 0.5) * 0.18;
            let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

            let entity_index = spawned_count;
            commands.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(material_handle.clone()),
                Transform::from_xyz(x, y, z)
                    .with_rotation(rotation)
                    .with_scale(scale),
                VolumetricCloud,
                Name::new(format!("VolumetricCloud_{}", entity_index)),
                Visibility::Visible,
                bevy::camera::visibility::NoFrustumCulling,
            ));

            spawned_count += 1;
            if entity_index < 3 {
                log::info!(
                    "[VOLUMETRIC CLOUDS] Cloud {}: cluster={}, pos=({}, {}, {}), radius={}, scale=({}, {}, {})",
                    entity_index,
                    cluster_index,
                    x,
                    y,
                    z,
                    radius,
                    scale.x,
                    scale.y,
                    scale.z
                );
            }
        }

        cluster_index += 1;
    }

    log::info!(
        "[VOLUMETRIC CLOUDS] Successfully spawned {} cloud blobs across {} clusters",
        spawned_count,
        cluster_index
    );
}

pub fn despawn_volumetric_clouds(
    mut commands: Commands,
    clouds: Query<Entity, With<VolumetricCloud>>,
) {
    for entity in clouds.iter() {
        commands.entity(entity).despawn();
    }
}

/// Applies structural cloud setting changes instantly by respawning cloud instances
/// when topology/layout fields change (count, size ranges, height ranges, spawn radius, enabled).
///
/// Material-only fields (density, opacity, brightness, noise, drift, TOD response)
/// are handled live by update systems and do not require respawn.
pub fn sync_volumetric_cloud_structure_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VolumetricCloudMaterial>>,
    cloud_settings: Res<VolumetricCloudSettings>,
    existing_clouds: Query<Entity, With<VolumetricCloud>>,
    mut previous_structural_settings: Local<Option<VolumetricCloudStructuralSettings>>,
) {
    let current_structural_settings = VolumetricCloudStructuralSettings::from(&*cloud_settings);

    let Some(previous) = *previous_structural_settings else {
        *previous_structural_settings = Some(current_structural_settings);
        return;
    };

    if !cloud_settings.is_changed() {
        return;
    }

    if previous != current_structural_settings {
        log::info!(
            "[VOLUMETRIC CLOUDS] Structural settings changed, respawning cloud instances immediately"
        );
        spawn_volumetric_clouds(
            commands,
            meshes,
            materials,
            cloud_settings,
            existing_clouds,
        );
    }

    *previous_structural_settings = Some(current_structural_settings);
}

pub fn update_volumetric_cloud_material_system(
    time: Res<Time>,
    cloud_settings: Res<VolumetricCloudSettings>,
    mut materials: ResMut<Assets<VolumetricCloudMaterial>>,
    query: Query<&MeshMaterial3d<VolumetricCloudMaterial>, With<VolumetricCloud>>,
) {
    if !cloud_settings.enabled {
        return;
    }

    for material_handle in query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.time = time.elapsed_secs();
            material.density = cloud_settings.density;
            material.opacity = cloud_settings.opacity;
            material.brightness = cloud_settings.brightness;
            material.noise_scale = cloud_settings.noise_scale;
            material.noise_octaves = cloud_settings.noise_octaves as f32;
            material.drift_speed = cloud_settings.drift_speed;
        }
    }
}

pub fn update_volumetric_cloud_lighting_system(
    zone_time: Option<Res<crate::resources::ZoneTime>>,
    zone_lighting: Res<crate::render::ZoneLighting>,
    cloud_settings: Res<VolumetricCloudSettings>,
    mut materials: ResMut<Assets<VolumetricCloudMaterial>>,
    query: Query<&MeshMaterial3d<VolumetricCloudMaterial>, With<VolumetricCloud>>,
) {
    let Some(zone_time) = zone_time else {
        return;
    };

    if !cloud_settings.enabled || cloud_settings.tod_response <= 0.0 {
        return;
    }

    let (sun_direction, sun_color, ambient_color, tod_factor) =
        calculate_cloud_lighting(&zone_time, &zone_lighting);

    for material_handle in query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.sun_direction = sun_direction;
            material.sun_color = sun_color * cloud_settings.tod_response;
            material.ambient_color = ambient_color;
            material.tod_factor = tod_factor;
        }
    }
}

fn calculate_cloud_lighting(
    zone_time: &crate::resources::ZoneTime,
    zone_lighting: &crate::render::ZoneLighting,
) -> (Vec3, Vec3, Vec3, f32) {
    use crate::resources::ZoneTimeState;

    let time_of_day = match zone_time.state {
        ZoneTimeState::Morning => {
            let t = zone_time.state_percent_complete;
            0.0 + t * 0.5
        }
        ZoneTimeState::Day => {
            let t = zone_time.state_percent_complete;
            0.5 + t * 0.25
        }
        ZoneTimeState::Evening => {
            let t = zone_time.state_percent_complete;
            0.75 + t * 0.25
        }
        ZoneTimeState::Night => 0.0,
    };

    let sun_angle = time_of_day * std::f32::consts::PI;
    let sun_direction = Vec3::new(-sun_angle.cos(), sun_angle.sin(), 0.3).normalize();

    let sun_color = match zone_time.state {
        ZoneTimeState::Morning => {
            let t = zone_time.state_percent_complete;
            Vec3::new(1.0, 0.7 + t * 0.2, 0.5 + t * 0.4)
        }
        ZoneTimeState::Day => Vec3::new(1.0, 0.98, 0.95),
        ZoneTimeState::Evening => {
            let t = zone_time.state_percent_complete;
            Vec3::new(1.0, 0.9 - t * 0.4, 0.8 - t * 0.5)
        }
        ZoneTimeState::Night => Vec3::new(0.2, 0.25, 0.4),
    };

    let ambient_color = zone_lighting.map_ambient_color;

    let tod_factor = match zone_time.state {
        ZoneTimeState::Morning => 0.5 + zone_time.state_percent_complete * 0.5,
        ZoneTimeState::Day => 1.0,
        ZoneTimeState::Evening => 1.0 - zone_time.state_percent_complete * 0.5,
        ZoneTimeState::Night => 0.3,
    };

    (sun_direction, sun_color, ambient_color, tod_factor)
}
