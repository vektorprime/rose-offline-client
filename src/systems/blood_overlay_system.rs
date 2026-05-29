//! Blood overlay system for texture-space combat blood.
//!
//! Generates per-material blood overlay textures and binds them to
//! [`RoseObjectExtension`](crate::render::RoseObjectExtension), so blood is drawn
//! on model UV textures (not world-space wound mesh quads).
//!
//! Each material entity gets its own overlay texture containing only the stains
//! that belong to that material's UV space, fixing the UV mismatch problem
//! (Root Cause #1) where a single shared overlay caused blood to appear at
//! wrong locations on different body parts.

use std::collections::HashMap;

use bevy::{
    pbr::{ExtendedMaterial, MeshMaterial3d, StandardMaterial},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    components::BloodOverlay,
    render::RoseObjectExtension,
    resources::{BloodEffectConfig, BloodOverlayAtlas},
};

/// Recursively collects all material entity + handle pairs from an entity and its descendants.
fn collect_material_entities_recursive(
    entity: Entity,
    query_children: &Query<&Children>,
    query_materials: &Query<(
        Entity,
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    )>,
    results: &mut Vec<(
        Entity,
        Handle<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    )>,
    visited: &mut std::collections::HashSet<Entity>,
) {
    // Avoid infinite loops from circular references
    if !visited.insert(entity) {
        return;
    }

    // Check if this entity has a material
    if let Ok((mat_entity, material)) = query_materials.get(entity) {
        results.push((mat_entity, material.0.clone()));
    }

    // Recurse into children
    if let Ok(children) = query_children.get(entity) {
        for child in children.iter() {
            collect_material_entities_recursive(
                child,
                query_children,
                query_materials,
                results,
                visited,
            );
        }
    }
}

/// System that generates per-material blood overlay textures for entities with blood stains
/// and applies them to the entity's materials.
///
/// This system queries for entities with [`BloodOverlay`] components that have
/// dirty textures and regenerates the overlay texture from the accumulated stains.
/// Each material gets its own overlay texture with only the stains that belong to
/// that material's UV space (Fix #1).
pub fn blood_overlay_generate_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut BloodOverlay,
            Option<&Children>,
            Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
            Option<&BloodOverlayTextures>,
        ),
        With<BloodOverlay>,
    >,
    query_children: Query<&Children>,
    query_materials: Query<(
        Entity,
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    )>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    atlas: Res<BloodOverlayAtlas>,
    config: Res<BloodEffectConfig>,
) {
    if !config.enable_blood {
        return;
    }

    if !config.show_wounds {
        return;
    }

    for (entity, mut blood_overlay, _children, own_material, existing_textures) in query.iter_mut()
    {
        // Collect all material entities from this entity and descendants
        let mut material_entities: Vec<(
            Entity,
            Handle<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        )> = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Add own material if present
        if let Some(handle) = own_material {
            material_entities.push((entity, handle.0.clone()));
        }

        // Recursively collect materials from all descendants
        collect_material_entities_recursive(
            entity,
            &query_children,
            &query_materials,
            &mut material_entities,
            &mut visited,
        );

        // If the overlay texture data is already generated and clean, we still need to
        // synchronize it onto the currently-live material handles. Character/NPC model parts
        // can be respawned or reassigned new material handles after blood was originally painted,
        // which leaves BloodOverlayTextures populated but the visible materials unbound.
        if !blood_overlay.texture_dirty {
            let Some(existing) = existing_textures else {
                continue;
            };

            for (mat_entity, mat_handle) in &material_entities {
                if let Some(material) = materials.get_mut(mat_handle) {
                    if let Some(overlay_handle) = existing.textures.get(mat_entity) {
                        material.extension.blood_overlay_texture = Some(overlay_handle.clone());
                        material.extension.blood_params =
                            Vec4::new(config.intensity.clamp(0.0, 1.0), 1.0, 0.0, 0.0);
                    } else {
                        material.extension.blood_overlay_texture = None;
                        material.extension.blood_params = Vec4::new(0.0, 0.0, 0.0, 0.0);
                    }
                }
            }

            continue;
        }

        // Build or update per-material overlay textures
        let mut per_material_textures: HashMap<Entity, Handle<Image>> = HashMap::new();

        // Start with existing textures if available
        if let Some(existing) = existing_textures {
            for (mat_entity, handle) in &existing.textures {
                per_material_textures.insert(*mat_entity, handle.clone());
            }
        }

        // For each material entity, generate/update its overlay texture
        for (mat_entity, _mat_handle) in &material_entities {
            let needs_update = blood_overlay.is_material_dirty(*mat_entity);

            if !needs_update {
                continue;
            }

            // Get stains specific to this material
            let material_stains: Vec<_> = blood_overlay.stains_for_material(*mat_entity);

            if material_stains.is_empty() {
                // No stains for this material — clear the overlay
                per_material_textures.remove(mat_entity);
                blood_overlay.mark_material_clean(*mat_entity);
                continue;
            }

            // Reuse existing texture image when possible
            let overlay_handle =
                if let Some(existing_handle) = per_material_textures.get(mat_entity) {
                    if let Some(image) = images.get_mut(existing_handle) {
                        if image.data.is_some() {
                            paint_overlay_texture(image, &material_stains, &atlas);
                            existing_handle.clone()
                        } else {
                            generate_overlay_texture(&mut images, &material_stains, &atlas)
                        }
                    } else {
                        generate_overlay_texture(&mut images, &material_stains, &atlas)
                    }
                } else {
                    generate_overlay_texture(&mut images, &material_stains, &atlas)
                };

            per_material_textures.insert(*mat_entity, overlay_handle);
            blood_overlay.mark_material_clean(*mat_entity);
        }

        // Store per-material texture map on the owner entity
        commands.entity(entity).insert(BloodOverlayTextures {
            textures: per_material_textures.clone(),
        });

        // Bind overlay textures to extension fields on each material
        for (mat_entity, mat_handle) in &material_entities {
            if let Some(material) = materials.get_mut(mat_handle) {
                if let Some(overlay_handle) = per_material_textures.get(mat_entity) {
                    material.extension.blood_overlay_texture = Some(overlay_handle.clone());
                    material.extension.blood_params =
                        Vec4::new(config.intensity.clamp(0.0, 1.0), 1.0, 0.0, 0.0);
                } else {
                    // No stains for this material — disable blood on it
                    material.extension.blood_overlay_texture = None;
                    material.extension.blood_params = Vec4::new(0.0, 0.0, 0.0, 0.0);
                }
            }
        }

        blood_overlay.texture_dirty = false;
    }
}

