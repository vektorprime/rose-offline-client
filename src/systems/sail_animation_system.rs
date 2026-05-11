use bevy::prelude::*;
use bevy_mesh::VertexAttributeValues;

use crate::components::{BoatState, PlayerCharacter, SailMesh, SailSide};
use crate::graphics::{GraphicsSettings, SailQuality};
use crate::resources::WindState;

fn sail_speed_factor(angle_to_wind: f32) -> f32 {
    let angle = angle_to_wind.abs();
    if angle < 0.78 {
        (angle / 0.78).powf(2.0) * 0.3
    } else if angle < 1.57 {
        let t = (angle - 0.78) / (1.57 - 0.78);
        0.3 + t * 0.7
    } else if angle < 2.36 {
        let t = (angle - 1.57) / (2.36 - 1.57);
        1.0 - t * 0.2
    } else {
        let t = (angle - 2.36) / (std::f32::consts::PI - 2.36);
        0.8 - t * 0.3
    }
}

fn nearest_parent_boat_state(
    mut current: Entity,
    parent_query: &Query<&ChildOf>,
    boat_query: &Query<&BoatState, With<PlayerCharacter>>,
) -> Option<BoatState> {
    for _ in 0..16 {
        if let Ok(boat) = boat_query.get(current) {
            return Some(boat.clone());
        }

        let Ok(parent) = parent_query.get(current) else {
            break;
        };
        current = parent.parent();
    }

    None
}

pub fn sail_animation_system(
    time: Res<Time>,
    wind: Res<WindState>,
    graphics_settings: Res<GraphicsSettings>,
    boat_query: Query<&BoatState, With<PlayerCharacter>>,
    parent_query: Query<&ChildOf>,
    mut sail_query: Query<(Entity, &mut SailMesh, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if matches!(
        graphics_settings.sailing.sail_deformation_quality,
        SailQuality::Low
    ) {
        return;
    }

    let t = time.elapsed_secs();

    for (sail_entity, mut sail_data, mesh_3d) in sail_query.iter_mut() {
        if sail_data.subdivisions == 0 || sail_data.base_positions.is_empty() {
            continue;
        }

        let Some(boat) = nearest_parent_boat_state(sail_entity, &parent_query, &boat_query) else {
            continue;
        };
        if !boat.active {
            continue;
        }

        let angle_to_wind = (boat.heading - wind.angle).rem_euclid(std::f32::consts::TAU);
        let angle_to_wind_abs = if angle_to_wind > std::f32::consts::PI {
            std::f32::consts::TAU - angle_to_wind
        } else {
            angle_to_wind
        };

        let fill_factor = sail_speed_factor(angle_to_wind_abs).clamp(0.0, 1.0);
        sail_data.billow = fill_factor;

        let apparent = (wind.angle - boat.heading).sin();
        sail_data.side = if apparent > 0.05 {
            SailSide::Starboard
        } else if apparent < -0.05 {
            SailSide::Port
        } else {
            SailSide::Center
        };

        let side_sign = match sail_data.side {
            SailSide::Port => -1.0,
            SailSide::Starboard => 1.0,
            SailSide::Center => 1.0,
        };

        let luff_factor = (1.0 - fill_factor).clamp(0.0, 1.0);
        let billow_depth = 0.6;
        let luff_depth = 0.3;

        let Some(mesh) = meshes.get_mut(&mesh_3d.0) else {
            continue;
        };

        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };

        for (i, pos) in positions.iter_mut().enumerate() {
            if i >= sail_data.base_positions.len() {
                break;
            }

            let base = sail_data.base_positions[i];
            let u = if sail_data.width.abs() > f32::EPSILON {
                ((base[0] / sail_data.width) + 0.5).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let v = if sail_data.height.abs() > f32::EPSILON {
                (base[1] / sail_data.height).clamp(0.0, 1.0)
            } else {
                0.5
            };

            let parabola = 4.0 * u * (1.0 - u);
            let triangle = 1.0 - (2.0 * v - 1.0).abs();
            let billow_amount = fill_factor * parabola * triangle;
            let luff_amount =
                luff_factor * (t * 8.0 + v * 3.0 + u * 5.0).sin() * (1.0 - v).clamp(0.0, 1.0);

            let z_offset = side_sign * (billow_amount * billow_depth + luff_amount * luff_depth);

            pos[0] = base[0];
            pos[1] = base[1];
            pos[2] = z_offset;
        }
    }
}

