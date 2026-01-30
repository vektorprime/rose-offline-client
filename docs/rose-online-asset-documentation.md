# Rose Online Game Asset Technical Documentation

Based on comprehensive analysis of the `rose-file-readers` source code, this document provides detailed technical specifications for Rose Online's asset formats, enabling engine reimplementation and shader development.

---

## 1. Model Formats (ZMS - Zelu Mesh Static)

### 1.1 ZMS File Structure

**Magic Headers:** `ZMS0005`, `ZMS0006`, `ZMS0007`, `ZMS0008`

**Format Flags (bitmask):**
| Flag | Bit | Description |
|------|-----|-------------|
| POSITION | 1 | Vertex positions (3x f32) |
| NORMAL | 2 | Vertex normals (3x f32) |
| COLOR | 3 | Vertex colors (4x f32 RGBA) |
| BONE_WEIGHT | 4 | Skinning weights (4x f32) |
| BONE_INDEX | 5 | Bone indices (4x u16 in v8, 4x u32 in v5-6) |
| TANGENT | 6 | Tangents for normal mapping (3x f32) |
| UV1 | 7 | Texture coordinates channel 1 (2x f32) |
| UV2 | 8 | Lightmap/secondary UV (2x f32) |
| UV3 | 9 | Third UV channel (2x f32) |
| UV4 | 10 | Fourth UV channel (2x f32) |

### 1.2 Version Differences

**Version 5/6:**
- Positions scaled by 100.0 (must divide by 100)
- Bone indices stored as u32 and remapped through bone lookup table
- Per-vertex ID prefix before each vertex attribute
- Triangle indices: u32 → u16 conversion

**Version 7/8:**
- Direct memory layout reading (no per-vertex IDs)
- Bone indices stored as u16
- Added `strip_indices` for triangle strips
- Added `material_num_faces` for multi-material meshes
- Added `pool_type` field (v8 only)

### 1.3 Skeleton Format (ZMD - Zelu Mesh Deform)

**Magic Headers:** `ZMD0002`, `ZMD0003`

```rust
struct ZmdBone {
    parent: u16,           // Parent bone index (0xFFFF for root)
    position: Vec3<f32>,   // Local position
    rotation: Quat4<f32>,  // Quaternion in WXYZ order (v3), identity for v2
}
```

**Structure:**
1. Bone count (u32)
2. For each bone: parent (u32→u16), name (null-terminated), position, rotation
3. Dummy bone count (u32) - attachment points, not part of skeleton
4. For each dummy: name, parent, position, rotation

### 1.4 Object Container (ZSC - Zelu Scene Container)

The ZSC format is a scene graph that assembles meshes, materials, and animations into game objects.

**Structure:**
```
[mesh_count: u16] → array of null-terminated mesh paths (.ZMS)
[material_count: u16] → array of ZscMaterial
[effect_count: u16] → array of null-terminated effect paths (.EFT)
[object_count: u16] → array of ZscObject
```

**ZscMaterial Properties:**
- `path`: Texture path (VFS path)
- `is_skin`: Enable skinning
- `alpha_enabled`: Alpha blending
- `two_sided`: Disable backface culling
- `alpha_test`: Alpha test threshold (0-1)
- `z_write_enabled`/`z_test_enabled`: Depth buffer settings
- `blend_mode`: Normal (0) or Lighten (1)
- `specular_enabled`: Enable specular lighting
- `alpha`: Global alpha value
- `glow`: Emissive glow effect (Simple/Light/TextureLight/Alpha)

**ZscObjectPart Properties (tag-value pairs):**
- ID 1: Position (Vec3)
- ID 2: Rotation (Quaternion XYZW)
- ID 3: Scale (Vec3)
- ID 5: Bone index (u16)
- ID 6: Dummy index (u16)
- ID 7: Parent part index (u16, 0 = none)
- ID 29: Collision shape + flags
- ID 30: Animation path

---

## 2. Animation Format (ZMO - Zelu Motion)

**Magic Header:** `ZMO0002`

**Header:**
- FPS: u32
- Num frames: u32
- Channel count: u32

**Channel Types:**
| Type | ID | Data |
|------|-----|------|
| Empty | 1 | No data |
| Position | 2 | Vec3 per frame |
| Rotation | 4 | Quaternion (WXYZ) per frame |
| Normal | 8 | Vec3 per frame |
| Alpha | 16 | f32 per frame |
| UV1-UV4 | 32/64/128/256 | Vec2 per frame |
| Texture | 512 | f32 texture index |
| Scale | 1024 | f32 scale value |

**Extended Data (EZMO / 3ZMO):**
- Frame events stored at end of file
- `total_attack_frames`: Count of attack events (frames 10, 20-28, 56-57, 66-67)
- `interpolation_interval_ms` (3ZMO only): Animation blending time

---

## 3. Zone/World File Formats

### 3.1 ZON - Zone Configuration

**Block-based structure** with block type, offset pairs:

