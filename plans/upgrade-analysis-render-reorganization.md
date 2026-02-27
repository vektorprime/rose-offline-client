# Bevy 0.17 Render Crate Reorganization - Migration Analysis

## Executive Summary

This document provides a comprehensive analysis of all import changes required for the ROSE Offline Client codebase due to Bevy 0.17's render crate reorganization. The analysis identified **43 source files** requiring import modifications.

## Overview of Bevy 0.17 Render Crate Changes

Bevy 0.17 moved many types from `bevy_render` and `bevy_core_pipeline` to new specialized crates:

| Old Location | New Location |
|--------------|--------------|
| `bevy_render::Camera` | `bevy_camera::Camera` |
| `bevy_render::Camera3d` | `bevy_camera::Camera3d` |
| `bevy_render::Camera2d` | `bevy_camera::Camera2d` |
| `bevy_render::Projection` | `bevy_camera::Projection` |
| `bevy_render::Visibility` | `bevy_camera::visibility::Visibility` |
| `bevy_render::Frustum`, `Aabb` | `bevy_camera::primitives` |
| `bevy_render::Mesh` | `bevy_mesh::Mesh` |
| `bevy_render::Image` | `bevy_image::Image` |
| `bevy_render::Shader` | `bevy_shader::Shader` |
| `bevy_core_pipeline::Bloom` | `bevy_post_process::Bloom` |
| `bevy_core_pipeline::DepthOfField` | `bevy_post_process::DepthOfField` |
| `bevy_core_pipeline::Smaa` | `bevy_anti_alias::Smaa` |
| `bevy_core_pipeline::TemporalAntiAlias` | `bevy_anti_alias::taa::TemporalAntiAliasing` |

---

## Files Requiring Changes - Grouped by Crate

### 1. bevy_camera Migration

Types moving to `bevy_camera`:
- `Camera`, `Camera3d`, `Camera2d`
- `Projection`, `PerspectiveProjection`
- `Visibility`, `InheritedVisibility`, `ViewVisibility`
- `NoFrustumCulling`, `RenderLayers`
- `Aabb`, `Frustum`
- `Exposure`
- `ColorGrading`, `ColorGradingGlobal`, `ColorGradingSection`
- `VisibilitySystems`

#### Files Affected:

##### [`src/lib.rs`](src/lib.rs)
**Current imports:**
```rust
use bevy::{
    prelude::{
        Camera, Camera3d, PerspectiveProjection, Projection,
        InheritedVisibility, ViewVisibility, Visibility,
    },
    render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
    render::view::VisibilitySystems,
    render::camera::Exposure,
};
```

**New imports needed:**
```rust
use bevy_camera::{
    Camera, Camera3d, Projection, PerspectiveProjection,
    Visibility, InheritedVisibility, ViewVisibility,
    NoFrustumCulling, RenderLayers,
    primitives::Aabb,
    Exposure,
    color_grading::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
    visibility::VisibilitySystems,
};
```

**Types affected:**
- `Camera` (line 16)
- `Camera3d` (line 16)
- `PerspectiveProjection` (line 18)
- `Projection` (line 19)
- `InheritedVisibility` (line 17)
- `ViewVisibility` (line 20)
- `Visibility` (line 20)
- `ColorGrading` (line 13)
- `ColorGradingGlobal` (line 13)
- `ColorGradingSection` (line 13)
- `VisibilitySystems` (line 22)
- `Exposure` (line 23)

---

##### [`src/zone_loader.rs`](src/zone_loader.rs)
**Current imports:**
```rust
use bevy::render::{
    mesh::{Indices, Mesh, PrimitiveTopology},
    primitives::Aabb,
    render_asset::RenderAssetUsages,
    view::{NoFrustumCulling, ViewVisibility, InheritedVisibility, RenderLayers},
};
```

**Types affected:**
- `Aabb` (lines 294, 2034, 2327, 2560, 2656, 2727, etc.)
- `NoFrustumCulling` (lines 296, 2033, 2326, 2559, 2655, 2726, etc.)
- `ViewVisibility` (lines 296, 2029, 2323, 2557, 2654, 2725, etc.)
- `InheritedVisibility` (lines 296, 2030, 2325, 2558, 2653, 2724, etc.)
- `RenderLayers` (lines 296, 2035, 2328, 2561, 2657, 2728, etc.)
- `Visibility` (line 287)

---

