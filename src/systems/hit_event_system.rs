use bevy::{
    ecs::query::QueryData,
    prelude::{
        Commands, Entity, GlobalTransform, MessageReader, MessageWriter, Query, Res, ResMut,
        Vec3,
    },
};

use rose_game_common::{
    components::{AbilityValues, HealthPoints, ManaPoints, MoveSpeed, StatusEffects},
    data::Damage,
};

use crate::{
    components::{
        ClientEntity, ClientEntityType, Dead, DeathBloodHandled, ModelHeight, NextCommand,
        PendingDamageList,
        PendingSkillEffectList, PendingSkillTargetList,
    },
    events::{BloodEffectEvent, HitEvent, SpawnEffectData, SpawnEffectEvent},
    resources::{BloodEffectConfig, ClientEntityList, DamageDigitsSpawner, GameData},
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

#[derive(QueryData)]
#[query_data(mutable)]
pub struct HitAttackerQuery<'w> {
    entity: Entity,
    pending_skill_target_list: &'w mut PendingSkillTargetList,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct HitDefenderQuery<'w> {
    entity: Entity,
    client_entity: &'w ClientEntity,
    pending_damage_list: &'w mut PendingDamageList,
    pending_skill_effect_list: &'w mut PendingSkillEffectList,
    ability_values: &'w AbilityValues,
    health_points: &'w mut HealthPoints,
    global_transform: &'w GlobalTransform,
    mana_points: Option<&'w mut ManaPoints>,
    model_height: Option<&'w ModelHeight>,
    move_speed: &'w MoveSpeed,
    status_effects: &'w mut StatusEffects,
}

fn apply_damage(
    commands: &mut Commands,
    defender: &mut HitDefenderQueryItem,
    damage: Damage,
    is_killed: bool,
    damage_digits_spawner: &DamageDigitsSpawner,
    client_entity_list: &mut ClientEntityList,
) {
    damage_digits_spawner.spawn(
        commands,
        defender.global_transform,
        defender
            .model_height
            .map_or(1.8, |model_height| model_height.height),
        damage.amount,
        client_entity_list
            .player_entity
            .map_or(false, |player_entity| defender.entity == player_entity),
    );

    if is_killed {
        commands
            .entity(defender.entity)
            .insert(Dead)
            .insert(DeathBloodHandled)
            .insert(NextCommand::with_die());

        if defender.client_entity.entity_type != ClientEntityType::Character {
            commands.entity(defender.entity).remove::<ClientEntity>();
            client_entity_list.remove(defender.client_entity.id);
        }
    }
}

pub fn hit_event_system(
    mut commands: Commands,
    mut query_defender: Query<HitDefenderQuery>,
    query_transform: Query<&GlobalTransform>,
    mut hit_events: MessageReader<HitEvent>,
    mut spawn_effect_events: MessageWriter<SpawnEffectEvent>,
    mut blood_effect_events: MessageWriter<BloodEffectEvent>,
    mut client_entity_list: ResMut<ClientEntityList>,
    damage_digits_spawner: Res<DamageDigitsSpawner>,
    game_data: Res<GameData>,
    blood_config: Res<BloodEffectConfig>,
) {
    for event in hit_events.read() {
        let defender = query_defender.get_mut(event.defender).ok();
        if defender.is_none() {
            continue;
        }
        let mut defender = defender.unwrap();

        // Apply pending damage
        let mut damage = Damage {
            amount: 0,
            is_critical: false,
            apply_hit_stun: false,
        };
        let mut is_killed = false;
        let mut has_damage = false;

        if event.apply_damage {
            let mut i = 0;
            while i < defender.pending_damage_list.len() {
                if defender.pending_damage_list[i].attacker == Some(event.attacker)
                    && event.skill_id
                        == defender.pending_damage_list[i]
                            .from_skill
                            .map(|(damage_skill_id, _)| damage_skill_id)
                {
                    let pending_damage = defender.pending_damage_list.remove(i);
                    damage.amount += pending_damage.damage.amount;
                    damage.is_critical |= pending_damage.damage.is_critical;
                    damage.apply_hit_stun |= pending_damage.damage.apply_hit_stun;
                    is_killed |= pending_damage.is_kill;
                    has_damage = true;
                } else {
                    i += 1;
                }
            }

            if has_damage || !event.ignore_miss {
                apply_damage(
                    &mut commands,
                    &mut defender,
                    damage,
                    is_killed,
                    &damage_digits_spawner,
                    &mut client_entity_list,
                );
            }

            if has_damage && damage.amount > 0 {
                let defender_pos = defender.global_transform.translation();
                let impact_direction = query_transform
                    .get(event.attacker)
                    .map(|transform| {
                        normalize_or(defender_pos - transform.translation(), Vec3::Y)
                    })
                    .unwrap_or(Vec3::Y);

                if is_killed {
                    blood_effect_events.write(BloodEffectEvent::kill_spatter_with_profile(
                        defender_pos,
                        Vec3::Y,
                        damage.amount,
                        impact_direction,
                        event.blood_profile,
                    ));
                } else {
                    blood_effect_events.write(BloodEffectEvent::hit_spatter_with_profile(
                        defender_pos,
                        Vec3::Y,
                        damage.amount,
                        impact_direction,
                        event.blood_profile,
                    ));
                }

                if blood_config.enable_blood && blood_config.show_wounds {
                    let model_h = defender.model_height.map_or(1.8, |h| h.height);
                    let wound_events = if is_killed { 3 } else { 2 };
                    for _ in 0..wound_events {
                        let (wound_position, wound_normal) = random_local_wound_pose(model_h);
                        blood_effect_events.write(BloodEffectEvent::show_wound(
                            defender.entity,
                            wound_position,
                            wound_normal,
                        ));
                    }
                }
            }
        }

        if let Some(effect_data) = event
            .effect_id
            .and_then(|id| game_data.effect_database.get_effect(id))
        {
            if damage.is_critical {
                if let Some(effect_file_id) = effect_data.hit_effect_critical {
                    spawn_effect_events.write(SpawnEffectEvent::AtEntity(
                        defender.entity,
                        SpawnEffectData::with_file_id(effect_file_id),
                    ));
                }
            }

            if let Some(effect_file_id) = effect_data.hit_effect_normal {
                spawn_effect_events.write(SpawnEffectEvent::AtEntity(
                    defender.entity,
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }

        if let Some(skill_data) = event.skill_id.and_then(|id| game_data.skills.get_skill(id)) {
            if let Some(effect_file_id) = skill_data.hit_effect_file_id {
                spawn_effect_events.write(SpawnEffectEvent::OnEntity(
                    defender.entity,
                    skill_data.hit_link_dummy_bone_id,
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }
    }
}
