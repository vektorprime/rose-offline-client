# Bevy 0.15.4 WGSL Shader Comprehensive Documentation

## Table of Contents

1. [Introduction](#introduction)
2. [Shader Pipeline Overview](#shader-pipeline-overview)
3. [Shader Types and Entry Points](#shader-types-and-entry-points)
4. [Vertex Shaders](#vertex-shaders)
5. [Fragment Shaders](#fragment-shaders)
6. [Compute Shaders](#compute-shaders)
7. [Shader Imports and Modularity](#shader-imports-and-modularity)
8. [Bindings and Uniforms](#bindings-and-uniforms)
9. [Material System Integration](#material-system-integration)
10. [Shader Preprocessing](#shader-preprocessing)
11. [Built-in Bevy Shader Functions](#built-in-bevy-shader-functions)
12. [Advanced Techniques](#advanced-techniques)
13. [Performance Considerations](#performance-considerations)
14. [Common Gotchas and Pitfalls](#common-gotchas-and-pitfalls)
15. [Debugging Shaders](#debugging-shaders)

---

## Introduction

WGSL (WebGPU Shading Language) is the shader language used by Bevy's rendering backend. In Bevy 0.15.4, shaders are tightly integrated with the ECS architecture and the render graph system.

### Key Concepts

- **Shader Source**: WGSL code stored as `.wgsl` files or embedded strings
- **Shader Imports**: Bevy's module system for shader code reuse
- **Shader Defs**: Preprocessor-like conditional compilation
- **Bindings**: GPU resources (uniforms, textures, storage buffers)
- **Vertex/Fragment/Compute**: Three shader stages in Bevy

---

## Shader Pipeline Overview

### Bevy's Render Pipeline Architecture

```
┌─────────────────┐
│   Asset Load    │
│   (.wgsl file)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Shader Imports  │
│   Resolution    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Shader Defs    │
│   Processing    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ WGSL → SPIR-V   │
│   Compilation   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  GPU Pipeline   │
│    Creation     │
└─────────────────┘
```

### File Structure

```
assets/shaders/
├── my_shader.wgsl          # Main shader file
├── common/
│   ├── math.wgsl           # Reusable math functions
│   └── lighting.wgsl       # Lighting utilities
└── compute/
    └── particle.wgsl       # Compute shader example
```

---

## Shader Types and Entry Points

### Entry Point Attributes

WGSL uses attributes to mark shader entry points:

```wgsl
// Vertex shader entry point
@vertex
fn vertex(/* inputs */) -> VertexOutput {
    // vertex processing
}

// Fragment shader entry point
@fragment
fn fragment(/* inputs */) -> @location(0) vec4<f32> {
    // fragment processing
}

// Compute shader entry point
@compute @workgroup_size(8, 8, 1)
fn compute(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // compute processing
}
```

### Critical Gotchas

1. **Entry point names matter**: Bevy expects `vertex`, `fragment`, and `compute` as default names unless specified otherwise in `SpecializedRenderPipeline`
2. **Multiple entry points**: A single `.wgsl` file can contain multiple entry points for different stages
3. **Unused entry points**: If you have both vertex and fragment in one file, both will be compiled even if only one is used

---

## Vertex Shaders

### Standard Vertex Shader Structure

```wgsl
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_view_bindings

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    #ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
    #endif
    #ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
    #endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    #ifdef VERTEX_COLORS
    @location(3) color: vec4<f32>,
    #endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    // Get mesh data
    var model = mesh_functions::get_model_matrix(vertex.instance_index);
    
    // Transform position to world space
    out.world_position = model * vec4<f32>(vertex.position, 1.0);
    
    // Transform position to clip space
    out.clip_position = mesh_functions::mesh_position_local_to_clip(
        model,
        vec4<f32>(vertex.position, 1.0)
    );
    
    // Transform normal to world space
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );
    
    out.uv = vertex.uv;
    
    #ifdef VERTEX_COLORS
    out.color = vertex.color;
    #endif
    
    return out;
}
```

### Vertex Input Attributes

#### Standard Mesh Attributes

| Location | Type | Purpose | Always Available |
|----------|------|---------|------------------|
| 0 | vec3<f32> | Position | Yes |
| 1 | vec3<f32> | Normal | Yes |
| 2 | vec2<f32> | UV | Yes |
| 3 | vec4<f32> | Tangent | Conditional (#ifdef VERTEX_TANGENTS) |
| 4 | vec4<f32> | Color | Conditional (#ifdef VERTEX_COLORS) |
| 5 | vec4<u32> | Joint Indices | Conditional (skinning) |
| 6 | vec4<f32> | Joint Weights | Conditional (skinning) |

#### Custom Vertex Attributes

To add custom vertex attributes:

```rust
// Rust side
#[derive(Component)]
struct CustomVertexAttribute;

impl MeshVertexAttribute for CustomVertexAttribute {
    const ATTRIBUTE: MeshVertexAttribute = MeshVertexAttribute::new(
        "CustomAttribute",
        2748678923, // Unique ID
        VertexFormat::Float32x4
    );
}
```

```wgsl
// Shader side
@location(7) custom_attribute: vec4<f32>,
```

### Built-in Vertex Functions (from bevy_pbr::mesh_functions)

```wgsl
// Transform position from local to clip space
fn mesh_position_local_to_clip(model: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32>

// Transform position from local to world space
fn mesh_position_local_to_world(model: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32>

// Transform normal from local to world space
fn mesh_normal_local_to_world(vertex_normal: vec3<f32>, instance_index: u32) -> vec3<f32>

// Transform tangent from local to world space
fn mesh_tangent_local_to_world(model: mat4x4<f32>, vertex_tangent: vec4<f32>) -> vec4<f32>

// Get model matrix for instanced rendering
fn get_model_matrix(instance_index: u32) -> mat4x4<f32>
```

### Vertex Shader Gotchas

1. **Builtin position must be vec4<f32>**: The `@builtin(position)` output must be in clip space with w component
2. **Instance index**: For non-instanced meshes, `instance_index` is always 0
3. **Location conflicts**: Custom locations must not conflict with standard attributes (0-6 are reserved)
4. **Interpolation**: All vertex outputs (except builtins) are linearly interpolated by default
5. **Precision**: Vertex shaders run in full 32-bit precision on most hardware

---

## Fragment Shaders

### Standard Fragment Shader Structure

```wgsl
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_types
#import bevy_pbr::mesh_view_bindings

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @builtin(front_facing) is_front: bool,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    #ifdef VERTEX_COLORS
    @location(3) color: vec4<f32>,
    #endif
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Normalize interpolated normal
    let N = normalize(in.world_normal);
    
    // Sample base color texture
    var base_color = textureSample(
        base_color_texture,
        base_color_sampler,
        in.uv
    );
    
    #ifdef VERTEX_COLORS
    base_color = base_color * in.color;
    #endif
    
    // Lighting calculations
    let view_direction = normalize(view.world_position.xyz - in.world_position.xyz);
    
    // Output final color
    return vec4<f32>(base_color.rgb, base_color.a);
}
```

### Fragment Built-ins

| Built-in | Type | Description |
|----------|------|-------------|
| position | vec4<f32> | Fragment position in screen space |
| front_facing | bool | True if front face, false if back face |
| sample_index | u32 | MSAA sample index |
| sample_mask | u32 | Sample coverage mask |

### Fragment Output Targets

```wgsl
// Single render target
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return color;
}

// Multiple render targets (MRT)
struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) material: vec4<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    out.normal = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    out.material = vec4<f32>(0.5, 0.5, 0.0, 1.0);
    return out;
}
```

### Alpha Blending and Transparency

```wgsl
// Premultiplied alpha
fn premultiply_alpha(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * color.a, color.a);
}

// Alpha cutoff (for alpha mask)
fn alpha_discard(alpha: f32, cutoff: f32) {
    if (alpha < cutoff) {
        discard;
    }
}
```

### Fragment Shader Gotchas

1. **Discard implications**: Using `discard` prevents early-z optimization
2. **Derivative functions**: `dpdx()`, `dpdy()`, `fwidth()` only work in uniform control flow
3. **Non-uniform control flow**: Texture sampling in non-uniform control flow requires `textureSampleLevel`
4. **Interpolation qualifiers**: WGSL doesn't support `flat`, `noperspective` directly (use `@interpolate()`)
5. **Fragment position**: `@builtin(position)` is in pixel coordinates with (0.5, 0.5) at pixel center
6. **Depth output**: Use `@builtin(frag_depth)` to manually write depth (disables early-z)

```wgsl
// Custom interpolation
@location(0) @interpolate(flat) instance_id: u32,
@location(1) @interpolate(linear, center) uv: vec2<f32>,
@location(2) @interpolate(linear, centroid) world_pos: vec3<f32>,
```

---

## Compute Shaders

### Basic Compute Shader Structure

```wgsl
@group(0) @binding(0)
var<storage, read_write> output: array<vec4<f32>>;

@group(0) @binding(1)
var<uniform> params: ComputeParams;

struct ComputeParams {
    count: u32,
    delta_time: f32,
}

@compute @workgroup_size(64, 1, 1)
fn compute(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    if (index >= params.count) {
        return;
    }
    
    // Process data
    output[index] = vec4<f32>(f32(index), params.delta_time, 0.0, 1.0);
}
```

### Workgroup Size Optimization

```wgsl
// Good for 1D data processing
@compute @workgroup_size(256, 1, 1)

// Good for 2D image processing
@compute @workgroup_size(8, 8, 1)

// Good for 3D volume processing
@compute @workgroup_size(4, 4, 4)
```

**Workgroup Size Guidelines:**
- Total size (x * y * z) should be a multiple of 32 (warp/wavefront size)
- Maximum total size is typically 256 or 1024 depending on hardware
- Prefer power-of-2 sizes for better performance
- Consider cache line size and memory access patterns

### Shared Memory (Workgroup Memory)

```wgsl
var<workgroup> shared_data: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn compute(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let local_index = local_id.x;
    
    // Load data into shared memory
    shared_data[local_index] = input[global_id.x];
    
    // Synchronize all threads in workgroup
    workgroupBarrier();
    
    // Use shared data (all threads can now see all data)
    var sum = 0.0;
    for (var i = 0u; i < 256u; i++) {
        sum += shared_data[i];
    }
    
    output[global_id.x] = sum;
}
```

### Synchronization and Barriers

```wgsl
// Wait for all memory operations in workgroup
workgroupBarrier();

// Wait for all storage operations (across workgroups)
storageBarrier();

// Combined barrier (rare, usually workgroupBarrier is enough)
workgroupBarrier();
storageBarrier();
```

### Compute Shader Dispatch from Rust

```rust
// In your compute system
fn dispatch_compute(
    mut commands: Commands,
    pipeline: Res<ComputePipeline>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    let workgroup_count_x = (image_width + 7) / 8;
    let workgroup_count_y = (image_height + 7) / 8;
    
    render_context.command_encoder().dispatch_workgroups(
        workgroup_count_x,
        workgroup_count_y,
        1,
    );
}
```

### Compute Shader Gotchas

1. **Workgroup bounds checking**: Always check `global_invocation_id` against data size
2. **Barrier placement**: Barriers must be in uniform control flow (same path for all threads)
3. **Race conditions**: Without proper barriers, threads can read stale data
4. **Shared memory size**: Limited (typically 16KB-32KB per workgroup)
5. **Atomic operations**: Only available on storage buffers, not uniform buffers
6. **Read-write hazards**: Can't safely read and write same buffer without barriers

---

## Shader Imports and Modularity

### Import System

Bevy uses `#import` directives to include shader modules:

```wgsl
// Import entire module
#import bevy_pbr::mesh_functions

// Import specific functions (not supported - import whole module)
// Use functions with module prefix
let pos = mesh_functions::mesh_position_local_to_clip(model, vertex_pos);
```

### Standard Bevy Shader Imports

```wgsl
// Core mesh utilities
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_types

// PBR lighting
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_types
#import bevy_pbr::lighting

// Shadows
#import bevy_pbr::shadows

// Skinning
#import bevy_pbr::skinning

// Utility functions
#import bevy_pbr::utils
#import bevy_render::maths
#import bevy_render::view
#import bevy_render::globals
```

### Creating Custom Shader Modules

**File: `assets/shaders/custom_utils.wgsl`**

```wgsl
#define_import_path custom::utils

// Constants are automatically available
const PI: f32 = 3.14159265359;
const TWO_PI: f32 = 6.28318530718;

// Functions are available via module prefix
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.13);
    p3 += dot(p3, p3.yzx + 3.333);
    return fract((p3.x + p3.y) * p3.z);
}

fn rotate2D(angle: f32) -> mat2x2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat2x2<f32>(c, -s, s, c);
}
```

**Using the custom module:**

```wgsl
#import custom::utils

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let random = utils::hash(in.uv);
    let rotated_uv = utils::rotate2D(utils::PI * 0.25) * in.uv;
    return vec4<f32>(random, 0.0, 0.0, 1.0);
}
```

### Registering Custom Shader Imports (Rust)

```rust
use bevy::render::render_resource::ShaderLoader;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_shader_imports)
        .run();
}

fn setup_shader_imports(asset_server: Res<AssetServer>) {
    // Shaders in assets/shaders/ with #define_import_path are automatically registered
    // No manual registration needed in Bevy 0.15.4
}
```

### Import Gotchas

1. **Circular imports**: Not allowed, will cause compilation error
2. **Import path case sensitivity**: Paths are case-sensitive
3. **Module prefix required**: Must use `module::function()`, can't import individual functions
4. **No partial imports**: Can't cherry-pick specific functions from a module
5. **Import order doesn't matter**: Imports are resolved automatically
6. **Relative imports**: Not supported, always use absolute paths from import root

---

## Bindings and Uniforms

### Binding Layout

WGSL uses groups and bindings to organize GPU resources:

```
@group(G) @binding(B)
```

- **Group**: Set of related bindings (typically 0-3)
- **Binding**: Specific resource within a group (0-N)

### Standard Bevy Binding Groups

| Group | Purpose | Typical Contents |
|-------|---------|------------------|
| 0 | View | Camera, lights, globals |
| 1 | Material | Textures, material properties |
| 2 | Mesh | Mesh transform, skinning |
| 3 | Custom | User-defined resources |

### Uniform Buffers

```wgsl
struct MaterialUniforms {
    base_color: vec4<f32>,
    roughness: f32,
    metallic: f32,
    emissive: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> material: MaterialUniforms;

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return material.base_color;
}
```

**Rust side:**

```rust
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    base_color: Color,
    #[uniform(0)]
    roughness: f32,
    #[uniform(0)]
    metallic: f32,
    #[uniform(0)]
    emissive: Color,
}
```

### Uniform Buffer Alignment Rules

**Critical**: WGSL has strict alignment requirements:

```wgsl
// BAD - misaligned
struct BadUniforms {
    value1: f32,      // offset 0, size 4
    value2: vec3<f32>, // offset 4 - WRONG! vec3 needs 16-byte alignment
}

// GOOD - properly aligned
struct GoodUniforms {
    value1: f32,        // offset 0, size 4
    _padding: f32,      // offset 4, size 4
    _padding2: f32,     // offset 8, size 4
    _padding3: f32,     // offset 12, size 4
    value2: vec3<f32>,  // offset 16, size 12
}

// BETTER - reorder to avoid padding
struct BetterUniforms {
    value2: vec3<f32>,  // offset 0, size 12
    value1: f32,        // offset 12, size 4
}
```

**Alignment Rules:**
- `f32`, `i32`, `u32`: 4-byte alignment
- `vec2<T>`: 8-byte alignment
- `vec3<T>`: 16-byte alignment (wastes 4 bytes!)
- `vec4<T>`: 16-byte alignment
- `mat2x2<T>`: 8-byte alignment
- `mat3x3<T>`: 16-byte alignment per column
- `mat4x4<T>`: 16-byte alignment per column
- Structs: Aligned to largest member, padded to multiple of 16 bytes

### Storage Buffers

```wgsl
// Read-only storage buffer
@group(0) @binding(0)
var<storage, read> input_data: array<vec4<f32>>;

// Read-write storage buffer
@group(0) @binding(1)
var<storage, read_write> output_data: array<vec4<f32>>;

// Runtime-sized array (length unknown at compile time)
@group(0) @binding(2)
var<storage, read> dynamic_data: array<f32>; // No size specified
```

**Rust side:**

```rust
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct ComputeMaterial {
    #[storage(0, read_only)]
    input_buffer: Handle<RawBufferVec<Vec4>>,
    
    #[storage(1)]
    output_buffer: Handle<RawBufferVec<Vec4>>,
}
```

### Textures and Samplers

```wgsl
// 2D texture and sampler
@group(1) @binding(0)
var base_color_texture: texture_2d<f32>;

@group(1) @binding(1)
var base_color_sampler: sampler;

// Usage
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let color = textureSample(
        base_color_texture,
        base_color_sampler,
        in.uv
    );
    return color;
}

// Cube map
@group(1) @binding(2)
var environment_map: texture_cube<f32>;

@group(1) @binding(3)
var environment_sampler: sampler;

// Sample cube map
let env_color = textureSample(
    environment_map,
    environment_sampler,
    reflect_direction
);

// 3D texture
@group(1) @binding(4)
var volume_texture: texture_3d<f32>;

// Texture array
@group(1) @binding(5)
var texture_array: texture_2d_array<f32>;

// Storage texture (for compute shaders)
@group(0) @binding(6)
var output_texture: texture_storage_2d<rgba8unorm, write>;
```

**Rust side:**

```rust
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct TexturedMaterial {
    #[texture(0)]
    #[sampler(1)]
    base_color_texture: Handle<Image>,
    
    #[texture(2)]
    #[sampler(3)]
    normal_map: Option<Handle<Image>>,
}
```

### Texture Functions

```wgsl
// Sample with automatic LOD
textureSample(texture, sampler, coords) -> vec4<f32>

// Sample with explicit LOD
textureSampleLevel(texture, sampler, coords, level) -> vec4<f32>

// Sample with bias
textureSampleBias(texture, sampler, coords, bias) -> vec4<f32>

// Sample with gradient
textureSampleGrad(texture, sampler, coords, ddx, ddy) -> vec4<f32>

// Load texel directly (no filtering)
textureLoad(texture, coords, level) -> vec4<f32>

// Store to storage texture
textureStore(storage_texture, coords, value)

// Get texture dimensions
textureDimensions(texture) -> vec2<u32>
textureDimensions(texture, level) -> vec2<u32>

// Get number of mip levels
textureNumLevels(texture) -> u32

// Get number of samples (MSAA)
textureNumSamples(texture) -> u32
```

### Binding Gotchas

1. **Group/binding uniqueness**: Each (group, binding) pair must be unique
2. **Binding gaps**: You can skip binding numbers, but avoid for compatibility
3. **Storage buffer alignment**: Same rules as uniforms apply
4. **Texture format matching**: Shader texture type must match actual texture format
5. **Sampler binding**: Textures and samplers must be separate bindings
6. **Storage texture formats**: Limited formats available (rgba8unorm, rgba16float, etc.)
7. **Uniform buffer size limit**: Typically 64KB, use storage buffers for large data
8. **Dynamic offsets**: Not directly exposed in WGSL, handled by Bevy's binding system

---

## Material System Integration

### Creating a Custom Material

**Shader file: `assets/shaders/custom_material.wgsl`**

```wgsl
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_view_bindings

struct CustomMaterial {
    base_color: vec4<f32>,
    time: f32,
    intensity: f32,
}

@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@group(1) @binding(1)
var base_texture: texture_2d<f32>;

@group(1) @binding(2)
var base_sampler: sampler;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let model = mesh_functions::get_model_matrix(vertex.instance_index);
    out.world_position = model * vec4<f32>(vertex.position, 1.0);
    out.clip_position = mesh_functions::mesh_position_local_to_clip(
        model,
        vec4<f32>(vertex.position, 1.0)
    );
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );
    out.uv = vertex.uv;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(base_texture, base_sampler, in.uv);
    let animated_color = material.base_color * (sin(material.time) * 0.5 + 0.5);
    return base * animated_color * material.intensity;
}
```

**Rust side:**

```rust
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::pbr::{MaterialPlugin, Material};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<CustomMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, update_material)
        .run();
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    base_color: Color,
    #[uniform(0)]
    time: f32,
    #[uniform(0)]
    intensity: f32,
    
    #[texture(1)]
    #[sampler(2)]
    base_texture: Handle<Image>,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }
    
    fn vertex_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }
    
    // Optional: Customize alpha mode
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    
    // Optional: Specify shader defs
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Add custom pipeline configuration
        Ok(())
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    let material = materials.add(CustomMaterial {
        base_color: Color::rgb(1.0, 0.0, 0.5),
        time: 0.0,
        intensity: 1.0,
        base_texture: asset_server.load("textures/base.png"),
    });
    
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
    
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3.0, 3.0, 3.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn update_material(
    time: Res<Time>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    for (_, material) in materials.iter_mut() {
        material.time = time.elapsed_seconds();
    }
}
```

### Material Extensions

Extending the PBR material:

```rust
use bevy::pbr::{ExtendedMaterial, MaterialExtension};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct MyExtension {
    #[uniform(100)]
    quantize_steps: u32,
}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/my_extension.wgsl".into()
    }
}

type MyMaterial = ExtendedMaterial<StandardMaterial, MyExtension>;

// In shader: assets/shaders/my_extension.wgsl
```

```wgsl
#import bevy_pbr::pbr_fragment

@group(1) @binding(100)
var<uniform> my_extension: MyExtension;

struct MyExtension {
    quantize_steps: u32,
}

@fragment
fn fragment(
    in: pbr_fragment::FragmentInput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // Get the base PBR output
    var pbr_input = pbr_fragment::pbr_input_from_standard_material(in, is_front);
    
    // Modify it
    let steps = f32(my_extension.quantize_steps);
    pbr_input.material.base_color.r = floor(pbr_input.material.base_color.r * steps) / steps;
    pbr_input.material.base_color.g = floor(pbr_input.material.base_color.g * steps) / steps;
    pbr_input.material.base_color.b = floor(pbr_input.material.base_color.b * steps) / steps;
    
    // Return final color
    return pbr_fragment::pbr(pbr_input);
}
```

---

## Shader Preprocessing

### Shader Defs

Bevy uses `#ifdef`, `#ifndef`, `#else`, and `#endif` for conditional compilation:

```wgsl
#ifdef VERTEX_COLORS
    @location(3) color: vec4<f32>,
#endif

#ifndef CUSTOM_FEATURE
    // Default behavior
#else
    // Custom behavior
#endif

#ifdef FEATURE_A
    // Feature A code
#else ifdef FEATURE_B
    // Feature B code
#else
    // Default code
#endif
```

### Setting Shader Defs from Rust

```rust
impl Material for CustomMaterial {
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Add shader defs based on mesh layout
        if layout.contains(Mesh::ATTRIBUTE_COLOR) {
            descriptor.vertex.shader_defs.push("VERTEX_COLORS".into());
            descriptor.fragment.as_mut().unwrap().shader_defs.push("VERTEX_COLORS".into());
        }
        
        // Add custom shader defs
        descriptor.fragment.as_mut().unwrap().shader_defs.push("CUSTOM_LIGHTING".into());
        
        Ok(())
    }
}
```

### Common Bevy Shader Defs

| Shader Def | Purpose |
|------------|---------|
| VERTEX_COLORS | Mesh has vertex colors |
| VERTEX_TANGENTS | Mesh has tangents (for normal mapping) |
| SKINNED | Mesh is skinned |
| STANDARDMATERIAL_NORMAL_MAP | Material has normal map |
| TONEMAP_IN_SHADER | Apply tonemapping in shader |
| DEBAND_DITHER | Apply dithering |
| ENVIRONMENT_MAP | Environment map available |
| SHADOW_FILTER_METHOD_GAUSSIAN | Use Gaussian shadow filtering |

### Shader Import Conditions

```wgsl
#ifdef SKINNED
    #import bevy_pbr::skinning
#endif

#ifdef CUSTOM_LIGHTING
    #import custom::advanced_lighting
#else
    #import bevy_pbr::lighting
#endif
```

### Preprocessor Gotchas

1. **No expressions**: Can't use `#ifdef (A && B)` or `#ifdef A || B`
2. **Nesting limit**: Deeply nested conditions may fail on some platforms
3. **Define order**: Defs must be defined before use, imports can reference them
4. **String comparison**: Defs are simple presence checks, no value comparison
5. **No `#define` values**: Unlike C, can't do `#define VALUE 10`

---

## Built-in Bevy Shader Functions

### View Bindings

```wgsl
#import bevy_render::view

// Access camera/view data
view.view_proj                  // View-projection matrix
view.inverse_view_proj          // Inverse view-projection
view.view                       // View matrix
view.inverse_view               // Inverse view matrix  
view.projection                 // Projection matrix
view.inverse_projection         // Inverse projection
view.world_position             // Camera world position
view.viewport                   // Viewport dimensions (vec4)
```

### Mesh Functions

```wgsl
#import bevy_pbr::mesh_functions

// Get model matrix for instance
mesh_functions::get_model_matrix(instance_index) -> mat4x4<f32>

// Get previous model matrix (for motion vectors)
mesh_functions::get_previous_model_matrix(instance_index) -> mat4x4<f32>

// Transform positions
mesh_functions::mesh_position_local_to_world(model, position) -> vec4<f32>
mesh_functions::mesh_position_local_to_clip(model, position) -> vec4<f32>

// Transform normals
mesh_functions::mesh_normal_local_to_world(normal, instance_index) -> vec3<f32>

// Transform tangents
mesh_functions::mesh_tangent_local_to_world(model, tangent) -> vec4<f32>
```

### Math Utilities

```wgsl
#import bevy_render::maths

// Saturate (clamp to 0-1)
maths::saturate(value) -> f32

// Power preserving sign
maths::powsign(value, power) -> f32

// Safe normalize (returns zero vector if length is zero)
maths::safe_normalize(vec) -> vec3<f32>
```

### Coordinate System Conversions

```wgsl
// UV to NDC (normalized device coordinates)
fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * 2.0 - 1.0;
}

// NDC to UV
fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return ndc * 0.5 + 0.5;
}

// Screen position to world position
fn screen_to_world(screen_pos: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(
        screen_pos.x * 2.0 - 1.0,
        (1.0 - screen_pos.y) * 2.0 - 1.0,
        depth,
        1.0
    );
    let world_pos = view.inverse_view_proj * ndc;
    return world_pos.xyz / world_pos.w;
}
```

### Color Space Functions

```wgsl
// sRGB to linear
fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    return pow(srgb, vec3<f32>(2.2));
}

// Linear to sRGB
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    return pow(linear, vec3<f32>(1.0 / 2.2));
}

// More accurate sRGB conversion
fn srgb_to_linear_accurate(srgb: vec3<f32>) -> vec3<f32> {
    return select(
        pow((srgb + 0.055) / 1.055, vec3<f32>(2.4)),
        srgb / 12.92,
        srgb <= vec3<f32>(0.04045)
    );
}
```

### Noise Functions

```wgsl
// Hash function (pseudo-random)
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

// Value noise
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    
    let u = f * f * (3.0 - 2.0 * f); // Smoothstep
    
    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}
```

---

## Advanced Techniques

### Parallax Occlusion Mapping

```wgsl
fn parallax_occlusion_mapping(
    uv: vec2<f32>,
    view_dir: vec3<f32>,
    height_map: texture_2d<f32>,
    height_sampler: sampler,
    scale: f32,
    num_layers: f32,
) -> vec2<f32> {
    let layer_depth = 1.0 / num_layers;
    var current_layer_depth = 0.0;
    
    let P = view_dir.xy * scale;
    let delta_uv = P / num_layers;
    
    var current_uv = uv;
    var current_height = textureSample(height_map, height_sampler, current_uv).r;
    
    // Step through layers
    for (var i = 0; i < i32(num_layers); i++) {
        if (current_layer_depth >= current_height) {
            break;
        }
        current_uv -= delta_uv;
        current_height = textureSample(height_map, height_sampler, current_uv).r;
        current_layer_depth += layer_depth;
    }
    
    // Occlusion (interpolation between last two heights)
    let prev_uv = current_uv + delta_uv;
    let after_depth = current_height - current_layer_depth;
    let before_depth = textureSample(height_map, height_sampler, prev_uv).r 
        - current_layer_depth + layer_depth;
    
    let weight = after_depth / (after_depth - before_depth);
    return mix(current_uv, prev_uv, weight);
}
```

### Screen Space Reflections

```wgsl
fn screen_space_reflection(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    roughness: f32,
    scene_depth: texture_2d<f32>,
    scene_color: texture_2d<f32>,
) -> vec3<f32> {
    let view_dir = normalize(view.world_position.xyz - world_pos);
    let reflect_dir = reflect(-view_dir, normal);
    
    // Ray march in screen space
    let ray_origin = world_pos;
    let ray_dir = reflect_dir;
    
    var ray_pos = ray_origin;
    let max_steps = 32;
    let step_size = 0.1;
    
    for (var i = 0; i < max_steps; i++) {
        ray_pos += ray_dir * step_size;
        
        // Project to screen space
        let clip_pos = view.view_proj * vec4<f32>(ray_pos, 1.0);
        var screen_pos = clip_pos.xy / clip_pos.w;
        screen_pos = screen_pos * 0.5 + 0.5;
        screen_pos.y = 1.0 - screen_pos.y;
        
        // Check depth
        let scene_depth_val = textureSample(
            scene_depth,
            point_clamp_sampler,
            screen_pos
        ).r;
        
        if (clip_pos.z > scene_depth_val) {
            return textureSample(scene_color, linear_sampler, screen_pos).rgb;
        }
    }
    
    return vec3<f32>(0.0);
}
```

### Vertex Animation (Wind Effect)

```wgsl
fn apply_wind(
    position: vec3<f32>,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    time: f32,
) -> vec3<f32> {
    let wind_direction = vec3<f32>(1.0, 0.0, 0.5);
    let wind_speed = 2.0;
    let wind_strength = 0.3;
    
    // Use world position for variation
    let wave = sin(world_position.x * 0.5 + time * wind_speed) 
             * cos(world_position.z * 0.5 + time * wind_speed * 0.7);
    
    // Affect vertices based on height (top moves more)
    let height_factor = position.y;
    let displacement = wind_direction * wave * wind_strength * height_factor;
    
    return position + displacement;
}
```

### Dissolve Effect

```wgsl
@group(1) @binding(0)
var noise_texture: texture_2d<f32>;

@group(1) @binding(1)
var noise_sampler: sampler;

@group(1) @binding(2)
var<uniform> dissolve_params: DissolveParams;

struct DissolveParams {
    threshold: f32,
    edge_width: f32,
    edge_color: vec4<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let noise = textureSample(noise_texture, noise_sampler, in.uv).r;
    
    // Discard pixels below threshold
    if (noise < dissolve_params.threshold) {
        discard;
    }
    
    // Edge glow
    let edge = dissolve_params.threshold + dissolve_params.edge_width;
    var color = base_color;
    
    if (noise < edge) {
        let t = (noise - dissolve_params.threshold) / dissolve_params.edge_width;
        color = mix(dissolve_params.edge_color, color, t);
    }
    
    return color;
}
```

### Custom Lighting Model

```wgsl
fn custom_lighting(
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    light_color: vec3<f32>,
    base_color: vec3<f32>,
    roughness: f32,
    metallic: f32,
) -> vec3<f32> {
    let N = normalize(normal);
    let V = normalize(view_dir);
    let L = normalize(light_dir);
    let H = normalize(V + L);
    
    let NdotL = max(dot(N, L), 0.0);
    let NdotV = max(dot(N, V), 0.0);
    let NdotH = max(dot(N, H), 0.0);
    let VdotH = max(dot(V, H), 0.0);
    
    // Diffuse (Lambert)
    let diffuse = base_color / 3.14159265359;
    
    // Specular (Cook-Torrance)
    let alpha = roughness * roughness;
    
    // Normal Distribution Function (GGX/Trowbridge-Reitz)
    let alpha2 = alpha * alpha;
    let denom = NdotH * NdotH * (alpha2 - 1.0) + 1.0;
    let D = alpha2 / (3.14159265359 * denom * denom);
    
    // Geometry Function (Smith)
    let k = alpha / 2.0;
    let G1_V = NdotV / (NdotV * (1.0 - k) + k);
    let G1_L = NdotL / (NdotL * (1.0 - k) + k);
    let G = G1_V * G1_L;
    
    // Fresnel (Schlick)
    let F0 = mix(vec3<f32>(0.04), base_color, metallic);
    let F = F0 + (1.0 - F0) * pow(1.0 - VdotH, 5.0);
    
    let specular = (D * G * F) / max(4.0 * NdotV * NdotL, 0.001);
    
    let kD = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    
    return (kD * diffuse + specular) * light_color * NdotL;
}
```

### Triplanar Mapping

```wgsl
fn triplanar_mapping(
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    texture: texture_2d<f32>,
    texture_sampler: sampler,
    scale: f32,
) -> vec4<f32> {
    let blend = abs(world_normal);
    blend = blend / (blend.x + blend.y + blend.z);
    
    let uv_x = world_position.yz * scale;
    let uv_y = world_position.xz * scale;
    let uv_z = world_position.xy * scale;
    
    let color_x = textureSample(texture, texture_sampler, uv_x);
    let color_y = textureSample(texture, texture_sampler, uv_y);
    let color_z = textureSample(texture, texture_sampler, uv_z);
    
    return color_x * blend.x + color_y * blend.y + color_z * blend.z;
}
```

---

## Performance Considerations

### Shader Compilation Costs

1. **Shader permutations**: Each unique combination of shader defs creates a new pipeline
2. **Specialization**: Minimize dynamic shader defs where possible
3. **Compilation time**: Large shaders with many imports take longer to compile
4. **Pipeline cache**: Bevy caches compiled shaders between runs

### Runtime Performance Tips

```wgsl
// BAD - Expensive division in loop
for (var i = 0u; i < 100u; i++) {
    let value = f32(i) / 100.0; // Division every iteration
    // ...
}

// GOOD - Precompute reciprocal
let inv_count = 1.0 / 100.0;
for (var i = 0u; i < 100u; i++) {
    let value = f32(i) * inv_count; // Multiplication is faster
    // ...
}

// BAD - Texture sampling in tight loop without gradient
for (var i = 0u; i < 10u; i++) {
    color += textureSample(tex, samp, uv + offset[i]);
}

// GOOD - Use textureSampleLevel or compute outside loop
let lod = 0.0;
for (var i = 0u; i < 10u; i++) {
    color += textureSampleLevel(tex, samp, uv + offset[i], lod);
}

// BAD - Normalize in vertex shader, interpolate, normalize again in fragment
// (normal interpolation requires renormalization)
@vertex fn vertex() -> VertexOutput {
    out.normal = normalize(transform_normal(in.normal));
}
@fragment fn fragment(in: FragmentInput) {
    let N = normalize(in.normal); // Required, but adds cost
}

// GOOD - If you only need approximate normals
// Pass unnormalized from vertex, normalize once in fragment

// BAD - Complex branching in fragment shader
if (condition_a) {
    // Complex path A
} else if (condition_b) {
    // Complex path B
} else {
    // Complex path C
}

// BETTER - Use select() for simple branches
let result = select(value_b, value_a, condition);

// BEST - Minimize branches, use math instead
let blend = f32(condition);
let result = mix(value_a, value_b, blend);
```

### Memory Bandwidth Optimization

```wgsl
// BAD - Multiple texture fetches
let color1 = textureSample(tex, samp, uv);
let color2 = textureSample(tex, samp, uv + vec2<f32>(0.01, 0.0));
let color3 = textureSample(tex, samp, uv + vec2<f32>(0.0, 0.01));
let color4 = textureSample(tex, samp, uv + vec2<f32>(0.01, 0.01));

// GOOD - Use textureGather if possible (fetches 4 texels in one call)
let gathered = textureGather(0, tex, samp, uv); // channel 0 (red)

// BAD - Reading same uniform multiple times
let value1 = my_uniform.data * 2.0;
let value2 = my_uniform.data * 3.0;
let value3 = my_uniform.data * 4.0;

// GOOD - Cache in local variable
let data = my_uniform.data;
let value1 = data * 2.0;
let value2 = data * 3.0;
let value3 = data * 4.0;
```

### Precision Considerations

```wgsl
// For positions, normals, and critical calculations, use f32
var world_position: vec3<f32>;

// For colors and less critical data, f32 is still standard
// WGSL doesn't have f16 by default, but some implementations support it

// Integer types for indices and counts
var index: u32;
var count: i32;

// Be aware of precision loss
let a = 16777216.0; // 2^24
let b = a + 1.0;
// b might equal a due to f32 precision limits
```

### Compute Shader Optimization

```wgsl
// Use shared memory for frequently accessed data
var<workgroup> shared_data: array<f32, 256>;

@compute @workgroup_size(256)
fn compute(
    @builtin(local_invocation_index) local_idx: u32,
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    // Coalesced memory access (good)
    shared_data[local_idx] = input[global_id.x];
    
    workgroupBarrier();
    
    // All threads can now efficiently access shared data
    var sum = 0.0;
    for (var i = 0u; i < 256u; i++) {
        sum += shared_data[i];
    }
    
    output[global_id.x] = sum;
}

// BAD - Uncoalesced access
shared_data[local_idx] = input[local_idx * 256u]; // Scattered reads

// GOOD - Coalesced access
shared_data[local_idx] = input[local_idx]; // Sequential reads
```

---

## Common Gotchas and Pitfalls

### 1. Binding Group Overlap

```rust
// BAD - Group 1 is used by Material system
#[derive(AsBindGroup)]
struct MyMaterial {
    #[uniform(0)]
    color: Color,
}

// In shader
@group(0) @binding(0)  // This conflicts with view bindings!
var<uniform> my_data: MyData;
```

**Solution**: Use the correct groups:
- Group 0: View (camera, globals)
- Group 1: Material
- Group 2: Mesh
- Group 3+: Custom

### 2. Struct Alignment Issues

```wgsl
// BAD - Will cause alignment errors
struct MyUniforms {
    value1: f32,        // 4 bytes at offset 0
    value2: vec3<f32>,  // Needs 16-byte alignment, but is at offset 4!
}

// GOOD
struct MyUniforms {
    value2: vec3<f32>,  // 12 bytes at offset 0 (aligned to 16)
    value1: f32,        // 4 bytes at offset 12
    // Implicit padding to 16 bytes total
}
```

### 3. Shader Def Scoping

```wgsl
// This won't work as expected
#ifdef VERTEX_COLORS
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) color: vec4<f32>,  // Location 3 might conflict!
}
#else
struct VertexInput {
    @location(0) position: vec3<f32>,
}
#endif
```

**Solution**: Use conditionals inside struct definition:

```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    #ifdef VERTEX_COLORS
    @location(3) color: vec4<f32>,
    #endif
}
```

### 4. Texture Sampling in Non-Uniform Control Flow

```wgsl
// BAD - May not work on all hardware
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var color: vec4<f32>;
    if (in.world_position.y > 0.0) {
        color = textureSample(tex, samp, in.uv); // Error!
    } else {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    return color;
}

// GOOD - Use textureSampleLevel or rearrange code
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let sampled = textureSample(tex, samp, in.uv);
    var color: vec4<f32>;
    if (in.world_position.y > 0.0) {
        color = sampled;
    } else {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    return color;
}
```

### 5. Forgetting to Normalize Interpolated Normals

```wgsl
// BAD
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let N = in.world_normal; // Interpolated normals are NOT unit length
    let lighting = dot(N, light_dir); // Incorrect lighting
    // ...
}

// GOOD
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let N = normalize(in.world_normal); // Always normalize interpolated normals
    let lighting = dot(N, light_dir);
    // ...
}
```

### 6. Depth Buffer Precision

```wgsl
// BAD - Poor depth precision in distance calculations
let depth = in.clip_position.z;

// GOOD - Use proper depth reconstruction
let ndc_depth = in.clip_position.z / in.clip_position.w;
let linear_depth = linearize_depth(ndc_depth);
```

### 7. Color Space Confusion

```wgsl
// Textures are often in sRGB, but calculations should be in linear space

// BAD - Doing calculations in sRGB space
let tex_color = textureSample(tex, samp, uv);
let result = tex_color * 0.5; // Darkening in wrong space

// GOOD - Convert to linear, calculate, convert back if needed
// (Bevy handles this automatically for most textures)
// But if you need manual control:
let tex_color = textureSample(tex, samp, uv);
let linear = srgb_to_linear(tex_color.rgb);
let result = linear * 0.5;
```

### 8. Integer Division Truncation

```wgsl
// BAD - Integer division truncates
let a = 5u;
let b = 2u;
let result = a / b; // result is 2u, not 2.5

// GOOD - Convert to float for accurate division
let result = f32(a) / f32(b); // result is 2.5
```

### 9. Array Indexing with Non-Constant Values

```wgsl
// BAD - Dynamic indexing into constant arrays may not work
const my_values: array<f32, 4> = array<f32, 4>(1.0, 2.0, 3.0, 4.0);
let index = u32(in.uv.x * 4.0);
let value = my_values[index]; // May fail on some hardware

// GOOD - Use uniform/storage buffer or unroll manually
@group(1) @binding(0)
var<uniform> my_values: MyValues;

struct MyValues {
    data: array<f32, 4>,
}
```

### 10. Workgroup Barrier Misuse

```wgsl
// BAD - Barrier in non-uniform control flow
@compute @workgroup_size(64)
fn compute(@builtin(local_invocation_id) local_id: vec3<u32>) {
    if (local_id.x < 32u) {
        workgroupBarrier(); // Deadlock! Only half the threads hit this
    }
    // ...
}

// GOOD - Barrier must be in uniform control flow
@compute @workgroup_size(64)
fn compute(@builtin(local_invocation_id) local_id: vec3<u32>) {
    // All threads execute this
    shared_data[local_id.x] = input[local_id.x];
    
    workgroupBarrier(); // All threads reach this
    
    // Now safe to read shared data
}
```

---

## Debugging Shaders

### Validation Errors

Common WGSL validation errors:

1. **"binding X already used"** - Duplicate binding numbers in same group
2. **"location X already used"** - Duplicate vertex attribute or fragment output location
3. **"struct size does not match"** - Rust struct layout doesn't match WGSL
4. **"resource binding missing"** - Shader expects binding that isn't provided
5. **"entry point 'X' not found"** - Check entry point function name

### Visual Debugging Techniques

```wgsl
// Output intermediate values as colors
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Debug UV coordinates
    return vec4<f32>(in.uv, 0.0, 1.0);
    
    // Debug normals (remap from [-1,1] to [0,1])
    let N = normalize(in.world_normal);
    return vec4<f32>(N * 0.5 + 0.5, 1.0);
    
    // Debug a single channel
    let value = some_calculation();
    return vec4<f32>(value, value, value, 1.0);
    
    // Debug with color coding
    if (condition_a) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red
    } else if (condition_b) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0); // Green
    } else {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0); // Blue
    }
}
```

### Debug Output Techniques

```wgsl
// Create a debug visualization texture
@group(1) @binding(10)
var debug_output: texture_storage_2d<rgba8unorm, write>;

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Normal rendering
    let color = compute_lighting(in);
    
    // Write debug info to storage texture
    let screen_pos = vec2<u32>(in.position.xy);
    textureStore(debug_output, screen_pos, vec4<f32>(debug_value, 0.0, 0.0, 1.0));
    
    return color;
}
```

### Conditional Debug Mode

```rust
// Rust side - add debug shader def
if cfg!(debug_assertions) {
    descriptor.fragment.as_mut().unwrap()
        .shader_defs.push("DEBUG_MODE".into());
}
```

```wgsl
// Shader side
@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    #ifdef DEBUG_MODE
        // Debug visualization
        return vec4<f32>(in.uv, 0.0, 1.0);
    #else
        // Normal rendering
        return compute_final_color(in);
    #endif
}
```

### Performance Profiling

Use tools like:
- **RenderDoc**: Capture and analyze GPU frames
- **PIX** (Windows): DirectX debugging
- **Xcode Instruments** (macOS): Metal debugging
- **Chrome DevTools**: WebGPU profiling

### Common Debug Patterns

```wgsl
// Checkerboard pattern for UV debugging
fn checkerboard(uv: vec2<f32>, scale: f32) -> f32 {
    let scaled_uv = uv * scale;
    let cell = floor(scaled_uv);
    return f32((u32(cell.x) + u32(cell.y)) % 2u);
}

// Visualize depth
fn visualize_depth(depth: f32, near: f32, far: f32) -> vec3<f32> {
    let linear = (2.0 * near) / (far + near - depth * (far - near));
    return vec3<f32>(linear);
}

// Heat map visualization
fn heat_map(value: f32) -> vec3<f32> {
    let r = smoothstep(0.0, 0.33, value) - smoothstep(0.66, 1.0, value);
    let g = smoothstep(0.0, 0.33, value) * smoothstep(1.0, 0.66, value);
    let b = smoothstep(0.66, 1.0, value);
    return vec3<f32>(r, g, b);
}
```

---

## Complete Example: Advanced Custom Material

Here's a complete example combining many techniques:

**Rust: `custom_material.rs`**

```rust
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::pbr::{Material, MaterialPlugin};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct AdvancedMaterial {
    #[uniform(0)]
    pub base_color: Color,
    
    #[uniform(0)]
    pub roughness: f32,
    
    #[uniform(0)]
    pub metallic: f32,
    
    #[uniform(0)]
    pub emissive: Color,
    
    #[uniform(0)]
    pub time: f32,
    
    #[uniform(0)]
    pub wave_speed: f32,
    
    #[uniform(0)]
    pub wave_amplitude: f32,
    
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<Image>>,
    
    #[texture(3)]
    #[sampler(4)]
    pub normal_map: Option<Handle<Image>>,
    
    #[texture(5)]
    #[sampler(6)]
    pub noise_texture: Handle<Image>,
}

impl Material for AdvancedMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/advanced_material.wgsl".into()
    }
    
    fn fragment_shader() -> ShaderRef {
        "shaders/advanced_material.wgsl".into()
    }
    
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayout,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
            descriptor.vertex.shader_defs.push("VERTEX_TANGENTS".into());
            descriptor.fragment.as_mut().unwrap()
                .shader_defs.push("VERTEX_TANGENTS".into());
        }
        Ok(())
    }
}
```

**WGSL: `assets/shaders/advanced_material.wgsl`**

```wgsl
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_types
#import bevy_pbr::lighting

struct AdvancedMaterial {
    base_color: vec4<f32>,
    roughness: f32,
    metallic: f32,
    _padding1: f32,
    _padding2: f32,
    emissive: vec4<f32>,
    time: f32,
    wave_speed: f32,
    wave_amplitude: f32,
    _padding3: f32,
}

@group(1) @binding(0)
var<uniform> material: AdvancedMaterial;

@group(1) @binding(1)
var base_texture: texture_2d<f32>;

@group(1) @binding(2)
var base_sampler: sampler;

@group(1) @binding(3)
var normal_map: texture_2d<f32>;

@group(1) @binding(4)
var normal_sampler: sampler;

@group(1) @binding(5)
var noise_texture: texture_2d<f32>;

@group(1) @binding(6)
var noise_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    #ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
    #endif
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    #ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
    #endif
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let model = mesh_functions::get_model_matrix(vertex.instance_index);
    
    // Apply wave deformation
    var modified_position = vertex.position;
    let noise_uv = vertex.uv + material.time * 0.1;
    let noise = textureSampleLevel(
        noise_texture,
        noise_sampler,
        noise_uv,
        0.0
    ).r;
    
    let wave = sin(vertex.position.x * 2.0 + material.time * material.wave_speed) * noise;
    modified_position.y += wave * material.wave_amplitude;
    
    out.world_position = model * vec4<f32>(modified_position, 1.0);
    out.clip_position = mesh_functions::mesh_position_local_to_clip(
        model,
        vec4<f32>(modified_position, 1.0)
    );
    
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );
    
    #ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        model,
        vertex.tangent
    );
    #endif
    
    out.uv = vertex.uv;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample base texture
    var base_color = material.base_color;
    base_color *= textureSample(base_texture, base_sampler, in.uv);
    
    // Normal mapping
    var world_normal = normalize(in.world_normal);
    
    #ifdef VERTEX_TANGENTS
    let normal_map_sample = textureSample(normal_map, normal_sampler, in.uv).xyz;
    let tangent_normal = normal_map_sample * 2.0 - 1.0;
    
    let N = normalize(in.world_normal);
    let T = normalize(in.world_tangent.xyz);
    let B = cross(N, T) * in.world_tangent.w;
    let TBN = mat3x3<f32>(T, B, N);
    
    world_normal = normalize(TBN * tangent_normal);
    #endif
    
    // Simple lighting calculation
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    
    // Emissive pulsing
    let pulse = (sin(material.time * 2.0) * 0.5 + 0.5);
    let emissive = material.emissive.rgb * pulse;
    
    // Combine
    let ambient = base_color.rgb * 0.1;
    let final_color = ambient + emissive;
    
    return vec4<f32>(final_color, base_color.a);
}
```

This documentation should provide you with everything needed to create complex, production-quality shaders in Bevy 0.15.4.