/// Component that stores per-material blood overlay texture handles for an entity.
/// Each material entity (mesh part) gets its own overlay texture, fixing the
/// UV space mismatch problem (Root Cause #1).
#[derive(Component, Clone, Debug)]
pub struct BloodOverlayTextures {
    /// Map from material entity ID to its blood overlay texture handle.
    pub textures: HashMap<Entity, Handle<Image>>,
}

/// Legacy component for backward compatibility — stores a single shared overlay texture.
#[derive(Component, Clone, Debug)]
pub struct BloodOverlayTexture {
    /// The generated blood overlay texture handle.
    pub texture: Handle<Image>,
}

/// System that updates blood overlay intensity based on configuration.
pub fn blood_overlay_update_system(query: Query<&BloodOverlay>, config: Res<BloodEffectConfig>) {
    if !config.enable_blood || !config.show_wounds {
        return;
    }

    for _blood_overlay in query.iter() {
        // Blood overlay intensity is managed via the texture alpha
    }
}

/// DIAGNOSTIC: Force enable blood on all materials with known-good values.
/// This system sets blood_params to intensity=1.0, enabled=1.0 on all materials
/// that have a blood overlay texture, regardless of configuration.
/// Use this to isolate whether the issue is with parameter binding or texture generation.
pub fn blood_overlay_force_enable_system(
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
) {
    // Only run when DEBUG_FORCE_BLOOD environment variable is set to "1"
    let debug_force_blood = std::env::var("DEBUG_FORCE_BLOOD").unwrap_or_default() == "1";

    if !debug_force_blood {
        return;
    }

    bevy::log::warn!(
        "[BloodOverlay Force Enable] DEBUG_FORCE_BLOOD=1 detected - forcing blood on all materials"
    );

    let mut count = 0;
    for (_handle, mut material) in materials.iter_mut() {
        // Force blood_params to known-good values: intensity=1.0, enabled=1.0
        material.extension.blood_params = Vec4::new(1.0, 1.0, 0.0, 0.0);
        count += 1;
    }

    bevy::log::info!(
        "[BloodOverlay Force Enable] Set blood_params on {} materials",
        count
    );
}

