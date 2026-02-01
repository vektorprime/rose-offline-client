# Render Graph, Pipeline & Shader Diagnostics

> **Version**: 1.0 | **Bevy Version**: 0.13.2 | **Project**: Rose Online Client  
> **Purpose**: Comprehensive diagnostic documentation for render graph execution, pipeline cache, shader compilation, and GPU resource binding
> **Parent Document**: [`black-screen-diagnostic-protocol.md`](docs/black-screen-diagnostic-protocol.md)

---

## Table of Contents

1. [Render Graph Execution Diagnostics](#1-render-graph-execution-diagnostics)
2. [Render Phase Diagnostics](#2-render-phase-diagnostics)
3. [Pipeline Cache Diagnostics](#3-pipeline-cache-diagnostics)
4. [Shader Compilation Diagnostics](#4-shader-compilation-diagnostics)
5. [Mesh GPU Buffer Diagnostics](#5-mesh-gpu-buffer-diagnostics)
6. [Bind Group and Texture Diagnostics](#6-bind-group-and-texture-diagnostics)
7. [Depth Texture and Multisampling](#7-depth-texture-and-multisampling)
8. [Draw Call Generation](#8-draw-call-generation)
9. [Render Schedule Sequence](#9-render-schedule-sequence)

---

## 1. Render Graph Execution Diagnostics

### 1.1 Render Graph Architecture

Bevy 0.13.2 uses a directed acyclic graph (DAG) structure for render pass organization. The render graph consists of:

| Component | Type | Purpose |
|-----------|------|---------|
| `RenderGraph` | Resource | Main graph container with all nodes |
| `Node` | Trait | Individual render operations (passes, copies, etc.) |
| `NodeSlot` | Struct | Data passing between nodes (textures, buffers) |
| `RenderGraphApp` | Extension | Plugin API for graph configuration |
| `SubGraph` | Graph | Nested graph for complex render pipelines |

**Key API Paths**:
```rust
use bevy::render::render_graph::{RenderGraph, Node, RenderGraphApp};
use bevy::render::render_resource::{RenderPipeline, RenderPass};
```

### 1.2 Main Render Graph vs Sub-Graphs

#### Main Graph (3D Rendering)

The primary render graph in Bevy 0.13.2 is accessed via:

```rust
// In render app systems
fn debug_main_graph(render_graph: Res<RenderGraph>) {
    // Access main graph
    let main_graph = render_graph.get_graph(Graph::Main);
    
    // List all nodes
    for (node_name, node_id) in main_graph.iter_nodes() {
        info!("Main graph node: {:?}", node_name);
    }
}
```

**Built-in Main Graph Nodes (Bevy 0.13.2)**:

| Node Name | Purpose | Output |
|-----------|---------|--------|
| `MainPassDriver` | Drives main 3D rendering | - |
| `CameraDriverLabel` | Manages camera passes | View textures |
| `ShadowPass` | Shadow map generation | Shadow textures |
| `Prepass` | Depth/normal prepass | Depth texture |
| `DeferredPrepass` | G-buffer generation | G-buffers |
| `MainOpaquePass3D` | Opaque geometry | Color target |
| `MainAlphaMaskPass3D` | Alpha-masked geometry | Color target |
| `MainTransparentPass3D` | Transparent geometry | Color target |
| `Tonemapping` | HDR to LDR conversion | Final color |
| `Upscaling` | Resolution upscaling | Upscaled output |
| `Bloom` | Bloom post-processing | Bloomed output |

#### Sub-Graphs

Sub-graphs are nested render graphs for complex operations:

```rust
use bevy::render::render_graph::{RenderGraph, Graph};

// Register a sub-graph
fn setup_sub_graph(render_app: &mut RenderApp) {
    let mut sub_graph = RenderGraph::default();
    
    // Add nodes to sub-graph
    sub_graph.add_node("custom_pass", CustomPassNode);
    sub_graph.add_node_edge("input", "custom_pass");
    
    // Register as named graph
    render_app.add_sub_graph(Graph::Name("custom_effects".into()), sub_graph);
}
```

**Sub-Graph Use Cases**:
- Post-processing effects chains
- Multi-pass lighting calculations
- Custom shadow techniques
- Reflection probes

### 1.3 Node Execution Order Validation

**Node Dependencies**:

```rust
use bevy::render::render_graph::{RenderGraph, NodeLabel};

fn validate_node_order(render_graph: &RenderGraph) {
    let graph = render_graph.get_graph(Graph::Main);
    
    // Verify expected ordering
    let expected_order = vec![
        "Prepass",
        "ShadowPass", 
        "MainOpaquePass3D",
        "MainAlphaMaskPass3D",
        "MainTransparentPass3D",
        "Tonemapping",
    ];
    
    for (i, expected) in expected_order.iter().enumerate() {
        if let Some(node_id) = graph.get_node_id(NodeLabel::Name(expected)) {
            info!("Node {}: {:?} found at expected position {}", expected, node_id, i);
        } else {
            error!("Node {}: NOT FOUND - expected at position {}", expected, i);
        }
    }
}
```

**Edge Validation**:

```rust
fn validate_node_edges(render_graph: &RenderGraph) {
    let graph = render_graph.get_graph(Graph::Main);
    
    // Check that opaque pass runs before transparent
    if let (Some(opaque), Some(transparent)) = (
        graph.get_node_id(NodeLabel::Name("MainOpaquePass3D")),
        graph.get_node_id(NodeLabel::Name("MainTransparentPass3D"))
    ) {
        if graph.has_edge(opaque, transparent) {
            info!("✓ Correct: Opaque pass → Transparent pass");
        } else {
            error!("✗ Missing edge: Opaque pass should run before Transparent pass");
        }
    }
}
```

### 1.4 Node Dependency Resolution

**Dependency Types**:

| Dependency | API | Purpose |
|------------|-----|---------|
| `NodeEdge` | `add_node_edge(from, to)` | Execution order constraint |
| `SlotEdge` | `add_slot_edge(from, from_slot, to, to_slot)` | Data flow between nodes |

**Diagnostic System - Node Dependency Analysis**:

```rust
use bevy::render::render_graph::{RenderGraph, NodeState, RenderGraphContext};

fn diagnose_node_dependencies(
    render_graph: Res<RenderGraph>,
) {
    let graph = render_graph.get_graph(Graph::Main);
    
    for (node_name, node_id) in graph.iter_nodes() {
        let node_state = graph.get_node_state(node_id).unwrap();
        
        // Get input/output slots
        let inputs: Vec<_> = node_state.input_slots.iter().collect();
        let outputs: Vec<_> = node_state.output_slots.iter().collect();
        
        info!("Node {:?}:", node_name);
        info!("  Inputs: {:?}", inputs);
        info!("  Outputs: {:?}", outputs);
        
        // Check for disconnected required inputs
        for (slot_name, slot) in &inputs {
            if slot.required && !graph.has_input_edge(node_id, slot_name) {
                error!("  ✗ Required input '{}' is not connected!", slot_name);
            }
        }
    }
}
```

### 1.5 RenderGraphApp Configuration

**Plugin Setup Validation**:

```rust
use bevy::render::{RenderApp, RenderSet};
use bevy::render::render_graph::RenderGraphApp;

pub struct CustomRenderPlugin;

impl Plugin for CustomRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.get_sub_app_mut(RenderApp)
            .expect("RenderApp not available");
        
        // Add render systems to specific sets
        render_app.add_systems(Render, (
            prepare_custom_resources.in_set(RenderSet::Prepare),
            queue_custom_draws.in_set(RenderSet::Queue),
        ));
        
        // Add render command
        render_app.add_render_command::<Transparent3d, DrawCustomMesh>();
    }
    
    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app_mut(RenderApp)
            .expect("RenderApp not available");
        
        // Initialize render resources
        render_app.init_resource::<CustomPipeline>();
        render_app.init_resource::<SpecializedRenderPipelines<CustomPipeline>>();
    }
}
```

**Configuration Diagnostics**:

```rust
fn validate_render_graph_app_config(app: &mut App) {
    // Check RenderApp exists
    match app.get_sub_app(RenderApp) {
        Some(render_app) => {
            info!("[RENDER GRAPH] RenderApp is configured");
            
            // Check for required resources
            if render_app.world.contains_resource::<RenderGraph>() {
                info!("[RENDER GRAPH] RenderGraph resource present");
            } else {
                error!("[RENDER GRAPH] RenderGraph resource MISSING");
            }
        }
        None => {
            error!("[RENDER GRAPH] RenderApp NOT FOUND - render plugin not initialized");
        }
    }
}
```

### 1.6 Graph Runner Execution Timing

The render graph runner executes nodes in topological order:

```rust
use bevy::render::renderer::RenderContext;

fn diagnose_graph_execution(
    render_context: &mut RenderContext,
    render_graph: &RenderGraph,
) {
    let start_time = std::time::Instant::now();
    
    // The graph runner executes all nodes
    match render_graph.run(render_context, &RenderGraphContext::default()) {
        Ok(()) => {
            let elapsed = start_time.elapsed();
            info!("[GRAPH EXECUTION] Completed in {:?}", elapsed);
        }
        Err(e) => {
            error!("[GRAPH EXECUTION] FAILED: {:?}", e);
        }
    }
}
```

**Execution Timing Diagnostics**:

| Metric | Normal Range | Investigation Threshold |
|--------|--------------|------------------------|
| Full graph execution | 0.5-5ms | >10ms |
| Shadow pass | 0.1-2ms | >5ms |
| Opaque pass | 0.5-4ms | >8ms |
| Transparent pass | 0.1-2ms | >5ms |
| Post-processing | 0.1-2ms | >5ms |

---

## 2. Render Phase Diagnostics

### 2.1 Render Phase Architecture

Render phases organize draw calls by material properties for correct rendering order.

**Phase Types in Bevy 0.13.2**:

| Phase | Type | Purpose | Render Order |
|-------|------|---------|--------------|
| `Opaque3d` | Phase | Fully opaque geometry | Early Z, front-to-back |
| `AlphaMask3d` | Phase | Alpha-tested geometry | After opaque |
| `Transparent3d` | Phase | Blended transparency | Back-to-front, last |
| `Shadow` | Phase | Shadow casters | Shadow pass |
| `Opaque2d` | Phase | 2D opaque sprites | - |
| `Transparent2d` | Phase | 2D transparent sprites | - |

**Phase Component**:

```rust
use bevy::render::render_phase::RenderPhase;

// Entities in render world get RenderPhase<T> component
#[derive(Component)]
pub struct RenderPhase<T: PhaseItem> {
    pub items: Vec<T>,
}
```

### 2.2 Phase Ordering (Opaque → AlphaMask → Transparent)

**Correct Phase Order**:

```
1. Opaque3d       → Renders first, depth write enabled
2. AlphaMask3d    → Renders second, alpha testing
3. Transparent3d  → Renders last, back-to-front sorting
```

**Diagnostic System - Phase Order Validation**:

```rust
use bevy::core_pipeline::core_3d::{Opaque3d, AlphaMask3d, Transparent3d};

fn validate_phase_ordering(
    opaque_phases: Query<&RenderPhase<Opaque3d>>,
    alpha_mask_phases: Query<&RenderPhase<AlphaMask3d>>,
    transparent_phases: Query<&RenderPhase<Transparent3d>>,
) {
    let opaque_count: usize = opaque_phases.iter()
        .map(|p| p.items.len()).sum();
    let alpha_mask_count: usize = alpha_mask_phases.iter()
        .map(|p| p.items.len()).sum();
    let transparent_count: usize = transparent_phases.iter()
        .map(|p| p.items.len()).sum();
    
    info!("[PHASE DIAGNOSTIC] Phase item counts:");
    info!("  Opaque3d:       {} items", opaque_count);
    info!("  AlphaMask3d:    {} items", alpha_mask_count);
    info!("  Transparent3d:  {} items", transparent_count);
    
    if opaque_count == 0 && alpha_mask_count == 0 && transparent_count == 0 {
        error!("[PHASE DIAGNOSTIC] ✗ ALL PHASES EMPTY - nothing will render!");
    }
}
```

### 2.3 RenderPhase<T> Component Validation

**Phase Item Structure**:

```rust
pub trait PhaseItem: SortedRenderItem {
    type SortKey: Ord;
    
    fn entity(&self) -> Entity;
    fn draw_function(&self) -> DrawFunctionId;
    fn sort_key(&self) -> Self::SortKey;
    fn batch_range(&self) -> &Range<u32>;
    fn batch_range_mut(&mut self) -> &mut Range<u32>;
}
```

**Phase Item Validation**:

```rust
fn diagnose_render_phase_items<T: PhaseItem>(
    phases: Query<(Entity, &RenderPhase<T>)>,
) {
    for (view_entity, phase) in phases.iter() {
        info!("[PHASE] View {:?}: {} items", view_entity, phase.items.len());
        
        for (i, item) in phase.items.iter().enumerate() {
            // Validate draw function exists
            if item.draw_function().is_invalid() {
                error!("[PHASE] Item {} has invalid draw function!", i);
            }
            
            // Validate batch range
            if item.batch_range().is_empty() {
                warn!("[PHASE] Item {} has empty batch range", i);
            }
        }
        
        // Check sorting
        if !is_sorted_by_key(&phase.items, |i| i.sort_key()) {
            error!("[PHASE] Items are NOT properly sorted!");
        }
    }
}
```

### 2.4 Phase Item Submission

**Adding Items to Phases** (from queue systems):

```rust
fn queue_custom_meshes(
    mut transparent_phase: Query<&mut RenderPhase<Transparent3d>>,
    // ... other params
) {
    for mut phase in transparent_phase.iter_mut() {
        phase.add(Transparent3d {
            distance: camera_distance, // For sorting
            pipeline: pipeline_id,
            entity: mesh_entity,
            draw_function: draw_function_id,
            batch_range: 0..1,
            dynamic_offset: None,
        });
    }
}
```

**Submission Diagnostics**:

```rust
fn diagnose_phase_submission<T: PhaseItem>(
    phases: Query<(Entity, &RenderPhase<T>), Changed<RenderPhase<T>>>,
) {
    for (view_entity, phase) in phases.iter() {
        info!("[PHASE SUBMISSION] View {:?}: {} items submitted", 
            view_entity, phase.items.len());
        
        // Analyze distance-based sorting (for transparent)
        let distances: Vec<f32> = phase.items.iter()
            .map(|item| item.sort_key().as_f32())
            .collect();
        
        if distances.windows(2).any(|w| w[0] < w[1]) {
            warn!("[PHASE SUBMISSION] Transparent items not sorted back-to-front!");
        }
    }
}
```

### 2.5 Batch Generation

**Batching Criteria**:

| Property | Must Match for Batching | Description |
|----------|------------------------|-------------|
| Pipeline | Yes | Same render pipeline |
| Draw Function | Yes | Same draw function |
| Material | Yes | Same material bind group |
| Mesh | Often | Same vertex buffer layout |
| Dynamic Offsets | Yes | Same uniform offsets |

**Batch Diagnostics**:

```rust
fn analyze_batch_efficiency<T: PhaseItem>(
    phases: Query<&RenderPhase<T>>,
) {
    for phase in phases.iter() {
        let total_items = phase.items.len();
        let mut batches = 0;
        let mut current_batch = None;
        
        for item in &phase.items {
            let batch_key = (item.pipeline(), item.draw_function());
            
            if current_batch != Some(batch_key) {
                batches += 1;
                current_batch = Some(batch_key);
            }
        }
        
        let efficiency = if total_items > 0 {
            (total_items as f32) / (batches as f32)
        } else {
            0.0
        };
        
        info!("[BATCH ANALYSIS] Items: {}, Batches: {}, Efficiency: {:.2}x",
            total_items, batches, efficiency);
        
        if efficiency < 5.0 && total_items > 10 {
            warn!("[BATCH ANALYSIS] Low batch efficiency - too many state changes!");
        }
    }
}
```

---

## 3. Pipeline Cache Diagnostics

### 3.1 Specialized Mesh Pipeline Validation

**Pipeline Specialization Flow**:

```rust
use bevy::render::render_resource::SpecializedRenderPipeline;
use bevy::pbr::{MeshPipeline, MeshPipelineKey};

pub struct CustomPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub shader: Handle<Shader>,
}

impl SpecializedRenderPipeline for CustomPipeline {
    type Key = MeshPipelineKey;
    
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        
        // Modify descriptor based on key
        if key.contains(MeshPipelineKey::HDR) {
            // Configure for HDR
        }
        
        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            // Configure for prepass
        }
        
        descriptor
    }
}
```

**Specialization Key Components**:

| Key Flag | Description | Diagnostic Check |
|----------|-------------|------------------|
| `HDR` | HDR rendering target | Check view target format |
| `DEPTH_PREPASS` | Depth prepass enabled | Check prepass settings |
| `NORMAL_PREPASS` | Normal prepass enabled | Check prepass settings |
| `MOTION_VECTOR_PREPASS` | Motion vector prepass | Check prepass settings |
| `DEFERRED_PREPASS` | Deferred G-buffer | Check deferred settings |
| `MSAA` | MSAA sample count | Check MSAA resource |
| `SKINNED` | Skinned mesh | Check mesh attributes |
| `MORPH_TARGETS` | Morph target animation | Check mesh attributes |

**Pipeline Specialization Diagnostics**:

```rust
fn diagnose_pipeline_specialization<P: SpecializedRenderPipeline>(
    pipeline_cache: Res<PipelineCache>,
    specialized_pipelines: Res<SpecializedRenderPipelines<P>>,
) {
    let (cached, pending, failed) = specialized_pipelines.statistics();
    
    info!("[PIPELINE CACHE] Specialized pipelines:");
    info!("  Cached:  {}", cached);
    info!("  Pending: {}", pending);
    info!("  Failed:  {}", failed);
    
    if failed > 0 {
        error!("[PIPELINE CACHE] ✗ {} pipelines failed to specialize!", failed);
    }
    
    if pending > cached {
        warn!("[PIPELINE CACHE] More pending than cached - shader compilation backlog");
    }
}
```

### 3.2 Material Pipeline Specialization

**Material Pipeline Key** (from `object_material_simple.rs`):

```rust
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    pub has_lightmap: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub alpha_enabled: bool,
}

impl Material for ObjectMaterial {
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Add shader defines based on key
        if key.bind_group_data.has_lightmap {
            descriptor.vertex.shader_defs.push("LIGHTMAP_UV".into());
            if let Some(ref mut fragment) = descriptor.fragment {
                fragment.shader_defs.push("LIGHTMAP_UV".into());
            }
        }
        
        // Configure depth/stencil
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = key.bind_group_data.z_write_enabled;
        }
        
        // Configure culling
        descriptor.primitive.cull_mode = if key.bind_group_data.two_sided {
            None
        } else {
            Some(Face::Back)
        };
        
        Ok(())
    }
}
```

**Material Specialization Validation**:

```rust
fn validate_material_specialization<M: Material>(
    pipeline_cache: Res<PipelineCache>,
    materials: Res<RenderAssets<M>>,
) {
    for (handle, gpu_material) in materials.iter() {
        if let Some(pipeline) = gpu_material.pipeline_id {
            match pipeline_cache.get_render_pipeline_state(pipeline) {
                CachedRenderPipelineState::Ok(_) => {
                    // Pipeline is ready
                }
                CachedRenderPipelineState::Err(e) => {
                    error!("[MATERIAL SPECIALIZATION] {:?} failed: {:?}", handle, e);
                }
                state => {
                    warn!("[MATERIAL SPECIALIZATION] {:?} state: {:?}", handle, state);
                }
            }
        }
    }
}
```

### 3.3 Pipeline Compilation Errors

**Common Pipeline Compilation Errors**:

| Error | Cause | Resolution |
|-------|-------|------------|
| `ShaderNotLoaded` | Shader handle invalid | Verify shader asset loading |
| `ShaderCompilationError` | WGSL syntax error | Check shader compilation logs |
| `VertexLayoutMismatch` | Vertex attributes don't match | Verify mesh/shader attribute alignment |
| `BindGroupLayoutMismatch` | Layout doesn't match shader | Check bind group layouts |
| `PipelineLayoutError` | Push constant range issue | Validate push constant ranges |

**Error Diagnostics**:

```rust
fn diagnose_pipeline_errors(
    pipeline_cache: Res<PipelineCache>,
) {
    for (id, state) in pipeline_cache.iter() {
        match state {
            CachedRenderPipelineState::Err(e) => {
                error!("[PIPELINE ERROR] {:?}: {:?}", id, e);
                
                match e {
                    PipelineCacheError::ShaderNotLoaded(handle) => {
                        error!("  → Shader not loaded: {:?}", handle);
                    }
                    PipelineCacheError::ShaderCompilationError(msg) => {
                        error!("  → Compilation error: {}", msg);
                    }
                    PipelineCacheError::VertexLayoutMismatch { expected, found } => {
                        error!("  → Layout mismatch: expected {:?}, found {:?}", expected, found);
                    }
                    _ => {}
                }
            }
            CachedRenderPipelineState::Queued => {
                trace!("[PIPELINE] {:?}: Queued for compilation", id);
            }
            CachedRenderPipelineState::CreatingModule => {
                trace!("[PIPELINE] {:?}: Creating shader module", id);
            }
            _ => {}
        }
    }
}
```

### 3.4 Shader Preprocessing

**Shader Definition System**:

```rust
// From trail_effect.rs - shader preprocessing
impl SpecializedRenderPipeline for TrailEffectPipeline {
    fn specialize(&self, key: MeshPipelineKey) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::default();
        
        // Add definitions based on key
        if key.contains(MeshPipelineKey::HDR) {
            shader_defs.push("HDR".into());
        }
        
        // ... pipeline descriptor construction
        descriptor.vertex.shader_defs = shader_defs.clone();
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader_defs = shader_defs;
        }
        
        descriptor
    }
}
```

**Preprocessing Diagnostics**:

```rust
fn validate_shader_preprocessing(
    shader: &Shader,
    expected_defs: &[String],
) {
    info!("[SHADER PREPROCESSING] Shader: {:?}", shader);
    info!("  Expected definitions: {:?}", expected_defs);
    
    // Check that shader contains expected #ifdef blocks
    let shader_source = shader.source_code();
    
    for def in expected_defs {
        if !shader_source.contains(&format!("#ifdef {}", def)) {
            warn!("[SHADER PREPROCESSING] Definition '{}' not found in shader", def);
        }
    }
}
```

---

## 4. Shader Compilation Diagnostics

### 4.1 WGSL Syntax Validation

**Valid WGSL Structure** (from `object_material_simple.wgsl`):

```wgsl
// Correct binding syntax for Bevy 0.13.2
@group(2) @binding(0)
var<uniform> material: StaticMeshMaterialData;

@group(2) @binding(1)
var base_texture: texture_2d<f32>;

@group(2) @binding(2)
var base_sampler: sampler;

// Correct vertex structure with locations
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
};

// Correct entry points
@vertex
fn vertex(vertex: Vertex) -> VertexOutput { ... }

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> { ... }
```

**Common WGSL Errors**:

| Error Pattern | Cause | Resolution |
|--------------|-------|------------|
| `expected '(', found '['` | Using old `[[location(0)]]` syntax | Update to `@location(0)` |
| `unknown attribute: 'stage'` | Using old `[[stage(vertex)]]` | Update to `@vertex` |
| `expected ';', found 'var'` | Missing struct semicolon | Add semicolons to struct members |
| `undeclared identifier` | Missing import | Add `#import bevy_pbr::...` |
| `cannot cast` | Type mismatch | Use explicit constructors |

**Shader Syntax Validation**:

```rust
fn validate_wgsl_syntax(shader_source: &str) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    
    // Check for old attribute syntax
    if shader_source.contains("[[") || shader_source.contains("]]") {
        errors.push("Using deprecated WGSL attribute syntax [[...]] instead of @...".into());
    }
    
    // Check for correct group/binding syntax
    let group_binding_regex = regex::Regex::new(r"@group\(\d+\)\s*@binding\(\d+\)").unwrap();
    let old_group_binding = regex::Regex::new(r"group\(\d+\),\s*binding\(\d+\)").unwrap();
    
    if old_group_binding.is_match(shader_source) {
        errors.push("Using old group/binding syntax - should be @group(X) @binding(Y)".into());
    }
    
    // Check vertex output matches fragment input
    // ... additional validation
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

### 4.2 Shader Preprocessing Logs

**Shader Import System**:

```wgsl
// From object_material_simple.wgsl
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions::{mesh_position_local_to_world, mesh_normal_local_to_world, get_model_matrix}

#ifdef SKINNED
#import bevy_pbr::skinning::{skin_normals, skin_model}
#endif
```

**Import Resolution Diagnostics**:

```rust
fn diagnose_shader_imports(shader: &Shader, asset_server: &AssetServer) {
    let source = shader.source_code();
    
    // Extract all imports
    let import_regex = regex::Regex::new(r"#import\s+([\w::]+)").unwrap();
    
    for cap in import_regex.captures_iter(source) {
        let import_path = &cap[1];
        info!("[SHADER IMPORT] Found import: {}", import_path);
        
        // Check if import module exists
        match import_path {
            "bevy_pbr::mesh_view_bindings" => {}, // Built-in
            "bevy_pbr::mesh_bindings" => {}, // Built-in
            "bevy_pbr::mesh_functions" => {}, // Built-in
            "bevy_pbr::skinning" => {}, // Built-in
            "rose_client::zone_lighting" => {
                // Custom import - verify module registered
                if !is_custom_module_registered(import_path) {
                    error!("[SHADER IMPORT] Custom module '{}' not registered!", import_path);
                }
            }
            _ => warn!("[SHADER IMPORT] Unknown import: {}", import_path),
        }
    }
}
```

### 4.3 Bind Group Layout Mismatches

**Standard Bevy Bind Groups**:

| Group | Purpose | Contents |
|-------|---------|----------|
| 0 | View | Camera matrices, screen size |
| 1 | Mesh | Model matrix, skinning data |
| 2 | Material | Material properties, textures |
| 3+ | Custom | User-defined (e.g., zone lighting) |

**Layout Mismatch Diagnostics**:

```rust
fn validate_bind_group_layouts(
    shader: &Shader,
    pipeline_layout: &BindGroupLayout,
) {
    // Extract bind groups from shader
    let shader_bindings = extract_shader_bindings(shader);
    
    // Compare with pipeline layout
    for (group, bindings) in &shader_bindings {
        let expected_count = bindings.len();
        let actual_count = pipeline_layout.binding_count(*group);
        
        if expected_count != actual_count {
            error!("[BIND GROUP LAYOUT] Group {} mismatch: shader expects {}, layout has {}",
                group, expected_count, actual_count);
        }
        
        for (binding, ty) in bindings {
            if !pipeline_layout.has_binding(*group, *binding) {
                error!("[BIND GROUP LAYOUT] Missing binding @group({}) @binding({})",
                    group, binding);
            }
        }
    }
}
```

### 4.4 Entry Point Validation

**Entry Point Requirements**:

```wgsl
// Vertex shader must:
// 1. Be marked with @vertex
// 2. Return a struct with @builtin(position)
// 3. Accept vertex attributes with @location(X)

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = /* must set this */;
    return out;
}

// Fragment shader must:
// 1. Be marked with @fragment
// 2. Return @location(0) for color output

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
```

**Entry Point Diagnostics**:

```rust
fn validate_shader_entry_points(shader_source: &str) -> Result<(), String> {
    // Check for vertex entry point
    let has_vertex = shader_source.contains("@vertex");
    let has_fragment = shader_source.contains("@fragment");
    
    if !has_vertex {
        return Err("Missing @vertex entry point".into());
    }
    
    if !has_fragment {
        return Err("Missing @fragment entry point".into());
    }
    
    // Check vertex output has clip_position
    let vertex_fn = extract_function(shader_source, "vertex");
    if !vertex_fn.contains("@builtin(position)") {
        return Err("Vertex output missing @builtin(position)".into());
    }
    
    // Check fragment returns color
    let fragment_fn = extract_function(shader_source, "fragment");
    if !fragment_fn.contains("@location(0)") {
        return Err("Fragment output missing @location(0)".into());
    }
    
    Ok(())
}
```

### 4.5 Include Directive Resolution

**Custom Shader Imports** (from `zone_lighting.wgsl`):

```wgsl
// Define import path
#define_import_path rose_client::zone_lighting

// Now other shaders can import:
// #import rose_client::zone_lighting
```

**Import Resolution Diagnostics**:

```rust
fn diagnose_shader_includes(
    shader_assets: &Assets<Shader>,
) {
    for (handle, shader) in shader_assets.iter() {
        let source = shader.source_code();
        
        // Check for #define_import_path
        if let Some(cap) = regex::Regex::new(r"#define_import_path\s+(\S+)")
            .unwrap()
            .captures(source) 
        {
            let import_path = &cap[1];
            info!("[SHADER] {:?} defines import path: {}", handle, import_path);
        }
        
        // Check for #import directives
        let import_regex = regex::Regex::new(r"#import\s+(\S+)").unwrap();
        for cap in import_regex.captures_iter(source) {
            let import = &cap[1];
            
            // Check if import is resolvable
            if !can_resolve_import(import, shader_assets) {
                error!("[SHADER] {:?} cannot resolve import: {}", handle, import);
            }
        }
    }
}
```

---

## 5. Mesh GPU Buffer Diagnostics

### 5.1 Vertex Buffer Allocation

**Vertex Buffer Layout**:

```rust
use bevy::render::mesh::{Mesh, MeshVertexBufferLayout};
use bevy::render::render_resource::{VertexBufferLayout, VertexAttribute};

// Standard mesh vertex layout
fn create_vertex_buffer_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: (3 + 3 + 2) * 4, // position + normal + uv
        step_mode: VertexStepMode::Vertex,
        attributes: vec![
            // Position at location 0
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            // Normal at location 1
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 3 * 4,
                shader_location: 1,
            },
            // UV at location 2
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: (3 + 3) * 4,
                shader_location: 2,
            },
        ],
    }
}
```

**Vertex Buffer Diagnostics**:

```rust
fn diagnose_vertex_buffers(
    meshes: Res<Assets<Mesh>>,
    render_meshes: Res<RenderAssets<GpuMesh>>,
) {
    for (handle, mesh) in meshes.iter() {
        let vertex_count = mesh.count_vertices();
        let attribute_count = mesh.attributes().count();
        
        info!("[MESH VERTEX] {:?}: {} vertices, {} attributes",
            handle, vertex_count, attribute_count);
        
        // Check required attributes
        let has_position = mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some();
        let has_normal = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some();
        let has_uv = mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some();
        
        if !has_position {
            error!("[MESH VERTEX] {:?}: MISSING POSITION ATTRIBUTE!", handle);
        }
        if !has_normal {
            warn!("[MESH VERTEX] {:?}: Missing normal attribute", handle);
        }
        
        // Check GPU mesh exists
        if let Some(gpu_mesh) = render_meshes.get(handle) {
            info!("[MESH VERTEX] {:?}: GPU buffer allocated (ok)", handle);
        } else {
            error!("[MESH VERTEX] {:?}: GPU mesh NOT allocated!", handle);
        }
    }
}
```

### 5.2 Index Buffer Integrity

**Index Buffer Validation**:

```rust
fn diagnose_index_buffers(
    meshes: Res<Assets<Mesh>>,
) {
    for (handle, mesh) in meshes.iter() {
        match mesh.indices() {
            Some(Indices::U16(indices)) => {
                let max_index = *indices.iter().max().unwrap_or(&0) as usize;
                let vertex_count = mesh.count_vertices();
                
                info!("[MESH INDEX] {:?}: {} u16 indices", handle, indices.len());
                
                if max_index >= vertex_count {
                    error!("[MESH INDEX] {:?}: Index out of bounds! max={}, vertices={}",
                        handle, max_index, vertex_count);
                }
            }
            Some(Indices::U32(indices)) => {
                let max_index = *indices.iter().max().unwrap_or(&0) as usize;
                let vertex_count = mesh.count_vertices();
                
                info!("[MESH INDEX] {:?}: {} u32 indices", handle, indices.len());
                
                if max_index >= vertex_count {
                    error!("[MESH INDEX] {:?}: Index out of bounds! max={}, vertices={}",
                        handle, max_index, vertex_count);
                }
            }
            None => {
                // Non-indexed geometry
                info!("[MESH INDEX] {:?}: Non-indexed geometry", handle);
            }
        }
    }
}
```

### 5.3 GpuMesh Creation

**Render Mesh Conversion**:

```rust
use bevy::render::mesh::{GpuMesh, RenderMesh};

impl RenderAsset for Mesh {
    type PreparedAsset = GpuMesh;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<GpuMeshAllocator>,
    );
    
    fn prepare_asset(
        mesh: Self,
        (device, queue, allocator): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
        // Create GPU buffers
        let vertex_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("mesh_vertex_buffer"),
            contents: &vertex_data,
            usage: BufferUsages::VERTEX,
        });
        
        let index_buffer = indices.map(|i| {
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("mesh_index_buffer"),
                contents: &index_data,
                usage: BufferUsages::INDEX,
            })
        });
        
        Ok(GpuMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.count_vertices(),
            index_count: indices.map(|i| i.len()),
            primitive_topology: mesh.primitive_topology(),
            layout: mesh.get_mesh_vertex_buffer_layout(),
        })
    }
}
```

**GpuMesh Diagnostics**:

```rust
fn diagnose_gpu_meshes(
    gpu_meshes: Res<RenderAssets<GpuMesh>>,
) {
    let mut total_vertex_bytes = 0;
    let mut total_index_bytes = 0;
    
    for (handle, gpu_mesh) in gpu_meshes.iter() {
        let vertex_bytes = gpu_mesh.vertex_count * gpu_mesh.layout.array_stride();
        let index_bytes = gpu_mesh.index_count.map(|c| c * 4).unwrap_or(0);
        
        total_vertex_bytes += vertex_bytes;
        total_index_bytes += index_bytes;
        
        info!("[GPU MESH] {:?}:", handle);
        info!("  Vertices: {}, Vertex buffer: {} bytes", 
            gpu_mesh.vertex_count, vertex_bytes);
        if let Some(count) = gpu_mesh.index_count {
            info!("  Indices: {}, Index buffer: {} bytes", count, index_bytes);
        }
        info!("  Topology: {:?}", gpu_mesh.primitive_topology);
    }
    
    info!("[GPU MESH] Total GPU memory: {} KB vertex, {} KB index",
        total_vertex_bytes / 1024,
        total_index_bytes / 1024);
}
```

### 5.4 Mesh Uniform Buffer Alignment

**Mesh Uniforms** (Group 1):

```rust
// Bevy's standard mesh uniform structure
#[derive(ShaderType)]
pub struct MeshUniform {
    pub model: Mat4,           // 64 bytes
    pub inverse_model: Mat4,   // 64 bytes
    pub flags: u32,            // 4 bytes
    // ... padding to 16-byte alignment
}
```

**Alignment Diagnostics**:

```rust
fn diagnose_mesh_uniform_alignment(
    render_device: Res<RenderDevice>,
) {
    let min_uniform_alignment = render_device.limits().min_uniform_buffer_offset_alignment;
    
    info!("[MESH UNIFORM] Minimum uniform buffer alignment: {} bytes", 
        min_uniform_alignment);
    
    let mesh_uniform_size = std::mem::size_of::<MeshUniform>();
    let aligned_size = (mesh_uniform_size + min_uniform_alignment as usize - 1) 
        & !(min_uniform_alignment as usize - 1);
    
    info!("[MESH UNIFORM] MeshUniform size: {} bytes, aligned: {} bytes",
        mesh_uniform_size, aligned_size);
    
    if mesh_uniform_size % 16 != 0 {
        warn!("[MESH UNIFORM] MeshUniform is not 16-byte aligned!");
    }
}
```

---

## 6. Bind Group and Texture Diagnostics

### 6.1 Bind Group Layout Consistency

**Standard Bevy Bind Group Layouts**:

```rust
// View bind group (group 0)
let view_layout = render_device.create_bind_group_layout(
    "view_layout",
    &[
        // View uniform buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(ViewUniform::min_size()),
            },
            count: None,
        },
        // Lights buffer
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
);
```

**Layout Consistency Validation**:

```rust
fn validate_bind_group_layout_consistency(
    pipeline: &RenderPipelineDescriptor,
    shader: &Shader,
) {
    // Extract shader bindings
    let shader_bindings = parse_shader_bindings(shader);
    
    // Validate against pipeline layout
    for (group_index, group_layout) in pipeline.layout.iter().enumerate() {
        for entry in &group_layout.entries {
            let binding = entry.binding;
            
            // Check shader has this binding
            if !shader_bindings.has_binding(group_index as u32, binding) {
                error!("[BIND GROUP] Pipeline layout has @group({}) @binding({}) but shader doesn't use it!",
                    group_index, binding);
            }
            
            // Check visibility matches
            let shader_visibility = shader_bindings.get_visibility(group_index as u32, binding);
            if entry.visibility != shader_visibility {
                warn!("[BIND GROUP] Visibility mismatch @group({}) @binding({}): pipeline={:?}, shader={:?}",
                    group_index, binding, entry.visibility, shader_visibility);
            }
        }
    }
}
```

### 6.2 Texture Binding Validation

**Texture Binding Types**:

| Texture Type | WGSL Type | Binding Type | Usage |
|--------------|-----------|--------------|-------|
| 2D Color | `texture_2d<f32>` | `TextureSampleType::Float` | Albedo, normal |
| Depth | `texture_depth_2d` | `TextureSampleType::Depth` | Shadow maps |
| Storage | `texture_storage_2d` | `BindingType::StorageTexture` | Compute output |
| Cube | `texture_cube` | `TextureViewDimension::Cube` | Skybox |
| Array | `texture_2d_array` | `TextureViewDimension::D2Array` | Texture arrays |

**Texture Binding Diagnostics**:

```rust
fn diagnose_texture_bindings(
    gpu_images: Res<RenderAssets<Image>>,
    bind_groups: Query<&BindGroup>,
) {
    for (handle, gpu_image) in gpu_images.iter() {
        info!("[TEXTURE] {:?}:", handle);
        info!("  Size: {}x{}", gpu_image.size.x, gpu_image.size.y);
        info!("  Format: {:?}", gpu_image.texture_format);
        info!("  Mip levels: {}", gpu_image.mip_level_count);
        
        // Check texture view
        match gpu_image.texture_view.dimension() {
            TextureViewDimension::D2 => {},
            dim => info!("  View dimension: {:?}", dim),
        }
        
        // Check sampler
        info!("  Sampler: {:?}", gpu_image.sampler.address_mode_u);
    }
}
```

### 6.3 Sampler Configuration

**Sampler Types**:

```rust
use bevy::render::render_resource::SamplerDescriptor;

