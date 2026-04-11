use std::collections::HashMap;

use bevy::{
    asset::LoadState,
    prelude::{
        AssetServer, Assets, Commands, Handle, Image, Query, Res, ResMut, Resource, Vec2, With,
    },
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_egui::{egui, EguiContexts};
use enum_map::{enum_map, Enum, EnumMap};

use rose_file_readers::{IdFile, TsiFile, TsiSprite, VirtualFilesystem};

use crate::{
    exe_resource_loader::ExeResourceCursor,
    ui::widgets::{Dialog, Widget},
    VfsResource,
};

#[derive(Clone)]
pub struct UiSprite {
    pub texture_id: egui::TextureId,
    pub uv: egui::Rect,
    pub width: f32,
    pub height: f32,
}

impl UiSprite {
    pub fn draw(&self, ui: &mut egui::Ui, pos: egui::Pos2) {
        let rect = egui::Rect::from_min_size(pos, egui::vec2(self.width, self.height));
        let mut mesh = egui::epaint::Mesh::with_texture(self.texture_id);
        mesh.add_rect_with_uv(rect, self.uv, egui::Color32::WHITE);
        ui.painter().add(egui::epaint::Shape::mesh(mesh));
    }

    pub fn draw_stretched(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let mut mesh = egui::epaint::Mesh::with_texture(self.texture_id);
        mesh.add_rect_with_uv(rect, self.uv, egui::Color32::WHITE);
        ui.painter().add(egui::epaint::Shape::mesh(mesh));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Enum)]
pub enum UiSpriteSheetType {
    Ui,
    ExUi,
    Item,
    Skill,
    StateIcon,
    ItemSocketGem,
    ItemSocketEmpty,
    MinimapArrow,
    ClanMarkBackground,
    ClanMarkForeground,
    TargetMark,
}

#[derive(Clone)]
pub struct UiTexture {
    pub handle: Handle<Image>,
    pub texture_id: egui::TextureId,
    pub size: Option<Vec2>,
    pub premultiplied_alpha: bool,
}

pub struct UiSpriteSheet {
    pub sprites: Vec<TsiSprite>,
    pub loaded_textures: Vec<UiTexture>,
    pub sprites_by_name: Option<IdFile>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Enum)]
pub enum UiCursorType {
    #[default]
    Default,
    Attack,
    Inventory,
    PickupItem,
    Left,
    Right,
    Npc,
    User,
    Wheel,
    NoUi,
    Repair,
    Appraisal,
}

/// UI cursor configuration
#[derive(Default, Clone)]
pub struct UiCursor {
    pub handle: Handle<ExeResourceCursor>,
    /// Whether the cursor has been loaded
    pub loaded: bool,
}

impl UiCursor {
    pub fn new(handle: Handle<ExeResourceCursor>) -> Self {
        Self {
            handle,
            loaded: false,
        }
    }
}

#[derive(Resource)]
pub struct UiResources {
    pub loaded_all_textures: bool,
    pub loaded_required_textures: bool,
    pub sprite_sheets: EnumMap<UiSpriteSheetType, Option<UiSpriteSheet>>,

    pub dialog_files: HashMap<String, Handle<Dialog>>,
    pub dialog_login: Handle<Dialog>,
    pub dialog_bank: Handle<Dialog>,
    pub dialog_character_info: Handle<Dialog>,
    pub dialog_chatbox: Handle<Dialog>,
    pub dialog_clan: Handle<Dialog>,
    pub dialog_create_avatar: Handle<Dialog>,
    pub dialog_create_clan: Handle<Dialog>,
    pub dialog_game_menu: Handle<Dialog>,
    pub dialog_inventory: Handle<Dialog>,
    pub dialog_message_box: Handle<Dialog>,
    pub dialog_number_input: Handle<Dialog>,
    pub dialog_minimap: Handle<Dialog>,
    pub dialog_npc_store: Handle<Dialog>,
    pub dialog_npc_transaction: Handle<Dialog>,
    pub dialog_party: Handle<Dialog>,
    pub dialog_party_option: Handle<Dialog>,
    pub dialog_personal_store: Handle<Dialog>,
    pub dialog_player_info: Handle<Dialog>,
    pub dialog_quest_list: Handle<Dialog>,
    pub dialog_respawn: Handle<Dialog>,
    pub dialog_select_server: Handle<Dialog>,
   pub dialog_skill_list: Handle<Dialog>,
    pub dialog_skill_tree: Handle<Dialog>,
 
