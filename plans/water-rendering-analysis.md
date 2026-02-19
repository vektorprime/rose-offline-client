# Water Rendering Issue Analysis

## Problem Summary
Water in the game appears as solid blue and doesn't render correctly with the animated textures and transparency effects.

---

## Current Implementation Analysis (Bevy 0.15 - Broken)

### Water Material Creation ([`src/zone_loader.rs:1880-1899`](src/zone_loader.rs:1880))
```rust
let water_material = {
    let mut water_material_textures = Vec::with_capacity(25);
    for i in 1..=25 {
        let path = format!("3DDATA/JUNON/WATER/OCEAN01_{:02}.DDS", i);
        let handle = asset_server.load(&path);
        water_material_textures.push(handle);
    }

    let material = standard_materials.add(bevy::pbr::StandardMaterial {
        base_color_texture: water_material_textures.first().cloned(), // ❌ Only uses first texture!
        unlit: true,
        ..Default::default()
    });
    material
};
```

**Issues:**
- Uses `StandardMaterial` instead of a custom water material
- Only the first texture is bound - the other 24 animated frames are ignored
- `unlit: true` bypasses lighting but doesn't provide water effects

### Water Shader ([`src/render/shaders/rose_water_extension.wgsl`](src/render/shaders/rose_water_extension.wgsl))
```wgsl
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    out.color = apply_pbr_lighting(pbr_input);  // ❌ Standard PBR, no water effects
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
```

**Issues:**
- Shader doesn't reference any water-specific uniforms or textures
- No texture animation
- No transparency/alpha blending
- No zone lighting integration

### RoseWaterExtension ([`src/render/water_material_extension.rs`](src/render/water_material_extension.rs))
```rust
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct RoseWaterExtension {
    #[uniform(100)]
    pub uv_animation_params: Vec4,          // ❌ Never used in shader

    #[texture(101, dimension = "2d")]
    #[sampler(102)]
    pub water_texture: Option<Handle<Image>>, // ❌ Never used in shader
}
```

**Issues:**
- Extension defines bind group entries but the shader never uses them
- The extension is registered but the material uses `StandardMaterial`, not `ExtendedMaterial`

---

## Working Implementation Analysis (Bevy 0.11)

### WaterMaterial ([`exjam/rose-offline-client/src/render/water_material.rs`](../exjam-rose-offline-client/rose-offline-client/src/render/water_material.rs))
```rust
pub struct WaterMaterial {
    pub textures: Vec<Handle<Image>>,  // ✅ All 25 textures
}

impl Material for WaterMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend  // ✅ Proper transparency
    }

    fn specialize(...) {
        // ✅ Additive blending for water glow effect
        color_target_state.blend = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,  // Additive!
                operation: BlendOperation::Add,
            },
            ...
        });
        
        // ✅ Depth write disabled for transparency
        descriptor.depth_stencil.as_mut().unwrap().depth_write_enabled = false;
        
        // ✅ Zone lighting integration
        descriptor.layout.insert(3, pipeline.data.zone_lighting_layout.clone());
        
        // ✅ Push constants for animation
        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..WaterPushConstantData::SHADER_SIZE.get() as u32,
        });
    }
}
```

### Animation System
```rust
#[derive(Clone, ShaderType, Resource)]
pub struct WaterPushConstantData {
    pub current_index: i32,  // Current texture frame
    pub next_index: i32,     // Next texture frame
    pub next_weight: f32,    // Blend factor
}

fn extract_water_push_constant_data(mut commands: Commands, time: Extract<Res<Time>>) {
    let time = time.elapsed_seconds_wrapped() * 10.0;
    let current_index = (time as i32) % 25;
    let next_index = (current_index + 1) % 25;
    let next_weight = time.fract();
    
    commands.insert_resource(WaterPushConstantData { current_index, next_index, next_weight });
}
```

