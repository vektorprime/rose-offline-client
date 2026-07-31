use bevy::prelude::Resource;
use std::collections::HashMap;

use rose_data::ItemType;
use rose_data_irose::encode_item_type;

use crate::scripting::lua4::Lua4Value;

pub const SV_SEX: i32 = 0;
pub const SV_BIRTH: i32 = 1;
pub const SV_CLASS: i32 = 2;
pub const SV_UNION: i32 = 3;
pub const SV_RANK: i32 = 4;
pub const SV_FAME: i32 = 5;
pub const SV_STR: i32 = 6;
pub const SV_DEX: i32 = 7;
pub const SV_INT: i32 = 8;
pub const SV_CON: i32 = 9;
pub const SV_CHA: i32 = 10;
pub const SV_SEN: i32 = 11;
pub const SV_EXP: i32 = 12;
pub const SV_LEVEL: i32 = 13;
pub const SV_POINT: i32 = 14;

#[derive(Resource)]
pub struct LuaGameConstants {
    pub constants: HashMap<String, Lua4Value>,
}

impl Default for LuaGameConstants {
    fn default() -> Self {
        let constants: HashMap<String, Lua4Value> = [
            ("SV_SEX", SV_SEX.into()),
            ("SV_BIRTH", SV_BIRTH.into()),
            ("SV_CLASS", SV_CLASS.into()),
            ("SV_UNION", SV_UNION.into()),
            ("SV_RANK", SV_RANK.into()),
            ("SV_FAME", SV_FAME.into()),
            ("SV_STR", SV_STR.into()),
            ("SV_DEX", SV_DEX.into()),
            ("SV_INT", SV_INT.into()),
            ("SV_CON", SV_CON.into()),
            ("SV_CHA", SV_CHA.into()),
            ("SV_SEN", SV_SEN.into()),
            ("SV_EXP", SV_EXP.into()),
            ("SV_LEVEL", SV_LEVEL.into()),
            ("SV_POINT", SV_POINT.into()),
            ("ITEM_TYPE_FACE_ITEM", encode_item_type(ItemType::Face).unwrap().into()),
            ("ITEM_TYPE_HELMET", encode_item_type(ItemType::Head).unwrap().into()),
            ("ITEM_TYPE_ARMOR", encode_item_type(ItemType::Body).unwrap().into()),
            ("ITEM_TYPE_GAUNTLET", encode_item_type(ItemType::Hands).unwrap().into()),
            ("ITEM_TYPE_BOOTS", encode_item_type(ItemType::Feet).unwrap().into()),
            ("ITEM_TYPE_KNAPSACK", encode_item_type(ItemType::Back).unwrap().into()),
            ("ITEM_TYPE_JEWEL", encode_item_type(ItemType::Jewellery).unwrap().into()),
            ("ITEM_TYPE_WEAPON", encode_item_type(ItemType::Weapon).unwrap().into()),
            ("ITEM_TYPE_SUBWPN", encode_item_type(ItemType::SubWeapon).unwrap().into()),
            ("ITEM_TYPE_USE", encode_item_type(ItemType::Consumable).unwrap().into()),
            ("ITEM_TYPE_ETC", encode_item_type(ItemType::Gem).unwrap().into()),
            ("ITEM_TYPE_GEM", encode_item_type(ItemType::Gem).unwrap().into()),
            ("ITEM_TYPE_NATURAL", encode_item_type(ItemType::Material).unwrap().into()),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();

        Self { constants }
    }
}
