use bevy::prelude::Resource;
use std::collections::HashMap;

use rose_game_common::messages::client::ClientMessage;

use crate::scripting::{
    lua4::Lua4Value, lua_closures, quest::quest_check_conditions, LuaClosure, LuaUserValueEntity,
    ScriptFunctionContext, ScriptFunctionResources,
};

macro_rules! lua_i32_closure {
    ($name:ident, $default:expr, |$context:ident, $parameters:ident| $body:expr) => {
        #[allow(non_snake_case, unused_variables)]
        fn $name(
            _resources: &ScriptFunctionResources,
            $context: &mut ScriptFunctionContext,
            $parameters: Vec<Lua4Value>,
        ) -> Vec<Lua4Value> {
            let result = (|| -> Option<i32> {
                $body
            })()
            .unwrap_or($default);

            vec![result.into()]
        }
    };
}

#[derive(Resource)]
pub struct LuaQuestFunctions {
    pub closures: HashMap<String, LuaClosure>,
}

impl Default for LuaQuestFunctions {
    fn default() -> Self {
        lua_closures!(
            "QF_checkQuestCondition" => QF_checkQuestCondition,
            "QF_doQuestTrigger" => QF_doQuestTrigger,
            "QF_findQuest" => QF_findQuest,
            "QF_getEventOwner" => QF_getEventOwner,
            "QF_getEpisodeVAR" => QF_getEpisodeVAR,
            "QF_getJobVAR" => QF_getJobVAR,
            "QF_getPlanetVAR" => QF_getPlanetVAR,
            "QF_getQuestCount" => QF_getQuestCount,
            "QF_getQuestID" => QF_getQuestID,
            "QF_getQuestItemQuantity" => QF_getQuestItemQuantity,
            "QF_getQuestSwitch" => QF_getQuestSwitch,
            "QF_getQuestVar" => QF_getQuestVar,
            "QF_getUserSwitch" => QF_getUserSwitch,
            "QF_getNpcQuestZeroVal" => QF_getNpcQuestZeroVal,
        )
    }
}

#[allow(non_snake_case)]
fn QF_checkQuestCondition(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(quest_trigger_name) = parameters[0].to_string() {
        if let Ok(true) =
            quest_check_conditions(resources, context, quest_trigger_name.as_str().into())
        {
            return vec![1.into()];
        }
    }

    vec![0.into()]
}

#[allow(non_snake_case)]
fn QF_doQuestTrigger(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = if let Ok(quest_trigger_name) = parameters[0].to_string() {
        if let Ok(true) =
            quest_check_conditions(resources, context, quest_trigger_name.as_str().into())
        {
            if let Some(game_connection) = resources.game_connection.as_ref() {
                game_connection
                    .client_message_tx
                    .send(ClientMessage::QuestTrigger {
                        trigger: quest_trigger_name.as_str().into(),
                    })
                    .ok();
            }

            1
        } else {
            0
        }
    } else {
        0
    };

    vec![result.into()]
}

lua_i32_closure!(QF_findQuest, -1, |context, parameters| {
    let quest_id = parameters.get(0)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    quest_state
        .find_active_quest_index(quest_id)
        .map(|x| x as i32)
});

#[allow(non_snake_case)]
fn QF_getEventOwner(
    _resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    if let Ok(lua_value_entity) = parameters[0].to_user_type::<LuaUserValueEntity>() {
        if let Some(entity) = lua_value_entity.owner_entity {
            if let Ok(client_entity) = context.query_client_entity.get(entity) {
                return vec![client_entity.id.0.into()];
            }
        }
    }

    vec![0.into()]
}

lua_i32_closure!(QF_getEpisodeVAR, -1, |context, parameters| {
    let var_id = parameters.get(0)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    Some(*quest_state.episode_variables.get(var_id)? as i32)
});

lua_i32_closure!(QF_getJobVAR, -1, |context, parameters| {
    let var_id = parameters.get(0)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    Some(*quest_state.job_variables.get(var_id)? as i32)
});

lua_i32_closure!(QF_getPlanetVAR, -1, |context, parameters| {
    let var_id = parameters.get(0)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    Some(*quest_state.planet_variables.get(var_id)? as i32)
});

lua_i32_closure!(QF_getQuestCount, 0, |context, _parameters| {
    let quest_state = context.query_quest.single().ok()?;
    Some(
        quest_state
            .active_quests
            .iter()
            .filter(|x| x.is_some())
            .count() as i32,
    )
});

lua_i32_closure!(QF_getQuestID, -1, |context, parameters| {
    let quest_index = parameters.get(0)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    let quest = quest_state.get_quest(quest_index)?;
    Some(quest.quest_id as i32)
});

#[allow(non_snake_case)]
fn QF_getQuestItemQuantity(
    resources: &ScriptFunctionResources,
    context: &mut ScriptFunctionContext,
    parameters: Vec<Lua4Value>,
) -> Vec<Lua4Value> {
    let result = || -> Option<i32> {
        let quest_id = parameters.get(0)?.to_usize().ok()?;
        let item_base1000 = parameters.get(1)?.to_usize().ok()?;

        let item_reference = resources
            .game_data
            .data_decoder
            .decode_item_base1000(item_base1000)?;
        let quest_state = context.query_quest.single().ok()?;
        let quest = quest_state.find_active_quest(quest_id)?;
        for item in quest.items.iter().flatten() {
            if item.get_item_reference() == item_reference {
                return Some(item.get_quantity() as i32);
            }
        }

        Some(0)
    }()
    .unwrap_or(-1);

    vec![result.into()]
}

lua_i32_closure!(QF_getQuestSwitch, -1, |context, parameters| {
    let quest_index = parameters.get(0)?.to_usize().ok()?;
    let quest_switch_id = parameters.get(1)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    let quest = quest_state.get_quest(quest_index)?;
    Some(*quest.switches.get(quest_switch_id)? as i32)
});

lua_i32_closure!(QF_getQuestVar, -1, |context, parameters| {
    let quest_index = parameters.get(0)?.to_usize().ok()?;
    let quest_var_id = parameters.get(1)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    let quest = quest_state.get_quest(quest_index)?;
    Some(*quest.variables.get(quest_var_id)? as i32)
});

lua_i32_closure!(QF_getUserSwitch, -1, |context, parameters| {
    let switch_id = parameters.get(0)?.to_usize().ok()?;

    let quest_state = context.query_quest.single().ok()?;
    Some(*quest_state.quest_switches.get(switch_id)? as i32)
});

lua_i32_closure!(QF_getNpcQuestZeroVal, 0, |context, parameters| {
    let npc_id = parameters.get(0)?.to_usize().ok()?;

    for npc in context.query_npc.iter() {
        if npc.id.get() as usize == npc_id {
            return Some(npc.quest_index as i32);
        }
    }

    None
});
