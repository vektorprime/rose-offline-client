use bevy::{
    prelude::{ButtonInput, KeyCode, Local, Res, ResMut, Resource},
    ecs::system::SystemParam,
};
use bevy_egui::{egui, EguiContexts};
use regex::Regex;
use rose_data::{ItemReference, ItemType};
use rose_data_irose::encode_item_type;
use rose_game_common::messages::client::ClientMessage;

use crate::{
    events::{ChatboxEvent, FlightToggleEvent, PingState},
    resources::{GameConnection, GameData, UiResources, UiSpriteSheetType},
    systems::is_fly_command,
};

/// Resource to track admin menu state
#[derive(Resource)]
pub struct UiStateAdminMenu {
    pub admin_menu_open: bool,
    
    // Input fields
    pub tp_player_name: String,
    pub announce_message: String,
    pub money_amount: String,
    pub speed_value: String,
    pub mm_zone_id: String,
    pub mm_x: String,
    pub mm_y: String,
    pub mon_id: String,
    pub mon_count: String,
    pub item_type: String,
    pub item_id: String,
    pub item_qty: String,
    
    // Toggle states
    pub god_mode: bool,
    pub ghost_mode: bool,
    
    // Item popup state
    pub show_item_popup: bool,
    pub selected_item_type: ItemType,
    pub item_search_filter: String,
    filtered_items: Vec<u16>,
}

impl Default for UiStateAdminMenu {
    fn default() -> Self {
        Self {
            admin_menu_open: false,
            tp_player_name: String::new(),
            announce_message: String::new(),
            money_amount: String::new(),
            speed_value: String::new(),
            mm_zone_id: String::new(),
            mm_x: String::new(),
            mm_y: String::new(),
            mon_id: String::new(),
            mon_count: String::new(),
            item_type: String::new(),
            item_id: String::new(),
            item_qty: String::new(),
            god_mode: false,
            ghost_mode: false,
            show_item_popup: false,
            selected_item_type: ItemType::Face,
            item_search_filter: String::new(),
            filtered_items: Vec::new(),
        }
    }
}

/// System that handles keyboard shortcut for admin menu
pub fn admin_menu_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state_admin_menu: ResMut<UiStateAdminMenu>,
) {
    // F10 to toggle admin menu
    if keyboard.just_pressed(KeyCode::F10) {
        ui_state_admin_menu.admin_menu_open = !ui_state_admin_menu.admin_menu_open;
    }
}