**Block Types:**
| ID | Name | Content |
|----|------|---------|
| 0 | ZoneInfo | Grid per patch, grid size |
| 1 | EventPositions | Named spawn points |
| 2 | Textures | Tile texture paths |
| 3 | Tiles | Tile definitions |
| 4 | Economy | (unused) |

**ZonTile Structure:**
```rust
struct ZonTile {
    layer1: u32,      // Primary texture index
    layer2: u32,      // Secondary texture index (blending)
    offset1: u32,     // Texture atlas offset for layer 1
    offset2: u32,     // Texture atlas offset for layer 2
    blend: bool,      // Enable layer blending
    rotation: ZonTileRotation, // 1=None, 2=FlipH, 3=FlipV, 4=Flip, 5=CW90, 6=CCW90
}
```

### 3.2 IFO - Zone Object Placement

**Block-based structure** for world objects:

| Block ID | Object Type |
|----------|-------------|
| 1 | DecoObject - Decorative static objects |
| 2 | NPC - Non-player characters |
| 3 | CnstObject - Construction/buildings |
| 4 | SoundObject - Ambient sound emitters |
| 5 | EffectObject - Particle effects |
| 6 | AnimatedObject - Animated scenery |
| 8 | MonsterSpawn - Monster spawn points |
| 9 | WaterPlanes - Water surface definitions |
| 10 | Warp - Zone transition portals |
| 11 | CollisionObject - Invisible collision |
| 12 | EventObject - Quest/script triggers |

**Common Object Structure:**
```rust
struct IfoObject {
    object_name: String,      // u8-length prefixed
    warp_id: u16,
    event_id: u16,
    object_type: u32,
    object_id: u32,
    minimap_position: Vec2<u32>,
    rotation: Quat4<f32>,     // XYZW order
    position: Vec3<f32>,
    scale: Vec3<f32>,
}
```

### 3.3 HIM - Heightmap

```rust
struct HimFile {
    width: u32,      // Grid width
    height: u32,     // Grid height
    heights: Vec<f32>, // width * height f32 values
}
```
- 8 bytes skipped after dimensions (likely bounds data)
- Row-major storage order

### 3.4 TIL - Tile Indices

```rust
struct TilFile {
    width: u32,
    height: u32,
    tiles: Vec<u32>, // Tile index per grid cell
}
```
- 3 bytes skipped per tile before reading u32 index

---

## 4. Light Map System (LIT)

Lightmaps are baked lighting textures applied to objects:

```rust
struct LitObject {
    id: u32,                    // Object ID matching IFO
    parts: Vec<LitObjectPart>,  // Per-mesh-part lightmaps
}

struct LitObjectPart {
    object_part_index: u32,     // Which mesh part this applies to
    filename: String,           // Lightmap texture path
    parts_per_row: u32,         // Atlas layout
    part_index: u32,           // Position in atlas
}
```

**Usage:** Lightmaps are typically applied using UV2 coordinates in the ZMS mesh.

---

## 5. UV Mapping and Coordinate Systems

### 5.1 Coordinate System

**Rose Online uses a right-handed coordinate system:**
- **X:** Right
- **Y:** Up (height)
- **Z:** Forward/Into screen

### 5.2 UV Coordinate Conventions

- **UV1:** Primary texture coordinates (0-1 range)
- **UV2:** Lightmap coordinates (may be scaled for atlasing)
- **UV3/UV4:** Additional channels for special effects

### 5.3 Rotation Conventions

- **Quaternions:** Stored as WXYZ (ZMD, ZMO) or XYZW (IFO, ZSC)
- **Euler angles:** Pitch/Yaw/Roll (X/Y/Z rotation order)
- **Tile rotation:** 90-degree increments + flips

### 5.4 Winding Order

Based on the rendering implementation:
- Triangle winding appears to be **counter-clockwise**
- `two_sided` material flag disables backface culling

---

## 6. Texture and Material System

### 6.1 Texture References

Textures are referenced by VFS paths with extensions:
- `.DDS` - DirectDraw Surface (compressed textures)
- `.TGA` - Targa (uncompressed/alpha)
- `.BMP` - Bitmap (UI elements)

### 6.2 TSI - Texture/Sprite Index

Used for UI sprites and atlases:
```rust
struct TsiSprite {
    texture_id: u16,
    left, top, right, bottom: i32, // Rectangle in texture
    name: String[32],
}
```

### 6.3 Material Blend Modes

| Mode | Description |
|------|-------------|
| Normal | Standard alpha blending |
| Lighten | Additive/Screen blending |

### 6.4 Glow Types

| Type | Description |
|------|-------------|
| Simple | Uniform glow color |
| Light | Light-style glow |
| TextureLight | Glow modulated by texture |
| Alpha | Glow modulated by alpha |

---

## 7. Particle and Effect System

### 7.1 PTL - Particle Definition

