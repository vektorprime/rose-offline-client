use std::{num::NonZeroU16, sync::Arc};

use arrayvec::ArrayVec;
use bevy::{
    asset::RenderAssetUsages,
    ecs::query::QueryData,
    image::ImageSampler,
    log::{info, warn},
    platform::collections::HashMap,
    prelude::{
        Assets, Changed, ChildOf, Color, Commands, Entity, MessageReader,
        GlobalTransform, Handle, Image, Local, Query, Res, ResMut, Transform, Vec2, Vec3,
        Visibility, With, Without,
    },
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_camera::{Camera, visibility::{NoFrustumCulling, VisibilityClass}};
use bevy_egui::{egui, EguiContexts, PrimaryEguiContext};

use rose_game_common::components::{Level, Npc, Team};

use crate::{
    components::{
        ClientEntityName, ModelHeight, NameTag, NameTagEntity, NameTagHealthbarBackground,
        NameTagHealthbarForeground, NameTagName, NameTagTargetMark, NameTagType, PlayerCharacter,
    },
    events::LoadZoneEvent,
    render::WorldUiRect,
    resources::{GameData, NameTagSettings, UiResources, UiSpriteSheetType},
};

const ORDER_HEALTH_BACKGROUND: u8 = 0;
const ORDER_HEALTH_FOREGROUND: u8 = 1;
const ORDER_NAME: u8 = 2;
const ORDER_TARGET_MARK: u8 = 2;
const MAX_NAME_ROWS: usize = 2;

pub struct NameTagData {
    pub image: Handle<Image>,
    pub size: Vec2,
    pub rects: ArrayVec<WorldUiRect, MAX_NAME_ROWS>,
}

#[derive(Clone)]
pub struct NameTagPendingData {
    pub galley: Arc<egui::Galley>,
    pub colors: ArrayVec<Color, MAX_NAME_ROWS>,
    pub name_tag_type: NameTagType,
}

#[derive(Default)]
pub struct NameTagCache {
    pub cache: HashMap<String, NameTagData>,
    pub pending: HashMap<Entity, NameTagPendingData>,
    pub pixels_per_point: f32,
}

#[derive(QueryData)]
pub struct PlayerQuery<'w> {
    level: &'w Level,
    team: &'w Team,
}

#[derive(QueryData)]
pub struct NameTagObjectQuery<'w> {
    entity: Entity,
    name: &'w ClientEntityName,
    model_height: &'w ModelHeight,
    npc: Option<&'w Npc>,
    level: Option<&'w Level>,
    team: Option<&'w Team>,
}

pub fn get_monster_name_tag_color(
    player_level: Option<&Level>,
    monster_level: Option<&Level>,
    monster_team: Option<&Team>,
) -> egui::Color32 {
    let level_diff = player_level.map_or(1, |level| level.level) as i32
        - monster_level.map_or(1, |level| level.level) as i32;

    if monster_team.map_or(false, |team| team.id == Team::DEFAULT_NPC_TEAM_ID) {
        egui::Color32::GREEN
    } else if level_diff <= -23 {
        egui::Color32::from_rgb(224, 149, 255)
    } else if level_diff <= -16 {
        egui::Color32::from_rgb(255, 136, 200)
    } else if level_diff <= -10 {
        egui::Color32::from_rgb(255, 113, 107)
    } else if level_diff <= -4 {
        egui::Color32::from_rgb(255, 166, 107)
    } else if level_diff <= 3 {
        egui::Color32::from_rgb(255, 228, 122)
    } else if level_diff <= 8 {
        egui::Color32::from_rgb(150, 255, 122)
    } else if level_diff <= 14 {
        egui::Color32::from_rgb(137, 243, 255)
    } else if level_diff <= 21 {
        egui::Color32::from_rgb(202, 243, 255)
    } else {
        egui::Color32::from_rgb(217, 217, 217)
    }
}

