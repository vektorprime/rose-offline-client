use std::time::Duration;

use bevy::{
    math::Vec3Swizzles,
    prelude::{Entity, Local, MessageReader, MessageWriter, Query, Res, With},
};

use rose_data::{
    AbilityType, AmmoIndex, EquipmentIndex, Item, ItemClass, ItemType, SkillBasicCommand,
    SkillTargetFilter, SkillType, VehiclePartIndex,
};
use rose_game_common::{
    components::{
        AbilityValues, CharacterInfo, Equipment, ExperiencePoints, GuildMembership, HealthPoints,
        Hotbar, HotbarSlot, Inventory, ItemDrop, ItemSlot, Level, ManaPoints, MoveMode, MoveSpeed,
        SkillList, SkillPoints, Stamina, StatPoints, Team, UnionMembership,
    },
    messages::client::ClientMessage,
};

use crate::{
    bundles::ability_values_get_value,
    components::{
        Bank, Clan, ClientEntity, ClientEntityType, Command, ConsumableCooldownGroup, Cooldowns,
        PartyInfo, PlayerCharacter, Position,
    },
    events::{ChatboxEvent, PlayerCommandEvent, QuestScrollEvent},
    resources::{GameConnection, GameData, SelectedTarget},
    ui::{
        attack_refusal, bank_has_space_for, inventory_has_space_for, is_cast_skill_type,
        skill_on_cooldown, skill_use_refusal, ChatFeedback, ChatFeedbackThrottle, SkillCaster,
        UiStateInventory,
    },
};

