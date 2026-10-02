use bevy::{
    app::{App, Plugin},
    asset::{io::Reader, AssetApp, AssetLoader, LoadContext, RenderAssetUsages},
    image::{
        CompressedImageFormatSupport, CompressedImageFormats, ImageAddressMode, ImagePlugin,
        ImageSampler, ImageSamplerDescriptor,
    },
    prelude::{Image, Resource, TypePath},
    render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
    },
};
use log::warn;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

/// Custom asset loader for DDS files.
///
/// BC1/BC2/BC3 (DXT1/3/5) textures are uploaded as-is in their compressed format
/// when the GPU supports BC sampling, so the hardware decodes them (4-8x less memory
/// and upload bandwidth, no CPU decode). Everything else, and the cases listed in
/// [`DdsImageLoader::can_upload_bc`], is converted to RGBA8 on the CPU.
///
/// The file's mip chain is loaded on both paths and sampled trilinearly, like the
/// original client (D3DX loads every level stored in the .dds; min/mag/mip filters
/// are LINEAR, anisotropy unused). Without mips distant textures shimmer, and the
/// Texture Quality min-LOD clamp has no smaller level to clamp to. See
/// [`wanted_mip_levels`] for the textures kept single-level.
#[derive(TypePath)]
pub struct DdsImageLoader {
    /// Compressed formats the render device can sample (from Bevy's
    /// `CompressedImageFormatSupport`).
    supported_compressed_formats: CompressedImageFormats,
    /// Texture Quality clamp given to new mipmapped textures.
    lod_min_clamp: TextureLodMinClamp,
}

/// Texture Quality's sampler `lod_min_clamp`, shared between
/// `apply_texture_quality_system` (writer) and [`DdsImageLoader`] (reader), so
/// textures loaded after the setting is applied start with the clamp.
///
/// Fixing the sampler after the first upload is not enough: Bevy captures an
/// image's sampler in a material's bind group when the material is prepared and
/// does not rebuild that bind group when the image changes later.
#[derive(Resource, Clone, Default)]
pub struct TextureLodMinClamp(Arc<AtomicU32>);

impl TextureLodMinClamp {
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, lod_min_clamp: f32) {
        self.0.store(lod_min_clamp.to_bits(), Ordering::Relaxed);
    }
}

/// Main-world copy of `ImagePlugin::default_sampler`, the sampler Bevy uses for
/// images whose sampler is `ImageSampler::Default`. Bevy keeps it only in the
/// render world; code that turns a `Default` sampler into an explicit descriptor
/// needs it to keep the filtering (`ImageSamplerDescriptor::default()` is NEAREST
/// filtering, not the global default).
#[derive(Resource, Clone, Debug)]
pub struct ImagePluginDefaultSampler(pub ImageSamplerDescriptor);

/// Registers [`DdsImageLoader`] once the render device exists, and inserts the
/// [`TextureLodMinClamp`] and [`ImagePluginDefaultSampler`] resources.
///
/// Registration happens in `finish()` because `CompressedImageFormatSupport` is only
/// inserted once the renderer is initialized (Bevy's own `ImageLoader` does the same).
pub struct DdsImageLoaderPlugin;

impl Plugin for DdsImageLoaderPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let supported_compressed_formats = app
            .world()
            .get_resource::<CompressedImageFormatSupport>()
            .map_or(CompressedImageFormats::NONE, |support| support.0);
        log::info!(
            "[DDS LOADER] GPU BC texture support: {}",
            supported_compressed_formats.contains(CompressedImageFormats::BC)
        );
        let lod_min_clamp = TextureLodMinClamp::default();
        app.insert_resource(lod_min_clamp.clone());
        app.register_asset_loader(DdsImageLoader {
            supported_compressed_formats,
            lod_min_clamp,
        });

        let default_sampler = app
            .get_added_plugins::<ImagePlugin>()
            .first()
            .map(|image_plugin| image_plugin.default_sampler.clone());
        if let Some(default_sampler) = default_sampler {
            app.insert_resource(ImagePluginDefaultSampler(default_sampler));
        }
    }
}

impl DdsImageLoader {
    /// Whether a BC1-3 texture can be uploaded compressed instead of CPU-decoded.
    ///
    /// Not for:
    /// - GPUs without BC support;
    /// - cubemaps (the `#cube` label replicates one 2D image into 6 layers);
    /// - sizes that are not a multiple of the 4x4 block (wgpu requires it for the
    ///   base level; smaller mips are stored and uploaded as whole blocks);
    /// - UI textures under `3ddata/control/`: the UI premultiplies their alpha on
    ///   the CPU (`premultiply_image_alpha`), which needs uncompressed pixels.
    fn can_upload_bc(&self, info: &DdsInfo, is_cube: bool, is_ui: bool) -> bool {
        self.supported_compressed_formats
            .contains(CompressedImageFormats::BC)
            && !is_cube
            && !is_ui
            && info.width % 4 == 0
            && info.height % 4 == 0
    }

    /// Trilinear sampler (the original client's LINEAR min/mag/mip filters), with
    /// the current Texture Quality clamp when there are mips to clamp.
    ///
    /// Address mode follows the original client (zz_material.cpp): the base
    /// texture stage wraps (models tile their UVs past 0..1), every other stage
    /// clamps. Lightmap pages (only ever bound as lightmaps), UI textures and
    /// cubemaps therefore clamp; everything else repeats.
    fn sampler(&self, mip_levels: u32, wrap: bool) -> ImageSampler {
        let mut descriptor = ImageSamplerDescriptor::linear();
        if mip_levels > 1 {
            descriptor.lod_min_clamp = self.lod_min_clamp.get();
        }
        if wrap {
            descriptor.address_mode_u = ImageAddressMode::Repeat;
            descriptor.address_mode_v = ImageAddressMode::Repeat;
            descriptor.address_mode_w = ImageAddressMode::Repeat;
        }
        ImageSampler::Descriptor(descriptor)
    }

