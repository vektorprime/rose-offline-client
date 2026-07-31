//! Shared coordinate conversion and zone bootstrap-file helpers for the map editor.

use bevy::prelude::Vec3;

/// World-space X of the zone center
pub const ZONE_CENTER_X: f32 = 5200.0;
/// World-space Z of the zone center
pub const ZONE_CENTER_Z: f32 = -5200.0;
/// Size of one IFO block in meters
pub const BLOCK_SIZE_METERS: f32 = 160.0;
/// Number of blocks per zone dimension
pub const ZONE_BLOCK_COUNT: u32 = 64;

/// Convert world coordinates to IFO block coordinates
pub fn world_to_block_coords(world_translation: Vec3) -> (u32, u32) {
    let local_x = world_translation.x - ZONE_CENTER_X;
    let local_z = world_translation.z - ZONE_CENTER_Z;
    let block_x = ((local_x + ZONE_CENTER_X) / BLOCK_SIZE_METERS).floor() as u32;
    let block_y = ((local_z + ZONE_CENTER_X) / BLOCK_SIZE_METERS).floor() as u32;
    (
        block_x.clamp(0, ZONE_BLOCK_COUNT - 1),
        block_y.clamp(0, ZONE_BLOCK_COUNT - 1),
    )
}

/// Write a HIM file (width + height + 2 reserved u32s + per-cell f32 heights in cm)
pub fn write_him_file(
    path: &std::path::Path,
    width: u32,
    height: u32,
    heights_cm: &[f32],
) -> std::io::Result<()> {
    use std::io::Write;
    let mut data = Vec::with_capacity(16 + heights_cm.len() * 4);
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    for h in heights_cm {
        data.extend_from_slice(&h.to_le_bytes());
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(&data)?;
    Ok(())
}

/// Write a TIL file (width + height + per-cell 3 reserved bytes + tile index u32)
pub fn write_til_file(
    path: &std::path::Path,
    width: u32,
    height: u32,
    tiles: &[u32],
) -> std::io::Result<()> {
    use std::io::Write;
    let mut data = Vec::with_capacity(8 + tiles.len() * 7);
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    for tile in tiles {
        data.extend_from_slice(&[0u8; 3]);
        data.extend_from_slice(&tile.to_le_bytes());
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(&data)?;
    Ok(())
}
