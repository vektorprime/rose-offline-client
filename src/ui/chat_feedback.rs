//! System chat lines for actions refused for lack of a resource (MP, HP, stamina, Zuly, ammo,
//! fuel, the required weapon, cooldowns, inventory or storage space).
//!
//! The checks mirror the rose-offline server (`skill_use_requirements_met`, the attack checks in
//! `command_system`, the store / bank / equipment transactions). The server refuses all of these
//! silently, so the client reports the refusal itself and does not send a request that would
//! only be dropped.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use bevy::prelude::MessageWriter;

use rose_data::{
    AbilityType, AmmoIndex, EquipmentIndex, Item, ItemClass, SkillCooldown, SkillData, SkillType,
    VehiclePartIndex,
};
use rose_game_common::components::{
    AbilityValues, Equipment, ExperiencePoints, HealthPoints, Inventory, ManaPoints, MoveMode,
    Stamina,
};

use crate::{
    components::{Bank, Cooldowns},
    events::ChatboxEvent,
    resources::GameData,
};

/// The same line is written at most once per interval, so a held hotkey or repeated clicks
/// cannot flood the chat box.
const CHAT_FEEDBACK_REPEAT_INTERVAL: Duration = Duration::from_secs(1);

/// rose-offline grows a bank to at most 90 slots (`BANK_MAX_NORMAL_SLOTS`) while the client
/// always shows 160, so only slots below this bound (or below the last stored item) are free.
const SERVER_BANK_SLOTS: usize = 90;

/// Per-system throttle for feedback lines; keep it in a `Local`.
#[derive(Default)]
pub struct ChatFeedbackThrottle {
    last_sent: HashMap<String, Instant>,
}

impl ChatFeedbackThrottle {
    /// Whether `message` may be written now; when it may, it counts as written.
    pub fn should_send(&mut self, message: &str) -> bool {
        let now = Instant::now();
        self.last_sent
            .retain(|_, sent| now.duration_since(*sent) < CHAT_FEEDBACK_REPEAT_INTERVAL);

        if self.last_sent.contains_key(message) {
            return false;
        }

        self.last_sent.insert(message.to_string(), now);
        true
    }

