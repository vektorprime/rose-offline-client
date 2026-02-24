//! Transform Gizmo System for the Map Editor
//! 
//! Handles transform manipulation when dragging gizmos and updates entity transforms
//! based on editor mode (Translate/Rotate/Scale).

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map_editor::components::{EditorGizmo, GizmoType, SelectedInEditor};
use crate::map_editor::resources::{EditorAction, EditorMode, MapEditorState};

/// Resource to track active gizmo drag state
#[derive(Resource, Default)]
pub struct GizmoDragState {
    /// Whether we're currently dragging a gizmo
    pub is_dragging: bool,
    
    /// The axis being dragged (for translate/scale)
    pub active_axis: Option<GizmoAxis>,
    
    /// The original transform when drag started
    pub original_transform: Option<Transform>,
    
    /// The entity being dragged
    pub dragged_entity: Option<Entity>,
    
    /// Mouse position when drag started
    pub drag_start_mouse_pos: Option<Vec2>,
}

/// Axis for gizmo manipulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    Free,
}

impl Default for GizmoAxis {
    fn default() -> Self {
        Self::Free
    }
}

/// System to handle transform gizmo manipulation
pub fn transform_gizmo_system(
    mut commands: Commands,
    mut map_editor_state: ResMut<MapEditorState>,
    mut gizmo_drag_state: ResMut<GizmoDragState>,
    mut selected_transforms: Query<&mut Transform, With<SelectedInEditor>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut egui_contexts: EguiContexts,
    cameras: Query<&Camera>,
    camera_transforms: Query<&GlobalTransform>,
    windows: Query<&Window>,
) {
    // Don't process if editor is disabled or egui is capturing input
    if !map_editor_state.enabled {
        return;
    }
    
    // Check if egui wants the input
    let ctx = egui_contexts.ctx_mut();
    if ctx.wants_pointer_input() || ctx.wants_keyboard_input() {
        return;
    }
    
    // Get the primary window
    let Ok(window) = windows.get_single() else {
        return;
    };
    
    // Handle keyboard shortcuts for switching modes
    handle_mode_switches(&mut map_editor_state, &keyboard);
    
    // Handle snap-to-grid toggle
    if keyboard.just_pressed(KeyCode::KeyG) && keyboard.pressed(KeyCode::ControlLeft) {
        map_editor_state.snap_to_grid = !map_editor_state.snap_to_grid;
        log::info!(
            "[Gizmo] Snap to grid: {}",
            map_editor_state.snap_to_grid
        );
    }
    
    // Only process transform operations in transform modes
    let editor_mode = map_editor_state.editor_mode;
    if !matches!(editor_mode, EditorMode::Translate | EditorMode::Rotate | EditorMode::Scale) {
        return;
    }
    
    // Get mouse position
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    
    // Handle drag start
    if mouse.just_pressed(MouseButton::Left) && !gizmo_drag_state.is_dragging {
        // Store original transforms for all selected entities
        if map_editor_state.selection_count() > 0 {
            gizmo_drag_state.is_dragging = true;
            gizmo_drag_state.drag_start_mouse_pos = Some(mouse_pos);
            
            // Get the first selected entity for tracking
            if let Some(first_entity) = map_editor_state.first_selected() {
                gizmo_drag_state.dragged_entity = Some(first_entity);
                
                // Store original transform
                if let Ok(transform) = selected_transforms.get(first_entity) {
                    gizmo_drag_state.original_transform = Some(*transform);
                }
            }
        }
    }
    
    // Handle drag end
    if mouse.just_released(MouseButton::Left) && gizmo_drag_state.is_dragging {
        // Record the action for undo
        if let (Some(entity), Some(original)) = (
            gizmo_drag_state.dragged_entity,
            gizmo_drag_state.original_transform,
        ) {
            if let Ok(current_transform) = selected_transforms.get(entity) {
                // Only record if transform actually changed
                if *current_transform != original {
                    map_editor_state.push_action(EditorAction::TransformEntity {
                        entity,
                        old_transform: original,
                        new_transform: *current_transform,
                    });
                }
            }
        }
        
        // Reset drag state
        *gizmo_drag_state = GizmoDragState::default();
    }
    
    // Handle dragging
    if gizmo_drag_state.is_dragging {
        let Some(drag_start) = gizmo_drag_state.drag_start_mouse_pos else {
            return;
        };
        
        let delta_mouse = mouse_pos - drag_start;
        
        // Calculate transform delta based on editor mode
        match editor_mode {
            EditorMode::Translate => {
                apply_translation(
                    &mut selected_transforms,
                    &map_editor_state,
                    delta_mouse,
                    gizmo_drag_state.active_axis,
                );
            }
            EditorMode::Rotate => {
                apply_rotation(
                    &mut selected_transforms,
                    &map_editor_state,
                    delta_mouse,
                    gizmo_drag_state.active_axis,
                );
            }
            EditorMode::Scale => {
                apply_scale(
                    &mut selected_transforms,
                    &map_editor_state,
                    delta_mouse,
                    gizmo_drag_state.active_axis,
                );
            }
            _ => {}
        }
    }
}

