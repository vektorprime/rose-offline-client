//! Graphics Settings Resource and Enums
//!
//! This module defines all graphics-related configuration options that can be
//! modified at runtime through the settings UI.

use bevy::{prelude::*, reflect};

/// VSync configuration options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum VsyncMode {
    /// VSync disabled - unlimited FPS, possible tearing
    Disabled,
    /// VSync enabled - caps to refresh rate, no tearing
    #[default]
    Enabled,
    /// Mailbox mode - triple buffering, lowest latency with no tearing
    Mailbox,
}

impl VsyncMode {
    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            VsyncMode::Disabled => "Disabled",
            VsyncMode::Enabled => "Enabled",
            VsyncMode::Mailbox => "Mailbox (Triple Buffer)",
        }
    }
}

// NOTE: no MSAA setting. The main camera always uses deferred rendering
// (DeferredPrepass), and Bevy's `check_msaa` forces `Msaa::Off` on deferred
// cameras ("MSAA is incompatible with deferred rendering"). SMAA/FXAA are the
// anti-aliasing options.

/// Shadow quality presets that configure cascade settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum ShadowQuality {
    /// Shadows disabled
    Off,
    /// Low: 1 cascade, 1024 shadow map
    Low,
    /// Medium: 2 cascades, 2048 shadow map
    Medium,
    /// High: 3 cascades, 2048 shadow map (default; reduced from 4 to avoid view uniform buffer overrun)
    #[default]
    High,
    /// Ultra: 4 cascades, 4096 shadow map
    Ultra,
}

impl ShadowQuality {
    /// Returns the cascade count for this quality level
    pub fn cascade_count(&self) -> usize {
        match self {
            ShadowQuality::Off => 0,
            ShadowQuality::Low => 1,
            ShadowQuality::Medium => 2,
            ShadowQuality::High => 3, // Reduced from4 to avoid buffer overrun
            ShadowQuality::Ultra => 4,
        }
    }

    /// Returns the shadow map resolution for this quality level
    pub fn shadow_map_size(&self) -> usize {
        match self {
            ShadowQuality::Off => 0,
            ShadowQuality::Low => 1024,
            ShadowQuality::Medium | ShadowQuality::High => 2048,
            ShadowQuality::Ultra => 4096,
        }
    }

    /// Returns the maximum shadow distance
    pub fn max_distance(&self) -> f32 {
        match self {
            ShadowQuality::Off => 0.0,
            ShadowQuality::Low => 50.0,
            ShadowQuality::Medium => 100.0,
            ShadowQuality::High => 200.0,
            ShadowQuality::Ultra => 400.0,
        }
    }

    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ShadowQuality::Off => "Off",
            ShadowQuality::Low => "Low",
            ShadowQuality::Medium => "Medium",
            ShadowQuality::High => "High",
            ShadowQuality::Ultra => "Ultra",
        }
    }
}

/// Shadow filtering method.
///
/// Bevy's `ShadowFilteringMethod::Temporal` is intentionally not offered: it
/// is a per-frame randomized filter meant to be resolved by TAA, and this
/// client has no TAA, so it only produced shimmering shadow edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum GraphicsShadowFilteringMethod {
    /// Hardware 2x2 PCF (fastest, lowest quality)
    Hardware2x2,
    /// Gaussian filtering (soft, stable; best quality without TAA)
    #[default]
    Gaussian,
}

impl GraphicsShadowFilteringMethod {
    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            GraphicsShadowFilteringMethod::Hardware2x2 => "Hardware 2x2",
            GraphicsShadowFilteringMethod::Gaussian => "Gaussian",
        }
    }
}

/// Texture quality levels affecting mip selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum TextureQuality {
    /// Lowest quality, highest mip bias
    Low,
    /// Medium quality
    Medium,
    /// High quality (default)
    #[default]
    High,
    /// Maximum quality, no mip bias
    Ultra,
}

impl TextureQuality {
    /// Returns the mip bias for this quality level.
    /// NOTE: lod_min_clamp cannot express negative bias, so Ultra (-0.5) behaves
    /// like High (full res) in apply_texture_quality_system; the enum keeps -0.5
    /// for a future anisotropy bump.
    pub fn mip_bias(&self) -> f32 {
        match self {
            TextureQuality::Low => 2.0,
            TextureQuality::Medium => 1.0,
            TextureQuality::High => 0.0,
            TextureQuality::Ultra => -0.5,
        }
    }

    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TextureQuality::Low => "Low",
            TextureQuality::Medium => "Medium",
            TextureQuality::High => "High",
            TextureQuality::Ultra => "Ultra",
        }
    }
}