### Water Shader ([`exjam/rose-offline-client/src/render/shaders/water_material.wgsl`](../exjam-rose-offline-client/rose-offline-client/src/render/shaders/water_material.wgsl))
```wgsl
@group(1) @binding(0)
var water_array_texture: binding_array<texture_2d<f32>>;  // ✅ Texture array

var<push_constant> water_texture_index: WaterTextureIndex;  // ✅ Animation data

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // ✅ Sample two textures and blend
    let color1 = textureSample(water_array_texture[water_texture_index.current_index], ...);
    let color2 = textureSample(water_array_texture[water_texture_index.next_index], ...);
    let water_color = mix(color1, color2, water_texture_index.next_weight);
    
    // ✅ Apply zone lighting
    return apply_zone_lighting(in.world_position, in.world_normal, water_color, view_z);
}
```

---

## Root Causes

| Issue | Current (Broken) | Working (Bevy 0.11) |
|-------|------------------|---------------------|
| Material Type | `StandardMaterial` | Custom `WaterMaterial` |
| Texture Binding | Single texture | Texture array (25 frames) |
| Animation | None | Push constants with frame blending |
| Blending | None (opaque) | Additive (SrcAlpha + One) |
| Depth Write | Enabled | Disabled |
| Zone Lighting | Not applied | Applied via custom shader |
| Shader | Standard PBR | Custom water shader |

---

## Recommended Fix

### Option A: Port Bevy 0.11 WaterMaterial (Recommended)

Create a custom water material system similar to Bevy 0.11 but adapted for Bevy 0.15:

1. **Create `WaterMaterial`** - Custom material with texture array support
2. **Create animation system** - Extract push constant data based on time
3. **Create water shader** - Sample texture array with animation blending
4. **Configure blending** - Additive blending with depth write disabled
5. **Integrate zone lighting** - Apply zone ambient color

### Option B: Use ExtendedMaterial (Alternative)

Adapt the existing `RoseWaterExtension` to work properly:

1. **Fix the shader** - Actually use the extension's bind group data
2. **Add texture array** - Bind all 25 textures, not just one
3. **Add animation** - Uniform buffer for texture indices
4. **Configure blend state** - Override in specialize()

---

## Implementation Steps for Option A

### Step 1: Create WaterMaterial Component
Location: [`src/render/water_material.rs`](src/render/water_material.rs) (new file)

```rust
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[texture(0, dimension = "2d")]
    #[sampler(1)]
    pub normals: Handle<Image>,
    
    #[uniform(2)]
    pub settings: WaterSettings,
}
```

### Step 2: Create Animation Resource
```rust
#[derive(Clone, ShaderType, Resource)]
pub struct WaterAnimationData {
    pub current_index: i32,
    pub next_index: i32,
    pub next_weight: f32,
}
```

### Step 3: Create Extraction System
```rust
fn extract_water_animation_data(mut commands: Commands, time: Extract<Res<Time>>) {
    let time = time.elapsed_seconds_wrapped() * 10.0;
    commands.insert_resource(WaterAnimationData {
        current_index: (time as i32) % 25,
        next_index: ((time as i32) + 1) % 25,
        next_weight: time.fract(),
    });
}
```

### Step 4: Update spawn_water in zone_loader.rs
- Use `WaterMaterial` instead of `StandardMaterial`
- Bind all 25 textures as array

### Step 5: Create Water Shader
- Use `binding_array<texture_2d<f32>>` for texture array
- Sample and blend two textures based on animation data
- Apply zone lighting

---

## Files to Modify

1. [`src/render/water_material_extension.rs`](src/render/water_material_extension.rs) - Replace with proper WaterMaterial
2. [`src/render/shaders/rose_water_extension.wgsl`](src/render/shaders/rose_water_extension.wgsl) - Replace with water shader
3. [`src/zone_loader.rs`](src/zone_loader.rs) - Update water material creation (lines 1880-1899)
4. [`src/render/mod.rs`](src/render/mod.rs) - Update exports
5. [`src/lib.rs`](src/lib.rs) - Update plugin registration

---

## Reference Files

- Working implementation: [`C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client\src\render\water_material.rs`](../exjam-rose-offline-client/rose-offline-client/src/render/water_material.rs)
- Working shader: [`C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client\src\render\shaders\water_material.wgsl`](../exjam-rose-offline-client/rose-offline-client/src/render/shaders/water_material.wgsl)
- Bevy 0.15 water example: [`C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.15.4\examples\3d\ssr.rs`](../bevy-collection/bevy-0.15.4/examples/3d/ssr.rs)