fn create_pending_nametag(
    name_tag_settings: &NameTagSettings,
    egui_context: &mut EguiContexts,
    object: &NameTagObjectQueryItem,
    player: Option<&PlayerQueryItem>,
    name_tag_type: NameTagType,
) -> NameTagPendingData {
    let layout_job = match name_tag_type {
        NameTagType::Character => egui::epaint::text::LayoutJob::single_section(
            object.name.name.clone(),
            egui::TextFormat::simple(
                egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                if object.team.map_or(false, |team| {
                    Some(team.id) != player.map(|player| player.team.id)
                }) {
                    egui::Color32::RED
                } else {
                    egui::Color32::WHITE
                },
            ),
        ),
        NameTagType::Monster => egui::epaint::text::LayoutJob::single_section(
            object.name.name.clone(),
            egui::TextFormat::simple(
                egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                get_monster_name_tag_color(
                    player.map(|player| player.level),
                    object.level,
                    object.team,
                ),
            ),
        ),
        NameTagType::Npc => {
            if let Some((job, name)) = object.name.name.split_once(']') {
                let mut job = job.trim().to_string();
                job.push(']');
                job.push('\n');
                let name = name.trim();

                let mut layout_job = egui::epaint::text::LayoutJob::single_section(
                    job,
                    egui::TextFormat::simple(
                        egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                        egui::Color32::from_rgb(255, 206, 174),
                    ),
                );
                layout_job.append(
                    name,
                    0.0,
                    egui::TextFormat::simple(
                        egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                        egui::Color32::from_rgb(231, 255, 174),
                    ),
                );
                layout_job
            } else {
                egui::epaint::text::LayoutJob::single_section(
                    object.name.name.clone(),
                    egui::TextFormat::simple(
                        egui::FontId::proportional(name_tag_settings.font_size[name_tag_type]),
                        egui::Color32::GREEN,
                    ),
                )
            }
        }
    };

    let colors: ArrayVec<Color, MAX_NAME_ROWS> = layout_job
        .sections
        .iter()
        .map(|x| x.format.color)
        .map(|x| {
            let [r, g, b, _] = x.to_array().map(|c| c as f32 / 255.0);
            Color::srgb(r, g, b)
        })
        .collect();
    let galley = egui_context
        .ctx_mut()
        .unwrap()
        .fonts_mut(|fonts| fonts.layout_job(layout_job));

    NameTagPendingData {
        galley,
        colors,
        name_tag_type,
    }
}

