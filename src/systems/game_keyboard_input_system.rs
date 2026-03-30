use bevy::{
    input::ButtonInput,
    math::{Vec2, Vec3},
    prelude::{
        Camera3d, KeyCode, Local, MessageWriter, Query, Res, State, Time, Transform, With,
    },
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_egui::EguiContexts;
use rose_game_common::components::MoveSpeed;

use crate::{
    components::{FlightState, PlayerCharacter, Position},
    events::PlayerCommandEvent,
    resources::AppState,
};

const WASD_MOVE_COMMAND_INTERVAL_SECS: f32 = 0.10;
const WASD_MOVE_COMMAND_LEAD_TIME_SECS: f32 = 0.25;

/// Keyboard movement input (W/A/S/D) for player character movement.
///
/// This sends periodic `PlayerCommandEvent::Move` commands while movement keys are held,
/// using camera-relative movement on the ground plane.
pub fn game_keyboard_input_system(
    app_state: Res<State<AppState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query_window: Query<&CursorOptions, With<PrimaryWindow>>,
    query_camera: Query<&Transform, With<Camera3d>>,
    query_player: Query<(&Position, &MoveSpeed, Option<&FlightState>), With<PlayerCharacter>>,
    mut egui_ctx: EguiContexts,
    time: Res<Time>,
    mut move_command_cooldown: Local<f32>,
    mut last_move_direction: Local<Option<Vec2>>,
    mut player_command_events: MessageWriter<PlayerCommandEvent>,
) {
    if *app_state.get() != AppState::Game {
        return;
    }

    if egui_ctx.ctx_mut().unwrap().wants_keyboard_input() {
        return;
    }

    let Ok(cursor_options) = query_window.single() else {
        return;
    };

    if !matches!(cursor_options.grab_mode, CursorGrabMode::None) {
        // Cursor is currently grabbed
        return;
    }

    let Ok(camera_transform) = query_camera.single() else {
        return;
    };

    let Ok((player_position, move_speed, player_flight_state)) = query_player.single() else {
        return;
    };

    // Disable WASD ground movement while flying; flight_movement_system handles flight controls.
    if player_flight_state.map_or(false, |flight_state| flight_state.is_flying) {
        return;
    }

    let camera_rotation = camera_transform.rotation;

    // Build camera-relative movement vectors on the ground plane.
    let camera_forward = (camera_rotation * -Vec3::Z).with_y(0.0).normalize_or_zero();
    let camera_right = (camera_rotation * Vec3::X).with_y(0.0).normalize_or_zero();

    let mut move_world = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        move_world += camera_forward;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        move_world -= camera_forward;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        move_world -= camera_right;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        move_world += camera_right;
    }

    if move_world.length_squared() == 0.0 {
        *last_move_direction = None;
        *move_command_cooldown = 0.0;
        return;
    }

    let move_world = move_world.normalize();

    // Convert world direction (x/right, z/back) to game Position direction (x/right, y/forward).
    let move_direction = Vec2::new(move_world.x, -move_world.z).normalize_or_zero();

    if move_direction == Vec2::ZERO {
        return;
    }

    *move_command_cooldown -= time.delta_secs();

    let started_moving = keyboard_input.just_pressed(KeyCode::KeyW)
        || keyboard_input.just_pressed(KeyCode::KeyA)
        || keyboard_input.just_pressed(KeyCode::KeyS)
        || keyboard_input.just_pressed(KeyCode::KeyD);

    let direction_changed = last_move_direction
        .map_or(true, |last_direction| last_direction.dot(move_direction) < 0.999);

    if *move_command_cooldown <= 0.0 || started_moving || direction_changed {
        let lead_distance = move_speed.speed * WASD_MOVE_COMMAND_LEAD_TIME_SECS;
        let destination = Position::new(Vec3::new(
            player_position.x + move_direction.x * lead_distance,
            player_position.y + move_direction.y * lead_distance,
            player_position.z,
        ));

        player_command_events.write(PlayerCommandEvent::Move(destination, None));

        *move_command_cooldown = WASD_MOVE_COMMAND_INTERVAL_SECS;
        *last_move_direction = Some(move_direction);
    }
}
