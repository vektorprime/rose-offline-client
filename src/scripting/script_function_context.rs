use bevy::{
    ecs::system::SystemParam,
    prelude::{Commands, Query, With},
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

// NOTE: ScriptFunctionContext contains all of the queries and Commands needed by script functions.
// Commands::queue() is used to defer message dispatching, avoiding lifetime constraints of MessageWriter.

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

    // Commands for deferred event dispatching via Commands::queue()
    pub commands: Commands<'w, 's>,
}

impl<'w, 's> ScriptFunctionContext<'w, 's> {
    /// Queue a BankEvent to be dispatched later
    pub fn queue_bank_event(&mut self, event: BankEvent) {
        self.commands.queue(|w: &mut bevy::prelude::World| {
            w.write_message(event);
        });
    }

    /// Queue a ChatboxEvent to be dispatched later
    pub fn queue_chatbox_event(&mut self, event: ChatboxEvent) {
        self.commands.queue(|w: &mut bevy::prelude::World| {
            w.write_message(event);
        });
    }

    /// Queue a ClanDialogEvent to be dispatched later
    pub fn queue_clan_dialog_event(&mut self, event: ClanDialogEvent) {
        self.commands.queue(|w: &mut bevy::prelude::World| {
            w.write_message(event);
        });
    }

    /// Queue a NpcStoreEvent to be dispatched later
    pub fn queue_npc_store_event(&mut self, event: NpcStoreEvent) {
        self.commands.queue(|w: &mut bevy::prelude::World| {
            w.write_message(event);
        });
    }

    /// Queue a SystemFuncEvent to be dispatched later
    pub fn queue_system_func_event(&mut self, event: SystemFuncEvent) {
        self.commands.queue(|w: &mut bevy::prelude::World| {
            w.write_message(event);
        });
    }
}
