use std::sync::Arc;

use bevy::{
    log::info,
    prelude::{
        Assets, Color, Commands, Entity, EventReader, GlobalTransform, Handle, Image, Query, Res,
        ResMut, Transform, Vec2, Vec3, Visibility, With, Without, InheritedVisibility,
        ViewVisibility,
    },
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::NoFrustumCulling,
    },
    window::PrimaryWindow,
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    components::{
        ChatBubble, ChatBubbleBackground, ChatBubbleEntity, ChatBubbleText, ClientEntityName,
        ModelHeight,
    },
    events::ChatBubbleEvent,
    render::WorldUiRect,
};

const CHAT_BUBBLE_PADDING: f32 = 8.0;
const CHAT_BUBBLE_BACKGROUND_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const CHAT_BUBBLE_ORDER_TEXT: u8 = 10;
const CHAT_BUBBLE_ORDER_BACKGROUND: u8 = 9;
const CHAT_BUBBLE_VERTICAL_OFFSET: f32 = 2.0;  // Reduced from 20.0 - offset above model height
const CHAT_BUBBLE_DEFAULT_HEIGHT: f32 = 2.0; // Fallback height if ModelHeight not available
const CHAT_BUBBLE_MAX_WIDTH: f32 = 250.0;
const CHAT_BUBBLE_FONT_SIZE: f32 = 14.0;

