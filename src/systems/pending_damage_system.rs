use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Time, With};

use rose_game_common::{components::HealthPoints, data::Damage};

use crate::{
    components::{ClientEntity, Dead, NextCommand, PendingDamageList},
    resources::ClientEntityList,
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
) {
    if health_points.hp < damage.amount as i32 {
        health_points.hp = 0;
    } else {
        health_points.hp -= damage.amount as i32;
    }

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
    )>,
    dead_entities: Query<(), With<Dead>>,
    time: Res<Time>,
    mut client_entity_list: ResMut<ClientEntityList>,
) {
    let delta_time = time.delta_secs();

    for (entity, client_entity, mut health_points, mut pending_damage_list) in query_target.iter_mut() {
        let mut i = 0;
        while i < pending_damage_list.len() {
            let pending_damage = &mut pending_damage_list[i];
            pending_damage.age += delta_time;

            if pending_damage.is_immediate
                || pending_damage.age > MAX_DAMAGE_AGE
                || pending_damage
                    .attacker
                    .map_or(true, |attacker| dead_entities.contains(attacker))
            {
                let pending_damage = pending_damage_list.remove(i);
                apply_damage(
                    &mut commands,
                    entity,
                    client_entity,
                    &mut health_points,
                    &mut pending_damage_list,
                    pending_damage.damage,
                    pending_damage.is_kill,
                    &mut client_entity_list,
                );
            } else {
                i += 1;
            }
        }
    }
}
