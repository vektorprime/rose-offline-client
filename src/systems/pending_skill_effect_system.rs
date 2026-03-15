use bevy::prelude::{Entity, EventReader, EventWriter, Query, Res, Time};
use std::time::Instant;

use rose_data::{AbilityType, AnimationEventFlags, SkillData, StatusEffectType};
use rose_game_common::components::{
    AbilityValues, HealthPoints, ManaPoints, MoveSpeed, StatusEffects,
};

use crate::{
    animation::AnimationFrameEvent,
    bundles::ability_values_get_value,
    components::{PendingSkillEffectList, PendingSkillTargetList},
    events::HitEvent,
    resources::GameData,
};

// After 10 seconds, apply skill effects regardless
const MAX_SKILL_EFFECT_AGE: f32 = 10.0;

fn apply_skill_effect(
    skill_data: &SkillData,
    game_data: &GameData,
    current_instant: Instant,
    entity: Entity,
    ability_values: &AbilityValues,
    health_points: &mut HealthPoints,
    mana_points: Option<&mut ManaPoints>,
    move_speed: &MoveSpeed,
    pending_skill_effect_list: &mut PendingSkillEffectList,
    status_effects: &mut StatusEffects,
    caster_intelligence: i32,
    effect_success: [bool; 2],
) {
    let mut mana_points = mana_points;
    for (skill_effect_index, success) in effect_success.iter().enumerate() {
        if !success {
            continue;
        }

        let status_effect_data = skill_data
            .status_effects
            .get(skill_effect_index)
            .and_then(|x| x.as_ref())
            .and_then(|status_effect_id| {
                game_data
                    .status_effects
                    .get_status_effect(*status_effect_id)
            });
        if let Some(status_effect_data) = status_effect_data {
            let adjust_value = if matches!(
                status_effect_data.status_effect_type,
                StatusEffectType::AdditionalDamageRate
            ) {
                skill_data.power as i32
            } else if let Some(skill_add_ability) =
                skill_data.add_ability[skill_effect_index].as_ref()
            {
                // We only need components which can potentially be altered by status effects
                let ability_value = ability_values_get_value(
                    skill_add_ability.ability_type,
                    ability_values,
                    None,
                    None,
                    None,
                    Some(health_points),
                    None,
                    None,
                    mana_points.as_deref(),
                    Some(move_speed),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap_or(0);

                game_data
                    .ability_value_calculator
                    .calculate_skill_adjust_value(
                        skill_add_ability,
                        caster_intelligence,
                        ability_value,
                    )
            } else {
                0
            };

            status_effects.apply_status_effect(
                status_effect_data,
                current_instant.checked_add(skill_data.status_effect_duration).unwrap_or(current_instant),
                adjust_value,
            );
        }

        let add_ability = skill_data
            .add_ability
            .get(skill_effect_index)
            .and_then(|x| x.as_ref());
        if let Some(add_ability) = add_ability {
            match add_ability.ability_type {
                AbilityType::Health => {
                    health_points.hp = i32::min(
                        ability_values.get_max_health(),
                        health_points.hp
                            + game_data
                                .ability_value_calculator
                                .calculate_skill_adjust_value(
                                    add_ability,
                                    caster_intelligence,
                                    health_points.hp,
                                ),
                    );
                }
                AbilityType::Mana => {
                    if let Some(mana_points) = mana_points.as_mut() {
                        mana_points.mp = i32::min(
                            ability_values.get_max_mana(),
                            mana_points.mp + add_ability.value,
                        );
                    }
                }
                AbilityType::Stamina | AbilityType::Money => {
                    log::warn!(
                        "Unimplemented skill status effect add ability_type {:?}, value {}",
                        add_ability.ability_type,
                        add_ability.value
                    )
                }
                _ => {}
            }
        }
    }
}

pub fn pending_skill_effect_system(
    mut query_caster: Query<(Entity, &mut PendingSkillTargetList)>,
    mut query_target: Query<(
        Entity,
        &AbilityValues,
        &mut HealthPoints,
        Option<&mut ManaPoints>,
        &MoveSpeed,
        &mut PendingSkillEffectList,
        &mut StatusEffects,
    )>,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    mut hit_events: EventWriter<HitEvent>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    // Apply skill effects triggered by animation frames
    for event in animation_frame_events.read() {
        if !event
            .flags
            .contains(AnimationEventFlags::APPLY_PENDING_SKILL_EFFECT)
        {
            continue;
        }

        if let Ok((caster_entity, mut caster_pending_skill_target_list)) =
            query_caster.get_mut(event.entity)
        {
            // Find all our skill targets
            for pending_skill_target in caster_pending_skill_target_list.drain(..) {
                if let Ok((
                    target_entity,
                    ability_values,
                    mut health_points,
                    mut mana_points,
                    move_speed,
                    mut pending_skill_effect_list,
                    mut status_effects,
                )) = query_target.get_mut(pending_skill_target.defender_entity)
                {
                    // Apply any skill affects from caster_entity
                    let mut i = 0;
                    while i < pending_skill_effect_list.len() {
                        if pending_skill_effect_list[i].caster_entity != Some(caster_entity)
                        {
                            i += 1;
                            continue;
                        }

                        let pending_skill_effect = pending_skill_effect_list
                            .pending_skill_effects
                            .remove(i);

                        if let Some(skill_data) =
                            game_data.skills.get_skill(pending_skill_effect.skill_id)
                        {
                            hit_events.write(HitEvent::with_skill_effect(
                                event.entity,
                                target_entity,
                                pending_skill_effect.skill_id,
                            ));

                            apply_skill_effect(
                                skill_data,
                                &game_data,
                                Instant::now(),
                                target_entity,
                                ability_values,
                                health_points.as_mut(),
                                mana_points.as_deref_mut(),
                                move_speed,
                                pending_skill_effect_list.as_mut(),
                                status_effects.as_mut(),
                                pending_skill_effect.caster_intelligence,
                                pending_skill_effect.effect_success,
                            );
                        }
                    }
                }
            }
        }
    }

    // Apply expired skill effects
    let delta_time = time.delta_secs();
    for (
        target_entity,
        ability_values,
        mut health_points,
        mut mana_points,
        move_speed,
        mut pending_skill_effect_list,
        mut status_effects,
    ) in query_target.iter_mut()
    {
        let mut i = 0;
        while i < pending_skill_effect_list.pending_skill_effects.len() {
            let pending_skill_effect = &mut pending_skill_effect_list.pending_skill_effects[i];
            pending_skill_effect.age += delta_time;

            if pending_skill_effect.age > MAX_SKILL_EFFECT_AGE {
                let pending_skill_effect =
                    pending_skill_effect_list.pending_skill_effects.remove(i);

                if let Some(skill_data) =
                    game_data.skills.get_skill(pending_skill_effect.skill_id)
                {
                    hit_events.write(HitEvent::with_skill_effect(
                        pending_skill_effect.caster_entity.unwrap_or(target_entity),
                        target_entity,
                        pending_skill_effect.skill_id,
                    ));

                    apply_skill_effect(
                        skill_data,
                        &game_data,
                        Instant::now(),
                        target_entity,
                        ability_values,
                        health_points.as_mut(),
                        mana_points.as_deref_mut(),
                        move_speed,
                        pending_skill_effect_list.as_mut(),
                        status_effects.as_mut(),
                        pending_skill_effect.caster_intelligence,
                        pending_skill_effect.effect_success,
                    );
                }
            } else {
                i += 1;
            }
        }
    }
}