/// System that listens for ChatBubbleEvents and spawns chat bubble entities
pub fn chat_bubble_spawn_system(
    mut commands: Commands,
    mut chat_bubble_events: EventReader<ChatBubbleEvent>,
    query_target_by_name: Query<(Entity, &ClientEntityName)>,
    query_model_height: Query<&ModelHeight>,
    query_existing_bubble: Query<(Entity, &ChatBubbleEntity)>,
    mut egui_context: EguiContexts,
    mut images: ResMut<Assets<Image>>,
    query_window: Query<Entity, With<PrimaryWindow>>,
    egui_managed_textures: Res<bevy_egui::EguiManagedTextures>,
) {
    let Ok(window_entity) = query_window.get_single() else {
        return;
    };

    let pixels_per_point = egui_context.ctx_mut().pixels_per_point();

    for event in chat_bubble_events.read() {
        // Find target entity
        let target_entity = match event.target_entity {
            Some(entity) => Some(entity),
            None => query_target_by_name
                .iter()
                .find(|(_, name)| name.name == event.entity_name)
                .map(|(entity, _)| entity),
        };

        let Some(target_entity) = target_entity else {
            info!("[CHAT_BUBBLE] Could not find target entity for event: {:?}", event.entity_name);
            continue;
        };

        // Get model height for positioning (use default if not available yet)
        let model_height_value = query_model_height
            .get(target_entity)
            .map(|mh| mh.height)
            .unwrap_or(CHAT_BUBBLE_DEFAULT_HEIGHT);

        info!("[CHAT_BUBBLE] Spawning bubble for entity {:?} with height {}", target_entity, model_height_value);

        // Despawn any existing chat bubble for this target
        for (bubble_entity, bubble) in query_existing_bubble.iter() {
            if bubble.target_entity == target_entity {
                commands.entity(bubble_entity).despawn_recursive();
            }
        }

        // Create egui text layout
        let layout_job = egui::epaint::text::LayoutJob::single_section(
            event.text.clone(),
            egui::TextFormat::simple(
                egui::FontId::proportional(CHAT_BUBBLE_FONT_SIZE),
                egui::Color32::from_rgb(
                    (event.color.to_srgba().red * 255.0) as u8,
                    (event.color.to_srgba().green * 255.0) as u8,
                    (event.color.to_srgba().blue * 255.0) as u8,
                ),
            ),
        );

        let galley = egui_context
            .ctx_mut()
            .fonts(|fonts| fonts.layout_job(layout_job));

        // Calculate text bounds
        let mut max_bounds = Vec2::new(0.0, 0.0);
        let mut font_source_textures: Vec<&egui::ColorImage> = Vec::new();

        for row in galley.rows.iter() {
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

            row_max.x += 8.0;
            row_max.y += 4.0;
            max_bounds = max_bounds.max(row_max);

            // Get the texture for the font used this row
            let font_texture_id = match row.visuals.mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                egui::TextureId::User(_) => continue,
            };
            if let Some(managed_texture) = egui_managed_textures
                .0
                .get(&(window_entity, font_texture_id))
            {
                font_source_textures.push(&managed_texture.color_image);
            }
        }

        // Add padding
        let text_size = Vec2::new(
            max_bounds.x + CHAT_BUBBLE_PADDING * 2.0,
            max_bounds.y + CHAT_BUBBLE_PADDING * 2.0,
        );

        // Allocate texture for text
        let target_texture_width = (text_size.x as u32).next_power_of_two();
        let target_texture_height = (text_size.y as u32).next_power_of_two();
        let data_len = (target_texture_width * target_texture_height * 4) as usize;
        let mut text_data = vec![0u8; data_len];

        // Copy glyphs to texture
        for (row_index, row) in galley.rows.iter().enumerate() {
            if row_index >= font_source_textures.len() {
                continue;
            }
            let row_font_texture = font_source_textures[row_index];

            unsafe {
                let src = row_font_texture.pixels.as_ptr();
                let src_stride = row_font_texture.width();
                let dst = text_data.as_mut_ptr();
                let dst_stride = target_texture_width as usize;

                for glyph in row.glyphs.iter() {
                    let uv_min = glyph.uv_rect.min;
                    let uv_max = glyph.uv_rect.max;

                    let mut dst_y = ((glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point)
                        .round() as usize
                        + CHAT_BUBBLE_PADDING as usize;

                    let dst_x = ((glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point).round()
                        as usize
                        + CHAT_BUBBLE_PADDING as usize;

                    for uv_y in uv_min[1]..uv_max[1] {
                        let mut src_row = src.add(uv_y as usize * src_stride + uv_min[0] as usize);
                        let mut dst_row = dst.add(dst_y * dst_stride * 4 + dst_x * 4);

                        for _ in uv_min[0]..uv_max[0] {
                            let pixel = (*src_row).to_array();

                            *dst_row.add(0) = pixel[0];
                            *dst_row.add(1) = pixel[1];
                            *dst_row.add(2) = pixel[2];
                            *dst_row.add(3) = pixel[3];

                            src_row = src_row.add(1);
                            dst_row = dst_row.add(4);
                        }
                        dst_y += 1;
                    }
                }
            }
        }

        // Apply outline to text
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

        // Create text image
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

        // Create background image (simple rounded rectangle)
        let bg_width = (text_size.x as u32).next_power_of_two();
        let bg_height = (text_size.y as u32).next_power_of_two();
        let bg_data_len = (bg_width * bg_height * 4) as usize;
        let mut bg_data = vec![0u8; bg_data_len];

        // Fill background with semi-transparent black
        for y in 0..bg_height {
            for x in 0..bg_width {
                let idx = ((y * bg_width + x) * 4) as usize;
                // Simple rounded corners
                let corner_radius = 8.0;
                let x_f = x as f32;
                let y_f = y as f32;
                let in_corner = (x_f < corner_radius && y_f < corner_radius)
                    || (x_f > bg_width as f32 - corner_radius && y_f < corner_radius)
                    || (x_f < corner_radius && y_f > bg_height as f32 - corner_radius)
                    || (x_f > bg_width as f32 - corner_radius
                        && y_f > bg_height as f32 - corner_radius);

                let alpha = if in_corner {
                    // Check if inside rounded corner
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
                        180u8
                    } else {
                        0u8
                    }
                } else {
                    180u8
                };

                bg_data[idx] = 0;
                bg_data[idx + 1] = 0;
                bg_data[idx + 2] = 0;
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

        // Spawn chat bubble parent entity
        let bubble_height = model_height_value + CHAT_BUBBLE_VERTICAL_OFFSET;
        let bubble_entity = commands
            .spawn((
                ChatBubbleEntity { target_entity },
                ChatBubble::new(target_entity, event.text.clone(), event.duration),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Transform::from_translation(Vec3::new(0.0, bubble_height, 0.0)),
                GlobalTransform::default(),
                NoFrustumCulling,
            ))
            .id();

        // Spawn background rect
        let bg_uv_x1 = text_size.x / bg_width as f32;
        let bg_uv_y1 = text_size.y / bg_height as f32;

        commands
            .spawn((
                ChatBubbleBackground,
                WorldUiRect {
                    image: bg_image_handle, // Use strong handle to prevent deallocation
                    screen_offset: Vec2::new(-text_size.x / 2.0, -text_size.y),
                    screen_size: text_size,
                    uv_min: Vec2::new(0.0, 0.0),
                    uv_max: Vec2::new(bg_uv_x1, bg_uv_y1),
                    color: CHAT_BUBBLE_BACKGROUND_COLOR,
                    order: CHAT_BUBBLE_ORDER_BACKGROUND,
                },
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                NoFrustumCulling,
            ))
            .set_parent(bubble_entity);

        // Spawn text rect
        let text_uv_x1 = text_size.x / target_texture_width as f32;
        let text_uv_y1 = text_size.y / target_texture_height as f32;

        commands
            .spawn((
                ChatBubbleText,
                WorldUiRect {
                    image: text_image_handle, // Use strong handle to prevent deallocation
                    screen_offset: Vec2::new(-text_size.x / 2.0, -text_size.y),
                    screen_size: text_size,
                    uv_min: Vec2::new(0.0, 0.0),
                    uv_max: Vec2::new(text_uv_x1, text_uv_y1),
                    color: event.color,
                    order: CHAT_BUBBLE_ORDER_TEXT,
                },
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                NoFrustumCulling,
            ))
            .set_parent(bubble_entity);

        // Add bubble as child of target entity
        commands.entity(target_entity).add_child(bubble_entity);
    }
}
