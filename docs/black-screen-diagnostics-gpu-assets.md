# GPU Resource Management & Asset Loading Diagnostic Guide

> **Version**: 1.0 | **Bevy Version**: 0.13.2 | **Project**: Rose Online Client  
> **Purpose**: Comprehensive diagnostic documentation for GPU resources, memory management, and asset loading failures

---

## Table of Contents

1. [GPU Resource Management Diagnostics](#1-gpu-resource-management-diagnostics)
2. [Asset Loading State Diagnostics](#2-asset-loading-state-diagnostics)
3. [Texture Asset Diagnostics](#3-texture-asset-diagnostics)
4. [Mesh Asset Diagnostics](#4-mesh-asset-diagnostics)
5. [VFS (Virtual File System) Diagnostics](#5-vfs-virtual-file-system-diagnostics)
6. [Async Asset Streaming](#6-async-asset-streaming)
7. [Diagnostic Tools and Techniques](#7-diagnostic-tools-and-techniques)
8. [Asset Preprocessing Validation](#8-asset-preprocessing-validation)

---

## 1. GPU Resource Management Diagnostics

### 1.1 GPU Memory Allocation Tracking

Bevy 0.13.2 uses WGPU for GPU resource management. Memory allocation failures manifest as black screens or application panics.

#### Key Resources to Monitor

```rust
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderApp;

// Query WGPU limits
fn gpu_memory_diagnostics(render_device: Res<RenderDevice>) {
    let limits = render_device.limits();
    log::info!("[GPU MEMORY] max_texture_dimension_1d: {}", limits.max_texture_dimension_1d);
    log::info!("[GPU MEMORY] max_texture_dimension_2d: {}", limits.max_texture_dimension_2d);
    log::info!("[GPU MEMORY] max_texture_dimension_3d: {}", limits.max_texture_dimension_3d);
    log::info!("[GPU MEMORY] max_texture_array_layers: {}", limits.max_texture_array_layers);
    log::info!("[GPU MEMORY] max_bind_groups: {}", limits.max_bind_groups);
    log::info!("[GPU MEMORY] max_bindings_per_bind_group: {}", limits.max_bindings_per_bind_group);
    log::info!("[GPU MEMORY] max_sampled_textures_per_shader_stage: {}", 
        limits.max_sampled_textures_per_shader_stage);
    log::info!("[GPU MEMORY] max_samplers_per_shader_stage: {}", 
        limits.max_samplers_per_shader_stage);
    log::info!("[GPU MEMORY] max_uniform_buffer_binding_size: {}", 
        limits.max_uniform_buffer_binding_size);
    log::info!("[GPU MEMORY] max_storage_buffer_binding_size: {}", 
        limits.max_storage_buffer_binding_size);
}
```

#### Common GPU Memory Failure Modes

| Failure Mode | Symptoms | Diagnostic | Resolution |
|--------------|----------|------------|------------|
| Texture OOM | `CreateTextureError`, black screen | Check texture dimensions against `max_texture_dimension_2d` | Downsample textures, use texture streaming |
| Buffer OOM | `CreateBufferError`, panic on mesh load | Monitor buffer sizes vs `max_buffer_size` | Split large meshes, use indexed geometry |
| Bind Group Exhaustion | `CreateBindGroupError::TooMany` | Count active bind groups vs `max_bind_groups` | Reuse bind groups, use bindless descriptors |
| Uniform Overflow | `max_uniform_buffer_binding_size` exceeded | Validate uniform buffer sizes | Use storage buffers for large data |

### 1.2 Buffer Allocation Failures

Mesh and uniform buffer allocation failures are common causes of black screens.

#### Mesh Buffer Diagnostics

```rust
use bevy::render::mesh::Mesh;
use bevy::render::render_asset::RenderAssets;

fn mesh_buffer_diagnostics(
    meshes: Res<Assets<Mesh>>,
    render_meshes: Res<RenderAssets<Mesh>>,
) {
    for (handle_id, mesh) in meshes.iter() {
        let vertex_count = mesh.count_vertices();
        let index_count = mesh.indices().map(|i| i.len()).unwrap_or(0);
        
        log::info!("[MESH BUFFER] Handle: {:?}", handle_id);
        log::info!("[MESH BUFFER]   Vertices: {}", vertex_count);
        log::info!("[MESH BUFFER]   Indices: {}", index_count);
        log::info!("[MESH BUFFER]   Attributes: {:?}", mesh.attribute_names());
        
        // Check if GPU buffer exists
        let handle = meshes.get_handle(handle_id);
        if let Some(gpu_mesh) = render_meshes.get(&handle) {
            log::info!("[MESH BUFFER]   GPU buffer: OK");
        } else {
            log::warn!("[MESH BUFFER]   GPU buffer: NOT FOUND - possible allocation failure");
        }
    }
}
```

#### Critical Buffer Size Limits

```rust
// Check if mesh exceeds GPU limits
fn validate_mesh_for_gpu(mesh: &Mesh, render_device: &RenderDevice) -> Result<(), String> {
    let limits = render_device.limits();
    let vertex_count = mesh.count_vertices() as u64;
    
    // Estimate buffer size (position: 12 bytes + normal: 12 bytes + uv: 8 bytes)
    let estimated_bytes = vertex_count * 32;
    
    if estimated_bytes > limits.max_buffer_size {
        return Err(format!(
            "Mesh too large: {} bytes exceeds limit {}",
            estimated_bytes, limits.max_buffer_size
        ));
    }
    
    Ok(())
}
```

### 1.3 Texture Memory Limits

Rose Online DDS textures can be large. The custom [`DdsImageLoader`](src/dds_image_loader.rs:18) converts all textures to `R8G8B8A8` format.

#### Texture Memory Calculation

```rust
fn estimate_texture_memory(width: u32, height: u32, format: TextureFormat) -> u64 {
    let bytes_per_pixel = match format {
        TextureFormat::R8Unorm => 1,
        TextureFormat::Rg8Unorm => 2,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => 4,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb => 4,
        TextureFormat::R16Float => 2,
        TextureFormat::Rg16Float => 4,
        TextureFormat::Rgba16Float => 8,
        TextureFormat::R32Float => 4,
        TextureFormat::Rg32Float => 8,
        TextureFormat::Rgba32Float => 16,
        _ => 4, // Conservative estimate
    };
    
    // Account for mipmaps (roughly 33% more)
    let base_size = (width * height) as u64 * bytes_per_pixel;
    (base_size as f64 * 1.33) as u64
}
```

#### Texture Dimension Validation

```rust
fn validate_texture_dimensions(
    width: u32,
    height: u32,
    render_device: &RenderDevice,
) -> Result<(), String> {
    let limits = render_device.limits();
    
    if width > limits.max_texture_dimension_2d {
        return Err(format!(
            "Texture width {} exceeds max {}",
            width, limits.max_texture_dimension_2d
        ));
    }
    
    if height > limits.max_texture_dimension_2d {
        return Err(format!(
            "Texture height {} exceeds max {}",
            height, limits.max_texture_dimension_2d
        ));
    }
    
    Ok(())
}
```

### 1.4 Bind Group Pool Exhaustion

Bevy 0.13.2 creates bind groups for materials, textures, and uniform buffers.

#### Bind Group Diagnostics

```rust
fn bind_group_diagnostics(
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
) {
    let material_count = materials.len();
    let texture_count = images.len();
    
    // Estimate bind groups: 1 per material + 1 per unique texture combination
    let estimated_bind_groups = material_count + texture_count;
    
    log::info!("[BIND GROUP ESTIMATE] Materials: {}, Textures: {}", 
        material_count, texture_count);
    log::info!("[BIND GROUP ESTIMATE] Estimated bind groups needed: {}", 
        estimated_bind_groups);
}
```

#### Bind Group Layout Validation

```rust
use bevy::render::render_resource::BindGroupLayout;

fn validate_bind_group_layout(layout: &BindGroupLayout, render_device: &RenderDevice) {
    let limits = render_device.limits();
    
    // Count bindings per stage
    let mut vertex_bindings = 0;
    let mut fragment_bindings = 0;
    
    for entry in layout.entries.iter() {
        if entry.visibility.contains(ShaderStages::VERTEX) {
            vertex_bindings += 1;
        }
        if entry.visibility.contains(ShaderStages::FRAGMENT) {
            fragment_bindings += 1;
        }
    }
    
    log::info!("[BIND GROUP LAYOUT] Vertex bindings: {}", vertex_bindings);
    log::info!("[BIND GROUP LAYOUT] Fragment bindings: {}", fragment_bindings);
    
    if vertex_bindings > limits.max_bindings_per_bind_group {
        log::error!("[BIND GROUP LAYOUT] Exceeds vertex binding limit!");
    }
}
```

### 1.5 Resource Leak Detection

Detect GPU resource leaks that cause OOM over time.

```rust
use bevy::diagnostic::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic};

pub const GPU_TEXTURE_COUNT: DiagnosticId = 
    DiagnosticId::from_u128(0x1234567890abcdef);

fn register_gpu_diagnostics(app: &mut App) {
    app.register_diagnostic(
        Diagnostic::new(GPU_TEXTURE_COUNT, "gpu_texture_count", 60)
            .with_suffix(" textures")
    );
}

fn update_gpu_diagnostics(
    images: Res<Assets<Image>>,
    meshes: Res<Assets<Mesh>>,
    mut diagnostics: Diagnostics,
) {
    diagnostics.add_measurement(GPU_TEXTURE_COUNT, || images.len() as f64);
    // Monitor for continuous growth indicating leaks
}
```

---

## 2. Asset Loading State Diagnostics

### 2.1 Asset Server State Validation

The asset server manages all asset loading in Bevy 0.13.2.

```rust
use bevy::asset::{AssetServer, LoadState};

fn asset_server_diagnostics(asset_server: Res<AssetServer>) {
    log::info!("[ASSET SERVER] Server active: true");
    log::info!("[ASSET SERVER] Asset sources: {:?}", 
        // Available via internal APIs - check registration
        "Check VfsAssetReaderPlugin initialization"
    );
}
```

### 2.2 Asset Loading Progress Tracking

Track loading progress for zone assets, models, and textures.

```rust
use bevy::asset::{AssetServer, LoadState};
use bevy::prelude::Handle;

#[derive(Resource, Default)]
pub struct AssetLoadingTracker {
    pub pending_assets: Vec<(String, HandleUntyped)>,
    pub loaded_count: usize,
    pub failed_count: usize,
}

fn track_asset_loading(
    asset_server: Res<AssetServer>,
    mut tracker: ResMut<AssetLoadingTracker>,
) {
    let mut newly_loaded = 0;
    let mut newly_failed = 0;
    
    tracker.pending_assets.retain(|(path, handle)| {
        match asset_server.get_load_state(handle.id()) {
            Some(LoadState::Loaded) => {
                log::info!("[ASSET LOADED] {}", path);
                newly_loaded += 1;
                false // Remove from pending
            }
            Some(LoadState::Failed(err)) => {
                log::error!("[ASSET FAILED] {}: {:?}", path, err);
                newly_failed += 1;
                false // Remove from pending
            }
            Some(LoadState::Loading) => {
                log::debug!("[ASSET LOADING] {}", path);
                true // Keep in pending
            }
            _ => true,
        }
    });
    
    tracker.loaded_count += newly_loaded;
    tracker.failed_count += newly_failed;
    
    log::info!("[ASSET PROGRESS] Loaded: {}, Failed: {}, Pending: {}",
        tracker.loaded_count, tracker.failed_count, tracker.pending_assets.len());
}
```

### 2.3 Handle Validity Checking

Verify handles are strong (referenced) and point to valid assets.

```rust
use bevy::asset::Handle;

fn validate_handle<T: Asset>(
    handle: &Handle<T>,
    assets: &Assets<T>,
    context: &str,
) -> bool {
    // Check if handle is strong (referenced by something)
    let is_strong = handle.is_strong();
    log::info!("[HANDLE VALIDATE] {} - Strong: {}", context, is_strong);
    
    // Check if asset exists
    let exists = assets.get(handle).is_some();
    log::info!("[HANDLE VALIDATE] {} - Asset exists: {}", context, exists);
    
    is_strong && exists
}

// Usage for mesh handles
fn validate_mesh_handles(
    query: Query<&Handle<Mesh>>,
    meshes: Res<Assets<Mesh>>,
) {
    for (i, handle) in query.iter().enumerate() {
        validate_handle(handle, meshes.as_ref(), &format!("Mesh {}", i));
    }
}
```

### 2.4 Asset Dependency Resolution

Rose Online assets have complex dependencies (ZSC -> ZMS, DDS textures, etc.).

```rust
#[derive(Resource)]
pub struct AssetDependencyGraph {
    // Track which assets depend on others
    pub dependencies: HashMap<String, Vec<String>>,
}

fn log_asset_dependencies(
    model_loader: Res<ModelLoader>,
    vfs: Res<VirtualFilesystemResource>,
) {
    // Example: ZSC file dependencies
    log::info!("[ASSET DEPS] Character ZSC depends on:");
    log::info!("[ASSET DEPS]   - Skeleton ZMD (bone hierarchy)");
    log::info!("[ASSET DEPS]   - Mesh ZMS files (geometry)");
    log::info!("[ASSET DEPS]   - Texture DDS files (materials)");
    
    // The model_loader already loads these from VFS
    // Verify they exist:
    if let Ok(_) = vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MBODY.ZSC") {
        log::info!("[ASSET DEPS] LIST_MBODY.ZSC: FOUND");
    }
}
```

### 2.5 Loading State Machine

Bevy 0.13.2 uses `LoadState` enum for asset states.

```rust
use bevy::asset::LoadState;

fn diagnose_load_state(state: LoadState, asset_path: &str) {
    match state {
        LoadState::NotLoaded => {
            log::warn!("[LOAD STATE] {}: Not loaded - handle never used?", asset_path);
        }
        LoadState::Loading => {
            log::info!("[LOAD STATE] {}: Loading - asset reader processing", asset_path);
        }
        LoadState::Loaded => {
            log::info!("[LOAD STATE] {}: Loaded - ready for use", asset_path);
        }
        LoadState::Failed(err) => {
            log::error!("[LOAD STATE] {}: Failed - {:?}", asset_path, err);
            // Common failures:
            // - AssetReaderError::NotFound: VFS path issue
            // - AssetReaderError::Io: File corruption or read error
        }
    }
}
```

---

## 3. Texture Asset Diagnostics

### 3.1 DDS Texture Loading Validation

The [`DdsImageLoader`](src/dds_image_loader.rs:18) handles Rose Online's DDS textures.

#### DDS Loading Diagnostics

```rust
fn dds_loading_diagnostics(
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    for (handle_id, image) in images.iter() {
        let handle = images.get_handle(handle_id);
        let path = asset_server.get_path(&handle)
            .map(|p| p.to_string())
            .unwrap_or_default();
        
        if path.ends_with(".dds") {
            log::info!("[DDS TEXTURE] Path: {}", path);
            log::info!("[DDS TEXTURE]   Size: {}x{}", 
                image.texture_descriptor.size.width,
                image.texture_descriptor.size.height);
            log::info!("[DDS TEXTURE]   Format: {:?}", 
                image.texture_descriptor.format);
            log::info!("[DDS TEXTURE]   Mip levels: {}", 
                image.texture_descriptor.mip_level_count);
            log::info!("[DDS TEXTURE]   Data length: {} bytes", 
                image.data.len());
        }
    }
}
```

#### DDS Format Detection

Based on [`parse_dds_header()`](src/dds_image_loader.rs:118):

```rust
fn diagnose_dds_format(bytes: &[u8]) {
    if bytes.len() < 128 {
        log::error!("[DDS FORMAT] File too small for header");
        return;
    }
    
    // Check magic
    if &bytes[0..4] != b"DDS " {
        log::error!("[DDS FORMAT] Invalid magic bytes");
        return;
    }
    
    let height = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let width = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let mip_count = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
    
    log::info!("[DDS FORMAT] Dimensions: {}x{}", width, height);
    log::info!("[DDS FORMAT] Mipmaps: {}", mip_count);
    
    // Check pixel format flags at offset 76
    let pf_flags = u32::from_le_bytes([bytes[76], bytes[77], bytes[78], bytes[79]]);
    
    if pf_flags & 0x4 != 0 {
        // FourCC format
        let four_cc = std::str::from_utf8(&bytes[80..84]).unwrap_or("????");
        log::info!("[DDS FORMAT] FourCC: {}", four_cc);
        // DXT1, DXT3, DXT5, BC1, BC2, BC3, BC4, BC5, BC6H, BC7
    } else if pf_flags & 0x40 != 0 {
        // RGB format
        let rgb_bits = u32::from_le_bytes([bytes[84], bytes[85], bytes[86], bytes[87]]);
        log::info!("[DDS FORMAT] RGB bits: {}", rgb_bits);
    }
}
```

### 3.2 Texture Format Compatibility

The loader converts all formats to `Rgba8UnormSrgb`.

#### Format Compatibility Matrix

| Source Format | Supported | Conversion Path | Notes |
|---------------|-----------|-----------------|-------|
| R8G8B8 | Yes | CPU convert to RGBA | 24-bit RGB |
| R8G8B8A8 | Yes | Direct load | 32-bit RGBA |
| B8G8R8 | Yes | CPU convert to RGBA | Swizzle required |
| B8G8R8A8 | Yes | CPU convert to RGBA | Swizzle required |
| BC1/DXT1 | Yes | CPU decompress | No alpha or 1-bit alpha |
| BC2/DXT3 | Yes | CPU decompress | 4-bit explicit alpha |
| BC3/DXT5 | Yes | CPU decompress | Interpolated alpha |
| BC4 | Partial | Via image crate | Single channel |
| BC5 | Partial | Via image crate | Two channel |
| BC6H | No | Fallback to image crate | HDR format |
| BC7 | No | Fallback to image crate | High quality |

### 3.3 Mipmap Generation Status

Bevy 0.13.2 does NOT automatically generate mipmaps for loaded textures.

```rust
fn mipmap_diagnostics(image: &Image) {
    let mip_count = image.texture_descriptor.mip_level_count;
    let expected_mips = calculate_mip_levels(
        image.texture_descriptor.size.width,
        image.texture_descriptor.size.height,
    );
    
    log::info!("[MIPMAP] Current: {}, Expected: {}", mip_count, expected_mips);
    
    if mip_count < expected_mips {
        log::warn!("[MIPMAP] Missing mipmaps - may cause aliasing");
    }
}

fn calculate_mip_levels(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height);
    (max_dim as f32).log2().floor() as u32 + 1
}
```

### 3.4 Texture Dimension Limits

Verify textures fit within GPU limits.

```rust
fn validate_texture_against_limits(image: &Image, render_device: &RenderDevice) {
    let limits = render_device.limits();
    let size = image.texture_descriptor.size;
    
    let max_dim = limits.max_texture_dimension_2d;
    
    if size.width > max_dim || size.height > max_dim {
        log::error!("[TEXTURE LIMIT] {}x{} exceeds max {}",
            size.width, size.height, max_dim);
    }
    
    // Rose Online specific: Some zone textures are 2048x2048+
    // Check against common limits:
    // - WebGL2: 2048
    // - Desktop: 16384+
    log::info!("[TEXTURE LIMIT] {}x{} (max allowed: {})",
        size.width, size.height, max_dim);
}
```

### 3.5 GPU Texture Upload Failures

Detect failures in the render world texture upload.

```rust
use bevy::render::texture::GpuImage;
use bevy::render::render_asset::RenderAssets;

fn gpu_texture_upload_diagnostics(
    images: Res<Assets<Image>>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    for (handle_id, _) in images.iter() {
        let handle = images.get_handle(handle_id);
        
        if gpu_images.get(&handle).is_none() {
            log::warn!("[GPU TEXTURE] Handle {:?} not in GPU cache - upload pending or failed", 
                handle_id);
        }
    }
    
    log::info!("[GPU TEXTURE] CPU images: {}, GPU images: {}",
        images.len(), gpu_images.len());
}
```

---

## 4. Mesh Asset Diagnostics

### 4.1 ZMS Model Loading Validation

The [`ZmsAssetLoader`](src/zms_asset_loader.rs:28) loads Rose Online ZMS mesh files.

#### ZMS Loading Diagnostics

```rust
fn zms_loading_diagnostics(
    meshes: Res<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    for (handle_id, mesh) in meshes.iter() {
        let handle = meshes.get_handle(handle_id);
        let path = asset_server.get_path(&handle)
            .map(|p| p.to_string())
            .unwrap_or_default();
        
        if path.ends_with(".zms") {
            log::info!("[ZMS MESH] Path: {}", path);
            log::info!("[ZMS MESH]   Vertices: {}", mesh.count_vertices());
            log::info!("[ZMS MESH]   Indices: {}", 
                mesh.indices().map(|i| i.len()).unwrap_or(0));
            log::info!("[ZMS MESH]   Attributes: {:?}", mesh.attribute_names());
            
            // Check for required attributes
            let has_positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some();
            let has_normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some();
            let has_uvs = mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some();
            
            log::info!("[ZMS MESH]   Has positions: {}", has_positions);
            log::info!("[ZMS MESH]   Has normals: {}", has_normals);
            log::info!("[ZMS MESH]   Has UVs: {}", has_uvs);
        }
    }
}
```

### 4.2 Vertex Format Compatibility

ZMS files support multiple vertex formats. The loader extracts available attributes.

#### Vertex Attribute Validation

```rust
fn validate_vertex_format(mesh: &Mesh) -> Result<(), String> {
    // Required attributes for PBR rendering
    let required = vec![
        Mesh::ATTRIBUTE_POSITION,
    ];
    
    for attr in &required {
        if mesh.attribute(*attr).is_none() {
            return Err(format!("Missing required attribute: {:?}", attr));
        }
    }
    
    // Optional but recommended
    let optional = vec![
        Mesh::ATTRIBUTE_NORMAL,
        Mesh::ATTRIBUTE_UV_0,
        Mesh::ATTRIBUTE_TANGENT,
    ];
    
    for attr in &optional {
        if mesh.attribute(*attr).is_none() {
            log::warn!("[VERTEX FORMAT] Missing optional attribute: {:?}", attr);
        }
    }
    
    Ok(())
}
```

#### ZMS-Specific Attributes

From [`ZmsAssetLoader`](src/zms_asset_loader.rs:34):

| ZMS Field | Bevy Attribute | Usage |
|-----------|----------------|-------|
| `position` | `ATTRIBUTE_POSITION` | Vertex positions |
| `normal` | `ATTRIBUTE_NORMAL` | Vertex normals |
| `tangent` | `ATTRIBUTE_TANGENT` | Tangents for normal maps |
| `color` | `ATTRIBUTE_COLOR` | Vertex colors |
| `uv1` | `ATTRIBUTE_UV_0` | Primary UVs |
| `uv2` | `MESH_ATTRIBUTE_UV_1` | Secondary UVs |
| `uv3` | `MESH_ATTRIBUTE_UV_2` | Third UV set |
| `uv4` | `MESH_ATTRIBUTE_UV_3` | Fourth UV set |
| `bone_indices` | `ATTRIBUTE_JOINT_INDEX` | Skinning bone indices |
| `bone_weights` | `ATTRIBUTE_JOINT_WEIGHT` | Skinning weights |

### 4.3 Index Buffer Integrity

Validate index buffer consistency.

```rust
fn validate_index_buffer(mesh: &Mesh) -> Result<(), String> {
    let vertex_count = mesh.count_vertices();
    
    if let Some(indices) = mesh.indices() {
        let index_count = indices.len();
        log::info!("[INDEX BUFFER] Count: {}", index_count);
        
        // Check for out-of-bounds indices
        let mut max_index = 0;
        for i in 0..index_count {
            let idx = indices.get(i).unwrap() as usize;
            max_index = max_index.max(idx);
            
            if idx >= vertex_count {
                return Err(format!(
                    "Index {} at position {} exceeds vertex count {}",
                    idx, i, vertex_count
                ));
            }
        }
        
        log::info!("[INDEX BUFFER] Max index: {} (vertex count: {})", 
            max_index, vertex_count);
    } else {
        log::warn!("[INDEX BUFFER] No indices - using vertex order");
    }
    
    Ok(())
}
```

### 4.4 Mesh Primitive Topology

ZMS files use triangle lists.

```rust
fn validate_primitive_topology(mesh: &Mesh) {
    use bevy::render::render_resource::PrimitiveTopology;
    
    // ZMS loader always creates TriangleList
    log::info!("[PRIMITIVE TOPOLOGY] Expected: TriangleList");
    
    // Note: In Bevy 0.13.2, topology is set at Mesh creation
    // and cannot be easily queried at runtime
}
```

### 4.5 GPU Mesh Buffer Upload

Monitor GPU buffer creation for meshes.

```rust
use bevy::render::mesh::GpuMesh;
use bevy::render::render_asset::RenderAssets;

fn gpu_mesh_upload_diagnostics(
    meshes: Res<Assets<Mesh>>,
    gpu_meshes: Res<RenderAssets<Mesh>>,
) {
    let cpu_count = meshes.len();
    let gpu_count = gpu_meshes.len();
    
    log::info!("[GPU MESH] CPU meshes: {}, GPU meshes: {}", cpu_count, gpu_count);
    
    if gpu_count < cpu_count {
        log::warn!("[GPU MESH] {} meshes not yet uploaded to GPU",
            cpu_count - gpu_count);
    }
    
    for (handle_id, _) in meshes.iter() {
        let handle = meshes.get_handle(handle_id);
        
        if gpu_meshes.get(&handle).is_none() {
            log::debug!("[GPU MESH] Handle {:?} pending GPU upload", handle_id);
        }
    }
}
```

---

## 5. VFS (Virtual File System) Diagnostics

### 5.1 VFS Initialization State

The [`VfsAssetIo`](src/vfs_asset_io.rs:109) provides asset reading from Rose Online's data archives.

```rust
fn vfs_initialization_diagnostics(
    vfs_io: Option<Res<VfsAssetIo>>,
    vfs_resource: Option<Res<VirtualFilesystemResource>>,
) {
    match vfs_io {
        Some(_) => log::info!("[VFS INIT] VfsAssetIo resource: PRESENT"),
        None => log::error!("[VFS INIT] VfsAssetIo resource: MISSING"),
    }
    
    match vfs_resource {
        Some(_) => log::info!("[VFS INIT] VirtualFilesystem resource: PRESENT"),
        None => log::error!("[VFS INIT] VirtualFilesystem resource: MISSING"),
    }
}
```

### 5.2 File Existence Validation

Verify files exist in the VFS before loading.

```rust
fn vfs_file_existence_diagnostics(vfs: Res<VirtualFilesystemResource>) {
    let test_files = vec![
        "3DDATA/AVATAR/MALE.ZMD",
        "3DDATA/AVATAR/LIST_MBODY.ZSC",
        "3DDATA/AVATAR/LIST_MFACE.ZSC",
        "3DDATA/WEAPON/LIST_WEAPON.ZSC",
        "3DDATA/NPC/PART_NPC.ZSC",
    ];
    
    for path in test_files {
        match vfs.exists(path) {
            true => log::info!("[VFS FILE] {}: EXISTS", path),
            false => log::error!("[VFS FILE] {}: NOT FOUND", path),
        }
    }
}
```

### 5.3 Asset Path Resolution

The VFS resolves paths through [`read()`](src/vfs_asset_io.rs:122).

```rust
fn vfs_path_resolution_diagnostics() {
    log::info!("[VFS PATH] Resolution rules:");
    log::info!("[VFS PATH]   1. Strip .no_skin suffix");
    log::info!("[VFS PATH]   2. Strip .zmo_texture suffix");
    log::info!("[VFS PATH]   3. Handle .zone_loader specially");
    log::info!("[VFS PATH]   4. Bypass VFS for shaders/ paths");
    log::info!("[VFS PATH]   5. Fallback to local filesystem if VFS miss");
}
```

#### Path Normalization Logic

From [`vfs_asset_io.rs`](src/vfs_asset_io.rs:135):

```rust
// Path normalization happens in VfsAssetIo::read()
let path_str = path
    .to_str()
    .unwrap()
    .trim_end_matches(".no_skin")      // ZMS no-skin variant
    .trim_end_matches(".zmo_texture"); // Animation texture reference
```

### 5.4 Archive Mounting Status

Check which archives are mounted.

```rust
fn vfs_archive_diagnostics(vfs: Res<VirtualFilesystemResource>) {
    // The VFS mounts data.idx files
    log::info!("[VFS ARCHIVE] Mounted archives:");
    
    // Typical Rose Online archives:
    // - data.idx (main game data)
    // - patch.idx (updates)
    
    // Note: Actual archive listing depends on rose_file_readers API
    log::info!("[VFS ARCHIVE] Check rose_file_readers::VirtualFilesystem for details");
}
```

### 5.5 File Read Errors

Diagnose VFS read failures.

```rust
fn vfs_read_error_diagnostics(vfs: Res<VirtualFilesystemResource>) {
    let test_path = "3DDATA/AVATAR/INVALID_FILE.ZSC";
    
    match vfs.open_file(test_path) {
        Ok(_) => log::info!("[VFS READ] {}: Success", test_path),
        Err(e) => {
            log::error!("[VFS READ] {}: {:?}", test_path, e);
            // Common errors:
            // - File not found in any archive
            // - Archive read error
            // - Path normalization issue
        }
    }
}
```

#### VFS Error Types

| Error | Cause | Diagnostic |
|-------|-------|------------|
| File not found | Missing asset or wrong path | Verify path case sensitivity |
| Archive error | Corrupted data.idx | Verify file integrity |
| Path parse error | Invalid characters | Check for null bytes |
| IO error | Disk/read failure | Check system resources |

---

## 6. Async Asset Streaming

### 6.1 Asset Loading Futures Status

Bevy 0.13.2 loads assets asynchronously.

```rust
use bevy::tasks::AsyncComputeTaskPool;

fn async_loading_diagnostics(asset_server: Res<AssetServer>) {
    // Asset loading uses bevy::tasks internally
    let task_pool = AsyncComputeTaskPool::get();
    
    log::info!("[ASYNC LOADING] Task pool active");
    log::info!("[ASYNC LOADING] AssetServer using async I/O");
    
    // Check for stuck loads
    // (No direct API, but can infer from load state not changing)
}
```

### 6.2 Background Loading Queue Depth

Monitor the loading queue to detect bottlenecks.

```rust
#[derive(Resource, Default)]
pub struct LoadingQueueMetrics {
    pub queue_depth_history: Vec<usize>,
    pub max_observed_depth: usize,
}

fn track_loading_queue_depth(
    asset_server: Res<AssetServer>,
    tracker: Res<AssetLoadingTracker>,
    mut metrics: ResMut<LoadingQueueMetrics>,
) {
    let depth = tracker.pending_assets.len();
    metrics.queue_depth_history.push(depth);
    metrics.max_observed_depth = metrics.max_observed_depth.max(depth);
    
    if depth > 100 {
        log::warn!("[QUEUE DEPTH] High loading queue: {} assets", depth);
    }
    
    // Keep history bounded
    if metrics.queue_depth_history.len() > 300 {
        metrics.queue_depth_history.remove(0);
    }
}
```

### 6.3 Asset Hot-Reloading State

Bevy 0.13.2 supports hot-reloading for development.

```rust
fn hot_reload_diagnostics(asset_server: Res<AssetServer>) {
    // Check if asset watching is enabled
    log::info!("[HOT RELOAD] Asset watching: Check AssetServer configuration");
    
    // Hot-reload workflow:
    // 1. Asset file changes on disk
    // 2. AssetServer detects change
    // 3. Asset is reloaded
    // 4. Systems using Handle<T> see updated asset
}
```

### 6.4 Load Deadlock Detection

Detect circular dependencies or stuck loads.

```rust
#[derive(Resource)]
pub struct LoadDeadlockDetector {
    pub load_start_times: HashMap<HandleUntyped, std::time::Instant>,
    pub timeout: std::time::Duration,
}

impl Default for LoadDeadlockDetector {
    fn default() -> Self {
        Self {
            load_start_times: HashMap::new(),
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

fn detect_load_deadlocks(
    mut detector: ResMut<LoadDeadlockDetector>,
    asset_server: Res<AssetServer>,
    tracker: Res<AssetLoadingTracker>,
) {
    let now = std::time::Instant::now();
    
    // Check for stuck loads
    for (handle, start_time) in detector.load_start_times.iter() {
        if now.duration_since(*start_time) > detector.timeout {
            log::error!("[DEADLOCK] Asset load timed out after {:?}", detector.timeout);
            log::error!("[DEADLOCK] Handle: {:?}", handle);
            
            // Possible causes:
            // - VFS file read hanging
            // - Asset loader panic (caught by Bevy)
            // - Circular dependency
        }
    }
}
```

---

## 7. Diagnostic Tools and Techniques

### 7.1 Bevy AssetDebug Resource

Bevy 0.13.2 provides asset debugging through reflection.

```rust
fn asset_debug_inspection(
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    materials: Res<Assets<StandardMaterial>>,
) {
    log::info!("[ASSET DEBUG] === Asset Storage Stats ===");
    log::info!("[ASSET DEBUG] Meshes: {}", meshes.len());
    log::info!("[ASSET DEBUG] Images: {}", images.len());
    log::info!("[ASSET DEBUG] Materials: {}", materials.len());
    
    // Iterate all assets with their handles
    for (handle_id, mesh) in meshes.iter() {
        log::debug!("[ASSET DEBUG] Mesh {:?}: {} vertices", 
            handle_id, mesh.count_vertices());
    }
}
```

### 7.2 WGPU Memory Reporting

Enable WGPU memory reporting for GPU resource tracking.

```rust
use bevy::render::renderer::RenderAdapter;
use bevy::render::RenderApp;

fn wgpu_memory_reporting(render_adapter: Res<RenderAdapter>) {
    // Query adapter info
    let info = render_adapter.get_info();
    log::info!("[WGPU MEMORY] Adapter: {}", info.name);
    log::info!("[WGPU MEMORY] Backend: {:?}", info.backend);
    log::info!("[WGPU MEMORY] Device Type: {:?}", info.device_type);
    
    // Note: Detailed memory reporting requires WGPU profiler features
}
```

### 7.3 RenderDoc Resource Inspection

Use RenderDoc to inspect GPU resources:

1. **Launch with RenderDoc**:
   ```bash
   # In RenderDoc, set executable path to your built client
   # Set working directory to project root
   ```

2. **Capture Frame** (F12 during black screen)

3. **Inspect Resources**:
   - **Texture Viewer**: Verify DDS textures loaded correctly
   - **Mesh Output**: Check ZMS mesh vertex positions
   - **Resource Inspector**: View buffer sizes and bind groups

4. **Check for**:
   - Textures showing as black (load failure)
   - Meshes with NaN vertices (ZMS parse error)
   - Missing bind groups (layout mismatch)

### 7.4 Tracy Profiler Integration

Enable Tracy for asset loading profiling.

```toml
# Cargo.toml
[dependencies.bevy]
version = "=0.13.2"
features = [
    "trace_tracy",
    "trace_tracy_memory",
]
```

```rust
// Add tracing spans
fn load_zms_with_tracing(reader: &mut Reader, ...) {
    let _span = info_span!("load_zms", path = ?load_context.path()).entered();
    // ... loading code
}
```

### 7.5 Memory Pressure Detection

Detect when approaching memory limits.

```rust
#[derive(Resource)]
pub struct MemoryPressureMonitor {
    pub last_check: std::time::Instant,
    pub check_interval: std::time::Duration,
}

fn check_memory_pressure(
    mut monitor: ResMut<MemoryPressureMonitor>,
    images: Res<Assets<Image>>,
    meshes: Res<Assets<Mesh>>,
) {
    if monitor.last_check.elapsed() < monitor.check_interval {
        return;
    }
    monitor.last_check = std::time::Instant::now();
    
    // Estimate memory usage
    let mut estimated_texture_bytes: usize = 0;
    for (_, image) in images.iter() {
        estimated_texture_bytes += image.data.len();
    }
    
    let mut estimated_mesh_bytes: usize = 0;
    for (_, mesh) in meshes.iter() {
        // Rough estimate: 32 bytes per vertex
        estimated_mesh_bytes += mesh.count_vertices() * 32;
    }
    
    log::info!("[MEMORY PRESSURE] Estimated texture memory: {:.2} MB",
        estimated_texture_bytes as f64 / 1_048_576.0);
    log::info!("[MEMORY PRESSURE] Estimated mesh memory: {:.2} MB",
        estimated_mesh_bytes as f64 / 1_048_576.0);
}
```

---

## 8. Asset Preprocessing Validation

### 8.1 Shader Preprocessing Cache

Bevy 0.13.2 caches compiled shaders.

```rust
fn shader_cache_diagnostics(render_pipeline_cache: Res<PipelineCache>) {
    // Note: PipelineCache internals are not fully public in 0.13.2
    // Check for shader compilation errors in logs:
    // - "failed to process shader"
    // - "missing binding"
    // - "type mismatch"
    
    log::info!("[SHADER CACHE] Check logs for naga/compile errors");
}
```

### 8.2 Texture Compression Status

The [`DdsImageLoader`](src/dds_image_loader.rs:18) handles compressed textures.

```rust
fn texture_compression_diagnostics(images: Res<Assets<Image>>) {
    for (handle_id, image) in images.iter() {
        let format = image.texture_descriptor.format;
        
        let compression_info = match format {
            TextureFormat::Bc1RgbaUnorm | TextureFormat::Bc1RgbaUnormSrgb => 
                "BC1/DXT1 (GPU compressed)",
            TextureFormat::Bc2RgbaUnorm | TextureFormat::Bc2RgbaUnormSrgb => 
                "BC2/DXT3 (GPU compressed)",
            TextureFormat::Bc3RgbaUnorm | TextureFormat::Bc3RgbaUnormSrgb => 
                "BC3/DXT5 (GPU compressed)",
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => 
                "RGBA8 (uncompressed)",
            _ => "Other format",
        };
        
        log::info!("[TEXTURE COMPRESS] {:?}: {}", handle_id, compression_info);
    }
}
```

### 8.3 Model Data Transformation

ZMS files require coordinate system transformation.

```rust
fn zms_transformation_validation() {
    log::info!("[ZMS TRANSFORM] Coordinate conversion:");
    log::info!("[ZMS TRANSFORM]   Rose: Z-up, right-handed");
    log::info!("[ZMS TRANSFORM]   Bevy: Y-up, right-handed");
    log::info!("[ZMS TRANSFORM]   Transform: (x, y, z) -> (x, z, -y)");
    
    // Applied in ZmsAssetLoader:
    // for vert in zms.position.iter_mut() {
    //     let y = vert[1];
    //     vert[1] = vert[2];  // Z -> Y
    //     vert[2] = -y;       // -Y -> Z
    // }
}
```

### 8.4 Asset Metadata Validation

Check asset metadata for loading hints.

```rust
fn asset_metadata_diagnostics(
    asset_server: Res<AssetServer>,
    meshes: Res<Assets<Mesh>>,
) {
    for (handle_id, _) in meshes.iter() {
        let handle = meshes.get_handle(handle_id);
        
        if let Some(path) = asset_server.get_path(&handle) {
            // Check for labeled assets (e.g., "#material_num_faces")
            let path_str = path.to_string();
            
            if path_str.contains('#') {
                log::info!("[ASSET META] Labeled asset: {}", path_str);
            }
        }
    }
}
```

---

## Quick Reference: Bevy 0.13.2 Asset APIs

### Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `AssetServer` | `bevy::asset` | Central asset management |
| `Assets<T>` | `bevy::asset` | Storage for typed assets |
| `Handle<T>` | `bevy::asset` | Reference to an asset |
| `LoadState` | `bevy::asset` | Loading status enum |
| `AssetLoader` | `bevy::asset` | Trait for custom loaders |
| `AssetReader` | `bevy::asset::io` | Async file reading |

### Key Methods

```rust
// AssetServer
asset_server.load::<Mesh>("path/to/file.zms") -> Handle<Mesh>
asset_server.get_load_state(handle.id()) -> Option<LoadState>
asset_server.get_path(&handle) -> Option<&Path>

// Assets<T>
assets.get(&handle) -> Option<&T>
assets.get_mut(&handle) -> Option<&mut T>
assets.contains(&handle) -> bool
assets.len() -> usize

// Handle<T>
handle.is_strong() -> bool
handle.id() -> AssetId<T>
handle.clone_weak() -> Handle<T>
```

### System Ordering for Asset Diagnostics

```rust
app.add_systems(Update, (
    // Run after asset loading
    asset_loading_diagnostics
        .after(LoadingStateSet),
    
    // Run before render
    gpu_resource_diagnostics
        .before(RenderSet::Extract),
));
```

---

*Document Version: 1.0*  
*Last Updated: Based on Rose Online Client codebase*  
*Compatible with: Bevy 0.13.2, Rose Online ZMS/DDS formats*
