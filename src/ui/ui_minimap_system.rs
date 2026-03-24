use std::sync::Arc;

use bevy::{
    asset::Assets,
    input::ButtonInput,
    math::{Vec2, Vec3Swizzles},
    prelude::{
        AssetServer, Camera3d, Handle, Image, KeyCode, Local, MessageWriter, Query, Res, Transform,
        Vec3, With, Without,
    },
    window::PrimaryWindow,
};
use bevy_egui::{egui, EguiContexts};

use rose_data::ZoneId;
use rose_game_common::components::{CharacterInfo, Npc, Team};

use crate::{
    components::{ClientEntity, ClientEntityType, PartyInfo, PlayerCharacter, Position},
    resources::{CurrentZone, GameData, UiResources, UiSpriteSheetType},
    ui::{
        widgets::{DataBindings, Dialog, Widget},
        UiSoundEvent,
    },
    zone_loader::{ZoneLoaderAsset, ZoneNpc},
};

// Original map design constants (for original resolution images)
const ORIGINAL_MAP_BLOCK_PIXELS: f32 = 64.0;
const ORIGINAL_MAP_OUTLINE_PIXELS: f32 = ORIGINAL_MAP_BLOCK_PIXELS;

const ZONE_NAME_WIDTH: f32 = 102.0;
const ZONE_NAME_EXPANDED_WIDTH: f32 = 172.0;

const IID_PANE_BIG: i32 = 50;
// const IID_CAPTION_BIG: i32 = 51;
const IID_BTN_NORMAL: i32 = 52;
const IID_BTN_MINIMIZE_BIG: i32 = 53;
const IID_PANE_BIG_CHILDPANE: i32 = 60;
const IID_PANE_SMALL: i32 = 100;
// const IID_CAPTION_SMALL: i32 = 101;
const IID_BTN_EXPAND: i32 = 102;
const IID_BTN_MINIMIZE_SMALL: i32 = 103;
const IID_PANE_SMALL_CHILDPANE: i32 = 110;

// Default configuration values
const DEFAULT_WINDOW_SIZE: Vec2 = Vec2::new(200.0, 200.0);
const MIN_WINDOW_SIZE: Vec2 = Vec2::new(150.0, 150.0);
const MAX_WINDOW_SIZE: Vec2 = Vec2::new(800.0, 800.0);
const CENTERED_WINDOW_SIZE: Vec2 = Vec2::new(600.0, 600.0);
const DEFAULT_ZOOM: f32 = 1.5;
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 4.0;
const TITLE_BAR_HEIGHT: f32 = 21.0;
const TOGGLE_BAR_HEIGHT: f32 = 24.0;
const COORDS_BAR_HEIGHT: f32 = 17.0;

#[derive(Default)]
pub struct UiStateMinimap {
    pub zone_id: Option<ZoneId>,
    pub minimap_image: Handle<Image>,
    pub minimap_texture: egui::TextureId,
    pub minimap_image_size: Option<Vec2>,
    pub min_world_pos: Vec2,
    pub max_world_pos: Vec2,
    pub distance_per_pixel: f32,
    pub last_player_position: Vec2,
    pub is_expanded: bool,
    pub is_minimised: bool,
    pub scroll: Vec2,
    pub zone_name_pixels_per_point: f32,
    pub zone_name_text_galley: Option<Arc<egui::Galley>>,
    pub zone_name_text_expanded_galley: Option<Arc<egui::Galley>>,

    // Window state
    pub window_size: Vec2,
    pub first_frame: bool,

    // Zoom state
    pub zoom_level: f32,
    pub follow_player: bool,

    // Icon visibility toggles
    pub show_players: bool,
    pub show_npcs: bool,
    pub show_monsters: bool,

    // Image scale factor (for upscaled images)
    // 1.0 = original resolution, 2.0 = 2x upscaled, 4.0 = 4x upscaled
    pub image_scale: f32,
    
    // Scaled outline pixels (original * scale)
    pub scaled_outline_pixels: f32,
    
    // Centered mode (for ALT+M toggle)
    pub is_centered: bool,
}

impl UiStateMinimap {
    fn new() -> Self {
        Self {
            window_size: DEFAULT_WINDOW_SIZE,
            zoom_level: DEFAULT_ZOOM,
            follow_player: true,
            show_players: true,
            show_npcs: true,
            show_monsters: true,
            first_frame: true,
            image_scale: 1.0,
            scaled_outline_pixels: ORIGINAL_MAP_OUTLINE_PIXELS,
            is_centered: false,
            ..Default::default()
        }
    }
}

