use std::num::NonZeroUsize;

use bevy::{ecs::world, prelude::Mut};
use num_traits::{AsPrimitive, Saturating, Signed};
use rose_data::AbilityType;
use world::EntityWorldMut;

use rose_game_common::components::{
    AbilityValues, BasicStats, CharacterGender, CharacterInfo, ExperiencePoints, GuildMembership,
    HealthPoints, Inventory, Level, ManaPoints, Money, MoveSpeed, SkillPoints, Stamina, StatPoints,
    Team, UnionMembership, MAX_STAMINA,
};

pub fn ability_values_get_value(
    ability_type: AbilityType,
    ability_values: &AbilityValues,
    character_info: Option<&CharacterInfo>,
    experience_points: Option<&ExperiencePoints>,
    guild_membership: Option<&GuildMembership>,
    health_points: Option<&HealthPoints>,
    inventory: Option<&Inventory>,
    level: Option<&Level>,
    mana_points: Option<&ManaPoints>,
    move_speed: Option<&MoveSpeed>,
    skill_points: Option<&SkillPoints>,
    stamina: Option<&Stamina>,
    stat_points: Option<&StatPoints>,
    team: Option<&Team>,
    union_membership: Option<&UnionMembership>,
) -> Option<i32> {
    match ability_type {
        AbilityType::Gender => character_info.map(|x| match x.gender {
            CharacterGender::Male => 0,
            CharacterGender::Female => 1,
        }),
        AbilityType::Race => character_info.map(|x| (x.race / 2) as i32),
        AbilityType::Birthstone => character_info.map(|x| x.birth_stone as i32),
        AbilityType::Job => character_info.map(|x| x.job as i32),
        AbilityType::Rank => character_info.map(|x| x.rank as i32),
        AbilityType::Fame => character_info.map(|x| x.fame as i32),
        AbilityType::FameB => character_info.map(|x| x.fame_b as i32),
        AbilityType::FameG => character_info.map(|x| x.fame_g as i32),
        AbilityType::Face => character_info.map(|x| x.face as i32),
        AbilityType::Hair => character_info.map(|x| x.hair as i32),
        AbilityType::Strength => Some(ability_values.get_strength()),
        AbilityType::Dexterity => Some(ability_values.get_dexterity()),
        AbilityType::Intelligence => Some(ability_values.get_intelligence()),
        AbilityType::Concentration => Some(ability_values.get_concentration()),
        AbilityType::Charm => Some(ability_values.get_charm()),
        AbilityType::Sense => Some(ability_values.get_sense()),
        AbilityType::Attack => Some(ability_values.get_attack_power()),
        AbilityType::Defence => Some(ability_values.get_defence()),
        AbilityType::Hit => Some(ability_values.get_hit()),
        AbilityType::Resistance => Some(ability_values.get_resistance()),
        AbilityType::Avoid => Some(ability_values.get_avoid()),
        AbilityType::AttackSpeed => Some(ability_values.get_attack_speed()),
        AbilityType::Critical => Some(ability_values.get_critical()),
        AbilityType::Speed => move_speed.map(|x| x.speed as i32),
        AbilityType::Skillpoint => skill_points.map(|x| x.points as i32),
        AbilityType::BonusPoint => stat_points.map(|x| x.points as i32),
        AbilityType::Experience => experience_points.map(|x| x.xp as i32),
        AbilityType::Level => level.map(|x| x.level as i32),
        AbilityType::Money => inventory.map(|x| x.money.0 as i32),
        AbilityType::TeamNumber => team.map(|x| x.id as i32),
        AbilityType::Union => {
            union_membership.map(|x| x.current_union.map(|x| x.get() as i32).unwrap_or(0))
        }
        AbilityType::Stamina => stamina.map(|x| x.stamina as i32),
        AbilityType::MaxHealth => Some(ability_values.get_max_health()),
        AbilityType::MaxMana => Some(ability_values.get_max_mana()),
        AbilityType::Health => health_points.map(|x| x.hp),
        AbilityType::Mana => mana_points.map(|x| x.mp),
        // Weight: calculated from all inventory items
        AbilityType::Weight => inventory.map(|inv| {
            inv.calculate_total_weight(|_item_ref, quantity| {
                // Default weight calculation - returns weight per item * quantity
                // In a real implementation, this would look up the item's base weight
                // For now, return 0 as placeholder (server should track actual weight)
                0 * quantity
            })
        }),
        // SaveMana: Mana save percentage (0-100), clamped
        AbilityType::SaveMana => character_info.map(|ci| ci.save_mana.min(100) as i32),
        // PvpFlag: PvP flag state (0 = off, non-zero = on)
        AbilityType::PvpFlag => character_info.map(|ci| ci.pvp_flag),
        // HeadSize: Head size for appearance customization
        AbilityType::HeadSize => character_info.map(|ci| ci.head_size),
        // BodySize: Body size for appearance customization
        AbilityType::BodySize => character_info.map(|ci| ci.body_size),
        // DropRate: Drop rate percentage modifier
        AbilityType::DropRate => character_info.map(|ci| ci.drop_rate),
        // CurrentPlanet: Current planet ID
        AbilityType::CurrentPlanet => character_info.map(|ci| ci.current_planet as i32),
        // Guild values: return 0 when not in guild
        AbilityType::GuildNumber => guild_membership.and_then(|gm| {
            if gm.is_none() {
                Some(0)
            } else {
                Some(gm.guild_number as i32)
            }
        }),
        AbilityType::GuildScore => guild_membership.and_then(|gm| {
            if gm.is_none() {
                Some(0)
            } else {
                Some(gm.score)
            }
        }),
        AbilityType::GuildPosition => guild_membership.and_then(|gm| {
            if gm.is_none() {
                Some(0)
            } else {
                Some(gm.position as i32)
            }
        }),
        _ => {
            if let Some(index) = union_point_index(ability_type) {
                return union_membership.map(|x| x.points[index] as i32);
            }

            log::warn!(
                "ability_values_get_value unimplemented for ability type {:?}",
                ability_type
            );
            None
        }
    }
}

