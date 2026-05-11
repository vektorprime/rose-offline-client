//! Blood overlay atlas resource for model blood effects.
//!
//! This module provides the [`BloodOverlayAtlas`] resource which contains
//! pre-generated blood stain textures used for the overlay system.

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    reflect::Reflect,
};

/// Cached blood overlay textures used by the blood overlay system.
///
/// The atlas contains pre-generated blood stain textures that are painted
/// onto entity overlay textures during combat.
#[derive(Resource, Reflect, Default, Clone, Debug)]
#[reflect(Resource)]
pub struct BloodOverlayAtlas {
    /// Pre-generated blood stain texture variants.
    /// Each variant is a 256x256 RGBA8 texture with a different blood stain pattern.
    pub blood_stains: Vec<Handle<Image>>,
}

impl BloodOverlayAtlas {
    /// Number of blood stain variants in the atlas.
    pub const VARIANT_COUNT: usize = 8;

    /// Creates a new blood overlay atlas with all stain variants.
    pub fn new(images: &mut Assets<Image>) -> Self {
        let mut atlas = Self::default();
        for i in 0..Self::VARIANT_COUNT {
            atlas.blood_stains.push(BloodOverlayAtlas::create_blood_stain_texture_inner(images, i));
        }
        atlas
    }

    /// Creates a procedural blood stain texture.
    fn create_blood_stain_texture_inner(images: &mut Assets<Image>, variant: usize) -> Handle<Image> {
        let size = 256u32;
        let center = size as f32 * 0.5;
        let seed = variant as f32 + 1.0;

        let mut data = vec![0u8; (size * size * 4) as usize];

        // Generate blood stain pattern
        let spot_count = 8 + (variant % 5);
        for i in 0..spot_count {
            let fi = i as f32;
            let angle = hash01(fi, seed, 3.1) * std::f32::consts::TAU;
            let radius = (0.1 + hash01(fi, seed, 4.7) * 0.4) * center;
            let x = center + radius * angle.cos();
            let y = center + radius * angle.sin();
            let spot_size = 3.0 + hash01(fi, seed, 6.3) * 12.0;

            // Paint spot onto texture
            for dy in -spot_size as i32..=spot_size as i32 {
                for dx in -spot_size as i32..=spot_size as i32 {
                    let px = (x as i32 + dx).clamp(0, size as i32 - 1) as u32;
                    let py = (y as i32 + dy).clamp(0, size as i32 - 1) as u32;
                    let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                    if dist < spot_size {
                        let t = 1.0 - (dist / spot_size);
                        let alpha = t * t * 0.8;
                        let idx = ((py * size + px) * 4) as usize;
                        // Dark red blood color with variation
                        let r = (80.0 + hash01(px as f32, py as f32, seed) * 60.0) as u8;
                        let g = (5.0 + hash01(px as f32, py as f32, seed + 10.0) * 15.0) as u8;
                        let b = (3.0 + hash01(px as f32, py as f32, seed + 20.0) * 10.0) as u8;
                        data[idx] = r.min(255);
                        data[idx + 1] = g.min(255);
                        data[idx + 2] = b.min(255);
                        data[idx + 3] = (alpha * 255.0).min(255.0) as u8;
                    }
                }
            }
        }

        images.add(Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        ))
    }
}

fn hash01(x: f32, y: f32, seed: f32) -> f32 {
    let v = (x * 12.9898 + y * 78.233 + seed * 37.719).sin() * 43_758.5453;
    v - v.floor()
}
