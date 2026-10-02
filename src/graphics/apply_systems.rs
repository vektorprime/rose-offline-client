//! Apply Systems for Graphics Settings
//!
//! This module contains systems that apply `GraphicsSettings` changes to the
//! actual render configuration (cameras, lights, etc.).
//!
//! INVARIANT: every camera query here excludes `WaterReflectionCamera`. Post
//! components must never be inserted on the reflection camera: `#[require]`
//! chains (SSAO pulls DepthPrepass+NormalPrepass, MotionBlur pulls
//! DepthPrepass+MotionVectorPrepass) would give it prepass phases without
//! deferred phases, and Bevy 0.18.1 panics in `queue_prepass_material_meshes`
//! (prepass/mod.rs unwrap) once a deferred material is visible to it.

use crate::dds_image_loader::{ImagePluginDefaultSampler, TextureLodMinClamp};
use crate::graphics::*;
use bevy::{
    anti_alias::{
        fxaa::Fxaa,
        smaa::{Smaa, SmaaPreset},
    },
    core_pipeline::tonemapping::Tonemapping,
    image::{Image, ImageSampler, ImageSamplerDescriptor},
    math::cubic_splines::LinearSpline,
    pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
    post_process::motion_blur::MotionBlur,
    prelude::*,
};
use bevy_light::{
    CascadeShadowConfig, DirectionalLight, DirectionalLightShadowMap, ShadowFilteringMethod,
};
use bevy_post_process::auto_exposure::{AutoExposure, AutoExposureCompensationCurve};
use bevy_post_process::bloom::Bloom;
use bevy_post_process::dof::DepthOfField;

/// System that applies tonemapping settings to cameras.
///
/// RESTORED 2026-09-25 (second removal reverted): the white film/flash was
/// traced to ColorGrading exposure on clamped HDR, not the tonemap pass.
/// Color grading (and its writer) are now DELETED; this is a neutral pipeline.
pub fn apply_tonemapping_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<
        &mut Tonemapping,
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for mut tonemapping in cameras.iter_mut() {
        // GATED: writing marks the view changed and forces post-chain
        // re-specialization.
        let new = match graphics_settings.tonemapping {
            TonemappingMode::None => Tonemapping::None,
            TonemappingMode::Reinhard => Tonemapping::Reinhard,
            TonemappingMode::ReinhardLuminance => Tonemapping::ReinhardLuminance,
            TonemappingMode::AcesFitted => Tonemapping::AcesFitted,
            TonemappingMode::AgX => Tonemapping::AgX,
            TonemappingMode::SomewhatBoringDisplayTransform => {
                Tonemapping::SomewhatBoringDisplayTransform
            }
            TonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
            TonemappingMode::BlenderFilmic => Tonemapping::BlenderFilmic,
            TonemappingMode::KhronosPbrNeutral => Tonemapping::KhronosPbrNeutral,
        };
        if *tonemapping != new {
            *tonemapping = new;
        }
    }
}

