use std::sync::Arc;

use bevy::{
    asset::RenderAssetUsages,
    log::warn,
    prelude::{
        Assets, ChildOf, Color, Commands, Entity, GlobalTransform, Image, Local, MessageReader,
        Query, Res, ResMut, Transform, Vec2, Vec3, Visibility, With,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_camera::{
    visibility::{NoFrustumCulling, VisibilityClass},
    Camera,
};
use bevy_egui::{egui, EguiContexts, PrimaryEguiContext};

use crate::{
    components::{
        ChatBubble, ChatBubbleBackground, ChatBubbleEntity, ChatBubbleText, ClientEntityName,
        ModelHeight,
    },
    events::ChatBubbleEvent,
    render::WorldUiRect,
};

const CHAT_BUBBLE_PADDING: f32 = 8.0;
const CHAT_BUBBLE_BACKGROUND_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
const CHAT_BUBBLE_ORDER_TEXT: u8 = 10;
const CHAT_BUBBLE_ORDER_BACKGROUND: u8 = 9;
const CHAT_BUBBLE_VERTICAL_OFFSET: f32 = 0.5;
const CHAT_BUBBLE_DEFAULT_HEIGHT: f32 = 2.0;
const CHAT_BUBBLE_MAX_WIDTH: f32 = 250.0;
const CHAT_BUBBLE_FONT_SIZE: f32 = 14.0;

struct PendingChatBubble {
    target_entity: Entity,
    text: String,
    duration: f32,
    color: Color,
    galley: Arc<egui::Galley>,
}

#[derive(Default)]
pub struct ChatBubblePendingCache {
    pending: Vec<PendingChatBubble>,
}

pub fn chat_bubble_spawn_system(
    mut commands: Commands,
    mut chat_bubble_events: MessageReader<ChatBubbleEvent>,
    query_target_by_name: Query<(Entity, &ClientEntityName)>,
    query_model_height: Query<&ModelHeight>,
    query_existing_bubble: Query<(Entity, &ChatBubbleEntity)>,
    query_camera: Query<Entity, (With<Camera>, With<PrimaryEguiContext>)>,
    mut egui_context: EguiContexts,
    mut images: ResMut<Assets<Image>>,
    egui_managed_textures: Res<bevy_egui::EguiManagedTextures>,
    mut pending_cache: Local<ChatBubblePendingCache>,
) {
    let Ok(camera_entity) = query_camera.single() else {
        return;
    };

    let pixels_per_point = egui_context
        .ctx_mut()
        .ok()
        .map(|c| c.pixels_per_point())
        .unwrap_or(1.0);

    let mut new_pending: Vec<PendingChatBubble> = Vec::new();
    for event in chat_bubble_events.read() {
        let target_entity = match event.target_entity {
            Some(entity) => Some(entity),
            None => query_target_by_name
                .iter()
                .find(|(_, name)| name.name == event.entity_name)
                .map(|(entity, _)| entity),
        };

        let Some(target_entity) = target_entity else {
            continue;
        };

        let text_color = event.color.to_srgba();
        let mut layout_job = egui::epaint::text::LayoutJob::single_section(
            event.text.clone(),
            egui::TextFormat::simple(
                egui::FontId::new(CHAT_BUBBLE_FONT_SIZE, egui::FontFamily::Proportional),
                egui::Color32::from_rgb(
                    (text_color.red * 255.0) as u8,
                    (text_color.green * 255.0) as u8,
                    (text_color.blue * 255.0) as u8,
                ),
            ),
        );
        layout_job.wrap.max_width = CHAT_BUBBLE_MAX_WIDTH;

        let Ok(ctx) = egui_context.ctx_mut() else {
            continue;
        };
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(layout_job));

        // Force font glyph upload in bevy_egui 0.39 (textures are produced only by rendered widgets)
        egui::Area::new(egui::Id::new(("chat_bubble_font_upload", target_entity)))
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_max_size(egui::Vec2::ZERO);
                ui.label(galley.clone());
            });

        new_pending.push(PendingChatBubble {
            target_entity,
            text: event.text.clone(),
            duration: event.duration,
            color: event.color,
            galley,
        });
    }

    let mut pending_to_process = std::mem::take(&mut pending_cache.pending);
    pending_to_process.extend(new_pending);

    for pending in pending_to_process.into_iter() {
        let PendingChatBubble {
            target_entity,
            text,
            duration,
            color,
            galley,
        } = pending;

        let model_height_value = query_model_height
            .get(target_entity)
            .map(|mh| mh.height)
            .unwrap_or(CHAT_BUBBLE_DEFAULT_HEIGHT);

        for (bubble_entity, bubble) in query_existing_bubble.iter() {
            if bubble.target_entity == target_entity {
                commands.entity(bubble_entity).despawn();
            }
        }

        if galley.rows.is_empty() {
            continue;
        }

        let galley_rect = galley.rect;
        let min_pos = Vec2::new(galley_rect.min.x, galley_rect.min.y) * pixels_per_point;
        let max_pos = Vec2::new(galley_rect.max.x, galley_rect.max.y) * pixels_per_point;

        let mut used_texture_ids = std::collections::HashSet::new();
        for row in galley.rows.iter() {
            if let egui::TextureId::Managed(id) = row.visuals.mesh.texture_id {
                used_texture_ids.insert(id);
            }
        }

        let mut all_textures_ready = true;
        let mut font_textures = std::collections::HashMap::new();
        for &id in used_texture_ids.iter() {
            // bevy_egui 0.39 keys managed textures by (camera_entity, texture_id)
            if let Some(managed_texture) = egui_managed_textures.0.get(&(camera_entity, id)) {
                font_textures.insert(id, &managed_texture.color_image);
            } else {
                all_textures_ready = false;
                break;
            }
        }

        if !all_textures_ready {
            pending_cache.pending.push(PendingChatBubble {
                target_entity,
                text,
                duration,
                color,
                galley,
            });
            continue;
        }

        let text_size = Vec2::new(
            (max_pos.x - min_pos.x) + CHAT_BUBBLE_PADDING * 2.0,
            (max_pos.y - min_pos.y) + CHAT_BUBBLE_PADDING * 2.0,
        );

        let target_texture_width = (text_size.x as u32).next_power_of_two();
        let target_texture_height = (text_size.y as u32).next_power_of_two();
        let data_len = (target_texture_width * target_texture_height * 4) as usize;
        let mut text_data = vec![0u8; data_len];

        for row in galley.rows.iter() {
            let font_texture_id = match row.visuals.mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                _ => continue,
            };
            let Some(font_texture) = font_textures.get(&font_texture_id) else {
                continue;
            };

            unsafe {
                let src = font_texture.pixels.as_ptr();
                let src_stride = font_texture.width();
                let dst = text_data.as_mut_ptr();
                let dst_stride = target_texture_width as usize;

                for glyph in row.glyphs.iter() {
                    let uv_min = glyph.uv_rect.min;
                    let uv_max = glyph.uv_rect.max;

                    let mut dst_y = ((glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point
                        - min_pos.y)
                        .floor() as usize
                        + CHAT_BUBBLE_PADDING as usize;

                    let dst_x = ((glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point
                        - min_pos.x)
                        .floor() as usize
                        + CHAT_BUBBLE_PADDING as usize;

                    if dst_x + (uv_max[0] - uv_min[0]) as usize > target_texture_width as usize
                        || dst_y + (uv_max[1] - uv_min[1]) as usize
                            > target_texture_height as usize
                    {
                        continue;
                    }

                    for uv_y in uv_min[1]..uv_max[1] {
                        let mut src_row = src.add(uv_y as usize * src_stride + uv_min[0] as usize);
                        let mut dst_row = dst.add(dst_y * dst_stride * 4 + dst_x * 4);

                        for _ in uv_min[0]..uv_max[0] {
                            let pixel = (*src_row).to_array();
                            // bevy_egui font atlas channel packing may vary across versions.
                            // Use max channel as glyph coverage and store text as white+alpha,
                            // then tint in the world UI shader via vertex color.
                            let coverage = pixel[0].max(pixel[1]).max(pixel[2]).max(pixel[3]);

                            *dst_row.add(0) = 255;
                            *dst_row.add(1) = 255;
                            *dst_row.add(2) = 255;
                            *dst_row.add(3) = coverage;

                            src_row = src_row.add(1);
                            dst_row = dst_row.add(4);
                        }
                        dst_y += 1;
                    }
                }
            }
        }

        let mut outlined_data = text_data.clone();
        unsafe {
            let src = text_data.as_ptr();
            let dst = outlined_data.as_mut_ptr();
            let stride = target_texture_width as usize;

            for y in 2..text_size.y as usize - 2 {
                for x in 2..text_size.x as usize - 2 {
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

        let text_image = Image::new(
            Extent3d {
                width: target_texture_width,
                height: target_texture_height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            outlined_data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        let text_image_handle = images.add(text_image);

        let bg_width = (text_size.x as u32).next_power_of_two();
        let bg_height = (text_size.y as u32).next_power_of_two();
        let bg_data_len = (bg_width * bg_height * 4) as usize;
        let mut bg_data = vec![0u8; bg_data_len];

        for y in 0..bg_height {
            for x in 0..bg_width {
                let idx = ((y * bg_width + x) * 4) as usize;
                let corner_radius = 8.0;
                let x_f = x as f32;
                let y_f = y as f32;
                let in_corner = (x_f < corner_radius && y_f < corner_radius)
                    || (x_f > bg_width as f32 - corner_radius && y_f < corner_radius)
                    || (x_f < corner_radius && y_f > bg_height as f32 - corner_radius)
                    || (x_f > bg_width as f32 - corner_radius
                        && y_f > bg_height as f32 - corner_radius);

                let alpha = if in_corner {
                    let cx = if x_f < corner_radius {
                        corner_radius
                    } else {
                        bg_width as f32 - corner_radius
                    };
                    let cy = if y_f < corner_radius {
                        corner_radius
                    } else {
                        bg_height as f32 - corner_radius
                    };
                    let dist = ((x_f - cx).powi(2) + (y_f - cy).powi(2)).sqrt();
                    if dist < corner_radius {
                        255u8
                    } else {
                        0u8
                    }
                } else {
                    255u8
                };

                bg_data[idx] = 255;
                bg_data[idx + 1] = 255;
                bg_data[idx + 2] = 255;
                bg_data[idx + 3] = alpha;
            }
        }

        let bg_image = Image::new(
            Extent3d {
                width: bg_width,
                height: bg_height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bg_data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        let bg_image_handle = images.add(bg_image);

        let bubble_height = model_height_value + CHAT_BUBBLE_VERTICAL_OFFSET;
        let bubble_entity = commands
            .spawn((
                ChatBubbleEntity { target_entity },
                ChatBubble::new(target_entity, text.clone(), duration),
                NoFrustumCulling,
                Visibility::Inherited,
                VisibilityClass::default(),
                Transform::from_translation(Vec3::new(0.0, bubble_height, 0.0)),
                GlobalTransform::default(),
            ))
            .id();

        let bg_uv_x1 = text_size.x / bg_width as f32;
        let bg_uv_y1 = text_size.y / bg_height as f32;

        commands.spawn((
            ChatBubbleBackground,
            NoFrustumCulling,
            WorldUiRect {
                image: bg_image_handle,
                screen_offset: Vec2::new(-text_size.x / 2.0, -text_size.y),
                screen_size: text_size,
                uv_min: Vec2::new(0.0, 0.0),
                uv_max: Vec2::new(bg_uv_x1, bg_uv_y1),
                color: CHAT_BUBBLE_BACKGROUND_COLOR,
                order: CHAT_BUBBLE_ORDER_BACKGROUND,
            },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Inherited,
            VisibilityClass::default(),
            ChildOf(bubble_entity),
        ));

        let text_uv_x1 = text_size.x / target_texture_width as f32;
        let text_uv_y1 = text_size.y / target_texture_height as f32;

        commands.spawn((
            ChatBubbleText,
            NoFrustumCulling,
            WorldUiRect {
                image: text_image_handle,
                screen_offset: Vec2::new(-text_size.x / 2.0, -text_size.y),
                screen_size: text_size,
                uv_min: Vec2::new(0.0, 0.0),
                uv_max: Vec2::new(text_uv_x1, text_uv_y1),
                color,
                order: CHAT_BUBBLE_ORDER_TEXT,
            },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Inherited,
            VisibilityClass::default(),
            ChildOf(bubble_entity),
        ));

        commands.entity(target_entity).add_child(bubble_entity);
    }

    if !pending_cache.pending.is_empty() && egui_managed_textures.0.is_empty() {
        warn!("[CHAT_BUBBLE_DEBUG] Font textures not ready yet; pending chat bubbles will retry next frame");
    }
}
