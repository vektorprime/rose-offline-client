use std::time::Instant;

use bevy::prelude::{Assets, Entity, Local, MessageReader, MessageWriter, Query, Res, ResMut, Resource, With};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::PlayerCharacter,
    events::{ChatboxEvent, FlightToggleEvent, PingRequestEvent, PingState},
    resources::{GameConnection, UiResources},
    systems::{parse_chat_input, is_fly_command, is_ping_command},
    ui::{
        widgets::{DataBindings, Dialog},
        UiSoundEvent,
    },
};

const MAX_CHATBOX_ENTRIES: usize = 100;

// TODO: Implement the chat filters
// const IID_BTN_FILTER: i32 = 10;
const IID_EDITBOX: i32 = 15;

const IID_CHAT_LIST_IMAGE: i32 = 6;

const IID_LISTBOX_ALL: i32 = 20;
const IID_SCROLLBAR_ALL: i32 = 21;

const IID_LISTBOX_WHISPER: i32 = 25;
const IID_SCROLLBAR_WHISPER: i32 = 26;

const IID_LISTBOX_TRADE: i32 = 30;
const IID_SCROLLBAR_TRADE: i32 = 31;

const IID_LISTBOX_PARTY: i32 = 35;
const IID_SCROLLBAR_PARTY: i32 = 36;

const IID_LISTBOX_CLAN: i32 = 40;
const IID_SCROLLBAR_CLAN: i32 = 41;

const IID_LISTBOX_ALLIED: i32 = 45;
const IID_SCROLLBAR_ALLIED: i32 = 46;

const IID_RADIOBOX: i32 = 50;
const IID_BTN_ALL: i32 = 51;
const IID_BTN_WHISPER: i32 = 52;
const IID_BTN_TRADE: i32 = 53;
const IID_BTN_PARTY: i32 = 54;
const IID_BTN_CLAN: i32 = 55;
const IID_BTN_ALLIED: i32 = 56;

const CHAT_COLOR_TIMESTAMP: egui::Color32 = egui::Color32::from_rgb(150, 150, 150);
const CHAT_COLOR_NORMAL: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const CHAT_COLOR_SHOUT: egui::Color32 = egui::Color32::from_rgb(189, 250, 255);
const CHAT_COLOR_WHISPER: egui::Color32 = egui::Color32::from_rgb(201, 255, 144);
const CHAT_COLOR_ANNOUNCE: egui::Color32 = egui::Color32::from_rgb(255, 188, 172);
const CHAT_COLOR_PARTY: egui::Color32 = egui::Color32::from_rgb(255, 237, 140);
const CHAT_COLOR_SYSTEM: egui::Color32 = egui::Color32::from_rgb(255, 224, 229);
const CHAT_COLOR_QUEST: egui::Color32 = egui::Color32::from_rgb(151, 221, 241);
const CHAT_COLOR_ALLIED: egui::Color32 = egui::Color32::from_rgb(255, 228, 122);
const CHAT_COLOR_CLAN: egui::Color32 = egui::Color32::from_rgb(255, 228, 122);

pub struct UiStateChatbox {
    textbox_text: String,
    textbox_layout_job: egui::text::LayoutJob,
    cleanup_layout_text_counter: usize,
    selected_channel: i32,
    show_command_help: bool,
}

impl Default for UiStateChatbox {
    fn default() -> Self {
        Self {
            textbox_text: Default::default(),
            textbox_layout_job: Default::default(),
            cleanup_layout_text_counter: 0,
            selected_channel: IID_BTN_ALL,
            show_command_help: false,
        }
    }
}