##### [`src/model_loader.rs`](src/model_loader.rs)
**Current imports:**
```rust
use bevy::render::{
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    view::InheritedVisibility,
    view::NoFrustumCulling,
    view::ViewVisibility,
    primitives::Aabb,
};
```

**Types affected:**
- `InheritedVisibility` (line 13)
- `NoFrustumCulling` (line 13)
- `ViewVisibility` (line 13)
- `Aabb` (line 13)

---

##### [`src/systems/debug_rendering_system.rs`](src/systems/debug_rendering_system.rs)
**Current imports:**
```rust
use bevy::render::view::Visibility;
use bevy::render::primitives::Aabb;
use bevy::render::view::ViewUniformOffset;
use bevy::render::view::ExtractedView;
```

**Types affected:**
- `Visibility` (line 2)
- `Aabb` (line 3)
- `ViewUniformOffset` (line 7)
- `NoFrustumCulling` (line 1175)
- `RenderLayers` (lines 1179, 1220, 1299)
- `ViewVisibility` (line 1181)
- `ExtractedView` (line 1445)

---

##### [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs)
**Current imports:**
```rust
use bevy::render::view::ViewVisibility;
use bevy::render::view::InheritedVisibility;
```

**Types affected:**
- `ViewVisibility` (line 10)
- `InheritedVisibility` (line 12)

---

##### [`src/systems/name_tag_system.rs`](src/systems/name_tag_system.rs)
**Current imports:**
```rust
use bevy::render::{
    view::{ViewVisibility, InheritedVisibility, NoFrustumCulling},
};
```

**Types affected:**
- `ViewVisibility` (line 16)
- `InheritedVisibility` (line 16)
- `NoFrustumCulling` (lines 16, 503, 655, 670, 685, 704)

---

##### [`src/systems/zone_render_validation_system.rs`](src/systems/zone_render_validation_system.rs)
**Current imports:**
```rust
use bevy::render::{mesh::Mesh, primitives::Aabb, view::Visibility};
use bevy::render::mesh::Mesh3d;
```

**Types affected:**
- `Mesh` (line 7)
- `Aabb` (line 7)
- `Visibility` (line 7)
- `Mesh3d` (line 8)

---

##### [`src/systems/zone_time_system.rs`](src/systems/zone_time_system.rs)
**Current imports:**
```rust
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
```

**Types affected:**
- `ColorGrading` (line 6)
- `ColorGradingGlobal` (line 6)
- `ColorGradingSection` (line 6)

---

##### [`src/systems/chat_bubble_spawn_system.rs`](src/systems/chat_bubble_spawn_system.rs)
**Current imports:**
```rust
use bevy::render::{
    view::NoFrustumCulling,
};
```

**Types affected:**
- `NoFrustumCulling` (lines 13, 324, 349, 374)
- `InheritedVisibility` (line 8)
- `ViewVisibility` (line 8)

---

##### [`src/systems/character_select_system.rs`](src/systems/character_select_system.rs)
**Current imports:**
```rust
use bevy::prelude::{
    Camera, Camera3d, ViewVisibility, InheritedVisibility, Visibility,
};
```

**Types affected:**
- `Camera` (line 8)
- `Camera3d` (line 8)
- `ViewVisibility` (line 8)
- `InheritedVisibility` (line 8)
- `Visibility` (line 10)

---

##### [`src/systems/debug_render_directional_light_system.rs`](src/systems/debug_render_directional_light_system.rs)
**Current imports:**
```rust
use bevy::render::primitives::{CascadesFrusta, Frustum, HalfSpace};
```

**Types affected:**
- `Frustum` (lines 4, 22)
- `CascadesFrusta` (line 4)
- `HalfSpace` (line 4)

---

##### [`src/systems/debug_inspector_system.rs`](src/systems/debug_inspector_system.rs)
**Current imports:**
```rust
use bevy::prelude::{Camera, Camera3d, GlobalTransform, ...};
```

**Types affected:**
- `Camera` (line 4)
- `Camera3d` (line 4)

---

##### [`src/systems/vehicle_model_system.rs`](src/systems/vehicle_model_system.rs)
**Current imports:**
```rust
use bevy::prelude::{ViewVisibility, InheritedVisibility, Visibility, ...};
```

**Types affected:**
- `ViewVisibility` (line 4)
- `InheritedVisibility` (line 4)
- `Visibility` (line 5)

---

##### [`src/systems/model_viewer_system.rs`](src/systems/model_viewer_system.rs)
**Current imports:**
```rust
use bevy::render::view::{ViewVisibility, InheritedVisibility};
```

