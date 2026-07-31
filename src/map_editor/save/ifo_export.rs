//! IFO File Export System
//!
//! This module provides functionality to write IFO files in the binary format
//! used by Rose Online. The format uses a block-based structure where each
//! block type has its own section with a type ID and offset.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use super::ifo_types::*;

/// Block type identifiers matching the loader's enum
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
enum BlockType {
    DeprecatedMapInfo = 0,
    DecoObject = 1,
    Npc = 2,
    CnstObject = 3,
    SoundObject = 4,
    EffectObject = 5,
    AnimatedObject = 6,
    DeprecatedWater = 7,
    MonsterSpawn = 8,
    WaterPlanes = 9,
    Warp = 10,
    CollisionObject = 11,
    EventObject = 12,
}

/// IFO file writer for exporting zone data
pub struct IfoWriter {
    /// Output buffer
    buffer: Vec<u8>,
}

impl IfoWriter {
    /// Create a new IFO writer
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64 * 1024), // 64KB initial capacity
        }
    }

    /// Write a u8 length-prefixed string (max 255 chars)
    fn write_u8_string(vec: &mut Vec<u8>, s: &str) {
        // String length as u8 (max 255)
        let len = s.len().min(255) as u8;
        vec.push(len);

        // String bytes
        vec.extend_from_slice(&s.as_bytes()[..len as usize]);
    }

    /// Write a single IfoObject in the exact format the loader expects:
    /// - object_name: u8 length-prefixed string
    /// - warp_id: u16
    /// - event_id: u16
    /// - object_type: u32
    /// - object_id: u32
    /// - minimap_pos_x: u32
    /// - minimap_pos_y: u32
    /// - rotation: Quat4<f32> XYZW order
    /// - position: Vec3<f32>
    /// - scale: Vec3<f32>
    fn write_object(vec: &mut Vec<u8>, obj: &IfoObject) {
        // Object name (u8 length-prefixed string)
        Self::write_u8_string(vec, &obj.object_name);

        // warp_id (u16)
        vec.extend_from_slice(&obj.warp_id.to_le_bytes());

        // event_id (u16)
        vec.extend_from_slice(&obj.event_id.to_le_bytes());

        // object_type (u32)
        vec.extend_from_slice(&obj.object_type.to_le_bytes());

        // object_id (u32)
        vec.extend_from_slice(&obj.object_id.to_le_bytes());

        // minimap_pos_x (u32)
        vec.extend_from_slice(&obj.minimap_pos_x.to_le_bytes());

        // minimap_pos_y (u32)
        vec.extend_from_slice(&obj.minimap_pos_y.to_le_bytes());

        // rotation (Quat4<f32> XYZW order)
        for &v in &obj.rotation {
            vec.extend_from_slice(&v.to_le_bytes());
        }

        // position (Vec3<f32>)
        for &v in &obj.position {
            vec.extend_from_slice(&v.to_le_bytes());
        }

        // scale (Vec3<f32>)
        for &v in &obj.scale {
            vec.extend_from_slice(&v.to_le_bytes());
        }
    }

    /// Write an event object (object + quest_trigger_name + script_function_name)
    fn write_event_object(vec: &mut Vec<u8>, obj: &IfoEventObject) {
        Self::write_object(vec, &obj.object);
        Self::write_u8_string(vec, &obj.quest_trigger_name);
        Self::write_u8_string(vec, &obj.script_function_name);
    }

    /// Write a warp object (just the base object - warp_id is in IfoObject.warp_id)
    fn write_warp_object(vec: &mut Vec<u8>, obj: &IfoWarpObject) {
        Self::write_object(vec, &obj.object);
    }

    /// Write a sound object (object + sound_path + range + interval)
    fn write_sound_object(vec: &mut Vec<u8>, obj: &IfoSoundObject) {
        Self::write_object(vec, &obj.object);
        Self::write_u8_string(vec, &obj.sound_path);
        // Range (u32)
        vec.extend_from_slice(&obj.range.to_le_bytes());
        // Interval (u32)
        vec.extend_from_slice(&obj.interval.to_le_bytes());
    }

    /// Write an effect object (object + effect_path)
    fn write_effect_object(vec: &mut Vec<u8>, obj: &IfoEffectObject) {
        Self::write_object(vec, &obj.object);
        Self::write_u8_string(vec, &obj.effect_path);
    }

    /// Write an NPC object (object + ai_id + quest_file_name)
    fn write_npc(vec: &mut Vec<u8>, npc: &IfoNpc) {
        Self::write_object(vec, &npc.object);
        // AI ID (u32)
        vec.extend_from_slice(&npc.ai_id.to_le_bytes());
        // Quest file name (u8 length-prefixed string)
        Self::write_u8_string(vec, &npc.quest_file_name);
    }

    /// Write a monster spawn point
    /// Format (from rose-file-readers/src/ifo.rs:249-289):
    /// - object: IfoObject
    /// - spawn_name: u8 length-prefixed string
    /// - basic_count: u32
    /// - For each basic spawn:
    ///   - monster_name: u8 length-prefixed string (we write empty string)
    ///   - monster_id: u32
    ///   - monster_count: u32
    /// - tactic_count: u32
    /// - For each tactic spawn:
    ///   - monster_name: u8 length-prefixed string (we write empty string)
    ///   - monster_id: u32
    ///   - monster_count: u32
    /// - interval: u32
    /// - limit_count: u32
    /// - range: u32
    /// - tactic_points: u32
    fn write_monster_spawn(vec: &mut Vec<u8>, spawn: &IfoMonsterSpawnPoint) {
        // Write base object
        Self::write_object(vec, &spawn.object);

        // Write spawn name
        Self::write_u8_string(vec, &spawn.spawn_name);

        // Write basic spawns
        let basic_count = spawn.basic_spawns.len() as u32;
        vec.extend_from_slice(&basic_count.to_le_bytes());
        for basic in &spawn.basic_spawns {
            // Monster name (empty - loader discards this anyway)
            Self::write_u8_string(vec, "");
            // Monster ID
            vec.extend_from_slice(&basic.id.to_le_bytes());
            // Monster count
            vec.extend_from_slice(&basic.count.to_le_bytes());
        }

        // Write tactic spawns
        let tactic_count = spawn.tactic_spawns.len() as u32;
        vec.extend_from_slice(&tactic_count.to_le_bytes());
        for tactic in &spawn.tactic_spawns {
            // Monster name (empty - loader discards this anyway)
            Self::write_u8_string(vec, "");
            // Monster ID
            vec.extend_from_slice(&tactic.id.to_le_bytes());
            // Monster count
            vec.extend_from_slice(&tactic.count.to_le_bytes());
        }

        // Write spawn parameters
        vec.extend_from_slice(&spawn.interval.to_le_bytes());
        vec.extend_from_slice(&spawn.limit_count.to_le_bytes());
        vec.extend_from_slice(&spawn.range.to_le_bytes());
        vec.extend_from_slice(&spawn.tactic_points.to_le_bytes());
    }

    /// Write a water plane (start Vec3 + end Vec3)
    fn write_water_plane(vec: &mut Vec<u8>, plane: &IfoWaterPlane) {
        // Start position (3 x f32)
        for &v in &plane.start {
            vec.extend_from_slice(&v.to_le_bytes());
        }
        // End position (3 x f32)
        for &v in &plane.end {
            vec.extend_from_slice(&v.to_le_bytes());
        }
    }

    /// Write a complete IFO block to the buffer using the block-based format
    /// that the loader expects:
    /// - block_count: u32
    /// - For each block: block_type (u32) + block_offset (u32)
    /// - Then the actual block data at each offset
    pub fn write_block(&mut self, block: &IfoBlock) -> io::Result<()> {
        log::debug!(
            "[IFO Writer] Writing block ({}, {}) with {} objects",
            block.block_x,
            block.block_z,
            block.total_objects()
        );

        self.buffer.clear();

        // Build data for each block type that has objects
        // Use a map to store block data by type for order preservation
        let mut block_data_map: std::collections::HashMap<u32, Vec<u8>> =
            std::collections::HashMap::new();

        // Build data for each block type that has objects
        if !block.deco_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.deco_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.deco_objects {
                Self::write_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::DecoObject as u32, data);
        }

        if !block.cnst_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.cnst_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.cnst_objects {
                Self::write_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::CnstObject as u32, data);
        }

        if !block.event_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.event_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.event_objects {
                Self::write_event_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::EventObject as u32, data);
        }

        if !block.warp_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.warp_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.warp_objects {
                Self::write_warp_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::Warp as u32, data);
        }

        if !block.sound_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.sound_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.sound_objects {
                Self::write_sound_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::SoundObject as u32, data);
        }

        if !block.effect_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.effect_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.effect_objects {
                Self::write_effect_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::EffectObject as u32, data);
        }

        if !block.animated_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.animated_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.animated_objects {
                Self::write_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::AnimatedObject as u32, data);
        }

        if !block.collision_objects.is_empty() {
            let mut data = Vec::new();
            let count = block.collision_objects.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for obj in &block.collision_objects {
                Self::write_object(&mut data, obj);
            }
            block_data_map.insert(BlockType::CollisionObject as u32, data);
        }

        if !block.npcs.is_empty() {
            let mut data = Vec::new();
            let count = block.npcs.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for npc in &block.npcs {
                Self::write_npc(&mut data, npc);
            }
            block_data_map.insert(BlockType::Npc as u32, data);
        }

        // Add monster spawn support
        if !block.monster_spawns.is_empty() {
            let mut data = Vec::new();
            let count = block.monster_spawns.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for spawn in &block.monster_spawns {
                Self::write_monster_spawn(&mut data, spawn);
            }
            block_data_map.insert(BlockType::MonsterSpawn as u32, data);
        }

        if !block.water_planes.is_empty() || block.water_size > 0.0 {
            let mut data = Vec::new();
            // water_size (f32)
            data.extend_from_slice(&block.water_size.to_le_bytes());
            // water plane count (u32)
            let count = block.water_planes.len() as u32;
            data.extend_from_slice(&count.to_le_bytes());
            for plane in &block.water_planes {
                Self::write_water_plane(&mut data, plane);
            }
            block_data_map.insert(BlockType::WaterPlanes as u32, data);
        }

        // Determine block order: use original order if available, otherwise use sorted order
        let block_order: Vec<u32> = if !block.original_block_order.is_empty() {
            // Use original order, but only include blocks that still have data
            block
                .original_block_order
                .iter()
                .filter(|&&block_type| block_data_map.contains_key(&block_type))
                .copied()
                .collect()
        } else {
            // No original order - use sorted order for consistency
            let mut keys: Vec<u32> = block_data_map.keys().copied().collect();
            keys.sort();
            keys
        };

        // Now write the file header
        let block_count = block_order.len() as u32;
        self.buffer.extend_from_slice(&block_count.to_le_bytes());

        // Calculate the header size: block_count (4 bytes) + each block entry (8 bytes)
        let header_size = 4 + (block_count as usize * 8);

        // Calculate offsets for each block in order
        let mut current_offset = header_size as u32;
        let mut block_offsets: Vec<(u32, u32)> = Vec::new();
        for block_type in &block_order {
            if let Some(data) = block_data_map.get(block_type) {
                block_offsets.push((*block_type, current_offset));
                current_offset += data.len() as u32;
            }
        }

        // Write block headers (type + offset pairs)
        for (block_type, offset) in &block_offsets {
            self.buffer.extend_from_slice(&block_type.to_le_bytes());
            self.buffer.extend_from_slice(&offset.to_le_bytes());
        }

        // Write block data in order
        for block_type in &block_order {
            if let Some(data) = block_data_map.remove(block_type) {
                self.buffer.extend_from_slice(&data);
            }
        }

        Ok(())
    }

    /// Save the buffer to a file
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;
        file.write_all(&self.buffer)?;
        file.sync_all()?;

        Ok(())
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Get a reference to the buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Default for IfoWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Export a single IFO block to a file
pub fn export_ifo_block(block: &IfoBlock, path: &Path) -> io::Result<usize> {
    let mut writer = IfoWriter::new();
    writer.write_block(block)?;
    writer.save_to_file(path)?;
    Ok(writer.buffer_size())
}