    /// Parses `bytes` and builds the image (the caller sets the sampler).
    fn load_image(&self, bytes: &[u8], asset_path: &str, is_cube: bool) -> anyhow::Result<Image> {
        let info = parse_dds_header(bytes)?;
        let is_ui = is_ui_texture(asset_path);

        // Mip levels to load: the wanted chain minus levels missing from a truncated
        // file. A truncated level 0 is left to each path, as before mips were loaded
        // (BC: CPU decode of the blocks present; uncompressed: error).
        let wanted_levels = wanted_mip_levels(&info, is_cube, is_ui);
        let complete_levels = complete_mip_levels(bytes.len(), &info, wanted_levels);
        if complete_levels > 0 && complete_levels < wanted_levels {
            warn!(
                "[DDS LOADER] {}: file truncated, using {} of {} mip levels",
                asset_path, complete_levels, wanted_levels
            );
        }
        let mip_levels = complete_levels.max(1);

        // BC1-3: upload the blocks for the GPU to decode when possible.
        let bc_format = match info.format {
            DdsFormat::Bc1Dxt1 => Some(TextureFormat::Bc1RgbaUnormSrgb),
            DdsFormat::Bc2Dxt3 => Some(TextureFormat::Bc2RgbaUnormSrgb),
            DdsFormat::Bc3Dxt5 => Some(TextureFormat::Bc3RgbaUnormSrgb),
            _ => None,
        };
        if let Some(bc_format) = bc_format {
            if complete_levels > 0 && self.can_upload_bc(&info, is_cube, is_ui) {
                if let Some(image) = load_bc_direct(bytes, &info, bc_format, mip_levels) {
                    return Ok(image);
                }
            }
        }

        // Everything else is converted to R8G8B8A8 on the CPU, one mip level at a time.
        let decode_level: DecodeLevelFn = match info.format {
            DdsFormat::R8G8B8 => convert_rgb_to_rgba,
            DdsFormat::R8G8B8A8 => copy_rgba,
            DdsFormat::B8G8R8A8 => convert_bgra_to_rgba,
            DdsFormat::R8G8B8X8 => convert_rgbx_to_rgba,
            DdsFormat::B8G8R8X8 => convert_bgrx_to_rgba,
            DdsFormat::B8G8R8 => convert_bgr_to_rgba,
            DdsFormat::A1R5G5B5 => convert_a1r5g5b5_to_rgba,
            DdsFormat::R5G6B5 => convert_r5g6b5_to_rgba,
            DdsFormat::A4R4G4B4 => convert_a4r4g4b4_to_rgba,
            DdsFormat::B5G6R5 => convert_b5g6r5_to_rgba,
            DdsFormat::R32G32B32A32Float => convert_rgba32f_to_rgba,
            DdsFormat::Bc1Dxt1 => decompress_bc1_to_rgba,
            DdsFormat::Bc2Dxt3 => decompress_bc2_to_rgba,
            DdsFormat::Bc3Dxt5 => decompress_bc3_to_rgba,
            // Alpha-only textures are common for particle masks.
            // Convert to white RGB + alpha so shader tinting remains visible.
            DdsFormat::A8 => convert_a8_to_rgba,
            // Luminance-only textures map luminance to RGB with full alpha.
            DdsFormat::L8 => convert_l8_to_rgba,
            // Luminance+alpha textures are often used by legacy VFX masks.
            DdsFormat::L8A8 => convert_l8a8_to_rgba,
            DdsFormat::Bc4 | DdsFormat::Bc5 | DdsFormat::Bc6H | DdsFormat::Bc7 => {
                // No decoder here, and the image crate fallback only decodes
                // DXT1/3/5: reject instead of misreading.
                anyhow::bail!(
                    "unsupported DDS format {:?} (only BC1-BC3 block compression is supported)",
                    info.format
                );
            }
            _ => {
                // Try image crate as fallback - it will also convert to RGBA8
                warn!("[DDS LOADER] Format {:?}, trying image crate", info.format);
                return try_image_crate(bytes, asset_path, is_cube);
            }
        };
        decode_mip_chain(bytes, &info, mip_levels, is_cube, decode_level)
    }
}

impl AssetLoader for DdsImageLoader {
    type Asset = Image;
    type Settings = ();
    type Error = anyhow::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move {
            let asset_path = load_context.path().path().to_string_lossy().to_string();
            let is_cube = load_context.path().label() == Some("cube");

            // Read all bytes from the reader
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let mut image = self.load_image(&bytes, &asset_path, is_cube)?;
            let wrap =
                !is_cube && !is_ui_texture(&asset_path) && !is_lightmap_texture(&asset_path);
            image.sampler = self.sampler(image.texture_descriptor.mip_level_count, wrap);
            Ok(image)
        }
    }

    fn extensions(&self) -> &[&str] {
        // Use uppercase .DDS extension to avoid conflict with Bevy's built-in image loader
        // which handles lowercase .dds through the image crate
        &["DDS", "dds"]
    }
}