**Types affected:**
- `ViewVisibility` (line 10)
- `InheritedVisibility` (line 10)
- `Camera3d` (line 7)

---

##### [`src/systems/move_destination_effect_system.rs`](src/systems/move_destination_effect_system.rs)
**Current imports:**
```rust
use bevy::prelude::{ViewVisibility, InheritedVisibility, Visibility, ...};
```

**Types affected:**
- `ViewVisibility` (lines 4, 56)
- `InheritedVisibility` (lines 4, 54, 55)
- `Visibility` (lines 5, 55)

---

##### [`src/systems/spawn_projectile_system.rs`](src/systems/spawn_projectile_system.rs)
**Current imports:**
```rust
use bevy::render::{mesh::skinning::SkinnedMesh, view::InheritedVisibility, view::ViewVisibility};
```

**Types affected:**
- `InheritedVisibility` (lines 6, 71)
- `ViewVisibility` (lines 6, 72)

---

##### [`src/systems/visible_status_effects_system.rs`](src/systems/visible_status_effects_system.rs)
**Current imports:**
```rust
use bevy::prelude::{ViewVisibility, InheritedVisibility, Visibility, ...};
```

**Types affected:**
- `ViewVisibility` (line 3)
- `InheritedVisibility` (line 3)
- `Visibility` (line 4)

---

##### [`src/systems/character_model_add_collider_system.rs`](src/systems/character_model_add_collider_system.rs)
**Current imports:**
```rust
use bevy::render::primitives::Aabb;
```

**Types affected:**
- `Aabb` (lines 11, 29, 33)

---

##### [`src/systems/npc_model_add_collider_system.rs`](src/systems/npc_model_add_collider_system.rs)
**Current imports:**
```rust
use bevy::render::primitives::Aabb;
```

**Types affected:**
- `Aabb` (lines 11, 29, 30)

---

##### [`src/systems/personal_store_model_add_collider_system.rs`](src/systems/personal_store_model_add_collider_system.rs)
**Current imports:**
```rust
use bevy::render::primitives::Aabb;
```

**Types affected:**
- `Aabb` (lines 8, 24)

---

##### [`src/systems/item_drop_model_system.rs`](src/systems/item_drop_model_system.rs)
**Current imports:**
```rust
use bevy::render::primitives::Aabb;
```

**Types affected:**
- `Aabb` (lines 11, 70)

---

##### [`src/systems/fish_system.rs`](src/systems/fish_system.rs)
**Types affected:**
- `Visibility` (lines 181, 182, 192, 193)
- `InheritedVisibility` (lines 182, 193)
- `ViewVisibility` (lines 183, 194)

---

##### [`src/systems/bird_system.rs`](src/systems/bird_system.rs)
**Types affected:**
- `Visibility` (lines 210, 221, 234, 247)
- `InheritedVisibility` (lines 211, 222, 235, 248)
- `ViewVisibility` (lines 212, 223, 236, 249)

---

##### [`src/systems/dirt_dash_system.rs`](src/systems/dirt_dash_system.rs)
**Types affected:**
- `Visibility` (line 184)
- `InheritedVisibility` (line 185)
- `ViewVisibility` (line 186)

---

##### [`src/systems/wind_effect_system.rs`](src/systems/wind_effect_system.rs)
**Types affected:**
- `Visibility` (line 249)
- `InheritedVisibility` (line 250)
- `ViewVisibility` (line 251)

---

##### [`src/systems/wing_spawn_system.rs`](src/systems/wing_spawn_system.rs)
**Types affected:**
- `Visibility` (lines 161, 192)
- `InheritedVisibility` (lines 162, 193)
- `ViewVisibility` (lines 163, 194)

---

##### [`src/ui/ui_debug_physics.rs`](src/ui/ui_debug_physics.rs)
**Current imports:**
```rust
use bevy::render::view::{ViewVisibility, InheritedVisibility};
```

**Types affected:**
- `ViewVisibility` (line 9)
- `InheritedVisibility` (line 9)
- `Camera` (line 5)
- `Camera3d` (line 5)
- `Visibility` (line 6)

---

##### [`src/ui/ui_debug_effect_list.rs`](src/ui/ui_debug_effect_list.rs)
**Current imports:**
```rust
use bevy::render::view::{ViewVisibility, InheritedVisibility};
```

**Types affected:**
- `ViewVisibility` (line 7)
- `InheritedVisibility` (line 7)
- `Visibility` (line 5)

