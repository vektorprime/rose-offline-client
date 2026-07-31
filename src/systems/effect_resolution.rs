use rose_data::{AmmoIndex, EffectId, EquipmentIndex, ItemClass, VehiclePartIndex};
use rose_game_common::components::{Equipment, Npc};

use crate::{
    events::BloodImpactProfile,
    resources::GameData,
};

pub fn weapon_to_blood_profile(item_class: ItemClass) -> BloodImpactProfile {
    match item_class {
        ItemClass::Bow | ItemClass::Crossbow | ItemClass::DualGuns | ItemClass::Gun => {
            BloodImpactProfile::Projectile
        }
        ItemClass::Launcher => BloodImpactProfile::Blunt,
        ItemClass::Katar | ItemClass::DualSwords => BloodImpactProfile::Pierce,
        _ => BloodImpactProfile::Slash,
    }
}

pub fn resolve_vehicle_arms_bullet_effect_id(
    equipment: &Equipment,
    game_data: &GameData,
) -> Option<EffectId> {
    equipment
        .get_vehicle_item(VehiclePartIndex::Arms)
        .and_then(|arms| game_data.items.get_vehicle_item(arms.item.item_number))
        .and_then(|vehicle_item_data| vehicle_item_data.bullet_effect_id)
}

pub fn resolve_weapon_bullet_effect_id(
    equipment: &Equipment,
    game_data: &GameData,
) -> Option<EffectId> {
    game_data
        .items
        .get_weapon_item(
            equipment
                .get_equipment_item(EquipmentIndex::Weapon)
                .map(|weapon| weapon.item.item_number)
                .unwrap_or(0),
        )
        .and_then(|weapon_item_data| {
            match weapon_item_data.item_data.class {
                ItemClass::Bow | ItemClass::Crossbow => Some(AmmoIndex::Arrow),
                ItemClass::Gun | ItemClass::DualGuns => Some(AmmoIndex::Bullet),
                ItemClass::Launcher => Some(AmmoIndex::Throw),
                _ => None,
            }
            .and_then(|ammo_index| equipment.get_ammo_item(ammo_index))
            .and_then(|ammo_item| game_data.items.get_material_item(ammo_item.item.item_number))
            .and_then(|ammo_item_data| ammo_item_data.bullet_effect_id)
            .or(weapon_item_data.bullet_effect_id)
        })
}

pub fn resolve_weapon_hit_effect_id(
    equipment: Option<&Equipment>,
    npc: Option<&Npc>,
    game_data: &GameData,
) -> Option<EffectId> {
    equipment
        .and_then(|equipment| {
            game_data.items.get_weapon_item(
                equipment
                    .get_equipment_item(EquipmentIndex::Weapon)
                    .map(|weapon| weapon.item.item_number)
                    .unwrap_or(0),
            )
        })
        .and_then(|weapon_item_data| weapon_item_data.effect_id)
        .or_else(|| {
            npc.and_then(|npc| game_data.npcs.get_npc(npc.id))
                .and_then(|npc_data| npc_data.hand_hit_effect_id)
        })
}

pub fn weapon_blood_profile(equipment: Option<&Equipment>, game_data: &GameData) -> BloodImpactProfile {
    equipment
        .and_then(|equipment| {
            game_data.items.get_weapon_item(
                equipment
                    .get_equipment_item(EquipmentIndex::Weapon)
                    .map(|weapon| weapon.item.item_number)
                    .unwrap_or(0),
            )
        })
        .map(|weapon_item_data| weapon_to_blood_profile(weapon_item_data.item_data.class))
        .unwrap_or(BloodImpactProfile::Slash)
}