/// System that applies shadow quality settings to directional lights.
/// Includes MoonLight: it casts shadows while the sun is down (the time-of-day
/// system owns the on/off flags), so it needs the same cascade layout.
/// Skips SkyFillLight: the sky-bounce fill never casts shadows by design.
pub fn apply_shadow_quality_system(
    graphics_settings: Res<GraphicsSettings>,
    mut directional_lights: Query<
        (&mut DirectionalLight, Option<&mut CascadeShadowConfig>),
        Without<crate::render::zone_lighting::SkyFillLight>,
    >,
    mut shadow_map_resource: ResMut<DirectionalLightShadowMap>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    let quality = &graphics_settings.shadow_quality;

    // Enable/disable shadows based on quality
    let shadow_maps_enabled = *quality != ShadowQuality::Off;

    // Only update shadow map resolution when shadows are enabled.
    // wgpu requires non-zero texture dimensions, so we keep the previous/valid size
    // when shadows are disabled. The shadow_maps_enabled flag on lights controls
    // whether shadows are actually rendered.
    // Guarded: writing `size` marks the resource changed and Bevy reallocates
    // ALL shadow maps, so an unconditional write here stalled every frame
    // while the Graphics tab was open (massive hitch, idle CPU/GPU).
    if shadow_maps_enabled {
        let size = quality.shadow_map_size();
        if shadow_map_resource.size != size {
            shadow_map_resource.size = size;
        }
    }

    // NOTE: this system intentionally does NOT write
    // `light.shadow_maps_enabled`. The per-light on/off flag is owned by
    // the time-of-day table (`sync_...` in zone_lighting.rs), which ANDs
    // the quality switch with sun elevation. Writing it here fought with
    // that system (1Hz on/off flap + pipeline re-specialization storm =
    // fullscreen flashing). See bevy-0.19-upgrade-plan.md.
    for (_light, cascade_config) in directional_lights.iter_mut() {

        // Calculate bounds for cascades (gated: rebuilding the vec + writing
        // marks the light changed and re-specializes shadow pipelines).
        if let Some(mut config) = cascade_config {
            let cascade_count = quality.cascade_count();
            if cascade_count > 0 {
                let max_distance = graphics_settings
                    .shadow_max_distance
                    .min(quality.max_distance());

                // Calculate bounds for cascades
                let first_bound = max_distance / cascade_count as f32;
                let bounds: Vec<f32> = (0..cascade_count)
                    .map(|i| first_bound * (i + 1) as f32)
                    .collect();

                if config.bounds != bounds {
                    config.bounds = bounds;
                }
                if config.overlap_proportion != 0.2 {
                    config.overlap_proportion = 0.2;
                }
                if config.minimum_distance != 0.1 {
                    config.minimum_distance = 0.1;
                }
            }
        }
    }
}

/// System that applies bloom settings to cameras.
/// Disabling REMOVES the component so the bloom pyramid pass is skipped entirely.
/// Previously intensity was set to 0.0, which still dispatched the full pass.
pub fn apply_bloom_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&Bloom>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, bloom) in cameras.iter() {
        if graphics_settings.bloom_enabled {
            // SINGLE-OWNER + write-on-change: Bloom is owned by this system
            // only (PostProcessing page no longer toggles it). Re-inserting an
            // identical component churns ViewTarget/pipeline re-specialization.
            let same = matches!(bloom, Some(existing)
                if (existing.intensity - graphics_settings.bloom_intensity).abs() <= 1e-4);
            if !same {
                commands.entity(entity).insert(Bloom {
                    intensity: graphics_settings.bloom_intensity,
                    ..Bloom::NATURAL
                });
            }
        } else if bloom.is_some() {
            commands.entity(entity).remove::<Bloom>();
        }
    }
}

/// System that applies the shadow filtering method.
///
/// `ShadowFilteringMethod` is a CAMERA component in Bevy ("add this component
/// to a Camera3d"). This used to query `With<DirectionalLight>`, which matched
/// nothing, so the dropdown never took effect.
pub fn apply_shadow_filtering_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<
        &mut ShadowFilteringMethod,
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    for mut filtering in cameras.iter_mut() {
        // GATED (was unconditional).
        let new = match graphics_settings.shadow_filtering {
            GraphicsShadowFilteringMethod::Hardware2x2 => ShadowFilteringMethod::Hardware2x2,
            GraphicsShadowFilteringMethod::Gaussian => ShadowFilteringMethod::Gaussian,
        };
        if *filtering != new {
            *filtering = new;
        }
    }
}