---

##### [`src/ui/ui_character_create_system.rs`](src/ui/ui_character_create_system.rs)
**Current imports:**
```rust
use bevy::render::view::{ViewVisibility, InheritedVisibility};
```

**Types affected:**
- `ViewVisibility` (lines 8, 372)
- `InheritedVisibility` (lines 8, 370)
- `Visibility` (line 5)

---

##### [`src/ui/ui_item_drop_name_system.rs`](src/ui/ui_item_drop_name_system.rs)
**Types affected:**
- `Camera` (line 3)
- `Camera3d` (line 3)

---

##### [`src/ui/ui_minimap_system.rs`](src/ui/ui_minimap_system.rs)
**Types affected:**
- `Camera3d` (lines 7, 96)

---

##### [`src/ui/ui_debug_window_system.rs`](src/ui/ui_debug_window_system.rs)
**Types affected:**
- `Camera3d` (lines 4, 60)

---

##### [`src/ui/ui_character_select_system.rs`](src/ui/ui_character_select_system.rs)
**Types affected:**
- `Camera3d` (lines 3, 39)

---

##### [`src/ui/ui_character_select_name_tag_system.rs`](src/ui/ui_character_select_name_tag_system.rs)
**Types affected:**
- `Camera` (line 1)
- `Camera3d` (lines 1, 8)

---

##### [`src/ui/ui_debug_entity_inspector_system.rs`](src/ui/ui_debug_entity_inspector_system.rs)
**Types affected:**
- `Camera3d` (lines 2, 35)

---

##### [`src/ui/ui_settings_system.rs`](src/ui/ui_settings_system.rs)
**Current imports:**
```rust
use bevy::core_pipeline::dof::DepthOfFieldMode;
```

**Types affected:**
- `DepthOfFieldMode` (line 1)

---

##### [`src/ui/ui_debug_zone_lighting_system.rs`](src/ui/ui_debug_zone_lighting_system.rs)
**Current imports:**
```rust
use bevy::core_pipeline::bloom::Bloom;
```

**Types affected:**
- `Bloom` (lines 2, 13)

---

##### [`src/resources/zone_debug_diagnostics.rs`](src/resources/zone_debug_diagnostics.rs)
**Types affected:**
- `Visibility` (lines 240, 244)
- `InheritedVisibility` (lines 240, 246)
- `ViewVisibility` (lines 240, 247)

---

##### [`src/resources/damage_digits_spawner.rs`](src/resources/damage_digits_spawner.rs)
**Current imports:**
```rust
use bevy::render::{primitives::Aabb, view::NoFrustumCulling};
```

**Types affected:**
- `Aabb` (line 7)
- `NoFrustumCulling` (line 7)
- `Visibility` (lines 75, 87)
- `InheritedVisibility` (lines 76, 88)
- `ViewVisibility` (lines 77, 89)

---

##### [`src/render/particle_render_data.rs`](src/render/particle_render_data.rs)
**Current imports:**
```rust
use bevy::{math::*, prelude::*, render::primitives::Aabb};
```

**Types affected:**
- `Aabb` (lines 1, 63, 74, 75)

---

##### [`src/render/zone_lighting.rs`](src/render/zone_lighting.rs)
**Current imports:**
```rust
use bevy::render::camera::Exposure;
use bevy::render::view::RenderLayers;
```

**Types affected:**
- `Exposure` (line 16)
- `RenderLayers` (lines 26, 113)

---

##### [`src/render/underwater_effect.rs`](src/render/underwater_effect.rs)
**Current imports:**
```rust
use bevy::render::view::{ExtractedView, ViewTarget};
```

**Types affected:**
- `ExtractedView` (lines 35, 426)

---

##### [`src/render/world_ui.rs`](src/render/world_ui.rs)
**Current imports:**
```rust
use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::render::view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms};
```

**Types affected:**
- `Transparent3d` (lines 8, 69, 137, 481, 486, 678)
- `ExtractedView` (lines 31, 485)
- `ViewUniformOffset` (lines 31, 446)
- `ViewTarget` (line 31)
- `ViewUniform` (line 31)
- `ViewUniforms` (line 31)

---

##### [`src/effect_loader.rs`](src/effect_loader.rs)
**Current imports:**
```rust
use bevy::render::{
    primitives::Aabb,
    view::{ViewVisibility, InheritedVisibility, NoFrustumCulling},
};
```