/// Handle keyboard shortcuts for switching editor modes
fn handle_mode_switches(map_editor_state: &mut MapEditorState, keyboard: &ButtonInput<KeyCode>) {
    // W for Translate mode
    if keyboard.just_pressed(KeyCode::KeyW) {
        map_editor_state.editor_mode = EditorMode::Translate;
        log::info!("[Gizmo] Switched to Translate mode");
    }
    
    // E for Rotate mode
    if keyboard.just_pressed(KeyCode::KeyE) {
        map_editor_state.editor_mode = EditorMode::Rotate;
        log::info!("[Gizmo] Switched to Rotate mode");
    }
    
    // R for Scale mode
    if keyboard.just_pressed(KeyCode::KeyR) {
        map_editor_state.editor_mode = EditorMode::Scale;
        log::info!("[Gizmo] Switched to Scale mode");
    }
    
    // Q for Select mode
    if keyboard.just_pressed(KeyCode::KeyQ) {
        map_editor_state.editor_mode = EditorMode::Select;
        log::info!("[Gizmo] Switched to Select mode");
    }
}

/// Apply translation to all selected entities
fn apply_translation(
    transforms: &mut Query<&mut Transform, With<SelectedInEditor>>,
    map_editor_state: &MapEditorState,
    delta_mouse: Vec2,
    active_axis: Option<GizmoAxis>,
) {
    // Convert mouse delta to world units (simplified - assumes orthographic-like behavior)
    let move_speed = 0.01; // Units per pixel of mouse movement
    
    let axis = active_axis.unwrap_or(GizmoAxis::Free);
    
    let mut delta = Vec3::ZERO;
    match axis {
        GizmoAxis::X => delta.x = delta_mouse.x * move_speed,
        GizmoAxis::Y => delta.y = -delta_mouse.y * move_speed, // Y is inverted in screen space
        GizmoAxis::Z => delta.z = delta_mouse.x * move_speed,
        GizmoAxis::XY => {
            delta.x = delta_mouse.x * move_speed;
            delta.y = -delta_mouse.y * move_speed;
        }
        GizmoAxis::XZ => {
            delta.x = delta_mouse.x * move_speed;
            delta.z = delta_mouse.y * move_speed;
        }
        GizmoAxis::YZ => {
            delta.y = -delta_mouse.y * move_speed;
            delta.z = delta_mouse.x * move_speed;
        }
        GizmoAxis::Free => {
            // Free movement in XZ plane by default
            delta.x = delta_mouse.x * move_speed;
            delta.z = delta_mouse.y * move_speed;
        }
    }
    
    // Apply snap-to-grid if enabled
    if map_editor_state.snap_to_grid {
        delta = snap_to_grid(delta, map_editor_state.grid_size);
    }
    
    // Apply to all selected entities
    for mut transform in transforms.iter_mut() {
        transform.translation += delta;
    }
}