// Standard texture sampler
let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
    address_mode_u: AddressMode::Repeat,
    address_mode_v: AddressMode::Repeat,
    mag_filter: FilterMode::Linear,
    min_filter: FilterMode::Linear,
    mipmap_filter: FilterMode::Linear,
    ..default()
});

// Shadow sampler (comparison)
let shadow_sampler = render_device.create_sampler(&SamplerDescriptor {
    address_mode_u: AddressMode::ClampToEdge,
    address_mode_v: AddressMode::ClampToEdge,
    mag_filter: FilterMode::Linear,
    min_filter: FilterMode::Linear,
    compare: Some(CompareFunction::LessEqual),
    ..default()
});
```

**Sampler Diagnostics**:

```rust
fn diagnose_sampler_configuration(
    gpu_images: Res<RenderAssets<Image>>,
) {
    for (handle, gpu_image) in gpu_images.iter() {
        let sampler = &gpu_image.sampler;
        
        // Check for common issues
        if sampler.mag_filter == FilterMode::Nearest 
            && sampler.min_filter == FilterMode::Linear {
            warn!("[SAMPLER] {:?}: Mixed filter modes may cause artifacts", handle);
        }
        
        // Check address modes for non-power-of-2 textures
        // ... additional checks
    }
}
```

### 6.4 Uniform Buffer Alignment Requirements

**Alignment Rules**:

```
Standard WGSL alignment:
- vec2<T>: 8 bytes
- vec3<T>: 16 bytes (4-byte padding after)
- vec4<T>: 16 bytes
- mat4x4<f32>: 16 bytes (column alignment)
- Struct: 16-byte alignment
```

**Alignment Validation**:

```rust
fn validate_uniform_alignment(uniform_data: &[u8]) -> Vec<String> {
    let mut issues = Vec::new();
    
    // Check for vec3 followed by float (common mistake)
    // vec3 needs 16 bytes but only uses 12
    
    // Check struct alignment
    if uniform_data.len() % 16 != 0 {
        issues.push(format!(
            "Uniform buffer size {} is not 16-byte aligned", 
            uniform_data.len()
        ));
    }
    
    issues
}