/// System that applies SSAO enable/quality to cameras via insert/remove.
/// Previously `ssao_enabled` was never read here and the camera always kept SSAO
/// (downgraded to Low at best), so Low settings still paid the full SSAO pass.
pub fn apply_ssao_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&ScreenSpaceAmbientOcclusion>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, ssao) in cameras.iter() {
        if !graphics_settings.ssao_enabled || graphics_settings.ssao_quality == SsaoQuality::Off
        {
            if ssao.is_some() {
                commands
                    .entity(entity)
                    .remove::<ScreenSpaceAmbientOcclusion>();
            }
            continue;
        }
        let level = match graphics_settings.ssao_quality {
            SsaoQuality::Off => continue, // handled above
            SsaoQuality::Low => ScreenSpaceAmbientOcclusionQualityLevel::Low,
            SsaoQuality::Medium => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            SsaoQuality::High => ScreenSpaceAmbientOcclusionQualityLevel::High,
            SsaoQuality::Ultra => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        };
        // SINGLE-OWNER + write-on-change (see apply_bloom_system).
        let same = matches!(ssao, Some(existing) if existing.quality_level == level);
        if !same {
            commands.entity(entity).insert(ScreenSpaceAmbientOcclusion {
                quality_level: level,
                ..Default::default()
            });
        }
    }
}

/// System that applies SMAA quality to cameras via insert/remove.
/// The camera no longer spawns SMAA by default; this adds it only when requested.
pub fn apply_smaa_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&Smaa>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    for (entity, smaa) in cameras.iter() {
        let preset = match graphics_settings.smaa_quality {
            SmaaQuality::Disabled => None,
            SmaaQuality::Low => Some(SmaaPreset::Low),
            SmaaQuality::Medium => Some(SmaaPreset::Medium),
            SmaaQuality::High => Some(SmaaPreset::High),
            SmaaQuality::Ultra => Some(SmaaPreset::Ultra),
        };
        match preset {
            None => {
                if smaa.is_some() {
                    commands.entity(entity).remove::<Smaa>();
                }
            }
            Some(preset) => {
                let needs_insert = match smaa {
                    Some(existing) => existing.preset != preset,
                    None => true,
                };
                if needs_insert {
                    commands.entity(entity).insert(Smaa {
                        preset,
                        ..Default::default()
                    });
                }
            }
        }
    }
}

/// Bottom/top of the default `AutoExposure` histogram range (-8..=8), i.e. the
/// span of metered scene averages (log2 luminance) the curve must cover.
const AUTO_EXPOSURE_MIN_LOG_LUM: f32 = -8.0;
const AUTO_EXPOSURE_MAX_LOG_LUM: f32 = 8.0;
/// Metered scene average (log2 luminance, before exposure) at and above which
/// Auto Exposure fully adapts to the target. Daylight scenes meter around -1
/// (unit-scale terrain lighting ~0.5-0.8, 7000-lux PBR at EV100 9.7).
const AUTO_EXPOSURE_FULL_ADAPTATION_LOG_LUM: f32 = -2.0;
/// Fraction of the extra darkness below the full-adaptation point that Auto
/// Exposure compensates. 0.5 = half, so night/caves stay visibly darker than
/// day instead of being lifted to daylight brightness.
const AUTO_EXPOSURE_DARK_ADAPTATION: f32 = 0.5;

/// Builds the camera's Auto Exposure compensation curve.
///
/// Bevy's shader sets `target = curve(avg) - avg`, so the curve's y value is
/// the log2 luminance the metered scene average is driven to. Bevy's default
/// curve is flat 0 (average -> 1.0, very bright, and night lifted to day).
/// This curve is flat at `target_ev` for daylight-and-brighter scenes and
/// slopes down below that for partial adaptation in dark scenes.
pub fn auto_exposure_compensation_curve(target_ev: f32) -> AutoExposureCompensationCurve {
    let target_ev = target_ev.clamp(-6.0, 2.0);
    let dark_target_ev = target_ev
        - (1.0 - AUTO_EXPOSURE_DARK_ADAPTATION)
            * (AUTO_EXPOSURE_FULL_ADAPTATION_LOG_LUM - AUTO_EXPOSURE_MIN_LOG_LUM);
    AutoExposureCompensationCurve::from_curve(LinearSpline::new([
        Vec2::new(AUTO_EXPOSURE_MIN_LOG_LUM, dark_target_ev),
        Vec2::new(AUTO_EXPOSURE_FULL_ADAPTATION_LOG_LUM, target_ev),
        Vec2::new(AUTO_EXPOSURE_MAX_LOG_LUM, target_ev),
    ]))
    .unwrap_or_else(|err| {
        log::error!("[AUTO-EXPOSURE] compensation curve build failed: {err}");
        AutoExposureCompensationCurve::default()
    })
}