fn generate_text_galley(
    ctx: &egui::Context,
    width: f32,
    height: f32,
    text: String,
) -> Arc<egui::Galley> {
    let style = ctx.style();
    let text_format = egui::text::TextFormat {
        font_id: egui::FontSelection::Default.resolve(&style),
        color: egui::Color32::WHITE,
        background: egui::Color32::TRANSPARENT,
        italics: false,
        underline: egui::Stroke::NONE,
        strikethrough: egui::Stroke::NONE,
        valign: egui::Align::Center,
        extra_letter_spacing: 0.0,
        line_height: None,
        expand_bg: 0.0,
    };

    let mut text_job = egui::text::LayoutJob::single_section(text, text_format);
    text_job.first_row_min_height = height;
    text_job.wrap.max_width = width;
    text_job.wrap.max_rows = 1;
    text_job.wrap.break_anywhere = true;

    ctx.layer_painter(egui::LayerId::background()).layout_job(text_job)
}

/// Check if an entity is a monster (NPC with hostile team)
fn is_monster_entity(client_entity: &ClientEntity, team: &Team, player_team: &Team) -> bool {
    client_entity.entity_type == ClientEntityType::Monster
        || (client_entity.entity_type == ClientEntityType::Npc && team.id != player_team.id && team.id != Team::DEFAULT_NPC_TEAM_ID)
}

