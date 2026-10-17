//! Cel ("toon") shading for the ROSE object and effect materials and the terrain.
//!
//! - `toon_bands.wgsl`: `ToonParams` and the band shape (terminator, mid band, hard
//!   shadow edge, shade floor), shared with `terrain_material.wgsl`.
//! - `toon_lighting.wgsl`: `toon_lighting()`, used instead of Bevy's
//!   `apply_pbr_lighting` by `rose_object_extension.wgsl` and `rose_effect_mesh.wgsl`.
//!
//! The parameters live in one read-only storage buffer ([`TOON_PARAMS_BUFFER_HANDLE`])
//! bound by every ROSE object material (binding 109), effect material (binding 103) and
//! terrain material (binding 3). [`CelShadingSettings`] (Settings > Cel Shading) is
//! written into it by [`apply_cel_shading_settings_system`]. The buffer never changes
//! size, so Bevy rewrites the existing GPU buffer in place (`GpuShaderBuffer::
//! prepare_asset`) and the materials' bind groups stay valid: slider changes show up
//! on the next frame without re-preparing any material.
//!
//! The outlines are a separate screen-space pass, see [`crate::render::cel_outline`];
//! the same system keeps the main camera's [`CelOutline`] in sync with the settings.

use bevy::{
    asset::{weak_handle, Assets},
    color::{Color, ColorToComponents},
    math::Vec4,
    prelude::{
        App, Camera3d, Commands, DetectChanges, Entity, Handle, Plugin, PostUpdate,
        PreStartup, Query, Res, ResMut, Resource, Shader, With, Without,
    },
    render::{render_resource::ShaderType, storage::ShaderBuffer},
};

use crate::render::{CelOutline, WaterReflectionCamera};

pub const TOON_BANDS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c3e1a7d2-5b4f-4e8a-0000-000000000000");

pub const TOON_LIGHTING_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c3e1a7d2-5b4f-4e8a-0000-000000000001");

/// The shared [`ToonParams`] buffer. Inserted at startup and never removed: a material
/// binding it is not prepared (not drawn) until the buffer exists.
pub const TOON_PARAMS_BUFFER_HANDLE: Handle<ShaderBuffer> =
    weak_handle!("c3e1a7d2-5b4f-4e8a-0000-000000000003");

/// GPU layout of the cel shading parameters. Matches `ToonParams` in `toon_bands.wgsl`.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct ToonParams {
    /// x = terminator N.L, y = full-light N.L, z = mid band level, w = band edge softness
    pub bands: Vec4,
    /// x = key light kept in shade, y = ambient kept in shade,
    /// z = shadow threshold, w = shadow edge softness
    pub shade: Vec4,
    /// x = rim strength, y = specular strength, z = 1 cel / 0 PBR, w = unused
    pub extras: Vec4,
}

/// Runtime cel shading settings (Settings > Cel Shading).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct CelShadingSettings {
    /// Cel lighting on models and terrain; off = Bevy PBR on models, smooth terrain.
    pub toon_lighting: bool,
    /// N.L where the key light starts (the terminator between shade and mid band).
    pub terminator: f32,
    /// N.L where the key light is fully on (the edge between mid and lit band).
    pub full_light: f32,
    /// Brightness of the middle band, 0 = as dark as the shade, 1 = as bright as lit.
    pub mid_band_level: f32,
    /// Half width of the band edges in N.L units (0 = razor sharp).
    pub band_softness: f32,
    /// Share of the sun / moon light kept in the shade band (and in cast shadows).
    pub shade_key_light: f32,
    /// Share of the ambient light kept in the shade band. Below 1 keeps the bands
    /// visible when the key light is weak (night).
    pub shade_ambient_light: f32,
    /// Half width of the cast-shadow edge (0 = razor sharp).
    pub shadow_softness: f32,
    /// Rim light on the lit side of silhouettes.
    pub rim_strength: f32,
    /// Hard highlight on materials with a ROSE specular map.
    pub specular_strength: f32,

    /// Screen-space outlines on the main camera.
    pub outlines: bool,
    /// Line width in pixels at 1080p (scaled with the resolution).
    pub outline_thickness: f32,
    pub outline_color: Color,
    pub outline_opacity: f32,
    /// Silhouette threshold: relative depth jump that counts as an edge. Lower = more lines.
    pub outline_depth_threshold: f32,
    /// Crease angle in degrees: sharper corners than this get a line.
    pub outline_crease_angle: f32,
    /// Depth threshold multiplier for alpha-masked parts (foliage). Higher = fewer lines.
    pub outline_foliage_scale: f32,
    /// Distance (m) where lines start fading out, and where they are gone.
    pub outline_fade_start: f32,
    pub outline_fade_end: f32,
}

