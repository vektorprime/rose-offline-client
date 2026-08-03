use bevy::{
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, Query, ResMut, With, Without},
};

use crate::{
    animation::CameraAnimation,
    systems::{FreeCamera, OrbitCamera},
    ui::UiStateDebugWindows,
};

pub fn zone_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<
        Entity,
        (With<Camera3d>, Without<crate::render::WaterReflectionCamera>),
    >,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    // Reset camera to optimal zone viewing position
    // Zone center is approximately (5200.0, 0.0, -5200.0)
    let camera_position = Vec3::new(5120.0, 50.0, -5120.0);
    let camera_yaw: f32 = -45.0;
    let camera_pitch: f32 = -20.0;

    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<OrbitCamera>()
            .remove::<CameraAnimation>()
            .insert(FreeCamera::new(camera_position, camera_yaw, camera_pitch));
    }

    // Open relevant debug windows
    ui_state_debug_windows.camera_info_open = true;
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}
