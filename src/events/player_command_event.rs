use bevy::prelude::{Entity, Message};

use rose_data::{AmmoIndex, EquipmentIndex, VehiclePartIndex};
use rose_game_common::components::{HotbarSlot, ItemSlot, SkillSlot};

use crate::components::Position;

#[derive(Message, Clone)]
pub enum PlayerCommandEvent {
    UseSkill(SkillSlot),
    LevelUpSkill(SkillSlot),
    DropItem(ItemSlot),
    DropItemWithQuantity(ItemSlot, usize),
    UseItem(ItemSlot),
    UseHotbar(usize, usize),
    SetHotbar(usize, usize, Option<HotbarSlot>),
    Attack(Entity),
    Move(Position, Option<Entity>),
    UnequipAmmo(AmmoIndex),
    UnequipEquipment(EquipmentIndex),
    UnequipVehicle(VehiclePartIndex),
    EquipAmmo(ItemSlot),
    EquipEquipment(ItemSlot),
    EquipVehicle(ItemSlot),
    DropMoney(usize),
    BankDepositItem(ItemSlot),
    BankWithdrawItem(usize),
    EnterRepairMode(ItemSlot), // Enter repair mode with the repair tool slot
    ExitRepairMode,            // Exit repair mode
    RepairItem(ItemSlot),      // Repair an equipment item
}
