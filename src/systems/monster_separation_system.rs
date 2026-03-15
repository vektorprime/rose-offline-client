use bevy::prelude::*;
use crate::components::{MonsterSeparation, Position, ClientEntity, ClientEntityType};

/// System that pushes overlapping hostile monsters apart.
/// Only applies to entities with ClientEntityType::Monster.
/// Note: Position is in centimeters, so we need to scale our separation values accordingly.
pub fn monster_separation_system(
    mut query: Query<
        (Entity, &mut Position, &MonsterSeparation, &ClientEntity),
    >,
    time: Res<Time>,
) {
    // Convert separation radius from meters to centimeters for comparison with Position
    // Collect all monster positions for overlap checking
    let monster_positions: Vec<(Entity, Vec3, f32)> = query
        .iter()
        .filter(|(_, _, _, client_entity)| client_entity.entity_type == ClientEntityType::Monster)
        .map(|(e, pos, sep, _)| {
            // Convert separation_radius from meters to centimeters
            (e, pos.position, sep.separation_radius * 100.0)
        })
        .collect();
    
    for (entity, mut position, separation, client_entity) in query.iter_mut() {
        // Only apply to hostile monsters
        if client_entity.entity_type != ClientEntityType::Monster {
            continue;
        }
        
        let mut total_separation = Vec3::ZERO;
        let mut overlap_count = 0;
        
        let my_radius_cm = separation.separation_radius * 100.0; // Convert to centimeters
        
        for (other_entity, other_pos, other_radius_cm) in &monster_positions {
            if *other_entity == entity {
                continue;
            }
            
            let distance = (position.position - *other_pos).length();
            let min_distance = my_radius_cm + other_radius_cm;
            
            if distance < min_distance && distance > 0.001 {
                // Calculate overlap and push direction
                let overlap = min_distance - distance;
                let direction = (position.position - *other_pos).normalize();
                
                // Add separation force proportional to overlap
                // overlap is in centimeters, force is a multiplier
                total_separation += direction * overlap * separation.separation_force;
                overlap_count += 1;
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