**Types affected:**
- `Aabb` (line 10)
- `ViewVisibility` (lines 13, 86, 181, 224, 298, 374)
- `InheritedVisibility` (lines 13, 85, 180, 223, 297, 373)
- `NoFrustumCulling` (lines 13, 236, 376)
- `Visibility` (lines 6, 84, 179, 222, 296, 372)

---

##### [`src/map_editor/systems/model_placement_system.rs`](src/map_editor/systems/model_placement_system.rs)
**Current imports:**
```rust
use bevy::render::{alpha::AlphaMode, view::RenderLayers};
```

**Types affected:**
- `RenderLayers` (lines 17, 421)
- `Camera` (line 8)
- `Camera3d` (lines 8, 77, 473, 583)
- `Visibility` (lines 10, 276, 410)
- `InheritedVisibility` (lines 10, 277, 411)
- `ViewVisibility` (lines 10, 278, 412)
- `NoFrustumCulling` (line 416)
- `Aabb` (line 417)

---

##### [`src/map_editor/systems/duplicate_system.rs`](src/map_editor/systems/duplicate_system.rs)
**Current imports:**
```rust
use bevy::render::{alpha::AlphaMode, view::RenderLayers};
```

**Types affected:**
- `RenderLayers` (lines 9, 345)
- `Visibility` (lines 112, 113, 307)
- `InheritedVisibility` (lines 113, 308)
- `ViewVisibility` (lines 114, 309)
- `NoFrustumCulling` (line 340)
- `Aabb` (line 341)

---

##### [`src/map_editor/systems/selection_system.rs`](src/map_editor/systems/selection_system.rs)
**Types affected:**
- `Camera` (line 9)
- `Camera3d` (lines 9, 55)

---

##### [`src/map_editor/systems/keyboard_shortcuts_system.rs`](src/map_editor/systems/keyboard_shortcuts_system.rs)
**Types affected:**
- `Camera3d` (lines 32, 110)

---

##### [`src/map_editor/systems/grid_system.rs`](src/map_editor/systems/grid_system.rs)
**Types affected:**
- `InheritedVisibility` (lines 9, 140)

---

##### [`src/map_editor/systems/selection_highlight_system.rs`](src/map_editor/systems/selection_highlight_system.rs)
**Types affected:**
- `InheritedVisibility` (lines 9, 37)

---

##### [`src/map_editor/mod.rs`](src/map_editor/mod.rs)
**Types affected:**
- `Camera3d` (lines 153)

---

##### [`src/audio/spatial_sound.rs`](src/audio/spatial_sound.rs)
**Types affected:**
- `Camera3d` (lines 5, 89)

---

##### [`src/animation/camera_animation.rs`](src/animation/camera_animation.rs)
**Current imports:**
```rust
use bevy::prelude::{Projection, Query, ...};
```

**Types affected:**
- `Projection` (lines 4, 30)
- `PerspectiveProjection` (line 100)

---

### 2. bevy_mesh Migration

Types moving to `bevy_mesh`:
- `Mesh`
- `Mesh3d`
- `Indices`
- `PrimitiveTopology`
- `VertexAttributeValues`
- `MeshVertexBufferLayoutRef`
- `SkinnedMesh`
- `SkinnedMeshInverseBindposes`

#### Files Affected:

##### [`src/zone_loader.rs`](src/zone_loader.rs)
**Types affected:**
- `Mesh` (line 293)
- `Indices` (line 293)
- `PrimitiveTopology` (lines 293, 2488, 2635)

---

##### [`src/model_loader.rs`](src/model_loader.rs)
**Types affected:**
- `Mesh` (line 9)
- `Mesh3d` (line 9)
- `SkinnedMesh` (line 13)
- `SkinnedMeshInverseBindposes` (line 13)

---

##### [`src/zms_asset_loader.rs`](src/zms_asset_loader.rs)
**Current imports:**
```rust
use bevy::render::{
    mesh::{Indices, VertexAttributeValues},
    render_asset::RenderAssetUsages,
    render_resource::PrimitiveTopology,
};
```

**Types affected:**
- `Indices` (line 13)
- `VertexAttributeValues` (line 13)
- `PrimitiveTopology` (lines 15, 75, 191)

---

##### [`src/systems/debug_rendering_system.rs`](src/systems/debug_rendering_system.rs)
**Types affected:**
- `Mesh` (line 7)
- `Mesh3d` (line 8)

---

##### [`src/systems/fish_system.rs`](src/systems/fish_system.rs)
**Current imports:**
```rust
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
```

