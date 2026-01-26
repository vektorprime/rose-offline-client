use std::sync::Arc;

use bevy::{
    prelude::{Assets, Entity, EventReader, Local, Query, Res, With},
};
use bevy_egui::{egui, EguiContexts};
use rose_file_readers::{ConFile, ConMessageType};
use rose_game_common::components::QuestState;

use crate::{
    components::{ClanMembership, ClientEntity, ClientEntityName, PlayerCharacter, Position},
    events::{BankEvent, ChatboxEvent, ClanDialogEvent, ConversationDialogEvent, NpcStoreEvent, SystemFuncEvent},
    resources::{GameData, UiResources, UiSprite},
    scripting::{
        lua4::{Lua4Function, Lua4VM, Lua4VMError, Lua4VMRustClosures, Lua4Value},
        LuaGameConstants, LuaGameFunctions, LuaQuestFunctions, LuaUserValueEntity,
        ScriptFunctionResources,
    },
    ui::{widgets::Dialog, DataBindings, DialogInstance},
    VfsResource,
};

pub struct GeneratedDialogResponse {
    pub text: egui::text::LayoutJob,
    pub galley: Option<Arc<egui::text::Galley>>,
    pub action_function: String,
    pub menu_index: i32,
}

#[derive(Default)]
pub struct GeneratedDialog {
    pub message: egui::text::LayoutJob,
    pub responses: Vec<GeneratedDialogResponse>,
}

pub struct ConversationDialogState {
    pub owner_entity: Option<Entity>,
    pub con_file: ConFile,
    pub generated_dialog: GeneratedDialog,
    pub lua_vm: Lua4VM,
    pub event_object_handle: Arc<dyn std::any::Any + Send + Sync>,
}

pub struct LuaVMContext<'a, 'w, 's> {
    pub function_context: &'a mut crate::scripting::ScriptFunctionContext<'w, 's>,
    pub function_resources: &'a ScriptFunctionResources<'w, 's>,
    pub game_constants: &'a LuaGameConstants,
    pub game_functions: &'a LuaGameFunctions,
    pub quest_functions: &'a LuaQuestFunctions,
}

impl<'a, 'w, 's> Lua4VMRustClosures for LuaVMContext<'a, 'w, 's> {
    fn call_rust_closure(
        &mut self,
        name: &str,
        parameters: Vec<Lua4Value>,
    ) -> Result<Vec<Lua4Value>, Lua4VMError> {
        if let Some(closure) = self.quest_functions.closures.get(name) {
            Ok(closure(
                self.function_resources,
                self.function_context,
                parameters,
            ))
        } else if let Some(closure) = self.game_functions.closures.get(name) {
            Ok(closure(
                self.function_resources,
                self.function_context,
                parameters,
            ))
        } else {
            Err(Lua4VMError::GlobalNotFound(name.to_string()))
        }
    }
}

fn create_conversation_dialog(
    con_file: ConFile,
    user_context: &mut LuaVMContext,
    owner_entity: Option<Entity>,
) -> Option<ConversationDialogState> {
    let mut lua_vm = Lua4VM::new();

    for (name, value) in user_context.game_constants.constants.iter() {
        lua_vm.set_global(name.clone(), value.clone());
    }

    for (name, _) in user_context.game_functions.closures.iter() {
        lua_vm.set_global(name.clone(), Lua4Value::RustClosure(name.clone()));
    }

    for (name, _) in user_context.quest_functions.closures.iter() {
        lua_vm.set_global(name.clone(), Lua4Value::RustClosure(name.clone()));
    }

    let lua_function = Lua4Function::from_bytes(&con_file.script_binary).ok()?;
    lua_vm
        .call_lua_function(user_context, &lua_function, &[])
        .ok()?;

    Some(ConversationDialogState {
        owner_entity,
        con_file,
        event_object_handle: Arc::new(LuaUserValueEntity { owner_entity }),
        generated_dialog: Default::default(),
        lua_vm,
    })
}

// TODO: Fix parse_message for Bevy 0.13
// fn parse_message(message: &str, user_context: &LuaVMContext) -> String {
//     let mut string = String::with_capacity(message.len());

//     let mut remaining = message;
//     while let Some(template_start) = remaining.find(|c| c == '<') {
//         let (before_template, template) = remaining.split_at(template_start);
//
//         let template_end = template.find(|c| c == '>');
//         if template_end.is_none() {
//             return string;
//         }
//         let template_end = template_end.unwrap();
//         let (template, after_template) = template.split_at(template_end + 1);
//
//         string += before_template;
//         string += match template {
//             "<NAME>" => user_context
//                 .function_context
//                 .query_player_stats
//                 .get_single()
//                 .map(|player| player.1.name.clone())
//                 .ok(),
//             "<LEVEL>" => user_context
//                 .function_context
//                 .query_player_stats
//                 .get_single()
//                 .map(|player| format!("{}", player.4.level))
//                 .ok(),
//             _ => None,
//         }
//         .unwrap_or_else(|| template.to_string())
//         .as_str();
//         remaining = after_template;
//     }
//
//     string += remaining;
//     string
// }