#[derive(Debug, Clone, Copy)]
enum DdsFormat {
    Unknown,
    R8G8B8,            // 24-bit RGB
    R8G8B8A8,          // 32-bit RGBA
    B8G8R8,            // 24-bit BGR
    B8G8R8A8,          // 32-bit BGRA
    R8G8B8X8,          // 32-bit RGB + unused byte (no alpha)
    B8G8R8X8,          // 32-bit BGR + unused byte (no alpha)
    R5G6B5,            // 16-bit RGB (Rose Online format)
    B5G6R5,            // 16-bit RGB
    B5G5R5A1,          // 16-bit RGBA
    B4G4R4A4,          // 16-bit RGBA
    A4R4G4B4,          // 16-bit RGBA (Rose Online format)
    A1R5G5B5,          // 16-bit RGBA (Rose Online format)
    L8,                // 8-bit luminance
    A8,                // 8-bit alpha
    L8A8,              // 16-bit luminance + alpha
    R32G32B32A32Float, // 128-bit float RGBA (DX10 header only)
    Bc1Dxt1,           // BC1 / DXT1
    Bc2Dxt3,           // BC2 / DXT3
    Bc3Dxt5,           // BC3 / DXT5
    Bc4,               // BC4 (ATI1)
    Bc5,               // BC5 (ATI2/3Dc)
    Bc6H,              // BC6H
    Bc7,               // BC7
}

struct DdsInfo {
    width: u32,
    height: u32,
    depth: u32,
    /// Mip levels the file provides (see [`usable_mip_count`]), at least 1.
    mip_count: u32,
    format: DdsFormat,
    data_offset: usize,
}

/// Largest accepted width/height. Nothing bigger is a real texture (GPUs cap 2D
/// textures at 16384), and the bound keeps the size arithmetic from overflowing.
const MAX_DDS_DIMENSION: u32 = 32768;

fn parse_dds_header(bytes: &[u8]) -> anyhow::Result<DdsInfo> {
    use std::io::{Cursor, Read};

    if bytes.len() < 128 {
        anyhow::bail!("DDS file too small");
    }

    let mut cursor = Cursor::new(bytes);

    // Read magic
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    if &magic != b"DDS " {
        anyhow::bail!("Invalid DDS magic");
    }

    // Read DDS_HEADER
    let mut header = [0u8; 124];
    cursor.read_exact(&mut header)?;

    let size = read_u32(&header, 0);
    if size != 124 {
        anyhow::bail!("Invalid DDS header size: {}", size);
    }

    let flags = read_u32(&header, 4);
    let height = read_u32(&header, 8);
    let width = read_u32(&header, 12);
    if width == 0 || height == 0 || width > MAX_DDS_DIMENSION || height > MAX_DDS_DIMENSION {
        anyhow::bail!("Invalid DDS size: {}x{}", width, height);
    }
    let _pitch_or_linear_size = read_u32(&header, 16);
    let depth = read_u32(&header, 20);
    let mip_map_count = if flags & 0x20000 != 0 {
        // DDSD_MIPMAPCOUNT
        read_u32(&header, 24)
    } else {
        1
    };

    // Pixel format at offset 76
    let _pf_size = read_u32(&header, 72);
    let pf_flags = read_u32(&header, 76);
    let mut pf_four_cc = [0u8; 4];
    pf_four_cc.copy_from_slice(&header[80..84]);
    let pf_rgb_bit_count = read_u32(&header, 84);
    let pf_r_bit_mask = read_u32(&header, 88);
    let pf_g_bit_mask = read_u32(&header, 92);
    let pf_b_bit_mask = read_u32(&header, 96);
    let pf_a_bit_mask = read_u32(&header, 100);

    // Caps at offset 104
    let _caps = read_u32(&header, 104);
    let caps2 = read_u32(&header, 108);
    let _is_cubemap = (caps2 & 0xFE00) != 0;
    // DDSCAPS2_VOLUME
    let is_volume = (caps2 & 0x200000) != 0;

    // Determine format
    let format = if pf_flags & 0x4 != 0 {
        // DDPF_FOURCC
        match &pf_four_cc {
            b"DXT1" | b"BC1\0" => DdsFormat::Bc1Dxt1,
            b"DXT2" | b"DXT3" | b"BC2\0" => DdsFormat::Bc2Dxt3,
            b"DXT4" | b"DXT5" | b"BC3\0" => DdsFormat::Bc3Dxt5,
            b"ATI1" | b"BC4U" | b"BC4\0" => DdsFormat::Bc4,
            b"ATI2" | b"BC5U" | b"BC5\0" => DdsFormat::Bc5,
            b"DX10" => {
                // DX10 extended header - need to parse more
                return parse_dx10_header(bytes, width, height, depth, mip_map_count);
            }
            _ => {
                warn!(
                    "[DDS LOADER] Unknown FourCC: {:?}",
                    std::str::from_utf8(&pf_four_cc)
                );
                DdsFormat::Unknown
            }
        }
    } else if pf_flags & 0x40 != 0 {
        // DDPF_RGB
        if pf_rgb_bit_count == 24 {
            if pf_r_bit_mask == 0xFF0000 && pf_g_bit_mask == 0x00FF00 && pf_b_bit_mask == 0x0000FF {
                DdsFormat::B8G8R8
            } else {
                DdsFormat::R8G8B8
            }
        } else if pf_rgb_bit_count == 32 {
            // Without DDPF_ALPHAPIXELS (X8R8G8B8 / X8B8G8R8) the fourth byte is
            // undefined, not alpha: those textures are opaque.
            let has_alpha = pf_flags & 0x1 != 0 && pf_a_bit_mask != 0;
            let is_bgr =
                pf_r_bit_mask == 0xFF0000 && pf_g_bit_mask == 0x00FF00 && pf_b_bit_mask == 0x0000FF;
            match (is_bgr, has_alpha) {
                (true, true) => DdsFormat::B8G8R8A8,
                (true, false) => DdsFormat::B8G8R8X8,
                (false, true) => DdsFormat::R8G8B8A8,
                (false, false) => DdsFormat::R8G8B8X8,
            }
        } else if pf_rgb_bit_count == 16 {
            if pf_r_bit_mask == 0x7C00
                && pf_g_bit_mask == 0x03E0
                && pf_b_bit_mask == 0x001F
                && pf_a_bit_mask == 0x8000
            {
                DdsFormat::A1R5G5B5
            } else if pf_r_bit_mask == 0xF800 && pf_g_bit_mask == 0x07E0 && pf_b_bit_mask == 0x001F
            {
                DdsFormat::R5G6B5
            } else if pf_r_bit_mask == 0x0F00
                && pf_g_bit_mask == 0x00F0
                && pf_b_bit_mask == 0x000F
                && pf_a_bit_mask == 0xF000
            {
                DdsFormat::A4R4G4B4
            } else if pf_r_bit_mask == 0x001F && pf_g_bit_mask == 0x07E0 && pf_b_bit_mask == 0xF800
            {
                DdsFormat::B5G6R5
            } else {
                DdsFormat::Unknown
            }
        } else {
            DdsFormat::Unknown
        }
    } else if pf_flags & 0x200 != 0 {
        // DDPF_ALPHA
        DdsFormat::A8
    } else if pf_flags & 0x20000 != 0 {
        // DDPF_LUMINANCE
        if pf_rgb_bit_count == 8 {
            DdsFormat::L8
        } else if pf_rgb_bit_count == 16 {
            DdsFormat::L8A8
        } else {
            DdsFormat::Unknown
        }
    } else {
        DdsFormat::Unknown
    };

    let data_offset = cursor.position() as usize;

    Ok(DdsInfo {
        width,
        height,
        depth: if depth == 0 { 1 } else { depth },
        mip_count: usable_mip_count(width, height, mip_map_count, is_volume),
        format,
        data_offset,
    })
}

