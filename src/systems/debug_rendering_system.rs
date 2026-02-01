use bevy::prelude::*;
use bevy::render::view::Visibility;
use bevy::render::primitives::Aabb;
use crate::components::{Zone, ZoneObject};
use crate::render::{
    EffectMeshMaterial, TerrainMaterial, WaterMaterial, SkyMaterial, 
    ParticleMaterial, DamageDigitMaterial, ObjectMaterial
};

/// Debug system to log entity visibility information
pub fn debug_entity_visibility(
    query: Query<(
        Entity,
        &Transform,
        &ViewVisibility,
        &Visibility,
        Option<&Name>,
    ), (With<Handle<Mesh>>, Without<Camera>)>,
) {
    let total_entities = query.iter().count();
    
    if total_entities == 0 {
        warn!("[DEBUG] No mesh entities found in scene!");
        return;
    }
    
    let visible_count = query.iter()
        .filter(|(_, _, view_vis, _, _)| view_vis.get())
        .count();
    
    // Count visibility component states
    let mut visible_component_count = 0;
    let mut hidden_component_count = 0;
    let mut inherited_component_count = 0;
    
    for (_, _, _, visibility, _) in query.iter() {
        match visibility {
            Visibility::Visible => visible_component_count += 1,
            Visibility::Hidden => hidden_component_count += 1,
            Visibility::Inherited => inherited_component_count += 1,
        }
    }
    
    info!("[DEBUG] Entity visibility stats:");
    info!("[DEBUG]   Total mesh entities: {}", total_entities);
    info!("[DEBUG]   Visible entities (ViewVisibility): {}", visible_count);
    info!("[DEBUG]   Hidden entities (ViewVisibility): {}", total_entities - visible_count);
    info!("[DEBUG]   Visibility component states:");
    info!("[DEBUG]     Visible: {}", visible_component_count);
    info!("[DEBUG]     Hidden: {}", hidden_component_count);
    info!("[DEBUG]     Inherited: {}", inherited_component_count);
    
    // Log first 5 visible entities with more detail
    let mut visible_logged = 0;
    for (entity, transform, view_vis, visibility, name) in query.iter() {
        if view_vis.get() && visible_logged < 5 {
            let name_str = name.map(|n| n.as_str()).unwrap_or("<unnamed>");
            let visibility_str = match visibility {
                Visibility::Visible => "Visible",
                Visibility::Hidden => "Hidden",
                Visibility::Inherited => "Inherited",
            };
            info!("[DEBUG]   Visible entity '{}': pos=({:.1}, {:.1}, {:.1}), scale=({:.1}, {:.1}, {:.1}), visibility={}",
                  name_str, transform.translation.x, transform.translation.y, transform.translation.z,
                  transform.scale.x, transform.scale.y, transform.scale.z, visibility_str);
            visible_logged += 1;
        }
    }
    
    // Log first 5 hidden entities with more detail
    let mut hidden_logged = 0;
    for (entity, transform, view_vis, visibility, name) in query.iter() {
        if !view_vis.get() && hidden_logged < 5 {
            let name_str = name.map(|n| n.as_str()).unwrap_or("<unnamed>");
            let visibility_str = match visibility {
                Visibility::Visible => "Visible",
                Visibility::Hidden => "Hidden",
                Visibility::Inherited => "Inherited",
            };
            info!("[DEBUG]   Hidden entity '{}': pos=({:.1}, {:.1}, {:.1}), scale=({:.1}, {:.1}, {:.1}), visibility={}",
                  name_str, transform.translation.x, transform.translation.y, transform.translation.z,
                  transform.scale.x, transform.scale.y, transform.scale.z, visibility_str);
            hidden_logged += 1;
        }
    }
}