/// Tonemapping algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum TonemappingMode {
    /// No tonemapping
    None,
    /// Reinhard simple
    Reinhard,
    /// Reinhard luminance
    ReinhardLuminance,
    /// ACES filmic
    AcesFitted,
    /// AgX (neutral, requires LUT)
    AgX,
    /// Somewhat boring display transform
    SomewhatBoringDisplayTransform,
    /// TonyMcMapface (neutral) - default filmic curve for the HDR atmosphere pipeline
    #[default]
    TonyMcMapface,
    /// Blender filmic
    BlenderFilmic,
    /// Khronos PBR Neutral: near-identity below ~0.76 so base colors (the
    /// painted textures) stay faithful and saturated; only highlights compress.
    KhronosPbrNeutral,
}

impl TonemappingMode {
    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TonemappingMode::None => "None",
            TonemappingMode::Reinhard => "Reinhard",
            TonemappingMode::ReinhardLuminance => "Reinhard Luminance",
            TonemappingMode::AcesFitted => "ACES Fitted",
            TonemappingMode::AgX => "AgX",
            TonemappingMode::SomewhatBoringDisplayTransform => "Somewhat Boring",
            TonemappingMode::TonyMcMapface => "TonyMcMapface",
            TonemappingMode::BlenderFilmic => "Blender Filmic",
            TonemappingMode::KhronosPbrNeutral => "Khronos PBR Neutral",
        }
    }
}

/// SSAO quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum SsaoQuality {
    /// SSAO disabled
    Off,
    /// Low quality, fewer samples
    Low,
    /// Medium quality (default)
    #[default]
    Medium,
    /// High quality, more samples
    High,
    /// Ultra quality, maximum samples
    Ultra,
}

impl SsaoQuality {
    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            SsaoQuality::Off => "Off",
            SsaoQuality::Low => "Low",
            SsaoQuality::Medium => "Medium",
            SsaoQuality::High => "High",
            SsaoQuality::Ultra => "Ultra",
        }
    }

    /// Returns the quality level as a u32 for simple comparisons
    pub fn quality_level(&self) -> u32 {
        match self {
            SsaoQuality::Off => 0,
            SsaoQuality::Low => 1,
            SsaoQuality::Medium => 2,
            SsaoQuality::High => 3,
            SsaoQuality::Ultra => 4,
        }
    }
}

/// SMAA quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum SmaaQuality {
    /// SMAA disabled
    Disabled,
    /// Low quality
    Low,
    /// Medium quality
    Medium,
    /// High quality
    High,
    /// Ultra quality (default; the camera spawns without any MSAA)
    #[default]
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum SailQuality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum WaveQuality {
    Low,
    High,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone)]
pub struct SailingGraphicsSettings {
    pub wake_particles_enabled: bool,
    pub sail_deformation_quality: SailQuality,
    pub ocean_wave_quality: WaveQuality,
    pub bow_spray_enabled: bool,
}

impl Default for SailingGraphicsSettings {
    fn default() -> Self {
        Self {
            wake_particles_enabled: true,
            sail_deformation_quality: SailQuality::Medium,
            ocean_wave_quality: WaveQuality::High,
            bow_spray_enabled: true,
        }
    }
}

impl SmaaQuality {
    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            SmaaQuality::Disabled => "Off",
            SmaaQuality::Low => "Low",
            SmaaQuality::Medium => "Medium",
            SmaaQuality::High => "High",
            SmaaQuality::Ultra => "Ultra",
        }
    }
}

/// Resource for storing graphics settings that can be modified at runtime.
/// These settings control visual quality and performance tradeoffs.
#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
pub struct GraphicsSettings {
    // === Display Settings ===
    /// VSync mode: 0 = Off, 1 = On (FIFO), 2 = Mailbox
    pub vsync_mode: VsyncMode,

    /// View distance / draw distance in meters
    pub view_distance: f32,

    // === Shadow Settings ===
    /// Shadow quality preset
    pub shadow_quality: ShadowQuality,

    /// Maximum shadow draw distance in world units
    pub shadow_max_distance: f32,

    /// Shadow filtering method
    pub shadow_filtering: GraphicsShadowFilteringMethod,

    // === Effects Settings ===
    /// Bloom effect enabled
    pub bloom_enabled: bool,

    /// Bloom intensity (0.0 - 1.0)
    pub bloom_intensity: f32,

    /// Motion blur enabled
    pub motion_blur_enabled: bool,

    /// Motion blur intensity (0.0 - 1.0)
    pub motion_blur_intensity: f32,

    /// SSAO enabled
    pub ssao_enabled: bool,

    /// SSAO quality level
    pub ssao_quality: SsaoQuality,

    /// Depth of field enabled
    pub dof_enabled: bool,

    // === Advanced Settings ===
    /// Tonemapping algorithm
    pub tonemapping: TonemappingMode,

    /// Texture quality level
    pub texture_quality: TextureQuality,

    /// FXAA enabled (fallback if MSAA disabled)
    pub fxaa_enabled: bool,

    /// SMAA quality level (alternative to FXAA)
    pub smaa_quality: SmaaQuality,

    /// Auto Exposure enabled (histogram auto-exposure component on the camera)
    pub auto_exposure_enabled: bool,