fn parse_dx10_header(
    bytes: &[u8],
    width: u32,
    height: u32,
    depth: u32,
    mip_count: u32,
) -> anyhow::Result<DdsInfo> {
    if bytes.len() < 128 + 20 {
        anyhow::bail!("DDS DX10 file too small");
    }

    // DX10 header starts at offset 128
    let dx10_header = &bytes[128..148];
    let dxgi_format = read_u32(dx10_header, 0);
    let resource_dimension = read_u32(dx10_header, 4);
    let misc_flag = read_u32(dx10_header, 8);
    let _array_size = read_u32(dx10_header, 12);
    let _misc_flags2 = read_u32(dx10_header, 16);

    // DXGI_FORMAT values; anything else is Unknown (image crate fallback).
    let format = match dxgi_format {
        2 => DdsFormat::R32G32B32A32Float, // DXGI_FORMAT_R32G32B32A32_FLOAT
        28 => DdsFormat::R8G8B8A8,         // DXGI_FORMAT_R8G8B8A8_UNORM
        29 => DdsFormat::R8G8B8A8,         // DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
        71 => DdsFormat::Bc1Dxt1,          // DXGI_FORMAT_BC1_UNORM
        72 => DdsFormat::Bc1Dxt1,          // DXGI_FORMAT_BC1_UNORM_SRGB
        74 => DdsFormat::Bc2Dxt3,          // DXGI_FORMAT_BC2_UNORM
        75 => DdsFormat::Bc2Dxt3,          // DXGI_FORMAT_BC2_UNORM_SRGB
        77 => DdsFormat::Bc3Dxt5,          // DXGI_FORMAT_BC3_UNORM
        78 => DdsFormat::Bc3Dxt5,          // DXGI_FORMAT_BC3_UNORM_SRGB
        80 => DdsFormat::Bc4,              // DXGI_FORMAT_BC4_UNORM
        81 => DdsFormat::Bc4,              // DXGI_FORMAT_BC4_SNORM
        83 => DdsFormat::Bc5,              // DXGI_FORMAT_BC5_UNORM
        84 => DdsFormat::Bc5,              // DXGI_FORMAT_BC5_SNORM
        95 => DdsFormat::Bc6H,             // DXGI_FORMAT_BC6H_UF16
        96 => DdsFormat::Bc6H,             // DXGI_FORMAT_BC6H_SF16
        98 => DdsFormat::Bc7,              // DXGI_FORMAT_BC7_UNORM
        99 => DdsFormat::Bc7,              // DXGI_FORMAT_BC7_UNORM_SRGB
        _ => {
            warn!("[DDS LOADER] Unknown DX10 DXGI format: {}", dxgi_format);
            DdsFormat::Unknown
        }
    };

    let _is_cubemap = (misc_flag & 0x4) != 0;
    // D3D10_RESOURCE_DIMENSION_TEXTURE3D
    let is_volume = resource_dimension == 4;

    Ok(DdsInfo {
        width,
        height,
        depth: if depth == 0 { 1 } else { depth },
        mip_count: usable_mip_count(width, height, mip_count, is_volume),
        format,
        data_offset: 148, // After standard header + DX10 header
    })
}