pub fn ui_minimap_system(
    mut egui_context: EguiContexts,
    mut ui_state: Local<UiStateMinimap>,
    mut ui_sound_events: MessageWriter<UiSoundEvent>,
    query_player: Query<(&Position, &Team, Option<&PartyInfo>), With<PlayerCharacter>>,
    query_characters: Query<(&CharacterInfo, &Position, &Team), Without<PlayerCharacter>>,
    // Query for monsters (NPCs that are hostile)
    query_monsters: Query<(&Position, &ClientEntity, &Team, Option<&crate::components::ClientEntityName>), (With<Npc>, Without<PlayerCharacter>)>,
    asset_server: Res<AssetServer>,
    query_camera: Query<&Transform, With<Camera3d>>,
    images: Res<Assets<Image>>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    query_window: Query<&bevy::window::Window, With<PrimaryWindow>>,
) {
    // Initialize state on first frame
    if ui_state.first_frame {
        *ui_state = UiStateMinimap::new();
    }
    let ui_state = &mut *ui_state;

    // Handle ALT+M shortcut to toggle centered mode
    let alt_pressed = keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight);
    let m_just_pressed = keyboard.just_pressed(KeyCode::KeyM);
    
    if alt_pressed && m_just_pressed {
        ui_state.is_centered = !ui_state.is_centered;
        // Resize window based on centered mode
        if ui_state.is_centered {
            ui_state.window_size = CENTERED_WINDOW_SIZE;
        } else {
            ui_state.window_size = DEFAULT_WINDOW_SIZE;
        }
    }

    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_minimap) {
        dialog
    } else {
        return;
    };

    if current_zone.is_none() {
        return;
    }
    let current_zone = current_zone.unwrap();

    // Return early if the zone loader asset hasn't loaded yet
    let current_zone_data = match zone_loader_assets.get(&current_zone.handle) {
        Some(data) => data,
        None => return,
    };

    let Ok(camera_transform) = query_camera.single() else {
        return;
    };
    let camera_forward_2d = camera_transform.forward().xz().normalize_or_zero();
    let camera_angle = -camera_forward_2d.angle_between(Vec2::Y);

    // If zone has changed, reload the minimap image
    let pixels_per_point = egui_context.ctx_mut().unwrap().pixels_per_point();
    if ui_state.zone_id != Some(current_zone.id)
        || pixels_per_point != ui_state.zone_name_pixels_per_point
    {
        let zone_data = game_data.zone_list.get_zone(current_zone.id);
        if ui_state.zone_id != Some(current_zone.id) {
            ui_state.minimap_image = Default::default();
            ui_state.minimap_texture = Default::default();
            ui_state.minimap_image_size = Default::default();
            ui_state.scroll = Vec2::ZERO; // Reset scroll on zone change
            ui_state.image_scale = 1.0; // Reset scale on zone change
            ui_state.scaled_outline_pixels = ORIGINAL_MAP_OUTLINE_PIXELS;

            if let Some(minimap_path) =
                zone_data.and_then(|zone_data| zone_data.minimap_path.as_ref())
            {
                ui_state.minimap_image = asset_server.load(minimap_path.path().to_string_lossy().into_owned());
                ui_state.minimap_texture =
                    egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(ui_state.minimap_image.clone()));
            }

            ui_state.zone_id = Some(current_zone.id);
            ui_state.last_player_position = Vec2::default();
        }

        let zone_name = zone_data.map_or("???", |zone_data| zone_data.name.as_str());
        let ctx = egui_context.ctx_mut().unwrap();
        ui_state.zone_name_text_galley = Some(generate_text_galley(
            ctx,
            ZONE_NAME_WIDTH,
            16.0,
            zone_name.to_string(),
        ));
        ui_state.zone_name_text_expanded_galley = Some(generate_text_galley(
            ctx,
            ZONE_NAME_EXPANDED_WIDTH,
            16.0,
            zone_name.to_string(),
        ));
        ui_state.zone_name_pixels_per_point = pixels_per_point;
    }

    if ui_state.minimap_image_size.is_none() {
        if let Some(minimap_image) = images.get(&ui_state.minimap_image) {
            let minimap_image_size = Vec2::new(minimap_image.size()[0] as f32, minimap_image.size()[1] as f32);
            ui_state.minimap_image_size = Some(minimap_image_size);

            if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
                let world_block_size =
                    16.0 * current_zone_data.zon.grid_per_patch * current_zone_data.zon.grid_size;
                
                // ============================================================
                // MINIMAP COORDINATE SYSTEM - CORRECTED ANALYSIS
                // ============================================================
                //
                // ORIGINAL image dimensions: 576 x 512 pixels
                // - Outline: 64 pixels (ORIGINAL_MAP_OUTLINE_PIXELS)
                // - Usable: (576-128) x (512-128) = 448 x 384 pixels
                // - Blocks: 448/64 x 384/64 = 7 x 6 blocks
                // - distance_per_pixel = world_block_size / 64
                //
                // UPSCALED image (e.g., 2168 x 1928):
                // - Scale X: 2168 / 576 = 3.76x
                // - Scale Y: 1928 / 512 = 3.76x
                // - Scaled outline: 64 * 3.76 = 241 pixels
                // - distance_per_pixel = world_block_size / (64 * 3.76)
                //
                // ============================================================
                
                // ORIGINAL map dimensions (correct values)
                const ORIGINAL_IMAGE_WIDTH: f32 = 576.0;
                const ORIGINAL_IMAGE_HEIGHT: f32 = 512.0;
                
                // Calculate scale from actual dimensions vs original
                let scale_x = minimap_image_size.x / ORIGINAL_IMAGE_WIDTH;
                let scale_y = minimap_image_size.y / ORIGINAL_IMAGE_HEIGHT;
                ui_state.image_scale = ((scale_x + scale_y) / 2.0).max(1.0);
                
                // Scale the outline pixels
                ui_state.scaled_outline_pixels = ORIGINAL_MAP_OUTLINE_PIXELS * ui_state.image_scale;
                
                // Calculate scaled pixels per block
                let scaled_pixels_per_block = ORIGINAL_MAP_BLOCK_PIXELS * ui_state.image_scale;
                
                // Calculate how many world blocks the image covers
                let minimap_blocks_x = (minimap_image_size.x - 2.0 * ui_state.scaled_outline_pixels) / scaled_pixels_per_block;
                let minimap_blocks_y = (minimap_image_size.y - 2.0 * ui_state.scaled_outline_pixels) / scaled_pixels_per_block;

                // World coverage from zone data
                let min_pos_x = zone_data.minimap_start_x as f32 * world_block_size;
                let min_pos_y = (64.0 - zone_data.minimap_start_y as f32 + 1.0) * world_block_size;

                let max_pos_x = min_pos_x + minimap_blocks_x * world_block_size;
                let max_pos_y = min_pos_y - minimap_blocks_y * world_block_size;

                ui_state.min_world_pos = Vec2::new(min_pos_x, min_pos_y);
                ui_state.max_world_pos = Vec2::new(max_pos_x, max_pos_y);
                
                // distance_per_pixel must be scaled - more pixels = smaller world distance per pixel
                ui_state.distance_per_pixel = world_block_size / scaled_pixels_per_block;
            }
        }
    }

    let (player_position, player_team, player_party) =
        if let Ok((player_position, player_team, player_party)) = query_player.single() {
            (Some(player_position), Some(player_team), player_party)
        } else {
            (None, None, None)
        };
    let player_position_changed = if let Some(player_position) = player_position {
        if ui_state.minimap_image_size.is_some()
            && ui_state.last_player_position != player_position.xy()
        {
            ui_state.last_player_position = player_position.xy();
            true
        } else {
            false
        }
    } else {
        false
    };

    // Use dynamic window size instead of fixed dialog size
    let dialog_width = ui_state.window_size.x;
    let dialog_height = ui_state.window_size.y;

    let mut response_expand_button = None;
    let mut response_shrink_button = None;
    let mut response_big_minimise_button = None;
    let mut response_small_minimise_button = None;
    let minimised = ui_state.minimap_image_size.is_none() || ui_state.is_minimised;

    // Get scaled outline pixels for coordinate calculations
    let scaled_outline = ui_state.scaled_outline_pixels;

    // Map relative position calculation (world to map coordinates)
    // Uses scaled outline to account for upscaled images
    let map_relative_position = |ui_state: &UiStateMinimap, position: Vec3| -> Vec2 {
        let minimap_player_x = scaled_outline
            + f32::max(
                0.0,
                (position.x - ui_state.min_world_pos.x) / ui_state.distance_per_pixel,
            );
        let minimap_player_y = scaled_outline
            + f32::max(
                0.0,
                (ui_state.min_world_pos.y - position.y) / ui_state.distance_per_pixel,
            );
        Vec2::new(minimap_player_x, minimap_player_y)
    };

    // Calculate minimap area size (excluding title bar, toggle bar, and coords bar)
    let minimap_area_height = if minimised {
        0.0
    } else {
        dialog_height - TITLE_BAR_HEIGHT - TOGGLE_BAR_HEIGHT - COORDS_BAR_HEIGHT - 4.0
    };
    let minimap_size = Vec2::new(dialog_width - 2.0, minimap_area_height);

    // Determine window position based on centered mode
    let window_anchor = if ui_state.is_centered {
        egui::Align2::CENTER_CENTER
    } else {
        egui::Align2::RIGHT_TOP
    };

    // Allow larger window size in centered mode
    let max_window_size = if ui_state.is_centered {
        egui::vec2(1200.0, 1200.0)
    } else {
        MAX_WINDOW_SIZE.to_array().into()
    };
    
    egui::Window::new("Minimap")
        .anchor(window_anchor, [0.0, 0.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(!minimised)
        .min_size(if minimised { egui::vec2(150.0, 25.0) } else { MIN_WINDOW_SIZE.to_array().into() })
        .max_size(if minimised { egui::vec2(300.0, 25.0) } else { max_window_size })
        .default_width(dialog_width)
        .default_height(dialog_height)
        .show(egui_context.ctx_mut().unwrap(), |ui| {
            // Update window size from actual ui size
            let actual_size = ui.min_rect().size();
            if !minimised && actual_size.x > 1.0 && actual_size.y > 1.0 {
                ui_state.window_size = Vec2::new(actual_size.x, actual_size.y);
            }

            let image_size = ui_state.minimap_image_size.unwrap_or(minimap_size);
            let minimap_rect = egui::Rect::from_min_size(
                ui.min_rect().min + egui::vec2(1.0, TITLE_BAR_HEIGHT),
                egui::vec2(minimap_size.x, minimap_size.y),
            );

            let minimap_player_pos =
                player_position.map(|p| map_relative_position(ui_state, p.position));

            // Calculate visible area based on zoom level
            let zoom = ui_state.zoom_level;
            let visible_width = minimap_size.x / zoom;
            let visible_height = minimap_size.y / zoom;

            // Map absolute position calculation (accounting for zoom and scroll)
            let map_absolute_position = |ui_state: &UiStateMinimap, position: Vec3| -> Vec2 {
                let map_pos = map_relative_position(ui_state, position);
                // Convert map coordinates to screen coordinates with zoom
                Vec2::new(minimap_rect.min.x, minimap_rect.min.y)
                    + (map_pos - ui_state.scroll) * zoom
            };

            if !minimised {
                let response = ui.allocate_rect(minimap_rect, egui::Sense::click_and_drag());

                // Handle zoom with scroll wheel
                let mut zoom_delta = 0.0;
                ui.input(|input| {
                    for event in &input.raw.events {
                        if let egui::Event::MouseWheel { delta, .. } = event {
                            // Use a consistent zoom factor
                            zoom_delta += delta.y * 0.1;
                        }
                    }
                });

                if zoom_delta != 0.0 && response.hovered() {
                    let old_zoom = ui_state.zoom_level;
                    ui_state.zoom_level = (ui_state.zoom_level * (1.0 + zoom_delta))
                        .clamp(MIN_ZOOM, MAX_ZOOM);

                    // Zoom towards cursor position
                    if let Some(cursor_pos) = response.hover_pos() {
                        let cursor_offset = Vec2::new(
                            cursor_pos.x - minimap_rect.min.x,
                            cursor_pos.y - minimap_rect.min.y,
                        );
                        let map_cursor_pos = ui_state.scroll + cursor_offset / old_zoom;
                        let new_map_cursor_pos = ui_state.scroll + cursor_offset / ui_state.zoom_level;
                        ui_state.scroll += (map_cursor_pos - new_map_cursor_pos);
                    }
                }

                // Handle dragging
                if response.dragged() {
                    let delta = ui.input(|input| input.pointer.delta());
                    ui_state.scroll.x -= delta.x / zoom;
                    ui_state.scroll.y -= delta.y / zoom;
                    ui_state.follow_player = false; // Disable follow when user drags
                } else if ui_state.follow_player && player_position_changed {
                    // Follow player mode: center on player
                    if let Some(target_center) = minimap_player_pos {
                        ui_state.scroll.x = target_center.x - visible_width / 2.0;
                        ui_state.scroll.y = target_center.y - visible_height / 2.0;
                    }
                }

                // Clamp scroll to valid bounds
                let max_scroll_x = (image_size.x - visible_width).max(0.0);
                let max_scroll_y = (image_size.y - visible_height).max(0.0);
                ui_state.scroll.x = ui_state.scroll.x.clamp(0.0, max_scroll_x);
                ui_state.scroll.y = ui_state.scroll.y.clamp(0.0, max_scroll_y);

                // Calculate UV coordinates for the visible portion
                let minimap_uv = egui::Rect::from_min_max(
                    egui::pos2(
                        ui_state.scroll.x / image_size.x,
                        ui_state.scroll.y / image_size.y,
                    ),
                    egui::pos2(
                        (ui_state.scroll.x + visible_width) / image_size.x,
                        (ui_state.scroll.y + visible_height) / image_size.y,
                    ),
                );

                // Draw map texture
                if ui.is_rect_visible(minimap_rect) {
                    let mut mesh = egui::epaint::Mesh::with_texture(ui_state.minimap_texture);
                    mesh.add_rect_with_uv(minimap_rect, minimap_uv, egui::Color32::WHITE);
                    ui.painter().add(egui::epaint::Shape::mesh(mesh));
                }
            }

            // Draw dialog frame and title bar
            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [
                        (IID_BTN_EXPAND, &mut response_expand_button),
                        (IID_BTN_NORMAL, &mut response_shrink_button),
                        (IID_BTN_MINIMIZE_BIG, &mut response_big_minimise_button),
                        (IID_BTN_MINIMIZE_SMALL, &mut response_small_minimise_button),
                    ],
                    visible: &mut [
                        (IID_PANE_SMALL, !ui_state.is_expanded),
                        (IID_PANE_BIG, ui_state.is_expanded),
                        (IID_PANE_BIG_CHILDPANE, !ui_state.is_minimised),
                        (IID_PANE_SMALL_CHILDPANE, !ui_state.is_minimised),
                    ],
                    ..Default::default()
                },
                |ui, _bindings| {
                    let zone_name_width = if ui_state.is_expanded {
                        ZONE_NAME_EXPANDED_WIDTH
                    } else {
                        ZONE_NAME_WIDTH
                    };
                    let zone_name_rect = egui::Rect::from_min_size(
                        egui::pos2(5.0, 2.0),
                        egui::vec2(zone_name_width, 16.0),
                    )
                    .translate(ui.min_rect().min.to_vec2());

                    ui.allocate_ui_at_rect(zone_name_rect, |ui| {
                        let galley = if ui_state.is_expanded {
                            ui_state.zone_name_text_expanded_galley.as_ref()
                        } else {
                            ui_state.zone_name_text_galley.as_ref()
                        };

                        if let Some(galley) = galley {
                            ui.horizontal_top(|ui| ui.label(galley.clone()));
                        }
                    });
                },
            );

            if !minimised {
                let zoom = ui_state.zoom_level;

                // Get icon sprites
                let enemy_character_icon =
                    ui_resources.get_sprite_by_index(UiSpriteSheetType::StateIcon, 73);
                let party_character_icon =
                    ui_resources.get_sprite(UiSpriteSheetType::Ui as i32, "ID_MINIMAP_PARTYMEMBER");
                let other_character_icon =
                    ui_resources.get_sprite(UiSpriteSheetType::Ui as i32, "ID_OTHER_AVATAR");
                let monster_icon =
                    ui_resources.get_sprite_by_index(UiSpriteSheetType::StateIcon, 73); // Use enemy icon for monsters

                // Draw other characters (if enabled)
                if ui_state.show_players {
                    for (character_info, character_position, character_team) in query_characters.iter()
                    {
                        let icon_image = if player_team
                            .map_or(false, |player_team| character_team.id != player_team.id)
                        {
                            enemy_character_icon.as_ref()
                        } else if player_party.map_or(false, |player_party| {
                            player_party
                                .members
                                .iter()
                                .any(|member| member.get_character_id() == character_info.unique_id)
                        }) {
                            party_character_icon.as_ref()
                        } else {
                            other_character_icon.as_ref()
                        };
                        let Some(icon_image) = icon_image else {
                            continue;
                        };

                        let character_minimap_position =
                            map_absolute_position(ui_state, character_position.position);

                        // Scale icon size with zoom (but clamp to reasonable bounds)
                        let icon_scale = zoom.clamp(0.75, 1.5);
                        let icon_size = Vec2::new(icon_image.width, icon_image.height) * icon_scale;
                        let icon_rect = egui::Rect::from_min_size(
                            (character_minimap_position - icon_size / 2.0)
                                .to_array()
                                .into(),
                            icon_size.to_array().into(),
                        );

                        if minimap_rect.contains_rect(icon_rect) {
                            icon_image.draw(ui, icon_rect.min);
                        }
                    }
                }

                // Draw NPC markers (if enabled)
                if ui_state.show_npcs {
                    for &ZoneNpc {
                        npc_id,
                        position: npc_position,
                    } in current_zone_data.npcs.iter()
                    {
                        let Some(npc_data) = game_data.npcs.get_npc(npc_id) else {
                            continue;
                        };
                        let Some(icon_image) = ui_resources.get_sprite_by_index(
                            UiSpriteSheetType::StateIcon,
                            npc_data.npc_minimap_icon_index as usize,
                        ) else {
                            continue;
                        };

                        let npc_minimap_position = map_absolute_position(ui_state, npc_position);
                        let icon_scale = zoom.clamp(0.75, 1.5);
                        let icon_size = Vec2::new(icon_image.width, icon_image.height) * icon_scale;
                        let icon_rect = egui::Rect::from_min_size(
                            (npc_minimap_position - icon_size / 2.0).to_array().into(),
                            icon_size.to_array().into(),
                        );

                        if minimap_rect.contains_rect(icon_rect) {
                            icon_image.draw(ui, icon_rect.min);

                            let response = ui.allocate_rect(
                                egui::Rect::from_min_size(
                                    icon_rect.min + egui::vec2(6.0, 6.0),
                                    egui::vec2(8.0, 8.0),
                                ),
                                egui::Sense::hover(),
                            );
                            response.on_hover_text(npc_data.name.as_str());
                        }
                    }
                }

                // Draw monsters (if enabled)
                if ui_state.show_monsters {
                    if let Some(player_team) = player_team {
                        for (monster_position, client_entity, team, entity_name) in query_monsters.iter() {
                            // Only show hostile monsters
                            if !is_monster_entity(client_entity, team, player_team) {
                                continue;
                            }

                            let Some(icon_image) = monster_icon.as_ref() else {
                                continue;
                            };

                            let monster_minimap_position = map_absolute_position(ui_state, monster_position.position);
                            let icon_scale = zoom.clamp(0.75, 1.5);
                            let icon_size = Vec2::new(icon_image.width, icon_image.height) * icon_scale;
                            let icon_rect = egui::Rect::from_min_size(
                                (monster_minimap_position - icon_size / 2.0).to_array().into(),
                                icon_size.to_array().into(),
                            );

                            if minimap_rect.contains_rect(icon_rect) {
                                // Draw with red tint for hostile monsters
                                let rect = egui::Rect::from_min_size(icon_rect.min, icon_size.to_array().into());
                                let mut mesh = egui::epaint::Mesh::with_texture(icon_image.texture_id);
                                mesh.add_rect_with_uv(rect, icon_image.uv, egui::Color32::from_rgb(255, 100, 100));
                                ui.painter().add(egui::epaint::Shape::mesh(mesh));

                                // Show monster name on hover
                                let response = ui.allocate_rect(
                                    egui::Rect::from_min_size(
                                        icon_rect.min + egui::vec2(4.0, 4.0),
                                        egui::vec2(8.0, 8.0),
                                    ),
                                    egui::Sense::hover(),
                                );
                                if let Some(name) = entity_name {
                                    response.on_hover_text(&name.name);
                                }
                            }
                        }
                    }
                }

                // Draw player position arrow texture on a rotated rectangle to face camera position
                if let Some(minimap_player_pos) = minimap_player_pos {
                    let minimap_player_sprite = ui_resources.get_minimap_player_sprite().unwrap();
                    let player_icon_size =
                        Vec2::new(minimap_player_sprite.width, minimap_player_sprite.height);
                    let minimap_player_pos_screen = Vec2::new(minimap_rect.min.x, minimap_rect.min.y)
                        + (minimap_player_pos - ui_state.scroll) * zoom;
                    let widget_rect = egui::Rect::from_min_size(
                        (minimap_player_pos_screen - player_icon_size / 2.0)
                            .to_array()
                            .into(),
                        player_icon_size.to_array().into(),
                    );

                    if minimap_rect.contains_rect(widget_rect) {
                        ui.allocate_ui_at_rect(widget_rect, |ui| {
                            ui.centered_and_justified(|ui| {
                                let (rect, response) = ui.allocate_exact_size(
                                    player_icon_size.to_array().into(),
                                    egui::Sense::hover(),
                                );

                                // Calculate rotated rectangle from camera angle
                                let sin_a = camera_angle.sin();
                                let cos_a = camera_angle.cos();

                                let mut corners = [
                                    [-player_icon_size.x / 2.0, -player_icon_size.y / 2.0],
                                    [player_icon_size.x / 2.0, -player_icon_size.y / 2.0],
                                    [-player_icon_size.x / 2.0, player_icon_size.y / 2.0],
                                    [player_icon_size.x / 2.0, player_icon_size.y / 2.0],
                                ];

                                for corner in corners.iter_mut() {
                                    let rotated_x = corner[0] * cos_a - corner[1] * sin_a;
                                    let rotated_y = corner[0] * sin_a + corner[1] * cos_a;
                                    *corner = [rotated_x, rotated_y];
                                }

                                if ui.is_rect_visible(rect) {
                                    let mut mesh =
                                        egui::Mesh::with_texture(minimap_player_sprite.texture_id);
                                    let uv = minimap_player_sprite.uv;

                                    let color = egui::Color32::WHITE;
                                    let idx = mesh.vertices.len() as u32;
                                    mesh.add_triangle(idx, idx + 1, idx + 2);
                                    mesh.add_triangle(idx + 2, idx + 1, idx + 3);

                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos_screen + Vec2::from(corners[0]))
                                            .to_array()
                                            .into(),
                                        uv: uv.left_top(),
                                        color,
                                    });
                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos_screen + Vec2::from(corners[1]))
                                            .to_array()
                                            .into(),
                                        uv: uv.right_top(),
                                        color,
                                    });
                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos_screen + Vec2::from(corners[2]))
                                            .to_array()
                                            .into(),
                                        uv: uv.left_bottom(),
                                        color,
                                    });
                                    mesh.vertices.push(egui::epaint::Vertex {
                                        pos: (minimap_player_pos_screen + Vec2::from(corners[3]))
                                            .to_array()
                                            .into(),
                                        uv: uv.right_bottom(),
                                        color,
                                    });

                                    ui.painter().add(egui::Shape::mesh(mesh));
                                }

                                response
                            })
                            .inner
                        });
                    }
                }

                // Draw toggle bar
                let toggle_bar_rect = egui::Rect::from_min_size(
                    ui.min_rect().min + egui::vec2(1.0, dialog_height - TOGGLE_BAR_HEIGHT - COORDS_BAR_HEIGHT - 2.0),
                    egui::vec2(dialog_width - 2.0, TOGGLE_BAR_HEIGHT),
                );

                let mut mesh = egui::epaint::Mesh::default();
                mesh.add_colored_rect(
                    toggle_bar_rect,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                );
                ui.painter().add(egui::epaint::Shape::mesh(mesh));

                ui.allocate_ui_at_rect(toggle_bar_rect.shrink(2.0), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        // Players toggle
                        let players_text = if ui_state.show_players { "✓ Players" } else { "✗ Players" };
                        let players_color = if ui_state.show_players { egui::Color32::GREEN } else { egui::Color32::GRAY };
                        ui.label(egui::RichText::new(players_text).color(players_color).size(10.0));
                        if ui.small("+/-").clicked() {
                            ui_state.show_players = !ui_state.show_players;
                        }

                        // NPCs toggle
                        let npcs_text = if ui_state.show_npcs { "✓ NPCs" } else { "✗ NPCs" };
                        let npcs_color = if ui_state.show_npcs { egui::Color32::GREEN } else { egui::Color32::GRAY };
                        ui.label(egui::RichText::new(npcs_text).color(npcs_color).size(10.0));
                        if ui.small("+/-").clicked() {
                            ui_state.show_npcs = !ui_state.show_npcs;
                        }

                        // Monsters toggle
                        let monsters_text = if ui_state.show_monsters { "✓ Monsters" } else { "✗ Monsters" };
                        let monsters_color = if ui_state.show_monsters { egui::Color32::GREEN } else { egui::Color32::GRAY };
                        ui.label(egui::RichText::new(monsters_text).color(monsters_color).size(10.0));
                        if ui.small("+/-").clicked() {
                            ui_state.show_monsters = !ui_state.show_monsters;
                        }

                        ui.separator();

                        // Zoom indicator
                        ui.label(egui::RichText::new(format!("🔍 {:.1}x", ui_state.zoom_level)).color(egui::Color32::WHITE).size(10.0));

                        // Follow player toggle
                        let follow_text = if ui_state.follow_player { "📍" } else { "⭕" };
                        if ui.small(follow_text).on_hover_text("Toggle follow player").clicked() {
                            ui_state.follow_player = !ui_state.follow_player;
                        }
                        
                        // Centered mode indicator
                        let center_text = if ui_state.is_centered { "⊞" } else { "⊟" };
                        if ui.small(center_text).on_hover_text("Toggle centered mode (ALT+M)").clicked() {
                            ui_state.is_centered = !ui_state.is_centered;
                        }
                    });
                });

                // Draw player x, y coordinates
                let player_xy_rect = egui::Rect::from_min_size(
                    ui.min_rect().min + egui::vec2(1.0, dialog_height - COORDS_BAR_HEIGHT),
                    egui::vec2(dialog_width - 2.0, COORDS_BAR_HEIGHT),
                );

                let mut mesh = egui::epaint::Mesh::default();
                mesh.add_colored_rect(
                    player_xy_rect,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 190),
                );
                ui.painter().add(egui::epaint::Shape::mesh(mesh));

                ui.allocate_ui_at_rect(player_xy_rect.shrink(2.0), |ui| {
                    if let Some(player_position) = player_position {
                        ui.label(format!(
                            "{:0>4}, {:0>4}",
                            (player_position.position.x / 100.0) as i32,
                            (player_position.position.y / 100.0) as i32
                        ));
                    }
                });
            }
        });

    if response_expand_button.map_or(false, |r| r.clicked()) {
        ui_state.is_expanded = true;
        ui_state.window_size = Vec2::new(300.0, 300.0);
    }

    if response_shrink_button.map_or(false, |r| r.clicked()) {
        ui_state.is_expanded = false;
        ui_state.window_size = DEFAULT_WINDOW_SIZE;
    }

    if response_big_minimise_button.map_or(false, |r| r.clicked())
        || response_small_minimise_button.map_or(false, |r| r.clicked())
    {
        ui_state.is_minimised = !ui_state.is_minimised;
    }
}
