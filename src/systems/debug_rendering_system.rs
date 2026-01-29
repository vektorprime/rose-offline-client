use bevy::prelude::*;
use bevy::render::view::Visibility;
use bevy::render::camera::Camera;
use crate::components::ZoneObject;

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

    info!("[DEBUG] Entity visibility stats:");
    info!("[DEBUG]   Total mesh entities: {}", total_entities);
    info!("[DEBUG]   Visible entities: {}", visible_count);
    info!("[DEBUG]   Hidden entities: {}", total_entities - visible_count);

    // Log first 5 visible entities
    let mut visible_logged = 0;
    for (entity, transform, view_vis, visibility, name) in query.iter() {
        if view_vis.get() && visible_logged < 5 {
            let name_str = name.map(|n| n.as_str()).unwrap_or("<unnamed>");
            info!("[DEBUG]   Visible entity '{}': pos=({:.1}, {:.1}, {:.1})",
                  name_str, transform.translation.x, transform.translation.y, transform.translation.z);
            visible_logged += 1;
        }
    }

    // Log first 5 hidden entities
    let mut hidden_logged = 0;
    for (entity, transform, view_vis, visibility, name) in query.iter() {
        if !view_vis.get() && hidden_logged < 5 {
            let name_str = name.map(|n| n.as_str()).unwrap_or("<unnamed>");
            info!("[DEBUG]   Hidden entity '{}': pos=({:.1}, {:.1}, {:.1})",
                  name_str, transform.translation.x, transform.translation.y, transform.translation.z);
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
        Option<&Camera3d>,
    )>,
    meshes: Query<(
        Entity,
        &Handle<Mesh>,
        &GlobalTransform,
        &ViewVisibility,
        Option<&Handle<StandardMaterial>>,
    )>,
    mesh_assets: Res<Assets<Mesh>>,
    material_assets: Res<Assets<StandardMaterial>>,
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
        for (entity, camera, transform, camera_3d) in cameras.iter() {
            info!("[RENDER DIAGNOSTICS] Camera {:?}:", entity);
            info!("[RENDER DIAGNOSTICS]   Position: {:?}", transform.translation());
            info!("[RENDER DIAGNOSTICS]   Is 3D: {}", camera_3d.is_some());
            info!("[RENDER DIAGNOSTICS]   Is active: {}", camera.is_active);
            info!("[RENDER DIAGNOSTICS]   Target: {:?}", camera.target);
            
            if !camera.is_active {
                warn!("[RENDER DIAGNOSTICS] WARNING: Camera is not active!");
            }
        }
    }
    
    // Check mesh entities
    let mesh_entity_count = meshes.iter().count();
    info!("[RENDER DIAGNOSTICS] Mesh entities: {}", mesh_entity_count);
    
    let visible_meshes = meshes.iter()
        .filter(|(_, _, _, vis, _)| vis.get())
        .count();
    info!("[RENDER DIAGNOSTICS] Visible mesh entities: {}", visible_meshes);
    
    if mesh_entity_count == 0 {
        warn!("[RENDER DIAGNOSTICS] WARNING: No mesh entities in scene!");
    }
    
    if visible_meshes == 0 && mesh_entity_count > 0 {
        warn!("[RENDER DIAGNOSTICS] WARNING: Mesh entities exist but none are visible!");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes are in camera frustum");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes have Visibility::Visible");
        warn!("[RENDER DIAGNOSTICS]   - Check if meshes are behind the camera");
    }
    
    // Check first few mesh entities
    let mut logged = 0;
    for (entity, mesh_handle, transform, visibility, material) in meshes.iter() {
        if logged < 3 {
            info!("[RENDER DIAGNOSTICS] Mesh entity {:?}:", entity);
            info!("[RENDER DIAGNOSTICS]   Position: {:?}", transform.translation());
            info!("[RENDER DIAGNOSTICS]   Visible: {}", visibility.get());
            info!("[RENDER DIAGNOSTICS]   Has mesh asset: {}", mesh_assets.contains(mesh_handle));
            info!("[RENDER DIAGNOSTICS]   Has material: {}", material.is_some());
            logged += 1;
        }
    }
    
    // Check asset counts
    info!("[RENDER DIAGNOSTICS] Asset counts:");
    info!("[RENDER DIAGNOSTICS]   Meshes: {}", mesh_assets.len());
    info!("[RENDER DIAGNOSTICS]   Materials: {}", material_assets.len());
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
        warn!("[RENDER DIAGNOSTICS]   - Mesh transforms may be incorrect");
    }
    
    if camera_count > 0 && mesh_entity_count == 0 {
        warn!("[RENDER DIAGNOSTICS] POSSIBLE CAUSE: Camera exists but no mesh entities");
        warn!("[RENDER DIAGNOSTICS]   - Zone may not be loading properly");
        warn!("[RENDER DIAGNOSTICS]   - Entities may be despawned");
    }
    
    info!("========================================");
}

/// Lightweight render diagnostics that runs every frame without being too verbose
pub fn render_diagnostics_system_lightweight(
    cameras: Query<(Entity, &Camera, &GlobalTransform), With<Camera3d>>,
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
