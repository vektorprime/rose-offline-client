use bevy::prelude::Resource;
use std::collections::HashMap;

use rose_game_common::{components::CharacterGender, messages::ClientEntityId};

use crate::{
    events::{BankEvent, ClanDialogEvent, NpcStoreEvent},
    scripting::{
        lua4::Lua4Value,
        lua_closures,
        lua_game_constants::{
            SV_BIRTH, SV_CHA, SV_CLASS, SV_CON, SV_DEX, SV_EXP, SV_FAME, SV_INT, SV_LEVEL, SV_RANK,
            SV_SEN, SV_SEX, SV_STR, SV_UNION,
        },
        LuaClosure, ScriptFunctionContext, ScriptFunctionResources,
    },
};

#[derive(Resource)]
pub struct LuaGameFunctions {
    pub closures: HashMap<String, LuaClosure>,
}

impl Default for LuaGameFunctions {
    fn default() -> Self {
        lua_closures!(
            "GF_getVariable" => GF_getVariable,
            "GF_openBank" => GF_openBank,
            "GF_openStore" => GF_openStore,
            "GF_organizeClan" => GF_organizeClan,
        )
    }
}

#[allow(non_snake_case)]
fn GF_getVariable(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let variable_id = parameters[0].to_i32().unwrap();
    let Ok(character) = context.query_player_stats.single() else {
        return vec![0.into()];
    };

    // Tuple structure: (AbilityValues, CharacterInfo, BasicStats, ExperiencePoints, Level, UnionMembership)
    let value = match variable_id {
        SV_SEX => match character.1.gender {
            CharacterGender::Male => 0,
            CharacterGender::Female => 1,
        },
        SV_BIRTH => character.1.birth_stone as i32,
        SV_CLASS => character.1.job as i32,
        SV_UNION => character
            .5
            .current_union
            .map(|x| x.get() as i32)
            .unwrap_or(0),
        SV_RANK => character.1.rank as i32,
        SV_FAME => character.1.fame as i32,
        SV_STR => character.2.strength,
        SV_DEX => character.2.dexterity,
        SV_INT => character.2.intelligence,
        SV_CON => character.2.concentration,
        SV_CHA => character.2.charm,
        SV_SEN => character.2.sense,
        SV_EXP => character.3.xp as i32,
        SV_LEVEL => character.4.level as i32,
        _ => 0,
    };

    vec![value.into()]
}

#[allow(non_snake_case)]
fn GF_openBank(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    (|| -> Option<()> {
        let npc_client_entity_id = ClientEntityId(parameters.get(0)?.to_usize().ok()?);
        context.queue_bank_event(BankEvent::OpenBankFromClientEntity {
            client_entity_id: npc_client_entity_id,
        });
        Some(())
    })();
    vec![]
}

#[allow(non_snake_case)]
fn GF_openStore(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    (|| -> Option<()> {
        let npc_client_entity_id = ClientEntityId(parameters.get(0)?.to_usize().ok()?);
        context.queue_npc_store_event(NpcStoreEvent::OpenClientEntityStore(npc_client_entity_id));
        Some(())
    })();
    vec![]
}

#[allow(non_snake_case)]
fn GF_organizeClan(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    _parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    context.queue_clan_dialog_event(ClanDialogEvent::Open);
    vec![]
}
