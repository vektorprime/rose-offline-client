//! IFO File Data Structures
//! 
//! This module contains data structures for representing IFO file data
//! for export purposes. These structures mirror the IFO file format
//! used by Rose Online.

use bevy::math::{Quat, Vec3};

/// IFO file header magic bytes
pub const IFO_MAGIC: &[u8; 3] = b"IFO";

/// IFO file version
pub const IFO_VERSION: u32 = 0x0101;

/// Represents a single object in an IFO file
#[derive(Clone, Debug)]
pub struct IfoObject {
    /// Object ID from the ZSC file
    pub object_id: u32,
    /// Position in IFO coordinate space (centimeters)
    pub position: [f32; 3],
    /// Rotation as quaternion (x, y, z, w)
    pub rotation: [f32; 4],
    /// Scale factors
    pub scale: [f32; 3],
}

impl IfoObject {
    /// Create a new IfoObject with default values
    pub fn new(object_id: u32) -> Self {
        Self {
            object_id,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Create an IfoObject from Bevy Transform components
    /// Converts from Bevy's coordinate system to IFO coordinate system
    pub fn from_transform(object_id: u32, translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        // Convert from Bevy coordinates to IFO coordinates
        // Bevy: Y-up, IFO: Z-up
        // Position: multiply by 100 to convert from meters to centimeters
        // The zone_loader uses: Vec3::new(x, z, -y) for loading, so we reverse it
        Self {
            object_id,
            position: [
                translation.x * 100.0,
                -translation.z * 100.0,
                translation.y * 100.0,
            ],
            rotation: [
                rotation.x,
                -rotation.z,
                rotation.y,
                rotation.w,
            ],
            scale: [
                scale.x,
                scale.z,
                scale.y,
            ],
        }
    }

    /// Convert back to Bevy coordinate system
    pub fn to_bevy_transform(&self) -> (Vec3, Quat, Vec3) {
        let translation = Vec3::new(
            self.position[0] / 100.0,
            self.position[2] / 100.0,
            -self.position[1] / 100.0,
        );
        let rotation = Quat::from_xyzw(
            self.rotation[0],
            self.rotation[2],
            -self.rotation[1],
            self.rotation[3],
        );
        let scale = Vec3::new(
            self.scale[0],
            self.scale[2],
            self.scale[1],
        );
        (translation, rotation, scale)
    }
}

/// Event object with additional quest/script data
#[derive(Clone, Debug)]
pub struct IfoEventObject {
    /// Base object data
    pub object: IfoObject,
    /// Quest trigger name
    pub quest_trigger_name: String,
    /// Script function name
    pub script_function_name: String,
}

impl IfoEventObject {
    /// Create a new IfoEventObject
    pub fn new(object_id: u32) -> Self {
        Self {
            object: IfoObject::new(object_id),
            quest_trigger_name: String::new(),
            script_function_name: String::new(),
        }
    }
}

/// Warp object with destination warp ID
#[derive(Clone, Debug)]
pub struct IfoWarpObject {
    /// Base object data
    pub object: IfoObject,
    /// Destination warp gate ID
    pub warp_id: u16,
}

impl IfoWarpObject {
    /// Create a new IfoWarpObject
    pub fn new(object_id: u32, warp_id: u16) -> Self {
        Self {
            object: IfoObject::new(object_id),
            warp_id,
        }
    }
}

/// Sound object with sound path and range
#[derive(Clone, Debug)]
pub struct IfoSoundObject {
    /// Base object data
    pub object: IfoObject,
    /// Path to the sound file
    pub sound_path: String,
    /// Sound range/radius
    pub range: f32,
}

impl IfoSoundObject {
    /// Create a new IfoSoundObject
    pub fn new(object_id: u32) -> Self {
        Self {
            object: IfoObject::new(object_id),
            sound_path: String::new(),
            range: 100.0,
        }
    }
}

/// Effect object with effect path
#[derive(Clone, Debug)]
pub struct IfoEffectObject {
    /// Base object data
    pub object: IfoObject,
    /// Path to the effect file
    pub effect_path: String,
}

impl IfoEffectObject {
    /// Create a new IfoEffectObject
    pub fn new(object_id: u32) -> Self {
        Self {
            object: IfoObject::new(object_id),
            effect_path: String::new(),
        }
    }
}

/// Water plane definition
#[derive(Clone, Debug)]
pub struct IfoWaterPlane {
    /// Start corner of the water plane
    pub start: [f32; 3],
    /// End corner of the water plane
    pub end: [f32; 3],
}

impl IfoWaterPlane {
    /// Create a new IfoWaterPlane
    pub fn new() -> Self {
        Self {
            start: [0.0, 0.0, 0.0],
            end: [0.0, 0.0, 0.0],
        }
    }
}

/// Represents a single block (4x4 grid) in an IFO file
#[derive(Clone, Debug, Default)]
pub struct IfoBlock {
    /// Block X coordinate (0-3)
    pub block_x: u32,
    /// Block Z coordinate (0-3)
    pub block_z: u32,
    /// Decoration objects
    pub deco_objects: Vec<IfoObject>,
    /// Construction/building objects
    pub cnst_objects: Vec<IfoObject>,
    /// Event/trigger objects
    pub event_objects: Vec<IfoEventObject>,
    /// Warp/teleport objects
    pub warp_objects: Vec<IfoWarpObject>,
    /// Sound objects
    pub sound_objects: Vec<IfoSoundObject>,
    /// Effect objects
    pub effect_objects: Vec<IfoEffectObject>,
    /// Animated objects (morph objects)
    pub animated_objects: Vec<IfoObject>,
    /// Water planes
    pub water_planes: Vec<IfoWaterPlane>,
    /// Water size
    pub water_size: f32,
    /// NPCs in this block
    pub npcs: Vec<IfoObject>,
}

impl IfoBlock {
    /// Create a new IfoBlock
    pub fn new(block_x: u32, block_z: u32) -> Self {
        Self {
            block_x,
            block_z,
            ..Default::default()
        }
    }

    /// Get total object count in this block
    pub fn total_objects(&self) -> usize {
        self.deco_objects.len()
            + self.cnst_objects.len()
            + self.event_objects.len()
            + self.warp_objects.len()
            + self.sound_objects.len()
            + self.effect_objects.len()
            + self.animated_objects.len()
            + self.npcs.len()
    }
}

/// Complete IFO file data structure
#[derive(Clone, Debug)]
pub struct IfoFileData {
    /// File path (relative to zone directory)
    pub file_path: String,
    /// Block X coordinate in the zone (0-63)
    pub block_x: u32,
    /// Block Y coordinate in the zone (0-63)
    pub block_y: u32,
    /// Block data
    pub block: IfoBlock,
}

impl IfoFileData {
    /// Create a new IfoFileData for a specific block
    pub fn new(block_x: u32, block_y: u32) -> Self {
        let file_path = format!("{}_{}.IFO", block_x, block_y);
        Self {
            file_path,
            block_x,
            block_y,
            block: IfoBlock::new(block_x % 4, block_y % 4),
        }
    }

    /// Get the file name for this IFO file
    pub fn file_name(&self) -> String {
        format!("{}_{}.IFO", self.block_x, self.block_y)
    }
}

/// Zone export data containing all IFO blocks
#[derive(Clone, Debug, Default)]
pub struct ZoneExportData {
    /// Zone ID
    pub zone_id: u16,
    /// Zone path (directory)
    pub zone_path: String,
    /// All IFO blocks in the zone (64x64 = up to 4096 blocks)
    pub blocks: Vec<Option<IfoFileData>>,
}

impl ZoneExportData {
    /// Create a new ZoneExportData
    pub fn new(zone_id: u16, zone_path: String) -> Self {
        let mut blocks = Vec::with_capacity(64 * 64);
        blocks.resize_with(64 * 64, || None);
        Self {
            zone_id,
            zone_path,
            blocks,
        }
    }

    /// Get or create a block at the specified coordinates
    pub fn get_or_create_block(&mut self, block_x: u32, block_y: u32) -> &mut IfoFileData {
        let index = (block_x + block_y * 64) as usize;
        if self.blocks[index].is_none() {
            self.blocks[index] = Some(IfoFileData::new(block_x, block_y));
        }
        self.blocks[index].as_mut().unwrap()
    }

    /// Get a block at the specified coordinates (read-only)
    pub fn get_block(&self, block_x: u32, block_y: u32) -> Option<&IfoFileData> {
        let index = (block_x + block_y * 64) as usize;
        self.blocks[index].as_ref()
    }

    /// Count total objects in the zone
    pub fn total_objects(&self) -> usize {
        self.blocks
            .iter()
            .filter_map(|b| b.as_ref())
            .map(|b| b.block.total_objects())
            .sum()
    }

    /// Count blocks with data
    pub fn populated_block_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.is_some()).count()
    }
}