    pub cursors: EnumMap<UiCursorType, UiCursor>,
 
    pub skill_tree_dealer: Handle<Dialog>,
    pub skill_tree_hawker: Handle<Dialog>,
    pub skill_tree_muse: Handle<Dialog>,
    pub skill_tree_soldier: Handle<Dialog>,
}

#[derive(Default, Resource)]
pub struct UiRequestedCursor {
    pub moving_camera: bool,
    pub world_cursor: UiCursorType,
}

impl UiResources {
    pub fn get_sprite(&self, module_id: i32, sprite_name: &str) -> Option<UiSprite> {
        let sprite_sheet_type = match module_id {
            0 => UiSpriteSheetType::Ui,
            1 => UiSpriteSheetType::Item,
            3 => UiSpriteSheetType::ExUi,
            4 => UiSpriteSheetType::Skill,
            5 => UiSpriteSheetType::StateIcon,
            6 => UiSpriteSheetType::ItemSocketGem,
            7 => UiSpriteSheetType::ClanMarkBackground,
            8 => UiSpriteSheetType::ClanMarkForeground,
            9 => UiSpriteSheetType::TargetMark,
            _ => {
                return None;
            }
        };
        
        let sprite_sheet = match self.sprite_sheets[sprite_sheet_type].as_ref() {
            Some(sheet) => sheet,
            None => {
                return None;
            }
        };
        
        let sprites_by_name = match sprite_sheet.sprites_by_name.as_ref() {
            Some(map) => map,
            None => {
                return None;
            }
        };
        
        let sprite_index = match sprites_by_name.get(sprite_name) {
            Some(idx) => idx,
            None => {
                return None;
            }
        };

        self.get_sprite_by_index(sprite_sheet_type, *sprite_index as usize)
    }

    pub fn get_sprite_by_index(
        &self,
        sprite_sheet_type: UiSpriteSheetType,
        sprite_index: usize,
    ) -> Option<UiSprite> {
        let sprite_sheet = match self.sprite_sheets[sprite_sheet_type].as_ref() {
            Some(sheet) => sheet,
            None => {
                return None;
            }
        };
        
        let sprite = match sprite_sheet.sprites.get(sprite_index) {
            Some(s) => s,
            None => {
                return None;
            }
        };
        
        let texture = match sprite_sheet.loaded_textures.get(sprite.texture_id as usize) {
            Some(t) => t,
            None => {
                return None;
            }
        };
        
        let texture_size = match texture.size {
            Some(size) if size.x > 0.0 && size.y > 0.0 => size,
            Some(_) | None => {
                return None;
            }
        };

        Some(UiSprite {
            texture_id: texture.texture_id,
            uv: egui::Rect::from_min_max(
                egui::pos2(
                    (sprite.left as f32 + 0.5) / texture_size.x,
                    (sprite.top as f32 + 0.5) / texture_size.y,
                ),
                egui::pos2(
                    (sprite.right as f32 + 0.5) / texture_size.x,
                    (sprite.bottom as f32 + 0.5) / texture_size.y,
                ),
            ),
            width: ((sprite.right + 1) - sprite.left) as f32,
            height: ((sprite.bottom + 1) - sprite.top) as f32,
        })
    }

    pub fn get_sprite_image(&self, module_id: i32, sprite_name: &str) -> Option<&Handle<Image>> {
        let sprite_sheet_type = match module_id {
            0 => UiSpriteSheetType::Ui,
            1 => UiSpriteSheetType::Item,
            3 => UiSpriteSheetType::ExUi,
            4 => UiSpriteSheetType::Skill,
            5 => UiSpriteSheetType::StateIcon,
            6 => UiSpriteSheetType::ItemSocketGem,
            9 => UiSpriteSheetType::TargetMark,
            _ => return None,
        };
        let sprite_sheet = self.sprite_sheets[sprite_sheet_type].as_ref()?;
        let sprite_index = sprite_sheet
            .sprites_by_name
            .as_ref()
            .unwrap()
            .get(sprite_name)?;

        self.get_sprite_image_by_index(sprite_sheet_type, *sprite_index as usize)
    }

    pub fn get_sprite_image_by_index(
        &self,
        sprite_sheet_type: UiSpriteSheetType,
        sprite_index: usize,
    ) -> Option<&Handle<Image>> {
        let sprite_sheet = self.sprite_sheets[sprite_sheet_type].as_ref()?;
        let sprite = sprite_sheet.sprites.get(sprite_index)?;
        let texture = sprite_sheet
            .loaded_textures
            .get(sprite.texture_id as usize)?;
        Some(&texture.handle)
    }