/// Mip levels the file provides as this loader reads it: the header's count (0
/// means 1), capped at a full chain for the size (wgpu rejects more). Volume
/// textures keep only level 0: each of their levels holds every depth slice and
/// only slice 0 is read.
fn usable_mip_count(width: u32, height: u32, header_mip_count: u32, is_volume: bool) -> u32 {
    if is_volume {
        return 1;
    }
    let full_chain = 32 - width.max(height).leading_zeros();
    header_mip_count.clamp(1, full_chain.max(1))
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// UI textures live under `3ddata/control/`.
fn is_ui_texture(asset_path: &str) -> bool {
    asset_path
        .replace('\\', "/")
        .to_ascii_lowercase()
        .contains("3ddata/control/")
}

/// Zone lightmap pages live in each block's `LIGHTMAP/` folder.
fn is_lightmap_texture(asset_path: &str) -> bool {
    asset_path
        .replace('\\', "/")
        .to_ascii_lowercase()
        .contains("/lightmap/")
}

/// Mip levels to load. Kept single-level:
/// - cubemaps (`#cube`, the `EnvironmentMapLight`): the environment map shader
///   picks the specular level from roughness * (level count - 1), so a mip chain
///   would change the lighting;
/// - UI textures: the UI premultiplies only level 0 on the CPU and draws them 1:1
///   (the original client also creates its UI image textures without mips).
fn wanted_mip_levels(info: &DdsInfo, is_cube: bool, is_ui: bool) -> u32 {
    if is_cube || is_ui {
        1
    } else {
        info.mip_count
    }
}

/// Size of mip `level`: halved per level, never below 1.
fn mip_dimensions(info: &DdsInfo, level: u32) -> (u32, u32) {
    ((info.width >> level).max(1), (info.height >> level).max(1))
}

/// Size in the file of one `width` x `height` mip level, or `None` for formats
/// without a decoder here. BC levels are whole 4x4 blocks (at least one).
fn level_byte_size(format: DdsFormat, width: u32, height: u32) -> Option<usize> {
    let pixels = width as usize * height as usize;
    let blocks = (width as usize).div_ceil(4) * (height as usize).div_ceil(4);
    Some(match format {
        DdsFormat::Bc1Dxt1 => blocks * 8,
        DdsFormat::Bc2Dxt3 | DdsFormat::Bc3Dxt5 => blocks * 16,
        DdsFormat::A8 | DdsFormat::L8 => pixels,
        DdsFormat::R5G6B5
        | DdsFormat::B5G6R5
        | DdsFormat::A1R5G5B5
        | DdsFormat::A4R4G4B4
        | DdsFormat::L8A8 => pixels * 2,
        DdsFormat::R8G8B8 | DdsFormat::B8G8R8 => pixels * 3,
        DdsFormat::R8G8B8A8
        | DdsFormat::B8G8R8A8
        | DdsFormat::R8G8B8X8
        | DdsFormat::B8G8R8X8 => pixels * 4,
        DdsFormat::R32G32B32A32Float => pixels * 16,
        _ => return None,
    })
}

/// How many of the first `wanted` mip levels are fully present in a file of
/// `file_len` bytes (0 if level 0 is truncated or the format has no decoder).
fn complete_mip_levels(file_len: usize, info: &DdsInfo, wanted: u32) -> u32 {
    let mut end = info.data_offset;
    for level in 0..wanted {
        let (width, height) = mip_dimensions(info, level);
        let Some(level_size) = level_byte_size(info.format, width, height) else {
            return 0;
        };
        end = end.saturating_add(level_size);
        if end > file_len {
            return level;
        }
    }
    wanted
}

/// Decodes one mip level to RGBA8: `(level data, width, height)`. The data starts
/// at the level's first byte and may be shorter than the level if level 0 is
/// truncated.
type DecodeLevelFn = fn(&[u8], u32, u32) -> anyhow::Result<Vec<u8>>;

/// Decodes `mip_levels` levels, stored one after another from `info.data_offset`
/// (level 0 first), to RGBA8 and builds the image.
fn decode_mip_chain(
    bytes: &[u8],
    info: &DdsInfo,
    mip_levels: u32,
    is_cube: bool,
    decode_level: DecodeLevelFn,
) -> anyhow::Result<Image> {
    let mut rgba_data: Vec<u8> = Vec::with_capacity(
        (0..mip_levels)
            .map(|level| {
                let (width, height) = mip_dimensions(info, level);
                width as usize * height as usize * 4
            })
            .sum(),
    );
    let mut offset = info.data_offset;
    for level in 0..mip_levels {
        let (width, height) = mip_dimensions(info, level);
        let Some(level_size) = level_byte_size(info.format, width, height) else {
            anyhow::bail!("No mip level size for DDS format {:?}", info.format);
        };
        let start = offset.min(bytes.len());
        let end = offset.saturating_add(level_size).min(bytes.len());
        rgba_data.extend_from_slice(&decode_level(&bytes[start..end], width, height)?);
        offset = offset.saturating_add(level_size);
    }

    Ok(create_rgba_image(
        info.width,
        info.height,
        mip_levels,
        rgba_data,
        is_cube,
    ))
}

/// Builds an RGBA8 sRGB image from `rgba_data` holding `mip_levels` levels, level 0
/// first. For `#cube` the data is repeated for the 6 faces: the default
/// `TextureDataOrder::LayerMajor` expects each layer's whole mip chain in turn.
fn create_rgba_image(
    width: u32,
    height: u32,
    mip_levels: u32,
    rgba_data: Vec<u8>,
    is_cube: bool,
) -> Image {
    // Not `Image::new`: it debug-asserts that the data is exactly one level.
    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: if is_cube { 6 } else { 1 },
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.data = Some(if is_cube {
        rgba_data.repeat(6)
    } else {
        rgba_data
    });
    image.texture_descriptor.mip_level_count = mip_levels;
    if is_cube {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });
    }
    image
}

/// Builds an image from BC1/BC2/BC3 block data without decoding it.
///
/// The `mip_levels` levels are consecutive in the file, each stored as whole 4x4
/// blocks (one block for levels smaller than 4x4). That is the layout wgpu expects
/// for compressed mips: it uploads each level at its block-padded "physical" size.
/// Returns `None` if the file is too short for them.
fn load_bc_direct(
    bytes: &[u8],
    info: &DdsInfo,
    format: TextureFormat,
    mip_levels: u32,
) -> Option<Image> {
    let mut data_size = 0usize;
    for level in 0..mip_levels {
        let (width, height) = mip_dimensions(info, level);
        data_size += level_byte_size(info.format, width, height)?;
    }
    let data = bytes
        .get(info.data_offset..info.data_offset + data_size)?
        .to_vec();

    let mut image = Image::new(
        Extent3d {
            width: info.width,
            height: info.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = mip_levels;
    Some(image)
}

fn convert_rgb_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 3;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for RGB conversion");
    }

    let rgb_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let offset = i * 3;
        rgba_data.push(rgb_data[offset]); // R
        rgba_data.push(rgb_data[offset + 1]); // G
        rgba_data.push(rgb_data[offset + 2]); // B
        rgba_data.push(255); // A (fully opaque)
    }

    Ok(rgba_data)
}