fn add_value<T: Saturating + Copy + 'static, U: Signed + AsPrimitive<T>>(
    value: T,
    add_value: U,
) -> T {
    if add_value.is_negative() {
        value.saturating_sub(add_value.abs().as_())
    } else {
        value.saturating_add(add_value.as_())
    }
}

fn union_point_index(ability_type: AbilityType) -> Option<usize> {
    match ability_type {
        AbilityType::UnionPoint1 => Some(0),
        AbilityType::UnionPoint2 => Some(1),
        AbilityType::UnionPoint3 => Some(2),
        AbilityType::UnionPoint4 => Some(3),
        AbilityType::UnionPoint5 => Some(4),
        AbilityType::UnionPoint6 => Some(5),
        AbilityType::UnionPoint7 => Some(6),
        AbilityType::UnionPoint8 => Some(7),
        AbilityType::UnionPoint9 => Some(8),
        AbilityType::UnionPoint10 => Some(9),
        _ => None,
    }
}

macro_rules! add_value_match {
    (
        $ability_type:expr,
        $value:expr,
        basic_stats: $basic_stats:expr,
        stat_points: $stat_points:expr,
        skill_points: $skill_points:expr,
        inventory: $inventory:expr,
        stamina: $stamina:expr,
        health_points: $health_points:expr,
        max_health: $max_health:expr,
        mana_points: $mana_points:expr,
        max_mana: $max_mana:expr,
        experience_points: $experience_points:expr,
        level: $level:expr,
    ) => {
        match $ability_type {
            AbilityType::Strength => {
                if let Some(mut c) = $basic_stats {
                    c.strength = add_value(c.strength, $value);
                }
            }
            AbilityType::Dexterity => {
                if let Some(mut c) = $basic_stats {
                    c.dexterity = add_value(c.dexterity, $value);
                }
            }
            AbilityType::Intelligence => {
                if let Some(mut c) = $basic_stats {
                    c.intelligence = add_value(c.intelligence, $value);
                }
            }
            AbilityType::Concentration => {
                if let Some(mut c) = $basic_stats {
                    c.concentration = add_value(c.concentration, $value);
                }
            }
            AbilityType::Charm => {
                if let Some(mut c) = $basic_stats {
                    c.charm = add_value(c.charm, $value);
                }
            }
            AbilityType::Sense => {
                if let Some(mut c) = $basic_stats {
                    c.sense = add_value(c.sense, $value);
                }
            }
            AbilityType::BonusPoint => {
                if let Some(mut c) = $stat_points {
                    c.points = add_value(c.points, $value);
                }
            }
            AbilityType::Skillpoint => {
                if let Some(mut c) = $skill_points {
                    c.points = add_value(c.points, $value);
                }
            }
            AbilityType::Money => {
                if let Some(mut c) = $inventory {
                    c.try_add_money(Money($value as i64)).ok();
                }
            }
            AbilityType::Stamina => {
                if let Some(mut c) = $stamina {
                    c.stamina = u32::min(add_value(c.stamina, $value), MAX_STAMINA);
                }
            }
            AbilityType::Health => {
                let max_health = $max_health;
                if let Some(mut c) = $health_points {
                    c.hp = add_value(c.hp, $value).min(max_health.unwrap_or(i32::MAX));
                }
            }
            AbilityType::Mana => {
                let max_mana = $max_mana;
                if let Some(mut c) = $mana_points {
                    c.mp = add_value(c.mp, $value).min(max_mana.unwrap_or(i32::MAX));
                }
            }
            AbilityType::Experience => {
                if let Some(mut c) = $experience_points {
                    c.xp = add_value(c.xp, $value);
                }
            }
            AbilityType::Level => {
                if let Some(mut c) = $level {
                    c.level = add_value(c.level, $value);
                }
            }
            _ => {
                log::warn!(
                    "ability_values_add_value unimplemented for ability type {:?}",
                    $ability_type
                );
                return false;
            }
        }
    };
}