fn create_nametag_data(
    _camera_entity: Entity,
    egui_context: &mut EguiContexts,
    _egui_managed_textures: &bevy_egui::EguiManagedTextures,
    images: &mut Assets<Image>,
    pending_data: NameTagPendingData,
    debug_entity: Entity,
) -> Option<NameTagData> {
    let pixels_per_point = egui_context.ctx_mut().unwrap().pixels_per_point();

    // Calculate the size of name tag text
    let mut max_bounds = Vec2::new(0.0, 0.0);
    let mut row_bounds = Vec::new();
    // Read the CPU-side egui font atlas directly.
    // This is immediately available after layout and does not depend on render-pass texture uploads.
    let font_source_texture = egui_context
        .ctx_mut()
        .unwrap()
        .fonts_mut(|fonts| fonts.image());

    let mut atlas_nonzero = 0usize;
    let mut atlas_max = 0u8;
    for px in font_source_texture.pixels.iter() {
        let [r, g, b, a] = px.to_array();
        let v = r.max(g).max(b).max(a);
        if v > 0 {
            atlas_nonzero += 1;
            if v > atlas_max {
                atlas_max = v;
            }
        }
    }
    info!(
        "[NAME_TAG_DIAG] Entity {:?} cpu_atlas={}x{} atlas_nonzero={} atlas_max={}",
        debug_entity,
        font_source_texture.width(),
        font_source_texture.height(),
        atlas_nonzero,
        atlas_max
    );

    for (row_index, row) in pending_data.galley.rows.iter().enumerate() {
        let mut row_min = Vec2::new(10000.0, 10000.0);
        let mut row_max = Vec2::new(0.0, 0.0);

        for glyph in row.glyphs.iter() {
            let glyph_size = Vec2::new(
                glyph.uv_rect.max[0] as f32 - glyph.uv_rect.min[0] as f32,
                glyph.uv_rect.max[1] as f32 - glyph.uv_rect.min[1] as f32,
            );
            let glyph_min = Vec2::new(
                (glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point,
                (glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point,
            );
            let glyph_max = glyph_min + glyph_size;

            row_min = row_min.min(glyph_min);
            row_max = row_max.max(glyph_max);
        }

        let row_start_y = row_index as f32 * 8.0;
        row_min.y += row_start_y;
        row_max.y += row_start_y + 8.0;
        row_max.x += 8.0;

        max_bounds = max_bounds.max(row_max);
        row_bounds.push((row_min, row_max));

    }

    // info!("[NAME_TAG_DEBUG] All font textures found, max_bounds: {:?}", max_bounds);

    // Allocate texture
    let target_texture_width = (max_bounds.x as u32).next_power_of_two();
    let target_texture_height = (max_bounds.y as u32).next_power_of_two();
    let data_len = (target_texture_width * target_texture_height * 4) as usize;
    let mut data = vec![0; data_len];
    
    // info!("[NAME_TAG_DEBUG] Allocated texture: {}x{}", target_texture_width, target_texture_height);

    // Copy letters to texture
    let mut total_glyphs_copied = 0;
    for (row_index, row) in pending_data.galley.rows.iter().enumerate() {
        let row_font_texture = &font_source_texture;

        unsafe {
            let src = row_font_texture.pixels.as_ptr();
            let src_stride = row_font_texture.width();
            let dst = data.as_mut_ptr();
            let dst_stride = target_texture_width as usize;

            for glyph in row.glyphs.iter() {
                let uv_min = glyph.uv_rect.min;
                let uv_max = glyph.uv_rect.max;

                let mut dst_y = ((glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point).round()
                    as usize
                    + 4
                    + row_index * 8;

                let dst_x = ((glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point).round()
                    as usize
                    + 4;

                for uv_y in uv_min[1]..uv_max[1] {
                    let mut src_row = src.add(uv_y as usize * src_stride + uv_min[0] as usize);
                    let mut dst_row = dst.add(dst_y * dst_stride * 4 + dst_x * 4);

                    for _ in uv_min[0]..uv_max[0] {
                        let pixel = (*src_row).to_array();
                        // Keep RGB white so tint color is controlled only by vertex color,
                        // avoiding mixed black/white glyph colors from atlas RGB.
                        // Prefer atlas alpha when present for crisp edges;
                        // otherwise fall back to average RGB coverage.
                        let coverage = if pixel[3] > 0 {
                            pixel[3]
                        } else {
                            ((pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3) as u8
                        };

                        *dst_row.add(0) = 255;
                        *dst_row.add(1) = 255;
                        *dst_row.add(2) = 255;
                        *dst_row.add(3) = coverage;

                        src_row = src_row.add(1);
                        dst_row = dst_row.add(4);
                    }
                    dst_y += 1;
                }
                total_glyphs_copied += 1;
            }
        }
    }
    
    // info!("[NAME_TAG_DEBUG] Copied {} glyphs to texture", total_glyphs_copied);

    // Apply outline to text
    let mut outlined_data = data.clone();
    unsafe {
        let src = data.as_ptr();
        let dst = outlined_data.as_mut_ptr();
        let stride = target_texture_width as usize;

        for y in 2..max_bounds.y as usize - 2 {
            for x in 2..max_bounds.x as usize - 2 {
                let px_alpha = |x: usize, y: usize| {
                    let pixel_offset = x * 4 + y * 4 * stride;
                    *src.add(pixel_offset + 3) as u32
                };

                let mut alpha = 0u32;
                alpha += px_alpha(x, y - 2) / 2;
                alpha += px_alpha(x, y - 1);
                alpha += px_alpha(x, y + 1);
                alpha += px_alpha(x, y + 2) / 2;

                alpha += px_alpha(x - 2, y) / 2;
                alpha += px_alpha(x - 1, y);
                alpha += px_alpha(x + 1, y);
                alpha += px_alpha(x + 2, y) / 2;

                alpha += px_alpha(x - 1, y - 1) / 2;
                alpha += px_alpha(x - 1, y + 1) / 2;
                alpha += px_alpha(x + 1, y - 1) / 2;
                alpha += px_alpha(x + 1, y + 1) / 2;
                alpha = alpha.min(255);

                let pixel_offset = x * 4 + y * 4 * stride;
                *dst.add(pixel_offset + 3) = alpha as u8;
            }
        }
    }

    let mut total_nonzero_alpha = 0usize;
    let mut max_alpha = 0u8;
    for alpha in outlined_data.iter().skip(3).step_by(4) {
        if *alpha > 0 {
            total_nonzero_alpha += 1;
            if *alpha > max_alpha {
                max_alpha = *alpha;
            }
        }
    }
    info!(
        "[NAME_TAG_DIAG] Entity {:?} texture {}x{} alpha_nonzero={} max_alpha={}",
        debug_entity,
        target_texture_width,
        target_texture_height,
        total_nonzero_alpha,
        max_alpha
    );

    if total_nonzero_alpha == 0 {
        // In bevy_egui 0.39, the font atlas texture can exist before glyph pixels for
        // this specific text are uploaded. Treat fully transparent output as not-ready.
        return None;
    }

    let mut image = Image::new(
        Extent3d {
            width: target_texture_width,
            height: target_texture_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        outlined_data.clone(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    // Use nearest sampling so world-space text remains crisp instead of blurry.
    image.sampler = ImageSampler::nearest();
    let image = images.add(image);
    
    // info!("[NAME_TAG_DEBUG] Created image handle: {:?}", image);

    let mut rects: ArrayVec<WorldUiRect, 2> = ArrayVec::new();
    let mut row_offset_y = max_bounds.y - 8.0 * (pending_data.colors.len() - 1) as f32;

    if matches!(pending_data.name_tag_type, NameTagType::Monster) {
        // Give some space for monster health bar under name
        row_offset_y += 15.0;
    }

    // Create WorldUiRect for the outlined text
    for (row_index, row_color) in pending_data.colors.iter().enumerate() {
        let (row_bounds_min, row_bounds_max) = row_bounds[row_index];
        let row_size = row_bounds_max - row_bounds_min;
        let uv_x0 = row_bounds_min.x / target_texture_width as f32;
        let uv_x1 = row_bounds_max.x / target_texture_width as f32;
        let uv_y0 = row_bounds_min.y / target_texture_height as f32;
        let uv_y1 = row_bounds_max.y / target_texture_height as f32;

        let x0 = row_bounds_min.x.max(0.0).floor() as usize;
        let y0 = row_bounds_min.y.max(0.0).floor() as usize;
        let x1 = row_bounds_max.x.min(target_texture_width as f32).ceil() as usize;
        let y1 = row_bounds_max.y.min(target_texture_height as f32).ceil() as usize;
        let mut row_nonzero_alpha = 0usize;
        if x1 > x0 && y1 > y0 {
            for y in y0..y1 {
                for x in x0..x1 {
                    let idx = (y * target_texture_width as usize + x) * 4 + 3;
                    if outlined_data[idx] > 0 {
                        row_nonzero_alpha += 1;
                    }
                }
            }
        }
        info!(
            "[NAME_TAG_DIAG] Entity {:?} row={} bounds=({:.1},{:.1})-({:.1},{:.1}) uv=({:.4},{:.4})-({:.4},{:.4}) row_alpha_nonzero={} row_size={:?}",
            debug_entity,
            row_index,
            row_bounds_min.x,
            row_bounds_min.y,
            row_bounds_max.x,
            row_bounds_max.y,
            uv_x0,
            uv_y0,
            uv_x1,
            uv_y1,
            row_nonzero_alpha,
            row_size
        );

        rects.push(WorldUiRect {
            screen_offset: Vec2::new(-row_size.x / 2.0, row_offset_y - row_size.y),
            screen_size: row_size,
            image: image.clone(),
            uv_min: Vec2::new(uv_x0, uv_y0),
            uv_max: Vec2::new(uv_x1, uv_y1),
            color: *row_color,
            order: ORDER_NAME,
        });
        row_offset_y -= row_size.y - 8.0;
    }

    // info!("[NAME_TAG_DEBUG] Successfully created NameTagData with {} rects", rects.len());

    Some(NameTagData {
        image,
        size: max_bounds,
        rects,
    })
}

pub fn name_tag_system(
    mut commands: Commands,
    mut name_tag_cache: Local<NameTagCache>,
    query_add: Query<NameTagObjectQuery, Without<NameTagEntity>>,
    query_changed: Query<(Entity, Option<&NameTagEntity>), Changed<ClientEntityName>>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_nametags: Query<(Entity, &NameTagEntity)>,
    query_camera: Query<Entity, (With<Camera>, With<PrimaryEguiContext>)>,
    egui_managed_textures: Res<bevy_egui::EguiManagedTextures>,
    mut egui_context: EguiContexts,
    mut images: ResMut<Assets<Image>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    name_tag_settings: Res<NameTagSettings>,
    mut load_zone_events: MessageReader<LoadZoneEvent>,
) {
    // Get the camera entity with PrimaryEguiContext - this is the key for EguiManagedTextures
    let Ok(camera_entity) = query_camera.single() else {
        return;
    };
    
    let player = query_player.single().ok();
    let pixels_per_point = egui_context.ctx_mut().unwrap().pixels_per_point();

    if load_zone_events.read().last().is_some()
        || pixels_per_point != name_tag_cache.pixels_per_point
    {
        // When the zone changes, we flush all cached name tag textures to avoid leaking
        // If pixels_per_point has changed then we need to regenerate name tags using new DPI
        for (entity, name_tag_entity) in query_nametags.iter() {
            commands.entity(entity).remove::<NameTagEntity>();
            commands.entity(name_tag_entity.0).despawn();
        }

        name_tag_cache.cache.clear();
        name_tag_cache.pending.clear();
        name_tag_cache.pixels_per_point = pixels_per_point;
        return;
    }

    for (entity, name_tag_entity) in query_changed.iter() {
        // Despawn previous name tag
        if let Some(name_tag_entity) = name_tag_entity {
            commands.entity(entity).remove::<NameTagEntity>();
            commands.entity(name_tag_entity.0).despawn();
        }

        // Clear any pending name tag for this entity
        name_tag_cache.pending.remove(&entity);
    }

    // Check if we have pending entries but no entities to process - clear them
    if !name_tag_cache.pending.is_empty() {
        let add_count = query_add.iter().len();
        if add_count == 0 {
            name_tag_cache.pending.clear();
        }
    }
    
    for object in query_add.iter() {
        let name_tag_type = if let Some(npc) = object.npc {
            if object
                .team
                .map_or(false, |team| team.id == Team::DEFAULT_NPC_TEAM_ID)
                || game_data
                    .npcs
                    .get_npc(npc.id)
                    .map_or(false, |npc| npc.npc_type_index == NonZeroU16::new(999))
            {
                NameTagType::Npc
            } else {
                NameTagType::Monster
            }
        } else {
            NameTagType::Character
        };

        let name_tag_data = if let Some(name_tag_data) = name_tag_cache.cache.get(&object.name.name)
        {
            name_tag_data
        } else if let Some(pending_name_tag_data) = name_tag_cache.pending.remove(&object.entity) {
            if let Some(name_tag_data) = create_nametag_data(
                camera_entity,
                &mut egui_context,
                &egui_managed_textures,
                &mut images,
                pending_name_tag_data.clone(),
                object.entity,
            ) {
                name_tag_cache
                    .cache
                    .insert(object.name.name.clone(), name_tag_data);
                name_tag_cache.cache.get(&object.name.name).unwrap()
            } else {
                // Re-insert pending data to try again next frame
                name_tag_cache.pending.insert(object.entity, pending_name_tag_data);
                continue;
            }
        } else {
            // Create egui text and process on next frame.
            let pending_data = create_pending_nametag(
                &name_tag_settings,
                &mut egui_context,
                &object,
                player.as_ref(),
                name_tag_type,
            );

            name_tag_cache.pending.insert(object.entity, pending_data);
            continue;
        };

        // Spawn name tag entities
        let visibility = if name_tag_settings.show_all[name_tag_type] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        
        // info!("[NAME_TAG_DEBUG] Spawning name tag entity for '{}' with visibility {:?}", object.name.name, visibility);
        
        let name_tag_entity = commands
            .spawn((
                NameTag { name_tag_type },
                NoFrustumCulling,
                visibility,
                VisibilityClass::default(),
                Transform::from_translation(Vec3::new(0.0, object.model_height.height, 0.0)),
                GlobalTransform::default(),
            ))
            .id();

        // info!("[NAME_TAG_DEBUG] Spawned name tag entity {:?} at height {}", name_tag_entity, object.model_height.height);

        let target_mark = if let Some(npc_type_index) = object
            .npc
            .and_then(|npc| game_data.npcs.get_npc(npc.id))
            .and_then(|npc| npc.npc_type_index)
        {
            ui_resources
                .get_sprite_by_index(UiSpriteSheetType::TargetMark, npc_type_index.get() as usize)
                .zip(ui_resources.get_sprite_image_by_index(
                    UiSpriteSheetType::TargetMark,
                    npc_type_index.get() as usize,
                ))
        } else {
            None
        }
        .or_else(|| {
            ui_resources
                .get_sprite(0, "UI00_TARGETMARK")
                .zip(ui_resources.get_sprite_image(0, "UI00_TARGETMARK"))
        });

        let mut healthbar_fg_rect = None;
        let mut healthbar_bg_rect = None;
        let (health_foreground, health_background) = match name_tag_type {
            NameTagType::Character => (
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_RED_AVATAR")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_RED_AVATAR")),
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_BG_AVATAR")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_BG_AVATAR")),
            ),
            NameTagType::Monster => (
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_RED")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_RED")),
                ui_resources
                    .get_sprite(0, "UI00_GUAGE_BACKGROUND")
                    .zip(ui_resources.get_sprite_image(0, "UI00_GUAGE_BACKGROUND")),
            ),
            NameTagType::Npc => (None, None),
        };

        let mut health_bar_size = Vec2::ZERO;
        let mut health_bar_foreground_uv_x_bounds = (0.0, 0.0);
        if let (
            Some((health_foreground_sprite, health_foreground_image)),
            Some((health_background_sprite, health_background_image)),
        ) = (health_foreground, health_background)
        {
            let bar_width = health_background_sprite.width * pixels_per_point;
            let bar_height = health_background_sprite.height * pixels_per_point;
            let bar_offset_y = if matches!(name_tag_type, NameTagType::Character) {
                // Character health bar is behind name
                name_tag_data.rects[0].screen_offset.y + name_tag_data.rects[0].screen_size.y / 2.0
                    - bar_height / 2.0
            } else {
                // Monster health bar under name
                name_tag_data.rects[0].screen_offset.y - bar_height
            };
            health_bar_size = Vec2::new(bar_width, bar_height);

            healthbar_bg_rect = Some(WorldUiRect {
                screen_offset: Vec2::new(-bar_width / 2.0, bar_offset_y),
                screen_size: Vec2::new(bar_width, bar_height),
                image: health_background_image.clone(),
                uv_min: Vec2::new(
                    health_background_sprite.uv.min.x,
                    health_background_sprite.uv.min.y,
                ),
                uv_max: Vec2::new(
                    health_background_sprite.uv.max.x,
                    health_background_sprite.uv.max.y,
                ),
                color: Color::WHITE,
                order: ORDER_HEALTH_BACKGROUND,
            });

            health_bar_foreground_uv_x_bounds = (
                health_foreground_sprite.uv.min.x,
                health_foreground_sprite.uv.max.x,
            );
            healthbar_fg_rect = Some(WorldUiRect {
                screen_offset: Vec2::new(-bar_width / 2.0, bar_offset_y),
                screen_size: Vec2::new(bar_width, bar_height),
                image: health_foreground_image.clone(),
                uv_min: Vec2::new(
                    health_foreground_sprite.uv.min.x,
                    health_foreground_sprite.uv.min.y,
                ),
                uv_max: Vec2::new(
                    health_foreground_sprite.uv.max.x,
                    health_foreground_sprite.uv.max.y,
                ),
                color: Color::WHITE,
                order: ORDER_HEALTH_FOREGROUND,
            });
        }

        let mut target_marks: ArrayVec<WorldUiRect, 2> = ArrayVec::default();
        if let Some((target_mark_sprite, target_mark_image)) = target_mark {
            let mark_width = target_mark_sprite.width * pixels_per_point;
            let mark_height = target_mark_sprite.height * pixels_per_point;
            let mark_offset_y =
                name_tag_data.rects[0].screen_offset.y + name_tag_data.rects[0].screen_size.y / 2.0;

            target_marks.push(WorldUiRect {
                screen_offset: Vec2::new(
                    name_tag_data.rects[0]
                        .screen_offset
                        .x
                        .min(-health_bar_size.x / 2.0)
                        - mark_width,
                    mark_offset_y - mark_height / 2.0,
                ),
                screen_size: Vec2::new(mark_width, mark_height),
                image: target_mark_image.clone(),
                uv_min: Vec2::new(target_mark_sprite.uv.min.x, target_mark_sprite.uv.min.y),
                uv_max: Vec2::new(target_mark_sprite.uv.max.x, target_mark_sprite.uv.max.y),
                color: Color::WHITE,
                order: ORDER_TARGET_MARK,
            });

            target_marks.push(WorldUiRect {
                screen_offset: Vec2::new(
                    (name_tag_data.rects[0].screen_offset.x + name_tag_data.rects[0].screen_size.x)
                        .max(health_bar_size.x / 2.0),
                    mark_offset_y - mark_height / 2.0,
                ),
                screen_size: Vec2::new(mark_width, mark_height),
                image: target_mark_image.clone(),
                uv_min: Vec2::new(target_mark_sprite.uv.max.x, target_mark_sprite.uv.min.y),
                uv_max: Vec2::new(target_mark_sprite.uv.min.x, target_mark_sprite.uv.max.y),
                color: Color::WHITE,
                order: ORDER_TARGET_MARK,
            });
        }

        for (rect_idx, rect) in name_tag_data.rects.iter().enumerate() {
            let rect: WorldUiRect = rect.clone();
            info!(
                "[NAME_TAG_DEBUG] Spawning name rect {} with size {:?}, uv=({:.4},{:.4})-({:.4},{:.4}), image={:?}, name='{}'",
                rect_idx,
                rect.screen_size,
                rect.uv_min.x,
                rect.uv_min.y,
                rect.uv_max.x,
                rect.uv_max.y,
                rect.image.id(),
                object.name.name
            );
            commands
                .spawn((
                    NameTagName,
                    NoFrustumCulling,
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Inherited,
                    VisibilityClass::default(),
                ))
                .insert(ChildOf(name_tag_entity));
        }

        for rect in target_marks.drain(..) {
            commands
                .spawn((
                    NameTagTargetMark,
                    NoFrustumCulling,
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Hidden,
                    VisibilityClass::default(),
                ))
                .insert(ChildOf(name_tag_entity));
        }

        if let Some(rect) = healthbar_bg_rect.take() {
            commands
                .spawn((
                    NameTagHealthbarBackground,
                    NoFrustumCulling,
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Hidden,
                    VisibilityClass::default(),
                ))
                .insert(ChildOf(name_tag_entity));
        }

        if let Some(rect) = healthbar_fg_rect.take() {
            commands
                .spawn((
                    NameTagHealthbarForeground {
                        full_width: health_bar_size.x,
                        uv_min_x: health_bar_foreground_uv_x_bounds.0,
                        uv_max_x: health_bar_foreground_uv_x_bounds.1,
                    },
                    NoFrustumCulling,
                    rect,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Hidden,
                    VisibilityClass::default(),
                ))
                .insert(ChildOf(name_tag_entity));
        }

        commands
            .entity(object.entity)
            .insert(NameTagEntity(name_tag_entity))
            .add_child(name_tag_entity);
            
        // info!("[NAME_TAG_DEBUG] Successfully created name tag for entity {:?} name='{}'", object.entity, object.name.name);
    }
    
    // info!("[NAME_TAG_DEBUG] === NAME TAG SYSTEM END === Cache: {}, Pending: {}", name_tag_cache.cache.len(), name_tag_cache.pending.len());
}
