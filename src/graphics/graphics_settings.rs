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

/// MSAA anti-aliasing sample counts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum MsaaSamples {
    /// No MSAA (1 sample)
    #[default]
    X1,
    /// 2x MSAA
    X2,
    /// 4x MSAA
    X4,
    /// 8x MSAA
    X8,
}

impl MsaaSamples {
    /// Returns the sample count for Bevy's Msaa resource
    pub fn sample_count(&self) -> u32 {
        match self {
            MsaaSamples::X1 => 1,
            MsaaSamples::X2 => 2,
            MsaaSamples::X4 => 4,
            MsaaSamples::X8 => 8,
        }
    }

    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            MsaaSamples::X1 => "Off",
            MsaaSamples::X2 => "2x MSAA",
            MsaaSamples::X4 => "4x MSAA",
            MsaaSamples::X8 => "8x MSAA",
        }
    }
}

/// Shadow quality presets that configure cascade settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum ShadowQuality {
    /// Shadows disabled
    Off,
    /// Low: 1 cascade, 1024 shadow map
    Low,
    /// Medium: 2 cascades, 2048 shadow map
    #[default]
    Medium,
    /// High: 3 cascades, 2048 shadow map (reduced from4 to avoid view uniform buffer overrun)
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
            ShadowQuality::High => 3,  // Reduced from4 to avoid buffer overrun
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

/// Shadow filtering method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum GraphicsShadowFilteringMethod {
    /// Hardware 2x2 PCF (fastest, lowest quality)
    Hardware2x2,
    /// Gaussian filtering (balanced)
    #[default]
    Gaussian,
    /// Temporal filtering (slowest, highest quality)
    Temporal,
}

impl GraphicsShadowFilteringMethod {
    /// Returns a display-friendly name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            GraphicsShadowFilteringMethod::Hardware2x2 => "Hardware 2x2",
            GraphicsShadowFilteringMethod::Gaussian => "Gaussian",
            GraphicsShadowFilteringMethod::Temporal => "Temporal",
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
    /// Returns the mip bias for this quality level
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
    /// TonyMcMapface (default, neutral)
    #[default]
    TonyMcMapface,
    /// Blender filmic
    BlenderFilmic,
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
    #[default]
    Disabled,
    /// Low quality
    Low,
    /// Medium quality
    Medium,
    /// High quality
    High,
    /// Ultra quality
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

#[derive(Debug, Clone, Reflect)]
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
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
pub struct GraphicsSettings {
    // === Display Settings ===
    /// VSync mode: 0 = Off, 1 = On (FIFO), 2 = Mailbox
    pub vsync_mode: VsyncMode,

    /// MSAA sample count (1, 2, 4, 8)
    pub msaa_samples: MsaaSamples,

    /// View distance / draw distance in meters
    pub view_distance: f32,

    // === Shadow Settings ===
    /// Shadow quality preset
    pub shadow_quality: ShadowQuality,

    /// Maximum shadow draw distance in world units
    pub shadow_max_distance: f32,

    /// Shadow filtering method
    pub shadow_filtering: GraphicsShadowFilteringMethod,

    // === Post-Processing Settings ===
    /// Brightness adjustment (0.0 - 2.0, default 1.0)
    /// Applied through color grading exposure
    pub brightness: f32,

    /// Contrast adjustment (0.0 - 2.0, default 1.0)
    /// Applied through color grading contrast
    pub contrast: f32,

    /// Saturation adjustment (0.0 - 2.0, default 1.0)
    /// Applied through color grading saturation
    pub saturation: f32,

    /// Gamma correction (0.5 - 2.5, default 1.0)
    /// Applied through color grading gamma
    pub gamma: f32,

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
            msaa_samples: MsaaSamples::X1,
            view_distance: 500.0,

            // Shadows - medium quality
            shadow_quality: ShadowQuality::default(),
            shadow_max_distance: 150.0,
            shadow_filtering: GraphicsShadowFilteringMethod::default(),

            // Post-processing - neutral defaults
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,

            // Effects
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

