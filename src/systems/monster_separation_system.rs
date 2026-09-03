use crate::components::{ClientEntity, ClientEntityType, MonsterSeparation, Position};
use bevy::prelude::*;

/// System that pushes overlapping hostile monsters apart.
/// Only applies to entities with ClientEntityType::Monster.
/// Note: Position is in centimeters, so we need to scale our separation values accordingly.
///
/// PERF: spatial-hash grid + squared distances. Previously O(M^2) with `length()`
/// (sqrt) per pair and a per-frame Vec alloc, no early-out. Now monsters are bucketed
/// into 4m cells and each monster only checks its 3x3 neighborhood with
/// `length_squared()`, skipping sqrt except on actual overlaps.
pub fn monster_separation_system(
    mut query: Query<(Entity, &mut Position, &MonsterSeparation, &ClientEntity)>,
    time: Res<Time>,
) {
    use std::collections::HashMap;

    // Collect monster positions first (single pass).
    let monster_positions: Vec<(Entity, Vec3, f32, f32)> = query
        .iter()
        .filter(|(_, _, _, client_entity)| client_entity.entity_type == ClientEntityType::Monster)
        .map(|(e, pos, sep, _)| {
            (e, pos.position, sep.separation_radius * 100.0, sep.separation_force)
        })
        .collect();

    if monster_positions.len() < 2 {
        return;
    }

    // Spatial hash over the HORIZONTAL plane (Position x=right, y=forward in cm;
    // z is up). Previously bucketed (x,z), mixing height into the grid: hillside
    // pairs missed separation while same-height distant pairs wasted checks.
    // 500cm cells cover the largest separation diameter (2x2.0m radii=400cm) + margin.
    const CELL_CM: f32 = 500.0;
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (idx, (_, pos, _, _)) in monster_positions.iter().enumerate() {
        let cell = (
            (pos.x / CELL_CM).floor() as i32,
            (pos.y / CELL_CM).floor() as i32,
        );
        grid.entry(cell).or_default().push(idx);
    }

    // Snapshot forces first (avoids borrow conflict + order dependence), then apply.
    // Separation is horizontal-only (XY): height differences (bridges/hills) must not
    // suppress it, matching the original game's ground-plane push.
    let mut pushes: Vec<(Entity, Vec3)> = Vec::new();
    for (entity, pos, radius_cm, force) in monster_positions.iter() {
        let cell_x = (pos.x / CELL_CM).floor() as i32;
        let cell_y = (pos.y / CELL_CM).floor() as i32;
        let mut total = Vec3::ZERO;
        let mut count = 0u32;
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(indices) = grid.get(&(cell_x + dx, cell_y + dy)) else {
                    continue;
                };
                for &other_idx in indices {
                    let (other_entity, other_pos, other_radius_cm, _) =
                        &monster_positions[other_idx];
                    if *other_entity == *entity {
                        continue;
                    }
                    let min_distance = *radius_cm + *other_radius_cm;
                    debug_assert!(
                        min_distance <= CELL_CM,
                        "separation diameter exceeds grid cell"
                    );
                    // Horizontal-only delta (x,y); z (height) ignored.
                    let delta = Vec3::new(pos.x - other_pos.x, pos.y - other_pos.y, 0.0);
                    // Cheap squared reject before sqrt. Epsilon guards zero-length.
                    let dist_sq = delta.length_squared();
                    if dist_sq >= min_distance * min_distance || dist_sq < 1e-6 {
                        continue;
                    }
                    let distance = dist_sq.sqrt();
                    let overlap = min_distance - distance;
                    // Safe: dist_sq >= 1e-6 so distance > 0.001.
                    total += delta / distance * overlap * *force;
                    count += 1;
                }
            }
        }
        if count > 0 {
            total /= count as f32;
            pushes.push((*entity, total));
        }
    }

    if pushes.is_empty() {
        return;
    }
    let push_map: HashMap<Entity, Vec3> = pushes.into_iter().collect();

    // Apply averaged, clamped pushes (map lookup: only overlapping monsters present).
    for (entity, mut position, separation, _) in query.iter_mut() {
        if let Some(push) = push_map.get(&entity) {
            let max_sep_cm_per_sec = separation.max_separation * 100.0;
            position.position +=
                (*push).clamp_length_max(max_sep_cm_per_sec * time.delta_secs());
        }
    }
}