    pub fn get_item_socket_sprite(&self) -> Option<UiSprite> {
        let texture = &self.sprite_sheets[UiSpriteSheetType::ItemSocketEmpty]
            .as_ref()?
            .loaded_textures[0];
        let texture_size = texture.size?;

        Some(UiSprite {
            texture_id: self.sprite_sheets[UiSpriteSheetType::ItemSocketEmpty]
                .as_ref()?
                .loaded_textures[0]
                .texture_id,
            uv: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            width: texture_size.x,
            height: texture_size.y,
        })
    }

    pub fn get_minimap_player_sprite(&self) -> Option<UiSprite> {
        let texture = &self.sprite_sheets[UiSpriteSheetType::MinimapArrow]
            .as_ref()?
            .loaded_textures[0];
        let texture_size = texture.size?;

        Some(UiSprite {
            texture_id: self.sprite_sheets[UiSpriteSheetType::MinimapArrow]
                .as_ref()?
                .loaded_textures[0]
                .texture_id,
            uv: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            width: texture_size.x,
            height: texture_size.y,
        })
    }
}

fn load_ui_spritesheet(
    vfs: &VirtualFilesystem,
    asset_server: &AssetServer,
    egui_context: &mut EguiContexts,
    tsi_path: &str,
    id_path: &str,
) -> Result<UiSpriteSheet, anyhow::Error> {
    let tsi_file = vfs.read_file::<TsiFile, _>(tsi_path)?;
    let id_file = if id_path.is_empty() {
        None
    } else {
        Some(vfs.read_file::<IdFile, _>(id_path)?)
    };

    let mut loaded_textures = Vec::new();
    for (tsi_texture_index, tsi_texture) in tsi_file.textures.iter().enumerate() {
        let texture_path = format!("3ddata/control/res/{}", tsi_texture.filename).to_lowercase();
        let handle = asset_server.load(&texture_path);
        let texture_id = egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
        loaded_textures.push(UiTexture {
            handle,
            texture_id,
            size: None,
            premultiplied_alpha: false,
        });
    }

    Ok(UiSpriteSheet {
        sprites: tsi_file.sprites,
        loaded_textures,
        sprites_by_name: id_file,
    })
}

fn premultiply_image_alpha(image: &mut Image) {
    let width = image.width();
    let height = image.height();

    for x in 0..width {
        for y in 0..height {
            if let Ok(mut color) = image.get_color_at(x, y).map(|c| c.to_linear()) {
                color.red *= color.alpha;
                color.green *= color.alpha;
                color.blue *= color.alpha;

                let _ = image.set_color_at(x, y, bevy::prelude::Color::LinearRgba(color));
            }
        }
    }
}