/// Apply rotation to all selected entities
fn apply_rotation(
    transforms: &mut Query<&mut Transform, With<SelectedInEditor>>,
    map_editor_state: &MapEditorState,
    delta_mouse: Vec2,
    active_axis: Option<GizmoAxis>,
) {
    let rotate_speed = 0.5; // Degrees per pixel of mouse movement
    
    let axis = active_axis.unwrap_or(GizmoAxis::Y); // Default to Y axis rotation
    
    let mut euler_delta = Vec3::ZERO;
    match axis {
        GizmoAxis::X => euler_delta.x = delta_mouse.y * rotate_speed,
        GizmoAxis::Y => euler_delta.y = delta_mouse.x * rotate_speed,
        GizmoAxis::Z => euler_delta.z = delta_mouse.x * rotate_speed,
        _ => {
            // Free rotation defaults to Y axis
            euler_delta.y = delta_mouse.x * rotate_speed;
        }
    }
    
    // Apply snap-to-grid for rotation (snap to 15 degree increments)
    let rotation_snap: f32 = if map_editor_state.snap_to_grid { 15.0 } else { 0.0 };
    
    for mut transform in transforms.iter_mut() {
        let current_euler = transform.rotation.to_euler(EulerRot::XYZ);
        
        let mut new_euler = (
            current_euler.0 + euler_delta.x.to_radians(),
            current_euler.1 + euler_delta.y.to_radians(),
            current_euler.2 + euler_delta.z.to_radians(),
        );
        
        // Apply rotation snapping
        if rotation_snap > 0.0 {
            let snap_rad = rotation_snap.to_radians();
            new_euler.0 = (new_euler.0 / snap_rad).round() * snap_rad;
            new_euler.1 = (new_euler.1 / snap_rad).round() * snap_rad;
            new_euler.2 = (new_euler.2 / snap_rad).round() * snap_rad;
        }
        
        transform.rotation = Quat::from_euler(EulerRot::XYZ, new_euler.0, new_euler.1, new_euler.2);
    }
}

/// Apply scale to all selected entities
fn apply_scale(
    transforms: &mut Query<&mut Transform, With<SelectedInEditor>>,
    map_editor_state: &MapEditorState,
    delta_mouse: Vec2,
    active_axis: Option<GizmoAxis>,
) {
    let scale_speed = 0.005; // Scale factor per pixel of mouse movement
    
    let axis = active_axis.unwrap_or(GizmoAxis::Free);
    
    // Scale based on horizontal mouse movement
    let scale_delta = 1.0 + (delta_mouse.x * scale_speed);
    
    for mut transform in transforms.iter_mut() {
        let mut new_scale = transform.scale;
        
        match axis {
            GizmoAxis::X => new_scale.x *= scale_delta,
            GizmoAxis::Y => new_scale.y *= scale_delta,
            GizmoAxis::Z => new_scale.z *= scale_delta,
            GizmoAxis::XY => {
                new_scale.x *= scale_delta;
                new_scale.y *= scale_delta;
            }
            GizmoAxis::XZ => {
                new_scale.x *= scale_delta;
                new_scale.z *= scale_delta;
            }
            GizmoAxis::YZ => {
                new_scale.y *= scale_delta;
                new_scale.z *= scale_delta;
            }
            GizmoAxis::Free => {
                // Uniform scaling
                new_scale *= scale_delta;
            }
        }
        
        // Apply snap-to-grid for scale (snap to 0.1 increments)
        if map_editor_state.snap_to_grid {
            let snap = 0.1;
            new_scale.x = (new_scale.x / snap).round() * snap;
            new_scale.y = (new_scale.y / snap).round() * snap;
            new_scale.z = (new_scale.z / snap).round() * snap;
            
            // Clamp minimum scale
            new_scale = new_scale.max(Vec3::splat(0.1));
        }
        
        transform.scale = new_scale;
    }
}

/// Snap a vector to grid increments
fn snap_to_grid(value: Vec3, grid_size: f32) -> Vec3 {
    Vec3::new(
        (value.x / grid_size).round() * grid_size,
        (value.y / grid_size).round() * grid_size,
        (value.z / grid_size).round() * grid_size,
    )
}

/// System to draw gizmo visuals for the selected entity
pub fn draw_gizmo_visuals(
    mut gizmos: Gizmos,
    map_editor_state: Res<MapEditorState>,
    selected_transforms: Query<&Transform, With<SelectedInEditor>>,
) {
    if !map_editor_state.enabled {
        return;
    }
    
    // Only draw gizmos in transform modes
    if !matches!(
        map_editor_state.editor_mode,
        EditorMode::Translate | EditorMode::Rotate | EditorMode::Scale
    ) {
        return;
    }
    
    // Draw gizmo for each selected entity
    for transform in selected_transforms.iter() {
        let position = transform.translation;
        let rotation = transform.rotation;
        
        match map_editor_state.editor_mode {
            EditorMode::Translate => {
                draw_translate_gizmo(&mut gizmos, position, rotation);
            }
            EditorMode::Rotate => {
                draw_rotate_gizmo(&mut gizmos, position, rotation);
            }
            EditorMode::Scale => {
                draw_scale_gizmo(&mut gizmos, position, rotation);
            }
            _ => {}
        }
    }
}