/// Statistics about the export operation
#[derive(Debug, Default, Clone)]
pub struct ExportStats {
    /// Number of blocks successfully exported
    pub blocks_exported: usize,
    /// Number of blocks that failed to export
    pub blocks_failed: usize,
    /// Total bytes written
    pub bytes_written: usize,
    /// Total objects exported
    pub total_objects: usize,
}

impl ExportStats {
    /// Check if the export was completely successful
    pub fn is_success(&self) -> bool {
        self.blocks_failed == 0
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "Exported {} blocks ({} objects, {} bytes), {} failed",
            self.blocks_exported, self.total_objects, self.bytes_written, self.blocks_failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_u8_string() {
        let mut buffer = Vec::new();
        IfoWriter::write_u8_string(&mut buffer, "test");

        // Length prefix (1 byte) + string bytes (4 bytes)
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0], 4); // Length as u8
        assert_eq!(&buffer[1..5], b"test");
    }

    #[test]
    fn test_write_object() {
        let mut buffer = Vec::new();
        let obj = IfoObject::new(42);

        IfoWriter::write_object(&mut buffer, &obj);

        // Object should have:
        // - 1 byte length + 0 bytes string = 1 byte
        // - warp_id (2) + event_id (2) + object_type (4) + object_id (4) = 12 bytes
        // - minimap_pos_x (4) + minimap_pos_y (4) = 8 bytes
        // - rotation (16) + position (12) + scale (12) = 40 bytes
        // Total: 1 + 12 + 8 + 40 = 61 bytes
        assert_eq!(buffer.len(), 61);
    }

    #[test]
    fn test_block_format() {
        let mut writer = IfoWriter::new();
        let mut block = IfoBlock::new(0, 0);
        block.deco_objects.push(IfoObject::new(1));

        writer.write_block(&block).unwrap();

        // Should start with block_count
        let block_count = u32::from_le_bytes([
            writer.buffer[0],
            writer.buffer[1],
            writer.buffer[2],
            writer.buffer[3],
        ]);
        assert_eq!(block_count, 1); // One block type (DecoObject)

        // Next should be block_type (1 = DecoObject)
        let block_type = u32::from_le_bytes([
            writer.buffer[4],
            writer.buffer[5],
            writer.buffer[6],
            writer.buffer[7],
        ]);
        assert_eq!(block_type, BlockType::DecoObject as u32);

        // Then block_offset
        let _block_offset = u32::from_le_bytes([
            writer.buffer[8],
            writer.buffer[9],
            writer.buffer[10],
            writer.buffer[11],
        ]);
        // Offset should be 12 (4 for block_count + 8 for header entry)
    }
}