pub fn update_ui_resources(
    mut ui_resources: ResMut<UiResources>,
    mut images: ResMut<Assets<Image>>,
    cursors: Res<Assets<ExeResourceCursor>>,
    asset_server: Res<AssetServer>,
    mut dialog_assets: ResMut<Assets<Dialog>>,
    mut egui_context: EguiContexts,
) {
    if ui_resources.loaded_all_textures && ui_resources.loaded_required_textures {
        return;
    }

    let mut loaded_all = true;
    let mut loaded_required = true;

    for spritesheet in ui_resources
        .sprite_sheets
        .iter_mut()
        .filter_map(|(_, spritesheet)| spritesheet.as_mut())
    {
        for (texture_index, texture) in spritesheet.loaded_textures.iter_mut().enumerate() {
            let has_valid_size = texture.size.map_or(false, |size| size.x > 0.0 && size.y > 0.0);
            let needs_premultiply = !texture.premultiplied_alpha;
            if has_valid_size && !needs_premultiply {
                continue;
            }

            if let Some(image) = images.get_mut(&texture.handle) {
                let size = image.size().as_vec2();
                if size.x > 0.0 && size.y > 0.0 {
                    texture.size = Some(size);
                    if needs_premultiply {
                        premultiply_image_alpha(image);
                        texture.premultiplied_alpha = true;
                    }
                } else {
                    log::warn!("[UI RESOURCES] Texture has zero size: texture_index={}, size={:?}", texture_index, size);
                    loaded_all = false;
                    loaded_required = false;
                }
            } else {
                let load_state = asset_server.get_load_state(&texture.handle);
                let handle_path = format!("{:?}", texture.handle);
                log::warn!("[UI RESOURCES] Texture NOT in images resource: texture_index={}, load_state={:?}, handle={}", texture_index, load_state, handle_path);
                if matches!(load_state, Some(LoadState::Failed(_))) {
                    texture.size = Some(Vec2::ZERO);
                } else {
                    texture.size = Some(Vec2::ZERO);
                    loaded_all = false;
                    loaded_required = false;
                }
            }
        }
    }

    // Mark cursors as loaded (custom cursors will be implemented separately)
    for (_, ui_cursor) in ui_resources.cursors.iter_mut() {
        if ui_cursor.loaded {
            continue;
        }
        
        let load_state = asset_server.get_load_state(&ui_cursor.handle);
        if cursors.get(&ui_cursor.handle).is_some() {
            ui_cursor.loaded = true;
        } else {
            // Cursor not loaded yet or failed - mark as loaded to continue
            ui_cursor.loaded = true;
            if matches!(load_state, Some(LoadState::Failed(_))) {
                log::warn!("[UI RESOURCES] Cursor failed to load: {:?}", ui_cursor.handle);
            }
        }
    }

    let mut load_skill_tree = |skill_tree: &Handle<Dialog>| {
        let load_state = asset_server.get_load_state(skill_tree);
        if let Some(skill_tree) = dialog_assets.get_mut(skill_tree) {
            for widget in skill_tree.widgets.iter_mut() {
                if let Widget::Skill(skill_widget) = widget {
                    if let Some(texture) = skill_widget.ui_texture.as_mut() {
                        let texture_load_state = asset_server.get_load_state(&texture.handle);
                        if let Some(image) = images.get_mut(&texture.handle) {
                            texture.size = Some(image.size().as_vec2());
                            if !texture.premultiplied_alpha {
                                premultiply_image_alpha(image);
                                texture.premultiplied_alpha = true;
                            }
                        } else if matches!(texture_load_state, Some(LoadState::Failed(_))) {
                            texture.size = Some(Vec2::ZERO);
                            log::warn!("[UI RESOURCES] Skill tree texture failed to load: {:?}", texture.handle);
                        } else if matches!(texture_load_state, Some(LoadState::Loading) | Some(LoadState::NotLoaded)) {
                            loaded_all = false;
                        } else {
                            loaded_all = false;
                            log::warn!("[UI RESOURCES] Skill tree texture load state unknown (None): {:?}", texture.handle);
                        }
                    } else {
                        let handle = asset_server
                            .load(format!("3ddata/control/res/{}", &skill_widget.image).to_lowercase());
                        let texture_id = egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
                        skill_widget.ui_texture = Some(UiTexture {
                            handle: handle.clone(),
                            texture_id,
                            size: None,
                            premultiplied_alpha: false,
                        });
                        loaded_all = false;
                    }
                }
            }
        } else if matches!(load_state, Some(LoadState::Failed(_))) {
            log::warn!("[UI RESOURCES] Skill tree dialog failed to load: {:?}", skill_tree);
        } else if matches!(load_state, Some(LoadState::Loading) | Some(LoadState::NotLoaded)) {
            loaded_all = false;
        } else {
            loaded_all = false;
            log::warn!("[UI RESOURCES] Skill tree dialog load state unknown (None): {:?}", skill_tree);
        }
    };

    load_skill_tree(&ui_resources.skill_tree_soldier);
    load_skill_tree(&ui_resources.skill_tree_muse);
    load_skill_tree(&ui_resources.skill_tree_hawker);
    load_skill_tree(&ui_resources.skill_tree_dealer);

    ui_resources.loaded_all_textures = loaded_all;
    ui_resources.loaded_required_textures = loaded_required;
}

