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
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::*,
    reflect::TypePath,
    render::{alpha::AlphaMode, render_resource::*, renderer::RenderDevice},
};
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_shader::{Shader, ShaderRef};

/// Shader handle for the starry sky shader
pub const STARRY_SKY_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5e6f7a8b-9c0d-1e2f-3a4b-5c6d7e8f9a0b");

/// Plugin that registers the starry sky material
pub struct StarrySkyMaterialPlugin;

impl Plugin for StarrySkyMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            STARRY_SKY_SHADER_HANDLE,
            "shaders/starry_sky.wgsl",
            Shader::from_wgsl
        );

        // Register the material plugin for rendering
        // AlphaMode::Add will place this in Transparent3d phase which runs AFTER atmosphere
        // Note: prepass and shadows are controlled via enable_prepass() and enable_shadows() methods on Material trait
        app.add_plugins(MaterialPlugin::<StarrySkyMaterial>::default());

        // Insert default starry sky settings resource
        app.init_resource::<StarrySkySettings>();
    }
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
            star_density: 1.0,    // 50% of cells have stars (~3,000-5,000 stars)
            star_brightness: 5.0, // Normal brightness
            moon_phase: 0.5,      // Full moon
            moon_direction: Vec3::new(0.3, 0.8, 0.5).normalize(), // Upper right
            night_factor: 0.0, // Default to daytime (stars hidden) until zone_time_system updates it
        }
    }
}

/// Custom material for procedural starry sky rendering
/// Manual AsBindGroup implementation for Bevy 0.17 compatibility
#[derive(Asset, TypePath, Clone, Debug)]
pub struct StarrySkyMaterial {
    /// Current game time for twinkling animation
    pub time: f32,

    /// Star density setting
    pub star_density: f32,

    /// Star brightness multiplier
    pub star_brightness: f32,

    /// Night visibility factor (0.0 = day, 1.0 = night)
    pub night_factor: f32,

    /// Moon phase (0.0 to 1.0)
    pub moon_phase: f32,

    /// Moon direction in world space (padding to vec4)
    pub moon_direction: Vec3,
}

/// Data type for StarrySkyMaterial bind group
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StarrySkyMaterialKey;

impl AsBindGroup for StarrySkyMaterial {
    type Data = StarrySkyMaterialKey;
    type Param = ();

    fn label() -> &'static str {
        "starry_sky_material"
    }

    fn bind_group_data(&self) -> Self::Data {
        StarrySkyMaterialKey
    }

    fn as_bind_group(
        &self,
        layout_descriptor: &BindGroupLayoutDescriptor,
        render_device: &RenderDevice,
        pipeline_cache: &PipelineCache,
        _param: &mut (),
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        // Get the actual bind group layout from the pipeline cache
        let layout = pipeline_cache.get_bind_group_layout(layout_descriptor);

        // Create uniform buffer with all material data
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("starry_sky_material_uniforms"),
            contents: bytemuck::cast_slice(&[
                self.time,
                self.star_density,
                self.star_brightness,
                self.night_factor,
                self.moon_phase,
                self.moon_direction.x,
                self.moon_direction.y,
                self.moon_direction.z,
            ]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let entries = vec![BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }];

        let bind_group = render_device.create_bind_group("starry_sky_material", &layout, &entries);

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
        // We override as_bind_group, so this should never be called
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
        _force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        vec![
            // Uniform buffer for all starry sky material data
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]
    }
}

impl Default for StarrySkyMaterial {
    fn default() -> Self {
        Self {
            time: 0.0,
            star_density: 0.50, // Match StarrySkySettings default
            star_brightness: 1.0,
            night_factor: 0.0, // Default to daytime (stars hidden)
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

    /// Disable prepass for sky
    fn enable_prepass() -> bool {
        false
    }

    /// Sky doesn't cast shadows
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set up vertex buffer layout - we only need positions for a sky sphere
        let vertex_layout = layout
            .0
            .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        // Configure blending for standard alpha blending (Solution 2 for ghosting fix)
        // Standard alpha blending prevents color accumulation that causes ghosting
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha, // Standard alpha blending
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha, // Standard alpha blending
                        operation: BlendOperation::Add,
                    },
                });
            }
        }

        // CRITICAL: Disable depth writes and use GreaterEqual comparison for sky
        // GreaterEqual means "render where sky depth >= depth buffer" (closer to camera)
        // With reverse-z depth buffer, this prevents sky from bleeding through geometry
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = false;
            // Only render sky where no opaque objects are in front
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
        }

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
    use bevy_mesh::{Indices, VertexAttributeValues};

    // Create a large sphere (inverted for sky rendering)
    let sphere = Sphere::new(500.0);
    let mut mesh = Mesh::from(sphere);

    // Increase subdivision for better star field resolution
    // Note: Bevy 0.16 uses Sphere primitive which has default subdivisions
    // For a sky sphere we need high detail

    // Flip normals for inside rendering
    if let Some(normals) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        if let VertexAttributeValues::Float32x3(normals) = normals {
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
            Indices::U32(indices) => {
                // Reverse each triangle (swap v1 and v2 of each triangle)
                for chunk in indices.chunks_mut(3) {
                    chunk.swap(1, 2);
                }
            }
            Indices::U16(indices) => {
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

    if entity_count == 0 {
        return;
    }

    if starry_sky_settings.is_changed() || time.delta_secs() > 0.0 {
        for material_handle in query.iter() {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.time = time.elapsed_secs();
                material.star_density = starry_sky_settings.star_density;
                material.star_brightness = starry_sky_settings.star_brightness;
                material.night_factor = starry_sky_settings.night_factor;
                material.moon_phase = starry_sky_settings.moon_phase;
                material.moon_direction = starry_sky_settings.moon_direction;
            }
        }
    }
}

