use bevy::prelude::{Component, Vec3};

/// Editable metadata for map-editor water planes.
///
/// Values are stored in raw IFO coordinate space (centimeters) so save/export can
/// round-trip exactly without transform ambiguity.
#[derive(Component, Clone, Debug)]
pub struct MapEditorWaterPlane {
    pub block_x: u32,
    pub block_y: u32,
    pub start_ifo_cm: Vec3,
    pub end_ifo_cm: Vec3,
    /// Water texture scale parameter from IFO (centimeters)
    pub water_size: f32,
}

impl MapEditorWaterPlane {
    pub fn new(
        block_x: u32,
        block_y: u32,
        start_ifo_cm: Vec3,
        end_ifo_cm: Vec3,
        water_size: f32,
    ) -> Self {
        Self {
            block_x,
            block_y,
            start_ifo_cm,
            end_ifo_cm,
            water_size,
        }
    }

    pub fn center_local_meters(&self) -> Vec3 {
        let sx = self.start_ifo_cm.x / 100.0;
        let sy = self.start_ifo_cm.y / 100.0;
        let sz = -self.start_ifo_cm.z / 100.0;
        let ex = self.end_ifo_cm.x / 100.0;
        let ey = self.end_ifo_cm.y / 100.0;
        let ez = -self.end_ifo_cm.z / 100.0;
        Vec3::new((sx + ex) * 0.5, (sy + ey) * 0.5, (sz + ez) * 0.5)
    }
}

/// Editable terrain block data mirrored from HIM/TIL files.
#[derive(Component, Clone, Debug)]
pub struct MapEditorTerrainBlock {
    pub block_x: u32,
    pub block_y: u32,
    pub him_width: u32,
    pub him_height: u32,
    pub him_heights_cm: Vec<f32>,
    pub til_width: u32,
    pub til_height: u32,
    pub til_tiles: Vec<u32>,
    /// Uniform height offset applied to all HIM samples on save (centimeters)
    pub height_offset_cm: f32,
    /// Optional fill tile id applied to all TIL cells on save
    pub fill_tile_id: Option<u32>,
    /// Tracks whether this block was edited in the map editor
    pub dirty: bool,
}