```rust
struct PtlSequence {
    name: String,
    life: Range<f32>,           // Particle lifetime
    emit_rate: Range<f32>,      // Particles per second
    num_loops: i32,
    emit_radius: Range<Vec3>,   // Spawn area
    gravity: Range<Vec3>,       // Force applied
    texture_path: VfsPathBuf,
    num_particles: i32,
    align_type: u32,           // Billboard mode
    update_coords: PtlUpdateCoords, // World/Local/LocalPosition
    texture_atlas: (cols, rows), // Sprite sheet layout
    blend_modes: (src, dst, op), // D3D blend settings
    keyframes: Vec<PtlKeyframe>,
}
```

**Keyframe Types:**
- SizeXY, Timer, Red/Green/Blue/Alpha, ColourRGBA
- VelocityX/Y/Z/XYZ, Texture (atlas index), Rotation

### 7.2 EFT - Effect Container

Combines particles and meshes for complex effects:
- References PTL files for particles
- References ZMS/ZMO for animated meshes
- Supports sound effects
- Delay and repeat counts
- Link to parent object

---

## 8. Character Definition (CHR)

Links skeletons, models, motions, and effects for characters:

```rust
struct NpcModelData {
    name: String,
    skeleton_index: u16,       // Index into skeleton_files
    model_ids: Vec<u16>,       // Indices into ZSC objects
    motion_ids: Vec<(motion_id, motion_file_index)>,
    effect_ids: Vec<(motion_id, effect_file_index)>,
}
```

**Structure:**
1. Skeleton file list (null-terminated strings)
2. Motion file list (null-terminated strings)
3. Effect file list (null-terminated strings)
4. NPC definitions (selectively enabled via u8 flags)

---

## 9. Virtual File System (VFS)

### 9.1 Standard VFS

**Index file format:**
```
base_version: u32
current_version: u32
num_vfs: u32
For each VFS:
  filename: u16-length EUC-KR string
  offset: u32 (to file table in index)
```

**File entry:**
```
filename: u16-length EUC-KR
offset: u32 (within VFS data file)
size: u32
block_size: u32
is_deleted: u8
is_compressed: u8
is_encrypted: u8
version: u32
crc: u32
```

### 9.2 Path Normalization

All VFS paths:
1. Convert backslashes to forward slashes
2. Remove duplicate slashes
3. Convert to UPPERCASE
4. Trim whitespace

### 9.3 TitanROSE VFS

Custom encrypted format:
- Filename hash using proprietary algorithm
- XOR encryption on header blocks
- Hash table lookup for file access

---

## 10. Asset Dependency Chains

### 10.1 Character Loading Chain
```
CHR file
  → Skeleton (ZMD)
  → Models (ZSC)
    → Meshes (ZMS)
    → Materials
      → Textures (DDS/TGA)
    → Animations (ZMO)
  → Effects (EFT)
    → Particles (PTL)
      → Textures
```

### 10.2 Zone Loading Chain
```
ZON file
  → HIM (heightmaps)
  → TIL (tile indices)
  → IFO (object placement)
    → Object models (ZSC)
      → Meshes (ZMS)
      → Materials/Textures
    → Effect objects (EFT)
    → Water planes
  → LIT (lightmaps)
    → Lightmap textures
```

### 10.3 Effect Loading Chain
```
EFT file
  → Particle files (PTL)
    → Textures
  → Mesh files (ZMS)
  → Mesh animations (ZMO)
  → Sound files
```

---

## 11. String Encoding

- **Default:** EUC-KR (Korean character set)
- **Wide strings:** UTF-16LE (used in some formats)
- **Null-terminated:** Most string fields
- **Length-prefixed:** u8 or u32 prefix for variable-length strings

---

## 12. Binary Data Layout

**Endianness:** Little-endian for all multi-byte values

**Float format:** IEEE 754 single-precision (f32)

**Common vector layouts:**
- Vec2: x, y (f32)
- Vec3: x, y, z (f32)
- Vec4: x, y, z, w (f32)
- Quat4 (WXYZ): w, x, y, z (f32)
- Quat4 (XYZW): x, y, z, w (f32)

---

## 13. Rendering Pipeline Considerations

### 13.1 For Shader Development

**Vertex attributes available:**
```glsl
in vec3 POSITION;      // Always present
in vec3 NORMAL;        // For lighting
in vec4 COLOR;         // Vertex color modulation
in vec4 BONE_WEIGHTS;  // Skinning weights
in uvec4 BONE_INDICES; // Skinning indices
in vec3 TANGENT;       // Normal mapping
in vec2 UV1;           // Diffuse texture
in vec2 UV2;           // Lightmap
in vec2 UV3/UV4;       // Additional channels
```

**Uniform requirements:**
- Bone matrices for skinning (indexed by BONE_INDICES)
- Material flags (alpha test, two-sided, etc.)
- Glow parameters
- Lightmap texture (UV2)

### 13.2 Coordinate Transformations

```
Local space → Bone transform (if skinned) → World space → View → Projection
```

For non-skinned objects in ZSC:
```
Local space → Part transform (pos/rot/scale) → World space
```

---

This documentation provides the foundation for implementing a complete Rose Online asset pipeline in any graphics engine supporting standard 3D rendering features including skeletal animation, lightmapping, and particle effects.