/// Comprehensive render diagnostic system that runs every frame
/// to help diagnose black screen issues
pub fn render_diagnostics_system(
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
    )>,
    meshes: Query<(
        Entity,
        &Handle<Mesh>,
        &GlobalTransform,
        &ViewVisibility,
        &Visibility,
        Option<&Handle<StandardMaterial>>,
        Option<&Handle<EffectMeshMaterial>>,
        Option<&Handle<TerrainMaterial>>,
        Option<&Handle<WaterMaterial>>,
        Option<&Handle<SkyMaterial>>,
        Option<&Handle<ParticleMaterial>>,
        Option<&Handle<DamageDigitMaterial>>,
        Option<&Handle<ObjectMaterial>>,
    )>,
    mesh_assets: Res<Assets<Mesh>>,
    material_assets: Res<Assets<StandardMaterial>>,
    effect_material_assets: Res<Assets<EffectMeshMaterial>>,
    terrain_material_assets: Res<Assets<TerrainMaterial>>,
    water_material_assets: Res<Assets<WaterMaterial>>,
    sky_material_assets: Res<Assets<SkyMaterial>>,
    particle_material_assets: Res<Assets<ParticleMaterial>>,
    damage_digit_material_assets: Res<Assets<DamageDigitMaterial>>,
    object_material_assets: Res<Assets<ObjectMaterial>>,
    images: Res<Assets<Image>>,
    windows: Query<&Window>,
) {
    use bevy::log::info;
    
    info!("========================================");
    info!("[RENDER DIAGNOSTICS] Frame Report");
    info!("========================================");
    
    // Check window state
    for window in windows.iter() {
        info!("[RENDER DIAGNOSTICS] Window: {}x{}, present mode: {:?}",
            window.resolution.width(), window.resolution.height(), window.present_mode);
    }
    
    // Check camera state
    let camera_count = cameras.iter().count();
    info!("[RENDER DIAGNOSTICS] Cameras found: {}", camera_count);
    
    if camera_count == 0 {
        error!("[RENDER DIAGNOSTICS] CRITICAL: No cameras found! This will cause black screen.");
    } else {
        for (entity, camera, transform) in cameras.iter() {
            let position = transform.translation();
            let forward = transform.forward();
 
            info!("[RENDER DIAGNOSTICS] Camera {:?}:", entity);
            info!("[RENDER DIAGNOSTICS]   Position: ({:.2}, {:.2}, {:.2})", position.x, position.y, position.z);
            info!("[RENDER DIAGNOSTICS]   Forward vector: ({:.2}, {:.2}, {:.2})", forward.x, forward.y, forward.z);
            info!("[RENDER DIAGNOSTICS]   Is active: {}", camera.is_active);
            info!("[RENDER DIAGNOSTICS]   Target: {:?}", camera.target);
            
            // Check for invalid camera values
            if position.x.is_nan() || position.y.is_nan() || position.z.is_nan() {
                error!("[RENDER DIAGNOSTICS]   CRITICAL: Camera position contains NaN!");
            }
            
            if !camera.is_active {
                warn!("[RENDER DIAGNOSTICS]   WARNING: Camera is not active!");
            }
        }
    }
    
    // Check mesh entities
    let mesh_entity_count = meshes.iter().count();
    info!("[RENDER DIAGNOSTICS] Mesh entities: {}", mesh_entity_count);
    
    let visible_meshes = meshes.iter()
        .filter(|(_, _, _, vis, _, _, _, _, _, _, _, _, _)| vis.get())
        .count();
    info!("[RENDER DIAGNOSTICS] Visible mesh entities: {}", visible_meshes);
    
    // Count visibility component states
    let mut visible_component_count = 0;
    let mut hidden_component_count = 0;
    let mut inherited_component_count = 0;
    
    for (_, _, _, _, visibility, _, _, _, _, _, _, _, _) in meshes.iter() {
        match visibility {
            Visibility::Visible => visible_component_count += 1,
            Visibility::Hidden => hidden_component_count += 1,
            Visibility::Inherited => inherited_component_count += 1,
        }
    }
    
    info!("[RENDER DIAGNOSTICS] Visibility component states:");
    info!("[RENDER DIAGNOSTICS]   Visible: {}", visible_component_count);
    info!("[RENDER DIAGNOSTICS]   Hidden: {}", hidden_component_count);
    info!("[RENDER DIAGNOSTICS]   Inherited: {}", inherited_component_count);
    
    if mesh_entity_count == 0 {
        warn!("[RENDER DIAGNOSTICS] WARNING: No mesh entities in scene!");
    }
    
    if visible_meshes == 0 && mesh_entity_count > 0 {
        warn!("[RENDER DIAGNOSTICS] WARNING: Mesh entities exist but none are visible!");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes are in camera frustum");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes have Visibility::Visible");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes are behind the camera");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes have zero scale");
        warn!("[RENDER DIAGNOSTICS]   - Check if materials are fully transparent");
    }
    
    // Check first few mesh entities with more detail
    let mut logged = 0;
    for (entity, mesh_handle, transform, view_vis, visibility, 
         material, effect_material, terrain_material, water_material, 
         sky_material, particle_material, damage_digit_material, object_material) in meshes.iter() {
        if logged < 3 {
            let position = transform.translation();
            let visibility_str = match visibility {
                Visibility::Visible => "Visible",
                Visibility::Hidden => "Hidden",
                Visibility::Inherited => "Inherited",
            };
            
            let has_material = material.is_some() || effect_material.is_some() || 
                              terrain_material.is_some() || water_material.is_some() || 
                              sky_material.is_some() || particle_material.is_some() || 
                              damage_digit_material.is_some() || object_material.is_some();
            
            info!("[RENDER DIAGNOSTICS] Mesh entity {:?}:", entity);
            info!("[RENDER DIAGNOSTICS]   Position: ({:.2}, {:.2}, {:.2})", position.x, position.y, position.z);
            info!("[RENDER DIAGNOSTICS]   Visibility component: {}", visibility_str);
            info!("[RENDER DIAGNOSTICS]   ViewVisibility (computed): {}", view_vis.get());
            info!("[RENDER DIAGNOSTICS]   Has mesh asset: {}", mesh_assets.contains(mesh_handle));
            info!("[RENDER DIAGNOSTICS]   Has material: {}", has_material);
            
            if !has_material {
                warn!("[RENDER DIAGNOSTICS]   WARNING: Entity has no recognized material!");
            }

            // Check for NaN position
            if position.x.is_nan() || position.y.is_nan() || position.z.is_nan() {
                error!("[RENDER DIAGNOSTICS]   CRITICAL: Mesh position contains NaN!");
            }
            
            // Check material transparency for StandardMaterial
            if let Some(mat_handle) = material {
                if let Some(mat) = material_assets.get(mat_handle) {
                    let alpha = mat.base_color.a();
                    info!("[RENDER DIAGNOSTICS]   StandardMaterial alpha: {:.3}", alpha);
                    info!("[RENDER DIAGNOSTICS]   StandardMaterial alpha mode: {:?}", mat.alpha_mode);
                    if alpha < 0.01 {
                        warn!("[RENDER DIAGNOSTICS]   WARNING: Material is nearly transparent!");
                    }
                }
            }
            
            logged += 1;
        }
    }
    
    // Check asset counts
    info!("[RENDER DIAGNOSTICS] Asset counts:");
    info!("[RENDER DIAGNOSTICS]   Meshes: {}", mesh_assets.len());
    info!("[RENDER DIAGNOSTICS]   StandardMaterials: {}", material_assets.len());
    info!("[RENDER DIAGNOSTICS]   EffectMeshMaterials: {}", effect_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   TerrainMaterials: {}", terrain_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   WaterMaterials: {}", water_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   SkyMaterials: {}", sky_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   ParticleMaterials: {}", particle_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   DamageDigitMaterials: {}", damage_digit_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   ObjectMaterials: {}", object_material_assets.len());
    info!("[RENDER DIAGNOSTICS]   Images: {}", images.len());
    
    if mesh_assets.len() == 0 {
        warn!("[RENDER DIAGNOSTICS] WARNING: No mesh assets loaded!");
    }
    
    if images.len() == 0 {
        warn!("[RENDER DIAGNOSTICS] WARNING: No images/textures loaded!");
    }
    
    // Check for common black screen causes
    if camera_count > 0 && mesh_entity_count > 0 && visible_meshes == 0 {
        warn!("[RENDER DIAGNOSTICS] POSSIBLE CAUSE: Camera and meshes exist but meshes not visible");
        warn!("[RENDER DIAGNOSTICS]   - Camera may be facing wrong direction");
        warn!("[RENDER DIAGNOSTICS]   - Meshes may be outside camera frustum");
        warn!("[RENDER DIAGNOSTICS]   - Mesh transforms may be incorrect (zero scale, NaN)");
        warn!("[RENDER DIAGNOSTICS]   - Materials may be fully transparent");
        warn!("[RENDER DIAGNOSTICS]   - Visibility component may be set to Hidden");
    }
    
    if camera_count > 0 && mesh_entity_count == 0 {
        warn!("[RENDER DIAGNOSTICS] POSSIBLE CAUSE: Camera exists but no mesh entities");
        warn!("[RENDER DIAGNOSTICS]   - Zone may not be loading properly");
        warn!("[RENDER DIAGNOSTICS]   - Entities may be despawned");
    }
    
    if camera_count == 0 {
        error!("[RENDER DIAGNOSTICS] CRITICAL CAUSE: No cameras - nothing will render!");
    }
    
    info!("========================================");
}