fn is_valid_skill_target(
    filter: SkillTargetFilter,
    target: (Entity, Option<&CharacterInfo>, &ClientEntity, &Command, &Team),
    player_entity: Entity,
    player_team_id: u32,
    player_party: Option<&PartyInfo>,
    player_clan: Option<&Clan>,
) -> bool {
    let (target_entity, target_character_info, target_client_entity, target_command, target_team) =
        target;
    let target_is_alive = !target_command.is_die();
    let target_is_caster = target_entity == player_entity;

    match filter {
        SkillTargetFilter::OnlySelf => target_is_alive && target_is_caster,
        SkillTargetFilter::Group => {
            target_is_alive
                && (target_is_caster
                    || player_party.map_or(false, |party_info| {
                        party_info.contains_member(target_client_entity.id)
                    }))
        }
        SkillTargetFilter::Guild => {
            target_is_alive
                && (target_is_caster
                    || target_character_info.map_or(false, |character_info| {
                        player_clan.map_or(false, |clan| {
                            clan.find_member(&character_info.name).is_some()
                        })
                    }))
        }
        SkillTargetFilter::Allied => target_is_alive && target_team.id == player_team_id,
        SkillTargetFilter::Monster => {
            target_is_alive
                && matches!(
                    target_client_entity.entity_type,
                    ClientEntityType::Monster
                )
        }
        SkillTargetFilter::Enemy => {
            target_is_alive
                && target_team.id != Team::DEFAULT_NPC_TEAM_ID
                && target_team.id != player_team_id
        }
        SkillTargetFilter::EnemyCharacter => {
            target_is_alive
                && target_team.id != player_team_id
                && matches!(
                    target_client_entity.entity_type,
                    ClientEntityType::Character
                )
        }
        SkillTargetFilter::Character => {
            target_is_alive
                && matches!(
                    target_client_entity.entity_type,
                    ClientEntityType::Character
                )
        }
        SkillTargetFilter::CharacterOrMonster => {
            target_is_alive
                && matches!(
                    target_client_entity.entity_type,
                    ClientEntityType::Character | ClientEntityType::Monster
                )
        }
        SkillTargetFilter::DeadAlliedCharacter => {
            !target_is_alive
                && target_team.id == player_team_id
                && matches!(
                    target_client_entity.entity_type,
                    ClientEntityType::Character
                )
        }
        SkillTargetFilter::EnemyMonster => {
            target_is_alive
                && target_team.id != player_team_id
                && matches!(
                    target_client_entity.entity_type,
                    ClientEntityType::Monster
                )
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn player_command_system(
    mut player_command_events: MessageReader<PlayerCommandEvent>,
    mut query_player: Query<
        (
            Entity,
            Option<&Bank>,
            &Cooldowns,
            &mut Hotbar,
            &Inventory,
            &Position,
            &SkillList,
            &Team,
            Option<&Clan>,
            Option<&PartyInfo>,
        ),
        With<PlayerCharacter>,
    >,
    query_client_entity: Query<&ClientEntity>,
    query_dropped_items: Query<(&ClientEntity, &Position), With<ItemDrop>>,
    query_team: Query<(
        &ClientEntity,
        &Team,
        Option<&HealthPoints>,
        Option<&Command>,
    )>,
    query_skill_target: Query<(
        Entity,
        Option<&CharacterInfo>,
        &ClientEntity,
        &Command,
        &Team,
    )>,
    query_player_stats: Query<
        (
            &AbilityValues,
            &CharacterInfo,
            &Equipment,
            &ExperiencePoints,
            &HealthPoints,
            &Level,
            &ManaPoints,
            &MoveMode,
            &MoveSpeed,
            &SkillPoints,
            &Stamina,
            &StatPoints,
            &UnionMembership,
            Option<&GuildMembership>,
        ),
        With<PlayerCharacter>,
    >,
    (mut chatbox_events, mut chat_feedback): (
        MessageWriter<ChatboxEvent>,
        Local<ChatFeedbackThrottle>,
    ),
    mut quest_scroll_events: MessageWriter<QuestScrollEvent>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    selected_target: Res<SelectedTarget>,
    ui_state_inventory: Option<Res<UiStateInventory>>,
) {
    let query_player_result = query_player.single_mut();
    if query_player_result.is_err() {
        return;
    }
    let (
        player_entity,
        player_bank,
        player_cooldowns,
        mut player_hotbar,
        player_inventory,
        player_position,
        player_skill_list,
        player_team,
        player_clan,
        player_party_info,
    ) = query_player_result.unwrap();

    // Resource checks below mirror the server's refusals (skill_use_requirements_met, attack,
    // store / bank / equipment transactions). Those are silent on the server, so a refused
    // request is reported in chat and not sent. Without these components no check is made and
    // the request goes to the server as before.
    let player_stats = query_player_stats.single().ok();
    let player_equipment = player_stats.map(|stats| stats.2);
    let player_move_mode = player_stats.map(|stats| stats.7);
    let skill_caster = player_stats.map(|stats| SkillCaster {
        ability_values: stats.0,
        equipment: stats.2,
        experience_points: stats.3,
        health_points: stats.4,
        inventory: player_inventory,
        mana_points: stats.6,
        move_mode: stats.7,
        stamina: stats.10,
    });
    let attack_check = player_equipment
        .zip(player_move_mode)
        .and_then(|(equipment, move_mode)| attack_refusal(&game_data, equipment, move_mode));

    for event in player_command_events.read() {
        let mut event = event.clone();

        if let PlayerCommandEvent::UseHotbar(page, index) = event {
            if let Some(hotbar_slot) = player_hotbar
                .pages
                .get(page)
                .and_then(|page| page.get(index))
                .and_then(|slot| slot.as_ref())
            {
                match hotbar_slot {
                    HotbarSlot::Skill(skill_slot) => {
                        event = PlayerCommandEvent::UseSkill(*skill_slot);
                    }
                    HotbarSlot::Inventory(item_slot) => {
                        event = PlayerCommandEvent::UseItem(*item_slot);
                    }
                    unimplemented => {
                        log::warn!("Unimplemented use hotbar slot {:?}", unimplemented);
                    }
                }
            }
        }

        match event {
            PlayerCommandEvent::UseSkill(skill_slot) => {
                if let Some(skill_data) = player_skill_list
                    .get_skill(skill_slot)
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                {
                    if skill_on_cooldown(player_cooldowns, skill_data) {
                        chat_feedback.send(
                            &mut chatbox_events,
                            ChatFeedback::Waiting.message(&game_data),
                        );
                        continue;
                    }

                    if is_cast_skill_type(skill_data.skill_type) {
                        if let Some(refusal) = skill_caster
                            .as_ref()
                            .and_then(|caster| skill_use_refusal(&game_data, caster, skill_data))
                        {
                            chat_feedback.send(&mut chatbox_events, refusal.message(&game_data));
                            continue;
                        }
                    }

                    match skill_data.skill_type {
                        SkillType::BasicAction => match &skill_data.basic_command {
                            Some(SkillBasicCommand::Sit) => {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::SitToggle)
                                        .ok();
                                }
                            }
                            Some(SkillBasicCommand::PickupItem) => {
                                let mut nearest_item_drop = None;

                                for (item_client_entity, item_position) in
                                    query_dropped_items.iter()
                                {
                                    let distance = item_position
                                        .position
                                        .xy()
                                        .distance_squared(player_position.xy());

                                    if nearest_item_drop
                                        .as_ref()
                                        .map_or(true, |(nearest_distance, _, _)| {
                                            distance < *nearest_distance
                                        })
                                    {
                                        nearest_item_drop =
                                            Some((distance, item_position, item_client_entity.id));
                                    }
                                }

                                if let Some((_, target_position, target_entity_id)) =
                                    nearest_item_drop
                                {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::Move {
                                                target_entity_id: Some(target_entity_id),
                                                x: target_position.x,
                                                y: target_position.y,
                                                z: target_position.z as u16,
                                            })
                                            .ok();
                                    }
                                }
                            }
                            Some(SkillBasicCommand::Attack) => {
                                if let Some(selected_target_entity) = selected_target.selected {
                                    if let Ok((target_client_entity, target_team, target_hp, target_command)) =
                                        query_team.get(selected_target_entity)
                                    {
                                        if target_team.id != Team::DEFAULT_NPC_TEAM_ID
                                            && target_team.id != player_team.id
                                        {
                                            // Don't send doomed attacks at corpses: the server
                                            // rejects hp<=0 targets and answers Stop, which
                                            // looks like "clicking does nothing".
                                            let target_dead = target_command.map_or(false, |c| c.is_die())
                                                || target_hp.map_or(false, |hp| hp.hp <= 0);
                                            if target_dead {
                                                chatbox_events.write(ChatboxEvent::System(
                                                    "Invalid target".to_string(),
                                                ));
                                            } else if let Some(refusal) = attack_check {
                                                chat_feedback.send(
                                                    &mut chatbox_events,
                                                    refusal.message(&game_data),
                                                );
                                            } else if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                game_connection
                                                    .client_message_tx
                                                    .send(ClientMessage::Attack {
                                                        target_entity_id: target_client_entity.id,
                                                    })
                                                    .ok();
                                            }
                                        }
                                    }
                                }
                            }
                            Some(SkillBasicCommand::Jump) | Some(SkillBasicCommand::AirJump) => {
                                if let Some(action_motion_id) = skill_data.action_motion_id {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::UseEmote {
                                                motion_id: action_motion_id,
                                                is_stop: true,
                                            })
                                            .ok();
                                    }
                                }
                            }
                            Some(SkillBasicCommand::PartyInvite) => {
                                if let Some(selected_target_entity) = selected_target.selected {
                                    if let Ok((target_client_entity, target_team, ..)) =
                                        query_team.get(selected_target_entity)
                                    {
                                        if target_team.id == player_team.id {
                                            if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                let message = if player_party_info.is_none() {
                                                    ClientMessage::PartyCreate {
                                                        invited_entity_id: target_client_entity.id,
                                                    }
                                                } else {
                                                    ClientMessage::PartyInvite {
                                                        invited_entity_id: target_client_entity.id,
                                                    }
                                                };

                                                game_connection
                                                    .client_message_tx
                                                    .send(message)
                                                    .ok();
                                            }
                                        }
                                    }
                                }
                            }
                            Some(SkillBasicCommand::DriveVehicle) => {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::DriveToggle)
                                        .ok();
                                }
                            }
                            Some(unimplemented) => {
                                log::warn!(
                                    "Unimplemented skill basic command type: {:?}",
                                    unimplemented
                                );
                            }
                            None => {}
                        },

                        SkillType::Emote => {
                            if let Some(motion_id) = skill_data.action_motion_id {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::UseEmote {
                                            motion_id,
                                            is_stop: true,
                                        })
                                        .ok();
                                }
                            }
                        }

                        SkillType::CreateWindow => {
                            log::warn!("Unimplemented skill type: {:?}", skill_data.skill_type);
                        }

                        SkillType::SelfBoundDuration
                        | SkillType::SelfBound
                        | SkillType::SelfStateDuration
                        | SkillType::SummonPet
                        | SkillType::SelfDamage => {
                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::CastSkillSelf { skill_slot })
                                    .ok();
                            }
                        }

                        SkillType::EnforceWeapon
                        | SkillType::Immediate
                        | SkillType::TargetBound
                        | SkillType::TargetBoundDuration
                        | SkillType::TargetStateDuration
                        | SkillType::SelfAndTarget
                        | SkillType::Resurrection
                        | SkillType::EnforceBullet
                        | SkillType::FireBullet
                        | SkillType::AreaTarget => {
                            let target_entity_id = {
                                if let Ok(target) = query_skill_target
                                    .get(selected_target.selected.unwrap_or(player_entity))
                                {
                                    if is_valid_skill_target(
                                        skill_data.target_filter,
                                        target,
                                        player_entity,
                                        player_team.id,
                                        player_party_info,
                                        player_clan,
                                    ) {
                                        Some(target.2.id)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };

                            if let Some(target_entity_id) = target_entity_id {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::CastSkillTargetEntity {
                                            skill_slot,
                                            target_entity_id,
                                        })
                                        .ok();
                                }
                            } else {
                                chatbox_events
                                    .write(ChatboxEvent::System("Invalid target".to_string()));
                                continue;
                            }
                        }

                        SkillType::Passive => {} // Do nothing for passive skills
                        SkillType::Warp => {} // Warp skill is only used on items, so we should never hit it here
                    }
                }
            }
            PlayerCommandEvent::UseItem(item_slot) => {
                if let Some(item) = player_inventory.get_item(item_slot) {
                    if item.get_item_type() == ItemType::Consumable {
                        let consumable_item_data =
                            game_data.items.get_consumable_item(item.get_item_number());
                        let mut use_item_target = None;

                        if let Some(consumable_item_data) = consumable_item_data {
                            let cooldown_group = ConsumableCooldownGroup::from_item(
                                &item.get_item_reference(),
                                &game_data,
                            );
                            let cooldown_duration = match cooldown_group {
                                Some(ConsumableCooldownGroup::MagicItem) => {
                                    Some(Duration::from_millis(3000))
                                }
                                Some(_) => Some(Duration::from_millis(500)),
                                None => None,
                            };

                            // If item is a repair item, enter repair mode
                            if matches!(consumable_item_data.item_data.class, ItemClass::RepairTool)
                            {
                                // Enter repair mode with the repair tool slot
                                // The actual repair will be triggered when an equipment item is clicked
                                chatbox_events.write(ChatboxEvent::System(
                                    "Select an equipment item to repair".to_string(),
                                ));
                                // Note: The repair mode is handled in the UI system
                                // When an equipment item is clicked while in repair mode,
                                // it will send a RepairItem event
                                continue;
                            }

                            if matches!(
                                consumable_item_data.item_data.class,
                                ItemClass::QuestScroll
                            ) {
                                // QuestScroll items trigger quests when used.
                                // Show a dialog to confirm before triggering the quest.
                                log::info!(
                                    "QuestScroll item slot {:?} used, confile_index: {}",
                                    item_slot,
                                    consumable_item_data.confile_index
                                );

                                // The quest trigger name is derived from the confile_index
                                // For now, we use the confile_index as the trigger name
                                // In a full implementation, this would look up the actual quest trigger
                                let quest_trigger = consumable_item_data.confile_index.to_string();

                                // Dispatch event to show the quest scroll dialog
                                quest_scroll_events.write(QuestScrollEvent::Show {
                                    item_slot,
                                    quest_trigger,
                                });
                                continue;
                            }

                            // Check if item is on cooldown
                            if cooldown_group
                                .and_then(|cooldown_group| {
                                    player_cooldowns.get_consumable_cooldown_percent(cooldown_group)
                                })
                                .is_some()
                            {
                                chat_feedback.send(
                                    &mut chatbox_events,
                                    ChatFeedback::Waiting.message(&game_data),
                                );
                                continue;
                            }

                            // Server use_item_system: the item's ability requirement. Planet
                            // requirements (warp scrolls) are left to the server, which knows
                            // the planet of the current zone.
                            if let (
                                Some((require_ability_type, require_ability_value)),
                                Some(stats),
                            ) = (consumable_item_data.ability_requirement, player_stats)
                            {
                                let ability_value = ability_values_get_value(
                                    require_ability_type,
                                    stats.0,
                                    Some(stats.1),
                                    Some(stats.3),
                                    stats.13,
                                    Some(stats.4),
                                    Some(player_inventory),
                                    Some(stats.5),
                                    Some(stats.6),
                                    Some(stats.8),
                                    Some(stats.9),
                                    Some(stats.10),
                                    Some(stats.11),
                                    Some(player_team),
                                    Some(stats.12),
                                )
                                .unwrap_or(0);

                                if !matches!(require_ability_type, AbilityType::CurrentPlanet)
                                    && ability_value < require_ability_value
                                {
                                    chat_feedback.send(
                                        &mut chatbox_events,
                                        ChatFeedback::ItemRequirements.message(&game_data),
                                    );
                                    continue;
                                }
                            }

                            // Check if consumable requires a target
                            if matches!(consumable_item_data.item_data.class, ItemClass::MagicItem)
                            {
                                if let Some(skill_data) = consumable_item_data
                                    .use_skill_id
                                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                                {
                                    if matches!(
                                        skill_data.skill_type,
                                        SkillType::FireBullet
                                            | SkillType::TargetBoundDuration
                                            | SkillType::TargetBound
                                            | SkillType::TargetStateDuration
                                    ) {
                                        // Validate target using the same logic as skills
                                        let is_valid_target = if let Some(target_entity) =
                                            selected_target.selected
                                        {
                                            if let Ok(target) = query_skill_target.get(target_entity)
                                            {
                                                is_valid_skill_target(
                                                    skill_data.target_filter,
                                                    target,
                                                    player_entity,
                                                    player_team.id,
                                                    player_party_info,
                                                    player_clan,
                                                )
                                            } else {
                                                false
                                            }
                                        } else {
                                            false // No target selected
                                        };

                                        if is_valid_target {
                                            use_item_target =
                                                selected_target.selected.and_then(|e| {
                                                    query_client_entity.get(e).ok().map(|ce| ce.id)
                                                });
                                        } else {
                                            chatbox_events.write(ChatboxEvent::System(
                                                "Invalid target".to_string(),
                                            ));
                                            continue;
                                        }
                                    }

                                    // A scroll whose skill the server casts (use_item_system)
                                    // goes through the same cast checks as a normal skill.
                                    let casts_skill = skill_data.skill_type.is_self_skill()
                                        || matches!(
                                            skill_data.skill_type,
                                            SkillType::Immediate | SkillType::AreaTarget
                                        )
                                        || (skill_data.skill_type.is_target_skill()
                                            && use_item_target.is_some());
                                    if casts_skill {
                                        if skill_on_cooldown(player_cooldowns, skill_data) {
                                            chat_feedback.send(
                                                &mut chatbox_events,
                                                ChatFeedback::Waiting.message(&game_data),
                                            );
                                            continue;
                                        }

                                        if let Some(refusal) =
                                            skill_caster.as_ref().and_then(|caster| {
                                                skill_use_refusal(&game_data, caster, skill_data)
                                            })
                                        {
                                            chat_feedback.send(
                                                &mut chatbox_events,
                                                refusal.message(&game_data),
                                            );
                                            continue;
                                        }
                                    }
                                }
                            }

                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::UseItem {
                                        item_slot,
                                        target_entity_id: use_item_target,
                                    })
                                    .ok();
                            }
                        }
                    } else if item.get_item_type().is_equipment_item() {
                        // TODO: Equip item
                    }
                } else if matches!(item_slot, ItemSlot::Inventory(..)) {
                    // A hotbar slot whose stack has been used up.
                    chat_feedback.send(
                        &mut chatbox_events,
                        ChatFeedback::ItemUsedUp.message(&game_data),
                    );
                }
            }
            PlayerCommandEvent::EquipAmmo(item_slot) => {
                if let Some(item) = player_inventory.get_item(item_slot) {
                    let ammo_index = if let Some(item_data) =
                        game_data.items.get_base_item(item.get_item_reference())
                    {
                        match item_data.class {
                            ItemClass::Arrow => Some(AmmoIndex::Arrow),
                            ItemClass::Bullet => Some(AmmoIndex::Bullet),
                            ItemClass::Shell => Some(AmmoIndex::Throw),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    if let Some(ammo_index) = ammo_index {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::ChangeAmmo {
                                    ammo_index,
                                    item_slot: Some(item_slot),
                                })
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::EquipEquipment(item_slot) => {
                if let Some(item) = player_inventory.get_item(item_slot) {
                    let equipment_index = match item.get_item_type() {
                        ItemType::Face => Some(EquipmentIndex::Face),
                        ItemType::Head => Some(EquipmentIndex::Head),
                        ItemType::Body => Some(EquipmentIndex::Body),
                        ItemType::Hands => Some(EquipmentIndex::Hands),
                        ItemType::Feet => Some(EquipmentIndex::Feet),
                        ItemType::Back => Some(EquipmentIndex::Back),
                        ItemType::Jewellery => {
                            if let Some(jewellery_item) =
                                game_data.items.get_jewellery_item(item.get_item_number())
                            {
                                match jewellery_item.item_data.class {
                                    ItemClass::Ring => Some(EquipmentIndex::Ring),
                                    ItemClass::Necklace => Some(EquipmentIndex::Necklace),
                                    ItemClass::Earring => Some(EquipmentIndex::Earring),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        }
                        ItemType::Weapon => Some(EquipmentIndex::Weapon),
                        ItemType::SubWeapon => Some(EquipmentIndex::SubWeapon),
                        _ => None,
                    };

                    if let Some(equipment_index) = equipment_index {
                        // A two-handed weapon moves the equipped sub weapon into the inventory;
                        // the server refuses the swap when it does not fit.
                        let sub_weapon_does_not_fit =
                            matches!(equipment_index, EquipmentIndex::Weapon)
                                && game_data
                                    .items
                                    .get_base_item(item.get_item_reference())
                                    .map_or(false, |item_data| {
                                        item_data.class.is_two_handed_weapon()
                                    })
                                && player_equipment
                                    .and_then(|equipment| {
                                        equipment.get_equipment_item(EquipmentIndex::SubWeapon)
                                    })
                                    .map_or(false, |sub_weapon| {
                                        !inventory_has_space_for(
                                            player_inventory,
                                            Item::Equipment(sub_weapon.clone()),
                                        )
                                    });
                        if sub_weapon_does_not_fit {
                            chat_feedback.send(
                                &mut chatbox_events,
                                ChatFeedback::EquipmentSpace.message(&game_data),
                            );
                            continue;
                        }

                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::ChangeEquipment {
                                    equipment_index,
                                    item_slot: Some(item_slot),
                                })
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::EquipVehicle(item_slot) => {
                if let Some(item) = player_inventory.get_item(item_slot) {
                    let vehicle_part_index = if let Some(item_data) =
                        game_data.items.get_base_item(item.get_item_reference())
                    {
                        match item_data.class {
                            ItemClass::CartBody | ItemClass::CastleGearBody => {
                                Some(VehiclePartIndex::Body)
                            }
                            ItemClass::CartEngine | ItemClass::CastleGearEngine => {
                                Some(VehiclePartIndex::Engine)
                            }
                            ItemClass::CartWheels | ItemClass::CastleGearLeg => {
                                Some(VehiclePartIndex::Leg)
                            }
                            ItemClass::CartAccessory | ItemClass::CastleGearWeapon => {
                                Some(VehiclePartIndex::Arms)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };

                    if let Some(vehicle_part_index) = vehicle_part_index {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::ChangeVehiclePart {
                                    vehicle_part_index,
                                    item_slot: Some(item_slot),
                                })
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::UnequipAmmo(ammo_index) => {
                // Unequipped items go back into the inventory; the server keeps them equipped
                // when they do not fit.
                let unequipped_item = player_equipment
                    .and_then(|equipment| equipment.get_ammo_item(ammo_index))
                    .map(|ammo_item| Item::Stackable(ammo_item.clone()));
                if unequipped_item.map_or(false, |item| {
                    !inventory_has_space_for(player_inventory, item)
                }) {
                    chat_feedback.send(
                        &mut chatbox_events,
                        ChatFeedback::InventoryFull.message(&game_data),
                    );
                    continue;
                }

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeAmmo {
                            ammo_index,
                            item_slot: None,
                        })
                        .ok();
                }
            }
            PlayerCommandEvent::UnequipEquipment(equipment_index) => {
                let unequipped_item = player_equipment
                    .and_then(|equipment| equipment.get_equipment_item(equipment_index))
                    .map(|equipment_item| Item::Equipment(equipment_item.clone()));
                if unequipped_item.map_or(false, |item| {
                    !inventory_has_space_for(player_inventory, item)
                }) {
                    chat_feedback.send(
                        &mut chatbox_events,
                        ChatFeedback::InventoryFull.message(&game_data),
                    );
                    continue;
                }

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeEquipment {
                            equipment_index,
                            item_slot: None,
                        })
                        .ok();
                }
            }
            PlayerCommandEvent::UnequipVehicle(vehicle_part_index) => {
                let unequipped_item = player_equipment
                    .and_then(|equipment| equipment.get_vehicle_item(vehicle_part_index))
                    .map(|vehicle_item| Item::Equipment(vehicle_item.clone()));
                if unequipped_item.map_or(false, |item| {
                    !inventory_has_space_for(player_inventory, item)
                }) {
                    chat_feedback.send(
                        &mut chatbox_events,
                        ChatFeedback::InventoryFull.message(&game_data),
                    );
                    continue;
                }

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeVehiclePart {
                            vehicle_part_index,
                            item_slot: None,
                        })
                        .ok();
                }
            }
            PlayerCommandEvent::DropItem(item_slot) => {
                if let Some(item) = player_inventory.get_item(item_slot) {
                    // TODO: if item.get_quantity() > 1, show number input dialog for quantity
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::DropItem {
                                item_slot,
                                quantity: item.get_quantity() as usize,
                            })
                            .ok();
                    }
                }
            }
            PlayerCommandEvent::DropItemWithQuantity(item_slot, quantity) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::DropItem {
                            item_slot,
                            quantity,
                        })
                        .ok();
                }
            }
            PlayerCommandEvent::DropMoney(quantity) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::DropMoney { quantity })
                        .ok();
                }
            }
            PlayerCommandEvent::Attack(entity) => {
                if let Ok((target_client_entity, target_team, target_hp, target_command)) =
                    query_team.get(entity)
                {
                    if target_team.id != Team::DEFAULT_NPC_TEAM_ID
                        && target_team.id != player_team.id
                    {
                        let target_dead = target_command.map_or(false, |c| c.is_die())
                            || target_hp.map_or(false, |hp| hp.hp <= 0);
                        if target_dead {
                            chatbox_events
                                .write(ChatboxEvent::System("Invalid target".to_string()));
                        } else if let Some(refusal) = attack_check {
                            chat_feedback.send(&mut chatbox_events, refusal.message(&game_data));
                        } else if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::Attack {
                                    target_entity_id: target_client_entity.id,
                                })
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::Move(position, target_entity) => {
                let target_entity_id = target_entity
                    .and_then(|target_entity| query_client_entity.get(target_entity).ok())
                    .map(|target_client_entity| target_client_entity.id);

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::Move {
                            target_entity_id,
                            x: position.x,
                            y: position.y,
                            z: position.z as u16,
                        })
                        .ok();
                } else {
                    log::warn!("[RESPAWN_MOVE_DIAG] No game connection available!");
                }
            }
            PlayerCommandEvent::SetHotbar(page, page_index, hotbar_slot) => {
                if let Some(hotbar_page) = player_hotbar.pages.get_mut(page) {
                    if let Some(hotbar_page_slot) = hotbar_page.get_mut(page_index) {
                        *hotbar_page_slot = hotbar_slot.clone();
                    }
                }

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::SetHotbarSlot {
                            slot_index: page * player_hotbar.pages[0].len() + page_index,
                            slot: hotbar_slot,
                        })
                        .ok();
                }
            }
            PlayerCommandEvent::BankDepositItem(item_slot) => {
                if let Some(item) = player_inventory.get_item(item_slot) {
                    // The server drops a deposit that does not fit without a reply.
                    if player_bank.map_or(false, |bank| !bank_has_space_for(bank, item)) {
                        chat_feedback.send(
                            &mut chatbox_events,
                            ChatFeedback::StorageFull.message(&game_data),
                        );
                        continue;
                    }

                    // TODO: if item.get_quantity() > 1, show number input dialog for quantity
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::BankDepositItem {
                                item_slot,
                                item: item.clone(),
                                is_premium: false,
                            })
                            .ok();
                    }
                }
            }
            PlayerCommandEvent::BankWithdrawItem(bank_slot) => {
                if let Some(item) = player_bank
                    .and_then(|bank| bank.slots.get(bank_slot))
                    .and_then(|x| x.as_ref())
                {
                    // The server drops a withdrawal that does not fit without a reply.
                    if !inventory_has_space_for(player_inventory, item.clone()) {
                        chat_feedback.send(
                            &mut chatbox_events,
                            ChatFeedback::InventoryFull.message(&game_data),
                        );
                        continue;
                    }

                    // TODO: if item.get_quantity() > 1, show number input dialog for quantity
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::BankWithdrawItem {
                                bank_slot,
                                item: item.clone(),
                                is_premium: false,
                            })
                            .ok();
                    }
                }
            }
            PlayerCommandEvent::LevelUpSkill(skill_slot) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::LevelUpSkill { skill_slot })
                        .ok();
                }
            }
            PlayerCommandEvent::EnterRepairMode(_) => {
                // Repair mode is handled in the UI system
                // This event is sent when a repair tool is used
                // The UI system will track the repair mode state
            }
            PlayerCommandEvent::ExitRepairMode => {
                // Repair mode is handled in the UI system
                // This event is sent when repair mode should be exited
            }
            PlayerCommandEvent::RepairItem(item_slot) => {
                // Send repair request to server
                // The repair tool slot is stored in the UI state (repair_mode)
                if let Some(game_connection) = game_connection.as_ref() {
                    if let Some(ui_state) = ui_state_inventory.as_ref() {
                        if let Some(repair_tool_slot) = ui_state.repair_mode {
                            // Send repair request to server
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::RepairItemUsingItem {
                                    use_item_slot: repair_tool_slot,
                                    item_slot,
                                })
                                .ok();
                        } else {
                            chatbox_events
                                .write(ChatboxEvent::System("No repair tool selected".to_string()));
                        }
                    }
                }
            }
            PlayerCommandEvent::UseHotbar(_, _) => {} // Handled above
        }
    }
}
