use bevy::prelude::{
    Commands, Entity, GlobalTransform, MessageWriter, Query, Res, ResMut, Time, Vec3, With,
};

use rose_game_common::components::HealthPoints;

use crate::{
    animation::SkeletalAnimation,
    components::{
        ClientEntity, Command, Dead, DeathBloodHandled, ModelHeight, NextCommand,
        PendingDamageList, Projectile, ProjectileTarget, Vehicle,
    },
    events::{BloodEffectEvent, BloodImpactProfile},
    resources::{BloodEffectConfig, ClientEntityList, DamageDigitsSpawner, ProjectileIndex},
    systems::damage_effects::{emit_blood_and_wounds, normalize_or, spawn_damage_digits},
};

// After 5 seconds, expire pending damage and apply immediately
const MAX_DAMAGE_AGE: f32 = 5.0;

// A kill should never wait the full pending damage timeout. Once the grace period
// has passed, if no hit frame is expected (see hit_frame_expected) then apply the
// kill immediately - the server has already decided the entity is dead.
const KILL_GRACE_AGE: f32 = 0.25;

// Absolute cap for kills: even if a hit frame was expected but never fired (stuck
// animation, missed hit frame, etc.), apply the kill shortly after the death packet.
const KILL_MAX_DAMAGE_AGE: f32 = 1.5;

/// Returns true if a future client-side hit frame should apply this pending damage:
/// the attacker is mid attack/cast animation (the animation hit frame will fire), or
/// a projectile from the attacker is still in flight toward the defender (it will
/// fire the hit event on impact). Otherwise the kill can be applied immediately.
fn hit_frame_expected(
    attacker: Option<Entity>,
    defender: Entity,
    query_attacker: &Query<(&Command, Option<&SkeletalAnimation>, Option<&Vehicle>)>,
    query_animation: &Query<&SkeletalAnimation>,
    projectile_index: &ProjectileIndex,
    query_projectiles: &Query<&Projectile>,
) -> bool {
    let Some(attacker) = attacker else {
        return false;
    };

    let Ok((command, animation, vehicle)) = query_attacker.get(attacker) else {
        return false;
    };

    if matches!(command, Command::Attack(_) | Command::CastSkill(_)) {
        if animation.map_or(false, |animation| !animation.completed()) {
            return true;
        }

        // Vehicle attacks play on the driver model entity rather than the attacker
        if vehicle.map_or(false, |vehicle| {
            query_animation
                .get(vehicle.driver_model_entity)
                .map_or(false, |animation| !animation.completed())
        }) {
            return true;
        }
    }

    // A projectile in flight will fire the hit event on impact. Only the attacker's
    // own projectiles are checked (index), and each candidate is verified against
    // the world so a stale index entry can never delay a kill.
    projectile_index
        .get(&attacker)
        .is_some_and(|projectile_entities| {
            projectile_entities.iter().any(|&projectile_entity| {
                query_projectiles
                    .get(projectile_entity)
                    .is_ok_and(|projectile| {
                        projectile.source == attacker
                            && matches!(
                                projectile.target,
                                ProjectileTarget::Entity { entity } if entity == defender
                            )
                    })
            })
        })
}

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
    query_attacker: Query<(&Command, Option<&SkeletalAnimation>, Option<&Vehicle>)>,
    query_animation: Query<&SkeletalAnimation>,
    projectile_index: Res<ProjectileIndex>,
    query_projectiles: Query<&Projectile>,
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

            let is_kill = pending_damage.is_kill;
            let attacker = pending_damage.attacker;
            let attacker_dead = attacker.map_or(true, |attacker| dead_entities.contains(attacker));

            let kill_applies_now = is_kill
                && pending_damage.age > KILL_GRACE_AGE
                && !hit_frame_expected(
                    attacker,
                    entity,
                    &query_attacker,
                    &query_animation,
                    &projectile_index,
                    &query_projectiles,
                );

            if pending_damage.is_immediate
                || pending_damage.age > MAX_DAMAGE_AGE
                || attacker_dead
                || kill_applies_now
                || (is_kill && pending_damage.age > KILL_MAX_DAMAGE_AGE)
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
