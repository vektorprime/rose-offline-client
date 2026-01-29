use bevy::{
    asset::{io::Reader, AssetLoader, BoxedFuture, LoadContext},
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::Image,
    },
    tasks::futures_lite::AsyncReadExt,
};
use log::{info, warn};

/// Custom asset loader for DDS files that handles unsupported formats like R8G8B8
/// by converting them to formats Bevy can render (R8G8B8A8).
/// 
/// NOTE: All output is converted to R8G8B8A8 to avoid Bevy 0.13.2 issues with
/// compressed texture pixel_size calculations that cause panics.
#[derive(Default)]
pub struct DdsImageLoader;

impl AssetLoader for DdsImageLoader {
    type Asset = Image;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a, 'b>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'b>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let asset_path = load_context.path().to_string_lossy().to_string();
            
            // Read all bytes from the reader
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            
            info!("[DDS LOADER] Loading DDS texture: {}", asset_path);
            info!("[DDS LOADER] File size: {} bytes", bytes.len());
            
            // Parse the DDS header to determine format
            let dds_info = parse_dds_header(&bytes)?;
            info!("[DDS LOADER] DDS format: {:?}, {}x{}, mips: {}", 
                dds_info.format, dds_info.width, dds_info.height, dds_info.mip_count);
            
            // Handle based on format - ALL paths convert to R8G8B8A8
            // This avoids Bevy 0.13.2 panics with compressed texture pixel_size
            match dds_info.format {
                DdsFormat::R8G8B8 => {
                    info!("[DDS LOADER] Converting R8G8B8 to R8G8B8A8");
                    convert_rgb_to_rgba(&bytes, &dds_info)
                }
                DdsFormat::R8G8B8A8 | DdsFormat::B8G8R8A8 => {
                    info!("[DDS LOADER] Loading RGBA data directly");
                    load_rgba_direct(&bytes, &dds_info)
                }
                DdsFormat::B8G8R8 => {
                    info!("[DDS LOADER] Converting B8G8R8 to R8G8B8A8");
                    convert_bgr_to_rgba(&bytes, &dds_info)
                }
                DdsFormat::Bc1Dxt1 => {
                    info!("[DDS LOADER] Decompressing BC1/DXT1 to R8G8B8A8");
                    decompress_bc1_to_rgba(&bytes, &dds_info)
                }
                DdsFormat::Bc2Dxt3 => {
                    info!("[DDS LOADER] Decompressing BC2/DXT3 to R8G8B8A8");
                    decompress_bc2_to_rgba(&bytes, &dds_info)
                }
                DdsFormat::Bc3Dxt5 => {
                    info!("[DDS LOADER] Decompressing BC3/DXT5 to R8G8B8A8");
                    decompress_bc3_to_rgba(&bytes, &dds_info)
                }
                _ => {
                    // Try image crate as fallback - it will also convert to RGBA8
                    warn!("[DDS LOADER] Format {:?}, trying image crate", dds_info.format);
                    try_image_crate(&bytes, &asset_path)
                }
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["dds"]
    }
}

#[derive(Debug, Clone, Copy)]
enum DdsFormat {
    Unknown,
    R8G8B8,       // 24-bit RGB
    R8G8B8A8,     // 32-bit RGBA
    B8G8R8,       // 24-bit BGR
    B8G8R8A8,     // 32-bit BGRA
    B5G6R5,       // 16-bit RGB
    B5G5R5A1,     // 16-bit RGBA
    B4G4R4A4,     // 16-bit RGBA
    L8,           // 8-bit luminance
    A8,           // 8-bit alpha
    L8A8,         // 16-bit luminance + alpha
    Bc1Dxt1,      // BC1 / DXT1
    Bc2Dxt3,      // BC2 / DXT3  
    Bc3Dxt5,      // BC3 / DXT5
    Bc4,          // BC4 (ATI1)
    Bc5,          // BC5 (ATI2/3Dc)
    Bc6H,         // BC6H
    Bc7,          // BC7
}