/// Lightweight render diagnostics that runs every frame without being too verbose
pub fn render_diagnostics_system_lightweight(
    cameras: Query<(Entity, &Camera, &GlobalTransform)>,
    meshes: Query<&ViewVisibility, With<Handle<Mesh>>>,
    mesh_assets: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Always log first frame to confirm system is running
    if *frame_count == 1 {
        log::info!("[RENDER STATUS] Diagnostic system initialized - will report every 60 frames");
    }
    
    // Only log every 60 frames (approximately once per second at 60fps)
    if *frame_count % 60 != 0 {
        return;
    }
    
    let camera_count = cameras.iter().count();
    let mesh_entity_count = meshes.iter().count();
    let visible_meshes = meshes.iter().filter(|vis| vis.get()).count();
    let mesh_asset_count = mesh_assets.len();
    let image_count = images.len();
    
    // Log camera position for first camera
    if let Some((_, _, transform)) = cameras.iter().next() {
        let pos = transform.translation();
        log::info!("[RENDER STATUS] Frame {}: Cam pos=({:.1}, {:.1}, {:.1}), Meshes={}/{}, Assets={}/{} images",
            *frame_count, pos.x, pos.y, pos.z, visible_meshes, mesh_entity_count, mesh_asset_count, image_count);
    } else {
        log::warn!("[RENDER STATUS] Frame {}: NO CAMERA FOUND!", *frame_count);
    }
    
    // Log warnings if something looks wrong
    if camera_count == 0 {
        warn!("[RENDER STATUS] No cameras found!");
    }
    
    if mesh_entity_count > 0 && visible_meshes == 0 {
        warn!("[RENDER STATUS] {} meshes exist but none visible - possible culling or frustum issue", mesh_entity_count);
    }
}

/// Comprehensive frustum culling diagnostics to check if meshes are in camera view
pub fn frustum_culling_diagnostics(
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
    )>,
    meshes: Query<(
        Entity,
        &GlobalTransform,
        &ViewVisibility,
        &Visibility,
        Option<&Handle<Mesh>>,
    ), Without<Camera>>,
    mesh_assets: Res<Assets<Mesh>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }
    
    info!("========================================");
    info!("[FRUSTUM DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");
    
    for (cam_entity, camera, cam_transform) in cameras.iter() {
        info!("[FRUSTUM] Camera {:?}:", cam_entity);
        info!("[FRUSTUM]   Position: {:?}", cam_transform.translation());
        info!("[FRUSTUM]   Rotation (forward): {:?}", cam_transform.forward());
        info!("[FRUSTUM]   Is active: {}", camera.is_active);
        info!("[FRUSTUM]   Target: {:?}", camera.target);

        // Calculate distance to first few meshes
        let cam_pos = cam_transform.translation();
        let mut logged = 0;
        
        for (mesh_entity, mesh_transform, view_vis, visibility, mesh_handle) in meshes.iter() {
            if logged >= 5 {
                break;
            }

            let mesh_pos = mesh_transform.translation();
            let distance = cam_pos.distance(mesh_pos);
            let direction_to_mesh = (mesh_pos - cam_pos).normalize();
            let cam_forward = cam_transform.forward();
            let dot_product = cam_forward.dot(direction_to_mesh);
            let angle_to_mesh = dot_product.acos().to_degrees();
            
            let visibility_str = match visibility {
                Visibility::Visible => "Visible",
                Visibility::Hidden => "Hidden",
                Visibility::Inherited => "Inherited",
            };
            
            info!("[FRUSTUM]   Mesh {:?}:", mesh_entity);
            info!("[FRUSTUM]     Position: {:?}", mesh_pos);
            info!("[FRUSTUM]     Distance: {:.2}", distance);
            info!("[FRUSTUM]     Angle from camera forward: {:.2}°", angle_to_mesh);
            info!("[FRUSTUM]     Visibility component: {}", visibility_str);
            info!("[FRUSTUM]     ViewVisibility (computed): {}", view_vis.get());
            info!("[FRUSTUM]     In front of camera: {} (dot={:.2})",
                dot_product > 0.0, dot_product);
            
            // Check if mesh has valid asset
            if let Some(handle) = mesh_handle {
                info!("[FRUSTUM]     Mesh asset loaded: {}", mesh_assets.contains(handle));
            }
            
            logged += 1;
        }
    }
    
    info!("========================================");
}

/// Material transparency diagnostics to check if materials are rendering invisible
pub fn material_transparency_diagnostics(
    meshes: Query<(
        Entity,
        &GlobalTransform,
        &ViewVisibility,
        Option<&Handle<StandardMaterial>>,
        Option<&Handle<EffectMeshMaterial>>,
        Option<&Handle<TerrainMaterial>>,
        Option<&Handle<WaterMaterial>>,
        Option<&Handle<SkyMaterial>>,
        Option<&Handle<ParticleMaterial>>,
        Option<&Handle<DamageDigitMaterial>>,
        Option<&Handle<ObjectMaterial>>,
    )>,
    material_assets: Res<Assets<StandardMaterial>>,
    effect_material_assets: Res<Assets<EffectMeshMaterial>>,
    terrain_material_assets: Res<Assets<TerrainMaterial>>,
    water_material_assets: Res<Assets<WaterMaterial>>,
    sky_material_assets: Res<Assets<SkyMaterial>>,
    particle_material_assets: Res<Assets<ParticleMaterial>>,
    damage_digit_material_assets: Res<Assets<DamageDigitMaterial>>,
    object_material_assets: Res<Assets<ObjectMaterial>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }
    
    let mut transparent_count = 0;
    let mut opaque_count = 0;
    let mut no_material_count = 0;
    let mut logged = 0;
    
    info!("========================================");
    info!("[MATERIAL DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");
    
    for (entity, transform, view_vis, 
         material_handle, effect_handle, terrain_handle, water_handle, 
         sky_handle, particle_handle, damage_digit_handle, object_handle) in meshes.iter() {
        if logged >= 5 {
            break;
        }

        let position = transform.translation();
        let is_visible = view_vis.get();
        
        let mut found_material = false;

        // Check StandardMaterial
        if let Some(handle) = material_handle {
            found_material = true;
            if let Some(material) = material_assets.get(handle) {
                let alpha = material.base_color.a();
                let alpha_mode = material.alpha_mode;
                
                info!("[MATERIAL] Entity {:?} (StandardMaterial):", entity);
                info!("[MATERIAL]   Position: {:?}", position);
                info!("[MATERIAL]   ViewVisibility: {}", is_visible);
                info!("[MATERIAL]   Alpha: {:.3}", alpha);
                info!("[MATERIAL]   Alpha mode: {:?}", alpha_mode);
                
                if alpha < 0.01 {
                    warn!("[MATERIAL]   WARNING: Material is nearly invisible (alpha={:.3})", alpha);
                    transparent_count += 1;
                } else if alpha < 1.0 {
                    info!("[MATERIAL]   Material is partially transparent");
                    transparent_count += 1;
                } else {
                    opaque_count += 1;
                }
            } else {
                warn!("[MATERIAL] Entity {:?} has StandardMaterial handle but material not loaded!", entity);
            }
        }

        // Check EffectMeshMaterial
        if let Some(handle) = effect_handle {
            found_material = true;
            if effect_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (EffectMeshMaterial):", entity);
                info!("[MATERIAL]   Position: {:?}", position);
                info!("[MATERIAL]   ViewVisibility: {}", is_visible);
                opaque_count += 1; // Assume opaque for diagnostics unless we can check alpha
            } else {
                warn!("[MATERIAL] Entity {:?} has EffectMeshMaterial handle but material not loaded!", entity);
            }
        }

        // Check TerrainMaterial
        if let Some(handle) = terrain_handle {
            found_material = true;
            if terrain_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (TerrainMaterial):", entity);
                opaque_count += 1;
            }
        }

        // Check WaterMaterial
        if let Some(handle) = water_handle {
            found_material = true;
            if water_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (WaterMaterial):", entity);
                transparent_count += 1;
            }
        }

        // Check SkyMaterial
        if let Some(handle) = sky_handle {
            found_material = true;
            if sky_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (SkyMaterial):", entity);
                opaque_count += 1;
            }
        }

        // Check ParticleMaterial
        if let Some(handle) = particle_handle {
            found_material = true;
            if particle_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (ParticleMaterial):", entity);
                transparent_count += 1;
            }
        }

        // Check DamageDigitMaterial
        if let Some(handle) = damage_digit_handle {
            found_material = true;
            if damage_digit_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (DamageDigitMaterial):", entity);
                transparent_count += 1;
            }
        }

        // Check ObjectMaterial
        if let Some(handle) = object_handle {
            found_material = true;
            if object_material_assets.contains(handle) {
                info!("[MATERIAL] Entity {:?} (ObjectMaterial):", entity);
                opaque_count += 1;
            }
        }

        if !found_material {
            info!("[MATERIAL] Entity {:?} has no recognized material", entity);
            no_material_count += 1;
        }
        
        logged += 1;
    }
    
    info!("[MATERIAL] Summary: {} opaque, {} transparent, {} no material",
        opaque_count, transparent_count, no_material_count);
    info!("========================================");
}

