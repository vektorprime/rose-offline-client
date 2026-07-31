use bevy::prelude::{
    Commands, Entity, GlobalTransform, MessageWriter, Query, Res, ResMut, Time, Vec3, With,
};

use rose_game_common::components::HealthPoints;

use crate::{
    components::{ClientEntity, Dead, DeathBloodHandled, ModelHeight, NextCommand, PendingDamageList},
    events::{BloodEffectEvent, BloodImpactProfile},
    resources::{BloodEffectConfig, ClientEntityList, DamageDigitsSpawner},
    systems::damage_effects::{emit_blood_and_wounds, normalize_or, spawn_damage_digits},
};

// After 5 seconds, expire pending damage and apply immediately
const MAX_DAMAGE_AGE: f32 = 5.0;

pub fn pending_damage_system(
    mut commands: Commands,
    mut query_target: Query<(
        Entity,
        &ClientEntity,
        &mut HealthPoints,
        &mut PendingDamageList,
        &GlobalTransform,
        Option<&ModelHeight>,
    )>,
    dead_entities: Query<(), With<Dead>>,
    query_transform: Query<&GlobalTransform>,
    time: Res<Time>,
    mut blood_effect_events: MessageWriter<BloodEffectEvent>,
    mut client_entity_list: ResMut<ClientEntityList>,
    damage_digits_spawner: Res<DamageDigitsSpawner>,
    blood_config: Res<BloodEffectConfig>,
) {
    let delta_time = time.delta_secs();

    for (
        entity,
        client_entity,
        _health_points,
        mut pending_damage_list,
        global_transform,
        model_height,
    ) in query_target.iter_mut()
    {
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

                spawn_damage_digits(
                    &mut commands,
                    &damage_digits_spawner,
                    global_transform,
                    model_height,
                    pending_damage.damage.amount,
                    entity,
                    &client_entity_list,
                );

                if pending_damage.is_kill {
                    commands
                        .entity(entity)
                        .insert(Dead)
                        .insert(DeathBloodHandled)
                        .insert(NextCommand::with_die())
                        .remove::<ClientEntity>();
                    client_entity_list.remove(client_entity.id);
                }

                if pending_damage.damage.amount > 0 {
                    let defender_pos = global_transform.translation();
                    let impact_direction = pending_damage
                        .attacker
                        .and_then(|attacker| query_transform.get(attacker).ok())
                        .map(|transform| {
                            normalize_or(defender_pos - transform.translation(), Vec3::Y)
                        })
                        .unwrap_or(Vec3::Y);

                    emit_blood_and_wounds(
                        &mut blood_effect_events,
                        &blood_config,
                        defender_pos,
                        pending_damage.damage.amount,
                        pending_damage.is_kill,
                        impact_direction,
                        BloodImpactProfile::Slash,
                        entity,
                    );
                }
            } else {
                i += 1;
            }
        }
    }
}
