use bevy::{
    log::info,
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, Query, ResMut, With},
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
    // Reset camera
    let camera_position = Vec3::new(5120.0, 50.0, -5120.0);
    let camera_yaw: f32 = 135.0;
    let camera_pitch: f32 = -20.0;

    info!("[CAMERA FIX] Initializing zone viewer camera");
    info!("[CAMERA FIX] Camera position: {:?}", camera_position);
    info!("[CAMERA FIX] Camera yaw: {} degrees (looking towards negative Z)", camera_yaw);
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

    // Open relevant debug windows
    ui_state_debug_windows.camera_info_open = true;
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}
