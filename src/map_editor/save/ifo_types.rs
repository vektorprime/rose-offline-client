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
/// Fields are in the exact order they appear in the binary format
#[derive(Clone, Debug)]
pub struct IfoObject {
    /// Object name (model path or identifier)
    pub object_name: String,
    /// Warp ID for warp objects
    pub warp_id: u16,
    /// Event ID for event objects
    pub event_id: u16,
    /// Object type (determines behavior)
    pub object_type: u32,
    /// Object ID from the ZSC file
    pub object_id: u32,
    /// Minimap position X coordinate
    pub minimap_pos_x: u32,
    /// Minimap position Y coordinate
    pub minimap_pos_y: u32,
    /// Rotation as quaternion (XYZW order)
    pub rotation: [f32; 4],
    /// Position in IFO coordinate space
    pub position: [f32; 3],
    /// Scale factors
    pub scale: [f32; 3],
}

impl IfoObject {
    /// Create a new IfoObject with default values
    pub fn new(object_id: u32) -> Self {
        Self {
            object_name: String::new(),
            warp_id: 0,
            event_id: 0,
            object_type: 0,
            object_id,
            minimap_pos_x: 0,
            minimap_pos_y: 0,
            rotation: [0.0, 0.0, 0.0, 1.0],
            position: [0.0, 0.0, 0.0],
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
            object_name: String::new(),
            warp_id: 0,
            event_id: 0,
            object_type: 0,
            object_id,
            minimap_pos_x: 0,
            minimap_pos_y: 0,
            rotation: [
                rotation.x,
                -rotation.z,
                rotation.y,
                rotation.w,
            ],
            position: [
                translation.x * 100.0,
                -translation.z * 100.0,
                translation.y * 100.0,
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

    /// Create an IfoObject from rose_file_readers::IfoObject
    /// This copies the IFO data directly without coordinate conversion (data is already in IFO format)
    pub fn from_rose_ifo_object(ifo_object: &rose_file_readers::IfoObject) -> Self {
        Self {
            object_name: ifo_object.object_name.clone(),
            warp_id: ifo_object.warp_id,
            event_id: ifo_object.event_id,
            object_type: ifo_object.object_type,
            object_id: ifo_object.object_id,
            minimap_pos_x: ifo_object.minimap_position.x,
            minimap_pos_y: ifo_object.minimap_position.y,
            rotation: [ifo_object.rotation.x, ifo_object.rotation.y, ifo_object.rotation.z, ifo_object.rotation.w],
            position: [ifo_object.position.x, ifo_object.position.y, ifo_object.position.z],
            scale: [ifo_object.scale.x, ifo_object.scale.y, ifo_object.scale.z],
        }
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
/// Note: warp_id is stored in the base IfoObject, this struct is for semantic clarity
#[derive(Clone, Debug)]
pub struct IfoWarpObject {
    /// Base object data (warp_id is stored in object.warp_id)
    pub object: IfoObject,
}

impl IfoWarpObject {
    /// Create a new IfoWarpObject
    pub fn new(object_id: u32, warp_id: u16) -> Self {
        let mut object = IfoObject::new(object_id);
        object.warp_id = warp_id;
        Self { object }
    }
}

/// Sound object with sound path and range
#[derive(Clone, Debug)]
pub struct IfoSoundObject {
    /// Base object data
    pub object: IfoObject,
    /// Path to the sound file
    pub sound_path: String,
    /// Sound range/radius (stored as u32 in file)
    pub range: u32,
    /// Sound playback interval in seconds (stored as u32 in file)
    pub interval: u32,
}

impl IfoSoundObject {
    /// Create a new IfoSoundObject
    pub fn new(object_id: u32) -> Self {
        Self {
            object: IfoObject::new(object_id),
            sound_path: String::new(),
            range: 100,
            interval: 0,
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

/// NPC object with AI and quest data
#[derive(Clone, Debug)]
pub struct IfoNpc {
    /// Base object data
    pub object: IfoObject,
    /// AI ID for this NPC
    pub ai_id: u32,
    /// Quest file name
    pub quest_file_name: String,
}

impl IfoNpc {
    /// Create a new IfoNpc
    pub fn new(object_id: u32) -> Self {
        Self {
            object: IfoObject::new(object_id),
            ai_id: 0,
            quest_file_name: String::new(),
        }
    }
}

/// Monster spawn entry (basic or tactic)
#[derive(Clone, Debug)]
pub struct IfoMonsterSpawn {
    /// Monster ID
    pub id: u32,
    /// Monster count
    pub count: u32,
}

impl IfoMonsterSpawn {
    /// Create a new IfoMonsterSpawn
    pub fn new(id: u32, count: u32) -> Self {
        Self { id, count }
    }
}

/// Monster spawn point with basic and tactic spawns
#[derive(Clone, Debug)]
pub struct IfoMonsterSpawnPoint {
    /// Base object data
    pub object: IfoObject,
    /// Spawn name (stored for round-trip preservation)
    pub spawn_name: String,
    /// Basic spawn list
    pub basic_spawns: Vec<IfoMonsterSpawn>,
    /// Tactic spawn list
    pub tactic_spawns: Vec<IfoMonsterSpawn>,
    /// Spawn interval
    pub interval: u32,
    /// Limit count
    pub limit_count: u32,
    /// Spawn range
    pub range: u32,
    /// Tactic points
    pub tactic_points: u32,
}

impl IfoMonsterSpawnPoint {
    /// Create a new IfoMonsterSpawnPoint
    pub fn new(object_id: u32) -> Self {
        Self {
            object: IfoObject::new(object_id),
            spawn_name: String::new(),
            basic_spawns: Vec::new(),
            tactic_spawns: Vec::new(),
            interval: 0,
            limit_count: 0,
            range: 0,
            tactic_points: 0,
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

/// Block type identifiers for preserving original block order
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IfoBlockType {
    /// Deprecated map info (type 0)
    DeprecatedMapInfo = 0,
    /// Decoration objects (type 1)
    DecoObject = 1,
    /// NPCs (type 2)
    Npc = 2,
    /// Construction objects (type 3)
    CnstObject = 3,
    /// Sound objects (type 4)
    SoundObject = 4,
    /// Effect objects (type 5)
    EffectObject = 5,
    /// Animated objects (type 6)
    AnimatedObject = 6,
    /// Deprecated water (type 7)
    DeprecatedWater = 7,
    /// Monster spawns (type 8)
    MonsterSpawn = 8,
    /// Water planes (type 9)
    WaterPlanes = 9,
    /// Warp objects (type 10)
    Warp = 10,
    /// Collision objects (type 11)
    CollisionObject = 11,
    /// Event objects (type 12)
    EventObject = 12,
}

impl IfoBlockType {
    /// Convert from u32 to IfoBlockType
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::DeprecatedMapInfo),
            1 => Some(Self::DecoObject),
            2 => Some(Self::Npc),
            3 => Some(Self::CnstObject),
            4 => Some(Self::SoundObject),
            5 => Some(Self::EffectObject),
            6 => Some(Self::AnimatedObject),
            7 => Some(Self::DeprecatedWater),
            8 => Some(Self::MonsterSpawn),
            9 => Some(Self::WaterPlanes),
            10 => Some(Self::Warp),
            11 => Some(Self::CollisionObject),
            12 => Some(Self::EventObject),
            _ => None,
        }
    }

    /// Convert to u32
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

/// Represents a single block in an IFO file
#[derive(Clone, Debug, Default)]
pub struct IfoBlock {
    /// Block X coordinate in the zone (0-63)
    pub block_x: u32,
    /// Block Z coordinate in the zone (0-63)
    pub block_z: u32,
    /// Original block order from the loaded file (for preserving file structure)
    /// Stores the block type IDs in the order they appeared in the original file
    pub original_block_order: Vec<u32>,
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
    /// Collision objects
    pub collision_objects: Vec<IfoObject>,
    /// Water planes
    pub water_planes: Vec<IfoWaterPlane>,
    /// Water size
    pub water_size: f32,
    /// NPCs in this block
    pub npcs: Vec<IfoNpc>,
    /// Monster spawn points in this block
    pub monster_spawns: Vec<IfoMonsterSpawnPoint>,
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
            + self.collision_objects.len()
            + self.npcs.len()
            + self.monster_spawns.len()
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
    /// Whether this block has been modified and needs to be saved
    pub modified: bool,
}

impl IfoFileData {
    /// Create a new IfoFileData for a specific block (marked as unmodified)
    pub fn new(block_x: u32, block_y: u32) -> Self {
        let file_path = format!("{}_{}.IFO", block_x, block_y);
        Self {
            file_path,
            block_x,
            block_y,
            block: IfoBlock::new(block_x, block_y),
            modified: false,
        }
    }

    /// Create a new IfoFileData for a specific block (marked as modified)
    pub fn new_modified(block_x: u32, block_y: u32) -> Self {
        let file_path = format!("{}_{}.IFO", block_x, block_y);
        Self {
            file_path,
            block_x,
            block_y,
            block: IfoBlock::new(block_x, block_y),
            modified: true,
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

    /// Get or create a block at the specified coordinates (unmodified)
    pub fn get_or_create_block(&mut self, block_x: u32, block_y: u32) -> &mut IfoFileData {
        let index = (block_x + block_y * 64) as usize;
        if self.blocks[index].is_none() {
            self.blocks[index] = Some(IfoFileData::new(block_x, block_y));
        }
        self.blocks[index].as_mut().unwrap()
    }

    /// Get or create a block at the specified coordinates, marking it as modified
    /// This should be used when adding new objects from the editor
    pub fn get_or_create_modified_block(&mut self, block_x: u32, block_y: u32) -> &mut IfoFileData {
        let index = (block_x + block_y * 64) as usize;
        if self.blocks[index].is_none() {
            self.blocks[index] = Some(IfoFileData::new_modified(block_x, block_y));
        } else {
            // Mark existing block as modified
            self.blocks[index].as_mut().unwrap().modified = true;
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

    /// Create ZoneExportData from existing ZoneLoaderAsset blocks
    /// This pre-populates the export data with existing IFO data to preserve
    /// objects that weren't modified in the editor
    pub fn from_existing_blocks(
        zone_id: u16,
        blocks: &[Option<std::boxed::Box<crate::zone_loader::ZoneLoaderBlock>>],
        base_path: String,
    ) -> Self {
        let mut export_data = Self::new(zone_id, base_path);
        
        for (block_idx, block_opt) in blocks.iter().enumerate() {
            let Some(block) = block_opt else {
                continue;
            };
            
            let Some(ifo) = &block.ifo else {
                continue;
            };
            
            // Calculate block coordinates from index (64x64 grid)
            let block_x = (block_idx % 64) as u32;
            let block_y = (block_idx / 64) as u32;
            
            // Get or create the export block
            let export_block = export_data.get_or_create_block(block_x, block_y);
            
            // Convert deco objects from IFO
            for ifo_object in &ifo.deco_objects {
                export_block.block.deco_objects.push(IfoObject::from_rose_ifo_object(ifo_object));
            }
            
            // Convert cnst objects from IFO
            for ifo_object in &ifo.cnst_objects {
                export_block.block.cnst_objects.push(IfoObject::from_rose_ifo_object(ifo_object));
            }
            
            // Convert event objects from IFO
            for ifo_event in &ifo.event_objects {
                let mut event_obj = IfoEventObject::new(ifo_event.object.object_id);
                event_obj.object = IfoObject::from_rose_ifo_object(&ifo_event.object);
                event_obj.quest_trigger_name = ifo_event.quest_trigger_name.clone();
                event_obj.script_function_name = ifo_event.script_function_name.clone();
                export_block.block.event_objects.push(event_obj);
            }
            
            // Convert warp objects from IFO
            // Note: In rose_file_readers, warps are IfoObject with warp_id as a direct field
            for ifo_warp in &ifo.warps {
                let mut warp_obj = IfoWarpObject::new(ifo_warp.object_id, ifo_warp.warp_id);
                warp_obj.object = IfoObject::from_rose_ifo_object(ifo_warp);
                export_block.block.warp_objects.push(warp_obj);
            }
            
            // Convert sound objects from IFO
            for ifo_sound in &ifo.sound_objects {
                let mut sound_obj = IfoSoundObject::new(0);
                sound_obj.object = IfoObject::from_rose_ifo_object(&ifo_sound.object);
                sound_obj.sound_path = ifo_sound.sound_path.path().to_string_lossy().to_string();
                sound_obj.range = ifo_sound.range;
                sound_obj.interval = ifo_sound.interval.as_secs() as u32;
                export_block.block.sound_objects.push(sound_obj);
            }
            
            // Convert effect objects from IFO
            for ifo_effect in &ifo.effect_objects {
                let mut effect_obj = IfoEffectObject::new(0);
                effect_obj.object = IfoObject::from_rose_ifo_object(&ifo_effect.object);
                effect_obj.effect_path = ifo_effect.effect_path.path().to_string_lossy().to_string();
                export_block.block.effect_objects.push(effect_obj);
            }
            
            // Convert animated objects from IFO
            for ifo_object in &ifo.animated_objects {
                export_block.block.animated_objects.push(IfoObject::from_rose_ifo_object(ifo_object));
            }
            
            // Convert water planes from IFO
            for (start, end) in &ifo.water_planes {
                export_block.block.water_planes.push(IfoWaterPlane {
                    start: [start.x, start.y, start.z],
                    end: [end.x, end.y, end.z],
                });
            }
            
            // Set water size
            export_block.block.water_size = ifo.water_size;
            
            // Convert NPCs from IFO
            for ifo_npc in &ifo.npcs {
                let mut npc = IfoNpc::new(ifo_npc.object.object_id);
                npc.object = IfoObject::from_rose_ifo_object(&ifo_npc.object);
                npc.ai_id = ifo_npc.ai_id;
                npc.quest_file_name = ifo_npc.quest_file_name.clone();
                export_block.block.npcs.push(npc);
            }
            
            // Convert collision objects from IFO
            for ifo_object in &ifo.collision_objects {
                export_block.block.collision_objects.push(IfoObject::from_rose_ifo_object(ifo_object));
            }
            
            // Convert monster spawns from IFO
            for ifo_spawn in &ifo.monster_spawns {
                let mut spawn_point = IfoMonsterSpawnPoint::new(ifo_spawn.object.object_id);
                spawn_point.object = IfoObject::from_rose_ifo_object(&ifo_spawn.object);
                spawn_point.spawn_name = String::new(); // spawn_name is not stored in rose_file_readers
                spawn_point.basic_spawns = ifo_spawn.basic_spawns.iter()
                    .map(|s| IfoMonsterSpawn::new(s.id, s.count))
                    .collect();
                spawn_point.tactic_spawns = ifo_spawn.tactic_spawns.iter()
                    .map(|s| IfoMonsterSpawn::new(s.id, s.count))
                    .collect();
                spawn_point.interval = ifo_spawn.interval;
                spawn_point.limit_count = ifo_spawn.limit_count;
                spawn_point.range = ifo_spawn.range;
                spawn_point.tactic_points = ifo_spawn.tactic_points;
                export_block.block.monster_spawns.push(spawn_point);
            }
        }
        
        export_data
    }
}
