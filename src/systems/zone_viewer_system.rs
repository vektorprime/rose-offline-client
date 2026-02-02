use bevy::{
    log::info,
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, Query, ResMut, With, GlobalTransform, Transform},
};

use crate::{
    animation::CameraAnimation,
    systems::{FreeCamera, OrbitCamera},
    ui::UiStateDebugWindows,
};

pub fn zone_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    // Reset camera to optimal zone viewing position
    // Zone center is approximately (5200.0, 0.0, -5200.0)
    let camera_position = Vec3::new(5000.0, 150.0, -5000.0);
    let camera_yaw: f32 = -45.0;
    let camera_pitch: f32 = -30.0;  // Slightly steeper angle to see terrain

    info!("[CAMERA FIX] Initializing zone viewer camera for Bevy 0.14");
    info!("[CAMERA FIX] Camera position: {:?}", camera_position);
    info!("[CAMERA FIX] Looking toward zone center: (5200.0, 0.0, -5200.0)");
    info!("[CAMERA FIX] Camera yaw: {} degrees", camera_yaw);
    info!("[CAMERA FIX] Camera pitch: {} degrees", camera_pitch);

    // Calculate forward vector for verification
    let yaw_rad = f32::to_radians(camera_yaw);
    let pitch_rad = f32::to_radians(camera_pitch);
    let forward = Vec3::new(
        yaw_rad.sin() * pitch_rad.cos(),
        pitch_rad.sin(),
        yaw_rad.cos() * pitch_rad.cos(),
    );
    info!("[CAMERA FIX] Camera forward vector: {:?}", forward);
    info!("[CAMERA FIX] Camera looking towards: Z = {} (negative Z direction)", if forward.z < 0.0 { "NEGATIVE" } else { "POSITIVE" });

    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<OrbitCamera>()
            .remove::<CameraAnimation>()
            .insert(FreeCamera::new(
                camera_position,
                camera_yaw,
                camera_pitch,
            ));
    }

    info!("[CAMERA FIX] Zone viewer camera initialized successfully");

    // Enhanced camera logging
    info!("[CAMERA] Zone viewer camera initialized:");
    info!("[CAMERA]   Position: ({}, {}, {})", camera_position.x, camera_position.y, camera_position.z);
    info!("[CAMERA]   Yaw: {} degrees", camera_yaw);
    info!("[CAMERA]   Pitch: {} degrees", camera_pitch);
    info!("[CAMERA]   Looking direction: {}", calculate_look_direction(camera_yaw, camera_pitch));

    // Open relevant debug windows
    ui_state_debug_windows.camera_info_open = true;
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}

fn calculate_look_direction(yaw: f32, pitch: f32) -> String {
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();

    let x = yaw_rad.cos() * pitch_rad.cos();
    let y = pitch_rad.sin();
    let z = yaw_rad.sin() * pitch_rad.cos();

    format!("({:.2}, {:.2}, {:.2})", x, y, z)
}

/// Diagnostic system to log camera state every frame for debugging black screen issues
pub fn debug_camera_render_state_system(
    query_cameras: Query<(Entity, &Transform, &GlobalTransform, Option<&FreeCamera>), With<Camera3d>>,
) {
    for (entity, transform, global_transform, free_cam) in query_cameras.iter() {
        let translation = global_transform.translation();
        let forward = global_transform.forward();
        
        log::info!("[RENDER DEBUG] Camera Entity: {:?}", entity);
        log::info!("[RENDER DEBUG]   Local Position: {:.2}, {:.2}, {:.2}", 
            transform.translation.x, transform.translation.y, transform.translation.z);
        log::info!("[RENDER DEBUG]   Global Position: {:.2}, {:.2}, {:.2}", 
            translation.x, translation.y, translation.z);
        log::info!("[RENDER DEBUG]   Forward Direction: {:.2}, {:.2}, {:.2}", 
            forward.x, forward.y, forward.z);
        
        if let Some(_cam) = free_cam {
            log::info!("[RENDER DEBUG]   FreeCamera component present");
        }
        
        // Check if camera is at origin (common black screen cause)
        if translation.length() < 0.01 {
            log::warn!("[RENDER DEBUG] WARNING: Camera is at or near origin! This may cause black screen.");
        }
        
        // Check if camera is looking at origin
        let to_origin = -translation;
        let alignment = forward.dot(to_origin.normalize());
        if alignment > 0.9 {
            log::info!("[RENDER DEBUG] Camera is looking toward origin (alignment: {:.2})", alignment);
        }
        
        log::info!("[RENDER DEBUG] ===========================================");
    }
}