/// System that renders the admin menu UI
#[allow(clippy::too_many_arguments)]
pub fn ui_admin_menu_system(
    mut egui_context: EguiContexts,
    mut ui_state_admin_menu: ResMut<UiStateAdminMenu>,
    mut chatbox_events: bevy::prelude::MessageWriter<ChatboxEvent>,
    game_connection: Option<Res<GameConnection>>,
    ping_state: Res<PingState>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
) {
    if !ui_state_admin_menu.admin_menu_open {
        return;
    }

    let ctx = egui_context.ctx_mut().unwrap();
    
    egui::Window::new("Admin Menu (F10)")
        .default_width(350.0)
        .resizable(true)
        .show(&*ctx, |ui| {
            // Display current ping if available
            if let Some(ping_ms) = ping_state.last_ping_ms {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Ping: {} ms", ping_ms))
                        .color(egui::Color32::from_rgb(100, 255, 100)));
                });
                ui.separator();
            }
            
            // === Character Section ===
            ui.collapsing("Character", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("God Mode").clicked() {
                        send_command(&game_connection, "/god");
                        ui_state_admin_menu.god_mode = !ui_state_admin_menu.god_mode;
                    }
                    if ui.button("Ghost Mode").clicked() {
                        send_command(&game_connection, "/ghost");
                        ui_state_admin_menu.ghost_mode = !ui_state_admin_menu.ghost_mode;
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Heal").clicked() {
                        send_command(&game_connection, "/heal");
                    }
                    if ui.button("Revive").clicked() {
                        send_command(&game_connection, "/revive");
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        send_command(&game_connection, "/save");
                    }
                });
                
                ui.separator();
                
                // Speed setting
                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.speed_value);
                    if ui.button("Set").clicked() {
                        if !ui_state_admin_menu.speed_value.is_empty() {
                            let cmd = format!("/speed {}", ui_state_admin_menu.speed_value);
                            send_command(&game_connection, &cmd);
                        }
                    }
                });
                
                // Money setting
                ui.horizontal(|ui| {
                    ui.label("Money:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.money_amount);
                    if ui.button("Set").clicked() {
                        if !ui_state_admin_menu.money_amount.is_empty() {
                            let cmd = format!("/money {}", ui_state_admin_menu.money_amount);
                            send_command(&game_connection, &cmd);
                        }
                    }
                });
            });
            
            // === Teleportation Section ===
            ui.collapsing("Teleportation", |ui| {
                // Teleport to player
                ui.horizontal(|ui| {
                    ui.label("TP to:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.tp_player_name);
                    if ui.button("Go").clicked() {
                        if !ui_state_admin_menu.tp_player_name.is_empty() {
                            let cmd = format!("/tp {}", ui_state_admin_menu.tp_player_name);
                            send_command(&game_connection, &cmd);
                        }
                    }
                });
                
                // Map move to zone
                ui.horizontal(|ui| {
                    ui.label("Zone ID:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.mm_zone_id);
                });
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.mm_x);
                    ui.label("Y:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.mm_y);
                });
                ui.horizontal(|ui| {
                    if ui.button("Teleport to Zone").clicked() {
                        if !ui_state_admin_menu.mm_zone_id.is_empty() {
                            let mut cmd = format!("/mm {}", ui_state_admin_menu.mm_zone_id);
                            if !ui_state_admin_menu.mm_x.is_empty() {
                                cmd.push_str(&format!(" {}", ui_state_admin_menu.mm_x));
                            }
                            if !ui_state_admin_menu.mm_y.is_empty() {
                                cmd.push_str(&format!(" {}", ui_state_admin_menu.mm_y));
                            }
                            send_command(&game_connection, &cmd);
                        }
                    }
                    if ui.button("Zone List").clicked() {
                        send_command(&game_connection, "/zonelist");
                    }
                });
                
                if ui.button("Where (Show Position)").clicked() {
                    send_command(&game_connection, "/where");
                }
            });
            
            // === Server Section ===
            ui.collapsing("Server", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Players Online").clicked() {
                        send_command(&game_connection, "/players");
                    }
                    if ui.button("Nearby Players").clicked() {
                        send_command(&game_connection, "/who");
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Server Info").clicked() {
                        send_command(&game_connection, "/serverinfo");
                    }
                    if ui.button("Ping").clicked() {
                        send_command(&game_connection, "/ping");
                    }
                });
                
                ui.separator();
                
                // Announce
                ui.label("Announcement:");
                ui.text_edit_multiline(&mut ui_state_admin_menu.announce_message);
                ui.horizontal(|ui| {
                    if ui.button("Send Announcement").clicked() {
                        if !ui_state_admin_menu.announce_message.is_empty() {
                            let cmd = format!("/announce {}", ui_state_admin_menu.announce_message);
                            send_command(&game_connection, &cmd);
                            ui_state_admin_menu.announce_message.clear();
                        }
                    }
                });
            });
            
            // === Spawning Section ===
            ui.collapsing("Spawning", |ui| {
                // Item spawner popup button
                if ui.button("📦 Give Item (Popup)").clicked() {
                    ui_state_admin_menu.show_item_popup = true;
                }
                
                ui.separator();
                
                // Spawn monsters
                ui.horizontal(|ui| {
                    ui.label("Monster ID:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.mon_id);
                    ui.label("Count:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.mon_count);
                });
                ui.horizontal(|ui| {
                    if ui.button("Spawn Monsters").clicked() {
                        if !ui_state_admin_menu.mon_id.is_empty() && !ui_state_admin_menu.mon_count.is_empty() {
                            let cmd = format!("/mon {} {}", ui_state_admin_menu.mon_id, ui_state_admin_menu.mon_count);
                            send_command(&game_connection, &cmd);
                        }
                    }
                });
                
                ui.separator();
                
                // Give items (manual entry)
                ui.label("Manual Item Entry:");
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.item_type);
                    ui.label("ID:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.item_id);
                });
                ui.horizontal(|ui| {
                    ui.label("Qty:");
                    ui.text_edit_singleline(&mut ui_state_admin_menu.item_qty);
                });
                ui.horizontal(|ui| {
                    if ui.button("Give Item").clicked() {
                        if !ui_state_admin_menu.item_type.is_empty() && !ui_state_admin_menu.item_id.is_empty() {
                            let mut cmd = format!("/item {} {}", ui_state_admin_menu.item_type, ui_state_admin_menu.item_id);
                            if !ui_state_admin_menu.item_qty.is_empty() {
                                cmd.push_str(&format!(" {}", ui_state_admin_menu.item_qty));
                            }
                            send_command(&game_connection, &cmd);
                        }
                    }
                });
            });
            
            // === Quick Commands ===
            ui.collapsing("Quick Commands", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Help").clicked() {
                        send_command(&game_connection, "/help");
                    }
                    if ui.button("Info (Entity under cursor)").clicked() {
                        send_command(&game_connection, "/info");
                    }
                });
            });
        });
    
    // Render item spawner popup
    if ui_state_admin_menu.show_item_popup {
        render_item_spawner_popup(
            ctx,
            &mut ui_state_admin_menu,
            &game_data,
            &ui_resources,
            &game_connection,
        );
    }
}