struct DdsInfo {
    width: u32,
    height: u32,
    depth: u32,
    mip_count: u32,
    format: DdsFormat,
    data_offset: usize,
}

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
    let _pitch_or_linear_size = read_u32(&header, 16);
    let depth = read_u32(&header, 20);
    let mip_map_count = if flags & 0x20000 != 0 { // DDSD_MIPMAPCOUNT
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
    let _pf_a_bit_mask = read_u32(&header, 100);
    
    // Caps at offset 104
    let _caps = read_u32(&header, 104);
    let caps2 = read_u32(&header, 108);
    let _is_cubemap = (caps2 & 0xFE00) != 0;
    
    // Determine format
    let format = if pf_flags & 0x4 != 0 { // DDPF_FOURCC
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
                warn!("[DDS LOADER] Unknown FourCC: {:?}", std::str::from_utf8(&pf_four_cc));
                DdsFormat::Unknown
            }
        }
    } else if pf_flags & 0x40 != 0 { // DDPF_RGB
        if pf_rgb_bit_count == 24 {
            if pf_r_bit_mask == 0xFF0000 && pf_g_bit_mask == 0x00FF00 && pf_b_bit_mask == 0x0000FF {
                DdsFormat::B8G8R8
            } else {
                DdsFormat::R8G8B8
            }
        } else if pf_rgb_bit_count == 32 {
            if pf_r_bit_mask == 0xFF0000 && pf_g_bit_mask == 0x00FF00 && pf_b_bit_mask == 0x0000FF {
                DdsFormat::B8G8R8A8
            } else {
                DdsFormat::R8G8B8A8
            }
        } else if pf_rgb_bit_count == 16 {
            if pf_r_bit_mask == 0xF800 && pf_g_bit_mask == 0x07E0 && pf_b_bit_mask == 0x001F {
                DdsFormat::B5G6R5
            } else {
                DdsFormat::Unknown
            }
        } else {
            DdsFormat::Unknown
        }
    } else if pf_flags & 0x200 != 0 { // DDPF_ALPHA
        DdsFormat::A8
    } else if pf_flags & 0x20000 != 0 { // DDPF_LUMINANCE
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
        mip_count: if mip_map_count == 0 { 1 } else { mip_map_count },
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
    let _resource_dimension = read_u32(dx10_header, 4);
    let misc_flag = read_u32(dx10_header, 8);
    let _array_size = read_u32(dx10_header, 12);
    let _misc_flags2 = read_u32(dx10_header, 16);
    
    let format = match dxgi_format {
        2 => DdsFormat::R8G8B8A8,      // DXGI_FORMAT_R32G32B32A32_FLOAT
        28 => DdsFormat::R8G8B8A8,     // DXGI_FORMAT_R8G8B8A8_UNORM
        29 => DdsFormat::R8G8B8A8,     // DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
        71 => DdsFormat::Bc1Dxt1,      // DXGI_FORMAT_BC1_UNORM
        72 => DdsFormat::Bc1Dxt1,      // DXGI_FORMAT_BC1_UNORM_SRGB
        74 => DdsFormat::Bc2Dxt3,      // DXGI_FORMAT_BC2_UNORM
        75 => DdsFormat::Bc2Dxt3,      // DXGI_FORMAT_BC2_UNORM_SRGB
        77 => DdsFormat::Bc3Dxt5,      // DXGI_FORMAT_BC3_UNORM
        78 => DdsFormat::Bc3Dxt5,      // DXGI_FORMAT_BC3_UNORM_SRGB
        80 => DdsFormat::Bc4,          // DXGI_FORMAT_BC4_UNORM
        81 => DdsFormat::Bc4,          // DXGI_FORMAT_BC4_SNORM
        83 => DdsFormat::Bc5,          // DXGI_FORMAT_BC5_UNORM
        84 => DdsFormat::Bc5,          // DXGI_FORMAT_BC5_SNORM
        95 => DdsFormat::Bc6H,         // DXGI_FORMAT_BC6H_UF16
        96 => DdsFormat::Bc6H,         // DXGI_FORMAT_BC6H_SF16
        98 => DdsFormat::Bc7,          // DXGI_FORMAT_BC7_UNORM
        99 => DdsFormat::Bc7,          // DXGI_FORMAT_BC7_UNORM_SRGB
        _ => {
            warn!("[DDS LOADER] Unknown DX10 DXGI format: {}", dxgi_format);
            DdsFormat::Unknown
        }
    };
    
    let _is_cubemap = (misc_flag & 0x4) != 0;
    
    Ok(DdsInfo {
        width,
        height,
        depth: if depth == 0 { 1 } else { depth },
        mip_count: if mip_count == 0 { 1 } else { mip_count },
        format,
        data_offset: 148, // After standard header + DX10 header
    })
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn create_rgba_image(width: u32, height: u32, rgba_data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn convert_rgb_to_rgba(bytes: &[u8], info: &DdsInfo) -> anyhow::Result<Image> {
    let data_start = info.data_offset;
    let num_pixels = (info.width * info.height) as usize;
    let expected_size = num_pixels * 3;
    
    if bytes.len() < data_start + expected_size {
        anyhow::bail!("Not enough data for RGB conversion");
    }
    
    let rgb_data = &bytes[data_start..data_start + expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);
    
    for i in 0..num_pixels {
        let offset = i * 3;
        rgba_data.push(rgb_data[offset]);      // R
        rgba_data.push(rgb_data[offset + 1]);  // G
        rgba_data.push(rgb_data[offset + 2]);  // B
        rgba_data.push(255);                    // A (fully opaque)
    }
    
    Ok(create_rgba_image(info.width, info.height, rgba_data))
}

fn convert_bgr_to_rgba(bytes: &[u8], info: &DdsInfo) -> anyhow::Result<Image> {
    let data_start = info.data_offset;
    let num_pixels = (info.width * info.height) as usize;
    let expected_size = num_pixels * 3;
    
    if bytes.len() < data_start + expected_size {
        anyhow::bail!("Not enough data for BGR conversion");
    }
    
    let bgr_data = &bytes[data_start..data_start + expected_size];
    let mut rgba_data = Vec::with_capacity(num_pixels * 4);
    
    for i in 0..num_pixels {
        let offset = i * 3;
        rgba_data.push(bgr_data[offset + 2]);  // R (from B)
        rgba_data.push(bgr_data[offset + 1]);  // G
        rgba_data.push(bgr_data[offset]);      // B (from R)
        rgba_data.push(255);                    // A (fully opaque)
    }
    
    Ok(create_rgba_image(info.width, info.height, rgba_data))
}

fn load_rgba_direct(bytes: &[u8], info: &DdsInfo) -> anyhow::Result<Image> {
    let data_start = info.data_offset;
    let expected_size = (info.width * info.height * 4) as usize;
    
    if bytes.len() < data_start + expected_size {
        anyhow::bail!("Not enough data for RGBA");
    }
    
    let rgba_data = bytes[data_start..data_start + expected_size].to_vec();
    Ok(create_rgba_image(info.width, info.height, rgba_data))
}

// DXT decompression functions
// BC1/DXT1: 8 bytes per 4x4 block (64 bits)
// Color0 and Color1 are 16-bit RGB565 values
// If color0 > color1: 4 color block, else: 3 color + transparent

fn decompress_bc1_to_rgba(bytes: &[u8], info: &DdsInfo) -> anyhow::Result<Image> {
    let data_start = info.data_offset;
    let block_count_x = ((info.width + 3) / 4) as usize;
    let block_count_y = ((info.height + 3) / 4) as usize;
    let block_size = 8; // BC1 uses 8 bytes per block
    
    let mut rgba_data = vec![0u8; (info.width * info.height * 4) as usize];
    
    for by in 0..block_count_y {
        for bx in 0..block_count_x {
            let block_offset = data_start + (by * block_count_x + bx) * block_size;
            if block_offset + block_size > bytes.len() {
                break;
            }
            
            let color0 = u16::from_le_bytes([bytes[block_offset], bytes[block_offset + 1]]);
            let color1 = u16::from_le_bytes([bytes[block_offset + 2], bytes[block_offset + 3]]);
            let lookup = u32::from_le_bytes([
                bytes[block_offset + 4],
                bytes[block_offset + 5],
                bytes[block_offset + 6],
                bytes[block_offset + 7],
            ]);
            
            let colors = decode_bc1_colors(color0, color1);
            
            // Decode 4x4 block
            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px) as u32;
                    let y = (by * 4 + py) as u32;
                    if x < info.width && y < info.height {
                        let idx = ((py * 4 + px) * 2) as u32;
                        let color_idx = ((lookup >> idx) & 3) as usize;
                        let pixel_offset = ((y * info.width + x) * 4) as usize;
                        rgba_data[pixel_offset..pixel_offset + 4].copy_from_slice(&colors[color_idx]);
                    }
                }
            }
        }
    }
    
    Ok(create_rgba_image(info.width, info.height, rgba_data))
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
fn decompress_bc2_to_rgba(bytes: &[u8], info: &DdsInfo) -> anyhow::Result<Image> {
    let data_start = info.data_offset;
    let block_count_x = ((info.width + 3) / 4) as usize;
    let block_count_y = ((info.height + 3) / 4) as usize;
    let block_size = 16; // BC2 uses 16 bytes per block
    
    let mut rgba_data = vec![0u8; (info.width * info.height * 4) as usize];
    
    for by in 0..block_count_y {
        for bx in 0..block_count_x {
            let block_offset = data_start + (by * block_count_x + bx) * block_size;
            if block_offset + block_size > bytes.len() {
                break;
            }
            
            // Read alpha (4 bits per pixel, 16 pixels = 64 bits = 8 bytes)
            let mut alpha = [0u8; 16];
            for i in 0..8 {
                let byte = bytes[block_offset + i];
                alpha[i * 2] = (byte & 0x0F) * 17;     // Expand 4-bit to 8-bit
                alpha[i * 2 + 1] = ((byte >> 4) & 0x0F) * 17;
            }
            
            // Read color (same as BC1)
            let color0 = u16::from_le_bytes([bytes[block_offset + 8], bytes[block_offset + 9]]);
            let color1 = u16::from_le_bytes([bytes[block_offset + 10], bytes[block_offset + 11]]);
            let lookup = u32::from_le_bytes([
                bytes[block_offset + 12],
                bytes[block_offset + 13],
                bytes[block_offset + 14],
                bytes[block_offset + 15],
            ]);
            
            let colors = decode_bc1_colors(color0, color1);
            
            // Decode 4x4 block
            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px) as u32;
                    let y = (by * 4 + py) as u32;
                    if x < info.width && y < info.height {
                        let idx = (py * 4 + px) as usize;
                        let color_idx = ((lookup >> (idx * 2)) & 3) as usize;
                        let pixel_offset = ((y * info.width + x) * 4) as usize;
                        rgba_data[pixel_offset..pixel_offset + 3].copy_from_slice(&colors[color_idx][0..3]);
                        rgba_data[pixel_offset + 3] = alpha[idx];
                    }
                }
            }
        }
    }
    
    Ok(create_rgba_image(info.width, info.height, rgba_data))
}