**Types affected:**
- `Mesh` (line 11)
- `Indices` (line 11)
- `PrimitiveTopology` (lines 11, 311)

---

##### [`src/systems/bird_system.rs`](src/systems/bird_system.rs)
**Current imports:**
```rust
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
```

**Types affected:**
- `Mesh` (line 11)
- `Indices` (line 11)
- `PrimitiveTopology` (lines 11, 411, 497, 582)

---

##### [`src/systems/wing_spawn_system.rs`](src/systems/wing_spawn_system.rs)
**Current imports:**
```rust
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};
```

**Types affected:**
- `Mesh` (line 13)
- `Indices` (line 13)
- `PrimitiveTopology` (lines 13, 486)

---

##### [`src/systems/zone_render_validation_system.rs`](src/systems/zone_render_validation_system.rs)
**Types affected:**
- `Mesh` (line 7)
- `Mesh3d` (line 8)
- `VertexAttributeValues` (line 283)

---

##### [`src/systems/zone_memory_protection_system.rs`](src/systems/zone_memory_protection_system.rs)
**Current imports:**
```rust
use bevy::render::mesh::Mesh3d;
```

**Types affected:**
- `Mesh3d` (line 9)

---

##### [`src/systems/particle_sequence_system.rs`](src/systems/particle_sequence_system.rs)
**Current imports:**
```rust
use bevy::render::{
    mesh::{Indices, Mesh, PrimitiveTopology},
};
```

**Types affected:**
- `Indices` (line 10)
- `Mesh` (line 10)
- `PrimitiveTopology` (lines 10, 580)

---

##### [`src/systems/name_tag_system.rs`](src/systems/name_tag_system.rs)
**Types affected:**
- `Mesh` (line 5)

---

##### [`src/systems/chat_bubble_spawn_system.rs`](src/systems/chat_bubble_spawn_system.rs)
**Types affected:**
- `Mesh` (line 5)

---

##### [`src/systems/move_destination_effect_system.rs`](src/systems/move_destination_effect_system.rs)
**Types affected:**
- `Mesh` (line 5)

---

##### [`src/systems/npc_model_system.rs`](src/systems/npc_model_system.rs)
**Types affected:**
- `Mesh` (line 46)

---

##### [`src/systems/vehicle_model_system.rs`](src/systems/vehicle_model_system.rs)
**Types affected:**
- `Mesh` (line 44)

---

##### [`src/systems/spawn_effect_system.rs`](src/systems/spawn_effect_system.rs)
**Types affected:**
- `Mesh` (line 46)

---

##### [`src/systems/character_model_add_collider_system.rs`](src/systems/character_model_add_collider_system.rs)
**Types affected:**
- `SkinnedMesh` (line 10)
- `SkinnedMeshInverseBindposes` (line 10)

---

##### [`src/systems/npc_model_add_collider_system.rs`](src/systems/npc_model_add_collider_system.rs)
**Types affected:**
- `SkinnedMesh` (line 10)
- `SkinnedMeshInverseBindposes` (line 10)

---

##### [`src/render/mod.rs`](src/render/mod.rs)
**Current imports:**
```rust
use bevy::render::{mesh::MeshVertexAttribute, render_resource::VertexFormat};
```

**Types affected:**
- `MeshVertexAttribute` (line 3)

---

##### [`src/render/object_material_extension.rs`](src/render/object_material_extension.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (lines 12, 61)

---

##### [`src/render/sky_material.rs`](src/render/sky_material.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (lines 7, 88)

---

##### [`src/render/cartoon_sky_material.rs`](src/render/cartoon_sky_material.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (lines 15, 308)

---

##### [`src/render/terrain_material.rs`](src/render/terrain_material.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (lines 21, 102)

---

##### [`src/render/water_material.rs`](src/render/water_material.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (lines 24, 156)

---

##### [`src/render/extension_material_plugin.rs`](src/render/extension_material_plugin.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (line 18)

---

##### [`src/render/particle_material.rs`](src/render/particle_material.rs)
**Current imports:**
```rust
use bevy::render::mesh::MeshVertexBufferLayoutRef;
```

**Types affected:**
- `MeshVertexBufferLayoutRef` (lines 7, 73)

---

##### [`src/effect_loader.rs`](src/effect_loader.rs)
**Types affected:**
- `Mesh` (lines 35, 270, 358)
- `PrimitiveTopology` (line 359)

---