fn convert_bgr_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 3;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for BGR conversion");
    }

    let bgr_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let offset = i * 3;
        rgba_data.push(bgr_data[offset + 2]); // R (from B)
        rgba_data.push(bgr_data[offset + 1]); // G
        rgba_data.push(bgr_data[offset]); // B (from R)
        rgba_data.push(255); // A (fully opaque)
    }

    Ok(rgba_data)
}

fn copy_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let expected_size = width as usize * height as usize * 4;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for RGBA");
    }

    Ok(src[..expected_size].to_vec())
}

/// RGBX: copy as RGBA, then make it opaque (the X byte is undefined).
fn convert_rgbx_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let mut rgba = copy_rgba(src, width, height)?;
    rgba.chunks_exact_mut(4).for_each(|pixel| pixel[3] = 255);
    Ok(rgba)
}

/// BGRX: swizzle as BGRA, then make it opaque (the X byte is undefined).
fn convert_bgrx_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let mut rgba = convert_bgra_to_rgba(src, width, height)?;
    rgba.chunks_exact_mut(4).for_each(|pixel| pixel[3] = 255);
    Ok(rgba)
}

fn convert_bgra_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 4;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for BGRA conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(expected_size);

    for i in 0..num_pixels {
        let offset = i * 4;
        rgba_data.push(src_data[offset + 2]); // R
        rgba_data.push(src_data[offset + 1]); // G
        rgba_data.push(src_data[offset]); // B
        rgba_data.push(src_data[offset + 3]); // A
    }

    Ok(rgba_data)
}

/// DXGI_FORMAT_R32G32B32A32_FLOAT: four little-endian f32 per pixel, linear.
fn convert_rgba32f_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 16;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for R32G32B32A32_FLOAT conversion");
    }

    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for pixel in src[..expected_size].chunks_exact(16) {
        let channel = |i: usize| {
            f32::from_le_bytes([
                pixel[i * 4],
                pixel[i * 4 + 1],
                pixel[i * 4 + 2],
                pixel[i * 4 + 3],
            ])
        };
        // RGB is sRGB-encoded for the sRGB texture (sampling decodes it back to the
        // linear value); alpha stays linear. Values outside 0..=1 are clamped.
        rgba_data.push(linear_to_srgb8(channel(0)));
        rgba_data.push(linear_to_srgb8(channel(1)));
        rgba_data.push(linear_to_srgb8(channel(2)));
        rgba_data.push((channel(3).clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
    }

    Ok(rgba_data)
}

/// Encodes a linear channel value (clamped to 0..=1) as 8-bit sRGB.
fn linear_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0 + 0.5) as u8
}

fn convert_a8_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for A8 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for &a in src_data {
        // White RGB + source alpha allows particle color keyframes to tint correctly.
        rgba_data.push(255);
        rgba_data.push(255);
        rgba_data.push(255);
        rgba_data.push(a);
    }

    Ok(rgba_data)
}

fn convert_l8_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for L8 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for &luma in src_data {
        rgba_data.push(luma);
        rgba_data.push(luma);
        rgba_data.push(luma);
        rgba_data.push(255);
    }

    Ok(rgba_data)
}

fn convert_l8a8_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 2;

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for L8A8 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let luma = src_data[i * 2];
        let alpha = src_data[i * 2 + 1];

        rgba_data.push(luma);
        rgba_data.push(luma);
        rgba_data.push(luma);
        rgba_data.push(alpha);
    }

    Ok(rgba_data)
}

fn convert_a1r5g5b5_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 2; // 16 bits per pixel

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for A1R5G5B5 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let pixel = u16::from_le_bytes([src_data[i * 2], src_data[i * 2 + 1]]);

        // Extract components from A1R5G5B5 format
        // Bit layout: A1R5G5B5
        // Alpha: 1 bit (bit 15)
        // Red: 5 bits (bits 10-14)
        // Green: 5 bits (bits 5-9)
        // Blue: 5 bits (bits 0-4)

        let a = if (pixel & 0x8000) != 0 { 255 } else { 0 };
        let r = ((pixel >> 10) & 0x1F) as u8;
        let g = ((pixel >> 5) & 0x1F) as u8;
        let b = (pixel & 0x1F) as u8;

        // Expand 5-bit to 8-bit
        rgba_data.push((r << 3) | (r >> 2));
        rgba_data.push((g << 3) | (g >> 2));
        rgba_data.push((b << 3) | (b >> 2));
        rgba_data.push(a);
    }

    Ok(rgba_data)
}

fn convert_b5g6r5_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 2; // 16 bits per pixel

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for B5G6R5 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let pixel = u16::from_le_bytes([src_data[i * 2], src_data[i * 2 + 1]]);

        // Extract components from B5G6R5 format (selected for red mask 0x001F,
        // blue mask 0xF800)
        // Blue: 5 bits (bits 11-15)
        // Green: 6 bits (bits 5-10)
        // Red: 5 bits (bits 0-4)

        let b = ((pixel >> 11) & 0x1F) as u8;
        let g = ((pixel >> 5) & 0x3F) as u8;
        let r = (pixel & 0x1F) as u8;

        // Expand to 8-bit
        // For 5-bit: (value << 3) | (value >> 2)
        // For 6-bit: (value << 2) | (value >> 4)
        rgba_data.push((r << 3) | (r >> 2));
        rgba_data.push((g << 2) | (g >> 4));
        rgba_data.push((b << 3) | (b >> 2));
        rgba_data.push(255); // A (fully opaque - no alpha channel)
    }

    Ok(rgba_data)
}