    /// Auto Exposure target in EV: the log2 luminance (pre-tonemap) that a
    /// daylight scene's average is driven to. 0.0 is Bevy's default
    /// (average -> 1.0, very bright); -2.5 is photographic middle grey (0.18).
    /// Default -1.3 (tuned in-game). Dark scenes adapt only partially below this, so night and
    /// caves stay darker than day. Only used while Auto Exposure is enabled.
    pub auto_exposure_target_ev: f32,

    // === Ambient Lighting Settings ===
    /// Ambient light brightness (0.0 - 2.0, default 1.0)
    /// This is a multiplier applied to the base ambient light brightness
    pub ambient_light_brightness: f32,

    /// Ambient light color (RGB)
    pub ambient_light_color: Color,

    // === Terrain Lighting Settings ===
    /// Terrain light intensity scale (matches terrain lighting to model lighting)
    /// Higher values make the terrain brighter. Default is 5.0.
    pub terrain_light_intensity: f32,

    /// Sailing-specific graphics quality toggles.
    pub sailing: SailingGraphicsSettings,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            // Display - balanced defaults
            vsync_mode: VsyncMode::default(),
            view_distance: 500.0,

            // Shadows - high quality (3 cascades, 2048, 200m)
            shadow_quality: ShadowQuality::default(),
            shadow_max_distance: 200.0,
            shadow_filtering: GraphicsShadowFilteringMethod::default(),

            // Effects (Bloom + SSAO default ON; disable via Graphics tab on weak hardware)
            bloom_enabled: true,
            bloom_intensity: 0.15,
            motion_blur_enabled: false,
            motion_blur_intensity: 0.5,
            ssao_enabled: true,
            ssao_quality: SsaoQuality::default(),
            dof_enabled: false,

            // Advanced
            tonemapping: TonemappingMode::default(),
            texture_quality: TextureQuality::default(),
            fxaa_enabled: false,
            smaa_quality: SmaaQuality::default(),
            auto_exposure_enabled: true,
            auto_exposure_target_ev: -1.3,

            // Ambient Lighting
            ambient_light_brightness: 1.0,
            ambient_light_color: Color::WHITE,

            // Terrain Lighting
            terrain_light_intensity: 5.0,

            // Sailing
            sailing: SailingGraphicsSettings::default(),
        }
    }
}

impl GraphicsSettings {
    /// Low-end preset for older hardware
    pub fn low_preset() -> Self {
        Self {
            view_distance: 300.0,
            shadow_quality: ShadowQuality::Low,
            shadow_max_distance: 50.0,
            shadow_filtering: GraphicsShadowFilteringMethod::Hardware2x2,
            bloom_enabled: false,
            bloom_intensity: 0.0,
            motion_blur_enabled: false,
            motion_blur_intensity: 0.0,
            ssao_enabled: false,
            ssao_quality: SsaoQuality::Off,
            dof_enabled: false,
            tonemapping: TonemappingMode::Reinhard,
            texture_quality: TextureQuality::Low,
            fxaa_enabled: true,
            smaa_quality: SmaaQuality::Disabled,
            auto_exposure_enabled: false,
            ambient_light_brightness: 1.0,
            ..Default::default()
        }
    }

    /// Balanced preset for mid-range hardware
    /// Note: SSAO requires MSAA Off, so we use MSAA X1 and SSAO Low for better visual quality
    pub fn medium_preset() -> Self {
        Self {
            shadow_quality: ShadowQuality::Medium,
            shadow_max_distance: 100.0,
            bloom_intensity: 0.1,
            ssao_quality: SsaoQuality::Low,
            texture_quality: TextureQuality::Medium,
            ambient_light_brightness: 1.0,
            ..Default::default()
        }
    }

    /// High quality preset for modern hardware
    /// Note: SSAO requires MSAA Off, so we use MSAA X1 and SSAO Medium for better visual quality
    pub fn high_preset() -> Self {
        Self {
            view_distance: 800.0,
            shadow_quality: ShadowQuality::High,
            shadow_max_distance: 200.0,
            ambient_light_brightness: 1.0,
            ..Default::default()
        }
    }

    /// Ultra preset for high-end hardware
    /// Note: SSAO requires MSAA Off, so we use MSAA X1 and SSAO High for best visual quality
    pub fn ultra_preset() -> Self {
        Self {
            vsync_mode: VsyncMode::Mailbox,
            view_distance: 1500.0,
            shadow_quality: ShadowQuality::Ultra,
            shadow_max_distance: 400.0,
            // Gaussian, not Temporal: Temporal needs TAA (which this client
            // lacks) and just shimmered.
            shadow_filtering: GraphicsShadowFilteringMethod::Gaussian,
            bloom_intensity: 0.2,
            motion_blur_enabled: true,
            motion_blur_intensity: 0.3,
            ssao_quality: SsaoQuality::High,
            dof_enabled: true,
            texture_quality: TextureQuality::Ultra,
            smaa_quality: SmaaQuality::High,
            ambient_light_brightness: 1.0,
            ..Default::default()
        }
    }
}