/// Transform validation diagnostics to check for invalid transforms
pub fn transform_validation_diagnostics(
    meshes: Query<(
        Entity,
        &GlobalTransform,
        &ViewVisibility,
    )>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }
    
    let mut invalid_count = 0;
    let mut zero_scale_count = 0;
    let mut nan_count = 0;
    let mut logged = 0;
    
    info!("========================================");
    info!("[TRANSFORM DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");
    
    for (entity, transform, view_vis) in meshes.iter() {
        if logged >= 5 {
            break;
        }

        let translation = transform.translation();

        let has_nan = translation.x.is_nan() || translation.y.is_nan() || translation.z.is_nan();
        
        info!("[TRANSFORM] Entity {:?}:", entity);
        info!("[TRANSFORM]   Translation: {:?}", translation);
        info!("[TRANSFORM]   ViewVisibility: {}", view_vis.get());
        
        if has_nan {
            error!("[TRANSFORM]   CRITICAL: Transform contains NaN values!");
            nan_count += 1;
            invalid_count += 1;
        }
        
        logged += 1;
    }
    
    if invalid_count > 0 {
        warn!("[TRANSFORM] Found {} invalid transforms ({} with NaN, {} with zero scale)",
            invalid_count, nan_count, zero_scale_count);
    } else {
        info!("[TRANSFORM] All transforms appear valid");
    }
    
    info!("========================================");
}

/// Visibility component state diagnostics to check inherited visibility
pub fn visibility_state_diagnostics(
    meshes: Query<(
        Entity,
        &GlobalTransform,
        &ViewVisibility,
        &Visibility,
    )>,
    parents: Query<&Parent>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }
    
    let mut visible_count = 0;
    let mut hidden_count = 0;
    let mut inherited_count = 0;
    let mut mismatch_count = 0;
    let mut logged = 0;
    
    info!("========================================");
    info!("[VISIBILITY STATE DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");
    
    for (entity, transform, view_vis, visibility) in meshes.iter() {
        if logged >= 5 {
            break;
        }

        let position = transform.translation();
        let visibility_str = match visibility {
            Visibility::Visible => "Visible",
            Visibility::Hidden => "Hidden",
            Visibility::Inherited => "Inherited",
        };
        
        let is_visible = view_vis.get();
        
        // Check if there's a mismatch
        let is_mismatch = match visibility {
            Visibility::Visible => !is_visible,
            Visibility::Hidden => is_visible,
            Visibility::Inherited => false, // Inherited depends on parent
        };
        
        info!("[VISIBILITY] Entity {:?}:", entity);
        info!("[VISIBILITY]   Position: {:?}", position);
        info!("[VISIBILITY]   Visibility component: {}", visibility_str);
        info!("[VISIBILITY]   ViewVisibility (computed): {}", is_visible);
        
        // Check for parent
        if let Ok(parent) = parents.get(entity) {
            info!("[VISIBILITY]   Has parent: {:?}", parent.get());
            if *visibility == Visibility::Inherited {
                info!("[VISIBILITY]   Visibility depends on parent");
            }
        }
        
        if is_mismatch {
            warn!("[VISIBILITY]   MISMATCH: Visibility component is {} but ViewVisibility is {}!",
                visibility_str, is_visible);
            mismatch_count += 1;
        }
        
        match visibility {
            Visibility::Visible => visible_count += 1,
            Visibility::Hidden => hidden_count += 1,
            Visibility::Inherited => inherited_count += 1,
        }
        
        logged += 1;
    }
    
    info!("[VISIBILITY] Component states: {} Visible, {} Hidden, {} Inherited",
        visible_count, hidden_count, inherited_count);
    
    if mismatch_count > 0 {
        warn!("[VISIBILITY] Found {} visibility component mismatches!", mismatch_count);
    }
    
    info!("========================================");
}

