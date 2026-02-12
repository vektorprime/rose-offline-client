use bevy::{
    ecs::system::SystemParam,
    prelude::{EventWriter, Query, With},
};

use rose_game_common::components::{
    AbilityValues, BasicStats, CharacterInfo, Equipment, ExperiencePoints, HealthPoints, Inventory,
    Level, ManaPoints, MoveSpeed, Npc, QuestState, SkillPoints, Stamina, StatPoints, Team,
    UnionMembership,
};

use crate::{
    components::{ClanMembership, ClientEntity, PlayerCharacter},
    events::{BankEvent, ChatboxEvent, ClanDialogEvent, NpcStoreEvent, SystemFuncEvent},
};

// NOTE: ScriptFunctionContext contains all of the queries and event writers needed by script functions.

#[derive(SystemParam)]
pub struct ScriptFunctionContext<'w, 's> {
    pub query_quest: Query<'w, 's, &'static mut QuestState>,
    pub query_client_entity: Query<'w, 's, &'static ClientEntity>,
    pub query_player_stats: Query<'w, 's, (
        &'static AbilityValues,
        &'static CharacterInfo,
        &'static BasicStats,
        &'static ExperiencePoints,
        &'static Level,
        &'static UnionMembership,
    ), With<PlayerCharacter>>,
    pub query_player_mutable: Query<'w, 's, (
        &'static mut HealthPoints,
        &'static mut ManaPoints,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut MoveSpeed,
        &'static mut SkillPoints,
        &'static mut Stamina,
        &'static mut StatPoints,
        &'static mut Team,
    ), With<PlayerCharacter>>,
    pub query_player_clan: Query<'w, 's, &'static ClanMembership, With<PlayerCharacter>>,
    pub query_npc: Query<'w, 's, &'static Npc>,

    // Event writers for script functions
    pub bank_events: EventWriter<'w, BankEvent>,
    pub chatbox_events: EventWriter<'w, ChatboxEvent>,
    pub clan_dialog_events: EventWriter<'w, ClanDialogEvent>,
    pub npc_store_events: EventWriter<'w, NpcStoreEvent>,
    pub script_system_events: EventWriter<'w, SystemFuncEvent>,
}