/// The camera's shared Auto Exposure compensation curve asset, plus the target
/// it was last built for. Created at app build (src/lib.rs) so the camera can
/// spawn with it; rebuilt in place only by `apply_auto_exposure_system`.
#[derive(Resource)]
pub struct AutoExposureCurve {
    pub handle: Handle<AutoExposureCompensationCurve>,
    target_ev: f32,
}

impl AutoExposureCurve {
    pub fn new(curves: &mut Assets<AutoExposureCompensationCurve>, target_ev: f32) -> Self {
        Self {
            handle: curves.add(auto_exposure_compensation_curve(target_ev)),
            target_ev,
        }
    }

    /// The `AutoExposure` component for the main camera, using this curve.
    pub fn component(&self) -> AutoExposure {
        AutoExposure {
            compensation_curve: self.handle.clone(),
            ..Default::default()
        }
    }
}

/// System that applies AutoExposure enable to cameras via insert/remove, and
/// keeps the compensation curve in sync with `auto_exposure_target_ev`.
/// AutoExposure is spawned by default (matching `GraphicsSettings::default()`);
/// removing the component skips its histogram/adaptation compute passes
/// entirely. Needed as a live toggle to bisect night white-out regressions
/// (see pitfalls/atmosphere-flash.md for the earlier cyan-veil incident).
pub fn apply_auto_exposure_system(
    graphics_settings: Res<GraphicsSettings>,
    mut curve: ResMut<AutoExposureCurve>,
    mut curves: ResMut<Assets<AutoExposureCompensationCurve>>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&AutoExposure>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    // Rebuild the curve only when the target actually moved: the settings
    // resource is marked changed every frame the Graphics tab is open, and a
    // curve edit re-uploads its GPU texture. The camera keeps the same handle.
    let target_ev = graphics_settings.auto_exposure_target_ev;
    if (curve.target_ev - target_ev).abs() > 1e-3 {
        if let Some(mut asset) = curves.get_mut(&curve.handle) {
            *asset = auto_exposure_compensation_curve(target_ev);
        }
        curve.target_ev = target_ev;
    }

    for (entity, auto_exposure) in cameras.iter() {
        if graphics_settings.auto_exposure_enabled {
            let uses_curve =
                auto_exposure.is_some_and(|existing| existing.compensation_curve == curve.handle);
            if !uses_curve {
                commands.entity(entity).insert(curve.component());
            }
        } else if auto_exposure.is_some() {
            commands.entity(entity).remove::<AutoExposure>();
        }
    }
}

/// System that applies motion-blur enable to cameras via insert/remove.
/// Previously the camera hard-added MotionBlur with no removal path.
pub fn apply_motion_blur_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&MotionBlur>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    // motion_blur_intensity (0-1 slider) maps to shutter_angle (0-2.0 rad scale).
    let shutter_angle = (graphics_settings.motion_blur_intensity.clamp(0.0, 1.0) * 2.0).max(0.05);
    for (entity, blur) in cameras.iter() {
        if graphics_settings.motion_blur_enabled {
            let needs_insert = match blur {
                Some(existing) => (existing.shutter_angle - shutter_angle).abs() > 0.01,
                None => true,
            };
            if needs_insert {
                commands.entity(entity).insert(MotionBlur {
                    shutter_angle,
                    ..Default::default()
                });
            }
        } else if blur.is_some() {
            commands.entity(entity).remove::<MotionBlur>();
        }
    }
}

/// System that applies depth-of-field enable from GraphicsSettings.
/// The richer DepthOfFieldSettings resource owns the parameters; this only ensures
/// `dof_enabled=false` removes the component (insert path is owned by
/// apply_depth_of_field_settings so parameters stay in one place).
pub fn apply_dof_enabled_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&DepthOfField>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    if graphics_settings.dof_enabled {
        return;
    }
    for (entity, dof) in cameras.iter() {
        if dof.is_some() {
            commands.entity(entity).remove::<DepthOfField>();
        }
    }
}