/// Explicit active camera diagnostics to clearly show which camera is being used for rendering
pub fn active_camera_diagnostics(
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
    )>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Always log first frame to confirm system is running
    if *frame_count == 1 {
        log::info!("[ACTIVE CAMERA] Diagnostic system initialized - will report every 60 frames");
    }
    
    // Only run every 60 frames (approximately once per second at 60fps)
    if *frame_count % 60 != 0 {
        return;
    }
    
    info!("========================================");
    info!("[ACTIVE CAMERA] Frame {}", *frame_count);
    info!("========================================");
    
    let total_cameras = cameras.iter().count();
    info!("[ACTIVE CAMERA] Found {} camera(s) in scene", total_cameras);
    
    // Count active cameras
    let active_cameras: Vec<_> = cameras.iter()
        .filter(|(_, camera, _)| camera.is_active)
        .collect();
    
    let active_count = active_cameras.len();
    info!("[ACTIVE CAMERA] Active camera(s): {}", active_count);
    
    if total_cameras == 0 {
        warn!("[ACTIVE CAMERA] WARNING: No active camera found - this would cause black screen!");
        info!("========================================");
        return;
    }
    
    if active_count == 0 {
        warn!("[ACTIVE CAMERA] WARNING: NO active camera found - this would cause black screen!");
        warn!("[ACTIVE CAMERA] All {} camera(s) are inactive!", total_cameras);
    } else if active_count > 1 {
        warn!("[ACTIVE CAMERA] WARNING: Multiple active cameras found - this may cause rendering issues!");
        warn!("[ACTIVE CAMERA] Active cameras: {}", active_count);
    } else {
        info!("[ACTIVE CAMERA] Exactly one active camera - OK");
    }
    
    // Log details for each camera, highlighting active ones
    for (entity, camera, transform) in cameras.iter() {
        let position = transform.translation();
        let forward = transform.forward();
        let up = transform.up();

        if camera.is_active {
            info!("[ACTIVE CAMERA] *** ACTIVE CAMERA *** Entity: {:?}", entity);
            info!("[ACTIVE CAMERA]     Position: ({:.2}, {:.2}, {:.2})", position.x, position.y, position.z);
            info!("[ACTIVE CAMERA]     Forward: ({:.2}, {:.2}, {:.2})", forward.x, forward.y, forward.z);
            info!("[ACTIVE CAMERA]     Up: ({:.2}, {:.2}, {:.2})", up.x, up.y, up.z);
            info!("[ACTIVE CAMERA]     Target: {:?}", camera.target);
            info!("[ACTIVE CAMERA]     Order: {:?}", camera.order);
            
            // Check for invalid camera values
            if position.x.is_nan() || position.y.is_nan() || position.z.is_nan() {
                error!("[ACTIVE CAMERA]     CRITICAL: Active camera position contains NaN!");
            }
            
            if forward.x.is_nan() || forward.y.is_nan() || forward.z.is_nan() {
                error!("[ACTIVE CAMERA]     CRITICAL: Active camera forward vector contains NaN!");
            }
        } else {
            info!("[ACTIVE CAMERA] Inactive camera Entity: {:?} (not being used for rendering)", entity);
        }
    }
    
    // Summary for quick diagnosis
    if active_count == 1 {
        if let Some((entity, camera, transform)) = active_cameras.first() {
            let position = transform.translation();
            let forward = transform.forward();
            info!("[ACTIVE CAMERA] SUMMARY: Active camera {:?} at ({:.1}, {:.1}, {:.1}), facing ({:.1}, {:.1}, {:.1})",
                entity, position.x, position.y, position.z, forward.x, forward.y, forward.z);
        }
    } else if active_count == 0 {
        error!("[ACTIVE CAMERA] SUMMARY: CRITICAL - No active camera! Rendering will fail!");
    } else {
        warn!("[ACTIVE CAMERA] SUMMARY: Multiple active cameras detected - rendering may be ambiguous");
    }
    
    info!("========================================");
}

/// Camera configuration diagnostics to check camera setup
pub fn camera_configuration_diagnostics(
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
    )>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }
    
    info!("========================================");
    info!("[CAMERA CONFIG DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");
    
    for (entity, camera, transform) in cameras.iter() {
        info!("[CAMERA] Camera {:?}:", entity);
        info!("[CAMERA]   Is active: {}", camera.is_active);
        info!("[CAMERA]   Target: {:?}", camera.target);
        info!("[CAMERA]   Viewport: {:?}", camera.viewport);
        info!("[CAMERA]   Order: {:?}", camera.order);
        info!("[CAMERA]   Output mode: {:?}", camera.output_mode);

        let position = transform.translation();
        let forward = transform.forward();
        let up = transform.up();

        info!("[CAMERA]   Transform:");
        info!("[CAMERA]     Position: {:?}", position);
        info!("[CAMERA]     Forward vector: {:?}", forward);
        info!("[CAMERA]     Up vector: {:?}", up);
        
        // Check for invalid camera values
        if position.x.is_nan() || position.y.is_nan() || position.z.is_nan() {
            error!("[CAMERA]   CRITICAL: Camera position contains NaN!");
        }
        
        if forward.x.is_nan() || forward.y.is_nan() || forward.z.is_nan() {
            error!("[CAMERA]   CRITICAL: Camera forward vector contains NaN!");
        }
        
        if !camera.is_active {
            warn!("[CAMERA]   WARNING: Camera is not active!");
        }
    }
    
    info!("========================================");
}

