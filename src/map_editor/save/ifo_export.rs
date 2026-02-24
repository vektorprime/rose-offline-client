//! IFO File Export System
//! 
//! This module provides functionality to write IFO files in the binary format
//! used by Rose Online. It reverses the loading process from rose-file-readers.

use std::io::{self, Write};
use std::path::Path;
use std::fs::File;

use super::ifo_types::*;

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

    /// Write the IFO file header
    fn write_header(&mut self) -> io::Result<()> {
        // Magic bytes "IFO"
        self.buffer.extend_from_slice(IFO_MAGIC);
        
        // Version (little-endian u32)
        self.buffer.extend_from_slice(&IFO_VERSION.to_le_bytes());
        
        Ok(())
    }

    /// Write a null-terminated string
    fn write_string(&mut self, s: &str) {
        // String length as u16 (little-endian)
        let len = s.len().min(u16::MAX as usize) as u16;
        self.buffer.extend_from_slice(&len.to_le_bytes());
        
        // String bytes (not null-terminated in IFO format, length-prefixed)
        self.buffer.extend_from_slice(s.as_bytes());
    }

    /// Write a single IfoObject
    fn write_object(&mut self, obj: &IfoObject) {
        // Object ID (u32)
        self.buffer.extend_from_slice(&obj.object_id.to_le_bytes());
        
        // Position (3 x f32)
        for &v in &obj.position {
            self.buffer.extend_from_slice(&v.to_le_bytes());
        }
        
        // Rotation (4 x f32 - quaternion)
        for &v in &obj.rotation {
            self.buffer.extend_from_slice(&v.to_le_bytes());
        }
        
        // Scale (3 x f32)
        for &v in &obj.scale {
            self.buffer.extend_from_slice(&v.to_le_bytes());
        }
    }

    /// Write an event object
    fn write_event_object(&mut self, obj: &IfoEventObject) {
        self.write_object(&obj.object);
        self.write_string(&obj.quest_trigger_name);
        self.write_string(&obj.script_function_name);
    }

    /// Write a warp object
    fn write_warp_object(&mut self, obj: &IfoWarpObject) {
        self.write_object(&obj.object);
        // Warp ID (u16)
        self.buffer.extend_from_slice(&obj.warp_id.to_le_bytes());
    }

    /// Write a sound object
    fn write_sound_object(&mut self, obj: &IfoSoundObject) {
        self.write_object(&obj.object);
        self.write_string(&obj.sound_path);
        // Range (f32)
        self.buffer.extend_from_slice(&obj.range.to_le_bytes());
    }

    /// Write an effect object
    fn write_effect_object(&mut self, obj: &IfoEffectObject) {
        self.write_object(&obj.object);
        self.write_string(&obj.effect_path);
    }

    /// Write a water plane
    fn write_water_plane(&mut self, plane: &IfoWaterPlane) {
        // Start position (3 x f32)
        for &v in &plane.start {
            self.buffer.extend_from_slice(&v.to_le_bytes());
        }
        // End position (3 x f32)
        for &v in &plane.end {
            self.buffer.extend_from_slice(&v.to_le_bytes());
        }
    }

    /// Write a complete IFO block to the buffer
    pub fn write_block(&mut self, block: &IfoBlock) -> io::Result<()> {
        self.buffer.clear();
        self.write_header()?;

        // Write block coordinates
        self.buffer.extend_from_slice(&block.block_x.to_le_bytes());
        self.buffer.extend_from_slice(&block.block_z.to_le_bytes());

        // Write water size
        self.buffer.extend_from_slice(&block.water_size.to_le_bytes());

        // Write water planes count and data
        let water_count = block.water_planes.len() as u32;
        self.buffer.extend_from_slice(&water_count.to_le_bytes());
        for plane in &block.water_planes {
            self.write_water_plane(plane);
        }

        // Write decoration objects count and data
        let deco_count = block.deco_objects.len() as u32;
        self.buffer.extend_from_slice(&deco_count.to_le_bytes());
        for obj in &block.deco_objects {
            self.write_object(obj);
        }

        // Write construction objects count and data
        let cnst_count = block.cnst_objects.len() as u32;
        self.buffer.extend_from_slice(&cnst_count.to_le_bytes());
        for obj in &block.cnst_objects {
            self.write_object(obj);
        }

        // Write event objects count and data
        let event_count = block.event_objects.len() as u32;
        self.buffer.extend_from_slice(&event_count.to_le_bytes());
        for obj in &block.event_objects {
            self.write_event_object(obj);
        }

        // Write warp objects count and data
        let warp_count = block.warp_objects.len() as u32;
        self.buffer.extend_from_slice(&warp_count.to_le_bytes());
        for obj in &block.warp_objects {
            self.write_warp_object(obj);
        }

        // Write sound objects count and data
        let sound_count = block.sound_objects.len() as u32;
        self.buffer.extend_from_slice(&sound_count.to_le_bytes());
        for obj in &block.sound_objects {
            self.write_sound_object(obj);
        }

        // Write effect objects count and data
        let effect_count = block.effect_objects.len() as u32;
        self.buffer.extend_from_slice(&effect_count.to_le_bytes());
        for obj in &block.effect_objects {
            self.write_effect_object(obj);
        }

        // Write animated objects count and data
        let animated_count = block.animated_objects.len() as u32;
        self.buffer.extend_from_slice(&animated_count.to_le_bytes());
        for obj in &block.animated_objects {
            self.write_object(obj);
        }

        // Write NPCs count and data
        let npc_count = block.npcs.len() as u32;
        self.buffer.extend_from_slice(&npc_count.to_le_bytes());
        for obj in &block.npcs {
            self.write_object(obj);
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

/// Export all IFO blocks for a zone
pub fn export_zone_ifo_files(
    zone_data: &ZoneExportData,
    output_dir: &Path,
) -> io::Result<ExportStats> {
    let mut stats = ExportStats::default();

    for block_data in zone_data.blocks.iter().filter_map(|b| b.as_ref()) {
        let file_name = block_data.file_name();
        let file_path = output_dir.join(&file_name);

        match export_ifo_block(&block_data.block, &file_path) {
            Ok(size) => {
                stats.blocks_exported += 1;
                stats.bytes_written += size;
                log::info!(
                    "[IFO Export] Exported {} ({} bytes)",
                    file_name,
                    size
                );
            }
            Err(e) => {
                stats.blocks_failed += 1;
                log::error!(
                    "[IFO Export] Failed to export {}: {}",
                    file_name,
                    e
                );
            }
        }
    }

    stats.total_objects = zone_data.total_objects();
    Ok(stats)
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
            self.blocks_exported,
            self.total_objects,
            self.bytes_written,
            self.blocks_failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_header() {
        let mut writer = IfoWriter::new();
        writer.write_header().unwrap();
        
        assert_eq!(&writer.buffer[0..3], b"IFO");
    }

    #[test]
    fn test_write_object() {
        let mut writer = IfoWriter::new();
        let obj = IfoObject {
            object_id: 42,
            position: [100.0, 200.0, 300.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        };
        
        writer.write_object(&obj);
        
        // Object should take: 4 (id) + 12 (pos) + 16 (rot) + 12 (scale) = 44 bytes
        assert_eq!(writer.buffer.len(), 44);
    }

    #[test]
    fn test_write_string() {
        let mut writer = IfoWriter::new();
        writer.write_string("test");
        
        // Length prefix (2 bytes) + string bytes (4 bytes)
        assert_eq!(writer.buffer.len(), 6);
        assert_eq!(&writer.buffer[0..2], &[4, 0]); // Length as little-endian u16
        assert_eq!(&writer.buffer[2..6], b"test");
    }
}