fn diagnose_uniform_buffers<P: PhaseItem>(
    view_uniforms: Res<ViewUniforms>,
    render_device: Res<RenderDevice>,
) {
    let alignment = render_device.limits().min_uniform_buffer_offset_alignment;
    
    info!("[UNIFORM BUFFER] Device alignment requirement: {}", alignment);
    
    if let Some(binding) = view_uniforms.uniforms.binding() {
        match binding {
            BindingResource::Buffer(buffer_binding) => {
                info!("[UNIFORM BUFFER] View uniforms: size={}, offset={}",
                    buffer_binding.size(), buffer_binding.offset());
                
                if buffer_binding.offset() % alignment as u64 != 0 {
                    error!("[UNIFORM BUFFER] View uniform offset not aligned!");
                }
            }
            _ => {}
        }
    }
}
```

---

## 7. Depth Texture and Multisampling

### 7.1 Depth Attachment Configuration

**Depth Texture Setup**:

```rust
use bevy::render::texture::TextureDescriptor;

fn create_depth_texture(
    render_device: &RenderDevice,
    width: u32,
    height: u32,
    sample_count: u32,
) -> Texture {
    render_device.create_texture(&TextureDescriptor {
        label: Some("depth_texture"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
```

**Depth Configuration Diagnostics**:

```rust
fn diagnose_depth_attachment(
    view_targets: Query<&ViewTarget>,
) {
    for view_target in view_targets.iter() {
        if let Some(depth) = view_target.depth_texture() {
            info!("[DEPTH] Depth texture present:");
            info!("  Format: {:?}", depth.texture.format());
            info!("  Samples: {:?}", depth.texture.sample_count());
            info!("  Size: {:?}x{:?}", 
                depth.texture.width(), depth.texture.height());
        } else {
            warn!("[DEPTH] No depth texture - 3D rendering may have artifacts!");
        }
    }
}
```

### 7.2 MSAA Sample Count Validation

**MSAA Configuration**:

```rust
use bevy::render::view::Msaa;

// MSAA levels
const MSAA_OFF: Msaa = Msaa::Off;        // 1 sample
const MSAA_2X: Msaa = Msaa::Sample2;     // 2 samples
const MSAA_4X: Msaa = Msaa::Sample4;     // 4 samples
const MSAA_8X: Msaa = Msaa::Sample8;     // 8 samples

fn configure_msaa(app: &mut App) {
    app.insert_resource(Msaa::Sample4);
}
```

**MSAA Diagnostics**:

```rust
fn diagnose_msaa_configuration(
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
) {
    let sample_count = msaa.samples();
    let max_samples = render_device.limits().max_sampled_textures_per_shader_stage;
    
    info!("[MSAA] Current MSAA: {} samples", sample_count);
    info!("[MSAA] Max supported samples: {}", max_samples);
    
    if sample_count > max_samples {
        error!("[MSAA] Requested {} samples but device only supports {}!",
            sample_count, max_samples);
    }
    
    // Check pipeline MSAA keys match
    if sample_count > 1 {
        info!("[MSAA] MSAA is enabled - verify all pipelines use matching sample count");
    }
}
```

### 7.3 Depth Prepass Configuration

**Prepass Types**:

| Prepass | Purpose | Outputs |
|---------|---------|---------|
| `DepthPrepass` | Early Z-culling | Depth buffer |
| `NormalPrepass` | Normal reconstruction | Normal texture |
| `MotionVectorPrepass` | Motion blur/TAA | Motion vectors |
| `DeferredPrepass` | G-buffer | Albedo, normal, material |

**Prepass Diagnostics**:

```rust
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass, MotionVectorPrepass};

fn diagnose_prepass_configuration(
    cameras: Query<(
        Option<&DepthPrepass>,
        Option<&NormalPrepass>,
        Option<&MotionVectorPrepass>,
    ), With<Camera>>,
) {
    for (depth, normal, motion) in cameras.iter() {
        info!("[PREPASS] Camera prepass configuration:");
        info!("  Depth prepass: {}", depth.is_some());
        info!("  Normal prepass: {}", normal.is_some());
        info!("  Motion vector prepass: {}", motion.is_some());
        
        // Check for valid combinations
        if normal.is_some() && depth.is_none() {
            warn!("[PREPASS] Normal prepass without depth prepass may not work correctly");
        }
    }
}
```

**Pipeline Key Validation for Prepass**:

```rust
fn validate_prepass_pipeline_keys(
    pipelines: Res<SpecializedRenderPipelines<CustomPipeline>>,
) {
    for (key, id) in pipelines.iter() {
        let has_depth_prepass = key.contains(MeshPipelineKey::DEPTH_PREPASS);
        let has_normal_prepass = key.contains(MeshPipelineKey::NORMAL_PREPASS);
        let has_deferred = key.contains(MeshPipelineKey::DEFERRED_PREPASS);
        
        // Multiple prepass types should be properly combined
        if has_deferred && (has_depth_prepass || has_normal_prepass) {
            warn!("[PREPASS] Pipeline has both deferred and forward prepass flags");
        }
    }
}
```

---

## 8. Draw Call Generation

### 8.1 RenderCommand Implementation

**RenderCommand Trait**:

```rust
use bevy::render::render_phase::{RenderCommand, RenderCommandResult, PhaseItem};
use bevy::render::render_phase::TrackedRenderPass;

pub struct SetCustomBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetCustomBindGroup<I> {
    type Param = SRes<CustomRenderResources>;
    type ViewQuery = ();
    type ItemQuery = ();
    
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        resources: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &resources.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawCustomMesh;

impl<P: PhaseItem> RenderCommand<P> for DrawCustomMesh {
    type Param = SRes<CustomRenderResources>;
    type ViewQuery = ();
    type ItemQuery = Read<CustomMesh>;
    
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        mesh: Option<ROQueryItem<'w, Self::ItemQuery>>,
        resources: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(mesh) = mesh {
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.draw(0..mesh.vertex_count, 0..1);
        }
        RenderCommandResult::Success
    }
}
```

**RenderCommand Validation**:

```rust
fn diagnose_render_command<C: RenderCommand<P>, P: PhaseItem>(
    draw_functions: Res<DrawFunctions<P>>,
) {
    let draw_functions = draw_functions.read();
    
    if let Some(id) = draw_functions.get_id::<C>() {
        info!("[RENDER COMMAND] {:?} registered with ID {:?}", 
            std::any::type_name::<C>(), id);
    } else {
        error!("[RENDER COMMAND] {:?} NOT REGISTERED!", 
            std::any::type_name::<C>());
    }
}
```

### 8.2 DrawFunctions Registration

**Registration in Plugin**:

```rust
impl Plugin for CustomRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        
        // Register draw command for a phase
        render_app.add_render_command::<Transparent3d, DrawCustom>();
    }
}

// Type alias defining the complete draw command sequence
type DrawCustom = (
    SetItemPipeline,                              // Set pipeline
    SetMeshViewBindGroup<0>,                      // Set view uniforms @group(0)
    SetMeshBindGroup<1>,                          // Set mesh uniforms @group(1)
    SetMaterialBindGroup<CustomMaterial, 2>,      // Set material @group(2)
    DrawMesh,                                     // Execute draw
);
```

**Registration Diagnostics**:

```rust
fn diagnose_draw_functions_registration<P: PhaseItem>(
    draw_functions: Res<DrawFunctions<P>>,
) {
    let functions = draw_functions.read();
    
    info!("[DRAW FUNCTIONS] Registered for {:?}:", std::any::type_name::<P>());
    
    for (index, name) in functions.iter_names() {
        info!("  [{}]: {}", index, name);
    }
    
    if functions.is_empty() {
        error!("[DRAW FUNCTIONS] No draw functions registered!");
    }
}
```

### 8.3 Phase Item to Draw Call Conversion

**Phase Item Structure**:

```rust
// From trail_effect.rs
#[derive(Component)]
struct TrailEffectBatch {
    vertex_range: Range<u32>,
    handle: Handle<Image>,
}

fn queue_trail_effects(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
    // ... other params
) {
    let draw_trail_effect = transparent_draw_functions
        .read()
        .get_id::<DrawTrailEffect>()
        .unwrap();
    
    for (view, mut transparent_phase) in views.iter_mut() {
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);
        
        for (entity, batch) in trail_effect_batches.iter() {
            transparent_phase.add(Transparent3d {
                distance: 10.0, // Sorting distance
                pipeline: pipelines.specialize(&pipeline_cache, &trail_effect_pipeline, view_key),
                entity,
                draw_function: draw_trail_effect,
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}
```

**Conversion Diagnostics**:

```rust
fn diagnose_phase_item_conversion<P: PhaseItem>(
    phases: Query<(Entity, &RenderPhase<P>)>,
    pipeline_cache: Res<PipelineCache>,
) {
    for (view_entity, phase) in phases.iter() {
        info!("[DRAW CALLS] View {:?}: {} items", view_entity, phase.items.len());
        
        let mut ready_count = 0;
        let mut pending_count = 0;
        let mut error_count = 0;
        
        for item in &phase.items {
            // Check pipeline state
            if let Some(pipeline) = pipeline_cache.get_render_pipeline(item.pipeline()) {
                ready_count += 1;
            } else {
                match pipeline_cache.get_render_pipeline_state(item.pipeline()) {
                    CachedRenderPipelineState::Queued | 
                    CachedRenderPipelineState::CreatingModule => {
                        pending_count += 1;
                    }
                    CachedRenderPipelineState::Err(_) => {
                        error_count += 1;
                    }
                    _ => {}
                }
            }
        }
        
        info!("[DRAW CALLS]   Ready: {}, Pending: {}, Error: {}",
            ready_count, pending_count, error_count);
    }
}
```

---

## 9. Render Schedule Sequence

### 9.1 Extract Phase Systems

**Extract Schedule Purpose**: Copy data from main world to render world

```rust
use bevy::render::{Extract, ExtractSchedule};

fn extract_zone_lighting(
    mut commands: Commands,
    zone_lighting: Extract<Res<ZoneLighting>>,
) {
    commands.insert_resource(ZoneLightingUniformData {
        map_ambient_color: zone_lighting.map_ambient_color.extend(1.0),
        // ... other fields
    });
}

fn extract_trail_effects(
    mut extracted: ResMut<ExtractedTrailEffects>,
    query: Extract<Query<(&TrailEffect, &TrailEffectPositionHistory)>>,
) {
    extracted.trail_effects.clear();
    
    for (effect, history) in query.iter() {
        // Extract data for rendering
        extracted.trail_effects.push(ExtractedTrailEffect {
            texture: effect.trail_texture.clone_weak(),
            vertices: compute_vertices(history),
        });
    }
}
```

**Extract Phase Diagnostics**:

```rust
fn diagnose_extract_phase(render_app: &RenderApp) {
    let extract_schedule = render_app.get_schedule(ExtractSchedule)
        .expect("ExtractSchedule not found");
    
    info!("[EXTRACT PHASE] Systems in ExtractSchedule:");
    for system in extract_schedule.systems() {
        info!("  - {}", system.name());
    }
}
```

### 9.2 Prepare Phase Systems

**Prepare Schedule Purpose**: Allocate/Update GPU resources

```rust
use bevy::render::RenderSet;

fn prepare_trail_effects(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut trail_meta: ResMut<TrailEffectMeta>,
    extracted: Res<ExtractedTrailEffects>,
) {
    // Upload vertex data to GPU
    trail_meta.vertex_buffer.clear();
    
    for effect in &extracted.trail_effects {
        for vertex in &effect.vertices {
            trail_meta.vertex_buffer.push(*vertex);
        }
    }
    
    trail_meta.vertex_buffer.write_buffer(&render_device, &render_queue);
}

fn prepare_uniform_data(
    uniform_data: Res<ZoneLightingUniformData>,
    uniform_meta: ResMut<ZoneLightingUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let mut buffer = encase::UniformBuffer::new([0u8; 256]);
    buffer.write(uniform_data.as_ref()).unwrap();
    
    render_queue.write_buffer(&uniform_meta.buffer, 0, buffer.as_ref());
}
```

**Prepare Phase Diagnostics**:

```rust
fn diagnose_prepare_phase(render_app: &RenderApp) {
    info!("[PREPARE PHASE] Systems in RenderSet::Prepare:");
    
    // Check for buffer uploads
    if render_app.world.contains_resource::<TrailEffectMeta>() {
        let meta = render_app.world.resource::<TrailEffectMeta>();
        info!("[PREPARE] Trail vertex buffer: {} vertices", meta.vertex_count);
    }
}
```

### 9.3 Queue Phase Systems

**Queue Schedule Purpose**: Generate phase items and sort them

```rust
use bevy::render::RenderSet;

fn queue_trail_effects(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<&mut RenderPhase<Transparent3d>>,
    batches: Query<(Entity, &TrailEffectBatch)>,
    // ... other params
) {
    let draw_function = draw_functions.read()
        .get_id::<DrawTrailEffect>()
        .unwrap();
    
    for mut phase in views.iter_mut() {
        for (entity, batch) in batches.iter() {
            // Calculate distance for sorting (back-to-front for transparent)
            let distance = 10.0; // Should be actual distance to camera
            
            phase.add(Transparent3d {
                distance,
                pipeline: /* specialized pipeline */,
                entity,
                draw_function,
                batch_range: batch.vertex_range.clone(),
                dynamic_offset: None,
            });
        }
    }
}
```

**Queue Phase Diagnostics**:

```rust
fn diagnose_queue_phase(render_app: &RenderApp) {
    // Check phase population
    let mut total_items = 0;
    
    if let Ok(phases) = render_app.world.query::<&RenderPhase<Opaque3d>>().iter(&render_app.world).collect::<Vec<_>>() {
        let count: usize = phases.iter().map(|p| p.items.len()).sum();
        info!("[QUEUE] Opaque3d phase: {} items", count);
        total_items += count;
    }
    
    if let Ok(phases) = render_app.world.query::<&RenderPhase<Transparent3d>>().iter(&render_app.world).collect::<Vec<_>>() {
        let count: usize = phases.iter().map(|p| p.items.len()).sum();
        info!("[QUEUE] Transparent3d phase: {} items", count);
        total_items += count;
    }
    
    if total_items == 0 {
        error!("[QUEUE] No items queued for rendering!");
    }
}
```

### 9.4 Render Phase Systems

**Render Schedule Purpose**: Execute draw commands

```rust
use bevy::render::RenderSet;

// Rendering is handled by the graph runner, not individual systems
// But custom render nodes can be added:

pub struct CustomPassNode;

impl Node for CustomPassNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // Execute render pass
        let pass_descriptor = RenderPassDescriptor {
            label: Some("custom_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: /* color target */,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };
        
        let mut pass = render_context.begin_tracked_render_pass(pass_descriptor);
        
        // Execute phase items
        let phases = world.query::<&RenderPhase<Transparent3d>>();
        for phase in phases.iter(world) {
            phase.render(&mut pass, world, graph.view_entity());
        }
        
        Ok(())
    }
}
```

### 9.5 Cleanup Phase Systems

**Cleanup Schedule Purpose**: Release temporary resources

```rust
use bevy::render::RenderSet;

fn cleanup_frame_data(
    mut extracted: ResMut<ExtractedTrailEffects>,
) {
    // Clear extracted data for next frame
    extracted.trail_effects.clear();
}

fn cleanup_unused_bind_groups(
    mut material_bind_groups: ResMut<MaterialBindGroups>,
) {
    // Remove bind groups that weren't used this frame
    let handles_to_remove: Vec<_> = material_bind_groups
        .values
        .keys()
        .filter(|h| !material_bind_groups.used_this_frame.contains_key(*h))
        .cloned()
        .collect();
    
    for handle in handles_to_remove {
        material_bind_groups.values.remove(&handle);
    }
    
    material_bind_groups.used_this_frame.clear();
}
```

### 9.6 RenderSet Ordering

**Standard RenderSet Order**:

```
Render Schedule:
├── ExtractSchedule (separate - copies data from main world)
│   └── (extract systems)
│
└── Render (main render schedule)
    ├── RenderSet::Extract
    │   └── (additional extraction)
    │
    ├── RenderSet::Prepare
    │   ├── Prepare assets (meshes, textures)
    │   ├── Prepare uniforms
    │   └── Upload buffer data
    │
    ├── RenderSet::Queue
    │   ├── Queue phase items
    │   ├── Specialize pipelines
    │   └── Sort phase items
    │
    ├── RenderSet::PhaseSort
    │   └── Final phase item sorting
    │
    ├── RenderSet::Render
    │   └── Execute graph runner
    │
    └── RenderSet::Cleanup
        └── Release temporary resources
```

**RenderSet Validation**:

```rust
use bevy::render::RenderSet;

fn validate_render_set_ordering(app: &mut App) {
    let render_app = app.get_sub_app_mut(RenderApp)
        .expect("RenderApp not found");
    
    // Check system ordering
    render_app.add_systems(Render, (
        // Extract must run first
        extract_system.in_set(RenderSet::Extract),
        
        // Prepare runs after extract
        prepare_system.in_set(RenderSet::Prepare)
            .after(RenderSet::Extract),
        
        // Queue runs after prepare
        queue_system.in_set(RenderSet::Queue)
            .after(RenderSet::Prepare),
        
        // Cleanup runs last
        cleanup_system.in_set(RenderSet::Cleanup),
    ));
}
```

**System Set Diagnostics**:

```rust
fn diagnose_render_set_configuration(render_app: &RenderApp) {
    let render_schedule = render_app.get_schedule(Render)
        .expect("Render schedule not found");
    
    info!("[RENDER SET] Render schedule configuration:");
    
    for set in [
        RenderSet::Extract,
        RenderSet::Prepare,
        RenderSet::Queue,
        RenderSet::PhaseSort,
        RenderSet::Render,
        RenderSet::Cleanup,
    ] {
        let systems = render_schedule.systems_in_set(set);
        info!("  {:?}: {} systems", set, systems.len());
    }
}
```

---

## Appendix A: Error Patterns Quick Reference

### A.1 Shader Compilation Errors

| Error Pattern | Meaning | Resolution |
|--------------|---------|------------|
| `naga::front::wgsl::ParseError` | WGSL syntax error | Check line number, verify modern WGSL syntax |
| `binding doesn't exist` | Missing bind group entry | Add binding to bind group layout |
| `shader 'X' not found` | Shader asset not loaded | Verify shader handle, check asset loading |
| `vertex shader entry point not found` | Missing @vertex function | Add vertex shader entry point |
| `fragment shader entry point not found` | Missing @fragment function | Add fragment shader entry point |

### A.2 Pipeline Errors

| Error Pattern | Meaning | Resolution |
|--------------|---------|------------|
| `SpecializedMeshPipelineError` | Pipeline specialization failed | Check MeshPipelineKey handling |
| `Pipeline not found in cache` | Pipeline never specialized | Call specialize() before use |
| `vertex buffer layout mismatch` | Mesh attributes don't match shader | Verify attribute locations |
| `bind group layout mismatch` | Layout doesn't match shader | Check @group/@binding indices |

### A.3 Render Graph Errors

| Error Pattern | Meaning | Resolution |
|--------------|---------|------------|
| `Node X not found` | Node not registered | Register node with render graph |
| `Cyclic dependency detected` | Circular node edges | Remove cycle in graph |
| `Slot X not found` | Missing slot on node | Define slot in node implementation |
| `Graph not found` | Sub-graph doesn't exist | Register sub-graph with add_sub_graph |

### A.4 Resource Binding Errors

| Error Pattern | Meaning | Resolution |
|--------------|---------|------------|
| `Bind group not found` | Bind group not created | Create bind group in prepare phase |
| `Texture not found` | Image asset not loaded | Wait for image to load |
| `Buffer too small` | Uniform data exceeds buffer | Increase buffer size |
| `Misaligned dynamic offset` | Dynamic offset not aligned | Use correct alignment (256 bytes typical) |

---

## Appendix B: Key Bevy 0.13.2 Render APIs

### B.1 Core Render Types

```rust
// App configuration
use bevy::render::{RenderApp, RenderSet, ExtractSchedule};
use bevy::render::render_graph::{RenderGraph, Graph, Node};

// Pipeline
use bevy::render::render_resource::{
    RenderPipelineDescriptor, 
    SpecializedRenderPipeline,
    PipelineCache,
    CachedRenderPipelineState,
};

// Shaders
use bevy::render::render_resource::{Shader, ShaderRef, ShaderStages};

// Bind groups
use bevy::render::render_resource::{
    BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindingType,
};

// Buffers
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, BufferBindingType,
};

// Phases
use bevy::render::render_phase::{
    RenderPhase, PhaseItem, DrawFunctions, RenderCommand,
    AddRenderCommand, TrackedRenderPass,
};

// Materials
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
```

### B.2 Rose Online Custom Shaders

| Shader | Path | Bind Groups |
|--------|------|-------------|
| Object Material | `shaders/object_material_simple.wgsl` | view(0), mesh(1), material(2) |
| Particle | `shaders/particle.wgsl` | view(0), material(1) |
| Trail Effect | `shaders/trail_effect.wgsl` | view(0), material(1) |
| Sky | `shaders/sky_material.wgsl` | view(0), mesh(1), material(2) |
| Water | `shaders/water_material.wgsl` | view(0), mesh(1), material(2), zone_lighting(3) |
| Zone Lighting | `shaders/zone_lighting.wgsl` | module import |

---

*Document Version: 1.0*  
*Bevy Version: 0.13.2*  
*Compatible with: Rose Online Client codebase*