fn convert_r5g6b5_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 2; // 16 bits per pixel

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for R5G6B5 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let pixel = u16::from_le_bytes([src_data[i * 2], src_data[i * 2 + 1]]);

        // Extract components from R5G6B5 format
        // Bit layout: R5G6B5
        // Red: 5 bits (bits 11-15)
        // Green: 6 bits (bits 5-10)
        // Blue: 5 bits (bits 0-4)
        // No alpha channel (set to 255)

        let r = ((pixel >> 11) & 0x1F) as u8;
        let g = ((pixel >> 5) & 0x3F) as u8;
        let b = (pixel & 0x1F) as u8;

        // Expand to 8-bit
        // For 5-bit: (value << 3) | (value >> 2)
        // For 6-bit: (value << 2) | (value >> 4)
        rgba_data.push((r << 3) | (r >> 2));
        rgba_data.push((g << 2) | (g >> 4));
        rgba_data.push((b << 3) | (b >> 2));
        rgba_data.push(255); // A (fully opaque - no alpha channel)
    }

    Ok(rgba_data)
}

fn convert_a4r4g4b4_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let num_pixels = width as usize * height as usize;
    let expected_size = num_pixels * 2; // 16 bits per pixel

    if src.len() < expected_size {
        anyhow::bail!("Not enough data for A4R4G4B4 conversion");
    }

    let src_data = &src[..expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);

    for i in 0..num_pixels {
        let pixel = u16::from_le_bytes([src_data[i * 2], src_data[i * 2 + 1]]);

        // Extract components from A4R4G4B4 format
        // Bit layout: A4R4G4B4
        // Alpha: 4 bits (bits 12-15)
        // Red: 4 bits (bits 8-11)
        // Green: 4 bits (bits 4-7)
        // Blue: 4 bits (bits 0-3)

        let a = ((pixel >> 12) & 0x0F) as u8;
        let r = ((pixel >> 8) & 0x0F) as u8;
        let g = ((pixel >> 4) & 0x0F) as u8;
        let b = (pixel & 0x0F) as u8;

        // Expand 4-bit to 8-bit
        rgba_data.push((r << 4) | r);
        rgba_data.push((g << 4) | g);
        rgba_data.push((b << 4) | b);
        rgba_data.push((a << 4) | a);
    }

    Ok(rgba_data)
}

// DXT decompression functions (one mip level each)
// BC1/DXT1: 8 bytes per 4x4 block (64 bits)
// Color0 and Color1 are 16-bit RGB565 values
// If color0 > color1: 4 color block, else: 3 color + transparent
// Blocks missing from truncated data are left transparent black.

fn decompress_bc1_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let block_count_x = width.div_ceil(4) as usize;
    let block_count_y = height.div_ceil(4) as usize;
    let block_size = 8; // BC1 uses 8 bytes per block

    let mut rgba_data = vec![0u8; width as usize * height as usize * 4];

    for by in 0..block_count_y {
        for bx in 0..block_count_x {
            let block_offset = (by * block_count_x + bx) * block_size;
            if block_offset + block_size > src.len() {
                break;
            }

            let color0 = u16::from_le_bytes([src[block_offset], src[block_offset + 1]]);
            let color1 = u16::from_le_bytes([src[block_offset + 2], src[block_offset + 3]]);
            let lookup = u32::from_le_bytes([
                src[block_offset + 4],
                src[block_offset + 5],
                src[block_offset + 6],
                src[block_offset + 7],
            ]);

            let colors = decode_bc1_colors(color0, color1);

            // Decode 4x4 block
            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px) as u32;
                    let y = (by * 4 + py) as u32;
                    if x < width && y < height {
                        let idx = ((py * 4 + px) * 2) as u32;
                        let color_idx = ((lookup >> idx) & 3) as usize;
                        let pixel_offset = (y as usize * width as usize + x as usize) * 4;
                        rgba_data[pixel_offset..pixel_offset + 4]
                            .copy_from_slice(&colors[color_idx]);
                    }
                }
            }
        }
    }

    Ok(rgba_data)
}

fn decode_bc1_colors(color0: u16, color1: u16) -> [[u8; 4]; 4] {
    let mut colors = [[0u8; 4]; 4];

    // Color 0
    colors[0] = rgb565_to_rgba8(color0, 255);
    // Color 1
    colors[1] = rgb565_to_rgba8(color1, 255);

    if color0 > color1 {
        // 4 color mode
        colors[2] = interpolate_color(colors[0], colors[1], 1, 2);
        colors[3] = interpolate_color(colors[0], colors[1], 2, 1);
    } else {
        // 3 color + transparent mode
        colors[2] = interpolate_color(colors[0], colors[1], 1, 1);
        colors[3] = [0, 0, 0, 0]; // Transparent
    }

    colors
}

fn rgb565_to_rgba8(color: u16, alpha: u8) -> [u8; 4] {
    let r = ((color >> 11) & 0x1F) as u8;
    let g = ((color >> 5) & 0x3F) as u8;
    let b = (color & 0x1F) as u8;

    // Expand to 8-bit
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        alpha,
    ]
}

fn interpolate_color(c1: [u8; 4], c2: [u8; 4], w1: u8, w2: u8) -> [u8; 4] {
    [
        ((c1[0] as u16 * w1 as u16 + c2[0] as u16 * w2 as u16) / (w1 + w2) as u16) as u8,
        ((c1[1] as u16 * w1 as u16 + c2[1] as u16 * w2 as u16) / (w1 + w2) as u16) as u8,
        ((c1[2] as u16 * w1 as u16 + c2[2] as u16 * w2 as u16) / (w1 + w2) as u16) as u8,
        255,
    ]
}