/// System to make the moon light follow the camera and point in the moon direction
pub fn moon_light_follow_camera_system(
    camera_query: Query<&GlobalTransform, With<Camera>>,
    mut moon_query: Query<&mut Transform, With<MoonLight>>,
    starry_sky_settings: Res<StarrySkySettings>,
) {
    // Get camera position
    if let Ok(camera_transform) = camera_query.single() {
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

    // Check if ZoneTime resource exists
    let Some(zone_time) = zone_time else {
        return;
    };

    // Calculate new night factor based on time state
    let new_night_factor = match zone_time.state {
        ZoneTimeState::Night => 1.0,
        ZoneTimeState::Evening => {
            // Fade in during second half of evening
            if zone_time.state_percent_complete > 0.5 {
                (zone_time.state_percent_complete - 0.5) * 2.0
            } else {
                0.0
            }
        }
        ZoneTimeState::Morning => {
            // Fade out during first half of morning
            if zone_time.state_percent_complete < 0.5 {
                1.0 - zone_time.state_percent_complete * 2.0
            } else {
                0.0
            }
        }
        ZoneTimeState::Day => 0.0,
    };

    // Only update if changed (avoids unnecessary change detection)
    if starry_sky_settings.night_factor != new_night_factor {
        starry_sky_settings.night_factor = new_night_factor;
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
            enabled: true, // Camera is spawned WITH Atmosphere, so default is true
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
    mut scattering_mediums: ResMut<Assets<bevy::pbr::ScatteringMedium>>,
) {
    use crate::resources::ZoneTimeState;
    use bevy::pbr::{Atmosphere, AtmosphereSettings};

    // Check if ZoneTime resource exists
    let Some(zone_time) = zone_time else {
        // ZoneTime doesn't exist yet - keep atmosphere ENABLED (default daytime sky)
        // This happens during loading screen before zone is fully loaded

        // Ensure atmosphere is enabled if it was disabled
        if !atmosphere_state.enabled {
            if let Ok(camera_entity) = camera_query.single() {
                atmosphere_state.enabled = true;
                commands.entity(camera_entity).insert((
                    Atmosphere::earthlike(
                        scattering_mediums.add(bevy::pbr::ScatteringMedium::default()),
                    ),
                    AtmosphereSettings::default(),
                ));
            }
        }
        return;
    };

    // Determine if atmosphere should be enabled based on time state
    let should_enable_atmosphere = match zone_time.state {
        ZoneTimeState::Night => false, // Disable atmosphere at night to show stars
        ZoneTimeState::Evening => true, // Enable atmosphere during evening transition
        ZoneTimeState::Morning => true, // Enable atmosphere during morning transition
        ZoneTimeState::Day => true,    // Enable atmosphere during day
    };

    // Only make changes if state has changed
    if atmosphere_state.enabled != should_enable_atmosphere {
        atmosphere_state.enabled = should_enable_atmosphere;

        // Find the camera entity and toggle atmosphere components
        if let Ok(camera_entity) = camera_query.single() {
            if should_enable_atmosphere {
                // Re-add atmosphere components
                commands.entity(camera_entity).insert((
                    Atmosphere::earthlike(
                        scattering_mediums.add(bevy::pbr::ScatteringMedium::default()),
                    ),
                    AtmosphereSettings::default(),
                ));
            } else {
                // Remove atmosphere components to show stars
                commands.entity(camera_entity).remove::<Atmosphere>();
                commands
                    .entity(camera_entity)
                    .remove::<AtmosphereSettings>();
            }
        }
    }
}
