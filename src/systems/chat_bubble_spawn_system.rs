use std::sync::Arc;

use bevy::{
    log::{info, warn},
    prelude::{
        Assets, Color, Commands, Entity, EventReader, GlobalTransform, Handle, Image, Local, Query, Res,
        ResMut, Transform, Vec2, Vec3, Visibility, With, Without,
    },
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::{NoFrustumCulling, VisibilityClass},
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
const CHAT_BUBBLE_BACKGROUND_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
const CHAT_BUBBLE_ORDER_TEXT: u8 = 10;
const CHAT_BUBBLE_ORDER_BACKGROUND: u8 = 9;
const CHAT_BUBBLE_VERTICAL_OFFSET: f32 = 0.5;  // Reduced from 2.0 - offset above model height
const CHAT_BUBBLE_DEFAULT_HEIGHT: f32 = 2.0; // Fallback height if ModelHeight not available
const CHAT_BUBBLE_MAX_WIDTH: f32 = 250.0;
const CHAT_BUBBLE_FONT_SIZE: f32 = 14.0;

/// Pending chat bubble data waiting for font texture to be ready
struct PendingChatBubble {
    target_entity: Entity,
    text: String,
    duration: f32,
    color: Color,
    galley: Arc<egui::Galley>,
}

/// Cache for pending chat bubbles that need font textures to be ready
#[derive(Default)]
pub struct ChatBubblePendingCache {
    pending: Vec<PendingChatBubble>,
}

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
    mut pending_cache: Local<ChatBubblePendingCache>,
) {
    let Ok(window_entity) = query_window.get_single() else {
        warn!("[CHAT_BUBBLE_DEBUG] No primary window found!");
        return;
    };

    let pixels_per_point = egui_context.ctx_mut().pixels_per_point();
   //// info!("[CHAT_BUBBLE_DEBUG] Pixels per point: {}", pixels_per_point);

    // Log event count
    let event_count = chat_bubble_events.read().len();
   //// info!("[CHAT_BUBBLE_DEBUG] Processing {} new events", event_count);
    
    // Reset the event reader since we consumed it above for counting
    // Actually we need to re-read, let's handle this differently
    
    // Convert new events to pending bubbles
    let new_pending: Vec<PendingChatBubble> = chat_bubble_events
        .read()
        .filter_map(|event| {
           //// info!("[CHAT_BUBBLE_DEBUG] Event received - entity: {:?}, name: '{}', text: '{}'", 
            //    event.target_entity, event.entity_name, event.text);
            
            // Find target entity
            let target_entity = match event.target_entity {
                Some(entity) => {
                   //// info!("[CHAT_BUBBLE_DEBUG] Using direct entity reference: {:?}", entity);
                    Some(entity)
                },
                None => {
                    let found = query_target_by_name
                        .iter()
                        .find(|(_, name)| name.name == event.entity_name)
                        .map(|(entity, _)| entity);
                   //// info!("[CHAT_BUBBLE_DEBUG] Looking up entity by name '{}': found={:?}", event.entity_name, found);
                    found
                }
            };

            let Some(target_entity) = target_entity else {
                //warn!("[CHAT_BUBBLE_DEBUG] Could not find target entity for event: {:?}", event.entity_name);
                return None;
            };

            // Create egui text layout
            let mut layout_job = egui::epaint::text::LayoutJob::single_section(
                event.text.clone(),
                egui::TextFormat::simple(
                    egui::FontId::new(CHAT_BUBBLE_FONT_SIZE, egui::FontFamily::Proportional),
                    egui::Color32::from_rgb(
                        (event.color.to_srgba().red * 255.0) as u8,
                        (event.color.to_srgba().green * 255.0) as u8,
                        (event.color.to_srgba().blue * 255.0) as u8,
                    ),
                ),
            );
            layout_job.wrap.max_width = CHAT_BUBBLE_MAX_WIDTH;

            let galley = egui_context
                .ctx_mut()
                .fonts(|fonts| fonts.layout_job(layout_job));

           //// info!("[CHAT_BUBBLE_DEBUG] Created galley with {} rows", galley.rows.len());

            Some(PendingChatBubble {
                target_entity,
                text: event.text.clone(),
                duration: event.duration,
                color: event.color,
                galley,
            })
        })
        .collect();

   //// info!("[CHAT_BUBBLE_DEBUG] Converted {} events to pending bubbles", new_pending.len());

    // Process pending chat bubbles from previous frames (retry when font texture wasn't ready)
    let pending_from_cache = pending_cache.pending.len();
    let mut pending_to_process = std::mem::take(&mut pending_cache.pending);
    pending_to_process.extend(new_pending);
    
   //// info!("[CHAT_BUBBLE_DEBUG] Processing {} total bubbles ({} from cache, {} new)", 
    //    pending_to_process.len(), pending_from_cache, pending_to_process.len() - pending_from_cache);

    // Log egui managed textures state
   //// info!("[CHAT_BUBBLE_DEBUG] Egui managed textures count: {}", egui_managed_textures.0.len());
    for (key, _) in egui_managed_textures.0.iter() {
       //// info!("[CHAT_BUBBLE_DEBUG] Texture key: window={:?}, id={}", key.0, key.1);
    }

    for (bubble_idx, pending) in pending_to_process.into_iter().enumerate() {
        let PendingChatBubble {
            target_entity,
            text,
            duration,
            color,
            galley,
        } = pending;

       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Processing bubble for entity {:?}, text='{}'", 
        //    bubble_idx, target_entity, text);

        // Get model height for positioning (use default if not available yet)
        let model_height_value = query_model_height
            .get(target_entity)
            .map(|mh| mh.height)
            .unwrap_or(CHAT_BUBBLE_DEFAULT_HEIGHT);

       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Model height: {}", bubble_idx, model_height_value);

        // Despawn any existing chat bubble for this target
        let mut despawned_count = 0;
        for (bubble_entity, bubble) in query_existing_bubble.iter() {
            if bubble.target_entity == target_entity {
                commands.entity(bubble_entity).despawn_recursive();
                despawned_count += 1;
            }
        }
        if despawned_count > 0 {
           //// info!("[CHAT_BUBBLE_DEBUG] [{}] Despawned {} existing bubbles", bubble_idx, despawned_count);
        }

        // Calculate text bounds and check if font textures are ready
        let galley_rect = galley.rect;
        let min_pos = Vec2::new(galley_rect.min.x, galley_rect.min.y) * pixels_per_point;
        let max_pos = Vec2::new(galley_rect.max.x, galley_rect.max.y) * pixels_per_point;
        
        let mut all_textures_ready = true;
        let mut missing_texture_count = 0;
        let mut used_texture_ids = std::collections::HashSet::new();

        for row in galley.rows.iter() {
            if let egui::TextureId::Managed(id) = row.visuals.mesh.texture_id {
                used_texture_ids.insert(id);
            }
        }

        if galley.rows.is_empty() {
            // Empty text
            continue;
        }

        // Check if all required textures are ready
        let mut font_textures = std::collections::HashMap::new();
        for &id in used_texture_ids.iter() {
            if let Some(managed_texture) = egui_managed_textures.0.get(&(window_entity, id)) {
                font_textures.insert(id, &managed_texture.color_image);
            } else {
                all_textures_ready = false;
                missing_texture_count += 1;
            }
        }

       // info!("[CHAT_BUBBLE_DEBUG] Text: '{}', Bounds: min={:?}, max={:?}, size={:?}, textures_ready={}, rows={}",
        //    text, min_pos, max_pos, max_pos - min_pos, all_textures_ready, galley.rows.len());

       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Text bounds: {:?}, textures ready: {}, font textures found: {}", 
        //    bubble_idx, max_bounds, all_textures_ready, font_source_textures.len());

        // If font textures are not ready, re-add to pending queue and try next frame
        if !all_textures_ready {
            pending_cache.pending.push(PendingChatBubble {
                target_entity,
                text,
                duration,
                color,
                galley,
            });
            warn!("[CHAT_BUBBLE_DEBUG] [{}] Font texture not ready ({} missing), deferring bubble for entity {:?}",
                bubble_idx, missing_texture_count, target_entity);
            continue;
        }

        // Add padding to the calculated bounds
        let text_size = Vec2::new(
            (max_pos.x - min_pos.x) + CHAT_BUBBLE_PADDING * 2.0,
            (max_pos.y - min_pos.y) + CHAT_BUBBLE_PADDING * 2.0,
        );

        // Allocate texture for text
        let target_texture_width = (text_size.x as u32).next_power_of_two();
        let target_texture_height = (text_size.y as u32).next_power_of_two();
        let data_len = (target_texture_width * target_texture_height * 4) as usize;
        let mut text_data = vec![0u8; data_len];

        // Copy glyphs to texture
        let mut glyphs_copied = 0;
        for row in galley.rows.iter() {
            let font_texture_id = match row.visuals.mesh.texture_id {
                egui::TextureId::Managed(id) => id,
                _ => continue,
            };
            let font_texture = match font_textures.get(&font_texture_id) {
                Some(t) => t,
                None => continue,
            };

            unsafe {
                let src = font_texture.pixels.as_ptr();
                let src_stride = font_texture.width();
                let dst = text_data.as_mut_ptr();
                let dst_stride = target_texture_width as usize;

                for glyph in row.glyphs.iter() {
                    let uv_min = glyph.uv_rect.min;
                    let uv_max = glyph.uv_rect.max;

                    let mut dst_y = ((glyph.pos.y + glyph.uv_rect.offset.y) * pixels_per_point - min_pos.y)
                        .floor() as usize
                        + CHAT_BUBBLE_PADDING as usize;

                    let dst_x = ((glyph.pos.x + glyph.uv_rect.offset.x) * pixels_per_point - min_pos.x).floor()
                        as usize
                        + CHAT_BUBBLE_PADDING as usize;

                    // Safety check to prevent out-of-bounds writes
                    if dst_x + (uv_max[0] - uv_min[0]) as usize > target_texture_width as usize ||
                       dst_y + (uv_max[1] - uv_min[1]) as usize > target_texture_height as usize {
                        continue;
                    }

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
                    glyphs_copied += 1;
                }
            }
        }
       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Copied {} glyphs to texture", bubble_idx, glyphs_copied);

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
       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Created text image handle: {:?}", bubble_idx, text_image_handle);

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
       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Created background image handle: {:?}", bubble_idx, bg_image_handle);

        // Spawn chat bubble parent entity
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

       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Spawned parent bubble entity {:?} at height {}", 
        //    bubble_idx, bubble_entity, bubble_height);

        // Spawn background rect
        let bg_uv_x1 = text_size.x / bg_width as f32;
        let bg_uv_y1 = text_size.y / bg_height as f32;

        let bg_rect_entity = commands
            .spawn((
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
            ))
            .set_parent(bubble_entity)
            .id();

       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Spawned background rect entity {:?}", bubble_idx, bg_rect_entity);

        // Spawn text rect
        let text_uv_x1 = text_size.x / target_texture_width as f32;
        let text_uv_y1 = text_size.y / target_texture_height as f32;

        let text_rect_entity = commands
            .spawn((
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
            ))
            .set_parent(bubble_entity)
            .id();

       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Spawned text rect entity {:?}", bubble_idx, text_rect_entity);

        // Add bubble as child of target entity
        commands.entity(target_entity).add_child(bubble_entity);
        
       //// info!("[CHAT_BUBBLE_DEBUG] [{}] Successfully created chat bubble! Parent: {:?}, Children: bg={:?}, text={:?}", 
        //    bubble_idx, target_entity, bg_rect_entity, text_rect_entity);
    }
    
   //// info!("[CHAT_BUBBLE_DEBUG] Spawn system complete. Remaining in cache: {}", pending_cache.pending.len());
}