// BC2/DXT3: 16 bytes per 4x4 block
// First 8 bytes: explicit alpha (4 bits per pixel)
// Last 8 bytes: same color encoding as BC1
fn decompress_bc2_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let block_count_x = width.div_ceil(4) as usize;
    let block_count_y = height.div_ceil(4) as usize;
    let block_size = 16; // BC2 uses 16 bytes per block

    let mut rgba_data = vec![0u8; width as usize * height as usize * 4];

    for by in 0..block_count_y {
        for bx in 0..block_count_x {
            let block_offset = (by * block_count_x + bx) * block_size;
            if block_offset + block_size > src.len() {
                break;
            }

            // Read alpha (4 bits per pixel, 16 pixels = 64 bits = 8 bytes)
            let mut alpha = [0u8; 16];
            for i in 0..8 {
                let byte = src[block_offset + i];
                alpha[i * 2] = (byte & 0x0F) * 17; // Expand 4-bit to 8-bit
                alpha[i * 2 + 1] = ((byte >> 4) & 0x0F) * 17;
            }

            // Read color (same as BC1)
            let color0 = u16::from_le_bytes([src[block_offset + 8], src[block_offset + 9]]);
            let color1 = u16::from_le_bytes([src[block_offset + 10], src[block_offset + 11]]);
            let lookup = u32::from_le_bytes([
                src[block_offset + 12],
                src[block_offset + 13],
                src[block_offset + 14],
                src[block_offset + 15],
            ]);

            let colors = decode_bc1_colors(color0, color1);

            // Decode 4x4 block
            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px) as u32;
                    let y = (by * 4 + py) as u32;
                    if x < width && y < height {
                        let idx = (py * 4 + px) as usize;
                        let color_idx = ((lookup >> (idx * 2)) & 3) as usize;
                        let pixel_offset = (y as usize * width as usize + x as usize) * 4;
                        rgba_data[pixel_offset..pixel_offset + 3]
                            .copy_from_slice(&colors[color_idx][0..3]);
                        rgba_data[pixel_offset + 3] = alpha[idx];
                    }
                }
            }
        }
    }

    Ok(rgba_data)
}

// BC3/DXT5: 16 bytes per 4x4 block
// First 8 bytes: interpolated alpha (similar to BC1 color)
// Last 8 bytes: same color encoding as BC1
fn decompress_bc3_to_rgba(src: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let block_count_x = width.div_ceil(4) as usize;
    let block_count_y = height.div_ceil(4) as usize;
    let block_size = 16; // BC3 uses 16 bytes per block

    let mut rgba_data = vec![0u8; width as usize * height as usize * 4];

    for by in 0..block_count_y {
        for bx in 0..block_count_x {
            let block_offset = (by * block_count_x + bx) * block_size;
            if block_offset + block_size > src.len() {
                break;
            }

            // Read alpha lookup table
            let alpha0 = src[block_offset];
            let alpha1 = src[block_offset + 1];
            let alpha_lookup = u64::from_le_bytes([
                src[block_offset + 2],
                src[block_offset + 3],
                src[block_offset + 4],
                src[block_offset + 5],
                src[block_offset + 6],
                src[block_offset + 7],
                0,
                0,
            ]) & 0xFFFFFFFFFFFF; // 48 bits

            // Build alpha table
            let mut alpha_table = [0u8; 8];
            alpha_table[0] = alpha0;
            alpha_table[1] = alpha1;
            if alpha0 > alpha1 {
                // 8 alpha values
                for i in 2..8 {
                    alpha_table[i] = (((8 - i) as u16 * alpha0 as u16
                        + (i - 1) as u16 * alpha1 as u16)
                        / 7 as u16) as u8;
                }
            } else {
                // 6 alpha values + 0 + 255
                for i in 2..6 {
                    alpha_table[i] = (((6 - i) as u16 * alpha0 as u16
                        + (i - 1) as u16 * alpha1 as u16)
                        / 5 as u16) as u8;
                }
                alpha_table[6] = 0;
                alpha_table[7] = 255;
            }

            // Read color (same as BC1)
            let color0 = u16::from_le_bytes([src[block_offset + 8], src[block_offset + 9]]);
            let color1 = u16::from_le_bytes([src[block_offset + 10], src[block_offset + 11]]);
            let lookup = u32::from_le_bytes([
                src[block_offset + 12],
                src[block_offset + 13],
                src[block_offset + 14],
                src[block_offset + 15],
            ]);

            let colors = decode_bc1_colors(color0, color1);

            // Decode 4x4 block
            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px) as u32;
                    let y = (by * 4 + py) as u32;
                    if x < width && y < height {
                        let idx = (py * 4 + px) as usize;
                        let color_idx = ((lookup >> (idx * 2)) & 3) as usize;
                        let alpha_idx = ((alpha_lookup >> (idx * 3)) & 7) as usize;
                        let pixel_offset = (y as usize * width as usize + x as usize) * 4;
                        rgba_data[pixel_offset..pixel_offset + 3]
                            .copy_from_slice(&colors[color_idx][0..3]);
                        rgba_data[pixel_offset + 3] = alpha_table[alpha_idx];
                    }
                }
            }
        }
    }

    Ok(rgba_data)
}

fn try_image_crate(bytes: &[u8], asset_path: &str, is_cube: bool) -> anyhow::Result<Image> {
    use image::ImageFormat;

    match image::load_from_memory_with_format(bytes, ImageFormat::Dds) {
        Ok(dynamic_image) => {
            let rgba_image = dynamic_image.to_rgba8();
            let (width, height) = rgba_image.dimensions();

            Ok(create_rgba_image(
                width,
                height,
                1,
                rgba_image.into_raw(),
                is_cube,
            ))
        }
        Err(e) => {
            anyhow::bail!("Image crate failed to load {}: {:?}", asset_path, e)
        }
    }
}
