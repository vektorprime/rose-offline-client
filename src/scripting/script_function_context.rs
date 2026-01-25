use bevy::{
    ecs::system::SystemParam,
    prelude::{Query, With},
};

use rose_game_common::components::{
    AbilityValues, BasicStats, CharacterInfo, Equipment, ExperiencePoints, HealthPoints, Inventory,
    Level, ManaPoints, MoveSpeed, Npc, QuestState, SkillPoints, Stamina, StatPoints, Team,
    UnionMembership,
};

use crate::{
    components::{ClanMembership, ClientEntity, PlayerCharacter},
};

// NOTE: ScriptFunctionContext contains all of the queries needed by script functions.
// Event writers are handled separately due to lifetime constraints in Bevy 0.13.

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

    #[system_param(ignore)]
    pub phantom: std::marker::PhantomData<()>,
}