impl Default for CelShadingSettings {
    fn default() -> Self {
        Self {
            toon_lighting: true,
            terminator: 0.0,
            full_light: 0.45,
            mid_band_level: 0.55,
            band_softness: 0.02,
            // The first version kept no key light in shade and the full ambient,
            // which left the shade side of cliffs nearly black by day.
            shade_key_light: 0.2,
            shade_ambient_light: 0.75,
            shadow_softness: 0.08,
            rim_strength: 0.3,
            specular_strength: 0.6,

            outlines: true,
            // 1 px at 1080p, tuned in-game: 2.5 filled small distant models black.
            outline_thickness: 1.0,
            outline_color: Color::BLACK,
            outline_opacity: 1.0,
            outline_depth_threshold: 0.04,
            outline_crease_angle: 45.0,
            outline_foliage_scale: 5.0,
            outline_fade_start: 80.0,
            outline_fade_end: 300.0,
        }
    }
}

impl CelShadingSettings {
    pub fn toon_params(&self) -> ToonParams {
        ToonParams {
            bands: Vec4::new(
                self.terminator,
                self.full_light.max(self.terminator + 0.01),
                self.mid_band_level,
                self.band_softness.max(0.0005),
            ),
            shade: Vec4::new(
                self.shade_key_light,
                self.shade_ambient_light,
                0.5,
                self.shadow_softness.max(0.0005),
            ),
            extras: Vec4::new(
                self.rim_strength,
                self.specular_strength,
                if self.toon_lighting { 1.0 } else { 0.0 },
                0.0,
            ),
        }
    }

    /// The camera component for these settings; `None` when outlines are off.
    pub fn outline(&self) -> Option<CelOutline> {
        if !self.outlines {
            return None;
        }
        let color = self.outline_color.to_linear().to_vec3();
        Some(CelOutline {
            color: color.extend(self.outline_opacity),
            thickness: self.outline_thickness,
            depth_threshold: self.outline_depth_threshold,
            normal_threshold: 1.0 - self.outline_crease_angle.to_radians().cos(),
            masked_depth_scale: self.outline_foliage_scale,
            fade_start: self.outline_fade_start,
            fade_end: self.outline_fade_end.max(self.outline_fade_start + 1.0),
            ..Default::default()
        })
    }
}

/// Registers the cel shading shader libraries (`rose::toon_bands`,
/// `rose::toon_lighting`), the shared parameter buffer and [`CelShadingSettings`].
pub struct CelShadingPlugin;

impl Plugin for CelShadingPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::load_internal_asset!(
            app,
            TOON_BANDS_SHADER_HANDLE,
            "shaders/toon_bands.wgsl",
            Shader::from_wgsl
        );
        bevy::asset::load_internal_asset!(
            app,
            TOON_LIGHTING_SHADER_HANDLE,
            "shaders/toon_lighting.wgsl",
            Shader::from_wgsl
        );

        app.init_resource::<CelShadingSettings>()
            .add_systems(PreStartup, insert_toon_params_buffer)
            // PostUpdate: after the settings UI (Update) wrote the settings.
            .add_systems(PostUpdate, apply_cel_shading_settings_system);
    }
}

fn insert_toon_params_buffer(
    settings: Res<CelShadingSettings>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let _ = buffers.insert(
        &TOON_PARAMS_BUFFER_HANDLE,
        ShaderBuffer::from(settings.toon_params()),
    );
}

/// Writes [`CelShadingSettings`] into the shared parameter buffer (only when they
/// changed) and keeps every main camera's [`CelOutline`] equal to the settings
/// (checked each frame, written only on a difference, so a camera spawned later picks
/// them up). The water reflection camera never gets outlines.
pub fn apply_cel_shading_settings_system(
    mut commands: Commands,
    settings: Res<CelShadingSettings>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    cameras: Query<(Entity, Option<&CelOutline>), (With<Camera3d>, Without<WaterReflectionCamera>)>,
) {
    if settings.is_changed() {
        if let Some(mut buffer) = buffers.get_mut(&TOON_PARAMS_BUFFER_HANDLE) {
            buffer.set_data(settings.toon_params());
        }
    }

    let desired = settings.outline();
    for (entity, current) in &cameras {
        match (&desired, current) {
            (Some(desired), Some(current)) if desired == current => {}
            (Some(desired), _) => {
                commands.entity(entity).insert(*desired);
            }
            (None, Some(_)) => {
                commands.entity(entity).remove::<CelOutline>();
            }
            (None, None) => {}
        }
    }
}