pub fn ui_chatbox_system(
    mut egui_context: EguiContexts,
    mut ui_state_chatbox: Local<UiStateChatbox>,
    mut chatbox_events: MessageReader<ChatboxEvent>,
    game_connection: Option<Res<GameConnection>>,
    ui_resources: Res<UiResources>,
    mut ui_sound_events: MessageWriter<UiSoundEvent>,
    dialog_assets: Res<Assets<Dialog>>,
    mut flight_toggle_events: MessageWriter<FlightToggleEvent>,
    mut ping_request_events: MessageWriter<PingRequestEvent>,
    mut ping_state: ResMut<PingState>,
    player_query: Query<Entity, With<PlayerCharacter>>,
) {
    let ui_state_chatbox = &mut *ui_state_chatbox;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_chatbox) {
        dialog
    } else {
        return;
    };

    let local_time = chrono::Local::now();
    let timestamp = local_time.format("%H:%M:%S");

    for event in chatbox_events.read() {
        if ui_state_chatbox.textbox_layout_job.sections.len() == MAX_CHATBOX_ENTRIES {
            ui_state_chatbox.textbox_layout_job.sections.remove(0);
            ui_state_chatbox.cleanup_layout_text_counter += 1;

            if ui_state_chatbox.cleanup_layout_text_counter == MAX_CHATBOX_ENTRIES {
                let offset = ui_state_chatbox.textbox_layout_job.sections[0]
                    .byte_range
                    .start;
                ui_state_chatbox.textbox_layout_job.text =
                    ui_state_chatbox.textbox_layout_job.text.split_off(offset);

                for section in ui_state_chatbox.textbox_layout_job.sections.iter_mut() {
                    section.byte_range.start -= offset;
                    section.byte_range.end -= offset;
                }

                ui_state_chatbox.cleanup_layout_text_counter = 0;
            }
        }

        ui_state_chatbox.textbox_layout_job.append(
            &format!("[{}] ", timestamp),
            0.0,
            egui::TextFormat {
                color: CHAT_COLOR_TIMESTAMP,
                ..Default::default()
            },
        );

        match event {
            ChatboxEvent::Say(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_NORMAL,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Shout(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_SHOUT,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Whisper(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_WHISPER,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Party(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_PARTY,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Clan(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_CLAN,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Allied(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_ALLIED,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Announce(Some(name), text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_ANNOUNCE,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Announce(None, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_ANNOUNCE,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::System(text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_SYSTEM,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Quest(text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_QUEST,
                        ..Default::default()
                    },
                );
            }
        }
    }

    let mut chatbox_style = (*egui_context.ctx_mut().unwrap().style()).clone();
    chatbox_style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(
        chatbox_style.visuals.widgets.noninteractive.bg_fill.r(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.g(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.b(),
        128,
    );

    let style = egui_context.ctx_mut().unwrap().style();
    let frame_fill = style.visuals.window_fill();
    let frame_fill =
        egui::Color32::from_rgba_unmultiplied(frame_fill.r(), frame_fill.g(), frame_fill.b(), 128);

    let mut response_editbox = None;
    let mut response_all_button = None;
    let mut response_whisper_button = None;
    let mut response_trade_button = None;
    let mut response_party_button = None;
    let mut response_clan_button = None;
    let mut response_allied_button = None;

    egui::Window::new("Chat Box")
        .anchor(egui::Align2::LEFT_BOTTOM, [0.0, 0.0])
        .frame(egui::Frame::none().fill(frame_fill))
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut().unwrap(), |ui| {
            ui.visuals_mut().override_text_color =
                match ui_state_chatbox.textbox_text.chars().next() {
                    Some('!') => Some(CHAT_COLOR_SHOUT),
                    Some('@') => Some(CHAT_COLOR_WHISPER),
                    Some('#') => Some(CHAT_COLOR_PARTY),
                    Some('&') => Some(CHAT_COLOR_CLAN),
                    Some('~') => Some(CHAT_COLOR_ALLIED),
                    _ => Some(CHAT_COLOR_NORMAL),
                };

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    text: &mut [(IID_EDITBOX, &mut ui_state_chatbox.textbox_text)],
                    radio: &mut [(IID_RADIOBOX, &mut ui_state_chatbox.selected_channel)],
                    response: &mut [
                        (IID_EDITBOX, &mut response_editbox),
                        (IID_BTN_ALL, &mut response_all_button),
                        (IID_BTN_WHISPER, &mut response_whisper_button),
                        (IID_BTN_TRADE, &mut response_trade_button),
                        (IID_BTN_PARTY, &mut response_party_button),
                        (IID_BTN_CLAN, &mut response_clan_button),
                        (IID_BTN_ALLIED, &mut response_allied_button),
                    ],
                    visible: &mut [
                        (IID_CHAT_LIST_IMAGE, false),
                        (IID_LISTBOX_ALL, false),
                        (IID_SCROLLBAR_ALL, false),
                        (IID_LISTBOX_WHISPER, false),
                        (IID_SCROLLBAR_WHISPER, false),
                        (IID_LISTBOX_TRADE, false),
                        (IID_SCROLLBAR_TRADE, false),
                        (IID_LISTBOX_PARTY, false),
                        (IID_SCROLLBAR_PARTY, false),
                        (IID_LISTBOX_CLAN, false),
                        (IID_SCROLLBAR_CLAN, false),
                        (IID_LISTBOX_ALLIED, false),
                        (IID_SCROLLBAR_ALLIED, false),
                    ],
                    ..Default::default()
                },
                |ui, _bindings| {
                    ui.allocate_ui_at_rect(
                        egui::Rect::from_min_size(
                            ui.min_rect().min + egui::vec2(1.0, 0.0),
                            egui::vec2(390.0, 179.0),
                        ),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.label(ui_state_chatbox.textbox_layout_job.clone());
                                });
                        },
                    );
                },
            );

        // Show command help tooltip when typing "/"
        let is_typing_command = ui_state_chatbox.textbox_text.starts_with('/');
        if is_typing_command {
            // Update state to show help
            ui_state_chatbox.show_command_help = true;
        } else {
            ui_state_chatbox.show_command_help = false;
        }
        });

    // Show command help popup above the chatbox
    if ui_state_chatbox.show_command_help {
        egui::Area::new(egui::Id::new("chat_command_help"))
            .anchor(egui::Align2::LEFT_BOTTOM, [5.0, -165.0])
            .order(egui::Order::Foreground)
            .show(egui_context.ctx_mut().unwrap(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            ui.set_min_width(380.0);
                            ui.label(egui::RichText::new("Chat Commands (press Esc to close)")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 220, 100)));
                            ui.separator();
                            egui::Grid::new("command_help_grid")
                                .num_columns(1)
                                .spacing([4.0, 2.0])
                                .show(ui, |ui| {
                                    // Client-side commands
                                    ui.label(egui::RichText::new("Client-side (local):").strong().color(egui::Color32::from_rgb(150, 200, 255)));
                                    ui.end_row();
                                    ui.label("  /fly - Toggle flight mode");
                                    ui.end_row();
                                    ui.label("  /ping - Show latency to server");
                                    ui.end_row();
                                    
                                    // Server-side commands - Character
                                    ui.label(egui::RichText::new("Server-side - Character:").strong().color(egui::Color32::from_rgb(150, 255, 150)));
                                    ui.end_row();
                                    ui.label("  /help - List all server commands");
                                    ui.end_row();
                                    ui.label("  /where - Show current position");
                                    ui.end_row();
                                    ui.label("  /god - Toggle invincibility");
                                    ui.end_row();
                                    ui.label("  /ghost - Toggle ghost mode");
                                    ui.end_row();
                                    ui.label("  /heal - Fully restore HP/MP/Stamina");
                                    ui.end_row();
                                    ui.label("  /revive - Revive yourself");
                                    ui.end_row();
                                    ui.label("  /save - Force save character");
                                    ui.end_row();
                                    ui.label("  /speed <value> - Set move speed (e.g., /speed 4000)");
                                    ui.end_row();
                                    ui.label("  /mspeed <value> - Set move speed (e.g., /mspeed 1000)");
                                    ui.end_row();
                                    ui.label("  /level <level> - Set level");
                                    ui.end_row();
                                    ui.label("  /money <amount> - Set money (e.g., /money 1000000)");
                                    ui.end_row();
                                    
                                    // Server-side commands - Teleportation
                                    ui.label(egui::RichText::new("Server-side - Teleportation:").strong().color(egui::Color32::from_rgb(200, 150, 255)));
                                    ui.end_row();
                                    ui.label("  /tp <player> - Teleport to player (e.g., /tp John)");
                                    ui.end_row();
                                    ui.label("  /mm <zone> [x] [y] - Teleport to zone (e.g., /mm 1)");
                                    ui.end_row();
                                    ui.label("  /zonelist - List all zones with IDs");
                                    ui.end_row();
                                    
                                    // Server-side commands - Server
                                    ui.label(egui::RichText::new("Server-side - Server:").strong().color(egui::Color32::from_rgb(255, 200, 150)));
                                    ui.end_row();
                                    ui.label("  /players - List all online players");
                                    ui.end_row();
                                    ui.label("  /who - Show nearby players");
                                    ui.end_row();
                                    ui.label("  /serverinfo - Show server stats");
                                    ui.end_row();
                                    ui.label("  /announce <msg> - Global announcement (e.g., /announce Hello!)");
                                    ui.end_row();
                                    ui.label("  /info - Debug entity info under cursor");
                                    ui.end_row();
                                    
                                    // Server-side commands - Spawning
                                    ui.label(egui::RichText::new("Server-side - Spawning:").strong().color(egui::Color32::from_rgb(255, 150, 200)));
                                    ui.end_row();
                                    ui.label("  /mon <id> <count> - Spawn monsters (e.g., /mon 1 5)");
                                    ui.end_row();
                                    ui.label("  /item <type> <id> [qty] - Give item (e.g., /item 1 100 1)");
                                    ui.end_row();
                                    ui.label("  /bot <n> - Spawn bot characters");
                                    ui.end_row();
                                    
                                    // Server-side commands - Skills/Stats
                                    ui.label(egui::RichText::new("Server-side - Skills/Stats:").strong().color(egui::Color32::from_rgb(150, 255, 200)));
                                    ui.end_row();
                                    ui.label("  /skill add|remove <id> - Add/remove skill");
                                    ui.end_row();
                                    ui.label("  /add <ability> <value> - Add to ability");
                                    ui.end_row();
                                    ui.label("  /set <ability> <value> - Set ability value");
                                    ui.end_row();
                                    ui.label("  /rate <type> <value> - Set rates");
                                    ui.end_row();
                                    
                                    // Chat prefixes
                                    ui.label(egui::RichText::new("Chat prefixes:").strong().color(egui::Color32::from_rgb(255, 180, 100)));
                                    ui.end_row();
                                    ui.label("  ! - Shout (all players)");
                                    ui.end_row();
                                    ui.label("  @ - Whisper (private)");
                                    ui.end_row();
                                    ui.label("  # - Party  & - Clan  ~ - Allied");
                                    ui.end_row();
                                });
                        });
                });
            });
    }

    // Hide command help when Escape is pressed
    if egui_context.ctx_mut().unwrap().input(|input| input.key_pressed(egui::Key::Escape)) {
        ui_state_chatbox.show_command_help = false;
    }

    if let Some(response) = response_editbox {
        if response
            .ctx
            .input(|input| input.key_pressed(egui::Key::Enter))
        {
            if response.lost_focus() {
                if !ui_state_chatbox.textbox_text.is_empty() {
                    // Check if this is a "/fly" command before sending to server
                    if is_fly_command(&ui_state_chatbox.textbox_text) {
                        // Get the player entity and send flight toggle event
                        if let Ok(player_entity) = player_query.single() {
                            flight_toggle_events.write(FlightToggleEvent {
                                entity: player_entity,
                            });
                        }
                        // Clear the textbox without sending to server
                        ui_state_chatbox.textbox_text.clear();
                    } else if is_ping_command(&ui_state_chatbox.textbox_text) {
                        // Handle /ping command client-side
                        // Record the timestamp and send a ping request
                        ping_state.pending_ping_timestamp = Some(Instant::now());
                        
                        // Send a chat message to server to measure RTT
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::Chat {
                                    text: "/ping".to_string(),
                                })
                                .ok();
                        }
                        ui_state_chatbox.textbox_text.clear();
                    } else {
                        // Parse the chat input to detect chat type for logging
                        let parsed = parse_chat_input(&ui_state_chatbox.textbox_text);
                        
                        // Log the parsed result for debugging
                        tracing::debug!(
                            "Chat sent - Type: {:?}, Target: {:?}, Message: '{}'",
                            parsed.chat_type,
                            parsed.target,
                            parsed.message
                        );
                        
                        // Send the full text to the server (including prefix)
                        // The server handles prefix routing to appropriate chat channels
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::Chat {
                                    text: ui_state_chatbox.textbox_text.clone(),
                                })
                                .ok();
                            ui_state_chatbox.textbox_text.clear();
                        }
                    }
                }
            } else {
                response.request_focus();
            }
        }
    }

    // TODO: Update filters when changing category
    if response_all_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
    }

    if response_whisper_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('@');
    }

    if response_trade_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
    }

    if response_party_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('#');
    }

    if response_clan_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('&');
    }

    if response_allied_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('~');
    }
}