fn message_layout_job(response_number: Option<usize>, message: &str) -> egui::text::LayoutJob {
    let default_text_color = egui::Color32::BLACK;
    let mut remaining = message;
    let mut job = egui::text::LayoutJob::default();
    let mut current_text_format = egui::text::TextFormat {
        color: default_text_color,
        ..Default::default()
    };
    job.wrap.max_width = 300.0;

    if let Some(response_number) = response_number {
        job.append(
            &format!("{}. ", response_number + 1),
            0.0,
            current_text_format.clone(),
        );
    }

    while let Some(tag_start) = remaining.find('{') {
        let (before_tag, tag) = remaining.split_at(tag_start);
        let tag_end = tag.find('}');
        if tag_end.is_none() {
            break;
        }
        let tag_end = tag_end.unwrap();
        let (tag, after_tag) = tag.split_at(tag_end + 1);

        let tag_lower = tag.to_lowercase();
        match tag_lower.as_str() {
            "{br}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                job.append("\n", 0.0, current_text_format.clone());
            }
            "{b}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                current_text_format.italics = true;
            }
            "{/b}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                current_text_format.italics = false;
            }
            "{/fc}" => {
                job.append(before_tag, 0.0, current_text_format.clone());
                current_text_format.color = default_text_color;
            }
            tag if tag.starts_with("{fc=") => {
                let len = tag.len();
                let index_str = &tag[4..len - 1];
                if let Ok(color_index) = index_str.parse::<i32>() {
                    job.append(before_tag, 0.0, current_text_format.clone());
                    current_text_format.color = match color_index {
                        0 => egui::Color32::from_rgb(0, 0, 0),
                        1 => egui::Color32::from_rgb(0x80, 0, 0),
                        2 => egui::Color32::from_rgb(0, 0x80, 0),
                        3 => egui::Color32::from_rgb(0, 0, 0x80),
                        4 => egui::Color32::from_rgb(0x80, 0x80, 0),
                        5 => egui::Color32::from_rgb(0x80, 0, 0x80),
                        6 => egui::Color32::from_rgb(0, 0x80, 0x80),
                        7 => egui::Color32::from_rgb(0x80, 0x80, 0x80),
                        8 => egui::Color32::from_rgb(0xC0, 0xC0, 0xC0),
                        9 => egui::Color32::from_rgb(0xC0, 0xDC, 0xC0),
                        10 => egui::Color32::from_rgb(0xC0, 0xC0, 0xDC),
                        11 => egui::Color32::from_rgb(0xA6, 0xCA, 0xF0),
                        12 => egui::Color32::from_rgb(0xFF, 0, 0),
                        13 => egui::Color32::from_rgb(0, 0xFF, 0),
                        14 => egui::Color32::from_rgb(0, 0, 0xFF),
                        15 => egui::Color32::from_rgb(0xFF, 0xFF, 0),
                        16 => egui::Color32::from_rgb(0, 0xFF, 0xFF),
                        17 => egui::Color32::from_rgb(0xFF, 0xFB, 0xF0),
                        18 => egui::Color32::from_rgb(0xFF, 0xFF, 0xFF),
                        _ => default_text_color,
                    };
                }
            }
            _ => {}
        }

        remaining = after_tag;
    }

    if !remaining.is_empty() {
        job.append(remaining, 0.0, current_text_format);
    }

    job
}

