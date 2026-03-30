use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Time, With};

use rose_game_common::{components::HealthPoints, data::Damage};

use crate::{
    components::{ClientEntity, Dead, NextCommand, PendingDamageList},
    resources::{ClientEntityList, DamageDigitsSpawner},
};

// After 5 seconds, expire pending damage and apply immediately
const MAX_DAMAGE_AGE: f32 = 5.0;

fn apply_damage(
    commands: &mut Commands,
    entity: Entity,
    client_entity: &ClientEntity,
    health_points: &mut HealthPoints,
    pending_damage_list: &mut PendingDamageList,
    damage: Damage,
    is_killed: bool,
    client_entity_list: &mut ClientEntityList,
    damage_digits_spawner: &DamageDigitsSpawner,
    global_transform: &bevy::prelude::GlobalTransform,
    model_height: Option<&crate::components::ModelHeight>,
) {
    // log::info!("[PENDING_DAMAGE] Applying damage: {} to entity {:?}", damage.amount, entity);
    
    if health_points.hp < damage.amount as i32 {
        health_points.hp = 0;
    } else {
        health_points.hp -= damage.amount as i32;
    }

    // Spawn damage digits
    let height = model_height.map_or(1.8, |h| h.height);
    
    damage_digits_spawner.spawn(
        commands,
        global_transform,
        height,
        damage.amount,
        client_entity_list
            .player_entity
            .map_or(false, |player_entity| entity == player_entity),
    );
    // log::info!("[PENDING_DAMAGE] Spawned damage digits for damage: {}", damage.amount);

    if is_killed {
        commands
            .entity(entity)
            .insert(Dead)
            .insert(NextCommand::with_die())
            .remove::<ClientEntity>();
        client_entity_list.remove(client_entity.id);
    }
}

pub fn pending_damage_system(
    mut commands: Commands,
    mut query_target: Query<(
        Entity,
        &ClientEntity,
        &mut HealthPoints,
        &mut PendingDamageList,
        &bevy::prelude::GlobalTransform,
        Option<&crate::components::ModelHeight>,
    )>,
    dead_entities: Query<(), With<Dead>>,
    time: Res<Time>,
    mut client_entity_list: ResMut<ClientEntityList>,
    damage_digits_spawner: Res<DamageDigitsSpawner>,
) {
    // log::info!("[PENDING_DAMAGE_SYSTEM] System running, processing entities...");
    let delta_time = time.delta_secs();

    let mut total_entities_processed = 0;
    let mut total_damage_applied = 0;
    
    for (entity, client_entity, mut health_points, mut pending_damage_list, global_transform, model_height) in query_target.iter_mut() {
        total_entities_processed += 1;
        // log::info!("[PENDING_DAMAGE_SYSTEM] Processing entity {:?} with {} pending damage entries", entity, pending_damage_list.len());
        
        let mut i = 0;
        while i < pending_damage_list.len() {
            let pending_damage = &mut pending_damage_list[i];
            pending_damage.age += delta_time;

            // log::info!("[PENDING_DAMAGE_SYSTEM] Checking pending damage at index {}: age={:.2}s, is_immediate={}, is_kill={}, damage={}", 
            //    i, pending_damage.age, pending_damage.is_immediate, pending_damage.is_kill, pending_damage.damage.amount);

            if pending_damage.is_immediate
                || pending_damage.age > MAX_DAMAGE_AGE
                || pending_damage
                    .attacker
                    .map_or(true, |attacker| dead_entities.contains(attacker))
            {
                if pending_damage.is_immediate {
                    // log::info!("[PENDING_DAMAGE_SYSTEM] Applying damage because: is_immediate=true");
                } else if pending_damage.age > MAX_DAMAGE_AGE {
                    // log::info!("[PENDING_DAMAGE_SYSTEM] Applying damage because: age {:.2}s > MAX_DAMAGE_AGE {:.2}s", pending_damage.age, MAX_DAMAGE_AGE);
                } else {
                    // log::info!("[PENDING_DAMAGE_SYSTEM] Applying damage because: attacker is dead");
                }
                
                let pending_damage = pending_damage_list.remove(i);
                total_damage_applied += 1;
                
                // log::info!("[PENDING_DAMAGE_SYSTEM] Calling apply_damage for entity {:?}, damage={}, is_kill={}", entity, pending_damage.damage.amount, pending_damage.is_kill);
                apply_damage(
                    &mut commands,
                    entity,
                    client_entity,
                    &mut health_points,
                    &mut pending_damage_list,
                    pending_damage.damage,
                    pending_damage.is_kill,
                    &mut client_entity_list,
                    &damage_digits_spawner,
                    global_transform,
                    model_height,
                );
            } else {
                i += 1;
            }
        }
    }
    
    // log::info!("[PENDING_DAMAGE_SYSTEM] Finished processing: {} entities, {} damage entries applied", total_entities_processed, total_damage_applied);
}