    pub fn send(&mut self, chatbox_events: &mut MessageWriter<ChatboxEvent>, message: String) {
        if self.should_send(&message) {
            chatbox_events.write(ChatboxEvent::System(message));
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ChatFeedback {
    Waiting,
    NotEnoughAbility(AbilityType),
    SkillUnusable,
    SkillWhileDriving,
    SkillRequiredEquipment,
    SkillRequirements,
    InvalidTarget,
    ItemRequirements,
    ItemUsedUp,
    NotEnoughMoney,
    NotEnoughStatPoints,
    InventoryFull,
    EquipmentSpace,
    StorageFull,
    NoAmmo(AmmoIndex),
    AmmoUsedUp,
    WeaponBroken,
    VehicleOutOfFuel,
    VehicleWeaponBroken,
    StorePricesChanged,
    StoreTransactionFailed,
    StoreTooFarAway,
    StoreNotSameUnion,
    StoreNotEnoughUnionPoints,
}

impl ChatFeedback {
    /// The chat line, using the original client's text from LIST_STRING.STL where it has one.
    pub fn message(self, game_data: &GameData) -> String {
        match self {
            ChatFeedback::Waiting => game_string(game_data, 134, "Waiting..."),
            ChatFeedback::NotEnoughAbility(ability_type) => {
                not_enough_ability_message(game_data, ability_type)
            }
            ChatFeedback::SkillUnusable => game_string(game_data, 245, "Skill cannot be used."),
            ChatFeedback::SkillWhileDriving => {
                game_string(game_data, 442, "You cannot use skill while riding.")
            }
            ChatFeedback::SkillRequiredEquipment => {
                game_string(game_data, 247, "You must wear the required equipment.")
            }
            ChatFeedback::SkillRequirements => game_string(
                game_data,
                370,
                "Cannot use skill because of unfulfilled requirements.",
            ),
            ChatFeedback::InvalidTarget => game_string(game_data, 128, "Invalid target."),
            ChatFeedback::ItemRequirements => {
                game_string(game_data, 136, "Unusable due to unfulfilled requirements.")
            }
            ChatFeedback::ItemUsedUp => "You do not have any more of that item.".to_string(),
            ChatFeedback::NotEnoughMoney => {
                game_string(game_data, 42, "You do not have enough Zuly.")
            }
            ChatFeedback::NotEnoughStatPoints => "You do not have enough stat points.".to_string(),
            ChatFeedback::InventoryFull => {
                game_string(game_data, 187, "Insufficient space in your inventory.")
            }
            ChatFeedback::EquipmentSpace => game_string(
                game_data,
                199,
                "No more equipment can be worn unless more inventory space is made available.",
            ),
            ChatFeedback::StorageFull => game_string(
                game_data,
                332,
                "There is not enough space in your Storage for this item.",
            ),
            ChatFeedback::NoAmmo(ammo_index) => format!(
                "You need to equip {} to attack with this weapon.",
                match ammo_index {
                    AmmoIndex::Arrow => "arrows",
                    AmmoIndex::Bullet => "bullets",
                    AmmoIndex::Throw => "shells",
                }
            ),
            ChatFeedback::AmmoUsedUp => {
                game_string(game_data, 622, "You have run out of ammunition.")
            }
            ChatFeedback::WeaponBroken => {
                "Your weapon is broken and must be repaired before you can attack.".to_string()
            }
            ChatFeedback::VehicleOutOfFuel => "Your cart has run out of fuel.".to_string(),
            ChatFeedback::VehicleWeaponBroken => {
                "Your cart weapon is broken and must be repaired before you can attack.".to_string()
            }
            ChatFeedback::StorePricesChanged => {
                game_string(game_data, 340, "The prices have changed.")
            }
            // rose-offline also answers "NPC not found" when the bought items do not fit.
            ChatFeedback::StoreTransactionFailed => {
                "Store transaction failed. Check that you have enough inventory space.".to_string()
            }
            ChatFeedback::StoreTooFarAway => "You are too far away from the store.".to_string(),
            ChatFeedback::StoreNotSameUnion => {
                game_string(game_data, 250, "You have not joined this Faction.")
            }
            ChatFeedback::StoreNotEnoughUnionPoints => {
                game_string(game_data, 252, "More Faction Points are required.")
            }
        }
    }
}

/// A line of the game's LIST_STRING.STL (the original client's messages), or `fallback` when
/// the game data does not have it.
fn game_string(game_data: &GameData, string_id: u16, fallback: &str) -> String {
    let string_database = &game_data.string_database;
    string_database
        .client_strings
        .get_text_string(string_database.language, &string_id.to_string())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn ability_name(game_data: &GameData, ability_type: AbilityType) -> String {
    let name = game_data
        .string_database
        .get_ability_type(ability_type)
        .trim();
    if name.is_empty() {
        format!("{:?}", ability_type)
    } else {
        name.to_string()
    }
}

fn not_enough_ability_message(game_data: &GameData, ability_type: AbilityType) -> String {
    match ability_type {
        AbilityType::Mana => game_string(game_data, 127, "You do not have enough MP."),
        AbilityType::Money => ChatFeedback::NotEnoughMoney.message(game_data),
        AbilityType::Experience => "You do not have enough experience points.".to_string(),
        AbilityType::Fuel => "Your cart does not have enough fuel.".to_string(),
        AbilityType::Health | AbilityType::Stamina => format!(
            "You do not have enough {}.",
            ability_name(game_data, ability_type)
        ),
        _ => format!(
            "Your {} is too low to use this skill.",
            ability_name(game_data, ability_type)
        ),
    }
}

/// Whether the global, per-skill or group cooldown of `skill_data` is still running
/// (server: `skill_cooldown_ready`).
pub fn skill_on_cooldown(cooldowns: &Cooldowns, skill_data: &SkillData) -> bool {
    let skill_cooldown = match &skill_data.cooldown {
        SkillCooldown::Skill { .. } => cooldowns.has_skill_cooldown(skill_data.id),
        SkillCooldown::Group { group, .. } => cooldowns.has_skill_group_cooldown(group.get()),
    };

    skill_cooldown || cooldowns.has_global_cooldown()
}

/// Skill types the client sends as a CastSkill message; only those go through the server's
/// `skill_use_requirements_met` (basic actions and emotes have their own messages).
pub fn is_cast_skill_type(skill_type: SkillType) -> bool {
    skill_type.is_self_skill()
        || skill_type.is_target_skill()
        || matches!(skill_type, SkillType::AreaTarget)
}

/// The player components the server's skill use check reads.
pub struct SkillCaster<'a> {
    pub ability_values: &'a AbilityValues,
    pub equipment: &'a Equipment,
    pub experience_points: &'a ExperiencePoints,
    pub health_points: &'a HealthPoints,
    pub inventory: &'a Inventory,
    pub mana_points: &'a ManaPoints,
    pub move_mode: &'a MoveMode,
    pub stamina: &'a Stamina,
}

/// Why the server would refuse to cast `skill_data`, in the order of the server's
/// `skill_use_requirements_met` (driving, use_ability costs, required weapon). Cooldowns are
/// checked separately with [`skill_on_cooldown`].
pub fn skill_use_refusal(
    game_data: &GameData,
    caster: &SkillCaster,
    skill_data: &SkillData,
) -> Option<ChatFeedback> {
    if matches!(caster.move_mode, MoveMode::Drive) {
        return Some(ChatFeedback::SkillWhileDriving);
    }

    for &(use_ability_type, mut use_ability_value) in skill_data.use_ability.iter() {
        if use_ability_type == AbilityType::Mana {
            let use_mana_rate = (100 - caster.ability_values.get_save_mana()) as f32 / 100.0;
            use_ability_value = (use_ability_value as f32 * use_mana_rate) as i32;
        }

        let ability_value = match use_ability_type {
            AbilityType::Level => Some(caster.ability_values.level),
            AbilityType::Strength => Some(caster.ability_values.strength),
            AbilityType::Dexterity => Some(caster.ability_values.dexterity),
            AbilityType::Intelligence => Some(caster.ability_values.intelligence),
            AbilityType::Concentration => Some(caster.ability_values.concentration),
            AbilityType::Charm => Some(caster.ability_values.charm),
            AbilityType::Sense => Some(caster.ability_values.sense),
            AbilityType::Health => Some(caster.health_points.hp),
            AbilityType::Mana => Some(caster.mana_points.mp),
            AbilityType::Experience => {
                Some(i32::try_from(caster.experience_points.xp).unwrap_or(i32::MAX))
            }
            AbilityType::Money => Some(i32::try_from(caster.inventory.money.0).unwrap_or(i32::MAX)),
            AbilityType::Stamina => Some(i32::try_from(caster.stamina.stamina).unwrap_or(i32::MAX)),
            AbilityType::Fuel => Some(
                caster
                    .equipment
                    .get_vehicle_item(VehiclePartIndex::Engine)
                    .map_or(0, |item| item.life as i32),
            ),
            _ => None,
        };

        match ability_value {
            Some(ability_value) if ability_value < use_ability_value => {
                return Some(ChatFeedback::NotEnoughAbility(use_ability_type));
            }
            // The server values any other cost type at -999, so it can never be paid.
            None if -999 < use_ability_value => return Some(ChatFeedback::SkillUnusable),
            _ => {}
        }
    }

    if !has_required_equipment(game_data, caster.equipment, skill_data) {
        return Some(ChatFeedback::SkillRequiredEquipment);
    }

    None
}

/// Whether the weapon or sub weapon is of a class `skill_data` requires.
fn has_required_equipment(
    game_data: &GameData,
    equipment: &Equipment,
    skill_data: &SkillData,
) -> bool {
    if skill_data.required_equipment_class.is_empty() {
        return true;
    }

    let equipped_class = |equipment_index: EquipmentIndex| -> Option<ItemClass> {
        equipment
            .get_equipment_item(equipment_index)
            .and_then(|item| game_data.items.get_base_item(item.item))
            .map(|item_data| item_data.class)
    };
    let weapon_class = equipped_class(EquipmentIndex::Weapon);
    let sub_weapon_class = equipped_class(EquipmentIndex::SubWeapon);

    skill_data
        .required_equipment_class
        .iter()
        .any(|&required_class| {
            weapon_class == Some(required_class) || sub_weapon_class == Some(required_class)
        })
}

/// Why the server would cancel a normal attack by the player (`command_system`): a broken
/// weapon or empty ammo slot on foot, a broken engine or cart weapon while driving.
///
/// The server also cancels when fewer arrows / bullets remain than the attack fires, but the
/// client only learns the exact ammo count every 16 shots, so only an empty slot is reported.
pub fn attack_refusal(
    game_data: &GameData,
    equipment: &Equipment,
    move_mode: &MoveMode,
) -> Option<ChatFeedback> {
    if matches!(move_mode, MoveMode::Drive) {
        if equipment
            .get_vehicle_item(VehiclePartIndex::Engine)
            .map_or(false, |item| item.life == 0)
        {
            return Some(ChatFeedback::VehicleOutOfFuel);
        }

        if equipment
            .get_vehicle_item(VehiclePartIndex::Arms)
            .map_or(false, |item| item.life == 0)
        {
            return Some(ChatFeedback::VehicleWeaponBroken);
        }

        return None;
    }

    let weapon = equipment.get_equipment_item(EquipmentIndex::Weapon);
    if weapon.map_or(false, |item| item.life == 0) {
        return Some(ChatFeedback::WeaponBroken);
    }

    let ammo_index = weapon
        .and_then(|item| game_data.items.get_base_item(item.item))
        .and_then(|item_data| match item_data.class {
            ItemClass::Bow | ItemClass::Crossbow => Some(AmmoIndex::Arrow),
            ItemClass::Gun | ItemClass::DualGuns => Some(AmmoIndex::Bullet),
            ItemClass::Launcher => Some(AmmoIndex::Throw),
            _ => None,
        });

    match ammo_index {
        Some(ammo_index) if equipment.get_ammo_item(ammo_index).is_none() => {
            Some(ChatFeedback::NoAmmo(ammo_index))
        }
        _ => None,
    }
}

/// Whether `item` fits into the inventory the way the server adds it (stack onto a matching
/// stack of its page, else the first free slot of that page).
pub fn inventory_has_space_for(inventory: &Inventory, item: Item) -> bool {
    inventory.clone().try_add_item(item).is_ok()
}

/// Whether a deposit of `item` fits into the bank (server `Bank::try_add_item`).
pub fn bank_has_space_for(bank: &Bank, item: &Item) -> bool {
    if let Item::Stackable(stackable_item) = item {
        if bank
            .slots
            .iter()
            .flatten()
            .any(|bank_item| bank_item.can_stack_with(stackable_item).is_ok())
        {
            return true;
        }
    }

    let server_slot_count = bank
        .slots
        .iter()
        .rposition(|slot| slot.is_some())
        .map_or(0, |last_used_slot| last_used_slot + 1)
        .max(SERVER_BANK_SLOTS);

    bank.slots.len() < server_slot_count
        || bank
            .slots
            .iter()
            .take(server_slot_count)
            .any(|slot| slot.is_none())
}