/// Draw translation gizmo (arrows)
fn draw_translate_gizmo(gizmos: &mut Gizmos, position: Vec3, _rotation: Quat) {
    let arrow_length = 1.0;
    let arrow_head_size = 0.1;
    
    // X axis - Red
    let x_end = position + Vec3::X * arrow_length;
    gizmos.line(position, x_end, Color::srgb(1.0, 0.0, 0.0));
    gizmos.sphere(x_end, arrow_head_size, Color::srgb(1.0, 0.0, 0.0));
    
    // Y axis - Green
    let y_end = position + Vec3::Y * arrow_length;
    gizmos.line(position, y_end, Color::srgb(0.0, 1.0, 0.0));
    gizmos.sphere(y_end, arrow_head_size, Color::srgb(0.0, 1.0, 0.0));
    
    // Z axis - Blue
    let z_end = position + Vec3::Z * arrow_length;
    gizmos.line(position, z_end, Color::srgb(0.0, 0.0, 1.0));
    gizmos.sphere(z_end, arrow_head_size, Color::srgb(0.0, 0.0, 1.0));
}

/// Draw rotation gizmo (rings)
fn draw_rotate_gizmo(gizmos: &mut Gizmos, position: Vec3, _rotation: Quat) {
    let radius = 0.8;
    let segments = 32;
    
    // X axis ring - Red
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        
        let p1 = position + Vec3::new(0.0, angle1.cos() * radius, angle1.sin() * radius);
        let p2 = position + Vec3::new(0.0, angle2.cos() * radius, angle2.sin() * radius);
        gizmos.line(p1, p2, Color::srgb(1.0, 0.3, 0.3));
    }
    
    // Y axis ring - Green
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        
        let p1 = position + Vec3::new(angle1.cos() * radius, 0.0, angle1.sin() * radius);
        let p2 = position + Vec3::new(angle2.cos() * radius, 0.0, angle2.sin() * radius);
        gizmos.line(p1, p2, Color::srgb(0.3, 1.0, 0.3));
    }
    
    // Z axis ring - Blue
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        
        let p1 = position + Vec3::new(angle1.cos() * radius, angle1.sin() * radius, 0.0);
        let p2 = position + Vec3::new(angle2.cos() * radius, angle2.sin() * radius, 0.0);
        gizmos.line(p1, p2, Color::srgb(0.3, 0.3, 1.0));
    }
}

/// Draw scale gizmo (boxes)
fn draw_scale_gizmo(gizmos: &mut Gizmos, position: Vec3, _rotation: Quat) {
    let handle_size = 0.15;
    let handle_offset = 1.0;
    
    // X axis - Red
    let x_pos = position + Vec3::X * handle_offset;
    gizmos.cuboid(
        Transform::from_translation(x_pos).with_scale(Vec3::splat(handle_size)),
        Color::srgb(1.0, 0.0, 0.0),
    );
    
    // Y axis - Green
    let y_pos = position + Vec3::Y * handle_offset;
    gizmos.cuboid(
        Transform::from_translation(y_pos).with_scale(Vec3::splat(handle_size)),
        Color::srgb(0.0, 1.0, 0.0),
    );
    
    // Z axis - Blue
    let z_pos = position + Vec3::Z * handle_offset;
    gizmos.cuboid(
        Transform::from_translation(z_pos).with_scale(Vec3::splat(handle_size)),
        Color::srgb(0.0, 0.0, 1.0),
    );
    
    // Center box - White
    gizmos.cuboid(
        Transform::from_translation(position).with_scale(Vec3::splat(handle_size * 0.8)),
        Color::srgb(1.0, 1.0, 1.0),
    );
}

/// Plugin for transform gizmo systems
pub struct TransformGizmoPlugin;

impl Plugin for TransformGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GizmoDragState>()
            .add_systems(Update, (
                transform_gizmo_system,
                draw_gizmo_visuals,
            ).chain());
    }
}
