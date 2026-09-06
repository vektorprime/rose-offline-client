use bevy::prelude::*;
use bevy_mesh::VertexAttributeValues;

use crate::components::{BoatState, SailMesh, SailSide};
use crate::graphics::{GraphicsSettings, SailQuality};
use crate::resources::WindState;
use crate::sailing::{angle_to_wind_abs, sail_speed_factor};

fn nearest_parent_boat_state(
    mut current: Entity,
    parent_query: &Query<&ChildOf>,
    boat_query: &Query<&BoatState>,
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
    boat_query: Query<&BoatState>,
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

        let angle_to_wind_abs = angle_to_wind_abs(boat.heading, wind.angle);
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

        let Some(mut mesh) = meshes.get_mut(&mesh_3d.0) else {
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
