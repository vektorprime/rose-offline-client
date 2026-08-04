use crate::components::{ClientEntity, ClientEntityType, MonsterSeparation, Position};
use bevy::prelude::*;

/// System that pushes overlapping hostile monsters apart.
/// Only applies to entities with ClientEntityType::Monster.
/// Note: Position is in centimeters, so we need to scale our separation values accordingly.
///
/// Positions are collected once per frame and sorted by X so the inner overlap
/// scan can early-out as soon as the X delta exceeds the largest possible
/// overlap distance (sorted-sweep, same pattern as fish_system.rs). Buffers
/// are reused across frames to avoid per-frame allocations.
pub fn monster_separation_system(
    mut query: Query<(Entity, &mut Position, &MonsterSeparation, &ClientEntity)>,
    mut monster_positions: Local<Vec<(Entity, Vec3, f32)>>,
    mut order: Local<Vec<usize>>,
    time: Res<Time>,
) {
    // Convert separation radius from meters to centimeters for comparison with Position
    // Collect all monster positions for overlap checking
    monster_positions.clear();
    monster_positions.extend(
        query
            .iter()
            .filter(|(_, _, _, client_entity)| {
                client_entity.entity_type == ClientEntityType::Monster
            })
            .map(|(e, pos, sep, _)| {
                // Convert separation_radius from meters to centimeters
                (e, pos.position, sep.separation_radius * 100.0)
            }),
    );

    let n = monster_positions.len();
    if n < 2 {
        return;
    }

    // Sort indices by X so each monster only scans the X window that can
    // possibly overlap it (distance >= |dx|, so any pair with |dx| larger
    // than the sum of radii can never overlap).
    order.clear();
    order.extend(0..n);
    order.sort_by(|&a, &b| monster_positions[a].1.x.total_cmp(&monster_positions[b].1.x));

    let max_radius_cm = monster_positions
        .iter()
        .map(|(_, _, radius_cm)| *radius_cm)
        .fold(0.0_f32, f32::max);

    for (entity, mut position, separation, client_entity) in query.iter_mut() {
        // Only apply to hostile monsters
        if client_entity.entity_type != ClientEntityType::Monster {
            continue;
        }

        let mut total_separation = Vec3::ZERO;
        let mut overlap_count = 0;

        let my_radius_cm = separation.separation_radius * 100.0; // Convert to centimeters
        let max_dx = my_radius_cm + max_radius_cm;
        // First index whose X is >= mine; scanning left from here covers all
        // monsters with smaller X, scanning right covers all with larger X.
        let start = order.partition_point(|&i| monster_positions[i].1.x < position.position.x);

        for side in 0..2 {
            let mut k = start;
            loop {
                if side == 0 {
                    if k == 0 {
                        break;
                    }
                    k -= 1;
                } else {
                    k += 1;
                    if k >= n {
                        break;
                    }
                }

                let (other_entity, other_pos, other_radius_cm) = monster_positions[k];
                if other_entity == entity {
                    continue;
                }

                let dx = if side == 0 {
                    position.position.x - other_pos.x
                } else {
                    other_pos.x - position.position.x
                };
                if dx > max_dx {
                    break;
                }

                let distance = (position.position - other_pos).length();
                let min_distance = my_radius_cm + other_radius_cm;

                if distance < min_distance && distance > 0.001 {
                    // Calculate overlap and push direction
                    let overlap = min_distance - distance;
                    let direction = (position.position - other_pos).normalize();

                    // Add separation force proportional to overlap
                    // overlap is in centimeters, force is a multiplier
                    total_separation += direction * overlap * separation.separation_force;
                    overlap_count += 1;
                }
            }
        }

        if overlap_count > 0 {
            // Apply averaged separation, clamped to max (converted to centimeters)
            // max_separation is in meters per second, convert to cm/s
            let max_sep_cm_per_sec = separation.max_separation * 100.0;
            let separation_vector = (total_separation / overlap_count as f32)
                .clamp_length_max(max_sep_cm_per_sec * time.delta_secs());
            position.position += separation_vector;
        }
    }
}