impl GeneratedDialog {
    fn run_menu(
        &mut self,
        lua_vm: &mut Lua4VM,
        user_context: &mut LuaVMContext,
        con_file: &ConFile,
        event_object_handle: &Arc<dyn std::any::Any + Send + Sync>,
        game_data: &GameData,
        menu_idx: i32,
    ) -> bool {
        if menu_idx < 0 {
            return false;
        }

        let menu = &con_file.menus[menu_idx as usize];
        let mut any_added = false;
        for message in menu.messages.iter() {
            if !message.condition_function.is_empty() {
                match lua_vm.call_global_closure(
                    user_context,
                    &message.condition_function,
                    &[Lua4Value::UserData(event_object_handle.clone())],
                ) {
                    Ok(result) => {
                        let result = result
                            .get(0)
                            .and_then(|value| value.to_i32().ok())
                            .unwrap_or(0);

                        if result == 0 {
                            log::trace!(target: "con",
                                "Menu check function {} failed with result: {}",
                                &message.condition_function,
                                result
                            );
                            continue;
                        } else {
                            log::trace!(target: "con",
                                "Menu check function {} passed with result: {}",
                                &message.condition_function,
                                result
                            );
                        }
                    }
                    Err(error) => {
                        log::error!(target: "con",
                            "Error running conversation script function {}: {}",
                            &message.condition_function,
                            error
                        );
                        continue;
                    }
                }
            }

            match message.message_type {
                ConMessageType::Close
                | ConMessageType::PlayerSelect
                | ConMessageType::JumpSelect => {
                    if let Some(response_text) = game_data
                        .ltb_event
                        .get_string(message.string_id as usize, 2)
                        .map(|message| message.to_string())
                    {
                        self.responses.push(GeneratedDialogResponse {
                            text: message_layout_job(
                                Some(self.responses.len()),
                                response_text.as_str(),
                            ),
                            galley: None,
                            action_function: message.action_function.clone(),
                            menu_index: message.message_value,
                        });
                    } else {
                        log::debug!(target: "con", "Failed to get LTB response string in menu_idx {} with string_id {}", menu_idx, message.string_id);
                    }
                }
                ConMessageType::NextMessage | ConMessageType::ShowMessage => {
                    if let Some(message_text) = game_data
                        .ltb_event
                        .get_string(message.string_id as usize, 2)
                        .map(|message| message.to_string())
                    {
                        self.message = message_layout_job(None, message_text.as_str());
                        self.responses.clear();

                        self.run_menu(
                            lua_vm,
                            user_context,
                            con_file,
                            event_object_handle,
                            game_data,
                            message.message_value,
                        );
                    } else {
                        log::debug!(target: "con", "Failed to get LTB message string in menu_idx {} with string_id {}", menu_idx, message.string_id);
                    }
                }
            }

            any_added = true;
        }

        any_added
    }
}

struct UiConversationDialogSprites {
    message_top: UiSprite,
    message_middle: UiSprite,
    message_bottom: UiSprite,
    answer_top: UiSprite,
    answer_middle: UiSprite,
    answer_bottom: UiSprite,
}

pub struct UiConversationDialogState {
    dialog_instance: DialogInstance,
    sprites: Option<UiConversationDialogSprites>,
}

impl Default for UiConversationDialogState {
    fn default() -> Self {
        Self {
            dialog_instance: DialogInstance::new("DLGDIALOG.XML"),
            sprites: None,
        }
    }
}

pub fn conversation_dialog_system(
    mut current_dialog_state: Local<Option<ConversationDialogState>>,
    mut egui_context: EguiContexts,
    mut conversation_dialog_events: EventReader<ConversationDialogEvent>,
    mut ui_state: Local<UiConversationDialogState>,
    script_function_resources: ScriptFunctionResources,
    query_player_position: Query<&Position, With<PlayerCharacter>>,
    query_position: Query<&Position>,
    query_name: Query<&ClientEntityName>,
    lua_game_constants: Res<LuaGameConstants>,
    lua_game_functions: Res<LuaGameFunctions>,
    lua_quest_functions: Res<LuaQuestFunctions>,
    game_data: Res<GameData>,
    vfs_resource: Res<VfsResource>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = ui_state
        .dialog_instance
        .get_mut(&dialog_assets, &ui_resources)
    {
        dialog
    } else {
        return;
    };

    if ui_state.sprites.is_none() {
        ui_state.sprites = (|| {
            Some(UiConversationDialogSprites {
                message_top: ui_resources.get_sprite(0, "UI13_NPC_SCRIPT_IMAGE_TOP")?,
                message_middle: ui_resources.get_sprite(0, "UI13_NPC_SCRIPT_IMAGE_MIDDLE")?,
                message_bottom: ui_resources.get_sprite(0, "UI13_NPC_SCRIPT_IMAGE_BOTTOM")?,
                answer_top: ui_resources.get_sprite(0, "UI13_NPC_SCRIPT_ANSWER_TOP")?,
                answer_middle: ui_resources.get_sprite(0, "UI13_NPC_SCRIPT_ANSWER_MIDDLE")?,
                answer_bottom: ui_resources.get_sprite(0, "UI13_NPC_SCRIPT_ANSWER_BOTTOM")?,
            })
        })();
        if ui_state.sprites.is_none() {
            return;
        }
    }
    let dialog_sprites = ui_state.sprites.as_ref().unwrap();

    // TODO: Fix ScriptFunctionContext and LuaVMContext usage for Bevy 0.13
    // The system is currently disabled due to complex lifetime issues with ScriptFunctionContext
    // This needs to be refactored to work with Bevy 0.13's SystemParam requirements
    return;
}