##### [`src/map_editor/systems/model_placement_system.rs`](src/map_editor/systems/model_placement_system.rs)
**Types affected:**
- `Mesh` (lines 11, 292)
- `Mesh3d` (line 11)

---

##### [`src/map_editor/systems/grid_system.rs`](src/map_editor/systems/grid_system.rs)
**Types affected:**
- `Mesh` (line 9)

---

##### [`src/resources/zone_debug_diagnostics.rs`](src/resources/zone_debug_diagnostics.rs)
**Current imports:**
```rust
use bevy::render::mesh::Mesh3d;
```

**Types affected:**
- `Mesh3d` (line 9)

---

### 3. bevy_post_process Migration

Types moving to `bevy_post_process`:
- `Bloom`
- `DepthOfField`
- `DepthOfFieldMode`
- `Tonemapping`

#### Files Affected:

##### [`src/lib.rs`](src/lib.rs)
**Current imports:**
```rust
use bevy::{
    core_pipeline::bloom::Bloom,
    core_pipeline::dof::{DepthOfField, DepthOfFieldMode},
    core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass},
};
```

**Types affected:**
- `Bloom` (lines 8, 1774)
- `DepthOfField` (lines 9, 1793, 1981)
- `DepthOfFieldMode` (lines 9, 1794, 1997)
- `Tonemapping` (line 1772)

---

##### [`src/ui/ui_settings_system.rs`](src/ui/ui_settings_system.rs)
**Current imports:**
```rust
use bevy::core_pipeline::dof::DepthOfFieldMode;
```

**Types affected:**
- `DepthOfFieldMode` (line 1)

---

##### [`src/ui/ui_debug_zone_lighting_system.rs`](src/ui/ui_debug_zone_lighting_system.rs)
**Current imports:**
```rust
use bevy::core_pipeline::bloom::Bloom;
```

**Types affected:**
- `Bloom` (lines 2, 13)

---

### 4. bevy_anti_alias Migration

Types moving to `bevy_anti_alias`:
- `Smaa`
- `TemporalAntiAliasing`

#### Files Affected:

##### [`src/lib.rs`](src/lib.rs)
**Current imports:**
```rust
use bevy::core_pipeline::smaa::Smaa;
```

**Types affected:**
- `Smaa` (lines 11, 1778)

---

### 5. bevy_core_pipeline Types (May Stay or Move)

Types that may remain in `bevy_core_pipeline` or move:
- `Opaque3d`
- `Transparent3d`
- `DepthPrepass`
- `MotionVectorPrepass`

#### Files Affected:

##### [`src/lib.rs`](src/lib.rs)
**Types affected:**
- `DepthPrepass` (lines 10, 1780)
- `MotionVectorPrepass` (line 10)

---

##### [`src/systems/debug_rendering_system.rs`](src/systems/debug_rendering_system.rs)
**Current imports:**
```rust
use bevy::core_pipeline::core_3d::{Opaque3d, Transparent3d};
```

**Types affected:**
- `Opaque3d` (line 5)
- `Transparent3d` (lines 5, 1446)

---

##### [`src/render/world_ui.rs`](src/render/world_ui.rs)
**Current imports:**
```rust
use bevy::core_pipeline::core_3d::Transparent3d;
```

**Types affected:**
- `Transparent3d` (lines 8, 69, 678)

---

##### [`src/render/post_processing.rs`](src/render/post_processing.rs)
**Current imports:**
```rust
use bevy::core_pipeline::{
    core_3d::Opaque3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
```

**Types affected:**
- `Opaque3d` (lines 4, 137)

---

### 6. bevy_render Types That Stay

Types that remain in `bevy_render`:
- `RenderAssetUsages`
- `ShaderStorageBuffer`
- `RenderApp`, `Render`, `ExtractSchedule`, `RenderSet`
- `Extract`
- Various render resource types

#### Files Using These Types:

##### [`src/zone_loader.rs`](src/zone_loader.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 1170, 3216)
- `RenderAssetUsages` (line 295)

---

##### [`src/model_loader.rs`](src/model_loader.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 15, 247, 987)
- `RenderAssetUsages` (line 14)

---

##### [`src/effect_loader.rs`](src/effect_loader.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 12, 36, 271, 331-334)
- `RenderAssetUsages` (lines 2, 359)

---

##### [`src/systems/particle_sequence_system.rs`](src/systems/particle_sequence_system.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 13, 499, 530-543)
- `RenderAssetUsages` (lines 11, 48)

---

