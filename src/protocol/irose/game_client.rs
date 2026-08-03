use async_trait::async_trait;
use bevy::math::Vec3;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_data::{QuestTriggerHash, SkillId};
use rose_game_common::{
    components::MoveMode,
    messages::{
        client::ClientMessage,
        server::{
            CharacterData, CharacterDataItems, ConnectionRequestError, ServerMessage,
            SpawnEntityCharacter,
        },
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    game_client_packets::{
        PacketClientAttack, PacketClientBankMoveItem, PacketClientBankOpen,
        PacketClientCastSkillSelf, PacketClientCastSkillTargetEntity,
        PacketClientCastSkillTargetPosition, PacketClientChangeAmmo, PacketClientChangeEquipment,
        PacketClientChangeVehiclePart, PacketClientChat, PacketClientClanCommand,
        PacketClientConnectRequest, PacketClientCraftItem, PacketClientDropItemFromInventory,
        PacketClientEmote, PacketClientIncreaseBasicStat, PacketClientJoinZone,
        PacketClientLevelUpSkill, PacketClientMove, PacketClientMoveCollision,
        PacketClientMoveToggle, PacketClientMoveToggleType, PacketClientNpcStoreTransaction,
        PacketClientPartyReply, PacketClientPartyRequest, PacketClientPartyUpdateRules,
        PacketClientPersonalStoreBuyItem, PacketClientPersonalStoreListItems,
        PacketClientPickupItemDrop, PacketClientQuestRequest, PacketClientQuestRequestType,
        PacketClientRepairItemUsingItem, PacketClientRepairItemUsingNpc, PacketClientReviveRequest,
        PacketClientSailInput, PacketClientSetHotbarSlot, PacketClientSetReviveZone,
        PacketClientUseItem, PacketClientWarpGateRequest, PacketClientBoardBoat,
        PacketClientDisembarkBoat, encode_input_i8,
    },
    game_server_packets::{
        ConnectResult, PacketConnectionReply, PacketServerAdjustPosition, PacketServerAnnounceChat,
        PacketServerApplySkillDamage, PacketServerApplySkillEffect, PacketServerAttackEntity,
        PacketServerBankOpen, PacketServerBankTransaction, PacketServerCancelCastingSkill,
        PacketServerCastSkillSelf, PacketServerCastSkillTargetEntity,
        PacketServerCastSkillTargetPosition, PacketServerChangeNpcId,
        PacketServerCharacterInventory, PacketServerCharacterQuestData, PacketServerClanCommand,
        PacketServerClosePersonalStore, PacketServerCraftItem, PacketServerDamageEntity,
        PacketServerFinishCastingSkill, PacketServerJoinZone, PacketServerLearnSkillResult,
        PacketServerLevelUpSkillResult, PacketServerLocalChat, PacketServerLogoutResult,
        PacketServerMoveEntity, PacketServerMoveToggle, PacketServerMoveToggleType,
        PacketServerNpcStoreTransactionError, PacketServerOpenPersonalStore,
        PacketServerPartyMemberRewardItem, PacketServerPartyMemberUpdateInfo,
        PacketServerPartyMembers, PacketServerPartyReply, PacketServerPartyRequest,
        PacketServerPartyUpdateRules, PacketServerPersonalStoreItemList,
        PacketServerPersonalStoreTransactionResult,
        PacketServerPersonalStoreTransactionUpdateMoneyAndInventory,
        PacketServerPickupItemDropResult, PacketServerQuestResult, PacketServerQuestResultType,
        PacketServerRemoveEntities, PacketServerRepairedItemUsingNpc, PacketServerRewardItems,
        PacketServerRewardMoney, PacketServerRunNpcDeathTrigger, PacketServerSelectCharacter,
        PacketServerSetHotbarSlot, PacketServerShoutChat, PacketServerSpawnEntityCharacter,
        PacketServerSpawnEntityItemDrop, PacketServerSpawnEntityMonster,
        PacketServerSpawnEntityNpc, PacketServerStartCastingSkill, PacketServerStopMoveEntity,
        PacketServerTeleport, PacketServerUpdateAbilityValue, PacketServerUpdateAbilityValues,
        PacketServerUpdateAmmo, PacketServerUpdateBasicStat, PacketServerUpdateConsumableCooldown,
        PacketServerUpdateCooldown, PacketServerUpdateEquipment, PacketServerUpdateInventory,
        PacketServerUpdateItemLife, PacketServerUpdateLevel, PacketServerUpdateMoney,
        PacketServerUpdateSpeed, PacketServerUpdateStatusEffects, PacketServerUpdateVehiclePart,
        PacketServerUpdateXpStamina, PacketServerUseEmote, PacketServerUseItem,
        PacketServerWhisper, PacketServerSailState, PacketServerWindState, ServerPackets,
    },
    ClientPacketCodec, IROSE_112_TABLE,
};

use crate::protocol::{ProtocolClient, ProtocolClientError};

pub struct GameClient {
    server_address: SocketAddr,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
}

impl GameClient {
    pub fn new(
        server_address: SocketAddr,
        packet_codec_seed: u32,
        client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
        server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    ) -> Self {
        Self {
            server_address,
            client_message_rx,
            server_message_tx,
            packet_codec: Box::new(ClientPacketCodec::init(&IROSE_112_TABLE, packet_codec_seed)),
        }
    }

    async fn handle_packet(&self, packet: &Packet) -> Result<(), anyhow::Error> {
        macro_rules! server_message {
            ($packet_type:ident, $message:ident { $( $field:ident ),+ $(,)? }) => {
                let message = $packet_type::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::$message {
                        $( $field: message.$field ),+
                    })
                    .ok();
            };
        }

        match FromPrimitive::from_u16(packet.command) {
            Some(ServerPackets::ConnectReply) => {
                let response = PacketConnectionReply::try_from(packet)?;
                let message = match response.result {
                    ConnectResult::Ok => ServerMessage::ConnectionRequestSuccess {
                        packet_sequence_id: response.packet_sequence_id,
                    },
                    ConnectResult::InvalidPassword => ServerMessage::ConnectionRequestError {
                        error: ConnectionRequestError::InvalidPassword,
                    },
                    _ => ServerMessage::ConnectionRequestError {
                        error: ConnectionRequestError::Failed,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::SelectCharacter) => {
                let response = PacketServerSelectCharacter::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterData {
                        data: Box::new(CharacterData {
                            character_info: response.character_info,
                            position: response.position,
                            zone_id: response.zone_id,
                            basic_stats: response.basic_stats,
                            level: response.level,
                            equipment: response.equipment,
                            experience_points: response.experience_points,
                            skill_list: response.skill_list,
                            hotbar: response.hotbar,
                            health_points: response.health_points,
                            mana_points: response.mana_points,
                            stat_points: response.stat_points,
                            skill_points: response.skill_points,
                            union_membership: response.union_membership,
                            stamina: response.stamina,
                        }),
                    })
                    .ok();
            }
            Some(ServerPackets::CharacterInventory) => {
                let response = PacketServerCharacterInventory::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataItems {
                        data: Box::new(CharacterDataItems {
                            inventory: response.inventory,
                            equipment: response.equipment,
                        }),
                    })
                    .ok();
            }
            Some(ServerPackets::QuestData) => {
                let response = PacketServerCharacterQuestData::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataQuest {
                        quest_state: Box::new(response.quest_state),
                    })
                    .ok();
            }
            Some(ServerPackets::JoinZone) => {
                let response = PacketServerJoinZone::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::JoinZone {
                        entity_id: response.entity_id,
                        experience_points: response.experience_points,
                        team: response.team,
                        health_points: response.health_points,
                        mana_points: response.mana_points,
                        world_ticks: response.world_ticks,
                        craft_rate: response.craft_rate,
                        world_price_rate: response.world_price_rate,
                        item_price_rate: response.item_price_rate,
                        town_price_rate: response.town_price_rate,
                    })
                    .ok();
            }
            Some(ServerPackets::MoveEntity) | Some(ServerPackets::MoveEntityWithMoveMode) => {
                server_message!(
                    PacketServerMoveEntity,
                    MoveEntity { entity_id, target_entity_id, distance, x, y, z, move_mode }
                );
            }
            Some(ServerPackets::StopMoveEntity) => {
                server_message!(PacketServerStopMoveEntity, StopMoveEntity { entity_id, x, y, z });
            }
            Some(ServerPackets::AttackEntity) => {
                server_message!(
                    PacketServerAttackEntity,
                    AttackEntity { entity_id, target_entity_id, distance, x, y, z }
                );
            }
            Some(ServerPackets::PickupItemDropResult) => {
                let message = match PacketServerPickupItemDropResult::try_from(packet)? {
                    PacketServerPickupItemDropResult::Item {
                        drop_entity_id,
                        item_slot,
                        item,
                    } => ServerMessage::PickupDropItem {
                        drop_entity_id,
                        item_slot,
                        item,
                    },
                    PacketServerPickupItemDropResult::Money {
                        drop_entity_id,
                        money,
                    } => ServerMessage::PickupDropMoney {
                        drop_entity_id,
                        money,
                    },
                    PacketServerPickupItemDropResult::Error {
                        drop_entity_id,
                        error,
                    } => ServerMessage::PickupDropError {
                        drop_entity_id,
                        error,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::SpawnEntityCharacter) => {
                let message = PacketServerSpawnEntityCharacter::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityCharacter {
                        data: Box::new(SpawnEntityCharacter {
                            entity_id: message.entity_id,
                            position: message.position,
                            team: message.team,
                            health: message.health,
                            spawn_command_state: message.spawn_command_state,
                            move_mode: message.move_mode,
                            status_effects: message.status_effects,
                            character_info: message.character_info,
                            equipment: message.equipment,
                            level: message.level,
                            move_speed: message.move_speed,
                            passive_attack_speed: message.passive_attack_speed,
                            personal_store_info: message.personal_store_info,
                            clan_membership: message.clan_membership,
                        }),
                    })
                    .ok();
            }
            Some(ServerPackets::SpawnEntityNpc) => {
                server_message!(
                    PacketServerSpawnEntityNpc,
                    SpawnEntityNpc {
                        entity_id,
                        npc,
                        direction,
                        position,
                        team,
                        health,
                        spawn_command_state,
                        move_mode,
                        status_effects
                    }
                );
            }
            Some(ServerPackets::SpawnEntityMonster) => {
                server_message!(
                    PacketServerSpawnEntityMonster,
                    SpawnEntityMonster {
                        entity_id,
                        npc,
                        position,
                        team,
                        health,
                        spawn_command_state,
                        move_mode,
                        status_effects
                    }
                );
            }
            Some(ServerPackets::SpawnEntityItemDrop) => {
                server_message!(
                    PacketServerSpawnEntityItemDrop,
                    SpawnEntityItemDrop {
                        entity_id,
                        position,
                        dropped_item,
                        remaining_time,
                        owner_entity_id
                    }
                );
            }
            Some(ServerPackets::DamageEntity) => {
                let message = PacketServerDamageEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity {
                        attacker_entity_id: message.attacker_entity_id,
                        defender_entity_id: message.defender_entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        is_immediate: message.is_immediate,
                        from_skill: None,
                    })
                    .ok();
            }
            Some(ServerPackets::RemoveEntities) => {
                server_message!(PacketServerRemoveEntities, RemoveEntities { entity_ids });
            }
            Some(ServerPackets::Teleport) => {
                server_message!(
                    PacketServerTeleport,
                    Teleport { entity_id, zone_id, x, y, run_mode, ride_mode }
                );
            }
            Some(ServerPackets::LocalChat) => {
                let message = PacketServerLocalChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::LocalChat {
                        entity_id: message.entity_id,
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::ShoutChat) => {
                let message = PacketServerShoutChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ShoutChat {
                        name: message.name.to_string(),
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::AnnounceChat) => {
                let message = PacketServerAnnounceChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::AnnounceChat {
                        name: message.name.map(|x| x.to_string()),
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::Whisper) => {
                let message = PacketServerWhisper::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::Whisper {
                        from: message.from.to_string(),
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateAmmo) => {
                server_message!(
                    PacketServerUpdateAmmo,
                    UpdateAmmo { entity_id, ammo_index, item }
                );
            }
            Some(ServerPackets::UpdateEquipment) => {
                server_message!(
                    PacketServerUpdateEquipment,
                    UpdateEquipment { entity_id, equipment_index, item }
                );
            }
            Some(ServerPackets::UpdateInventory) | Some(ServerPackets::UpdateMoneyAndInventory) => {
                let PacketServerUpdateInventory {
                    items,
                    with_money: money,
                } = packet.try_into()?;
                self.server_message_tx
                    .send(ServerMessage::UpdateInventory { items, money })
                    .ok();
            }
            Some(ServerPackets::UpdateMoney) => {
                server_message!(PacketServerUpdateMoney, UpdateMoney { money });
            }
            Some(ServerPackets::UpdateVehiclePart) => {
                server_message!(
                    PacketServerUpdateVehiclePart,
                    UpdateVehiclePart { entity_id, vehicle_part_index, item }
                );
            }
            Some(ServerPackets::UpdateItemLife) => {
                server_message!(PacketServerUpdateItemLife, UpdateItemLife { item_slot, life });
            }
            Some(ServerPackets::UpdateBasicStat) => {
                server_message!(
                    PacketServerUpdateBasicStat,
                    UpdateBasicStat { basic_stat_type, value }
                );
            }
            Some(ServerPackets::UpdateAbilityValueRewardAdd)
            | Some(ServerPackets::UpdateAbilityValueRewardSet) => {
                let message = PacketServerUpdateAbilityValue::try_from(packet)?;
                if message.is_add {
                    self.server_message_tx
                        .send(ServerMessage::UpdateAbilityValueAdd {
                            ability_type: message.ability_type,
                            value: message.value,
                        })
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::UpdateAbilityValueSet {
                            ability_type: message.ability_type,
                            value: message.value,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::UpdateLevel) => {
                let message = PacketServerUpdateLevel::try_from(packet)?;
                if let Some((level, experience_points, stat_points, skill_points)) =
                    message.update_values
                {
                    self.server_message_tx
                        .send(ServerMessage::UpdateLevel {
                            entity_id: message.entity_id,
                            level,
                            experience_points,
                            stat_points,
                            skill_points,
                        })
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::LevelUpEntity {
                            entity_id: message.entity_id,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::UpdateSpeed) => {
                server_message!(
                    PacketServerUpdateSpeed,
                    UpdateSpeed {
                        entity_id,
                        run_speed,
                        passive_attack_speed
                    }
                );
            }
            Some(ServerPackets::UpdateStatusEffects) => {
                let message = PacketServerUpdateStatusEffects::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateStatusEffects {
                        entity_id: message.entity_id,
                        status_effects: message.status_effects,
                        updated_values: message.updated_values,
                        regen_effects: message.regen_effects.regens,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateXpStamina) => {
                server_message!(
                    PacketServerUpdateXpStamina,
                    UpdateXpStamina { xp, stamina, source_entity_id }
                );
            }
            Some(ServerPackets::QuestResult) => {
                let message = PacketServerQuestResult::try_from(packet)?;
                match message.result {
                    PacketServerQuestResultType::DeleteSuccess => {
                        self.server_message_tx
                            .send(ServerMessage::QuestDeleteResult {
                                success: true,
                                slot: message.slot as usize,
                                quest_id: message.quest_id as usize,
                            })
                            .ok();
                    }
                    PacketServerQuestResultType::DeleteFailed => {
                        self.server_message_tx
                            .send(ServerMessage::QuestDeleteResult {
                                success: false,
                                slot: message.slot as usize,
                                quest_id: message.quest_id as usize,
                            })
                            .ok();
                    }
                    PacketServerQuestResultType::TriggerSuccess => {
                        self.server_message_tx
                            .send(ServerMessage::QuestTriggerResult {
                                success: true,
                                trigger_hash: QuestTriggerHash {
                                    hash: message.quest_id,
                                },
                            })
                            .ok();
                    }
                    PacketServerQuestResultType::TriggerFailed => {
                        self.server_message_tx
                            .send(ServerMessage::QuestTriggerResult {
                                success: false,
                                trigger_hash: QuestTriggerHash {
                                    hash: message.quest_id,
                                },
                            })
                            .ok();
                    }
                    _ => {}
                }
            }
            Some(ServerPackets::RunNpcDeathTrigger) => {
                server_message!(PacketServerRunNpcDeathTrigger, RunNpcDeathTrigger { npc_id });
            }
            Some(ServerPackets::RewardMoney) => {
                server_message!(PacketServerRewardMoney, RewardMoney { money });
            }
            Some(ServerPackets::RewardItems) => {
                server_message!(PacketServerRewardItems, RewardItems { items });
            }
            Some(ServerPackets::SetHotbarSlot) => {
                server_message!(
                    PacketServerSetHotbarSlot,
                    SetHotbarSlot { slot_index, slot }
                );
            }
            Some(ServerPackets::LearnSkillResult) => {
                let message = match PacketServerLearnSkillResult::try_from(packet)? {
                    PacketServerLearnSkillResult::Success {
                        skill_slot,
                        skill_id,
                        updated_skill_points,
                    } => ServerMessage::LearnSkillSuccess {
                        skill_slot,
                        skill_id,
                        updated_skill_points,
                    },
                    PacketServerLearnSkillResult::Error { error } => {
                        ServerMessage::LearnSkillError { error }
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::LevelUpSkillResult) => {
                let message = match PacketServerLevelUpSkillResult::try_from(packet)? {
                    PacketServerLevelUpSkillResult::Success {
                        skill_slot,
                        skill_id,
                        skill_points,
                    } => ServerMessage::LevelUpSkillSuccess {
                        skill_slot,
                        skill_id,
                        skill_points,
                    },
                    PacketServerLevelUpSkillResult::Error {
                        error,
                        skill_points,
                    } => ServerMessage::LevelUpSkillError {
                        error,
                        skill_points,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::UseEmote) => {
                server_message!(
                    PacketServerUseEmote,
                    UseEmote { entity_id, motion_id, is_stop }
                );
            }
            Some(ServerPackets::UseItem) => {
                let message = PacketServerUseItem::try_from(packet)?;
                if let Some(inventory_slot) = message.inventory_slot {
                    self.server_message_tx
                        .send(ServerMessage::UseInventoryItem {
                            entity_id: message.entity_id,
                            item: message.item,
                            inventory_slot,
                        })
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::UseItem {
                            entity_id: message.entity_id,
                            item: message.item,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::ChangeNpcId) => {
                server_message!(PacketServerChangeNpcId, ChangeNpcId { entity_id, npc_id });
            }
            Some(ServerPackets::CastSkillSelf) => {
                server_message!(
                    PacketServerCastSkillSelf,
                    CastSkillSelf {
                        entity_id,
                        skill_id,
                        cast_motion_id
                    }
                );
            }
            Some(ServerPackets::CastSkillTargetEntity) => {
                server_message!(
                    PacketServerCastSkillTargetEntity,
                    CastSkillTargetEntity {
                        entity_id,
                        skill_id,
                        cast_motion_id,
                        target_entity_id,
                        target_distance,
                        target_position
                    }
                );
            }
            Some(ServerPackets::CastSkillTargetPosition) => {
                server_message!(
                    PacketServerCastSkillTargetPosition,
                    CastSkillTargetPosition {
                        entity_id,
                        skill_id,
                        cast_motion_id,
                        target_position
                    }
                );
            }
            Some(ServerPackets::StartCastingSkill) => {
                server_message!(PacketServerStartCastingSkill, StartCastingSkill { entity_id });
            }
            Some(ServerPackets::CancelCastingSkill) => {
                server_message!(
                    PacketServerCancelCastingSkill,
                    CancelCastingSkill { entity_id, reason }
                );
            }
            Some(ServerPackets::FinishCastingSkill) => {
                server_message!(
                    PacketServerFinishCastingSkill,
                    FinishCastingSkill { entity_id, skill_id }
                );
            }
            Some(ServerPackets::UpdateCooldown) => {
                server_message!(
                    PacketServerUpdateCooldown,
                    UpdateCooldown { skill_id, duration }
                );
            }
            Some(ServerPackets::UpdateConsumableCooldown) => {
                server_message!(
                    PacketServerUpdateConsumableCooldown,
                    UpdateConsumableCooldown { cooldown_group, duration }
                );
            }
            Some(ServerPackets::ApplySkillEffect) => {
                server_message!(
                    PacketServerApplySkillEffect,
                    ApplySkillEffect {
                        entity_id,
                        caster_entity_id,
                        caster_intelligence,
                        skill_id,
                        effect_success
                    }
                );
            }
            Some(ServerPackets::ApplySkillDamage) => {
                let message = PacketServerApplySkillDamage::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity {
                        attacker_entity_id: message.caster_entity_id,
                        defender_entity_id: message.entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        is_immediate: message.is_immediate,
                        from_skill: Some((message.skill_id, message.caster_intelligence)),
                    })
                    .ok();
            }
            Some(ServerPackets::MoveToggle) => {
                let message = PacketServerMoveToggle::try_from(packet)?;
                match message.move_toggle_type {
                    PacketServerMoveToggleType::Walk => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Walk,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Run => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Run,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Drive => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Drive,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Sail => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Sail,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Sit => {
                        self.server_message_tx
                            .send(ServerMessage::SitToggle {
                                entity_id: message.entity_id,
                            })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::SailState) => {
                let message = PacketServerSailState::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SailState {
                        entity_id: message.entity_id,
                        position: Vec3::new(message.x, message.y, message.z),
                        heading: message.heading,
                        speed: message.speed,
                        sail_trim: message.sail_trim,
                    })
                    .ok();
            }
            Some(ServerPackets::WindStateUpdate) => {
                let message = PacketServerWindState::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::WindStateUpdate {
                        angle: message.angle,
                        speed: message.speed,
                        gust_factor: message.gust_factor,
                    })
                    .ok();
            }
            Some(ServerPackets::NpcStoreTransactionError) => {
                server_message!(
                    PacketServerNpcStoreTransactionError,
                    NpcStoreTransactionError { error }
                );
            }
            Some(ServerPackets::PartyRequest) => {
                let message = match PacketServerPartyRequest::try_from(packet)? {
                    PacketServerPartyRequest::Create(entity_id) => {
                        ServerMessage::PartyCreate { entity_id }
                    }
                    PacketServerPartyRequest::Invite(entity_id) => {
                        ServerMessage::PartyInvite { entity_id }
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyReply) => {
                let message = match PacketServerPartyReply::try_from(packet)? {
                    PacketServerPartyReply::AcceptCreate(entity_id) => {
                        ServerMessage::PartyAcceptCreate { entity_id }
                    }
                    PacketServerPartyReply::AcceptInvite(entity_id) => {
                        ServerMessage::PartyAcceptInvite { entity_id }
                    }
                    PacketServerPartyReply::RejectInvite(reason, entity_id) => {
                        ServerMessage::PartyRejectInvite { reason, entity_id }
                    }
                    PacketServerPartyReply::Delete => ServerMessage::PartyDelete,
                    PacketServerPartyReply::ChangeOwner(entity_id) => {
                        ServerMessage::PartyChangeOwner { entity_id }
                    }
                    PacketServerPartyReply::MemberKicked(character_id) => {
                        ServerMessage::PartyMemberKicked { character_id }
                    }
                    PacketServerPartyReply::MemberDisconnect(character_id) => {
                        ServerMessage::PartyMemberDisconnect { character_id }
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyMembers) => {
                let message = match PacketServerPartyMembers::try_from(packet)? {
                    PacketServerPartyMembers::Leave {
                        leaver_character_id,
                        owner_character_id,
                    } => ServerMessage::PartyMemberLeave {
                        leaver_character_id,
                        owner_character_id,
                    },
                    PacketServerPartyMembers::List {
                        item_sharing,
                        xp_sharing,
                        owner_character_id,
                        members,
                    } => ServerMessage::PartyMemberList {
                        item_sharing,
                        xp_sharing,
                        owner_character_id,
                        members,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyMemberUpdateInfo) => {
                server_message!(
                    PacketServerPartyMemberUpdateInfo,
                    PartyMemberUpdateInfo { member_info }
                );
            }
            Some(ServerPackets::PartyMemberRewardItem) => {
                let message = PacketServerPartyMemberRewardItem::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PartyMemberRewardItem {
                        client_entity_id: message.entity_id,
                        item: message.item,
                    })
                    .ok();
            }
            Some(ServerPackets::PartyUpdateRules) => {
                server_message!(
                    PacketServerPartyUpdateRules,
                    PartyUpdateRules { item_sharing, xp_sharing }
                );
            }
            Some(ServerPackets::AdjustPosition) => {
                server_message!(
                    PacketServerAdjustPosition,
                    AdjustPosition { entity_id, position }
                );
            }
            Some(ServerPackets::PersonalStoreItemList) => {
                server_message!(
                    PacketServerPersonalStoreItemList,
                    PersonalStoreItemList { sell_items, buy_items }
                );
            }
            Some(ServerPackets::PersonalStoreTransactionResult) => {
                let message = PacketServerPersonalStoreTransactionResult::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PersonalStoreTransaction {
                        status: message.status,
                        store_entity_id: message.store_entity_id,
                        update_store: message.update_store_items,
                    })
                    .ok();
            }
            Some(ServerPackets::PersonalStoreTransactionUpdateMoneyAndInventory) => {
                server_message!(
                    PacketServerPersonalStoreTransactionUpdateMoneyAndInventory,
                    PersonalStoreTransactionUpdateInventory { items, money }
                );
            }
            Some(ServerPackets::BankOpen) => match PacketServerBankOpen::try_from(packet)? {
                PacketServerBankOpen::Open => {
                    self.server_message_tx.send(ServerMessage::BankOpen).ok();
                }
                PacketServerBankOpen::SetItems { items } => {
                    self.server_message_tx
                        .send(ServerMessage::BankSetItems { items })
                        .ok();
                }
                PacketServerBankOpen::UpdateItems { items } => {
                    self.server_message_tx
                        .send(ServerMessage::BankUpdateItems { items })
                        .ok();
                }
            },
            Some(ServerPackets::BankTransaction) => {
                let packet = PacketServerBankTransaction::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::BankTransaction {
                        inventory_item_slot: packet.inventory_item_slot,
                        inventory_item: packet.inventory_item,
                        inventory_money: packet.inventory_money,
                        bank_slot: packet.bank_slot,
                        bank_item: packet.bank_item,
                    })
                    .ok();
            }
            Some(ServerPackets::LogoutResult) => {
                let packet = PacketServerLogoutResult::try_from(packet)?;
                match packet.result {
                    Ok(_) => {
                        self.server_message_tx
                            .send(ServerMessage::LogoutSuccess)
                            .ok();
                    }
                    Err(wait_duration) => {
                        self.server_message_tx
                            .send(ServerMessage::LogoutFailed { wait_duration })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::OpenPersonalStore) => {
                let packet = PacketServerOpenPersonalStore::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::OpenPersonalStore {
                        entity_id: packet.entity_id,
                        skin: packet.skin,
                        title: packet.title.into(),
                    })
                    .ok();
            }
            Some(ServerPackets::ClosePersonalStore) => {
                server_message!(PacketServerClosePersonalStore, ClosePersonalStore { entity_id });
            }
            Some(ServerPackets::CraftItem) => {
                let packet = PacketServerCraftItem::try_from(packet)?;
                match packet {
                    PacketServerCraftItem::InsertGemFailed { error } => {
                        self.server_message_tx
                            .send(ServerMessage::CraftInsertGemError { error })
                            .ok();
                    }
                    PacketServerCraftItem::InsertGemSuccess { items } => {
                        self.server_message_tx
                            .send(ServerMessage::CraftInsertGem {
                                update_items: items,
                            })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::RepairedItemUsingNpc) => {
                server_message!(
                    PacketServerRepairedItemUsingNpc,
                    RepairedItemUsingNpc {
                        item_slot,
                        item,
                        updated_money
                    }
                );
            }
            Some(ServerPackets::ClanCommand) => {
                let packet = PacketServerClanCommand::try_from(packet)?;
                match packet {
                    PacketServerClanCommand::ClanInfo {
                        id,
                        name,
                        description,
                        mark,
                        level,
                        points,
                        money,
                        position,
                        contribution,
                        skills,
                    } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanInfo {
                                id,
                                mark,
                                level,
                                points,
                                money,
                                name,
                                description,
                                position,
                                contribution,
                                skills,
                            })
                            .ok();
                    }
                    PacketServerClanCommand::ClanUpdateInfo {
                        id,
                        mark,
                        level,
                        points,
                        money,
                        skills,
                    } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanUpdateInfo {
                                id,
                                mark,
                                level,
                                points,
                                money,
                                skills,
                            })
                            .ok();
                    }
                    PacketServerClanCommand::CharacterUpdateClan {
                        client_entity_id,
                        id,
                        name,
                        mark,
                        level,
                        position,
                    } => {
                        self.server_message_tx
                            .send(ServerMessage::CharacterUpdateClan {
                                client_entity_id,
                                id,
                                name,
                                mark,
                                level,
                                position,
                            })
                            .ok();
                    }
                    PacketServerClanCommand::ClanMemberConnected { name, channel_id } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanMemberConnected { name, channel_id })
                            .ok();
                    }
                    PacketServerClanCommand::ClanMemberDisconnected { name } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanMemberDisconnected { name })
                            .ok();
                    }
                    PacketServerClanCommand::ClanCreateError { error } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanCreateError { error })
                            .ok();
                    }
                    PacketServerClanCommand::ClanMemberList { members } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanMemberList { members })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::UpdateAbilityValues) => {
                server_message!(
                    PacketServerUpdateAbilityValues,
                    UpdateAbilityValues {
                        entity_id,
                        attack_power,
                        defence,
                        hit,
                        resistance,
                        avoid,
                        attack_speed,
                        critical,
                        max_health,
                        max_mana,
                        move_speed
                    }
                );
            }
            Some(ServerPackets::RepairedItemUsingItem) => {
                log::info!(
                    "Unimplemented ServerPackets::RepairedItemUsingItem {:?}",
                    packet
                );
            }
            None => log::info!("Unhandled GameClient packet {:?}", packet),
        }

        Ok(())
    }

    async fn handle_client_message(
        &self,
        connection: &mut Connection<'_>,
        message: ClientMessage,
    ) -> Result<(), anyhow::Error> {
        macro_rules! send_packet {
            ($packet:expr) => {
                connection.write_packet(Packet::from(&$packet)).await?;
            };
        }

        match message {
            ClientMessage::ConnectionRequest {
                login_token,
                ref password,
            } => {
                send_packet!(PacketClientConnectRequest {
                    login_token,
                    password_md5: &password.to_md5()
                });
            }
            ClientMessage::JoinZoneRequest => {
                send_packet!(PacketClientJoinZone { weight_rate: 0, z: 0 });
            }
            ClientMessage::Move {
                target_entity_id,
                x,
                y,
                z,
            } => {
                send_packet!(PacketClientMove {
                    target_entity_id,
                    x,
                    y,
                    z
                });
            }
            ClientMessage::Attack { target_entity_id } => {
                send_packet!(PacketClientAttack { target_entity_id });
            }
            ClientMessage::PickupItemDrop { target_entity_id } => {
                send_packet!(PacketClientPickupItemDrop { target_entity_id });
            }
            ClientMessage::Chat { ref text } => {
                send_packet!(PacketClientChat { text });
            }
            ClientMessage::ChangeAmmo {
                ammo_index,
                item_slot,
            } => {
                send_packet!(PacketClientChangeAmmo { ammo_index, item_slot });
            }
            ClientMessage::ChangeEquipment {
                equipment_index,
                item_slot,
            } => {
                send_packet!(PacketClientChangeEquipment { equipment_index, item_slot });
            }
            ClientMessage::ChangeVehiclePart {
                vehicle_part_index,
                item_slot,
            } => {
                send_packet!(PacketClientChangeVehiclePart { vehicle_part_index, item_slot });
            }
            ClientMessage::QuestDelete { slot, quest_id } => {
                send_packet!(PacketClientQuestRequest {
                    request_type: PacketClientQuestRequestType::DeleteQuest,
                    quest_slot: slot as u8,
                    quest_id: quest_id as u32
                });
            }
            ClientMessage::QuestTrigger { trigger } => {
                send_packet!(PacketClientQuestRequest {
                    request_type: PacketClientQuestRequestType::DoTrigger,
                    quest_slot: 0,
                    quest_id: trigger.hash
                });
            }
            ClientMessage::SetHotbarSlot { slot_index, slot } => {
                send_packet!(PacketClientSetHotbarSlot { slot_index, slot });
            }
            ClientMessage::IncreaseBasicStat { basic_stat_type } => {
                send_packet!(PacketClientIncreaseBasicStat { basic_stat_type });
            }
            ClientMessage::ReviveCurrentZone => {
                send_packet!(PacketClientReviveRequest::CurrentZone);
            }
            ClientMessage::ReviveSaveZone => {
                send_packet!(PacketClientReviveRequest::SaveZone);
            }
            ClientMessage::PersonalStoreListItems { store_entity_id } => {
                send_packet!(PacketClientPersonalStoreListItems {
                    target_entity_id: store_entity_id
                });
            }
            ClientMessage::DropItem {
                item_slot,
                quantity,
            } => {
                send_packet!(PacketClientDropItemFromInventory::Item(
                    item_slot,
                    quantity as u32
                ));
            }
            ClientMessage::DropMoney { quantity } => {
                send_packet!(PacketClientDropItemFromInventory::Money(quantity as u32));
            }
            ClientMessage::UseItem {
                item_slot,
                target_entity_id,
            } => {
                send_packet!(PacketClientUseItem { item_slot, target_entity_id });
            }
            ClientMessage::WarpGateRequest { warp_gate_id } => {
                send_packet!(PacketClientWarpGateRequest { warp_gate_id });
            }
            ClientMessage::LevelUpSkill { skill_slot } => {
                send_packet!(PacketClientLevelUpSkill {
                    skill_slot,
                    next_skill_idx: SkillId::new(0).unwrap() // 0 means server will use current_skill_idx + 1
                });
            }
            ClientMessage::UseEmote { motion_id, is_stop } => {
                send_packet!(PacketClientEmote { motion_id, is_stop });
            }
            ClientMessage::CastSkillSelf { skill_slot } => {
                send_packet!(PacketClientCastSkillSelf { skill_slot });
            }
            ClientMessage::CastSkillTargetEntity {
                skill_slot,
                target_entity_id,
            } => {
                send_packet!(PacketClientCastSkillTargetEntity { skill_slot, target_entity_id });
            }
            ClientMessage::CastSkillTargetPosition {
                skill_slot,
                position,
            } => {
                send_packet!(PacketClientCastSkillTargetPosition { skill_slot, position });
            }
            ClientMessage::RunToggle => {
                send_packet!(PacketClientMoveToggle {
                    toggle_type: PacketClientMoveToggleType::Run
                });
            }
            ClientMessage::SitToggle => {
                send_packet!(PacketClientMoveToggle {
                    toggle_type: PacketClientMoveToggleType::Sit
                });
            }
            ClientMessage::DriveToggle => {
                send_packet!(PacketClientMoveToggle {
                    toggle_type: PacketClientMoveToggleType::Drive
                });
            }
            ClientMessage::NpcStoreTransaction {
                npc_entity_id,
                buy_items,
                sell_items,
            } => {
                send_packet!(PacketClientNpcStoreTransaction { npc_entity_id, buy_items, sell_items });
            }
            ClientMessage::PartyCreate { invited_entity_id } => {
                send_packet!(PacketClientPartyRequest::Create(invited_entity_id));
            }
            ClientMessage::PartyInvite { invited_entity_id } => {
                send_packet!(PacketClientPartyRequest::Invite(invited_entity_id));
            }
            ClientMessage::PartyLeave => {
                send_packet!(PacketClientPartyRequest::Leave);
            }
            ClientMessage::PartyChangeOwner { new_owner_entity_id } => {
                send_packet!(PacketClientPartyRequest::ChangeOwner(new_owner_entity_id));
            }
            ClientMessage::PartyKick { character_id } => {
                send_packet!(PacketClientPartyRequest::Kick(character_id));
            }
            ClientMessage::PartyAcceptCreateInvite { owner_entity_id } => {
                send_packet!(PacketClientPartyReply::AcceptCreate(owner_entity_id));
            }
            ClientMessage::PartyAcceptJoinInvite { owner_entity_id } => {
                send_packet!(PacketClientPartyReply::AcceptJoin(owner_entity_id));
            }
            ClientMessage::PartyRejectInvite {
                reason,
                owner_entity_id,
            } => {
                send_packet!(PacketClientPartyReply::Reject(reason, owner_entity_id));
            }
            ClientMessage::PartyUpdateRules {
                item_sharing,
                xp_sharing,
            } => {
                send_packet!(PacketClientPartyUpdateRules { item_sharing, xp_sharing });
            }
            ClientMessage::MoveCollision { position } => {
                send_packet!(PacketClientMoveCollision { position });
            }
            ClientMessage::SailInput {
                rudder,
                throttle,
                heading,
                speed,
                sail_trim,
                x,
                y,
                z,
            } => {
                send_packet!(PacketClientSailInput {
                    rudder: encode_input_i8(rudder),
                    throttle: encode_input_i8(throttle),
                    heading,
                    speed,
                    sail_trim,
                    x,
                    y,
                    z
                });
            }
            ClientMessage::BoardBoat { x, y, z } => {
                send_packet!(PacketClientBoardBoat { x, y, z });
            }
            ClientMessage::DisembarkBoat { x, y, z } => {
                send_packet!(PacketClientDisembarkBoat { x, y, z });
            }
            ClientMessage::PersonalStoreBuyItem {
                store_entity_id,
                store_slot_index,
                buy_item,
            } => {
                send_packet!(PacketClientPersonalStoreBuyItem {
                    store_entity_id,
                    store_slot_index,
                    buy_item
                });
            }
            ClientMessage::BankOpen => {
                send_packet!(PacketClientBankOpen {});
            }
            ClientMessage::BankDepositItem {
                item_slot,
                item,
                is_premium,
            } => {
                send_packet!(PacketClientBankMoveItem::Deposit { item_slot, item, is_premium });
            }
            ClientMessage::BankWithdrawItem {
                bank_slot,
                item,
                is_premium,
            } => {
                send_packet!(PacketClientBankMoveItem::Withdraw { bank_slot, item, is_premium });
            }
            ClientMessage::SetReviveSaveZone => {
                send_packet!(PacketClientSetReviveZone);
            }
            ClientMessage::ClanCreate {
                name,
                description,
                mark,
            } => {
                send_packet!(PacketClientClanCommand::Create { name, description, mark });
            }
            ClientMessage::CraftInsertGem {
                equipment_index,
                item_slot,
            } => {
                send_packet!(PacketClientCraftItem::InsertGem { equipment_index, item_slot });
            }
            ClientMessage::CraftSkillDisassemble {
                skill_slot,
                item_slot,
            } => {
                send_packet!(PacketClientCraftItem::SkillDisassemble { skill_slot, item_slot });
            }
            ClientMessage::CraftNpcDisassemble {
                npc_entity_id,
                item_slot,
            } => {
                send_packet!(PacketClientCraftItem::NpcDisassemble { npc_entity_id, item_slot });
            }
            ClientMessage::CraftSkillUpgradeItem {
                skill_slot,
                item_slot,
                ingredients,
            } => {
                send_packet!(PacketClientCraftItem::SkillUpgradeItem { skill_slot, item_slot, ingredients });
            }
            ClientMessage::CraftNpcUpgradeItem {
                npc_entity_id,
                item_slot,
                ingredients,
            } => {
                send_packet!(PacketClientCraftItem::NpcUpgradeItem { npc_entity_id, item_slot, ingredients });
            }
            ClientMessage::RepairItemUsingItem {
                use_item_slot,
                item_slot,
            } => {
                send_packet!(PacketClientRepairItemUsingItem { use_item_slot, item_slot });
            }
            ClientMessage::RepairItemUsingNpc {
                npc_entity_id,
                item_slot,
            } => {
                send_packet!(PacketClientRepairItemUsingNpc { npc_entity_id, item_slot });
            }
            unimplemented => {
                log::info!("Unimplemented GameClient ClientMessage {:?}", unimplemented);
            }
        }
        Ok(())
    }
}

implement_protocol_client! { GameClient }