// BC3/DXT5: 16 bytes per 4x4 block  
// First 8 bytes: interpolated alpha (similar to BC1 color)
// Last 8 bytes: same color encoding as BC1
fn decompress_bc3_to_rgba(bytes: &[u8], info: &DdsInfo) -> anyhow::Result<Image> {
    let data_start = info.data_offset;
    let block_count_x = ((info.width + 3) / 4) as usize;
    let block_count_y = ((info.height + 3) / 4) as usize;
    let block_size = 16; // BC3 uses 16 bytes per block
    
    let mut rgba_data = vec![0u8; (info.width * info.height * 4) as usize];
    
    for by in 0..block_count_y {
        for bx in 0..block_count_x {
            let block_offset = data_start + (by * block_count_x + bx) * block_size;
            if block_offset + block_size > bytes.len() {
                break;
            }
            
            // Read alpha lookup table
            let alpha0 = bytes[block_offset];
            let alpha1 = bytes[block_offset + 1];
            let alpha_lookup = u64::from_le_bytes([
                bytes[block_offset + 2],
                bytes[block_offset + 3],
                bytes[block_offset + 4],
                bytes[block_offset + 5],
                bytes[block_offset + 6],
                bytes[block_offset + 7],
                0, 0,
            ]) & 0xFFFFFFFFFFFF; // 48 bits
            
            // Build alpha table
            let mut alpha_table = [0u8; 8];
            alpha_table[0] = alpha0;
            alpha_table[1] = alpha1;
            if alpha0 > alpha1 {
                // 8 alpha values
                for i in 2..8 {
                    alpha_table[i] = (((8 - i) as u16 * alpha0 as u16 + (i - 1) as u16 * alpha1 as u16) / 7 as u16) as u8;
                }
            } else {
                // 6 alpha values + 0 + 255
                for i in 2..6 {
                    alpha_table[i] = (((6 - i) as u16 * alpha0 as u16 + (i - 1) as u16 * alpha1 as u16) / 5 as u16) as u8;
                }
                alpha_table[6] = 0;
                alpha_table[7] = 255;
            }
            
            // Read color (same as BC1)
            let color0 = u16::from_le_bytes([bytes[block_offset + 8], bytes[block_offset + 9]]);
            let color1 = u16::from_le_bytes([bytes[block_offset + 10], bytes[block_offset + 11]]);
            let lookup = u32::from_le_bytes([
                bytes[block_offset + 12],
                bytes[block_offset + 13],
                bytes[block_offset + 14],
                bytes[block_offset + 15],
            ]);
            
            let colors = decode_bc1_colors(color0, color1);
            
            // Decode 4x4 block
            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px) as u32;
                    let y = (by * 4 + py) as u32;
                    if x < info.width && y < info.height {
                        let idx = (py * 4 + px) as usize;
                        let color_idx = ((lookup >> (idx * 2)) & 3) as usize;
                        let alpha_idx = ((alpha_lookup >> (idx * 3)) & 7) as usize;
                        let pixel_offset = ((y * info.width + x) * 4) as usize;
                        rgba_data[pixel_offset..pixel_offset + 3].copy_from_slice(&colors[color_idx][0..3]);
                        rgba_data[pixel_offset + 3] = alpha_table[alpha_idx];
                    }
                }
            }
        }
    }
    
    Ok(create_rgba_image(info.width, info.height, rgba_data))
}

fn try_image_crate(bytes: &[u8], asset_path: &str) -> anyhow::Result<Image> {
    use image::ImageFormat;
    
    match image::load_from_memory_with_format(bytes, ImageFormat::Dds) {
        Ok(dynamic_image) => {
            let rgba_image = dynamic_image.to_rgba8();
            let (width, height) = rgba_image.dimensions();
            
            Ok(create_rgba_image(width, height, rgba_image.into_raw()))
        }
        Err(e) => {
            anyhow::bail!("Image crate failed to load {}: {:?}", asset_path, e)
        }
    }
}