/// Render layer diagnostics to check if entities are in correct render layers
pub fn render_layer_diagnostics(
    meshes: Query<(
        Entity,
        &GlobalTransform,
        &ViewVisibility,
    ), With<Handle<Mesh>>>,
    cameras: Query<(
        Entity,
        &Camera,
    )>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[RENDER LAYER DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");

    // Check camera count
    let camera_count = cameras.iter().count();
    info!("[RENDER LAYERS] Cameras: {}", camera_count);

    // Check mesh render layers (simplified - just checking if meshes exist)
    let mut logged = 0;
    for (entity, transform, view_vis) in meshes.iter() {
        if logged >= 5 {
            break;
        }

        let position = transform.translation();
        let is_visible = view_vis.get();

        info!("[RENDER LAYERS] Mesh entity {:?}:", entity);
        info!("[RENDER LAYERS]   Position: {:?}", position);
        info!("[RENDER LAYERS]   ViewVisibility: {}", is_visible);
        info!("[RENDER LAYERS]   ✓ Mesh exists and has basic components");

        logged += 1;
    }

    info!("[RENDER LAYERS] NOTE: Render layer checking simplified - using default layers");
    info!("========================================");
}

/// AABB validation diagnostics to check if mesh bounding boxes are valid
pub fn aabb_validation_diagnostics(
    meshes: Query<(
        Entity,
        &GlobalTransform,
        &ViewVisibility,
        Option<&Aabb>,
    ), With<Handle<Mesh>>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[AABB VALIDATION DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");

    let mut valid_aabb_count = 0;
    let mut invalid_aabb_count = 0;
    let mut no_aabb_count = 0;
    let mut logged = 0;

    for (entity, transform, view_vis, aabb) in meshes.iter() {
        if logged >= 5 {
            break;
        }

        let position = transform.translation();
        let is_visible = view_vis.get();

        info!("[AABB] Entity {:?}:", entity);
        info!("[AABB]   Position: {:?}", position);
        info!("[AABB]   ViewVisibility: {}", is_visible);

        if let Some(aabb) = aabb {
            let center = aabb.center;
            let half_extents = aabb.half_extents;

            // Check if AABB is valid (not placeholder NEG_INFINITY to INFINITY)
            let is_placeholder = center.x.is_finite() && center.y.is_finite() && center.z.is_finite()
                && half_extents.x.is_finite() && half_extents.y.is_finite() && half_extents.z.is_finite()
                && (half_extents.x > 0.0 || half_extents.y > 0.0 || half_extents.z > 0.0)
                && half_extents.x < 1_000_000.0 && half_extents.y < 1_000_000.0 && half_extents.z < 1_000_000.0;

            info!("[AABB]   Center: {:?}", center);
            info!("[AABB]   Half extents: {:?}", half_extents);

            if is_placeholder {
                valid_aabb_count += 1;
                info!("[AABB]   ✓ AABB is valid");
            } else {
                invalid_aabb_count += 1;
                warn!("[AABB]   ✗ AABB appears to be placeholder, infinite, or extremely large!");
                warn!("[AABB]     This may prevent frustum culling from working correctly");
            }
        } else {
            no_aabb_count += 1;
            warn!("[AABB]   ✗ No AABB component!");
            warn!("[AABB]     Frustum culling may not work correctly");
        }

        logged += 1;
    }

    info!("[AABB] Summary: {} valid AABBs, {} invalid AABBs, {} no AABB",
        valid_aabb_count, invalid_aabb_count, no_aabb_count);

    if invalid_aabb_count > 0 || no_aabb_count > 0 {
        warn!("[AABB] WARNING: {} meshes have invalid or missing AABBs!",
            invalid_aabb_count + no_aabb_count);
    }

    info!("========================================");
}

/// Render pipeline submission diagnostics to check if entities are being submitted
pub fn render_pipeline_diagnostics(
    meshes: Query<(
        Entity,
        &Handle<Mesh>,
        &GlobalTransform,
        &ViewVisibility,
        Option<&Handle<StandardMaterial>>,
        Option<&Handle<EffectMeshMaterial>>,
        Option<&Handle<TerrainMaterial>>,
        Option<&Handle<WaterMaterial>>,
        Option<&Handle<SkyMaterial>>,
        Option<&Handle<ParticleMaterial>>,
        Option<&Handle<DamageDigitMaterial>>,
        Option<&Handle<ObjectMaterial>>,
    ), With<Handle<Mesh>>>,
    mesh_assets: Res<Assets<Mesh>>,
    material_assets: Res<Assets<StandardMaterial>>,
    effect_material_assets: Res<Assets<EffectMeshMaterial>>,
    terrain_material_assets: Res<Assets<TerrainMaterial>>,
    water_material_assets: Res<Assets<WaterMaterial>>,
    sky_material_assets: Res<Assets<SkyMaterial>>,
    particle_material_assets: Res<Assets<ParticleMaterial>>,
    damage_digit_material_assets: Res<Assets<DamageDigitMaterial>>,
    object_material_assets: Res<Assets<ObjectMaterial>>,
    cameras: Query<&Camera>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[RENDER PIPELINE DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");

    let camera_count = cameras.iter().count();
    info!("[RENDER PIPELINE] Cameras: {}", camera_count);

    // Count entities that would be submitted to render pipeline
    let mut ready_to_render = 0;
    let mut missing_mesh = 0;
    let mut missing_material = 0;
    let mut not_visible = 0;
    let mut logged = 0;

    for (entity, mesh_handle, transform, view_vis, 
         material_handle, effect_handle, terrain_handle, water_handle, 
         sky_handle, particle_handle, damage_digit_handle, object_handle) in meshes.iter() {
        if logged >= 5 {
            break;
        }

        let position = transform.translation();
        let is_visible = view_vis.get();
        let has_mesh = mesh_assets.contains(mesh_handle);
        
        let has_material = material_handle.map_or(false, |h| material_assets.contains(h)) ||
                          effect_handle.map_or(false, |h| effect_material_assets.contains(h)) ||
                          terrain_handle.map_or(false, |h| terrain_material_assets.contains(h)) ||
                          water_handle.map_or(false, |h| water_material_assets.contains(h)) ||
                          sky_handle.map_or(false, |h| sky_material_assets.contains(h)) ||
                          particle_handle.map_or(false, |h| particle_material_assets.contains(h)) ||
                          damage_digit_handle.map_or(false, |h| damage_digit_material_assets.contains(h)) ||
                          object_handle.map_or(false, |h| object_material_assets.contains(h));

        info!("[RENDER PIPELINE] Entity {:?}:", entity);
        info!("[RENDER PIPELINE]   Position: {:?}", position);
        info!("[RENDER PIPELINE]   ViewVisibility: {}", is_visible);
        info!("[RENDER PIPELINE]   Has mesh asset: {}", has_mesh);
        info!("[RENDER PIPELINE]   Has material: {}", has_material);

        let can_render = has_mesh && has_material && is_visible;

        if can_render {
            ready_to_render += 1;
            info!("[RENDER PIPELINE]   ✓ READY TO RENDER");
        } else {
            if !has_mesh {
                missing_mesh += 1;
                warn!("[RENDER PIPELINE]   ✗ Mesh asset not loaded");
            }
            if !has_material {
                missing_material += 1;
                warn!("[RENDER PIPELINE]   ✗ Material not loaded or recognized");
            }
            if !is_visible {
                not_visible += 1;
                warn!("[RENDER PIPELINE]   ✗ Not visible (ViewVisibility=false)");
            }
        }

        logged += 1;
    }

    info!("[RENDER PIPELINE] Summary:");
    info!("[RENDER PIPELINE]   Ready to render: {}", ready_to_render);
    info!("[RENDER PIPELINE]   Missing mesh: {}", missing_mesh);
    info!("[RENDER PIPELINE]   Missing material: {}", missing_material);
    info!("[RENDER PIPELINE]   Not visible: {}", not_visible);

    if ready_to_render == 0 && meshes.iter().count() > 0 {
        error!("[RENDER PIPELINE] CRITICAL: No entities are ready to render!");
        error!("[RENDER PIPELINE]   Check mesh and material loading");
    }

    info!("========================================");
}

/// Render stage diagnostics to log number of entities in each render stage
pub fn render_stage_diagnostics(
    meshes: Query<Entity, With<Handle<Mesh>>>,
    cameras: Query<Entity, With<Camera>>,
    lights: Query<Entity, With<DirectionalLight>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[RENDER STAGE DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");

    let mesh_count = meshes.iter().count();
    let camera_count = cameras.iter().count();
    let light_count = lights.iter().count();

    info!("[RENDER STAGE] Entity counts by type:");
    info!("[RENDER STAGE]   Mesh entities: {}", mesh_count);
    info!("[RENDER STAGE]   Camera entities: {}", camera_count);
    info!("[RENDER STAGE]   DirectionalLight entities: {}", light_count);

    if camera_count == 0 {
        error!("[RENDER STAGE] CRITICAL: No cameras - nothing will render!");
    }

    if light_count == 0 {
        warn!("[RENDER STAGE] WARNING: No lights - scene may be dark");
    }

    if mesh_count > 0 && camera_count > 0 && light_count > 0 {
        info!("[RENDER STAGE] ✓ Basic rendering requirements met");
    }

    info!("========================================");
}

/// Zone entity visibility diagnostics to check if zone entity (root parent) is visible
/// This is critical because child entities inherit visibility from parent
pub fn zone_entity_visibility_diagnostics(
    world: &World,
    zones: Query<(
        Entity,
        &Zone,
        &Transform,
        &GlobalTransform,
        &ViewVisibility,
        &Visibility,
    )>,
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
    )>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[ZONE ENTITY VISIBILITY DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");
    
    // DIAGNOSTIC: Querying for Zone entities
    log::info!("[ZONE ENTITY DIAGNOSTIC] About to query for entities with Zone component");

    // Check zone entities
    let zone_count = zones.iter().count();
    info!("[ZONE ENTITY] Zone entities found: {}", zone_count);

    if zone_count == 0 {
        warn!("[ZONE ENTITY] WARNING: No zone entities found!");
        info!("========================================");
        return;
    }

    for (entity, zone, transform, global_transform, view_vis, visibility) in zones.iter() {
        let position = transform.translation;
        let global_position = global_transform.translation();
        
        // Get InheritedVisibility if it exists
        let inherited_vis = world.get::<InheritedVisibility>(entity);
        let has_no_frustum_culling = world.get::<bevy::render::view::NoFrustumCulling>(entity).is_some();
        let aabb = world.get::<Aabb>(entity);
        let has_aabb = aabb.is_some();
        let parent = world.get::<Parent>(entity);
        let render_layers = world.get::<bevy::render::view::RenderLayers>(entity);
        let has_mesh = world.get::<Handle<Mesh>>(entity).is_some();
        let has_computed_visibility = world.get::<bevy::render::view::ViewVisibility>(entity).is_some();
        let inherited_vis_comp = world.get::<InheritedVisibility>(entity);

        info!("[ZONE ENTITY] Zone entity {:?}:", entity);
        info!("[ZONE ENTITY]   Has ViewVisibility component: {}", has_computed_visibility);
        info!("[ZONE ENTITY]   Has Mesh: {}", has_mesh);
        info!("[ZONE ENTITY]   Parent: {:?}", parent.map(|p: &Parent| p.get()));
        info!("[ZONE ENTITY]   Zone ID: {}", zone.id.get());
        info!("[ZONE ENTITY]   Local Position: ({:.2}, {:.2}, {:.2})", position.x, position.y, position.z);
        info!("[ZONE ENTITY]   Global Position: ({:.2}, {:.2}, {:.2})", global_position.x, global_position.y, global_position.z);
        info!("[ZONE ENTITY]   Visibility component: {:?}", visibility);
        info!("[ZONE ENTITY]   InheritedVisibility component: {:?}", inherited_vis_comp.map(|v| v.get()));
        info!("[ZONE ENTITY]   ViewVisibility (computed): {}", view_vis.get());
        info!("[ZONE ENTITY]   Has NoFrustumCulling: {}", has_no_frustum_culling);
        info!("[ZONE ENTITY]   Has Aabb: {}", has_aabb);
        if let Some(aabb) = aabb {
            info!("[ZONE ENTITY]   Aabb: center={:?}, half_extents={:?}", aabb.center, aabb.half_extents);
        }
        info!("[ZONE ENTITY]   RenderLayers: {:?}", render_layers);

        if !view_vis.get() {
            error!("[ZONE ENTITY]   CRITICAL: Zone entity ViewVisibility is FALSE!");
            error!("[ZONE ENTITY]   This will cause ALL child entities to be invisible!");
            error!("[ZONE ENTITY]   Child entities will inherit this visibility!");
        } else {
            info!("[ZONE ENTITY]   ✓ Zone entity is visible");
        }
    }

    // Check camera positions
    info!("[ZONE ENTITY] Camera information:");
    for (cam_entity, camera, cam_transform) in cameras.iter() {
        let cam_pos = cam_transform.translation();
        let cam_forward = cam_transform.forward();
        let cam_render_layers = world.get::<bevy::render::view::RenderLayers>(cam_entity);

        info!("[ZONE ENTITY]   Camera {:?}:", cam_entity);
        info!("[ZONE ENTITY]     Position: ({:.2}, {:.2}, {:.2})", cam_pos.x, cam_pos.y, cam_pos.z);
        info!("[ZONE ENTITY]     Forward: ({:.2}, {:.2}, {:.2})", cam_forward.x, cam_forward.y, cam_forward.z);
        info!("[ZONE ENTITY]     Is active: {}", camera.is_active);
        info!("[ZONE ENTITY]     RenderLayers: {:?}", cam_render_layers);

        // Check distance from camera to zone entities
        for (_, _, _, zone_global_transform, _, _) in zones.iter() {
            let zone_pos = zone_global_transform.translation();
            let distance = cam_pos.distance(zone_pos);
            info!("[ZONE ENTITY]     Distance to zone: {:.2}", distance);
        }
    }

    info!("========================================");
}

/// Parent-child visibility propagation diagnostics
/// Check if child entities are inheriting visibility from parent correctly
pub fn parent_child_visibility_diagnostics(
    world: &World,
    zones: Query<(
        Entity,
        &Zone,
        &ViewVisibility,
    )>,
    children: Query<(
        Entity,
        &Parent,
        &ViewVisibility,
        &Visibility,
        &InheritedVisibility,
        Option<&Handle<Mesh>>,
    )>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[PARENT-CHILD VISIBILITY DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");

    for (zone_entity, zone, zone_view_vis) in zones.iter() {
        info!("[PARENT-CHILD] Zone entity {:?} (ID: {}):", zone_entity, zone.id.get());
        info!("[PARENT-CHILD]   Zone ViewVisibility: {}", zone_view_vis.get());

        // Count children
        let mut child_count = 0;
        let mut visible_children = 0;
        let mut invisible_children = 0;

        for (child_entity, parent, child_view_vis, child_visibility, child_inherited_vis, mesh_handle) in children.iter() {
            // Check if this child is a child of zone
            if parent.get() == zone_entity {
                child_count += 1;

                if child_view_vis.get() {
                    visible_children += 1;
                } else {
                    invisible_children += 1;
                }

                // Log first 10 children
                if child_count <= 10 {
                    let visibility_str = match child_visibility {
                        Visibility::Visible => "Visible",
                        Visibility::Hidden => "Hidden",
                        Visibility::Inherited => "Inherited",
                    };
                    
                    let child_render_layers = world.get::<bevy::render::view::RenderLayers>(child_entity);
                    let child_has_aabb = world.get::<Aabb>(child_entity).is_some();
                    let child_has_no_frustum_culling = world.get::<bevy::render::view::NoFrustumCulling>(child_entity).is_some();
                    let child_has_computed_visibility = world.get::<bevy::render::view::ViewVisibility>(child_entity).is_some();
                    let child_inherited_vis_comp = world.get::<InheritedVisibility>(child_entity);

                    info!("[PARENT-CHILD]   Child {:?} (Mesh: {}):", child_entity, mesh_handle.is_some());
                    info!("[PARENT-CHILD]     Has ViewVisibility component: {}", child_has_computed_visibility);
                    info!("[PARENT-CHILD]     Visibility component: {}", visibility_str);
                    info!("[PARENT-CHILD]     InheritedVisibility component: {:?}", child_inherited_vis_comp.map(|v| v.get()));
                    info!("[PARENT-CHILD]     ViewVisibility: {}", child_view_vis.get());
                    info!("[PARENT-CHILD]     RenderLayers: {:?}", child_render_layers);
                    info!("[PARENT-CHILD]     Has Aabb: {}, NoFrustumCulling: {}", child_has_aabb, child_has_no_frustum_culling);

                    if !child_view_vis.get() && *child_visibility == Visibility::Visible {
                        warn!("[PARENT-CHILD]     Child has Visibility::Visible but ViewVisibility=false!");
                        warn!("[PARENT-CHILD]     This indicates parent visibility issue or propagation failure!");
                    }
                }
            }
        }

        info!("[PARENT-CHILD]   Total children: {}", child_count);
        info!("[PARENT-CHILD]   Visible children: {}", visible_children);
        info!("[PARENT-CHILD]   Invisible children: {}", invisible_children);

        if invisible_children > 0 && !zone_view_vis.get() {
            error!("[PARENT-CHILD]   DIAGNOSIS: Zone entity is invisible, causing {} children to be invisible!",
                invisible_children);
            error!("[PARENT-CHILD]   FIX: Make zone entity visible!");
        }
    }

    info!("========================================");
}

/// Comprehensive zone component diagnostics to check Zone component presence and lifecycle
/// This system helps diagnose why Zone component query returns 0 results
pub fn zone_component_lifecycle_diagnostics(
    world: &World,
    all_entities: Query<Entity, Without<Camera>>,
    zone_entities: Query<(Entity, &Zone)>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only run every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[ZONE COMPONENT LIFECYCLE] Frame {}", *frame_count);
    info!("========================================");

    // Count all entities (excluding cameras)
    let total_entities = all_entities.iter().count();
    info!("[ZONE COMPONENT] Total entities (excluding cameras): {}", total_entities);

    // Query for entities with Zone component
    let zone_count = zone_entities.iter().count();
    info!("[ZONE COMPONENT] Entities with Zone component: {}", zone_count);

    if zone_count > 0 {
        // Log each zone entity
        for (entity, zone) in zone_entities.iter() {
            info!("[ZONE COMPONENT] ✓ Zone entity found: {:?}, zone_id={}", entity, zone.id.get());
        }
    } else {
        warn!("[ZONE COMPONENT] ✗ NO entities with Zone component found!");
        warn!("[ZONE COMPONENT] This explains why zone entity query returns 0 results");
    }

    // Check for each required component individually using world.get()
    let mut entities_with_transform = 0;
    let mut entities_with_global_transform = 0;
    let mut entities_with_view_visibility = 0;
    let mut entities_with_visibility = 0;
    let mut zone_full_count = 0;

    for (entity, _) in zone_entities.iter() {
        let has_transform = world.get::<Transform>(entity).is_some();
        let has_global_transform = world.get::<GlobalTransform>(entity).is_some();
        let has_view_visibility = world.get::<ViewVisibility>(entity).is_some();
        let has_visibility = world.get::<Visibility>(entity).is_some();

        if has_transform { entities_with_transform += 1; }
        if has_global_transform { entities_with_global_transform += 1; }
        if has_view_visibility { entities_with_view_visibility += 1; }
        if has_visibility { entities_with_visibility += 1; }

        if has_transform && has_global_transform && has_view_visibility && has_visibility {
            zone_full_count += 1;
        }
    }

    info!("[ZONE COMPONENT] Zone entities with Transform: {}", entities_with_transform);
    info!("[ZONE COMPONENT] Zone entities with GlobalTransform: {}", entities_with_global_transform);
    info!("[ZONE COMPONENT] Zone entities with ViewVisibility: {}", entities_with_view_visibility);
    info!("[ZONE COMPONENT] Zone entities with Visibility: {}", entities_with_visibility);
    info!("[ZONE COMPONENT] Entities with Zone + ALL required components: {}", zone_full_count);

    if zone_count > 0 && zone_full_count == 0 {
        error!("[ZONE COMPONENT] ✗ CRITICAL: Zone entities exist but missing required components!");
        error!("[ZONE COMPONENT] Required: Zone, Transform, GlobalTransform, ViewVisibility, Visibility");

        // Check which components are missing for each entity
        for (entity, zone) in zone_entities.iter() {
            let has_transform = world.get::<Transform>(entity).is_some();
            let has_global_transform = world.get::<GlobalTransform>(entity).is_some();
            let has_view_visibility = world.get::<ViewVisibility>(entity).is_some();
            let has_visibility = world.get::<Visibility>(entity).is_some();

            info!("[ZONE COMPONENT] Zone entity {:?} component check:", entity);
            info!("[ZONE COMPONENT]   Zone: ✓");
            info!("[ZONE COMPONENT]   Transform: {}", if has_transform { "✓" } else { "✗" });
            info!("[ZONE COMPONENT]   GlobalTransform: {}", if has_global_transform { "✓" } else { "✗" });
            info!("[ZONE COMPONENT]   ViewVisibility: {}", if has_view_visibility { "✓" } else { "✗" });
            info!("[ZONE COMPONENT]   Visibility: {}", if has_visibility { "✓" } else { "✗" });

            if !has_transform || !has_global_transform || !has_view_visibility || !has_visibility {
                error!("[ZONE COMPONENT]   MISSING COMPONENT(S):");
                if !has_transform {
                    error!("[ZONE COMPONENT]     - Transform");
                }
                if !has_global_transform {
                    error!("[ZONE COMPONENT]     - GlobalTransform");
                }
                if !has_view_visibility {
                    error!("[ZONE COMPONENT]     - ViewVisibility");
                }
                if !has_visibility {
                    error!("[ZONE COMPONENT]     - Visibility");
                }
            }
        }
    } else if zone_count > 0 && zone_full_count > 0 {
        info!("[ZONE COMPONENT] ✓ All zone entities have all required components");

        for (entity, zone) in zone_entities.iter() {
            info!("[ZONE COMPONENT] Zone entity {:?} has all components:", entity);
            info!("[ZONE COMPONENT]   Zone ID: {}", zone.id.get());
            if let Some(transform) = world.get::<Transform>(entity) {
                info!("[ZONE COMPONENT]   Transform: {:?}", transform.translation);
            }
            if let Some(global_transform) = world.get::<GlobalTransform>(entity) {
                info!("[ZONE COMPONENT]   GlobalTransform: {:?}", global_transform.translation());
            }
            if let Some(view_vis) = world.get::<ViewVisibility>(entity) {
                info!("[ZONE COMPONENT]   ViewVisibility: {}", view_vis.get());
            }
            if let Some(visibility) = world.get::<Visibility>(entity) {
                info!("[ZONE COMPONENT]   Visibility: {:?}", visibility);
            }
        }
    }

    info!("========================================");
}