##### [`src/systems/damage_digit_render_system.rs`](src/systems/damage_digit_render_system.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 5, 25, 30-32, 58, 116-118)

---

##### [`src/render/damage_digit_material.rs`](src/render/damage_digit_material.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 3, 14, 17, 20)

---

##### [`src/render/particle_material.rs`](src/render/particle_material.rs)
**Types that stay:**
- `ShaderStorageBuffer` (lines 9, 22, 25, 28, 31, 173)

---

##### [`src/animation/zmo_asset_loader.rs`](src/animation/zmo_asset_loader.rs)
**Types that stay:**
- `RenderAssetUsages` (lines 6, 375)

---

##### [`src/animation/zmo_asset_loader_fixed.rs`](src/animation/zmo_asset_loader_fixed.rs)
**Types that stay:**
- `RenderAssetUsages` (lines 8, 367)

---

##### [`src/dds_image_loader.rs`](src/dds_image_loader.rs)
**Types that stay:**
- `RenderAssetUsages` (lines 5, 338)

---

##### [`src/systems/name_tag_system.rs`](src/systems/name_tag_system.rs)
**Types that stay:**
- `RenderAssetUsages` (lines 14, 346)

---

##### [`src/systems/chat_bubble_spawn_system.rs`](src/systems/chat_bubble_spawn_system.rs)
**Types that stay:**
- `RenderAssetUsages` (lines 11, 247, 309)

---

##### [`src/systems/blood_spatter_system.rs`](src/systems/blood_spatter_system.rs)
**Types that stay:**
- `RenderAssetUsages` (lines 7, 238)

---

---

## Summary Table

| Crate | Files Affected | Key Types |
|-------|----------------|-----------|
| `bevy_camera` | 35+ | Camera, Camera3d, Visibility, Aabb, RenderLayers, Exposure, ColorGrading |
| `bevy_mesh` | 20+ | Mesh, Mesh3d, Indices, PrimitiveTopology, SkinnedMesh |
| `bevy_post_process` | 3 | Bloom, DepthOfField, DepthOfFieldMode, Tonemapping |
| `bevy_anti_alias` | 1 | Smaa |
| `bevy_core_pipeline` (unchanged or minimal) | 4 | Opaque3d, Transparent3d, DepthPrepass |
| `bevy_render` (stays) | 15+ | ShaderStorageBuffer, RenderAssetUsages, render_resource types |

---

## Migration Strategy Recommendations

### Phase 1: Add New Crate Dependencies
Add the following to [`Cargo.toml`](Cargo.toml):
```toml
[dependencies]
bevy_camera = "0.17"
bevy_mesh = "0.17"
bevy_post_process = "0.17"
bevy_anti_alias = "0.17"
```

### Phase 2: Update Imports File by File
Process files in this order:
1. [`src/lib.rs`](src/lib.rs) - Main imports, most critical
2. [`src/zone_loader.rs`](src/zone_loader.rs) - Heavy render usage
3. [`src/model_loader.rs`](src/model_loader.rs) - Heavy render usage
4. [`src/render/`](src/render/) directory files
5. [`src/systems/`](src/systems/) files
6. [`src/ui/`](src/ui/) files
7. [`src/map_editor/`](src/map_editor/) files

### Phase 3: Handle Type Renames
Watch for renamed types:
- `TemporalAntiAlias` → `TemporalAntiAliasing` (note the additional 'i')

### Phase 4: Test Rendering
After migration:
1. Verify camera rendering works
2. Check mesh loading and display
3. Test post-processing effects (Bloom, DoF, SMAA)
4. Verify visibility culling behavior

---

## Notes

1. **Some types may be re-exported**: Bevy often re-exports types through `bevy::prelude`, so some imports may continue to work through prelude even after the move.

2. **Check migration guide**: Always refer to the official [Bevy 0.17 Migration Guide](bevy-0.16-to-0.17-migration-guide.md) for the most up-to-date information.

3. **Render pipeline types**: Types like `RenderApp`, `RenderSet`, `ExtractSchedule` are expected to remain in `bevy_render`.

4. **Storage buffers**: `ShaderStorageBuffer` appears to remain in `bevy_render::storage`.

---

## Files Not Requiring Changes

The following files were analyzed and found to not require render crate import changes:
- Files using only `bevy::prelude` types that haven't moved
- Files using only ECS types (Commands, Query, Entity, etc.)
- Files using only math types (Vec3, Transform, etc.)
- Files using only asset types through `Assets<T>` handles