/// Generates a blood overlay texture from the list of stains for a specific material.
fn generate_overlay_texture(
    images: &mut Assets<Image>,
    stains: &[&crate::components::BloodStain],
    atlas: &BloodOverlayAtlas,
) -> Handle<Image> {
    // 512×512 matches typical character base texture resolution (512 or 1024).
    // 256×256 was insufficient and caused blurry/pixelated blood details.
    let size = 512u32;
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0u8; (size * size * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );

    paint_overlay_texture(&mut image, stains, atlas);
    images.add(image)
}

fn paint_overlay_texture(
    image: &mut Image,
    stains: &[&crate::components::BloodStain],
    atlas: &BloodOverlayAtlas,
) {
    let Some(data) = image.data.as_mut() else {
        return;
    };

    for byte in data.iter_mut() {
        *byte = 0;
    }

    let size = image.texture_descriptor.size.width;

    for stain in stains.iter().filter(|s| s.visible) {
        let variant = if atlas.blood_stains.is_empty() {
            stain.texture_variant % BloodOverlayAtlas::VARIANT_COUNT
        } else {
            stain.texture_variant % atlas.blood_stains.len()
        };
        paint_simple_blood_spot(data, size, stain, variant);
    }
}

/// Paints a simple blood spot onto the overlay texture data.
fn paint_simple_blood_spot(
    data: &mut [u8],
    size: u32,
    stain: &crate::components::BloodStain,
    variant: usize,
) {
    let center_x = stain.uv_center.x * size as f32;
    let center_y = stain.uv_center.y * size as f32;
    let radius = stain.uv_size.x * size as f32 * 0.5;

    let radius_int = radius as isize;
    for dy in -radius_int..=radius_int {
        for dx in -radius_int..=radius_int {
            let px_f = center_x + dx as f32;
            let py_f = center_y + dy as f32;

            let px = px_f as isize;
            let py = py_f as isize;

            if px < 0 || px >= size as isize || py < 0 || py >= size as isize {
                continue;
            }

            let dx_f = dx as f32;
            let dy_f = dy as f32;
            let dist = (dx_f * dx_f + dy_f * dy_f).sqrt();
            let max_dist = radius;

            if dist > max_dist {
                continue;
            }

            let t = 1.0 - (dist / max_dist);
            let alpha = t * stain.alpha;

            if alpha < 0.1 {
                continue;
            }

            let idx = ((py * size as isize + px) * 4) as usize;

            // Dark red blood color with deterministic per-variant tint differences.
            let variant_t = (variant % BloodOverlayAtlas::VARIANT_COUNT) as f32;
            let blood_r = 95.0 + variant_t * 4.5;
            let blood_g = 6.0 + variant_t * 0.9;
            let blood_b = 4.0 + variant_t * 0.7;

            // Blend RGB with proper alpha compositing
            data[idx] = (data[idx] as f32 * (1.0 - alpha) + blood_r * alpha) as u8;
            data[idx + 1] = (data[idx + 1] as f32 * (1.0 - alpha) + blood_g * alpha) as u8;
            data[idx + 2] = (data[idx + 2] as f32 * (1.0 - alpha) + blood_b * alpha) as u8;
            // Use max alpha for proper visibility - the shader will multiply by intensity
            data[idx + 3] = (data[idx + 3].max((alpha * 255.0) as u8));
        }
    }
}

/// Plugin that registers all blood overlay systems.
pub struct BloodOverlayPlugin;

impl Plugin for BloodOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BloodOverlayAtlas>();
        app.add_systems(Startup, initialize_blood_overlay_atlas_system);
        app.add_systems(
            PostUpdate,
            (
                blood_overlay_generate_system,
                blood_overlay_update_system,
                // DIAGNOSTIC: Force enable blood on all materials when DEBUG_FORCE_BLOOD=1
                blood_overlay_force_enable_system,
            ),
        );
    }
}

fn initialize_blood_overlay_atlas_system(
    mut atlas: ResMut<BloodOverlayAtlas>,
    mut images: ResMut<Assets<Image>>,
) {
    if atlas.blood_stains.is_empty() {
        *atlas = BloodOverlayAtlas::new(&mut images);
    }
}
