# Asset Loading System Architecture

## 1. Overview
The asset loading system is a customized integration of the Bevy `AssetServer` with a specialized Virtual Filesystem (VFS). It is designed to handle legacy game formats, compressed textures, and complex zone-based loading while providing a priority-based override system where local files on disk take precedence over archived assets.

## 2. AssetServer Configuration
The system overrides the default Bevy asset reading mechanism by registering a custom `AssetReader`.

### VfsAssetIo
`VfsAssetIo` implements the `AssetReader` trait. It serves as the primary gateway for all asset requests.
- **Default Source**: Registered as `AssetSourceId::Default` via the `VfsAssetReaderPlugin`.
- **Specialized Source**: A `zone_loader` source is registered separately to bypass standard existence checks for virtual `.zone_loader` files.
- **Path Normalization**: It handles path trimming (e.g., removing `.no_skin` or `.zmo_texture` suffixes) to ensure consistent VFS lookups.

## 3. Custom Asset Loaders

### DdsImageLoader (`src/dds_image_loader.rs`)
Handles DirectX texture files (`.dds`).
- **Format Conversion**: To avoid Bevy 0.13.2 panics related to compressed texture pixel size calculations, all formats are converted to `R8G8B8A8` (RGBA8).
- **Supported Formats**: 
  - Uncompressed: `R8G8B8`, `B8G8R8`, `R8G8B8A8`, `B8G8R8A8`, `A1R5G5B5`, `R5G6B5`, `A4R4G4B4`, `B5G6R5`.
  - Compressed (BC/DXT): `BC1/DXT1`, `BC2/DXT3`, `BC3/DXT5`.
- **Special Features**: Supports cube maps by detecting the "cube" label in the `LoadContext`.

### ZmsAssetLoader (`src/zms_asset_loader.rs`)
Loads Rose model formats (`.zms`).
- **Vertex Attributes**: Populates `Mesh` with position, normal, tangent, color, joint weights (`ATTRIBUTE_JOINT_WEIGHT`), and joint indices (`ATTRIBUTE_JOINT_INDEX`).
- **UV Sets**: Supports up to four UV channels (UV1-UV4), mapped to `ATTRIBUTE_UV_0` through `MESH_ATTRIBUTE_UV_3`.
- **Non-Skins**: `ZmsNoSkinAssetLoader` provides a path to load meshes without joint data, which is critical for effect meshes to avoid bind group layout mismatches in the render pipeline.

### ZoneLoader (`src/zone_loader.rs`)
Manages the asynchronous loading of game zones.
- **Asset Type**: Implements `AssetLoader` for `.zone_loader` files.
- **Data Aggregation**: Loads `.zon` (zone definition), `.zsc` (constant and deco), and block files (`.him` for heightmaps, `.til` for tiles, `.ifo` for objects, `.lit` for lightmaps).
- **Memory Monitoring**: Integrates with a Windows-specific `memory_monitor` to track resident and virtual memory deltas during zone transitions.

### DialogLoader (`src/ui/dialog_loader.rs`)
Parses UI dialog definitions.
- **Format**: XML based, parsed using `quick_xml`.
- **Workflow**: Loads the XML structure into a `Dialog` asset, which is then used to construct a widget tree in the UI system.

## 4. Virtual Filesystem (VFS)
The VFS allows the game to treat archived data as a standard directory structure.

### VfsAssetIo Implementation (`src/vfs_asset_io.rs`)
The reader follows a strict priority order:
1. **Local Cache**: Checks the global `VFS_FILE_CACHE` for the requested path.
2. **Real Filesystem**: Checks the `base_path` (allows developers to override assets by placing files in a local folder).
3. **VFS**: Falls back to the virtual filesystem (AruaVfs, TitanVfs, IrosePh, etc.).
4. **Local Filesystem**: Final fallback for non-base_path local files.

**Shader Exception**: Paths containing `shaders/` bypass the VFS entirely to ensure `load_internal_asset!` works correctly.

## 5. Asset Caching

### Global VFS Cache
A static `OnceLock<RwLock<HashMap<String, Arc<Vec<u8>>>>>` stores raw file bytes. This prevents redundant I/O operations for frequently accessed assets.
- **Cleanup**: `clear_vfs_file_cache()` is called during zone transitions to prevent memory bloat.

### EffectCache (`src/effect_loader.rs`)
A specialized `Resource` that caches `EftFile` structures. Since particle effects are spawned frequently, caching the parsed file structure avoids repeated disk access and parsing overhead.

## 6. Code Examples

### VFS Priority Read Logic
`src/vfs_asset_io.rs:353-425`
```rust
// Priority: Real filesystem -> VFS -> Local
let real_filesystem_path = self.base_path.join(path_str);
if real_filesystem_path.exists() {
    if let Ok(data) = std::fs::read(&real_filesystem_path) {
        return Ok(VecReader::new(self.store_in_cache(path_str, data)));
    }
}
match self.vfs.open_file(path_str) {
    Ok(file) => { /* ... store in cache and return ... */ }
    Err(_) => { /* ... final local fallback ... */ }
}
```

### DDS Format Matching
`src/dds_image_loader.rs:192-248`
```rust
let format = if pf_flags & 0x4 != 0 { // DDPF_FOURCC
    match &pf_four_cc {
        b"DXT1" | b"BC1\0" => DdsFormat::Bc1Dxt1,
        // ... other FourCC matches ...
    }
} else if pf_flags & 0x40 != 0 { // DDPF_RGB
    // ... RGB bit count checks ...
}
```

## 7. Troubleshooting

| Issue | Cause | Solution |
| :--- | :--- | :--- |
| **Asset Not Found** | File missing from both VFS and `base_path` | Verify VFS path in `ZoneList` or add file to local override folder. |
| **Path Resolution Fail** | Case sensitivity or slash direction | `VfsAssetIo` normalizes paths; ensure `VfsPath` is used for VFS calls. |
| **Catastrophic Memory Leak** | Directory scanning in VFS | `VfsAssetIo::read_directory` must return an empty stream (see `src/vfs_asset_io.rs:446`) to prevent Bevys hot-reload from triggering infinite reload loops. |
| **Bind Group Mismatch** | Skinned mesh used for effects | Use `.no_skin` extension via `ZmsNoSkinAssetLoader` for effect meshes. |

## 8. Source File References
- **Bevy Source**: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_asset\src\`
- **VFS Core**: `src/vfs_asset_io.rs`
- **Image Loading**: `src/dds_image_loader.rs`
- **Model Loading**: `src/zms_asset_loader.rs`
- **Zone Logic**: `src/zone_loader.rs`
- **Effect Logic**: `src/effect_loader.rs`
- **UI Loading**: `src/ui/dialog_loader.rs`