/// Renders the item spawner popup window
fn render_item_spawner_popup(
    ctx: &egui::Context,
    ui_state: &mut UiStateAdminMenu,
    game_data: &Res<GameData>,
    ui_resources: &Res<UiResources>,
    game_connection: &Option<Res<GameConnection>>,
) {
    let mut show_popup = true;
    
    egui::Window::new("Item Spawner")
        .default_width(500.0)
        .default_height(400.0)
        .resizable(true)
        .open(&mut show_popup)
        .show(ctx, |ui| {
            // Category tabs - First row
            ui.horizontal(|ui| {
                let tabs_row1 = [
                    (ItemType::Face, "Face"),
                    (ItemType::Head, "Head"),
                    (ItemType::Body, "Body"),
                    (ItemType::Hands, "Hands"),
                    (ItemType::Feet, "Feet"),
                    (ItemType::Back, "Back"),
                    (ItemType::Jewellery, "Jewellery"),
                ];
                
                for (item_type, label) in tabs_row1 {
                    let selected = ui_state.selected_item_type == item_type;
                    if ui.selectable_label(selected, label).clicked() {
                        ui_state.selected_item_type = item_type;
                        ui_state.filtered_items.clear();
                    }
                }
            });
            
            // Category tabs - Second row
            ui.horizontal(|ui| {
                let tabs_row2 = [
                    (ItemType::Weapon, "Weapon"),
                    (ItemType::SubWeapon, "SubWeapon"),
                    (ItemType::Consumable, "Consumable"),
                    (ItemType::Gem, "Gem"),
                    (ItemType::Material, "Material"),
                    (ItemType::Quest, "Quest"),
                    (ItemType::Vehicle, "Vehicle"),
                ];
                
                for (item_type, label) in tabs_row2 {
                    let selected = ui_state.selected_item_type == item_type;
                    if ui.selectable_label(selected, label).clicked() {
                        ui_state.selected_item_type = item_type;
                        ui_state.filtered_items.clear();
                    }
                }
            });
            
            ui.separator();
            
            // Search filter
            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut ui_state.item_search_filter);
                if response.changed() {
                    ui_state.filtered_items.clear();
                }
                if ui.button("Clear").clicked() {
                    ui_state.item_search_filter.clear();
                    ui_state.filtered_items.clear();
                }
            });
            
            ui.separator();
            
            // Update filtered items if needed
            if ui_state.filtered_items.is_empty() {
                update_filtered_items(ui_state, game_data);
            }
            
            // Scrollable item list
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("item_spawner_grid")
                    .num_columns(4)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        // Header
                        ui.label(egui::RichText::new("Icon").strong());
                        ui.label(egui::RichText::new("ID").strong());
                        ui.label(egui::RichText::new("Name").strong());
                        ui.label(egui::RichText::new("Action").strong());
                        ui.end_row();
                        
                        // Items
                        for &item_id in &ui_state.filtered_items {
                            let item_reference = ItemReference::new(ui_state.selected_item_type, item_id as usize);
                            
                            if let Some(item_data) = game_data.items.get_base_item(item_reference) {
                                // Icon
                                if let Some(sprite) = ui_resources.get_sprite_by_index(
                                    UiSpriteSheetType::Item,
                                    item_data.icon_index as usize,
                                ) {
                                    ui.add(
                                        egui::Image::new((sprite.texture_id, egui::Vec2::new(32.0, 32.0)))
                                            .uv(sprite.uv)
                                    );
                                } else {
                                    ui.allocate_space(egui::Vec2::new(32.0, 32.0));
                                }
                                
                                // ID
                                ui.label(format!("{}", item_id));
                                
                                // Name
                                ui.label(&item_data.name);
                                
                                // Spawn button
                                if ui.button("Give").clicked() {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        if let Some(item_type_id) = encode_item_type(ui_state.selected_item_type) {
                                            let command = format!("/item {} {} 1", item_type_id, item_id);
                                            game_connection
                                                .client_message_tx
                                                .send(ClientMessage::Chat {
                                                    text: command,
                                                })
                                                .ok();
                                        }
                                    }
                                }
                                
                                ui.end_row();
                            }
                        }
                    });
            });
        });
    
    ui_state.show_item_popup = show_popup;
}

/// Updates the filtered items list based on current filter settings
fn update_filtered_items(ui_state: &mut UiStateAdminMenu, game_data: &Res<GameData>) {
    let filter_name_re = if !ui_state.item_search_filter.is_empty() {
        Some(
            Regex::new(&format!(
                "(?i){}",
                regex::escape(&ui_state.item_search_filter)
            ))
            .unwrap(),
        )
    } else {
        None
    };
    
    ui_state.filtered_items = game_data
        .items
        .iter_items(ui_state.selected_item_type)
        .filter_map(|item_reference| {
            game_data
                .items
                .get_base_item(item_reference)
                .map(|item_data| (item_reference, item_data))
        })
        .filter_map(|(item_reference, item_data)| {
            // Filter out items with empty names or names that don't match filter
            if item_data.name.is_empty()
                || !filter_name_re.as_ref().map_or(true, |re| re.is_match(&item_data.name))
            {
                None
            } else {
                Some(item_reference.item_number as u16)
            }
        })
        .collect();
}

/// Helper function to send a command to the server
fn send_command(game_connection: &Option<Res<GameConnection>>, command: &str) {
    if let Some(game_connection) = game_connection.as_ref() {
        game_connection
            .client_message_tx
            .send(ClientMessage::Chat {
                text: command.to_string(),
            })
            .ok();
    }
}