/// System that applies FXAA enable to cameras via insert/remove.
/// Previously `fxaa_enabled` (including Low preset's `fxaa:true` fallback) had no
/// consumer, so the preset promised AA it never got.
///
/// FXAA is only applied while SMAA is Disabled: Bevy 0.19.1 orders both passes
/// only `.after(tonemapping)`, so with both on they run in parallel and race
/// the main-texture ping-pong (random pre-tonemap/stale frames on screen).
pub fn apply_fxaa_system(
    graphics_settings: Res<GraphicsSettings>,
    mut commands: Commands,
    cameras: Query<
        (Entity, Option<&Fxaa>),
        (With<Camera>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    if !graphics_settings.is_changed() {
        return;
    }

    let fxaa_wanted = graphics_settings.fxaa_enabled
        && graphics_settings.smaa_quality == SmaaQuality::Disabled;
    for (entity, fxaa) in cameras.iter() {
        if fxaa_wanted {
            if fxaa.is_none() {
                commands.entity(entity).insert(Fxaa::default());
            }
        } else if fxaa.is_some() {
            commands.entity(entity).remove::<Fxaa>();
        }
    }
}

/// System that wires view_distance to camera far plane.
/// Previously the slider (100-2000m) had no consumer: users thought they lowered cost
/// but nothing changed. Mapping far = clamp(view_distance * 16, 6000, 12000):
/// default 500 -> 8000 (matches startup), Low 300 -> 6000 (tighter depth precision),
/// High/Ultra saturate at 12000 (sky radius 4000 + margin). Sky follows the camera so
/// it is never clipped within this range.
pub fn apply_view_distance_system(
    graphics_settings: Res<GraphicsSettings>,
    mut cameras: Query<&mut Projection, (With<Camera>, Without<crate::render::WaterReflectionCamera>)>,
) {
    if !graphics_settings.is_changed() {
        return;
    }
    let far = (graphics_settings.view_distance * 16.0).clamp(6000.0, 12000.0);
    for mut projection in cameras.iter_mut() {
        // Read-only check first (menu-open change spam must not mark cameras).
        let matches = matches!(projection.as_ref(), Projection::Perspective(p) if (p.far - far).abs() <= 1.0);
        if !matches {
            if let Projection::Perspective(perspective) = projection.as_mut() {
                perspective.far = far;
            }
        }
    }
}

/// System that wires texture_quality to image sampler LOD clamps.
///
/// Low/Medium set the sampler's `lod_min_clamp` to 2/1 so the GPU skips the
/// largest mips, like the original client's texture loading scale (textures
/// loaded at 1/4 and 1/2 size); High/Ultra use full resolution (lod_min_clamp
/// cannot express Ultra's negative bias).
///
/// Which images: only those with a mip chain (a min-LOD clamp does nothing on a
/// single-level image), which already leaves out render targets (water
/// reflection), cubemaps and `3ddata/control/` UI textures (the DDS loader keeps
/// both single-level), images built in code and PNG/TGA files from Bevy's
/// loader. Images registered with egui (UI, minimap) are kept at full
/// resolution even with mips: egui draws them at their own size, and the
/// original client loads its UI images unscaled. In practice the clamped images
/// are the DDS game textures.
///
/// On a settings change it checks every image and publishes the clamp to the
/// DDS loader (`TextureLodMinClamp`), so textures loaded later start with it.
/// Other frames it checks only images added, loaded or replaced since the last
/// run (a load that raced the change, or an image not from the DDS loader), or
/// every image again when the egui texture registry changed. Images are read
/// first and `get_mut` only for samplers that actually change: each `get_mut`
/// re-extracts and re-uploads the image.
///
/// A material captures its images' samplers when it is prepared and is not
/// re-prepared when an image changes (Bevy 0.19.1), so after a settings change
/// rewrote samplers, `refresh_materials_after_sampler_change` re-prepares the
/// materials once (`MaterialSamplersStale`). egui rebuilds its bind groups
/// every frame and needs nothing.
pub fn apply_texture_quality_system(
    graphics_settings: Res<GraphicsSettings>,
    loader_lod_min_clamp: Option<Res<TextureLodMinClamp>>,
    image_plugin_default: Option<Res<ImagePluginDefaultSampler>>,
    egui_user_textures: Option<Res<bevy_egui::EguiUserTextures>>,
    mut image_events: MessageReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut material_samplers_stale: ResMut<MaterialSamplersStale>,
) {
    // bias 2.0/1.0 (Low/Medium) -> force mip >= 2/1; 0.0/-0.5 (High/Ultra) -> full res.
    let lod_min = graphics_settings.texture_quality.mip_bias().max(0.0);
    let default_sampler = image_plugin_default.as_deref().map(|sampler| &sampler.0);
    let target_lod_min = |id: AssetId<Image>| -> f32 {
        let shown_by_egui = egui_user_textures
            .as_ref()
            .is_some_and(|textures| textures.image_id(id).is_some());
        if shown_by_egui {
            0.0
        } else {
            lod_min
        }
    };
    let needs_update = |id: AssetId<Image>, image: &Image| -> bool {
        needs_lod_min_clamp(image, target_lod_min(id), default_sampler)
    };

    if graphics_settings.is_changed() {
        if let Some(loader_lod_min_clamp) = &loader_lod_min_clamp {
            loader_lod_min_clamp.set(lod_min);
        }
    }
    let check_all = graphics_settings.is_changed()
        || egui_user_textures
            .as_ref()
            .is_some_and(|textures| textures.is_changed());

    let stale_images: Vec<AssetId<Image>> = if check_all {
        // Every image is checked below, which covers the queued messages too.
        image_events.clear();
        images
            .iter()
            .filter(|(id, image)| needs_update(*id, *image))
            .map(|(id, _)| id)
            .collect()
    } else {
        image_events
            .read()
            .filter_map(|event| match event {
                AssetEvent::Added { id }
                | AssetEvent::LoadedWithDependencies { id }
                | AssetEvent::Modified { id } => Some(*id),
                _ => None,
            })
            .filter(|id| {
                images
                    .get(*id)
                    .is_some_and(|image| needs_update(*id, image))
            })
            .collect()
    };

    // Materials already prepared with these images keep their old samplers
    // until re-prepared (see refresh_materials_after_sampler_change).
    if graphics_settings.is_changed() && !stale_images.is_empty() && !material_samplers_stale.0 {
        material_samplers_stale.0 = true;
    }

    for id in stale_images {
        // Re-check: one image can be listed by several messages (Added, then
        // LoadedWithDependencies), and only the first visit may write.
        if !images.get(id).is_some_and(|image| needs_update(id, image)) {
            continue;
        }
        let Some(mut image) = images.get_mut(id) else {
            continue;
        };
        let mut descriptor = match &image.sampler {
            ImageSampler::Descriptor(descriptor) => descriptor.clone(),
            // A Default sampler means the global default (linear), not
            // `ImageSamplerDescriptor::default()` (nearest), which
            // `get_or_init_descriptor()` would install.
            ImageSampler::Default => match default_sampler {
                Some(default_sampler) => default_sampler.clone(),
                None => continue,
            },
        };
        descriptor.lod_min_clamp = target_lod_min(id);
        image.sampler = ImageSampler::Descriptor(descriptor);
    }
}

/// Set by `apply_texture_quality_system` when a Texture Quality change rewrote
/// image samplers that existing materials captured at prepare time.
#[derive(Resource, Default)]
pub struct MaterialSamplersStale(bool);

/// Run condition for [`refresh_materials_after_sampler_change`].
pub fn material_samplers_stale(stale: Res<MaterialSamplersStale>) -> bool {
    stale.0
}

/// Re-prepares every material that can sample game textures, once, after a
/// Texture Quality change rewrote their images' samplers. Ordered before
/// `apply_texture_quality_system`, so it runs on the frame after the images
/// changed: the render world has prepared the new samplers by then, and the
/// re-prepared bind groups pick them up. `iter_mut` marks every asset Modified;
/// a one-time cost on an explicit settings change.
pub fn refresh_materials_after_sampler_change(
    mut material_samplers_stale: ResMut<MaterialSamplersStale>,
    mut object_materials: ResMut<Assets<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::render::RoseObjectExtension>>>,
    mut effect_materials: ResMut<Assets<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::render::RoseEffectExtension>>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut particle_materials: ResMut<Assets<crate::render::ParticleMaterial>>,
) {
    material_samplers_stale.0 = false;
    object_materials.iter_mut().for_each(|_| {});
    effect_materials.iter_mut().for_each(|_| {});
    standard_materials.iter_mut().for_each(|_| {});
    particle_materials.iter_mut().for_each(|_| {});
}

/// Whether `image` has a mip chain and its sampler's `lod_min_clamp` is not
/// `lod_min` yet. A `Default` sampler is only converted when the global default
/// sampler is known (`ImagePluginDefaultSampler`).
fn needs_lod_min_clamp(
    image: &Image,
    lod_min: f32,
    default_sampler: Option<&ImageSamplerDescriptor>,
) -> bool {
    if image.texture_descriptor.mip_level_count <= 1 {
        return false;
    }
    let current = match &image.sampler {
        ImageSampler::Descriptor(descriptor) => descriptor.lod_min_clamp,
        ImageSampler::Default => match default_sampler {
            Some(default_sampler) => default_sampler.lod_min_clamp,
            None => return false,
        },
    };
    (current - lod_min).abs() > f32::EPSILON
}

/// System that applies ambient lighting settings to the global AmbientLight resource.
/// The brightness is multiplied by a base value of 80.0 (Bevy's default ambient brightness).
///
/// NOTE: `sync_zone_lighting_to_bevy_lights_system` (Update) is the authority for
/// the ambient color/brightness each frame: it blends the zone's
/// `map_ambient_color` with the user's color at a constant Bevy-default base
/// (80.0 lux). Day/night variation comes from the sun + sky-fill lights so
/// shadows keep contrast. This system (PostUpdate) only seeds
/// the same blend when settings change so there is no one-frame flash of flat
/// white ambient and no ping-pong between the two writers.
pub fn apply_ambient_light_system(
    graphics_settings: Res<GraphicsSettings>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    zone_lighting: Option<Res<crate::render::ZoneLighting>>,
) {
    // Skip if settings haven't changed
    if !graphics_settings.is_changed() {
        return;
    }

    let user_color = graphics_settings.ambient_light_color.to_linear();
    if let Some(zone_lighting) = zone_lighting {
        let map = zone_lighting.map_ambient_color;
        let color = Color::from(LinearRgba::new(
            map.x * user_color.red,
            map.y * user_color.green,
            map.z * user_color.blue,
            1.0,
        ));
        // Guarded: same reason as brightness below (avoid dirtying the
        // resource every frame the settings UI is open).
        if ambient_light.color != color {
            ambient_light.color = color;
        }
    } else {
        // No zone loaded yet (menus): fall back to the plain user color.
        ambient_light.color = graphics_settings.ambient_light_color;
    }

    // Apply ambient light brightness
    // Base brightness is 80.0 (Bevy's default), multiplier ranges from 0.0 to 2.0.
    // Kept constant: day/night variation comes from sun + fill, not ambient.
    // Guarded: an unconditional write marks GlobalAmbientLight changed every
    // frame (forcing ambient re-evaluation across the render graph) whenever
    // settings are touched, e.g. while the Graphics tab is open.
    let brightness = 80.0 * graphics_settings.ambient_light_brightness;
    if ambient_light.brightness != brightness {
        ambient_light.brightness = brightness;
    }
}