            // Ambient Lighting
            ambient_light_brightness: 1.5,
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
            vsync_mode: VsyncMode::Enabled,
            msaa_samples: MsaaSamples::X1,
            view_distance: 300.0,
            shadow_quality: ShadowQuality::Low,
            shadow_max_distance: 50.0,
            shadow_filtering: GraphicsShadowFilteringMethod::Hardware2x2,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
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
            ambient_light_brightness: 1.0,
            ambient_light_color: Color::WHITE,
            terrain_light_intensity: 5.0,
            sailing: SailingGraphicsSettings::default(),
        }
    }

    /// Balanced preset for mid-range hardware
    /// Note: SSAO requires MSAA Off, so we use MSAA X1 and SSAO Low for better visual quality
    pub fn medium_preset() -> Self {
        Self {
            vsync_mode: VsyncMode::Enabled,
            msaa_samples: MsaaSamples::X1, // Must be X1 (Off) for SSAO compatibility
            view_distance: 500.0,
            shadow_quality: ShadowQuality::Medium,
            shadow_max_distance: 100.0,
            shadow_filtering: GraphicsShadowFilteringMethod::Gaussian,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            bloom_enabled: true,
            bloom_intensity: 0.1,
            motion_blur_enabled: false,
            motion_blur_intensity: 0.5,
            ssao_enabled: true,
            ssao_quality: SsaoQuality::Low,
            dof_enabled: false,
            tonemapping: TonemappingMode::TonyMcMapface,
            texture_quality: TextureQuality::Medium,
            fxaa_enabled: false,
            smaa_quality: SmaaQuality::Disabled,
            ambient_light_brightness: 1.0,
            ambient_light_color: Color::WHITE,
            terrain_light_intensity: 5.0,
            sailing: SailingGraphicsSettings::default(),
        }
    }

    /// High quality preset for modern hardware
    /// Note: SSAO requires MSAA Off, so we use MSAA X1 and SSAO Medium for better visual quality
    pub fn high_preset() -> Self {
        Self {
            vsync_mode: VsyncMode::Enabled,
            msaa_samples: MsaaSamples::X1, // Must be X1 (Off) for SSAO compatibility
            view_distance: 800.0,
            shadow_quality: ShadowQuality::High,
            shadow_max_distance: 200.0,
            shadow_filtering: GraphicsShadowFilteringMethod::Gaussian,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            bloom_enabled: true,
            bloom_intensity: 0.15,
            motion_blur_enabled: false,
            motion_blur_intensity: 0.5,
            ssao_enabled: true,
            ssao_quality: SsaoQuality::Medium,
            dof_enabled: false,
            tonemapping: TonemappingMode::TonyMcMapface,
            texture_quality: TextureQuality::High,
            fxaa_enabled: false,
            smaa_quality: SmaaQuality::Disabled,
            ambient_light_brightness: 1.0,
            ambient_light_color: Color::WHITE,
            terrain_light_intensity: 5.0,
            sailing: SailingGraphicsSettings::default(),
        }
    }

    /// Ultra preset for high-end hardware
    /// Note: SSAO requires MSAA Off, so we use MSAA X1 and SSAO High for best visual quality
    pub fn ultra_preset() -> Self {
        Self {
            vsync_mode: VsyncMode::Mailbox,
            msaa_samples: MsaaSamples::X1, // Must be X1 (Off) for SSAO compatibility
            view_distance: 1500.0,
            shadow_quality: ShadowQuality::Ultra,
            shadow_max_distance: 400.0,
            shadow_filtering: GraphicsShadowFilteringMethod::Temporal,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            bloom_enabled: true,
            bloom_intensity: 0.2,
            motion_blur_enabled: true,
            motion_blur_intensity: 0.3,
            ssao_enabled: true,
            ssao_quality: SsaoQuality::High,
            dof_enabled: true,
            tonemapping: TonemappingMode::TonyMcMapface,
            texture_quality: TextureQuality::Ultra,
            fxaa_enabled: false,
            smaa_quality: SmaaQuality::High,
            ambient_light_brightness: 1.0,
            ambient_light_color: Color::WHITE,
            terrain_light_intensity: 5.0,
            sailing: SailingGraphicsSettings::default(),
        }
    }
}
