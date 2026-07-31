use bevy::prelude::{Commands, Entity, GlobalTransform, MessageWriter, Vec3};

use crate::{
    components::ModelHeight,
    events::{BloodEffectEvent, BloodImpactProfile},
    resources::{BloodEffectConfig, ClientEntityList, DamageDigitsSpawner},
};
pub fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let len_sq = value.length_squared();
    if len_sq > 1e-6 {
        value / len_sq.sqrt()
    } else {
        fallback
    }
}

pub fn random_local_wound_pose() -> (Vec3, Vec3) {
    let y = -0.04 + rand::random::<f32>() * 0.18;
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let radial = 0.05 + rand::random::<f32>() * 0.16;
    let x = radial * angle.cos();
    let z = radial * angle.sin();
    let normal = normalize_or(Vec3::new(x, 0.05, z), Vec3::Z);
    (Vec3::new(x, y, z), normal)
}

pub fn spawn_damage_digits(
    commands: &mut Commands,
    damage_digits_spawner: &DamageDigitsSpawner,
    global_transform: &GlobalTransform,
    model_height: Option<&ModelHeight>,
    damage_amount: u32,
    entity: Entity,
    client_entity_list: &ClientEntityList,
) {
    let height = model_height.map_or(1.8, |h| h.height);

    damage_digits_spawner.spawn(
        commands,
        global_transform,
        height,
        damage_amount,
        client_entity_list
            .player_entity
            .map_or(false, |player_entity| entity == player_entity),
    );
}

pub fn emit_blood_and_wounds(
    blood_effect_events: &mut MessageWriter<BloodEffectEvent>,
    blood_config: &BloodEffectConfig,
    defender_pos: Vec3,
    damage_amount: u32,
    is_killed: bool,
    impact_direction: Vec3,
    blood_profile: BloodImpactProfile,
    entity: Entity,
) {
    if is_killed {
        blood_effect_events.write(BloodEffectEvent::kill_spatter_with_profile(
            defender_pos,
            Vec3::Y,
            damage_amount,
            impact_direction,
            blood_profile,
        ));
    } else {
        blood_effect_events.write(BloodEffectEvent::hit_spatter_with_profile(
            defender_pos,
            Vec3::Y,
            damage_amount,
            impact_direction,
            blood_profile,
        ));
    }

    if blood_config.enable_blood && blood_config.show_wounds {
        let wound_events = if is_killed { 3 } else { 2 };
        for _ in 0..wound_events {
            let (wound_position, wound_normal) = random_local_wound_pose();
            blood_effect_events.write(BloodEffectEvent::show_wound(
                entity,
                wound_position,
                wound_normal,
            ));
        }
    }
}