pub fn ability_values_add_value(
    ability_type: AbilityType,
    value: i32,
    ability_values: &AbilityValues,
    basic_stats: &mut Mut<BasicStats>,
    experience_points: &mut Mut<ExperiencePoints>,
    health_points: &mut Mut<HealthPoints>,
    inventory: &mut Mut<Inventory>,
    level: &mut Mut<Level>,
    mana_points: &mut Mut<ManaPoints>,
    skill_points: &mut Mut<SkillPoints>,
    stamina: &mut Mut<Stamina>,
    stat_points: &mut Mut<StatPoints>,
    union_membership: &mut Mut<UnionMembership>,
) -> bool {
    if let Some(index) = union_point_index(ability_type) {
        union_membership.points[index] = add_value(union_membership.points[index], value);
        return true;
    }

    add_value_match!(
        ability_type, value,
        basic_stats: Some(&mut *basic_stats),
        stat_points: Some(&mut *stat_points),
        skill_points: Some(&mut *skill_points),
        inventory: Some(&mut *inventory),
        stamina: Some(&mut *stamina),
        health_points: Some(&mut *health_points),
        max_health: Some(ability_values.get_max_health()),
        mana_points: Some(&mut *mana_points),
        max_mana: Some(ability_values.get_max_mana()),
        experience_points: Some(&mut *experience_points),
        level: Some(&mut *level),
    );

    true
}

pub fn ability_values_add_value_exclusive(
    ability_type: AbilityType,
    value: i32,
    entity: &mut EntityWorldMut,
) -> bool {
    if let Some(index) = union_point_index(ability_type) {
        if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
            union_membership.points[index] = add_value(union_membership.points[index], value);
        }
        return true;
    }

    add_value_match!(
        ability_type, value,
        basic_stats: entity.get_mut::<BasicStats>(),
        stat_points: entity.get_mut::<StatPoints>(),
        skill_points: entity.get_mut::<SkillPoints>(),
        inventory: entity.get_mut::<Inventory>(),
        stamina: entity.get_mut::<Stamina>(),
        health_points: entity.get_mut::<HealthPoints>(),
        max_health: entity.get::<AbilityValues>().map(|x| x.get_max_health()),
        mana_points: entity.get_mut::<ManaPoints>(),
        max_mana: entity.get::<AbilityValues>().map(|x| x.get_max_mana()),
        experience_points: entity.get_mut::<ExperiencePoints>(),
        level: entity.get_mut::<Level>(),
    );

    true
}

pub fn ability_values_set_value_exclusive(
    ability_type: AbilityType,
    value: i32,
    entity: &mut EntityWorldMut,
) -> bool {
    if let Some(index) = union_point_index(ability_type) {
        if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
            union_membership.points[index] = value as u32;
        }
        return true;
    }

    match ability_type {
        AbilityType::Gender => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                if value == 0 {
                    character_info.gender = CharacterGender::Male;
                } else {
                    character_info.gender = CharacterGender::Female;
                }
            }
        }
        AbilityType::Face => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.face = value as u8;
            }
        }
        AbilityType::Hair => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.hair = value as u8;
            }
        }
        AbilityType::Job => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.job = value as u16;
            }
        }
        AbilityType::Strength => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.strength = value;
            }
        }
        AbilityType::Dexterity => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.dexterity = value;
            }
        }
        AbilityType::Intelligence => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.intelligence = value;
            }
        }
        AbilityType::Concentration => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.concentration = value;
            }
        }
        AbilityType::Charm => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.charm = value;
            }
        }
        AbilityType::Sense => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.sense = value;
            }
        }
        AbilityType::Union => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                if value == 0 {
                    union_membership.current_union = None;
                } else {
                    union_membership.current_union = NonZeroUsize::new(value as usize);
                }
            }
        }
        AbilityType::Health => {
            let max_hp = entity
                .get::<AbilityValues>()
                .map(|ability_values| ability_values.get_max_health());

            if let Some(mut health_points) = entity.get_mut::<HealthPoints>() {
                let mut new_hp = value;
                if let Some(max_hp) = max_hp {
                    new_hp = new_hp.min(max_hp);
                }

                health_points.hp = new_hp;
            }
        }
        AbilityType::Mana => {
            let max_mp = entity
                .get::<AbilityValues>()
                .map(|ability_values| ability_values.get_max_mana());

            if let Some(mut mana_points) = entity.get_mut::<ManaPoints>() {
                let mut new_mp = value;
                if let Some(max_mp) = max_mp {
                    new_mp = new_mp.min(max_mp);
                }

                mana_points.mp = new_mp;
            }
        }
        AbilityType::Experience => {
            if let Some(mut experience_points) = entity.get_mut::<ExperiencePoints>() {
                experience_points.xp = value as u64;
            }
        }
        AbilityType::Level => {
            if let Some(mut level) = entity.get_mut::<Level>() {
                level.level = value as u32;
            }
        }
        AbilityType::TeamNumber => {
            if let Some(mut team) = entity.get_mut::<Team>() {
                team.id = value as u32;
            }
        }
        // PvpFlag: Set PvP flag state
        AbilityType::PvpFlag => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.pvp_flag = value;
            }
        }
        _ => {
            log::warn!(
                "ability_values_set_value unimplemented for ability type {:?}",
                ability_type
            );
            return false;
        }
    }

    true
}
