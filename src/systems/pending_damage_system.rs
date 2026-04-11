use bevy::prelude::{
    Commands, Entity, GlobalTransform, MessageWriter, Query, Res, ResMut, Time, Vec3, With,
};

use rose_game_common::{components::HealthPoints, data::Damage};

use crate::{
    components::{ClientEntity, Dead, DeathBloodHandled, NextCommand, PendingDamageList},
    events::{BloodEffectEvent, BloodImpactProfile},
    resources::{BloodEffectConfig, ClientEntityList, DamageDigitsSpawner},
};

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let len_sq = value.length_squared();
    if len_sq > 1e-6 {
        value / len_sq.sqrt()
    } else {
        fallback
    }
}

fn random_local_wound_pose(_model_height: f32) -> (Vec3, Vec3) {
    let y = -0.04 + rand::random::<f32>() * 0.18;
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let radial = 0.05 + rand::random::<f32>() * 0.16;
    let x = radial * angle.cos();
    let z = radial * angle.sin();
    let normal = normalize_or(Vec3::new(x, 0.05, z), Vec3::Z);
    (Vec3::new(x, y, z), normal)
}

// After 5 seconds, expire pending damage and apply immediately
const MAX_DAMAGE_AGE: f32 = 5.0;

fn apply_damage(
    commands: &mut Commands,
    entity: Entity,
    client_entity: &ClientEntity,
    pending_damage_list: &mut PendingDamageList,
    damage: Damage,
    is_killed: bool,
    client_entity_list: &mut ClientEntityList,
    damage_digits_spawner: &DamageDigitsSpawner,
    global_transform: &bevy::prelude::GlobalTransform,
    model_height: Option<&crate::components::ModelHeight>,
) {
    let _ = pending_damage_list;

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

    if is_killed {
        commands
            .entity(entity)
            .insert(Dead)
            .insert(DeathBloodHandled)
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
    query_transform: Query<&GlobalTransform>,
    time: Res<Time>,
    mut blood_effect_events: MessageWriter<BloodEffectEvent>,
    mut client_entity_list: ResMut<ClientEntityList>,
    damage_digits_spawner: Res<DamageDigitsSpawner>,
    blood_config: Res<BloodEffectConfig>,
) {
    let _ = &query_transform;

    // log::info!("[PENDING_DAMAGE_SYSTEM] System running, processing entities...");
    let delta_time = time.delta_secs();

    let mut total_entities_processed = 0;
    let mut total_damage_applied = 0;

    for (
        entity,
        client_entity,
        mut health_points,
        mut pending_damage_list,
        global_transform,
        model_height,
    ) in query_target.iter_mut()
    {
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
                    &mut pending_damage_list,
                    pending_damage.damage,
                    pending_damage.is_kill,
                    &mut client_entity_list,
                    &damage_digits_spawner,
                    global_transform,
                    model_height,
                );

                if pending_damage.damage.amount > 0 {
                    let defender_pos = global_transform.translation();
                    let impact_direction = pending_damage
                        .attacker
                        .and_then(|attacker| query_transform.get(attacker).ok())
                        .map(|transform| normalize_or(defender_pos - transform.translation(), Vec3::Y))
                        .unwrap_or(Vec3::Y);

                    if pending_damage.is_kill {
                        blood_effect_events.write(BloodEffectEvent::kill_spatter_with_profile(
                            defender_pos,
                            Vec3::Y,
                            pending_damage.damage.amount,
                            impact_direction,
                            BloodImpactProfile::Slash,
                        ));
                    } else {
                        blood_effect_events.write(BloodEffectEvent::hit_spatter_with_profile(
                            defender_pos,
                            Vec3::Y,
                            pending_damage.damage.amount,
                            impact_direction,
                            BloodImpactProfile::Slash,
                        ));
                    }

                    if blood_config.enable_blood && blood_config.show_wounds {
                        let model_h = model_height.map_or(1.8, |h| h.height);
                        let wound_events = if pending_damage.is_kill { 3 } else { 2 };
                        for _ in 0..wound_events {
                            let (wound_position, wound_normal) = random_local_wound_pose(model_h);
                            blood_effect_events.write(BloodEffectEvent::show_wound(
                                entity,
                                wound_position,
                                wound_normal,
                            ));
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
    }

    // log::info!("[PENDING_DAMAGE_SYSTEM] Finished processing: {} entities, {} damage entries applied", total_entities_processed, total_damage_applied);
}