pub fn load_ui_resources(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    mut egui_context: EguiContexts,
) {
    let vfs = &vfs_resource.vfs;

    let dialog_filenames = [
        "DELIVERYSTORE.XML", "DLGADDFRIEND.XML", "DLGAVATA.XML", "DLGAVATARSTORE.XML", "DLGBANK.XML",
        "DLGCHAT.XML", "DLGCHATFILTER.XML", "DLGCHATROOM.XML", "DLGCLAN.XML", "DLGCLANREGNOTICE.XML",
        "DLGCOMM.XML", "DLGCREATEAVATAR.XML", "DLGDEAL.XML", "DLGDIALOG.XML", "DLGDIALOGEVENT.XML",
        "DLGEXCHANGE.XML", "DLGGOODS.XML", "DLGHELP.XML", "DLGINFO.XML", "DLGINPUTNAME.XML",
        "DLGITEM.XML", "DLGLOGIN.XML", "DLGMAKE.XML", "DLGMEMO.XML", "DLGMEMOVIEW.XML", "DLGMENU.XML",
        "DLGMINIMAP.XML", "DLGNINPUT.XML", "DLGNOTIFY.XML", "DLGOPTION.XML", "DLGORGANIZECLAN.XML",
        "DLGPARTY.XML", "DLGPARTYOPTION.XML", "DLGPRIVATECHAT.XML", "DLGPRIVATESTORE.XML",
        "DLGQUEST.XML", "DLGQUICKBAR.XML", "DLGRESTART.XML", "DLGSELAVATAR.XML", "DLGSELECTEVENT.XML",
        "DLGSELONLYSVR.XML", "DLGSELSVR.XML", "DLGSEPARATE.XML", "DLGSKILL.XML", "DLGSKILLTREE.XML",
        "DLGSTORE.XML", "DLGSYSTEM.XML", "DLGSYSTEMMSG.XML", "DLGUPGRADE.XML", "MSGBOX.XML",
        "SKILLTREE_DEALER.XML", "SKILLTREE_HAWKER.XML", "SKILLTREE_MUSE.XML", "SKILLTREE_SOLDIER.XML",
    ];

    let mut dialog_files = HashMap::new();
    for filename in dialog_filenames {
        dialog_files.insert(
            filename.to_string(),
            asset_server.load(format!("3ddata/control/xml/{}", filename).to_lowercase()),
        );
    }

    let mut style = (*egui_context.ctx_mut().unwrap().style()).clone();
    style.visuals.window_fill = egui::Color32::from_rgba_unmultiplied(10, 10, 10, 220);
    style.visuals.window_stroke = egui::Stroke::NONE;
    style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
    egui_context.ctx_mut().unwrap().set_style(style);

    commands.init_resource::<UiRequestedCursor>();
    commands.insert_resource(UiResources {
        loaded_all_textures: false,
        loaded_required_textures: false,
        sprite_sheets: enum_map! {
            UiSpriteSheetType::Ui => load_ui_spritesheet(vfs, &asset_server, &mut egui_context, "3ddata/control/res/ui.tsi", "3ddata/control/xml/ui_strid.id").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::ExUi => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/exui.tsi", "3ddata/control/xml/exui_strid.id").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::StateIcon => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/stateicon.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::Skill => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/skillicon.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::Item => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/item1.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::ItemSocketGem => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/soketjam.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::TargetMark => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/targetmark.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::ClanMarkForeground => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/clancenter.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::ClanMarkBackground => load_ui_spritesheet(vfs, &asset_server, &mut egui_context,  "3ddata/control/res/clanback.tsi", "").map_err(|e| { log::warn!("Error loading ui resource: {}", e); e }).ok(),
            UiSpriteSheetType::MinimapArrow => {
                let handle = asset_server.load("3ddata/control/res/minimap_arrow.tga");
                let texture_id = egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
                Some(UiSpriteSheet {
                    sprites: vec![
                        TsiSprite { texture_id: 0, left: 0, top: 0, right: 0, bottom: 0, name: String::default() },
                    ],
                    loaded_textures: vec![
                        UiTexture { handle, texture_id, size: None, premultiplied_alpha: false },
                    ],
                    sprites_by_name: None,
                })
            }
            UiSpriteSheetType::ItemSocketEmpty => {
                let handle = asset_server.load("3ddata/control/res/soket.dds");
                let texture_id = egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()));
                Some(UiSpriteSheet {
                    sprites: vec![
                        TsiSprite { texture_id: 0, left: 0, top: 0, right: 0, bottom: 0, name: String::default() },
                    ],
                    loaded_textures: vec![
                        UiTexture { handle, texture_id, size: None, premultiplied_alpha: false },
                    ],
                    sprites_by_name: None,
                })
            }
        },
        dialog_bank: dialog_files["DLGBANK.XML"].clone(),
        dialog_character_info: dialog_files["DLGAVATA.XML"].clone(),
        dialog_chatbox: dialog_files["DLGCHAT.XML"].clone(),
        dialog_clan: dialog_files["DLGCLAN.XML"].clone(),
        dialog_create_avatar: dialog_files["DLGCREATEAVATAR.XML"].clone(),
        dialog_create_clan: dialog_files["DLGORGANIZECLAN.XML"].clone(),
        dialog_game_menu: dialog_files["DLGMENU.XML"].clone(),
        dialog_inventory: dialog_files["DLGITEM.XML"].clone(),
        dialog_login: dialog_files["DLGLOGIN.XML"].clone(),
        dialog_message_box: dialog_files["MSGBOX.XML"].clone(),
        dialog_number_input: dialog_files["DLGNINPUT.XML"].clone(),
        dialog_minimap: dialog_files["DLGMINIMAP.XML"].clone(),
        dialog_npc_store:  dialog_files["DLGSTORE.XML"].clone(),
        dialog_npc_transaction:  dialog_files["DLGDEAL.XML"].clone(),
        dialog_party: dialog_files["DLGPARTY.XML"].clone(),
        dialog_party_option: dialog_files["DLGPARTYOPTION.XML"].clone(),
        dialog_personal_store: dialog_files["DLGAVATARSTORE.XML"].clone(),
        dialog_player_info: dialog_files["DLGINFO.XML"].clone(),
        dialog_quest_list: dialog_files["DLGQUEST.XML"].clone(),
        dialog_respawn: dialog_files["DLGRESTART.XML"].clone(),
        dialog_select_server: dialog_files["DLGSELSVR.XML"].clone(),
        dialog_skill_list: dialog_files["DLGSKILL.XML"].clone(),
        dialog_skill_tree: dialog_files["DLGSKILLTREE.XML"].clone(),
        skill_tree_dealer: dialog_files["SKILLTREE_DEALER.XML"].clone(),
        skill_tree_hawker: dialog_files["SKILLTREE_HAWKER.XML"].clone(),
        skill_tree_muse: dialog_files["SKILLTREE_MUSE.XML"].clone(),
        skill_tree_soldier: dialog_files["SKILLTREE_SOLDIER.XML"].clone(),
        dialog_files,

        cursors: enum_map! {
            UiCursorType::Default =>  UiCursor::new(asset_server.load("trose.exe#cursor_196")),
            UiCursorType::Attack =>  UiCursor::new(asset_server.load("trose.exe#cursor_190")),
            UiCursorType::Inventory =>  UiCursor::new(asset_server.load("trose.exe#cursor_195")),
            UiCursorType::PickupItem =>  UiCursor::new(asset_server.load("trose.exe#cursor_194")),
            UiCursorType::Left =>  UiCursor::new(asset_server.load("trose.exe#cursor_193")),
            UiCursorType::Right =>  UiCursor::new(asset_server.load("trose.exe#cursor_191")),
            UiCursorType::Npc =>  UiCursor::new(asset_server.load("trose.exe#cursor_192")),
            UiCursorType::User =>  UiCursor::new(asset_server.load("trose.exe#cursor_199")),
            UiCursorType::Wheel =>  UiCursor::new(asset_server.load("trose.exe#cursor_197")),
            UiCursorType::NoUi =>  UiCursor::new(asset_server.load("trose.exe#cursor_201")),
            UiCursorType::Repair =>  UiCursor::new(asset_server.load("trose.exe#cursor_203")),
            UiCursorType::Appraisal =>  UiCursor::new(asset_server.load("trose.exe#cursor_206")),
        },
    });
}

/// System to apply the requested cursor to the primary window.
/// 
/// In Bevy 0.16+, custom cursors are set via the CursorIcon component on the window entity.
/// The custom_cursor feature must be enabled for this to work.
/// 
/// Note: This system currently uses egui's cursor management. To implement custom cursors
/// using Bevy's CustomCursorImage API:
/// 1. Enable the "custom_cursor" feature in Cargo.toml
/// 2. Load cursor images as Bevy Image assets
/// 3. Create CustomCursorImage structs with the image handles
/// 4. Insert CursorIcon::Custom(CustomCursor::Image(...)) on window entities
pub fn ui_requested_cursor_apply_system(
    _query_window: Query<&mut Window, With<PrimaryWindow>>,
    _ui_requested_cursor: Res<UiRequestedCursor>,
    _ui_resources: Res<UiResources>,
    _egui_ctx: EguiContexts,
) {
    // egui manages cursor input automatically when it wants pointer input
    // Custom cursor implementation using Bevy 0.16's CustomCursorImage API would go here
    // when cursor images are properly loaded
}
