//! Blood spatter effect systems for combat visual feedback.
//!
//! This module implements blood spatter decals that appear on terrain when
//! entities are killed. The system uses Bevy's ForwardDecal for rendering.

use bevy::{
    asset::RenderAssetUsages,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    components::{BloodSpatter, Dead, ModelHeight},
    events::BloodEffectEvent,
    resources::BloodEffectConfig,
};

/// System that listens for entities being marked as Dead and spawns blood spatter events.
///
/// This system queries for entities that just had the `Dead` component added
/// and sends [`BloodEffectEvent::SpawnSpatter`] events to trigger visual effects.
pub fn blood_spatter_on_death_system(
    mut blood_events: EventWriter<BloodEffectEvent>,
    query_dead: Query<&GlobalTransform, (Added<Dead>, With<ModelHeight>)>,
    config: Res<BloodEffectConfig>,
) {
    if !config.enable_blood {
        return;
    }

    for transform in query_dead.iter() {
        let position = transform.translation();

        // Spawn blood spatter at feet position
        blood_events.write(BloodEffectEvent::kill_spatter(
            position,
            Vec3::Y,
            0, // Final blow damage already applied
        ));
    }
}

/// System that processes blood effect events and spawns spatter decals.
///
/// This system handles:
/// - [`BloodEffectEvent::SpawnSpatter`] - Creates forward decal entities for blood
pub fn blood_spatter_spawn_system(
    mut commands: Commands,
    mut blood_events: EventReader<BloodEffectEvent>,
    config: Res<BloodEffectConfig>,
    query_spatters: Query<Entity, With<BloodSpatter>>,
    mut images: ResMut<Assets<Image>>,
    mut decal_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
) {
    if !config.enable_blood {
        blood_events.clear();
        return;
    }

    // Check if we need to enforce spatter limit
    let active_spatter_count = query_spatters.iter().len();
    let max_spatters = config.max_spatters;

    for event in blood_events.read() {
        if let BloodEffectEvent::SpawnSpatter {
            position,
            normal,
            damage_amount: _,
            is_kill,
        } = event
        {
            // Determine spatter count based on event type
            let spatter_count = if *is_kill {
                config.effective_kill_spatter_count()
            } else {
                config.effective_hit_spatter_count()
            };

            // Don't spawn if at limit
            if active_spatter_count >= max_spatters {
                continue;
            }

            let (min_size, max_size) = config.effective_spatter_size_range();

            // Spawn spatter decals
            for i in 0..spatter_count {
                // Stop if we hit the limit
                if active_spatter_count + i >= max_spatters {
                    break;
                }

                // Random offset within radius
                let offset_x = (rand::random::<f32>() - 0.5) * config.spatter_radius;
                let offset_z = (rand::random::<f32>() - 0.5) * config.spatter_radius;

                let spatter_pos = *position + Vec3::new(offset_x, 0.05, offset_z);

                // Random size within range
                let size = min_size + rand::random::<f32>() * (max_size - min_size);

                // Random rotation
                let rotation = rand::random::<f32>() * std::f32::consts::TAU;

                // Alpha based on damage (more damage = more opaque)
                let base_alpha = if *is_kill { 0.8 } else { 0.5 };
                let alpha = base_alpha * config.intensity;

                // Create procedural blood texture
                let blood_texture = create_blood_texture(&mut images);

                // Create decal material
                let material = ForwardDecalMaterial {
                    base: StandardMaterial {
                        base_color_texture: Some(blood_texture),
                        base_color: config.blood_color.with_alpha(alpha),
                        alpha_mode: AlphaMode::Blend,
                        cull_mode: None,
                        ..default()
                    },
                    extension: ForwardDecalMaterialExt {
                        depth_fade_factor: 0.3,
                    },
                };

                commands.spawn((
                    Name::new(format!("BloodSpatter_{}", i)),
                    ForwardDecal,
                    MeshMaterial3d(decal_materials.add(material)),
                    BloodSpatter {
                        lifetime: config.spatter_lifetime,
                        alpha,
                        size,
                    },
                    Transform::from_translation(spatter_pos)
                        .looking_to(Vec3::NEG_Y, *normal)
                        .with_scale(Vec3::new(size, size, 0.01))
                        .with_rotation(Quat::from_rotation_z(rotation)),
                ));
            }
        }
    }
}

/// System that fades out blood spatters over time and removes expired ones.
///
/// Spatters begin fading in the last 5 seconds of their lifetime.
pub fn blood_spatter_fade_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut BloodSpatter), With<ForwardDecal>>,
    time: Res<Time>,
    config: Res<BloodEffectConfig>,
) {
    if !config.enable_blood {
        return;
    }

    let delta = time.delta_secs();
    let fade_start_time = 5.0; // Start fading in last 5 seconds

    for (entity, mut spatter) in query.iter_mut() {
        spatter.lifetime -= delta;

        if spatter.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Fade out over last few seconds
        if spatter.lifetime < fade_start_time {
            let new_alpha = (spatter.lifetime / fade_start_time).clamp(0.0, 1.0) * config.intensity;
            spatter.alpha = new_alpha;
            // Note: Updating the material alpha requires mutable access to the material asset
            // For simplicity, we just update the component. A more advanced implementation
            // would update the material as well.
        }
    }
}

/// Creates a procedural blood texture.
///
/// This generates a simple circular gradient texture that looks like a blood splatter.
fn create_blood_texture(images: &mut Assets<Image>) -> Handle<Image> {
    // Use a reasonable texture size (doesn't need to match world size)
    let texture_size = 64u32;
    let center = texture_size as f32 / 2.0;

    // Create RGBA image data
    let mut data = vec![0u8; (texture_size * texture_size * 4) as usize];

    for y in 0..texture_size {
        for x in 0..texture_size {
            let x = x as f32;
            let y = y as f32;

            // Distance from center
            let dx = x - center;
            let dy = y - center;
            let distance = (dx * dx + dy * dy).sqrt();

            // Normalize distance (0 at center, 1 at edge)
            let normalized = distance / center;

            // Create irregular edge using simple noise-like function
            let angle = (dy.atan2(dx) * 4.0).sin() * 0.1;
            let irregular_edge = normalized + angle;

            // Alpha based on distance (fade towards edges)
            let alpha = if irregular_edge < 0.8 {
                1.0 - (irregular_edge / 0.8).powi(2)
            } else {
                0.0
            };

            // Add some variation for organic look
            let variation = ((x * 0.3).sin() * (y * 0.3).cos() * 0.1).abs();
            let final_alpha = (alpha + variation).clamp(0.0, 1.0);

            // Dark red color with alpha
            let idx = ((y as u32 * texture_size + x as u32) * 4) as usize;
            data[idx] = (120.0 + variation * 40.0) as u8; // R
            data[idx + 1] = 0; // G
            data[idx + 2] = 0; // B
            data[idx + 3] = (final_alpha * 255.0) as u8; // A
        }
    }

    let image = Image::new(
        Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    images.add(image)
}

/// Plugin that registers all blood spatter systems.
pub struct BloodSpatterPlugin;

impl Plugin for BloodSpatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                blood_spatter_on_death_system,
                blood_spatter_spawn_system,
                blood_spatter_fade_system,
            )
                .chain(),
        );
    }
}
